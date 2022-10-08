use crate::engine::packager::{Packager, PackagerBuilder};
use crate::engine::player::Context;
use crate::engine::EngineSettings;
use crate::Recipe;
use async_trait::async_trait;
use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::slice::Iter;
use std::sync::Arc;
use std::vec::IntoIter;
use tokio::process::Command;

#[derive(Debug)]
pub struct Apk {
    settings: Arc<EngineSettings>,
}

impl PackagerBuilder for Apk {
    type Output = Apk;

    fn build(settings: Arc<EngineSettings>) -> Self::Output {
        Apk { settings }
    }
}

#[async_trait]
impl Packager for Apk {
    async fn build_package<'a>(
        &self,
        recipe: &'a Recipe,
        context: Context<'a>,
    ) -> anyhow::Result<()> {
        let dir = self.settings.dest_path_for_context(context);
        let package_path = self.settings.package_path_for_packager("apk");

        tokio::fs::create_dir_all(&package_path).await?;

        let mut args = ApkArgs::new();

        args.info("name", &context.name())
            .info(
                "version",
                &format!("{}-r{}", recipe.version, recipe.revision),
            )
            .info("description", context.description())
            .info("license", recipe.license.join(" "))
            .info("origin", context.origin())
            .info("maintainer", recipe.maintainers.join(" "));

        if let Some(home) = &recipe.home {
            args.info("url", home);
        }

        if context.depends().len() > 0 {
            args.info("depends", context.depends().join(" "));
        }

        args.input(dir.as_os_str());

        let mut cmd = Command::new("apk")
            .current_dir(&package_path)
            .arg("mkpkg")
            .args(args)
            .spawn()?;
        cmd.wait().await?;

        Ok(())
    }
}

pub struct ApkArgs(Vec<Cow<'static, OsStr>>);

impl<'a> IntoIterator for &'a ApkArgs {
    type Item = &'a Cow<'static, OsStr>;
    type IntoIter = Iter<'a, Cow<'static, OsStr>>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

impl IntoIterator for ApkArgs {
    type Item = Cow<'static, OsStr>;
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl ApkArgs {
    pub fn new() -> Self {
        ApkArgs(vec![])
    }

    pub fn output<T: Into<OsString>>(&mut self, directory: T) -> &mut Self {
        self.0.push(Cow::Borrowed(OsStr::new("--output")));
        self.0.push(Cow::Owned(directory.into()));
        self
    }

    pub fn input<T: Into<OsString>>(&mut self, directory: T) -> &mut Self {
        self.0.push(Cow::Borrowed(OsStr::new("--files")));
        self.0.push(Cow::Owned(directory.into()));
        self
    }

    pub fn info<D1: Into<OsString>, D2: Into<OsString>>(
        &mut self,
        name: D1,
        value: D2,
    ) -> &mut Self {
        self.0.push(Cow::Borrowed(OsStr::new("--info")));

        let mut x: OsString = name.into();
        x.push(OsStr::new(":"));
        x.push(value.into());

        self.0.push(Cow::Owned(x));
        self
    }
}
