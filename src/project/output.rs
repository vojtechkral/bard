use camino::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use serde::Deserialize;
use toml::Value;

use super::{CmdSpec, Format, Metadata};
use crate::error::*;
use crate::util::PathBufExt;

#[derive(Deserialize, Debug)]
pub struct Output {
    pub file: PathBuf,
    pub template: Option<PathBuf>,

    #[serde(default)]
    pub format: Format,

    #[serde(rename = "process")]
    pub post_process: Option<CmdSpec>,
    #[serde(rename = "process_win")]
    pub post_process_win: Option<CmdSpec>,

    #[serde(flatten)]
    pub metadata: Metadata,
}

impl Output {
    pub fn resolve(&mut self, dir_templates: &Path, dir_output: &Path) -> Result<()> {
        if let Some(template) = self.template.as_mut() {
            template.resolve(dir_templates);
        }
        self.file.resolve(dir_output);

        if !matches!(self.format, Format::Auto) {
            return Ok(());
        }

        let ext = self.file.extension().map(str::to_lowercase);

        self.format = match ext.as_deref() {
            Some("html") | Some("xhtml") | Some("htm") | Some("xht") => Format::Html,
            Some("tex") => Format::Tex,
            Some("xml") => Format::Hovorka,
            Some("json") => Format::Json,
            _ => bail!(
                "Unknown or unsupported format of output file: {}\nHint: Specify format with  \
                 'format = ...'",
                self.file
            ),
        };

        Ok(())
    }

    pub fn output_filename(&self) -> &str {
        self.file.file_name().expect("OutputSpec: Invalid filename")
    }

    pub fn template_path(&self) -> Option<&Path> {
        match self.format {
            Format::Html | Format::Tex | Format::Hovorka => self.template.as_deref(),
            Format::Json => None,
            Format::Auto => Format::no_auto(),
        }
    }

    pub fn post_process(&self) -> Option<&CmdSpec> {
        if cfg!(windows) && self.post_process_win.is_some() {
            return self.post_process_win.as_ref();
        }

        self.post_process.as_ref()
    }

    pub fn template_filename(&self) -> String {
        self.template
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_else(|| String::from("<builtin>"))
    }

    pub fn dpi(&self) -> f64 {
        const DEFAULT: f64 = 144.0;

        self.metadata
            .get("dpi")
            .and_then(|value| match value {
                Value::Integer(i) => Some(*i as f64),
                Value::Float(f) => Some(*f),
                _ => None,
            })
            .unwrap_or(DEFAULT)
    }
}
