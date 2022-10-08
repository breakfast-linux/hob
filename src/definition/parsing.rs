use crate::definition::actions::ActionPlaybook;
use crate::definition::build_style::{BuildStyle, BuildStyleType, BuildStyleVariables};
use crate::definition::{
    Artifact, ArtifactSource, FetchArtifact, RecipeOptions, Side, Verification,
};
use crate::{Document, Recipe};
use kdl::{KdlDocument, KdlNode};
use miette::{Diagnostic, NamedSource, SourceSpan};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
#[error("Failed parsing hob document")]
pub struct HobParserCompoundError {
    #[source_code]
    pub source_code: NamedSource,
    #[related]
    pub(crate) errors: Vec<HobParseError>,
}

#[derive(Debug, Diagnostic, Error)]
#[error("{source}")]
pub struct HobParseErrorSourced {
    #[source_code]
    pub source_code: NamedSource,
    #[source]
    pub source: HobParseError,
}

#[derive(Debug, Diagnostic, Eq, PartialEq, Error)]
#[error("{kind}")]
pub struct HobParseError {
    /// Offset in chars of the error.
    #[label("{}", label.unwrap_or("here"))]
    pub span: SourceSpan,

    /// Label text for this span. Defaults to `"here"`.
    pub label: Option<&'static str>,

    /// Suggestion for fixing the parser error.
    #[help]
    pub help: Option<String>,

    /// Specific error kind for this parser error.
    pub kind: &'static str,
}

const EMPTY_NODES: &[KdlNode] = &[];

pub(crate) trait GetNodes {
    fn nodes(&self) -> &[KdlNode];
}

pub(crate) trait ProxyMap<T, R> {
    type Output;

    fn map<F: FnOnce(T) -> R>(self, data: F) -> Self::Output;
}

impl<T, R, T2> ProxyMap<T, R> for (Option<T>, T2) {
    type Output = (Option<R>, T2);

    fn map<F: FnOnce(T) -> R>(self, data: F) -> Self::Output {
        (self.0.map(data), self.1)
    }
}

impl GetNodes for KdlNode {
    fn nodes(&self) -> &[KdlNode] {
        self.children().map_or(EMPTY_NODES, |x| x.nodes())
    }
}

pub trait ParseDocument {
    fn parse_document(
        input: &KdlDocument,
        source: &str,
        filename: Option<&str>,
    ) -> miette::Result<Self>
    where
        Self: Sized,
    {
        let (data, errors) = Self::parse_document_with_errors(input);
        data.ok_or_else(|| {
            HobParserCompoundError {
                source_code: NamedSource::new(
                    filename
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "[memory.kdl]".to_string()),
                    source.to_string(),
                ),
                errors,
            }
            .into()
        })
    }

    fn parse_document_strict(
        input: &KdlDocument,
        source: &str,
        filename: Option<&str>,
    ) -> miette::Result<Self>
    where
        Self: Sized,
    {
        let (data, errors) = Self::parse_document_with_errors(input);

        match data {
            Some(obj) if errors.is_empty() => Ok(obj),

            _ => Err(HobParserCompoundError {
                source_code: NamedSource::new(
                    filename
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "[memory.kdl]".to_string()),
                    source.to_string(),
                ),
                errors,
            }
            .into()),
        }
    }

    fn parse_document_with_errors(input: &KdlDocument) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized;
}

pub trait ParseNode {
    fn parse_node(input: &KdlNode, source: &str, filename: Option<&str>) -> miette::Result<Self>
    where
        Self: Sized,
    {
        let (data, errors) = Self::parse_node_with_errors(input);
        data.ok_or_else(|| {
            HobParserCompoundError {
                source_code: NamedSource::new(
                    filename
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "[memory.kdl]".to_string()),
                    source.to_string(),
                ),
                errors,
            }
            .into()
        })
    }

    fn parse_node_strict(
        input: &KdlNode,
        source: &str,
        filename: Option<&str>,
    ) -> miette::Result<Self>
    where
        Self: Sized,
    {
        let (data, errors) = Self::parse_node_with_errors(input);

        match data {
            Some(obj) if errors.is_empty() => Ok(obj),

            _ => Err(HobParserCompoundError {
                source_code: NamedSource::new(
                    filename
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "[memory.kdl]".to_string()),
                    source.to_string(),
                ),
                errors,
            }
            .into()),
        }
    }

    fn parse_node_with_errors(input: &KdlNode) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized;
}

