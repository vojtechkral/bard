use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumVariantNames, VariantNames};

use crate::prelude::*;
use crate::project::Metadata;
use crate::util::PathBufExt;

#[derive(Serialize, Deserialize, Display, EnumVariantNames, PartialEq, Eq, Clone, Copy, Debug)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Format {
    Pdf,
    Html,
    Hovorka,
    Json,
    Xml,
}

impl Format {
    pub fn try_from_ext(path: &Path) -> Result<Self> {
        let format_hint = || {
            format!(
                "Hint: You can specify format with 'format = ...', supported formats are: {:?}.",
                Format::VARIANTS
            )
        };

        let ext = path
            .extension()
            .ok_or_else(|| {
                anyhow!(
                    "Could not detect format for output file '{}' - no extension.\n{}",
                    path,
                    format_hint(),
                )
            })?
            .to_ascii_lowercase();

        Ok(match ext.as_str() {
            "pdf" => Self::Pdf,
            "html" => Self::Html,
            "json" => Self::Json,
            "xml" => Self::Xml,
            _ => bail!(
                "Could not detect format based file on extension for: '{}'\n{}",
                path,
                format_hint(),
            ),
        })
    }
}

fn default_dpi() -> f64 {
    144.0
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Output {
    #[serde(skip_serializing)]
    pub file: PathBuf,
    #[serde(skip_serializing)]
    pub template: Option<PathBuf>,
    pub format: Option<Format>,
    #[serde(default)]
    pub sans_font: bool,
    pub toc_sort_key: Option<String>,
    #[serde(default = "default_dpi")]
    pub dpi: f64,
    pub script: Option<String>,

    #[serde(rename = "book", default, skip_serializing)]
    pub book_overrides: Metadata,
}

impl Output {
    pub fn resolve(&mut self, dir_templates: &Path, dir_output: &Path) -> Result<()> {
        if let Some(template) = self.template.as_mut() {
            template.resolve(dir_templates);
        }

        if self.format.is_none() {
            self.format = Some(Format::try_from_ext(&self.file)?);
        }

        self.file.resolve(dir_output);
        Ok(())
    }

    pub fn format(&self) -> Format {
        self.format.unwrap()
    }

    pub fn output_filename(&self) -> &str {
        self.file.file_name().expect("OutputSpec: Invalid filename")
    }

    pub fn template_path(&self) -> Option<&Path> {
        match self.format() {
            Format::Pdf | Format::Html | Format::Hovorka => self.template.as_deref(),
            Format::Json | Format::Xml => None,
        }
    }

    pub fn template_filename(&self) -> String {
        self.template
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_else(|| String::from("<builtin>"))
    }

    pub fn is_pdf(&self) -> bool {
        self.format() == Format::Pdf
    }

    pub fn override_book_section<'a>(&self, project_book: &'a Metadata) -> Cow<'a, Metadata> {
        if self.book_overrides.is_empty() {
            Cow::Borrowed(project_book)
        } else {
            let mut meta = project_book.clone();
            meta.extend(
                self.book_overrides
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone())),
            );
            Cow::Owned(meta)
        }
    }
}
