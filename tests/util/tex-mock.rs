//! Small binary that mocks xelatex and tectonic CLI, used in some integration tests.

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // NB. clap doesn't support -flags, so parsing by hand here...

    let args: Vec<_> = env::args().collect();
    if args
        .iter()
        .any(|arg| arg == "-version" || arg == "--version")
    {
        println!("TeX Mock 0.1");
        return;
    }

    let out_dir: PathBuf = {
        let flag_pos = args
            .iter()
            .position(|arg| arg == "-output-directory" || arg == "-o")
            .expect("Need the out dir argument");
        (&args[flag_pos + 1]).into()
    };

    let mut tex: PathBuf = args.iter().last().unwrap().into();
    tex.set_extension("pdf");
    let pdf = tex.file_name().unwrap();

    let mut dest = File::create(out_dir.join(pdf)).unwrap();
    for arg in env::args() {
        dest.write_all(arg.as_bytes()).unwrap();
        dest.write_all(b"\n").unwrap();
    }
}
