use hob_utils::ObjectTraversal;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, ObjectTraversal)]
pub struct BuildStyle {
    #[skip]
    pub style: BuildStyleType,
    pub vars: BuildStyleVariables,
}

#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub enum BuildStyleType {
    Noop,
    GnuConfigure,
    Configure,
}

impl BuildStyleType {
    pub fn parse<T: AsRef<str>>(data: T) -> Option<BuildStyleType> {
        Some(match data.as_ref() {
            "noop" => BuildStyleType::Noop,
            "configure" => BuildStyleType::Configure,
            "gnu-configure" => BuildStyleType::GnuConfigure,
            _ => return None,
        })
    }
}

impl Default for BuildStyleType {
    fn default() -> Self {
        BuildStyleType::Noop
    }
}

#[derive(Default, Debug, Clone, ObjectTraversal)]
pub struct BuildStyleVariables {
    pub cc_flags: Option<Vec<String>>,
    pub cxx_flags: Option<Vec<String>>,

    pub configure_script: Option<String>,
    pub configure_args: Option<Vec<String>>,

    pub make_command: Option<String>,
    pub make_use_env: Option<bool>,
    pub make_args: Option<Vec<String>>,
    pub make_env: Option<HashMap<String, String>>,
}
