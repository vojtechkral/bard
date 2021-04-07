use std::fs;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use handlebars::{self as hb, Handlebars, HelperDef, JsonValue, handlebars_helper};
use regex::{Regex, Error as ReError};
use image::image_dimensions;
use lazy_static::lazy_static;
use serde::Serialize;

use crate::book::Song;
use crate::project::{Metadata, Output, Project};
use crate::util::PathBufExt;
use crate::{PROGRAM_META, ProgramMeta};
use crate::error::*;
use super::Render;

type RegexCache = HashMap<String, Result<Regex, ReError>>;

lazy_static! {
    static ref REGEX_CACHE: Mutex<RegexCache> = Mutex::new(RegexCache::new());
}

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

handlebars_helper!(hb_eq: |v1: Json, v2: Json| {
    v1 == v2
});

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

handlebars_helper!(hb_matches: |value: str, regex: str| {
    let mut cache = REGEX_CACHE.lock().unwrap();

    if !cache.contains_key(regex) {
        let res = Regex::new(regex);
        if res.is_err() {
            eprintln!("Warning: `matches` helper: Invalid regular expression: `{}`", regex);
        }
        cache.insert(regex.into(), res);
    }

    match cache.get(regex) {
        Some(Ok(re)) => re.is_match(value),
        _ => false,
    }
});

struct ImgHelper {
    out_dir: PathBuf,
    result_i: usize,
    name: &'static str,
}

impl ImgHelper {
    fn width(project: &Project) -> Box<Self> {
        let out_dir = project.settings.dir_output().to_owned();
        Box::new(Self {
            out_dir,
            result_i: 0,
            name: "img_w",
        })
    }
    fn height(project: &Project) -> Box<Self> {
        let out_dir = project.settings.dir_output().to_owned();
        Box::new(Self {
            out_dir,
            result_i: 1,
            name: "img_h",
        })
    }
}

impl HelperDef for ImgHelper {
    fn call_inner<'reg: 'rc, 'rc>(
        &self, h: &hb::Helper<'reg, 'rc>, _: &'reg Handlebars<'reg>, _: &'rc hb::Context,
        _: &mut hb::RenderContext<'reg, 'rc>,
    ) -> Result<Option<hb::ScopedJson<'reg, 'rc>>, hb::RenderError> {
        let path: &str = h
            .param(0)
            .map(|x| x.value())
            .ok_or_else(|| hb::RenderError::new(format!("{}: Image path not supplied", self.name)))
            .and_then(|x| {
                x.as_str().ok_or_else(|| {
                    hb::RenderError::new(&format!(
                        "{}: Image path not a string, it's {:?} as JSON.",
                        self.name, x,
                    ))
                })
            })?;

        let pathbuf = Path::new(&path).to_owned().resolved(&self.out_dir);
        let (w, h) = image_dimensions(&pathbuf).map_err(|e| {
            hb::RenderError::new(&format!(
                "{}: Couldn't read image at `{}`: {}",
                self.name,
                pathbuf.display(),
                e
            ))
        })?;

        let res = [w, h][self.result_i];
        Ok(Some(hb::ScopedJson::Derived(JsonValue::from(res))))
    }
}

struct DpiHelper {
    dpi: f64,
}

impl DpiHelper {
    const INCH_MM: f64 = 25.4;

    fn new(output: &Output) -> Box<Self> {
        Box::new(Self { dpi: output.dpi() })
    }
}

impl HelperDef for DpiHelper {
    fn call_inner<'reg: 'rc, 'rc>(
        &self, h: &hb::Helper<'reg, 'rc>, _: &'reg Handlebars<'reg>, _: &'rc hb::Context,
        _: &mut hb::RenderContext<'reg, 'rc>,
    ) -> Result<Option<hb::ScopedJson<'reg, 'rc>>, hb::RenderError> {
        let value: f64 = h
            .param(0)
            .map(|x| x.value())
            .ok_or_else(|| hb::RenderError::new("px2mm: Input value not supplied"))
            .and_then(|x| {
                x.as_f64().ok_or_else(|| {
                    hb::RenderError::new(&format!(
                        "px2mm: Input value not a number, it's {:?} as JSON.",
                        x,
                    ))
                })
            })?;

        let res = (value / self.dpi) * Self::INCH_MM;
        Ok(Some(hb::ScopedJson::Derived(JsonValue::from(res))))
    }
}

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
        hb.register_helper("eq", Box::new(hb_eq));
        hb.register_helper("contains", Box::new(hb_contains));
        hb.register_helper("default", Box::new(hb_default));
        hb.register_helper("matches", Box::new(hb_matches));
        hb.register_helper("px2mm", DpiHelper::new(output));
        hb.register_helper("img_w", ImgHelper::width(project));
        hb.register_helper("img_h", ImgHelper::height(project));

        let tpl_name = if let Some(template) = output.template.as_ref() {
            // NB: unwrap() should be ok, UTF-8 validity is checked while parsing
            // project settings TOML:
            let tpl_name = template.to_str().unwrap().to_string();

            if template.exists() {
                hb.register_template_file(&tpl_name, &template)
                    .with_context(|| format!("Error in template file `{}`", template.display()))?;
            } else {
                fs::write(&template, DT::TPL_CONTENT.as_bytes()).with_context(|| {
                    format!(
                        "Error writing default template to file: `{}`",
                        template.display()
                    )
                })?;

                hb.register_template_string(&tpl_name, DT::TPL_CONTENT)
                    .expect("Internal error: Could not load default template");
            }

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

        fs::write(&self.output.file, html.as_bytes()).with_context(|| {
            format!(
                "Error writing output file: `{}`",
                self.output.file.display()
            )
        })?;

        Ok(self.output)
    }
}

pub struct RHtml;

impl DefaultTemaplate for RHtml {
    const TPL_NAME: &'static str = "html.hbs";
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


pub struct RHovorka;

impl DefaultTemaplate for RHovorka {
    const TPL_NAME: &'static str = "hovorka.hbs";
    const TPL_CONTENT: &'static str = include_str!("../../example/templates/hovorka.hbs");
}

impl Render for RHovorka {
    fn render<'a>(project: &'a Project, output: &'a Output) -> Result<&'a Output> {
        let render = HbRender::new::<Self>(project, output)?;
        render.render()
    }
}
