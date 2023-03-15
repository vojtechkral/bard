use std::{
    io, mem,
    process::{ChildStderr, ChildStdout, ExitStatus},
};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;

use crate::prelude::*;

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

mod process_generic;
mod process_nix;

#[cfg(not(unix))]
use process_generic as process_impl;
#[cfg(unix)]
use process_nix as process_impl;

use super::{VecExt, LINE_END};

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

#[cfg(test)]
mod tests;
