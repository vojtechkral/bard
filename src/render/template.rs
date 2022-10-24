use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use camino::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use handlebars::{self as hb, handlebars_helper, Handlebars, HelperDef, JsonValue, RenderError};
use image::image_dimensions;
use once_cell::sync::Lazy;
use regex::{Error as ReError, Regex};
use semver::Version;
use serde_json::Number;

use super::RenderContext;
use crate::prelude::*;
use crate::project::{Output, Project};
use crate::util::PathBufExt;

type RegexCache = HashMap<String, Result<Regex, ReError>>;

static REGEX_CACHE: Lazy<Mutex<RegexCache>> = Lazy::new(|| Mutex::new(RegexCache::new()));

// Default templates

pub struct DefaultTemaplate {
    pub filename: &'static str,
    pub content: &'static str,
}

macro_rules! default_template {
    ($name:ident, $filename:expr) => {
        pub static $name: crate::render::template::DefaultTemaplate =
            crate::render::template::DefaultTemaplate {
                filename: $filename,
                content: include_str!(concat!("./templates/", $filename)),
            };
    };
}

// HB helpers

macro_rules! hb_err {
    ($msg:literal) => {
        RenderError::new($msg)
    };

    ($fmt:literal, $($field:expr),+) => {
        RenderError::new(format!($fmt, $($field),+))
    };

    ($e:ident, $fmt:literal, $($field:expr),+) => {
        RenderError::from_error(&format!($fmt, $($field),+), $e)
    };
}

trait HandlebarsExt {
    fn with_helper<T>(self, name: &str, helper: T) -> Self
    where
        T: HelperDef + Send + Sync + 'static;
}

impl HandlebarsExt for Handlebars<'static> {
    fn with_helper<T>(mut self, name: &str, helper: T) -> Self
    where
        T: HelperDef + Send + Sync + 'static,
    {
        self.register_helper(name, Box::new(helper));
        self
    }
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
    fn width(project: &Project) -> Self {
        let out_dir = project.settings.dir_output().to_owned();
        Self {
            out_dir,
            result_i: 0,
            name: "img_w",
        }
    }
    fn height(project: &Project) -> Self {
        let out_dir = project.settings.dir_output().to_owned();
        Self {
            out_dir,
            result_i: 1,
            name: "img_h",
        }
    }
}

impl HelperDef for ImgHelper {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        h: &hb::Helper<'reg, 'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc hb::Context,
        _: &mut hb::RenderContext<'reg, 'rc>,
    ) -> Result<hb::ScopedJson<'reg, 'rc>, RenderError> {
        let path: &str = h
            .param(0)
            .map(|x| x.value())
            .ok_or_else(|| hb_err!("{}: Image path not supplied", self.name))
            .and_then(|x| {
                x.as_str().ok_or_else(|| {
                    hb_err!(
                        "{}: Image path not a string, it's {:?} as JSON.",
                        self.name,
                        x
                    )
                })
            })?;

        let pathbuf = Path::new(&path).to_owned().resolved(&self.out_dir);
        let (w, h) = image_dimensions(&pathbuf)
            .map_err(|e| hb_err!(e, "{}: Couldn't read image at `{}`", self.name, pathbuf))?;

        let res = [w, h][self.result_i];
        Ok(hb::ScopedJson::Derived(JsonValue::from(res)))
    }
}

struct DpiHelper {
    dpi: f64,
}

impl DpiHelper {
    const INCH_MM: f64 = 25.4;

    fn new(output: &Output) -> Self {
        Self { dpi: output.dpi() }
    }
}

