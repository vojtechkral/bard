use std::fmt::Display;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

use colored::{Color, Colorize};

use crate::prelude::*;

static USE_STDERR: AtomicBool = AtomicBool::new(false);

pub fn use_stderr(use_stderr: bool) {
    USE_STDERR.store(use_stderr, Ordering::Release);
}

fn stderr_used() -> bool {
    USE_STDERR.load(Ordering::Acquire)
}

fn indent_line(line: &str) {
    eprintln!("             {}", line);
}

fn status_inner(kind: impl Display, color: Color, status: impl Display) {
    if !stderr_used() {
        return;
    }

    let kind = format!("{:>12}", kind).bold().color(color);
    let status = format!("{}", status);
    let mut lines = status.lines();
    let first = lines.next().unwrap_or("");
    eprintln!("{} {}", kind, first);
    lines.for_each(indent_line);
}

pub fn indent(status: impl Display) {
    if !stderr_used() {
        return;
    }

    let status = format!("{}", status);
    status.lines().for_each(indent_line);
}

pub fn status(verb: &str, status: impl Display) {
    status_inner(verb, Color::Cyan, status);
}

pub fn success(verb: impl Display) {
    status_inner(verb, Color::Green, "");
}

pub fn warning(msg: impl Display) {
    status_inner("Warning", Color::Yellow, msg);
}

pub fn error(error: Error) {
    if !stderr_used() {
        return;
    }

    status_inner("bard error", Color::Red, &error);

    let mut source = error.source();
    while let Some(err) = source {
        let err_str = format!("{}", err);
        for line in err_str.lines() {
            eprintln!("  {} {}", "|".bold().red(), line);
        }

        source = err.source();
    }
}

// TODO: Use an app context for cli logging (with verbosity)

pub trait TerminalExt {
    fn rewind_line(&mut self) -> io::Result<()>;
}

impl TerminalExt for dyn term::Terminal<Output = io::Stderr> + Send {
    fn rewind_line(&mut self) -> io::Result<()> {
        if atty::is(atty::Stream::Stdout) {
            self.cursor_up()?;
            self.delete_line()?;
        }

        Ok(())
    }
}
