use crate::definition::{Artifact, ArtifactSource, Verification};
use crate::engine::EngineSettings;
use hex::ToHex;
use reqwest::Client;
use ring::digest::{Context, SHA256};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::ErrorKind;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug)]
pub struct Fetcher {
    settings: Arc<EngineSettings>,
    http_client: Client,
}

#[derive(Debug)]
pub struct FetchedArtifact<'a> {
    pub artifact: &'a Artifact,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct HobFetchError {
    kind: HobFetchErrorKind,
    affected: Option<HobFetchAffected>,
    artifact: Artifact,
    _inner: Option<Box<dyn Error + Sync + Send>>,
}

impl Error for HobFetchError {}

impl Display for HobFetchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            HobFetchErrorKind::VerificationFailed { hashes } => {
                write!(f, "verification failed (")?;
                let mut first = true;
                for FailedHash {
                    algo,
                    found,
                    expected,
                } in hashes
                {
                    if !first {
                        write!(f, ", ")?;
                    }

                    first = false;
                    write!(
                        f,
                        "{} expected {} but found {}",
                        algo,
                        hex::encode(expected),
                        hex::encode(found)
                    )?;
                }
                write!(f, ")")?;
            }
            HobFetchErrorKind::_Other => {
                write!(f, "unknown error occurred")?;
            }
        }

        write!(f, " for {:?}", self.artifact.source)?;

        match &self.affected {
            Some(HobFetchAffected::Fetched) => {
                write!(f, " with fetched file")?;
            }

            Some(HobFetchAffected::Cache(p)) => {
                write!(f, " with cached file ({})", p.display())?;
            }
            _ => {}
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct FailedHash {
    algo: &'static str,
    found: Box<[u8]>,
    expected: Box<[u8]>,
}

#[derive(Debug)]
pub enum HobFetchAffected {
    Fetched,
    Cache(PathBuf),
}

#[derive(Debug)]
pub enum HobFetchErrorKind {
    VerificationFailed { hashes: Vec<FailedHash> },
    _Other,
}

pub struct DigestPool<'a> {
    pool: Vec<(Context, &'a [u8], &'static str)>,
}

impl DigestPool<'_> {
    pub fn from_verification(verification: &Verification) -> DigestPool {
        let mut pool = vec![];

        if let Some(sha) = &verification.sha256 {
            pool.push((Context::new(&SHA256), &sha[..], "sha256"));
        }

        DigestPool { pool }
    }

    pub fn update(&mut self, data: &[u8]) {
        for (ctx, _, _) in &mut self.pool {
            ctx.update(data);
        }
    }

    pub fn finish(self) -> Result<(), Vec<FailedHash>> {
        let mut failed_hash = vec![];

        for (ctx, comp, algo) in self.pool {
            let dig = ctx.finish();
            if dig.as_ref() != comp {
                failed_hash.push(FailedHash {
                    algo,
                    found: Box::from(dig.as_ref()),
                    expected: Box::from(comp),
                });
            }
        }

        if failed_hash.is_empty() {
            Ok(())
        } else {
            Err(failed_hash)
        }
    }
}

impl Fetcher {
    pub fn new(settings: Arc<EngineSettings>) -> Self {
        Fetcher {
            settings,
            http_client: Client::new(),
        }
    }

    pub async fn fetch<'a>(&self, artifact: &'a Artifact) -> anyhow::Result<FetchedArtifact<'a>> {
        let path = self.create_artifact_path(artifact);
        if tokio::fs::metadata(&path)
            .await
            .map(|_| true)
            .or_else(|e| {
                if e.kind() == ErrorKind::NotFound {
                    Ok(false)
                } else {
                    Err(e)
                }
            })?
        {
            return if let Err(hashes) = self
                .verify_file(path.as_path(), &artifact.verification)
                .await?
            {
                Err(HobFetchError {
                    kind: HobFetchErrorKind::VerificationFailed { hashes },
                    affected: Some(HobFetchAffected::Cache(path)),
                    artifact: artifact.clone(),
                    _inner: None,
                }
                .into())
            } else {
                Ok(FetchedArtifact { artifact, path })
            };
        };

        return match &artifact.source {
            ArtifactSource::Fetch(fetch) => {
                let req = self.http_client.get(&fetch.url).build()?;
                let mut resp = self.http_client.execute(req).await?;

                let mut f = File::create(&path).await?;
                let mut pool = DigestPool::from_verification(&artifact.verification);

                while let Some(chunk) = resp.chunk().await? {
                    pool.update(&chunk);
                    f.write(&chunk).await?;
                }

                if let Err(hashes) = pool.finish() {
                    drop(f);
                    tokio::fs::remove_file(&path).await?;

                    return Err(HobFetchError {
                        kind: HobFetchErrorKind::VerificationFailed { hashes },
                        affected: Some(HobFetchAffected::Fetched),
                        artifact: artifact.clone(),
                        _inner: None,
                    }
                    .into());
                }

                f.sync_all().await?;

                Ok(FetchedArtifact { artifact, path })
            }
        };
    }

    fn create_artifact_path(&self, artifact: &Artifact) -> PathBuf {
        let name = artifact.file_name();
        let hash = artifact.hash_id();
        let file_name = format!("{}-{}", hash.encode_hex::<String>(), name);

        self.settings.cache_path().join(file_name)
    }

    pub async fn verify_file(
        &self,
        path: &Path,
        verification: &Verification,
    ) -> anyhow::Result<Result<(), Vec<FailedHash>>> {
        let mut file = OpenOptions::new()
            .read(true)
            .create(false)
            .write(false)
            .open(path)
            .await?;

        let mut pool = DigestPool::from_verification(verification);
        let mut buffer = vec![0; 4096];

        loop {
            let r = file.read(&mut buffer).await?;
            if r == 0 {
                break;
            }

            pool.update(&buffer[..r]);
        }

        Ok(pool.finish())
    }
}
