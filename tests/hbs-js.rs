use std::process::{Command, Stdio};

use camino::Utf8Path as Path;

use bard::render::DEFAULT_TEMPLATES;

mod util;
pub use util::*;

fn yarn(args: &[&str], dir: &Path) {
    let success = Command::new("yarn")
        .args(&*args)
        .current_dir(dir)
        .stdout(Stdio::null())
        .stdin(Stdio::null())
        .status()
        .unwrap()
        .success();
    assert!(success, "yarn command failed: {:?}", args);
}

/// This test calls the reference Handlebars JS implementation to parse
/// our default templates. There were historically errors in them that
/// the Rust implementation didn't reject.
#[ignore]
#[test]
fn hbs_js_parse() {
    let dir = ROOT / "tests/hbs-js";

    yarn(&["install"], &dir);

    // Parse each template with JS handlebars
    for default in &DEFAULT_TEMPLATES[..] {
        let mut path = ROOT / "src/render/templates/";
        path.push(default.filename);
        yarn(&["run", "handlebars", path.as_str()], &dir);
    }
}
