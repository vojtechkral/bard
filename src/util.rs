use std::fs;
use std::hash::Hash;
use std::path::Path as StdPath;
use std::sync::Arc;
use std::{collections::HashMap, ffi::OsString};

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

/// `str` utils

pub trait StrExt {
    fn to_os_string(&self) -> OsString;
}

impl StrExt for str {
    fn to_os_string(&self) -> OsString {
        self.to_string().into()
    }
}

/// Boxed str alias and extensions for `[u8]` and `Vec<u8>`
pub type BStr = Box<str>;

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
