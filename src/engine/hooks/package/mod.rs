use crate::definition::actions::Stage;
use crate::engine::build_state::BuildState;
use crate::engine::hooks::{Hook, HookTrigger};
use crate::utils::FileWalker;
use crate::Engine;
use async_trait::async_trait;
use std::cmp::min;
use std::ffi::CString;
use std::os::unix::prelude::OsStrExt;
use std::time::UNIX_EPOCH;

#[derive(Debug)]
pub struct FixPermissions;

#[async_trait]
impl Hook for FixPermissions {
    const PRIORITY: usize = 0;
    const TRIGGER: HookTrigger = HookTrigger::Before;
    const STAGE: Stage = Stage::Package;

    async fn run(&self, _state: &mut BuildState, _engine: &Engine) -> anyhow::Result<()> {
        println!("BIG TODO");

        Ok(())
    }
}

#[derive(Debug)]
pub struct PinTimestamps;

#[async_trait]
impl Hook for PinTimestamps {
    const PRIORITY: usize = 100;
    const TRIGGER: HookTrigger = HookTrigger::Before;
    const STAGE: Stage = Stage::Package;

    async fn run(&self, state: &mut BuildState, engine: &Engine) -> anyhow::Result<()> {
        let path = engine.settings.dest_path_for_recipe(state.recipe);

        let duration = state.build_time.duration_since(UNIX_EPOCH)?;

        let tv = libc::timeval {
            tv_sec: min(duration.as_secs(), libc::time_t::MAX as u64) as libc::time_t,
            tv_usec: duration.subsec_micros() as libc::suseconds_t,
        };

        let mut files = FileWalker::empty(true);

        files.push(path).await?;

        for side in &state.recipe.sides {
            files.push(engine.settings.dest_path_for_side(side)).await?;
        }

        while let Some(file) = files.next().await? {
            let p = file.path();
            let os_str = p.as_os_str();

            let c_str = CString::new(os_str.as_bytes())?;

            let data = [tv, tv];

            unsafe {
                if libc::lutimes(c_str.as_ptr(), &data as _) != 0 {
                    return Err(std::io::Error::last_os_error().into());
                }
            }
        }

        Ok(())
    }
}