#[macro_export]
macro_rules! parse_string_into {
    ($input:ident, $into:expr, $errors:expr, $name:literal) => {
        use $crate::definition::parsing::extract_single_string_value;

        match extract_single_string_value(
            $input,
            concat!($name, " missing"),
            concat!($name, " should be a string"),
            concat!("only 1 string expected for ", $name),
            concat!($name, " expected a value, property found instead"),
        ) {
            Ok(n) => $into = n.into(),
            Err(e) => $errors.push(e),
        };
    };
}

#[macro_export]
macro_rules! parse_bool_into {
    ($input:ident, $into:expr, $errors:expr, $name:literal) => {
        use $crate::definition::parsing::extract_single_bool_value;

        match extract_single_bool_value(
            $input,
            concat!($name, " missing"),
            concat!($name, " should be a bool"),
            concat!("only 1 bool expected for ", $name),
            concat!($name, " expected a value, property found instead"),
        ) {
            Ok(n) => $into = n.into(),
            Err(e) => $errors.push(e),
        };
    };
}

#[macro_export]
macro_rules! parse_string_list_into {
    ($input:ident, $into:ident, $errors:expr, $name:literal) => {
        use $crate::definition::parsing::{extract_string_values, ListExtHelper};

        match extract_string_values(
            $input,
            concat!($name, " expects only string values"),
            concat!($name, " expected values, property found instead"),
        ) {
            Ok(n) => $into.add(n),
            Err(e) => $errors.push(e),
        };
    };
}

#[macro_export]
macro_rules! parse_string_list_ext_into {
    ($input:ident, $into:expr, $errors:expr, $name:literal) => {
        use $crate::definition::parsing::{extract_string_values_with_extend, ListExtHelper};

        match extract_string_values_with_extend(
            $input,
            concat!($name, " expects only string values"),
            concat!($name, " expected values, property found instead"),
        ) {
            Ok((n, true)) => $into.add(n),
            Ok((n, false)) => $into.set(n),
            Err(e) => $errors.push(e),
        };
    };
}

pub trait ListExtHelper<T> {
    fn add(&mut self, value: Vec<T>);
    fn set(&mut self, value: Vec<T>);
}

impl<T> ListExtHelper<T> for Vec<T> {
    fn add(&mut self, value: Vec<T>) {
        self.extend(value);
    }

    fn set(&mut self, value: Vec<T>) {
        *self = value;
    }
}

impl<T> ListExtHelper<T> for Option<Vec<T>> {
    fn add(&mut self, value: Vec<T>) {
        if let Some(data) = self {
            data.extend(value)
        } else {
            *self = Some(value)
        }
    }

    fn set(&mut self, value: Vec<T>) {
        *self = Some(value);
    }
}

impl ParseDocument for Document {
    fn parse_document_with_errors(input: &KdlDocument) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        let mut recipes = vec![];
        let mut errors = vec![];

        for node in input.nodes() {
            match node.name().value() {
                "recipe" => {
                    let (recipe, err) = Recipe::parse_node_with_errors(node);
                    if let Some(recipe) = recipe {
                        recipes.push(recipe);
                    }
                    errors.extend(err);
                }

                _ => {}
            }
        }

        (Some(Document { recipes }), errors)
    }
}

impl ParseNode for Recipe {
    fn parse_node_with_errors(input: &KdlNode) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        let mut errors: Vec<HobParseError> = vec![];

        let mut name: String = "<unnamed>".to_string();
        let mut found_version = false;
        let mut version: String = "0.0.0".to_string();
        let mut description: String = "".to_string();
        let mut home: Option<String> = None;
        let mut source_dir: Option<String> = None;
        let mut options: Option<RecipeOptions> = None;
        let mut license: Vec<String> = vec![];
        let mut maintainers: Vec<String> = vec![];
        let mut depends: Vec<String> = vec![];
        let mut provides: Vec<String> = vec![];
        let mut artifacts: Vec<Artifact> = vec![];
        let revision: usize = 0;
        let mut style: Option<BuildStyle> = None;
        let mut playbooks = HashMap::new();

