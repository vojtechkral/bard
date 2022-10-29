use std::convert::TryInto;
use std::ops::Deref;
use std::path::Path as StdPath;
use std::process::{ChildStderr, ChildStdout, ExitStatus};
use std::{fs, io, iter, mem};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;

use lexical_sort::{lexical_cmp, PathSort};

use crate::prelude::*;

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

/// Scope guard
pub struct ScopeGuard<R, F: FnOnce() -> R> {
    guard: Option<F>,
    armed: bool,
}

impl<R, F> ScopeGuard<R, F>
where
    F: FnOnce() -> R,
{
    pub fn new(guard: F) -> Self {
        Self {
            guard: Some(guard),
            armed: true,
        }
    }

    pub fn set_armed(&mut self, armed: bool) {
        self.armed = armed
    }
}

impl<R, F> Drop for ScopeGuard<R, F>
where
    F: FnOnce() -> R,
{
    fn drop(&mut self) {
        if self.armed {
            let _ = self.guard.take().map(|f| f());
        }
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

// I/O utils

/// Like `std::io::Lines` but with raw bytes instead of UTF-8 strings.
pub struct BinaryLines<R> {
    read: R,
    buffer: Vec<u8>,
    read_buffer: Vec<u8>,
    eof: bool,
}

impl<R> BinaryLines<R>
where
    R: io::Read,
{
    const READ_SIZE: usize = 4096;

    pub fn new(read: R) -> Self {
        Self {
            read,
            buffer: vec![],
            read_buffer: vec![0; Self::READ_SIZE],
            eof: false,
        }
    }

    pub fn eof(&self) -> bool {
        self.eof
    }

    fn take_line(&mut self, search_from: usize) -> Option<Vec<u8>> {
        let search_slice = &self.buffer[search_from..];
        let lf_pos = search_slice.iter().position(|&b| b == b'\n')? + search_from;
        Some(self.buffer.remove_prefix(lf_pos + 1))
    }
}

impl<R> Iterator for BinaryLines<R>
where
    R: io::Read,
{
    type Item = io::Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.eof {
            return None;
        }

        if let Some(line) = self.take_line(0) {
            return Some(Ok(line));
        }

        loop {
            let num_read = match self.read.read(&mut self.read_buffer) {
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Some(Err(e)),
            };

            if num_read == 0 {
                self.eof = true;
                if !self.buffer.is_empty() {
                    if self.buffer.last() != Some(&b'\n') {
                        self.buffer.extend_from_slice(LINE_END.as_bytes());
                    }
                    return Some(Ok(mem::take(&mut self.buffer)));
                } else {
                    return None;
                }
            }

            let prev_len = self.buffer.len();
            self.buffer.extend_from_slice(&self.read_buffer[..num_read]);

            if let Some(line) = self.take_line(prev_len) {
                return Some(Ok(line));
            }
        }
    }
}

// Process utils

mod process_generic;
mod process_nix;

#[cfg(not(unix))]
use process_generic as process_impl;
#[cfg(unix)]
use process_nix as process_impl;

/// A `ChildStdout` and `ChildStderr` adaptor that can stream process output as lines
/// from both pipes in a non-blocking way. It also simultaneously stores all the lines internally.
pub struct ProcessLines {
    inner: process_impl::ProcessLines,
    lines: Vec<Vec<u8>>,
}

impl ProcessLines {
    pub fn new(stdout: ChildStdout, stderr: ChildStderr) -> Self {
        Self {
            inner: process_impl::ProcessLines::new(stdout, stderr),
            lines: vec![],
        }
    }

    pub fn read_line(&mut self) -> io::Result<Option<Vec<u8>>> {
        let res = self.inner.read_line();
        if let Ok(Some(line)) = res.as_ref() {
            self.lines.push(line.clone());
        }
        res
    }

    pub fn collected_lines(&self) -> impl Iterator<Item = &[u8]> {
        self.lines.iter().map(|v| v.as_slice())
    }
}

#[cfg(test)]
mod tests;

pub struct TempDir {
    path: PathBuf,
    remove: bool,
}

impl TempDir {
    const RAND_CHARS: u32 = 6;
    const RETRIES: u32 = 9001;

    pub fn new(prefix: impl Into<PathBuf>, remove: bool) -> Result<Self> {
        let mut path = prefix.into().into_string();

        let orig_len = path.len();
        path.reserve(Self::RAND_CHARS as usize + 1);
        for _ in 0..Self::RETRIES {
            path.push('.');
            Self::push_rand_chars(&mut path);
            if Self::create_dir(&path)? {
                return Ok(Self {
                    path: path.into(),
                    remove,
                });
            }

            path.truncate(orig_len);
        }

        bail!("Could not create temporary directory, prefix: `{}`", path);
    }

    fn push_rand_chars(s: &mut String) {
        for c in iter::repeat_with(fastrand::alphanumeric).take(Self::RAND_CHARS as usize) {
            s.push(c)
        }
    }

    fn create_dir(path: &str) -> Result<bool> {
        let path = <&Path>::from(path);
        match fs::create_dir(path) {
            Ok(_) => Ok(true),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Ok(false),
            Err(err) => Err(err).with_context(|| format!("Could not create directory `{}`", path)),
        }
    }

    pub fn set_remove(&mut self, remove: bool) {
        self.remove = remove;
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if self.remove {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

impl AsRef<Path> for TempDir {
    fn as_ref(&self) -> &Path {
        self.path.as_ref()
    }
}

impl Deref for TempDir {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
