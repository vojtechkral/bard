mod util;
use std::process::{Command, Stdio};

use bard::{book, project::Settings, util::Apply, PROGRAM_META};
use semver::Version;
pub use util::*;

fn get_version(args: &[&str]) -> String {
    Command::new(&bard_exe())
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .unwrap()
        .stdout
        .apply(|bytes| String::from_utf8_lossy(&bytes).trim_end().to_string())
}

#[test]
fn cli_version_program() {
    let ver = get_version(&["-V"]);
    let ver_long = get_version(&["--version"]);
    assert_eq!(ver, PROGRAM_META.version);
    assert_eq!(ver_long, PROGRAM_META.version);
}

#[test]
fn cli_version_settings() {
    let ver = get_version(&["--version-settings"]).parse::<u32>().unwrap();
    assert_eq!(ver, Settings::version());
}

#[test]
fn cli_version_ast() {
    let ver = Version::parse(&get_version(&["--version-ast"])).unwrap();
    assert_eq!(&ver, book::version::current());
}
