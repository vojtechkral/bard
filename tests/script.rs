use std::collections::HashMap;
use std::fs;

mod util;
pub use util::*;

#[test]
fn project_script() {
    let build = Builder::build_with_ps(TEST_PROJECTS / "script", "script").unwrap();
    let out_dir = build.project.settings.dir_output();

    let out = fs::read_to_string(out_dir.join("out.toml")).unwrap();
    let out: HashMap<String, String> = toml::from_str(&out).unwrap();
    assert_eq!(out["BARD"], build.app.bard_exe().to_str().unwrap());
    assert_eq!(out["OUTPUT"], out_dir.join("out.html").to_str().unwrap());
    assert_eq!(
        out["PROJECT_DIR"],
        build.project.project_dir.to_str().unwrap()
    );
    assert_eq!(out["OUTPUT_DIR"], out_dir.to_str().unwrap());

    // Build with post-processing disabled
    let build = Builder::build_with_name(TEST_PROJECTS / "script", "script-no-ps").unwrap();
    let out_dir = build.project.settings.dir_output();
    assert!(!out_dir.join("out1.toml").exists());
    assert!(!out_dir.join("out2.toml").exists());
}

#[test]
fn project_script_fail() {
    Builder::build_with_ps(TEST_PROJECTS / "script-fail", "script-fail").unwrap_err();
}
