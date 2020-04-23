use std::path::PathBuf;
use std::io;

use thiserror::Error;

pub use anyhow::{anyhow, bail, ensure, Context as _, Result};


#[derive(Error, Debug)]
#[error("Error writing file '{0}'")]
pub struct ErrorWritingFile(pub PathBuf, #[source] pub io::Error);


#[derive(Error, Debug)]
pub enum ErrorNotify {
    #[error("Could not watch files for changes")]
    Notify(#[from] notify::Error),

    #[error("Could not watch file for changes: '{path}'")]
    NotifyPath {
        path: PathBuf,
        source: notify::Error,
    },
}
