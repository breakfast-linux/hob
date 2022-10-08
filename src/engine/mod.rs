use crate::definition::actions::Stage;
use crate::definition::Side;
use crate::engine::build_state::BuildState;
use crate::engine::environment::Environment;
use crate::engine::extractor::Extractor;
use crate::engine::fetcher::Fetcher;
use crate::engine::hooks::{HookTrigger, SORTED_HOOKS};
use crate::engine::packager::{Packager, PackagerBuilder};
use crate::engine::player::{Context, Player};
use crate::Recipe;
use futures::future::join_all;
use futures::FutureExt;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::SystemTime;

mod build_state;
pub mod build_style;
mod environment;
mod extractor;
mod fetcher;
mod hooks;
pub mod packager;
mod player;

#[derive(Debug)]
pub struct Engine {
    fetcher: Fetcher,
    extractor: Extractor,
    player: Player,
    environment: Environment,
    packager: Box<dyn Packager>,
    pub settings: Arc<EngineSettings>,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum ChrootMethod {
    None,
    SystemChroot,
    _BubbleWrap,
    _Ethereal,
}

#[derive(Debug)]
pub struct EngineSettings {
    cache_path: PathBuf,
    source_path: PathBuf,
    root_path: PathBuf,
    dest_path: PathBuf,
    package_path: PathBuf,
    chroot_method: ChrootMethod,
}

impl EngineSettings {
    pub fn root_path(&self) -> &Path {
        self.root_path.as_path()
    }

    pub fn dest_path(&self) -> PathBuf {
        self.root_path.join(&self.dest_path)
    }

    pub fn dest_path_for_context(&self, context: Context) -> PathBuf {
        self.dest_path().join(context.name())
    }

    pub fn dest_path_for_recipe(&self, recipe: &Recipe) -> PathBuf {
        self.dest_path().join(&recipe.name)
    }

    pub fn dest_path_for_side(&self, side: &Side) -> PathBuf {
        self.dest_path().join(&side.name)
    }

    pub fn source_path(&self) -> PathBuf {
        self.root_path.join(&self.source_path)
    }

    pub fn source_path_for_recipe(&self, recipe: &Recipe) -> PathBuf {
        self.source_path().join(&recipe.name)
    }

    pub fn extracted_source_path_for_recipe(&self, recipe: &Recipe) -> PathBuf {
        self.source_path()
            .join(&recipe.name)
            .join(&recipe.source_dir)
    }

    pub fn package_path_for_packager(&self, packager: &str) -> PathBuf {
        self.package_path().join(packager)
    }

    pub fn package_path(&self) -> PathBuf {
        self.root_path.join(&self.package_path)
    }

    pub fn cache_path(&self) -> &Path {
        self.cache_path.as_path()
    }
}

#[derive(Debug)]
pub struct EngineError {
    errors: Vec<anyhow::Error>,
}

impl std::error::Error for EngineError {}

impl Display for EngineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Encountered {} errors:", self.errors.len())?;
        for err in &self.errors {
            write!(f, "\n\n\t{}", err)?;
        }

        Ok(())
    }
}

impl Engine {
    pub fn new<T: PackagerBuilder>() -> Self {
        Self::from_settings::<T>(EngineSettings {
            cache_path: PathBuf::from("/tmp/hob/cache"),
            source_path: PathBuf::from(".hob/src"),
            dest_path: PathBuf::from(".hob/dest"),
            root_path: PathBuf::from("/tmp/hob/root"),
            package_path: PathBuf::from(".hob/pkg"),
            chroot_method: ChrootMethod::SystemChroot,
        })
    }

    pub fn from_settings<T: PackagerBuilder>(settings: EngineSettings) -> Self {
        let settings = Arc::from(settings);
        Engine {
            fetcher: Fetcher::new(settings.clone()),
            extractor: Extractor::new(settings.clone()),
            player: Player::new(settings.clone()),
            environment: Environment::new(settings.clone()),
            packager: Box::new(T::build(settings.clone())),
            settings,
        }
    }

    pub async fn prepare_engine(&self) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(&self.settings.cache_path).await?;

