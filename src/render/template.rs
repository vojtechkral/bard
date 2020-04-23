use std::fs;
use std::collections::HashMap;

use tera::{self, Tera, Context, Value};
use serde_json::Map as JsonMap;

use crate::project::{Project, OutputSpec};
use crate::PROGRAM_META;
use crate::error::*;
use super::Render;


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

fn latex_tera_escaper(input: &str) -> String {
    latex_escape(input, false)
}

fn filter_latex_inner(
    val: &Value, args: &HashMap<String, Value>, pre_spaces: bool,
) -> tera::Result<Value> {
    match val {
        Value::String(s) => Ok(Value::String(latex_escape(&s, pre_spaces))),
        Value::Array(array) => {
            let mut escaped = Vec::with_capacity(array.len());
            for item in array {
                escaped.push(filter_latex_inner(item, args, pre_spaces)?);
            }
            Ok(Value::Array(escaped))
        }
        Value::Object(map) => {
            let mut escaped = JsonMap::new();
            for (key, value) in map.iter() {
                let value = filter_latex_inner(value, args, pre_spaces)?;
                dbg!(&value);
                escaped.insert(latex_escape(key, pre_spaces), value);
            }
            Ok(Value::Object(escaped))
        }
        other => Ok(other.clone()),
    }
}

fn filter_latex(val: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
    filter_latex_inner(val, args, false)
}

fn filter_pre(val: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
    filter_latex_inner(val, args, true)
}

fn filter_base64(val: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    match val {
        Value::String(s) => {
            let encoded = base64::encode(s.as_bytes());

            // Insert newlines at column 80:
            let mut cursor = encoded.as_str();
            let mut encoded_newlines = String::with_capacity(81 * encoded.len() / 80);
            while cursor.len() > 80 {
                let (p1, p2) = cursor.split_at(80);
                encoded_newlines.push_str(p1);
                encoded_newlines.push('\n');
                cursor = p2;
            }
            encoded_newlines.push_str(cursor);

            Ok(Value::String(encoded_newlines))
        }
        _ => Err(tera::Error::msg("base64 requires a string input")),
    }
}

pub trait DefaultTemaplate {
    const TPL_NAME: &'static str;
    const TPL_CONTENT: &'static str;
}

struct TeraRender<'a> {
    tera: Tera,
    tpl_name: String,
    project: &'a Project,
    output: &'a OutputSpec,
}

impl<'a> TeraRender<'a> {
    fn new<DT: DefaultTemaplate>(project: &'a Project, output: &'a OutputSpec) -> Result<Self> {
        let mut tera = Tera::default();

        let tpl_name = if let Some(template) = output.template.as_ref() {
            if !template.exists() {
                // Initialize the template file with default contents
                fs::write(&template, DT::TPL_CONTENT)?;
            }

            tera.add_template_file(&template, None)
                .context("Tera template error")?;

            template.to_str().unwrap().to_string()
        // NB: ^ unwrap should be ok, UTF-8 validity is checked while parsing
        // project settings TOML
        } else {
            tera.add_raw_template(DT::TPL_NAME, DT::TPL_CONTENT)
                .expect("Internal error: Could not load default Tera template");
            DT::TPL_NAME.to_string()
        };

        Ok(Self {
            tera,
            tpl_name,
            project,
            output,
        })
    }

    fn render(&self) -> Result<&'a OutputSpec> {
        let mut context = Context::new();
        context.insert("book", self.project.metadata());
        context.insert("songs", self.project.songs());
        context.insert("output", &self.output.metadata);
        context.insert("program", &PROGRAM_META);
        if let Some(debug) = self.project.parsing_debug() {
            context.insert("debug", debug);
        }

        let html = self.tera.render(&self.tpl_name, &context)?;

        fs::write(&self.output.file, html.as_bytes())
            .map_err(|err| ErrorWritingFile(self.output.file.to_owned(), err))?;

        Ok(self.output)
    }
}

pub struct RHtml;

impl DefaultTemaplate for RHtml {
    const TPL_NAME: &'static str = "template-html.html";
    const TPL_CONTENT: &'static str = include_str!("../../default/template-html.html");
}

impl Render for RHtml {
    fn render<'a>(project: &'a Project, output: &'a OutputSpec) -> Result<&'a OutputSpec> {
        let render = TeraRender::new::<Self>(project, output)?;
        render.render()
    }
}

pub struct RTex;

impl DefaultTemaplate for RTex {
    const TPL_NAME: &'static str = "template-tex.tex";
    const TPL_CONTENT: &'static str = include_str!("../../default/template-tex.tex");
}

impl Render for RTex {
    fn render<'a>(project: &'a Project, output: &'a OutputSpec) -> Result<&'a OutputSpec> {
        let mut render = TeraRender::new::<Self>(project, output)?;

        // Setup Latex escaping
        render.tera.set_escape_fn(latex_tera_escaper);
        render.tera.autoescape_on(vec![".tex"]);
        render.tera.register_filter("latex", filter_latex);
        render.tera.register_filter("pre", filter_pre);
        render.tera.register_filter("base64", filter_base64);

        render.render()
    }
}
