use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use bard::prelude::*;

mod util;
pub use util::*;

#[cfg(unix)]
fn bad_dir_name() -> OsString {
    use std::os::unix::ffi::OsStringExt as _;
    OsString::from_vec((b"bad-utf8-\xc0-\xc0\xaf"[..]).into())
}

#[cfg(windows)]
fn bad_dir_name() -> OsString {
    use std::os::windows::ffi::OsStringExt as _;
    OsString::from_wide(&[
        0x0062, 0x0061, 0x0064, 0x002d, 0x0075, 0x0074, 0x0066, 0x0031, 0x0036, 0x002d, 0xd834,
        0xd834, 0xdd1e, 0xdd1e,
    ])
}

/// Test that bard can bard when the containing path is invalid UTF-8.
///
/// Arguably it's somewhat pointless that bard can handle this, since neither xelatex
/// nor tectonic do. So we can't postprocess.
/// I guess at least you could generate HTML in such situation... `its-somethig-meme.jpg`.
#[test]
fn bad_unicode_path() {
    let workdir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(bad_dir_name());

    if workdir.exists() {
        fs::remove_dir_all(&workdir)
            .with_context(|| format!("Couldn't remove previous test run data: {:?}", workdir))
            .unwrap();
    }
    fs::create_dir(&workdir).unwrap();

    let app = Builder::app(false);
    bard::bard_init_at::<&Path>(&app, &workdir)
        .context("Failed to initialize")
        .unwrap();
    bard::bard_make_at::<&Path>(&app, &workdir)
        .context("Failed to build project")
        .unwrap();

    // Don't leave the dir with an invalid name on disk as it may be problematic for various tools.
    // For example, CI is unable to cache dependencies on Windows with this dir in /target.
    fs::remove_dir_all(&workdir).unwrap();
}
