use crate::definition::actions::Stage;
use crate::engine::build_state::BuildState;
use crate::engine::hooks::{Hook, HookTrigger};
use crate::utils::elf::ElfHeader;
use crate::utils::FileWalker;
use crate::Engine;
use async_trait::async_trait;
use std::io::SeekFrom;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, BufReader};

#[derive(Debug)]
pub struct CollectElf;

#[async_trait]
impl Hook for CollectElf {
    const PRIORITY: usize = 0;
    const TRIGGER: HookTrigger = HookTrigger::After;
    const STAGE: Stage = Stage::Install;

    async fn run(&self, state: &mut BuildState, engine: &Engine) -> anyhow::Result<()> {
        let mut dest = FileWalker::new(engine.settings.dest_path_for_recipe(&state.recipe)).await?;

        let mut buffer: [u8; 9] = [0; 9];

        while let Some(entry) = dest.next().await? {
            let file_type = entry.file_type().await?;
            if !file_type.is_file() || file_type.is_symlink() {
                continue;
            }

            let path = entry.path();
            let mut file = BufReader::new(File::open(&path).await?);
            if let Ok(_) = file.read_exact(&mut buffer).await {
                continue;
            }

            // the 9th byte is just to make sure we're not stripping empty archives
            // since those will error out
            if &buffer[..8] == b"!<arch>\n" {
                state.archives.push(path);
                continue;
            }

            file.seek(SeekFrom::Start(0)).await?;
            if let Some(elf_header) = ElfHeader::parse(&mut file).await? {
                state.elf_headers.insert(path, elf_header);
            }
        }

        Ok(())
    }
}
