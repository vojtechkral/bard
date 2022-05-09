use std::collections::HashMap;
use std::fs;
use std::io;
use std::sync::{Arc, Mutex};

use camino::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use handlebars::{self as hb, handlebars_helper, Handlebars, HelperDef, JsonValue};
use image::image_dimensions;
use lazy_static::lazy_static;
use regex::{Error as ReError, Regex};
use semver::Version;

use super::{Render, RenderContext};
use crate::error::*;
use crate::project::{Output, Project};
use crate::util::PathBufExt;

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
        &self,
        h: &hb::Helper<'reg, 'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc hb::Context,
        _: &mut hb::RenderContext<'reg, 'rc>,
    ) -> Result<hb::ScopedJson<'reg, 'rc>, hb::RenderError> {
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
                self.name, pathbuf, e
            ))
        })?;

        let res = [w, h][self.result_i];
        Ok(hb::ScopedJson::Derived(JsonValue::from(res)))
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
        &self,
        h: &hb::Helper<'reg, 'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc hb::Context,
        _: &mut hb::RenderContext<'reg, 'rc>,
    ) -> Result<hb::ScopedJson<'reg, 'rc>, hb::RenderError> {
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
        Ok(hb::ScopedJson::Derived(JsonValue::from(res)))
    }
}

struct VersionCheckHelper {
    version: Arc<Mutex<Option<Version>>>,
}

impl VersionCheckHelper {
    const FN_NAME: &'static str = "version_check";

    fn new() -> (Box<Self>, Arc<Mutex<Option<Version>>>) {
        let version = Arc::new(Mutex::new(None));
        let this = Box::new(Self {
            version: version.clone(),
        });
        (this, version)
    }
}

impl HelperDef for VersionCheckHelper {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        h: &hb::Helper<'reg, 'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc hb::Context,
        _: &mut hb::RenderContext<'reg, 'rc>,
    ) -> Result<hb::ScopedJson<'reg, 'rc>, hb::RenderError> {
        let version = h
            .param(0)
            .map(|x| x.value())
            .ok_or_else(|| {
                hb::RenderError::new(format!("{}: No version number supplied", Self::FN_NAME))
            })
            .and_then(|x| match x {
                JsonValue::String(s) => Ok(s.as_str()),
                _ => Err(hb::RenderError::new(format!(
                    "{}: Input value not a string",
                    Self::FN_NAME
                ))),
            })
            .and_then(|s| {
                Version::parse(s).map_err(|e| {
                    hb::RenderError::from_error(
                        &format!("{}: Could not parse version `{}`", Self::FN_NAME, s),
                        e,
                    )
                })
            })?;

        *self.version.lock().unwrap() = Some(version);

        Ok(hb::ScopedJson::Derived(JsonValue::String(String::new())))
    }
}

#[derive(Debug)]
struct HbRender<'a> {
    hb: Handlebars<'static>,
    tpl_name: String,
    project: &'a Project,
    output: &'a Output,
    default_content: &'static str,
    version: Arc<Mutex<Option<Version>>>,
}

impl<'a> HbRender<'a> {
    /// Version of the template to assume if it specifies none.
    const ASSUMED_FIRST_VERSION: Version = Version::new(1, 0, 0);

    fn new<DT: DefaultTemaplate>(project: &'a Project, output: &'a Output) -> Self {
        let mut hb = Handlebars::new();
        let (version_helper, version) = VersionCheckHelper::new();
        hb.register_helper("eq", Box::new(hb_eq));
        hb.register_helper("contains", Box::new(hb_contains));
        hb.register_helper("default", Box::new(hb_default));
        hb.register_helper("matches", Box::new(hb_matches));
        hb.register_helper("px2mm", DpiHelper::new(output));
        hb.register_helper("img_w", ImgHelper::width(project));
        hb.register_helper("img_h", ImgHelper::height(project));
        hb.register_helper(VersionCheckHelper::FN_NAME, version_helper);

        let tpl_name = output
            .template
            .as_ref()
            .map(|t| t.to_string())
            .unwrap_or_else(|| DT::TPL_NAME.to_string());

        Self {
            hb,
            tpl_name,
            project,
            output,
            default_content: DT::TPL_CONTENT,
            version,
        }
    }

