use std::process::ExitStatus;
use std::path::{self, PathBuf, Path};
use std::env;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;

use crate::error::*;

pub type BStr = Box<str>;

/// Byte slice extension (also for `Vec<u8>`)
pub trait ByteSliceExt {
    fn into_bstr(&self) -> BStr;
}

impl ByteSliceExt for [u8] {
    fn into_bstr(&self) -> BStr {
        String::from_utf8_lossy(self).into()
    }
}

impl ByteSliceExt for Vec<u8> {
    fn into_bstr(&self) -> BStr {
        self.as_slice().into_bstr()
    }
}

/// PathBuf extension
pub trait PathBufExt {
    /// If the path is relative, resolve it as absolute wrt. `base_dir`
    fn resolve(&mut self, base_dir: &Path);
    fn resolved(self, base_dir: &Path) -> Self;
    fn utf8_check(&self) -> Result<(), path::Display>;
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

    fn utf8_check(&self) -> Result<(), path::Display> {
        self.to_str().map(|_| ()).ok_or(self.display())
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
                bail!("Process killed by signal: {}", signal);
            }
        }

        match self.code() {
            Some(code) => bail!("Process exited with code: {}", code),
            None => bail!("Process failed with unknown error"),
        }
    }
}

// CwdGuard

/// Used for globbing, which doesn't support setting base for relative globs
pub struct CwdGuard {
    orig_path: PathBuf,
}

impl CwdGuard {
    pub fn new<P>(new_path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut orig_path = env::current_dir().context("Could not read current directory")?;

        let new_path = new_path.as_ref();
        env::set_current_dir(new_path).with_context(|| {
            orig_path.push(new_path);
            format!("Could not enter directory: `{}`", orig_path.display())
        })?;

        Ok(Self { orig_path })
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = env::set_current_dir(&self.orig_path);
    }
}
