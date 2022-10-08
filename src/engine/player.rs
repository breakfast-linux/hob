use crate::definition::actions::{Action, Stage};
use crate::definition::Side;
use crate::engine::build_state::BuildState;
use crate::engine::build_style::get_build_style;
use crate::engine::environment::Environment;
use crate::engine::EngineSettings;
use crate::Recipe;
use anyhow::bail;
use futures::future::BoxFuture;
use futures::FutureExt;
use std::sync::Arc;

#[derive(Debug)]
pub struct Player {
    settings: Arc<EngineSettings>,
}

const DEFAULT_ACTIONS: &[Action] = &[Action::Default];

#[derive(Copy, Clone, Debug)]
pub enum Context<'a> {
    Recipe(&'a Recipe),
    Side(&'a Recipe, &'a Side),
}

impl<'a> Context<'a> {
    pub fn recipe(&self) -> &Recipe {
        match self {
            Context::Recipe(r) => &r,
            Context::Side(r, _) => &r,
        }
    }

    pub fn origin(&self) -> &str {
        &self.recipe().name
    }

    pub fn name(&self) -> &str {
        match self {
            Context::Recipe(r) => &r.name,
            Context::Side(_, s) => &s.name,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Context::Recipe(r) => &r.description,
            Context::Side(_, s) => &s.description,
        }
    }

    pub fn depends(&self) -> &[String] {
        match self {
            Context::Recipe(r) => &r.depends,
            Context::Side(_, s) => &s.depends,
        }
    }
}

impl Player {
    pub fn new(settings: Arc<EngineSettings>) -> Player {
        Player { settings }
    }

