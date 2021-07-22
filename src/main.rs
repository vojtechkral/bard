use std::env;
use std::process;

use bard::{bard, cli};

fn main() {
    let args: Vec<_> = env::args_os().collect();
    cli::use_stderr(true);
    if let Err(err) = bard(&args[..]) {
        cli::error(err);
        process::exit(1);
    }
}
