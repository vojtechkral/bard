use std::ffi::{OsStr, OsString};
use std::{fs, io, iter, ops};

use crate::prelude::*;

/// Path extension
pub trait PathExt {
    /// Join a `stem` (eg. from some other filename) with this path
    /// and add an `extenion`.
    fn join_stem(&self, stem: &OsStr, extension: &str) -> PathBuf;
}

impl PathExt for Path {
    fn join_stem(&self, stem: &OsStr, extension: &str) -> PathBuf {
        let mut res: OsString = self.join(stem).into();
        res.push(extension);
        res.into()
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

// TempPath

#[derive(Clone, Copy, Debug)]
enum TempPathType {
    File,
    Dir,
}

/// A path that may be removed on drop. Also provides temp dir creation via `make_temp_dir()`.
#[derive(Debug)]
pub struct TempPath {
    path: PathBuf,
    typ: TempPathType,
    remove: bool,
}

impl TempPath {
    const RAND_CHARS: u32 = 6;
    const RETRIES: u32 = 9001;

    pub fn new_file(path: impl Into<PathBuf>, remove: bool) -> Self {
        Self {
            path: path.into(),
            typ: TempPathType::File,
            remove,
        }
    }

    pub fn new_dir(path: impl Into<PathBuf>, remove: bool) -> Self {
        Self {
            path: path.into(),
            typ: TempPathType::Dir,
            remove,
        }
    }

    pub fn make_temp_dir(prefix: impl Into<OsString>, remove: bool) -> Result<Self> {
        let prefix = prefix.into();

        let mut sufffix = String::with_capacity(Self::RAND_CHARS as usize + 1);
        for _ in 0..Self::RETRIES {
            sufffix.clear();
            for c in iter::repeat_with(fastrand::alphanumeric).take(Self::RAND_CHARS as usize) {
                sufffix.push(c)
            }

            let mut path = prefix.clone(); // have to clone due to the limited OsString API
            path.push(&sufffix);
            if Self::create_dir(&path)? {
                return Ok(Self::new_dir(path, remove));
            }
        }

        bail!(
            "Could not create temporary directory, prefix: {:?}",
            Path::new(&prefix)
        );
    }

    fn create_dir(path: impl AsRef<OsStr>) -> Result<bool> {
        let path = Path::new(path.as_ref());
        match fs::create_dir(path) {
            Ok(_) => Ok(true),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Ok(false),
            Err(err) => Err(err).with_context(|| format!("Could not create directory {:?}", path)),
        }
    }

    pub fn set_remove(&mut self, remove: bool) {
        self.remove = remove;
    }
}

impl Drop for TempPath {
    fn drop(&mut self) {
        if !self.remove {
            return;
        }

        let _ = match self.typ {
            TempPathType::File => fs::remove_file(&self.path),
            TempPathType::Dir => fs::remove_dir_all(&self.path),
        };
    }
}

impl AsRef<Path> for TempPath {
    fn as_ref(&self) -> &Path {
        self.path.as_ref()
    }
}

impl ops::Deref for TempPath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
