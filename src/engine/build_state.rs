use crate::definition::actions::Stage;
use crate::engine::fetcher::FetchedArtifact;
use crate::utils::elf::ElfHeader;
use crate::Recipe;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

pub struct BuildState<'a> {
    pub build_time: SystemTime,
    pub recipe: &'a Recipe,
    pub stage: Stage,
    pub artifacts: Vec<FetchedArtifact<'a>>,
    pub elf_headers: HashMap<PathBuf, ElfHeader>,
    pub archives: Vec<PathBuf>,
}