    fn load(&mut self) -> Result<Version> {
        if let Some(template) = self.output.template.as_ref() {
            if template.exists() {
                self.hb
                    .register_template_file(&self.tpl_name, &template)
                    .with_context(|| format!("Error in template file `{}`", template))?;
            } else {
                let parent = template.parent().unwrap(); // The temaplate should've been resolved as absolute in Project
                fs::create_dir_all(parent)
                    .and_then(|_| fs::write(&template, self.default_content.as_bytes()))
                    .with_context(|| {
                        format!("Error writing default template to file: `{}`", template)
                    })?;

                self.hb
                    .register_template_string(&self.tpl_name, self.default_content)
                    .expect("Internal error: Could not load default template");
            }
        } else {
            self.hb
                .register_template_string(&self.tpl_name, self.default_content)
                .expect("Internal error: Could not load default template");
        }

        // Render with no data to an IO Sink.
        // This will certainly fail, but if the version_check() helper is used on top
        // of the template, we will get the version in self.version.
        let _ = self.hb.render_to_write(&self.tpl_name, &(), io::sink());
        let version = self
            .version
            .lock()
            .unwrap()
            .clone()
            .unwrap_or(Self::ASSUMED_FIRST_VERSION);
        Ok(version)
    }

    fn render(&self) -> Result<()> {
        let context = RenderContext::new(self.project, self.output);
        let output = self.hb.render(&self.tpl_name, &context)?;

        fs::write(&self.output.file, output.as_bytes())
            .with_context(|| format!("Error writing output file: `{}`", self.output.file))?;

        Ok(())
    }
}

pub struct RHtml<'a>(HbRender<'a>);

impl<'a> DefaultTemaplate for RHtml<'a> {
    const TPL_NAME: &'static str = "html.hbs";
    const TPL_CONTENT: &'static str = include_str!("./templates/html.hbs");
}

impl<'a> Render<'a> for RHtml<'a> {
    fn new(project: &'a Project, output: &'a Output) -> Self {
        Self(HbRender::new::<Self>(project, output))
    }

    fn load(&mut self) -> Result<Option<Version>> {
        self.0.load().map(Some)
    }

    fn render(&self) -> Result<()> {
        self.0.render()
    }
}

pub struct RTex<'a>(HbRender<'a>);

impl<'a> DefaultTemaplate for RTex<'a> {
    const TPL_NAME: &'static str = "pdf.hbs";
    const TPL_CONTENT: &'static str = include_str!("./templates/pdf.hbs");
}

impl<'a> Render<'a> for RTex<'a> {
    fn new(project: &'a Project, output: &'a Output) -> Self {
        let mut render = HbRender::new::<Self>(project, output);

        // Setup Latex escaping
        render.hb.register_escape_fn(hb_latex_escape);
        render.hb.register_helper("pre", Box::new(hb_pre));

        Self(render)
    }

    fn load(&mut self) -> Result<Option<Version>> {
        self.0.load().map(Some)
    }

    fn render(&self) -> Result<()> {
        self.0.render()
    }
}

pub struct RHovorka<'a>(HbRender<'a>);

impl<'a> DefaultTemaplate for RHovorka<'a> {
    const TPL_NAME: &'static str = "hovorka.hbs";
    const TPL_CONTENT: &'static str = include_str!("./templates/hovorka.hbs");
}

impl<'a> Render<'a> for RHovorka<'a> {
    fn new(project: &'a Project, output: &'a Output) -> Self {
        Self(HbRender::new::<Self>(project, output))
    }

    fn load(&mut self) -> Result<Option<Version>> {
        self.0.load().map(Some)
    }

    fn render(&self) -> Result<()> {
        self.0.render()
    }
}
