use crate::engine::{ChrootMethod, EngineSettings};
use crate::Recipe;
use std::sync::Arc;
use tokio::process::Command;

#[derive(Debug)]
pub struct Environment {
    settings: Arc<EngineSettings>,
    pub bootstrap: bool,
    pub cpus: usize,
}

impl Environment {
    pub fn new(settings: Arc<EngineSettings>) -> Self {
        Environment {
            settings,
            bootstrap: true,
            cpus: num_cpus::get(),
        }
    }

    pub fn command(&self, recipe: &Recipe, name: &str, args: &[&str]) -> Command {
        let mut chroot_style = self.settings.chroot_method;
        if self.bootstrap {
            chroot_style = ChrootMethod::None;
        }

        let mut name = name;
        let mut args = args.to_vec();

        let root_path = self.settings.root_path().to_string_lossy();

        match chroot_style {
            ChrootMethod::SystemChroot => {
                let mut left = vec![root_path.as_ref(), "--", name];
                left.append(&mut args);
                args = left;
                name = "chroot";
            }
            ChrootMethod::_BubbleWrap => {
                todo!();
            }

            _ => {}
        }

        let mut cmd = Command::new(name);
        cmd.current_dir(self.settings.extracted_source_path_for_recipe(recipe));
        cmd.args(args);
        cmd
    }
}