        parse_string_into!(input, name, errors, "name of recipe");
        for node in input.nodes() {
            match node.name().value() {
                "version" => {
                    found_version = true;
                    parse_string_into!(node, version, errors, "version");
                }

                "description" => {
                    parse_string_into!(node, description, errors, "version");
                }

                "home" => {
                    parse_string_into!(node, home, errors, "version");
                }

                "source-dir" => {
                    parse_string_into!(node, source_dir, errors, "source-dir");
                }

                "depends" => {
                    parse_string_list_ext_into!(node, depends, errors, "depends");
                }

                "provides" => {
                    parse_string_list_ext_into!(node, provides, errors, "provides");
                }

                "license" => {
                    parse_string_list_into!(node, license, errors, "depends");
                }

                "maintainer" => {
                    parse_string_list_into!(node, maintainers, errors, "depends");
                }

                "artifacts" => {
                    let (artifacts_opt, err) = Vec::<Artifact>::parse_node_with_errors(node);

                    if let Some(arts) = artifacts_opt {
                        artifacts.extend(arts);
                    }

                    errors.extend(err);
                }

                "style" => {
                    if style.is_some() {
                        errors.push(HobParseError {
                            span: *node.span(),
                            label: Some("second definition of style here"),
                            help: None,
                            kind: "redefinition of style, can only have one build style",
                        });
                        continue;
                    }

                    let (styl, err) = BuildStyle::parse_node_with_errors(node);
                    errors.extend(err);

                    if let Some(styl) = styl {
                        style = Some(styl);
                    }
                }

                "options" => {
                    let (opt, err) = RecipeOptions::parse_node_with_errors(node);
                    errors.extend(err);

                    if let Some(opt) = opt {
                        options = Some(opt);
                    }
                }

                "install" | "prepare" | "build" | "extract" | "configure" => {
                    let (playbook, err) = ActionPlaybook::parse_node_with_errors(node);
                    errors.extend(err);

                    if let Some(playbook) = playbook {
                        playbooks.insert(playbook.stage, playbook);
                    }
                }

                _ => {}
            }
        }

        if !found_version {
            errors.push(HobParseError {
                span: *input.span(),
                label: None,
                help: None,
                kind: "recipe missing version",
            })
        }

        let mut recipe = Recipe {
            source_dir: source_dir.unwrap_or(format!("{}-{}", name, version)),
            name,
            version,
            revision,
            description,
            home,
            license,
            maintainers,
            depends,
            provides,
            artifacts,
            style: style.unwrap_or_default(),
            sides: vec![],
            options: options.unwrap_or_default(),
            playbooks,
        };

        let mut sides = vec![];
        for node in input.nodes() {
            if node.name().value() == "side" {
                let (side, err) = Side::parse_node_with_errors(node, &recipe);
                errors.extend(err);

                if let Some(side) = side {
                    sides.push(side)
                }
            }
        }

        recipe.sides = sides;

        (Some(recipe), errors)
    }
}

impl ParseNode for Vec<Artifact> {
    fn parse_node_with_errors(input: &KdlNode) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        let mut data = vec![];
        let mut errors = vec![];

        for node in input.nodes() {
            let (art, err) = Artifact::parse_node_with_errors(node);
            errors.extend(err);

            if let Some(art) = art {
                data.push(art);
            }
        }

        (Some(data), errors)
    }
}

impl ParseNode for Artifact {
    fn parse_node_with_errors(input: &KdlNode) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        let (verification, mut errors) = Verification::parse_node_with_errors(input);

        let verification = if let Some(ver) = verification {
            ver
        } else {
            return (None, errors);
        };

        let (source, err) = ArtifactSource::parse_node_with_errors(input);
        errors.extend(err);

        let source = if let Some(source) = source {
            source
        } else {
            return (None, errors);
        };

        (
            Some(Artifact {
                source,
                verification,
            }),
            errors,
        )
    }
}

impl ParseNode for ArtifactSource {
    fn parse_node_with_errors(input: &KdlNode) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        match input.name().value() {
            "fetch" => {
                let (obj, err) = FetchArtifact::parse_node_with_errors(input);
                (obj.map(ArtifactSource::Fetch), err)
            }

            _ => (
                None,
                vec![HobParseError {
                    span: *input.name().span(),
                    label: None,
                    help: None,
                    kind: "Unknown type of artifact",
                }],
            ),
        }
    }
}

