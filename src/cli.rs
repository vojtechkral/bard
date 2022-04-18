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

fn status_inner<K, S>(kind: K, color: Color, status: S)
where
    K: Display,
    S: Display,
{
    if stderr_used() {
        let kind = format!("{:>12}", kind).bold().color(color);
        let status = format!("{}", status).replace('\n', "\n             ");
        eprintln!("{} {}", kind, status);
    }
}

pub fn status<T: Display>(verb: &str, status: T) {
    status_inner(verb, Color::Cyan, status);
}

pub fn success<T: Display>(verb: T) {
    status_inner(verb, Color::Green, "");
}

pub fn warning<T: Display>(msg: T) {
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
