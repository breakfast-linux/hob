use hob_utils::{ObjectTraversal, ObjectWalker};

mod parsing;

#[derive(Debug, Clone, ObjectTraversal)]
pub struct ActionPlaybook {
    pub stage: Stage,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[repr(u8)]
pub enum Stage {
    Fetch,
    Prepare,
    Extract,
    Configure,
    Build,
    Install,
    Split,
    Package,
}

impl Stage {
    pub const fn stages() -> [Stage; 8] {
        [
            Stage::Prepare,
            Stage::Fetch,
            Stage::Extract,
            Stage::Configure,
            Stage::Build,
            Stage::Install,
            Stage::Split,
            Stage::Package,
        ]
    }
}

impl ObjectTraversal for Stage {
    fn traverse<T: ObjectWalker>(&mut self, _walker: &mut T) {}
}

impl Stage {
    pub fn from_str(input: &str) -> Option<Self> {
        Some(match input {
            "fetch" => Stage::Fetch,
            "prepare" => Stage::Prepare,
            "extract" => Stage::Extract,
            "configure" => Stage::Configure,
            "build" => Stage::Build,
            "install" => Stage::Install,

            _ => return None,
        })
    }
}

#[derive(Debug, Clone, ObjectTraversal)]
pub enum Action {
    Default,
    Cc(CcAction),
    Configure,
    Make,
    MakeInstall,
    Bin(BinAction),
    Man(ManAction),
    Link(LinkAction),
    Rm(RmAction),
    Dir(DirAction),
}

#[derive(Debug, Clone, ObjectTraversal)]
pub struct CcAction {
    pub input: Vec<String>,
    pub output: String,
}

#[derive(Debug, Clone, ObjectTraversal)]
pub struct BinAction {
    pub binaries: Vec<String>,
}

#[derive(Debug, Clone, ObjectTraversal)]
pub struct ManAction {
    pub man_files: Vec<String>,
}

#[derive(Debug, Clone, ObjectTraversal)]
pub struct LinkAction {
    pub source: Vec<String>,
    pub target: String,
}

#[derive(Debug, Clone, ObjectTraversal)]
pub struct RmAction {
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, ObjectTraversal)]
pub struct DirAction {
    pub targets: Vec<String>,
}
