use std::env;

use bard::{bard, cli};

fn main() {
    let args: Vec<_> = env::args_os().collect();
    cli::use_stderr(true);
    bard(&args[..]).unwrap_or_else(cli::error);
}
