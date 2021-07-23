use std::env;
use std::path::Path;

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
            .map_err(Error::new)
            .and_then(|exe| {
                exe.into_os_string()
                    .into_string()
                    .map_err(|os| anyhow!("Can't convert to UTF-8: {:?}", os))
            })
            .context("Could not read path to bard executable")?;

        // NOTE: Filenames should be known to be UTF-8-valid and canonicalized at this point
        let file_name = file.file_name().unwrap();
        let file_stem = file.file_stem().unwrap_or(file_name).to_str().unwrap();

        Ok(Self {
            bard,
            file: file.to_str().unwrap(),
            file_name: file_name.to_str().unwrap(),
            file_stem,
            project_dir: project_dir.to_str().unwrap(),
        })
    }
}
