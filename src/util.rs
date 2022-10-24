use std::convert::TryInto;
use std::fs;
use std::path::Path as StdPath;
use std::process::ExitStatus;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;

use lexical_sort::{lexical_cmp, PathSort};

use crate::prelude::*;

pub type BStr = Box<str>;

/// Byte slice extension (also for `Vec<u8>`)
pub trait ByteSliceExt {
    fn as_bstr(&self) -> BStr;
}

impl ByteSliceExt for [u8] {
    fn as_bstr(&self) -> BStr {
        String::from_utf8_lossy(self).as_ref().into()
    }
}

impl ByteSliceExt for Vec<u8> {
    fn as_bstr(&self) -> BStr {
        self.as_slice().as_bstr()
    }
}

/// PathBuf extension
pub trait PathBufExt {
    /// If the path is relative, resolve it as absolute wrt. `base_dir`
    fn resolve(&mut self, base_dir: &Path);
    fn resolved(self, base_dir: &Path) -> Self;
}

impl PathBufExt for PathBuf {
    fn resolve(&mut self, base_dir: &Path) {
        if self.is_relative() {
            *self = base_dir.join(&self);
        }
    }

    fn resolved(mut self, base_dir: &Path) -> Self {
        self.resolve(base_dir);
        self
    }
}

/// ExitStatus extension
pub trait ExitStatusExt {
    fn into_result(self) -> Result<()>;
}

impl ExitStatusExt for ExitStatus {
    fn into_result(self) -> Result<()> {
        if self.success() {
            return Ok(());
        }

        #[cfg(unix)]
        {
            if let Some(signal) = self.signal() {
                bail!("Program killed by signal: {}", signal);
            }
        }

        match self.code() {
            Some(code) => bail!("Program exited with code: {}", code),
            None => bail!("Program failed with unknown error"),
        }
    }
}

// Lexical sorting
// Basically forwards to the lexical-sort crate

pub fn sort_lexical<S>(slice: &mut [S])
where
    S: AsRef<str>,
{
    sort_lexical_by(slice, AsRef::as_ref)
}

pub fn sort_lexical_by<T, F>(slice: &mut [T], mut key_fn: F)
where
    F: FnMut(&T) -> &str,
{
    slice.sort_by(|lhs, rhs| lexical_cmp(key_fn(lhs), key_fn(rhs)));
}

pub fn sort_paths_lexical<S>(slice: &mut [S])
where
    S: AsRef<StdPath>,
{
    slice.path_sort(lexical_cmp);
}

// fs utils

fn read_dir_all_inner(res: &mut Vec<PathBuf>, path: &Path) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path: PathBuf = entry.path().try_into()?;
        if entry.file_type()?.is_dir() {
            // Recurse
            read_dir_all_inner(res, &path)?;
        } else {
            res.push(path);
        }
    }

    Ok(())
}

pub fn read_dir_all<P: AsRef<Path>>(path: P) -> Result<Vec<PathBuf>> {
    let mut res = vec![];
    read_dir_all_inner(&mut res, path.as_ref())?;
    Ok(res)
}
