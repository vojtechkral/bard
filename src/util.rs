use std::hash::Hash;
use std::path::Path as StdPath;
use std::sync::Arc;
use std::{collections::HashMap, ffi::OsString};
use std::{fmt, fs};

use lexical_sort::{lexical_cmp, PathSort};
use parking_lot::RwLock;

use crate::prelude::*;

mod path;
mod process;
pub mod xml_support;

pub use path::{PathBufExt, PathExt, TempPath};
pub use process::{ExitStatusExt, ProcessLines};

#[cfg(unix)]
pub const LINE_END: &str = "\n";
#[cfg(not(unix))]
pub const LINE_END: &str = "\r\n";

/// `Vec` utils
pub trait VecExt {
    fn remove_prefix(&mut self, size: usize) -> Self;
}

impl<T> VecExt for Vec<T>
where
    T: Clone,
{
    fn remove_prefix(&mut self, size: usize) -> Self {
        let size = size.min(self.len());
        let res = self[..size].to_vec();
        self.rotate_left(size);
        self.truncate(self.len() - size);
        res
    }
}

/// Boxed str alias and extensions for `[u8]` and `Vec<u8>`
pub type BStr = Box<str>;

/// `str` utils

pub trait StrExt {
    fn to_os_string(&self) -> OsString;
    fn clone_bstr(&self) -> Box<str>;
}

impl StrExt for str {
    fn to_os_string(&self) -> OsString {
        self.to_string().into()
    }

    fn clone_bstr(&self) -> BStr {
        Box::from(self)
    }
}

/// Apply a function to anything, like `let` in Kotlin.
pub trait Apply: Sized {
    fn apply<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R;
}

impl<T> Apply for T {
    fn apply<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
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
        let path = entry.path();
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

/// A very simple cache.
#[derive(Clone)]
pub struct Cache<K, V>(Arc<RwLock<HashMap<K, V>>>);

impl<K, V> Cache<K, V> {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }
}

impl<K, V> Cache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    pub fn try_get<E>(&self, key: &K, f: impl FnOnce() -> Result<V, E>) -> Result<V, E> {
        let cache = self.0.read();
        if let Some(value) = cache.get(key) {
            return Ok(value.clone());
        }

        drop(cache);

        let value = f()?;
        self.0.write().insert(key.clone(), value.clone());
        Ok(value)
    }
}

impl<K, V> Default for Cache<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> fmt::Debug for Cache<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(cache) = self.0.try_read() {
            write!(f, "Cache(unlocked, {} entries)", cache.len())
        } else {
            write!(f, "Cache(locked)")
        }
    }
}

/// Cache of image dimensions.
pub type ImgCache = Cache<PathBuf, (u32, u32)>;
