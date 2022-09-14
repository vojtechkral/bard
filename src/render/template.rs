use std::collections::HashMap;
use std::{env, fmt};
use std::fs;
use std::io;
use std::sync::{Arc, Mutex};

use tectonic;
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

pub struct DefaultTemaplate {
    pub filename: &'static str,
    pub content: &'static str,
}

macro_rules! declare_default_templates {
    ($all_name:ident : [ $(($name:ident, $filename:expr),)+ ]) => {
        $(pub static $name: DefaultTemaplate = DefaultTemaplate {
            filename: $filename,
            content: include_str!(concat!("./templates/", $filename)),
        };)+

        pub static $all_name: &'static [ &'static DefaultTemaplate ] = &[
            $(&$name,)+
        ];
    };
}

declare_default_templates!(
    DEFAULT_TEMPLATES: [
        (DEFAULT_TEMPLATE_TEX, "pdf.hbs"),
        (DEFAULT_TEMPLATE_HTML, "html.hbs"),
        (DEFAULT_TEMPLATE_HOVORKA, "hovorka.hbs"),
    ]
);

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

struct Cat<'a>(Vec<&'a JsonValue>);

impl<'a> fmt::Display for Cat<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for arg in self.0.iter() {
            match arg {
                JsonValue::Null => write!(f, "[null]")?,
                JsonValue::Bool(b) => write!(f, "{}", b)?,
                JsonValue::Number(n) => write!(f, "{}", n)?,
                JsonValue::String(s) => write!(f, "{}", s)?,
                JsonValue::Array(..) => write!(f, "[array]")?,
                JsonValue::Object(..) => write!(f, "[object]")?,
            }
        }
        Ok(())
    }
}

handlebars_helper!(hb_cat: |*args| {
    format!("{}", Cat(args))
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

    fn new(project: &'a Project, output: &'a Output, default: &DefaultTemaplate) -> Self {
        let mut hb = Handlebars::new();
        let (version_helper, version) = VersionCheckHelper::new();
        hb.register_helper("eq", Box::new(hb_eq));
        hb.register_helper("contains", Box::new(hb_contains));
        hb.register_helper("cat", Box::new(hb_cat));
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
            .unwrap_or_else(|| default.filename.to_string());

        Self {
            hb,
            tpl_name,
            project,
            output,
            default_content: default.content,
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

    /// Renders template and returns resulting bytes. In case it was a text file, you can get the string by using [`String::from_utf8`].
    fn render_bytes(&self) -> Result<Vec<u8>> {
        let context = RenderContext::new(self.project, self.output);
        let output = self.hb.render(&self.tpl_name, &context)?;

        Ok(output.into_bytes())
    }

    /// Save bytes to the output file
    fn save(&self, bytes: Vec<u8>) -> Result<()> {
        fs::write(&self.output.file, bytes)
            .with_context(|| format!("Error writing output file: `{}`", self.output.file))
    }

    /// Renders template and saves to file
    fn render(&self) -> Result<()> {
        self.save(self.render_bytes()?)
    }
}

pub struct RHtml<'a>(HbRender<'a>);

impl<'a> Render<'a> for RHtml<'a> {
    fn new(project: &'a Project, output: &'a Output) -> Self {
        Self(HbRender::new(project, output, &DEFAULT_TEMPLATE_HTML))
    }

    fn load(&mut self) -> Result<Option<Version>> {
        self.0.load().map(Some)
    }

    fn render(&self) -> Result<()> {
        self.0.render()
    }
}

pub struct RTex<'a>(HbRender<'a>);

impl<'a> Render<'a> for RTex<'a> {
    fn new(project: &'a Project, output: &'a Output) -> Self {
        let mut render = HbRender::new(project, output, &DEFAULT_TEMPLATE_TEX);

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

pub struct RPdf<'a>(RTex<'a>);

impl<'a> Render<'a> for RPdf<'a> {
    fn new(project: &'a Project, output: &'a Output) -> Self {
        Self(RTex::new(project, output))
    }

    fn load(&mut self) -> Result<Option<Version>> {
        self.0.load()
    }

    fn render(&self) -> Result<()> {
        //Render LaTeX
        let latex = String::from_utf8(self.0.0.render_bytes()?)?;
        // change working directory to `output` so that relative paths are same as when rendering LaTeX and then compiling it externally with XeLaTeX
        let path = env::current_dir()?;
        let mut output_path = path.clone();
        output_path.push("output");
        fs::create_dir_all(&output_path).with_context(|| "Cannot create output directory. Make sure you have permission to create directories here.")?;
        env::set_current_dir(&output_path).expect("WTF?! this should never happen.");
        // Compile LaTeX to pdf with Tectonic
        let res = match tectonic::latex_to_pdf(latex) {
            Ok(pdf) => {
                self.0.0.save(pdf)
            }
            Err(_e) => {
                //TODO provide more useful error
                Err(anyhow!(TectonicError {}))
            }
        };
        //change working directory back
        env::set_current_dir(&path).expect("WTF?! this should never happen.");

        return res
    }
}

/// Custom error type used when tectonic library returns an error. TODO add more useful details to the error
#[derive(Debug)]
struct TectonicError{ }

impl std::error::Error for TectonicError {}

impl fmt::Display for TectonicError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error creating PDF from Tex. Make sure the Tex (LaTeX) code is valid. TIP: Generate a .tex file instead of .pdf and try compiling it with TexLive or XeLaTeX. It may give you more detailed information about what went wrong.")
    }
}

pub struct RHovorka<'a>(HbRender<'a>);

impl<'a> Render<'a> for RHovorka<'a> {
    fn new(project: &'a Project, output: &'a Output) -> Self {
        Self(HbRender::new(project, output, &DEFAULT_TEMPLATE_HOVORKA))
    }

    fn load(&mut self) -> Result<Option<Version>> {
        self.0.load().map(Some)
    }

    fn render(&self) -> Result<()> {
        self.0.render()
    }
}