impl HelperDef for DpiHelper {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        h: &hb::Helper<'reg, 'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc hb::Context,
        _: &mut hb::RenderContext<'reg, 'rc>,
    ) -> Result<hb::ScopedJson<'reg, 'rc>, RenderError> {
        let value: f64 = h
            .param(0)
            .map(|x| x.value())
            .ok_or_else(|| hb_err!("px2mm: Input value not supplied"))
            .and_then(|x| {
                x.as_f64().ok_or_else(|| {
                    hb_err!("px2mm: Input value not a number, it's {:?} as JSON.", x)
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
    fn new() -> (Self, Arc<Mutex<Option<Version>>>) {
        let version = Arc::new(Mutex::new(None));
        let this = Self {
            version: version.clone(),
        };
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
    ) -> Result<hb::ScopedJson<'reg, 'rc>, RenderError> {
        let version = h
            .param(0)
            .map(|x| x.value())
            .ok_or_else(|| hb_err!("version_check: No version number supplied"))
            .and_then(|x| match x {
                JsonValue::String(s) => Ok(s.as_str()),
                _ => Err(hb_err!("version_check: Input value not a string")),
            })
            .and_then(|s| {
                Version::parse(s)
                    .map_err(|e| hb_err!(e, "version_check: Could not parse version `{}`", s))
            })?;

        *self.version.lock().unwrap() = Some(version);
        Ok(hb::ScopedJson::Derived(JsonValue::String(String::new())))
    }
}

/// Simple math helper, which can do the usual arithmetic operations on integers and floats. Tries to handle most edge-cases and provide useful error message to the user.
///
/// Usage: `{{ math 5 "+" 3 }}`, `{{ math 23.8 / -1.5}}`, `{{ math "3" "*" 8.5 }}`
///
/// Supported operations:
///    - \+ addition
///    - \- subtraction
///    - \* multiplication
///    - / **decimal** division (integers are converted to floats)
///    - // integer division (both numbers must be integers)
///    - % modulo
///    - & bitwise and (integers only)
///    - | bitwise or (integers only)
///    - ^ bitwise xor (integers only)
///    - << bitwise shift left (integers only)
///    - \>> bitwise shift right (integers only)
struct MathHelper;

impl MathHelper {
    fn hb_math_int(a: i64, operation: &str, b: i64) -> Result<i64, RenderError> {
        Ok(match operation {
            "+" => a + b,
            "-" => a - b,
            "*" => a * b,
            "//" => a / b, // normal division is done using floats to make it simples for inexperienced users. For integer division, use //.
            "%" => a % b,
            "&" => a & b,
            "|" => a | b,
            "^" => a ^ b,
            "<<" => a << b,
            ">>" => a >> b,
            _ => return Err(hb_err!("math: Operation \"{}\" is not possible with integers. Available operations on integers: +, -, *, /, //, %, &, |, ^, <<, >>", operation)),
        })
    }

    fn hb_math_float(a: f64, operation: &str, b: f64) -> Result<f64, RenderError> {
        Ok(match operation {
            "+" => a + b,
            "-" => a - b,
            "*" => a * b,
            "/" => a / b,
            "%" => a % b,
            _ => return Err(hb_err!("math: Operation \"{}\" is not possible with a decimal number. Available operations: +, -, *, /, %. (Also //, |, ^, <<, >>, but only if both numbers are integers)", operation)),
        })
    }
}

impl HelperDef for MathHelper {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        h: &hb::Helper<'reg, 'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc hb::Context,
        _: &mut hb::RenderContext<'reg, 'rc>,
    ) -> Result<hb::ScopedJson<'reg, 'rc>, RenderError> {
        let wrong_param_count = || {
            hb_err!("math: Found {} parameters, but math helper requires 3 parameters: number, operator as a string, number. Example: {{{{ math 1 \"+\" 2.5 }}}}.", h.params().len())
        };

        let a = h.param(0).ok_or_else(wrong_param_count)?.value();
        let operation = h.param(1).ok_or_else(wrong_param_count)?.value();
        let b = h.param(2).ok_or_else(wrong_param_count)?.value();
        let operation = operation.as_str().ok_or_else(|| {
            hb_err!("math: Second argument must be a string. Example: {{ math 1 \"+\" 2 }}.")
        })?;

        let aint = a
            .as_i64()
            .or_else(|| a.as_str().and_then(|s| i64::from_str(s).ok()));
        let bint = b
            .as_i64()
            .or_else(|| b.as_str().and_then(|s| i64::from_str(s).ok()));

        // try integer arithmetics
        if let (Some(aint), Some(bint)) = (aint, bint) {
            if operation != "/" {
                // normal division is done using floats to make it simpler for inexperienced users. For integer division, use //.
                return Ok(hb::ScopedJson::Derived(JsonValue::Number(Number::from(
                    Self::hb_math_int(aint, operation, bint)?,
                ))));
            }
        };

        // try float arithmetics
        let afloat = a
            .as_f64()
            .or_else(|| a.as_str().and_then(|s| f64::from_str(s).ok()))
            .ok_or_else(|| hb_err!("math: First number is not in valid format. Valid examples: 5, -62.53. Got this: {:?}", a))?;
        let bfloat = b
            .as_f64()
            .or_else(|| b.as_str().and_then(|s| f64::from_str(s).ok()))
            .ok_or_else(|| hb_err!("math: Second number is not in valid format. Valid examples: 5, -62.53. Got this: {:?}", b))?;

        let res = Self::hb_math_float(afloat, operation, bfloat)?;
        let res = Number::from_f64(res).ok_or_else(|| {
            hb_err!(
                "math: Calculation result is {}, which cannot be converted to JSON number.",
                res
            )
        })?;
        Ok(hb::ScopedJson::Derived(JsonValue::Number(res)))
    }
}

#[derive(Debug)]
pub(crate) struct HbRender {
    pub(crate) hb: Handlebars<'static>,
    pub(crate) tpl_name: String,
    pub(crate) version: Arc<Mutex<Option<Version>>>,
}

impl HbRender {
    /// Version of the template to assume if it specifies none.
    const ASSUMED_FIRST_VERSION: Version = Version::new(1, 0, 0);

    pub(crate) fn new(
        project: &Project,
        output: &Output,
        default: &DefaultTemaplate,
    ) -> Result<Self> {
        let (version_helper, version) = VersionCheckHelper::new();
        let mut hb = Handlebars::new()
            .with_helper("eq", hb_eq)
            .with_helper("contains", hb_contains)
            .with_helper("cat", hb_cat)
            .with_helper("default", hb_default)
            .with_helper("matches", hb_matches)
            .with_helper("math", MathHelper)
            .with_helper("px2mm", DpiHelper::new(output))
            .with_helper("img_w", ImgHelper::width(project))
            .with_helper("img_h", ImgHelper::height(project))
            .with_helper("version_check", version_helper);

        let tpl_name = output
            .template
            .as_ref()
            .map(|t| t.to_string())
            .unwrap_or_else(|| default.filename.to_string());

        if let Some(template) = output.template.as_ref() {
            if template.exists() {
                hb.register_template_file(&tpl_name, template)
                    .with_context(|| format!("Error in template file `{}`", template))?;
            } else {
                let parent = template.parent().unwrap(); // The temaplate should've been resolved as absolute in Project
                fs::create_dir_all(parent)
                    .and_then(|_| fs::write(template, default.content.as_bytes()))
                    .with_context(|| {
                        format!("Error writing default template to file: `{}`", template)
                    })?;

                hb.register_template_string(&tpl_name, default.content)
                    .expect("Internal error: Could not load default template");
            }
        } else {
            hb.register_template_string(&tpl_name, default.content)
                .expect("Internal error: Could not load default template");
        }

        // Render with no data to an IO Sink.
        // This will certainly fail, but if the version_check() helper is used on top
        // of the template, we will get the version in self.version.
        let _ = hb.render_to_write(&tpl_name, &(), io::sink());

        Ok(Self {
            hb,
            tpl_name,
            version,
        })
    }

    pub(crate) fn render(&self, output: &Path, context: RenderContext) -> Result<()> {
        let rendered = self.hb.render(&self.tpl_name, &context)?;

        fs::write(output, rendered.as_bytes())
            .with_context(|| format!("Error writing output file: `{}`", output))?;

        Ok(())
    }

    pub(crate) fn version(&self) -> Option<Version> {
        Some(
            self.version
                .lock()
                .unwrap()
                .clone()
                .unwrap_or(Self::ASSUMED_FIRST_VERSION),
        )
    }
}

#[cfg(test)]
mod tests;
