use crate::definition::actions::Action;
use crate::definition::build_style::BuildStyleType;

pub struct BuildStyle {
    pub style: BuildStyleType,
    pub configure: &'static [Action],
    pub build: &'static [Action],
    pub install: &'static [Action],
    pub check: &'static [Action],
}

pub const EMPTY_ACTIONS: &[Action] = &[];

pub const TYPES: &[BuildStyle] = &[
    BuildStyle {
        style: BuildStyleType::Noop,
        configure: EMPTY_ACTIONS,
        build: EMPTY_ACTIONS,
        install: EMPTY_ACTIONS,
        check: EMPTY_ACTIONS,
    },
    BuildStyle {
        style: BuildStyleType::Configure,
        configure: &[Action::Configure],
        build: &[Action::Make],
        install: &[Action::MakeInstall],
        check: &[],
    },
    BuildStyle {
        style: BuildStyleType::GnuConfigure,
        configure: &[Action::Configure],
        build: &[Action::Make],
        install: &[Action::MakeInstall],
        check: &[],
    },
];

pub fn get_build_style(style: BuildStyleType) -> &'static BuildStyle {
    TYPES.iter().find(|x| x.style == style).unwrap()
}