impl ParseNode for FetchArtifact {
    fn parse_node_with_errors(input: &KdlNode) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        let mut url = None;
        let mut errors = vec![];
        let mut file_name = None;
        for node in input.nodes() {
            match node.name().value() {
                "url" => {
                    parse_string_into!(node, url, errors, "url of artifact");
                }

                "name" => {
                    parse_string_into!(node, file_name, errors, "name of artifact");
                }

                _ => {}
            }
        }

        let res = if let Some(url) = url {
            Some(FetchArtifact {
                file_name: file_name.unwrap_or_else(|| {
                    url.rsplit('/')
                        .next()
                        .unwrap()
                        .split('?')
                        .next()
                        .unwrap()
                        .to_string()
                }),
                url,
            })
        } else {
            errors.push(HobParseError {
                span: *input.span(),
                label: None,
                help: None,
                kind: "fetch artifact requires an url to be given",
            });
            None
        };

        (res, errors)
    }
}

impl ParseNode for Verification {
    fn parse_node_with_errors(input: &KdlNode) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        let mut errors = vec![];
        let mut sha256 = None;
        for node in input.nodes() {
            match node.name().value() {
                "sha256" => {
                    let mut str_sha = None;
                    parse_string_into!(node, str_sha, errors, "sha256");

                    if let Some(str_sha) = str_sha {
                        match hex::decode(str_sha) {
                            Ok(v) if v.len() != 32 => errors.push(HobParseError {
                                span: *node.entries().first().unwrap().span(),
                                label: None,
                                help: None,
                                kind: "expected 32 byte long hex string for sha256",
                            }),
                            Ok(v) => sha256 = Some(v.try_into().unwrap()),
                            Err(v) => errors.push(HobParseError {
                                span: *node.entries().first().unwrap().span(),
                                label: None,
                                help: Some(format!("{}", v)),
                                kind: "invalid hex string",
                            }),
                        }
                    }
                }
                _ => {}
            }
        }

        (Some(Verification { sha256 }), errors)
    }
}

impl ParseNode for BuildStyle {
    fn parse_node_with_errors(input: &KdlNode) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        let mut errors = vec![];
        let mut style: Option<String> = None;
        parse_string_into!(input, style, errors, "name of build style");
        let style = style.and_then(BuildStyleType::parse);

        let style = if let Some(style) = style {
            style
        } else {
            errors.push(HobParseError {
                span: *input.entries().first().unwrap().span(),
                label: None,
                help: None,
                kind: "unknown build style found",
            });

            BuildStyleType::Noop
        };

        let (vars, err) = BuildStyleVariables::parse_node_with_errors(input);
        errors.extend(err);

        (vars.map(|vars| BuildStyle { style, vars }), errors)
    }
}

impl ParseNode for BuildStyleVariables {
    fn parse_node_with_errors(input: &KdlNode) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        let mut vars = BuildStyleVariables::default();
        let mut errors = vec![];

        for node in input.nodes() {
            match node.name().value() {
                "configure-script" => {
                    parse_string_into!(input, vars.configure_script, errors, "configure script");
                }

                "configure-args" => {
                    parse_string_list_ext_into!(
                        input,
                        vars.configure_args,
                        errors,
                        "configure args"
                    );
                }

                _ => {}
            }
        }

        (Some(vars), errors)
    }
}

impl Side {
    fn parse_node_with_errors(
        input: &KdlNode,
        recipe: &Recipe,
    ) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        let mut errors = vec![];
        let mut name = String::new();
        let mut description = recipe.description.clone();
        let mut depends = recipe.depends.clone();
        let mut claims = vec![];
        parse_string_into!(input, name, errors, "name of side");

        let mut errors = vec![];

        for node in input.nodes() {
            match node.name().value() {
                "description" => {
                    parse_string_into!(node, description, errors, "side description");
                }

                "depends" => {
                    parse_string_list_ext_into!(node, depends, errors, "side depends");
                }

                "claim" => {
                    parse_string_list_ext_into!(node, claims, errors, "side claims");
                }

                _ => {}
            }
        }

        (
            Some(Side {
                name,
                description,
                depends,
                claims,
            }),
            errors,
        )
    }
}

impl ParseNode for RecipeOptions {
    fn parse_node_with_errors(input: &KdlNode) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        let mut errors = vec![];
        let mut strip = None;

        for node in input.nodes() {
            match node.name().value() {
                "strip" => {
                    parse_bool_into!(node, strip, errors, "strip");
                }
                _ => {}
            }
        }

        (Some(RecipeOptions { strip }), errors)
    }
}

