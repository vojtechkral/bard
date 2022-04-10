use std::convert::TryFrom;
use std::env;

use camino::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use serde::{Deserialize, Serialize};

use crate::error::*;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum CmdSpec {
    Basic(String),
    Multiple(Vec<String>),
    Extended(Vec<Vec<String>>),
}

impl CmdSpec {
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Basic(s) => s.is_empty(),
            Self::Multiple(v) => v.is_empty(),
            Self::Extended(v) => v.is_empty(),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct PostProcessCtx<'a> {
    bard: String,
    file: &'a str,
    file_name: &'a str,
    file_stem: &'a str,
    project_dir: &'a str,
}

impl<'a> PostProcessCtx<'a> {
    pub fn new(file: &'a Path, project_dir: &'a Path) -> Result<Self> {
        let bard = env::current_exe()
            .map_err(Error::from)
            .and_then(|p| PathBuf::try_from(p).map_err(Error::from))
            .map(|p| p.to_string())
            .context("Could not read path to bard executable")?;

        // NOTE: Filenames should be canonicalized at this point
        let file_name = file.file_name().unwrap();
        let file_stem = file.file_stem().unwrap_or(file_name);

        Ok(Self {
            bard,
            file: file.as_str(),
            file_name,
            file_stem,
            project_dir: project_dir.as_str(),
        })
    }
}