    pub async fn play_build_stage<'a, 'b: 'a, 'c: 'a, 'd: 'a>(
        &'c self,
        state: &mut BuildState<'a>,
        context: Context<'b>,
        environment: &'d Environment,
    ) -> anyhow::Result<()> {
        self.play(state, context, environment, |state, context| {
            async move {
                let style = get_build_style(state.recipe.style.style);
                let actions = match state.stage {
                    Stage::Configure => style.configure,
                    Stage::Build => style.build,
                    Stage::Install => style.install,
                    _ => anyhow::bail!("Not part of build stages"),
                };

                for action in actions {
                    self.execute_action(state, context, environment, action)
                        .await?;
                }

                Ok(())
            }
            .boxed()
        })
        .await?;

        Ok(())
    }

    pub async fn play<
        'a,
        'b,
        'c: 'd,
        'd,
        F: Send
            + for<'e> FnOnce(&'e mut BuildState<'a>, Context<'b>) -> BoxFuture<'e, anyhow::Result<()>>,
    >(
        &self,
        state: &'c mut BuildState<'a>,
        context: Context<'b>,
        environment: &Environment,
        default: F,
    ) -> anyhow::Result<bool> {
        let playbook: &[Action] = state
            .recipe
            .playbooks
            .get(&state.stage)
            .map_or(DEFAULT_ACTIONS, |x| &x.actions);

        let mut found_default = false;
        let mut pb_iter = playbook.iter();

        while let Some(action) = pb_iter.next() {
            println!("    action {:?}", action);
            match action {
                Action::Default => {
                    found_default = true;
                    break;
                }
                v => {
                    self.execute_action(&mut *state, context, environment, v)
                        .await?
                }
            }
        }

        if found_default {
            default(&mut *state, context).await?;

            while let Some(action) = pb_iter.next() {
                println!("    action {:?}", action);
                match action {
                    Action::Default => {
                        bail!(".default called twice");
                    }
                    v => {
                        self.execute_action(&mut *state, context, environment, v)
                            .await?
                    }
                }
            }
        }

        Ok(found_default)
    }

    async fn execute_action<'a, 'b>(
        &self,
        state: &mut BuildState<'a>,
        context: Context<'b>,
        environment: &Environment,
        action: &Action,
    ) -> anyhow::Result<()> {
        match action {
            Action::Default => unimplemented!(),
            Action::Cc(cc) => {
                let mut cc_args = vec!["-o", &cc.output];

                if let Some(args) = state.recipe.style.vars.cc_flags.as_ref() {
                    for arg in args {
                        cc_args.push(arg);
                    }
                }

                for arg in &cc.input {
                    cc_args.push(arg);
                }

                let mut cmd = environment.command(&state.recipe, "gcc", &cc_args);
                let mut proc = cmd.spawn()?;
                let ec = proc.wait().await?;
                if !ec.success() {
                    bail!("cc failed");
                }
            }
            Action::Make => {
                let make_cmd = state
                    .recipe
                    .style
                    .vars
                    .make_command
                    .as_ref()
                    .map(|x| x.as_str())
                    .unwrap_or("make");

                let jobs = format!("-j{}", environment.cpus + 1);
                let mut make_args: Vec<&str> = vec![&jobs];

                if let Some(args) = state.recipe.style.vars.make_args.as_ref() {
                    for arg in args {
                        make_args.push(&arg)
                    }
                }

                let mut cmd = environment.command(&state.recipe, make_cmd, &make_args);
                let mut proc = cmd.spawn()?;
                let ec = proc.wait().await?;
                if !ec.success() {
                    bail!("make failed");
                }
            }
            Action::MakeInstall => {
                let make_cmd = state
                    .recipe
                    .style
                    .vars
                    .make_command
                    .as_ref()
                    .map(|x| x.as_str())
                    .unwrap_or("make");
                let mut make_args: Vec<&str> = vec![];

                if let Some(args) = state.recipe.style.vars.make_args.as_ref() {
                    for arg in args {
                        make_args.push(&arg)
                    }
                }

                let dest_dir = format!(
                    "DESTDIR={}",
                    self.settings.dest_path_for_recipe(&state.recipe).display()
                );

                make_args.push(&dest_dir);
                make_args.push("install");

                let mut cmd = environment.command(&state.recipe, make_cmd, &make_args);
                let mut proc = cmd.spawn()?;
                let ec = proc.wait().await?;
                if !ec.success() {
                    bail!("make install failed");
                }
            }
            Action::Bin(bin) => match context {
                Context::Recipe(_) => {
                    let dest_path = self.settings.dest_path_for_recipe(&state.recipe);
                    let src = self.settings.source_path_for_recipe(&state.recipe);
                    tokio::fs::create_dir_all(dest_path.join("path")).await?;
                    for item in &bin.binaries {
                        tokio::fs::copy(src.join(item), dest_path.join("bin").join(&item)).await?;
                    }
                }
                Context::Side(_, _) => {
                    todo!()
                }
            },
            Action::Man(man) => match context {
                Context::Recipe(_) => {
                    let dest_path = self.settings.dest_path_for_recipe(&state.recipe);
                    let src = self.settings.source_path_for_recipe(&state.recipe);
                    for item in &man.man_files {
                        let ext = item.rsplit(".").next().unwrap();
                        if !ext.chars().all(|x| x.is_digit(10)) {
                            bail!("invalid man file, should end with .<digit>");
                        }

                        tokio::fs::create_dir_all(
                            dest_path.join("usr/share/man").join(format!("man{}", ext)),
                        )
                        .await?;

                        tokio::fs::copy(
                            src.join(item),
                            dest_path
                                .join("usr/share/man")
                                .join(format!("man{}", ext))
                                .join(&item),
                        )
                        .await?;
                    }
                }
                Context::Side(_, _) => {
                    todo!()
                }
            },
            Action::Link(link) => {
                let dest_path = self.settings.dest_path_for_recipe(&state.recipe);
                for source in &link.source {
                    tokio::fs::symlink(source, dest_path.join(&link.target)).await?;
                }
            }
            Action::Rm(rm) => {
                let dest_path = self.settings.dest_path_for_recipe(&state.recipe);
                for item in &rm.targets {
                    let p = dest_path.join(item);
                    let md = tokio::fs::symlink_metadata(&p).await?;

                    if md.is_dir() && !md.is_symlink() {
                        tokio::fs::remove_dir_all(p).await?;
                    } else {
                        tokio::fs::remove_file(p).await?;
                    }
                }
            }
            Action::Dir(dir) => {
                let dest_path = self.settings.dest_path_for_recipe(&state.recipe);
                for item in &dir.targets {
                    tokio::fs::create_dir_all(dest_path.join(item)).await?;
                }
            }
            Action::Configure => {
                let configure_script = state
                    .recipe
                    .style
                    .vars
                    .configure_script
                    .as_ref()
                    .map(|x| x.as_str())
                    .unwrap_or("./configure");
                let mut configure_args: Vec<&str> = vec![];

                configure_args.push("--prefix=/usr");

                if let Some(args) = state.recipe.style.vars.configure_args.as_ref() {
                    for arg in args {
                        configure_args.push(&arg)
                    }
                }

                let mut cmd = environment.command(&state.recipe, configure_script, &configure_args);
                let mut proc = cmd.spawn()?;
                let ec = proc.wait().await?;
                if !ec.success() {
                    bail!("configure failed");
                }
            }
        }

        Ok(())
    }
}
