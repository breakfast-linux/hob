use crate::engine::fetcher::FetchedArtifact;
use crate::engine::EngineSettings;
use async_compression::tokio::bufread::{BzDecoder, GzipDecoder, XzDecoder};
use std::ffi::OsStr;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufRead, AsyncRead, ReadBuf};

#[derive(Debug)]
pub struct Extractor {
    settings: Arc<EngineSettings>,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
enum Archive {
    Zip,
    Tar,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
enum Compression {
    None,
    Gzip,
    Xz,
    Bz,
}

const GUESSES: &[(&'static str, Archive, Compression)] = &[
    (".tar.gz", Archive::Tar, Compression::Gzip),
    (".tar.xz", Archive::Tar, Compression::Xz),
    (".tar.bz", Archive::Tar, Compression::Bz),
    (".tar", Archive::Tar, Compression::None),
    (".zip", Archive::Zip, Compression::None),
];

enum Decompressor<R: AsyncBufRead> {
    PassThrough(R),
    Xz(XzDecoder<R>),
    Gzip(GzipDecoder<R>),
    Bz(BzDecoder<R>),
}

impl<R: AsyncBufRead + Unpin> AsyncRead for Decompressor<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match &mut *self {
            Decompressor::PassThrough(r) => AsyncRead::poll_read(Pin::new(r), cx, buf),
            Decompressor::Xz(r) => AsyncRead::poll_read(Pin::new(r), cx, buf),
            Decompressor::Gzip(r) => AsyncRead::poll_read(Pin::new(r), cx, buf),
            Decompressor::Bz(r) => AsyncRead::poll_read(Pin::new(r), cx, buf),
        }
    }
}

impl Extractor {
    pub fn new(settings: Arc<EngineSettings>) -> Self {
        Extractor { settings }
    }

    pub async fn extract<'a, P: AsRef<Path>>(
        &self,
        artifact: &FetchedArtifact<'a>,
        path: P,
    ) -> anyhow::Result<()> {
        let path = self.settings.source_path().join(path);
        tokio::fs::create_dir_all(&path).await?;

        let found = GUESSES
            .iter()
            .filter_map(|(ext, arch, comp)| {
                if artifact
                    .path
                    .file_name()
                    .and_then(OsStr::to_str)
                    .map_or(false, |x| x.ends_with(*ext))
                {
                    Some((*arch, *comp))
                } else {
                    None
                }
            })
            .next();

        let (arch, compr) = match found {
            None => {
                anyhow::bail!("couldn't guess archive type")
            }

            Some(x) => x,
        };

        let read = OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open(&artifact.path)
            .await?;

        let read = tokio::io::BufReader::new(read);
        let read = match compr {
            Compression::None => Decompressor::PassThrough(read),
            Compression::Gzip => Decompressor::Gzip(GzipDecoder::new(read)),
            Compression::Xz => Decompressor::Xz(XzDecoder::new(read)),
            Compression::Bz => Decompressor::Bz(BzDecoder::new(read)),
        };

        match arch {
            Archive::Zip => {
                let mut archive = async_zip::read::stream::ZipFileReader::new(read);
                while let Some(mut reader) = archive.entry_reader().await? {
                    let entry = reader.entry();
                    let child_path = path.join(entry.filename()).canonicalize()?;

                    if let Some(p) = child_path.parent() {
                        tokio::fs::create_dir_all(p).await?;
                    }
                    let mut f = File::create(child_path).await?;
                    tokio::io::copy(&mut reader, &mut f).await?;
                    f.sync_all().await?;
                }
            }
            Archive::Tar => {
                let mut archive = tokio_tar::Archive::new(read);
                archive.unpack(path).await?;
            }
        }

        Ok(())
    }
}