fn extract_single_bool_value(
    input: &KdlNode,
    missing_error: &'static str,
    wrong_type_error: &'static str,
    too_many_error: &'static str,
    property_found_error: &'static str,
) -> Result<bool, HobParseError> {
    match input.entries().len() {
        0 => Err(HobParseError {
            span: *input.name().span(),
            label: None,
            help: None,
            kind: missing_error,
        }),

        1 => {
            let name_entry = input.entries().first().unwrap();

            if name_entry.name().is_some() {
                return Err(HobParseError {
                    span: *name_entry.span(),
                    label: None,
                    help: None,
                    kind: property_found_error,
                });
            }

            if let Some(v) = name_entry.value().as_bool() {
                Ok(v)
            } else {
                Err(HobParseError {
                    span: *name_entry.span(),
                    label: None,
                    help: None,
                    kind: wrong_type_error,
                })
            }
        }

        _ => {
            let start_args = input.entries().first().unwrap().span().offset();
            let end_args = input
                .entries()
                .last()
                .map(|x| x.span().len() + x.span().offset())
                .unwrap();

            let span = SourceSpan::new(start_args.into(), (end_args - start_args).into());
            Err(HobParseError {
                span,
                label: None,
                help: None,
                kind: too_many_error,
            })
        }
    }
}

pub(crate) fn extract_single_string_value(
    input: &KdlNode,
    missing_error: &'static str,
    wrong_type_error: &'static str,
    too_many_error: &'static str,
    property_found_error: &'static str,
) -> Result<String, HobParseError> {
    match input.entries().len() {
        0 => Err(HobParseError {
            span: *input.name().span(),
            label: None,
            help: None,
            kind: missing_error,
        }),

        1 => {
            let name_entry = input.entries().first().unwrap();

            if name_entry.name().is_some() {
                return Err(HobParseError {
                    span: *name_entry.span(),
                    label: None,
                    help: None,
                    kind: property_found_error,
                });
            }

            if let Some(v) = name_entry.value().as_string() {
                Ok(v.to_string())
            } else {
                Err(HobParseError {
                    span: *name_entry.span(),
                    label: None,
                    help: None,
                    kind: wrong_type_error,
                })
            }
        }

        _ => {
            let start_args = input.entries().first().unwrap().span().offset();
            let end_args = input
                .entries()
                .last()
                .map(|x| x.span().len() + x.span().offset())
                .unwrap();

            let span = SourceSpan::new(start_args.into(), (end_args - start_args).into());
            Err(HobParseError {
                span,
                label: None,
                help: None,
                kind: too_many_error,
            })
        }
    }
}

pub(crate) fn extract_string_values(
    input: &KdlNode,
    wrong_type_error: &'static str,
    property_found_error: &'static str,
) -> Result<Vec<String>, HobParseError> {
    let mut values = vec![];

    for entry in input.entries() {
        if entry.name().is_some() {
            return Err(HobParseError {
                span: *entry.span(),
                label: None,
                help: None,
                kind: property_found_error,
            });
        }

        if let Some(v) = entry.value().as_string() {
            values.push(v.to_string());
        } else {
            return Err(HobParseError {
                span: *entry.span(),
                label: None,
                help: None,
                kind: wrong_type_error,
            });
        }
    }

    Ok(values)
}

pub(crate) fn extract_string_values_with_extend(
    input: &KdlNode,
    wrong_type_error: &'static str,
    property_found_error: &'static str,
) -> Result<(Vec<String>, bool), HobParseError> {
    let mut values = vec![];

    let mut first = true;
    let mut extends = true;

    for entry in input.entries() {
        if first && entry.name().map_or(false, |k| k.value() == "extends") {
            if let Some(v) = entry.value().as_bool() {
                extends = v;
            } else {
                return Err(HobParseError {
                    span: *entry.span(),
                    label: None,
                    help: None,
                    kind: "extends expects a bool",
                });
            }

            continue;
        }

        first = false;

        if entry.name().is_some() {
            return Err(HobParseError {
                span: *entry.span(),
                label: None,
                help: None,
                kind: property_found_error,
            });
        }

        if let Some(v) = entry.value().as_string() {
            values.push(v.to_string());
        } else {
            return Err(HobParseError {
                span: *entry.span(),
                label: None,
                help: None,
                kind: wrong_type_error,
            });
        }
    }

    Ok((values, extends))
}
