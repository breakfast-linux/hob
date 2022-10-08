use crate::engine::player::Context;
use crate::engine::EngineSettings;
use crate::Recipe;
use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::Arc;

mod apk;

pub use apk::Apk;

#[async_trait]
pub trait Packager: Send + Sync + Debug {
    async fn build_package<'a>(
        &self,
        recipe: &'a Recipe,
        context: Context<'a>,
    ) -> anyhow::Result<()>;
}

pub trait PackagerBuilder {
    type Output: Packager + 'static;

    fn build(settings: Arc<EngineSettings>) -> Self::Output;
}