        Ok(())
    }

    pub async fn run_hooks<'a>(
        &self,
        state: &mut BuildState<'a>,
        stage: Stage,
        trigger: HookTrigger,
    ) -> anyhow::Result<()> {
        for hook in SORTED_HOOKS.iter().copied() {
            if hook.when() == (stage, trigger) {
                println!("    running hook: {:?}", hook);
                hook.trigger(state, self).await?;
            }
        }
        Ok(())
    }

    pub async fn build_recipe(&self, recipe: &Recipe) -> anyhow::Result<()> {
        let mut state = BuildState {
            build_time: SystemTime::now(),
            recipe,
            stage: Stage::Prepare,
            artifacts: vec![],
            elf_headers: Default::default(),
            archives: vec![],
        };

        for stage in Stage::stages() {
            println!("running stage: {:?}", stage);
            state.stage = stage;

            println!("  running before hooks");
            self.run_hooks(&mut state, stage, HookTrigger::Before)
                .await?;

            println!("  running action");
            match stage {
                Stage::Prepare => {
                    self.prepare_engine().await?;
                }

                Stage::Fetch => {
                    self.player
                        .play(
                            &mut state,
                            Context::Recipe(recipe),
                            &self.environment,
                            |state, _| async { self.fetch(state).await }.boxed(),
                        )
                        .await?;
                }

                Stage::Extract => {
                    self.player
                        .play(
                            &mut state,
                            Context::Recipe(recipe),
                            &self.environment,
                            |state, _| async { self.extract(state).await }.boxed(),
                        )
                        .await?;
                }

                Stage::Split => {
                    self.split_claims(recipe).await?;
                }

                Stage::Package => {
                    self.package(&mut state, Context::Recipe(recipe)).await?;

                    for side in &recipe.sides {
                        self.package(&mut state, Context::Side(recipe, side))
                            .await?;
                    }
                }

                _ => {
                    self.player
                        .play_build_stage(&mut state, Context::Recipe(recipe), &self.environment)
                        .await?;
                }
            }

            println!("  running after hooks");
            self.run_hooks(&mut state, stage, HookTrigger::After)
                .await?;
        }

        Ok(())
    }

    async fn package<'a>(
        &self,
        state: &mut BuildState<'a>,
        context: Context<'a>,
    ) -> anyhow::Result<()> {
        self.packager.build_package(state.recipe, context).await
    }

    async fn extract<'a>(&self, state: &mut BuildState<'a>) -> anyhow::Result<()> {
        for item in &state.artifacts {
            self.extractor.extract(item, &state.recipe.name).await?;
        }

        Ok(())
    }

    async fn fetch<'a>(&self, state: &mut BuildState<'a>) -> anyhow::Result<()> {
        let all_fetch: Vec<_> = join_all(
            state
                .recipe
                .artifacts
                .iter()
                .map(|art| self.fetcher.fetch(art)),
        )
        .await;

        let mut errors = vec![];
        let mut ok = vec![];

        for item in all_fetch {
            match item {
                Ok(v) => ok.push(v),
                Err(e) => errors.push(e),
            }
        }

        if !errors.is_empty() {
            return Err(EngineError { errors }.into());
        }

        state.artifacts = ok;

        Ok(())
    }

    async fn split_claims(&self, recipe: &Recipe) -> anyhow::Result<()> {
        let dest = self.settings.dest_path_for_recipe(recipe);
        for side in &recipe.sides {
            let side_path = self.settings.dest_path_for_side(side);

            for claim in &side.claims {
                let own = claim.clone();
                let glob = wax::Glob::from_str(&own)?;
                for item in glob.walk(&dest) {
                    let item = match item {
                        Err(_) => continue,
                        Ok(item) => item,
                    };

                    let candidate = item.to_candidate_path();
                    let path = PathBuf::from(candidate.as_ref());
                    if let Some(parent) = path.parent() {
                        tokio::fs::create_dir_all(side_path.join(parent)).await?;
                    }

                    tokio::fs::rename(dest.join(&path), side_path.join(&path)).await?;
                }

                drop(glob);
            }
        }

        Ok(())
    }
}
