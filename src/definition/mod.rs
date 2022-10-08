pub mod actions;
pub mod build_style;
pub mod parsing;

use crate::definition::actions::{ActionPlaybook, Stage};
use crate::definition::build_style::BuildStyle;
use hob_utils::ObjectTraversal;
use ring::digest::{Context, SHA256};
use serde::Serialize;
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Default, Debug, Clone)]
pub struct Document {
    pub recipes: Vec<Recipe>,
}

#[derive(Default, Debug, Clone, ObjectTraversal)]
pub struct Recipe {
    pub name: String,
    pub version: String,
    pub source_dir: String,
    pub revision: usize,
    pub description: String,
    pub home: Option<String>,
    pub license: Vec<String>,
    pub maintainers: Vec<String>,
    pub depends: Vec<String>,
    pub provides: Vec<String>,
    pub artifacts: Vec<Artifact>,
    pub style: BuildStyle,
    pub sides: Vec<Side>,
    pub options: RecipeOptions,
    pub playbooks: HashMap<Stage, ActionPlaybook>,
}

#[derive(Default, Debug, Clone, ObjectTraversal)]
pub struct RecipeOptions {
    pub strip: Option<bool>,
}

#[derive(Serialize, Debug)]
pub struct RecipeTemplate {
    #[serde(rename = "self-ref")]
    pub self_ref: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub revision: usize,
}

impl Recipe {
    pub fn template_vars(&self) -> RecipeTemplate {
        RecipeTemplate {
            self_ref: format!("{}-{}-r{}", self.name, self.version, self.revision),
            name: self.name.clone(),
            version: self.version.clone(),
            revision: self.revision,
            description: self.description.clone(),
        }
    }
}

#[derive(Debug, Clone, ObjectTraversal)]
pub struct Artifact {
    pub source: ArtifactSource,
    #[skip]
    pub verification: Verification,
}

impl Artifact {
    pub fn file_name(&self) -> &str {
        self.source.file_name()
    }

    pub fn hash_id(&self) -> [u8; 32] {
        let name = self.source.method_name();
        let hash_data = self.source.hash_data();
        let mut digest = Context::new(&SHA256);
        digest.update(name);
        digest.update(hash_data.as_ref());
        let fin = digest.finish();

        return fin.as_ref()[..32].try_into().unwrap();
    }
}

#[derive(Debug, Clone, ObjectTraversal)]
pub enum ArtifactSource {
    Fetch(FetchArtifact),
}

impl ArtifactSource {
    pub fn method_name(&self) -> &[u8] {
        match self {
            ArtifactSource::Fetch(_) => b"fetch",
        }
    }

    pub fn file_name(&self) -> &str {
        match self {
            ArtifactSource::Fetch(f) => f.file_name(),
        }
    }

    pub fn hash_data(&self) -> Cow<'_, [u8]> {
        match self {
            ArtifactSource::Fetch(f) => f.hash_data(),
        }
    }
}

#[derive(Default, Debug, Clone, ObjectTraversal)]
pub struct FetchArtifact {
    pub url: String,
    pub file_name: String,
}

impl FetchArtifact {
    pub fn file_name(&self) -> &str {
        self.file_name.as_str()
    }

    pub fn hash_data(&self) -> Cow<'_, [u8]> {
        self.url.as_bytes().into()
    }
}

#[derive(Default, Debug, Clone)]
pub struct Verification {
    pub sha256: Option<[u8; 32]>,
}

#[derive(Default, Debug, Clone, ObjectTraversal)]
pub struct Side {
    pub name: String,
    pub description: String,
    pub depends: Vec<String>,
    pub claims: Vec<String>,
}
