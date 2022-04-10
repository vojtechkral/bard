use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Error;
use colored::{Color, Colorize};

static USE_STDERR: AtomicBool = AtomicBool::new(false);

pub fn use_stderr(use_stderr: bool) {
    USE_STDERR.store(use_stderr, Ordering::Release);
}

fn stderr_used() -> bool {
    USE_STDERR.load(Ordering::Acquire)
}

fn status_inner<S, T>(kind: S, color: Color, status: T)
where
    S: Display,
    T: Display,
{
    if stderr_used() {
        let kind = format!("{:>12}", kind).bold().color(color);
        eprintln!("{} {}", kind, status);
    }
}

pub fn status<T>(verb: &str, status: T)
where
    T: Display,
{
    status_inner(verb, Color::Cyan, status);
}

pub fn success(verb: &str) {
    status_inner(verb, Color::Green, "");
}

pub fn warning(msg: &str) {
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
