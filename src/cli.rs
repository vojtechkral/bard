use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Error;
use colored::*;

static USE_STDERR: AtomicBool = AtomicBool::new(false);

pub fn use_stderr(use_stderr: bool) {
    USE_STDERR.store(use_stderr, Ordering::Release);
}

fn stderr_used() -> bool {
    USE_STDERR.load(Ordering::Acquire)
}

pub fn cyan(s: &str) -> ColoredString {
    if stderr_used() {
        s.bold().cyan()
    } else {
        s.into()
    }
}

pub fn green(s: &str) -> ColoredString {
    if stderr_used() {
        s.bold().green()
    } else {
        s.into()
    }
}

pub fn yellow(s: &str) -> ColoredString {
    if stderr_used() {
        s.bold().yellow()
    } else {
        s.into()
    }
}

pub fn red(s: &str) -> ColoredString {
    if stderr_used() {
        s.bold().red()
    } else {
        s.into()
    }
}

pub fn status<T>(verb: &str, status: T)
where
    T: Display,
{
    if stderr_used() {
        eprintln!("{:>11} {}", cyan(verb), status);
    }
}

pub fn success(verb: &str) {
    if stderr_used() {
        eprintln!("{}", green(verb));
    }
}

pub fn error(error: Error) {
    if !stderr_used() {
        return;
    }

    eprintln!("{:>10}: {}", red("bard error"), error);

    let mut source = error.source();
    while let Some(err) = source {
        let err_str = format!("{}", err);
        for line in err_str.lines() {
            eprintln!("  {} {}", red("|"), line);
        }

        source = err.source();
    }
}
