use std::env;
use std::process;

fn main() {
    let args: Vec<_> = env::args_os().collect();
    process::exit(bard::bard(&args[..]));
}
