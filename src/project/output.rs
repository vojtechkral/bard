use serde::Deserialize;
use toml::Value;

use crate::prelude::*;
use crate::project::{Format, Metadata};
use crate::util::PathBufExt;

#[derive(Deserialize, Debug)]
pub struct Output {
    pub file: PathBuf,
    pub template: Option<PathBuf>,

    #[serde(default)]
    pub format: Format,

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
            Some("pdf") => Format::Pdf,
            Some("html") => Format::Html,
            Some("xml") => Format::Xml,
            Some("json") => Format::Json,
            _ => bail!(
                "Unknown or unsupported format of output file: {}\nHint: Specify format with  \
                 'format = ...'\nSupported formats are: pdf, html, json, and xml.",
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
            Format::Pdf | Format::Html | Format::Hovorka => self.template.as_deref(),
            Format::Json | Format::Xml => None,
            Format::Auto => Format::no_auto(),
        }
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
