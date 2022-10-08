extern crate core;

use crate::definition::parsing::{HobParserCompoundError, ParseDocument};
use crate::definition::{Document, Recipe, RecipeTemplate};
use crate::engine::packager::Apk;
use crate::engine::Engine;
use handlebars::Handlebars;
use hob_utils::{ObjectTraversal, ObjectWalker};
use kdl::KdlDocument;
use miette::NamedSource;

mod definition;
mod engine;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let musl = include_str!("../examples/musl.kdl");
    let kdl_document: KdlDocument = musl.parse()?;
    let (document, errors) = Document::parse_document_with_errors(&kdl_document);

    if !errors.is_empty() {
        let error = miette::Error::new(HobParserCompoundError {
            source_code: NamedSource::new("musl.kdl", musl),
            errors,
        });

        println!("{:?}", error);
    }

    let document = document.map(|mut x| {
        x.recipes.iter_mut().for_each(|y| {
            let vars = y.template_vars();
            y.traverse(&mut TemplateReplace {
                engine: Default::default(),
                vars,
            })
        });
        x
    });

    println!("{:#?}", document);

    if let Some(doc) = document {
        let engine = Engine::new::<Apk>();

        engine.prepare_engine().await?;
        engine.build_recipe(&doc.recipes[0]).await?;
    }

    Ok(())
}

pub struct TemplateReplace<'a> {
    engine: Handlebars<'a>,
    vars: RecipeTemplate,
}

impl ObjectWalker for TemplateReplace<'_> {
    fn enter_string(&mut self, value: &mut String) {
        *value = self.engine.render_template(value, &self.vars).unwrap();
    }
}
