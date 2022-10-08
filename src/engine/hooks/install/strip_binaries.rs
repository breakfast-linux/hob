use crate::definition::actions::Stage;
use crate::engine::build_state::BuildState;
use crate::engine::hooks::{Hook, HookTrigger};
use crate::Engine;
use async_trait::async_trait;
use tokio::process::Command;

#[derive(Debug)]
pub struct StripBinaries;

#[async_trait]
impl Hook for StripBinaries {
    const PRIORITY: usize = 100;
    const TRIGGER: HookTrigger = HookTrigger::After;
    const STAGE: Stage = Stage::Install;

    async fn run(&self, state: &mut BuildState, _engine: &Engine) -> anyhow::Result<()> {
        if !state.recipe.options.strip.unwrap_or(true) {
            return Ok(());
        }

        let mut binaries = vec![];
        let mut libraries = vec![];

        for (path, header) in &state.elf_headers {
            if header.machine != 0 && header.is_shared_object() {
                libraries.push(path.clone());
            }

            if header.is_executable() {
                binaries.push(path.clone());
            }
        }

        if binaries.len() > 0 {
            Command::new("strip").args(binaries).status().await?;
        }

        if libraries.len() > 0 {
            Command::new("strip")
                .arg("--strip-unneeded")
                .args(libraries)
                .status()
                .await?;
        }

        if state.archives.len() > 0 {
            Command::new("strip")
                .arg("--strip-debug")
                .args(&state.archives)
                .status()
                .await?;
        }

        Ok(())
    }
}
