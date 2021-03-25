use std::fs;

use handlebars::{Handlebars, JsonValue, handlebars_helper};

use crate::book::Song;
use crate::project::{Metadata, Output, Project};
use crate::{PROGRAM_META, ProgramMeta};
use crate::error::*;
use super::Render;


pub trait DefaultTemaplate {
    const TPL_NAME: &'static str;
    const TPL_CONTENT: &'static str;
}

fn latex_escape(input: &str, pre_spaces: bool) -> String {
    let mut res = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            ' ' if pre_spaces => res.push('~'),
            '&' | '%' | '$' | '#' | '_' | '{' | '}' => {
                res.push('\\');
                res.push(c);
            }
            '[' => res.push_str("{\\lbrack}"),
            ']' => res.push_str("{\\rbrack}"),
            '~' => res.push_str("{\\textasciitilde}"),
            '^' => res.push_str("{\\textasciicircum}"),
            '\\' => res.push_str("{\\textbackslash}"),
            c => res.push(c),
        }
    }

    res
}

fn hb_latex_escape(input: &str) -> String {
    latex_escape(input, false)
}

handlebars_helper!(hb_contains: |obj: object, key: str| {
    obj.contains_key(key)
});

handlebars_helper!(hb_default: |value: Json, def: Json| {
    match value {
        JsonValue::Null => def.clone(),
        other => other.clone(),
    }
});

handlebars_helper!(hb_pre: |input: str| {
    latex_escape(input, true)
});

#[derive(Serialize, Debug)]
struct HbContext<'a> {
    book: &'a Metadata,
    songs: &'a [Song],
    output: &'a Metadata,
    program: &'a ProgramMeta,
}

#[derive(Debug)]
struct HbRender<'a> {
    hb: Handlebars<'static>,
    tpl_name: String,
    project: &'a Project,
    output: &'a Output,
}

impl<'a> HbRender<'a> {
    fn new<DT: DefaultTemaplate>(project: &'a Project, output: &'a Output) -> Result<Self> {
        let mut hb = Handlebars::new();
        hb.register_helper("contains", Box::new(hb_contains));
        hb.register_helper("default", Box::new(hb_default));

        let tpl_name = if let Some(template) = output.template.as_ref() {
            // NB: unwrap() should be ok, UTF-8 validity is checked while parsing
            // project settings TOML:
            let tpl_name = template.to_str().unwrap().to_string();

            hb.register_template_file(&tpl_name, &template)
                .with_context(|| format!("Error in template file `{}`", template.display()))?;

            tpl_name
        } else {
            hb.register_template_string(DT::TPL_NAME, DT::TPL_CONTENT)
                .expect("Internal error: Could not load default template");
            DT::TPL_NAME.to_string()
        };

        Ok(Self {
            hb,
            tpl_name,
            project,
            output,
        })
    }

    fn render(&self) -> Result<&'a Output> {
        let context = HbContext {
            book: self.project.metadata(),
            songs: self.project.songs(),
            output: &self.output.metadata,
            program: &PROGRAM_META,
        };

        let html = self.hb.render(&self.tpl_name, &context)?;

        fs::write(&self.output.file, html.as_bytes())
            .map_err(|err| ErrorWritingFile(self.output.file.to_owned(), err))?;

        Ok(self.output)
    }
}

pub struct RHtml;

impl DefaultTemaplate for RHtml {
    const TPL_NAME: &'static str = "html.hbs";
    // FIXME: Real file
    const TPL_CONTENT: &'static str = include_str!("../../default/templates/html.hbs");
}

impl Render for RHtml {
    fn render<'a>(project: &'a Project, output: &'a Output) -> Result<&'a Output> {
        let render = HbRender::new::<Self>(project, output)?;
        render.render()
    }
}


pub struct RTex;

impl DefaultTemaplate for RTex {
    const TPL_NAME: &'static str = "pdf.hbs";
    // FIXME: Real file
    const TPL_CONTENT: &'static str = include_str!("../../default/templates/pdf.hbs");
}

impl Render for RTex {
    fn render<'a>(project: &'a Project, output: &'a Output) -> Result<&'a Output> {
        let mut render = HbRender::new::<Self>(project, output)?;

        // Setup Latex escaping
        render.hb.register_escape_fn(hb_latex_escape);
        render.hb.register_helper("pre", Box::new(hb_pre));

        render.render()
    }
}
