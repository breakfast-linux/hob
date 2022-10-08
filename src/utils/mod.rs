pub mod elf;

use std::path::Path;
use tokio::fs::{DirEntry, ReadDir};
use tokio::io;

pub struct FileWalker {
    omit_directories: bool,
    stack: Vec<ReadDir>,
}

impl FileWalker {
    pub fn empty(with_directories: bool) -> Self {
        Self {
            omit_directories: !with_directories,
            stack: vec![],
        }
    }

    pub async fn push(&mut self, path: impl AsRef<Path>) -> io::Result<&mut Self> {
        self.stack.push(tokio::fs::read_dir(path).await?);

        Ok(self)
    }

    pub async fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(FileWalker {
            omit_directories: true,
            stack: vec![tokio::fs::read_dir(path).await?],
        })
    }

    pub async fn next(&mut self) -> io::Result<Option<DirEntry>> {
        loop {
            let next = {
                let top = if let Some(top) = self.stack.last_mut() {
                    top
                } else {
                    return Ok(None);
                };

                top.next_entry().await?
            };

            let next = if let Some(v) = next {
                v
            } else {
                self.stack.pop();
                continue;
            };

            if !next.file_type().await?.is_dir() {
                return Ok(Some(next));
            }

            self.stack.push(tokio::fs::read_dir(next.path()).await?);

            if !self.omit_directories {
                return Ok(Some(next));
            }
        }
    }
}
