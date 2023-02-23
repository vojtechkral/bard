use std::process::{Command, Stdio};
use std::{env, fs};

use bard::render::DEFAULT_TEMPLATES;

mod util;
pub use util::*;

fn npx(args: &[&str]) {
    let cmd_env = env::var("NPX_CMD");
    let cmd = cmd_env.as_ref().map(|s| s.as_str()).unwrap_or("npx");

    let cmdline = args.iter().fold(cmd.to_string(), |mut cmdline, arg| {
        cmdline.push(' ');
        cmdline.push_str(arg);
        cmdline
    });
    eprintln!("{}", cmdline);

    let success = Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .status()
        .unwrap()
        .success();
    assert!(success, "npx command failed: {} {:?}", cmd, args);
}

/// This test calls the reference Handlebars JS implementation to parse
/// our default templates. There were historically errors in them that
/// the Rust implementation didn't reject.
#[test]
#[ignore = "requires node.js and npx"]
fn hbs_js_parse() {
    let handlebars_ver = env::var("HANDLEBARS_VER");
    let handlebars_ver = handlebars_ver
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("latest");
    let handlebars = format!("handlebars@{}", handlebars_ver);

    let out_dir = work_dir("hbs-js", false).unwrap();
    fs::create_dir_all(&out_dir).unwrap();
    let out = out_dir.join("out");

    // Parse each template with JS handlebars
    for default in DEFAULT_TEMPLATES {
        let mut path = ROOT / "src/render/templates/";
        path.push(default.filename);
        npx(&[
            "--yes",
            handlebars.as_str(),
            "-f",
            out.to_str().unwrap(),
            path.to_str().unwrap(),
        ]);

        let out_size = fs::metadata(&out).unwrap().len();
        fs::remove_file(&out).unwrap();
        assert!(out_size > 0);
    }
}
