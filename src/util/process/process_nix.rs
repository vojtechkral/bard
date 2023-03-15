#![cfg(unix)]

use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::process::{ChildStderr, ChildStdout};

use nix::poll::{self, PollFd, PollFlags};

use super::BinaryLines;

impl<R> AsRawFd for BinaryLines<R>
where
    R: AsRawFd,
{
    fn as_raw_fd(&self) -> RawFd {
        self.read.as_raw_fd()
    }
}

pub struct ProcessLines {
    stdout: BinaryLines<ChildStdout>,
    stderr: BinaryLines<ChildStderr>,
}

impl ProcessLines {
    pub fn new(stdout: ChildStdout, stderr: ChildStderr) -> Self {
        Self {
            stdout: BinaryLines::new(stdout),
            stderr: BinaryLines::new(stderr),
        }
    }

    pub fn read_line(&mut self) -> io::Result<Option<Vec<u8>>> {
        loop {
            if self.stdout.eof() && self.stderr.eof() {
                return Ok(None);
            }

            // let events = PollFlags::POLLIN & PollFlags::POLLHUP;
            let events = PollFlags::all();
            let p_stdout = PollFd::new(self.stdout.as_raw_fd(), events);
            let p_stderr = PollFd::new(self.stderr.as_raw_fd(), events);
            let mut fds = [p_stdout, p_stderr];

            poll::poll(&mut fds, -1)?;

            let [p_stdout, p_stderr] = fds;

            if p_stdout.revents().unwrap().intersects(events) {
                if let Some(line) = self.stdout.next().transpose()? {
                    return Ok(Some(line));
                }
            }

            if p_stderr.revents().unwrap().intersects(events) {
                if let Some(line) = self.stderr.next().transpose()? {
                    return Ok(Some(line));
                }
            }
        }
    }
}
