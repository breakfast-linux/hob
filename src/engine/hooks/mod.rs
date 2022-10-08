pub mod install;
pub mod package;

use crate::definition::actions::Stage;
use crate::engine::build_state::BuildState;
use crate::Engine;
use async_trait::async_trait;
use lazy_static::lazy_static;
use std::fmt::Debug;

#[async_trait]
pub trait HookVTable: Debug + Sync {
    fn prio(&self) -> usize;
    fn when(&self) -> (Stage, HookTrigger);

    async fn trigger(&self, state: &mut BuildState, engine: &Engine) -> anyhow::Result<()>;
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum HookTrigger {
    Before,
    After,
}

type HookRef = &'static dyn HookVTable;

const HOOKS: &[HookRef] = &[
    &install::CollectElf,
    &install::StripBinaries,
    &package::PinTimestamps,
    &package::FixPermissions,
];

lazy_static! {
    pub static ref SORTED_HOOKS: Vec<HookRef> = get_sorted_hooks();
}

fn get_sorted_hooks() -> Vec<HookRef> {
    let mut hooks = HOOKS.to_vec();
    hooks.sort_by_key(|v| (v.when(), v.prio()));
    hooks
}

#[async_trait]
pub trait Hook: Debug {
    const PRIORITY: usize;
    const TRIGGER: HookTrigger;
    const STAGE: Stage;

    async fn run(&self, state: &mut BuildState, engine: &Engine) -> anyhow::Result<()>;
}

#[async_trait]
impl<T: Hook + Sync> HookVTable for T {
    fn prio(&self) -> usize {
        Self::PRIORITY
    }

    fn when(&self) -> (Stage, HookTrigger) {
        (Self::STAGE, Self::TRIGGER)
    }

    async fn trigger(&self, state: &mut BuildState, engine: &Engine) -> anyhow::Result<()> {
        self.run(state, engine).await
    }
}
