use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

use serde_json as json;

use bard::MakeOpts;

mod util;
pub use util::*;

#[test]
fn project_postprocess() {
    let build = Builder::build_opts(TEST_PROJECTS / "postprocess", &MakeOpts::default()).unwrap();
    let out_dir = build.project.settings.dir_output();

    let exe = env::current_exe()
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap();

    let basic = fs::read_to_string(out_dir.join("process-basic.json")).unwrap();
    let basic: HashMap<String, String> = json::from_str(&basic).unwrap();
    assert_eq!(basic["bard"], exe);
    assert_eq!(basic["file_name"], "basic.html");
    assert_eq!(basic["file_stem"], "basic");

    let extended = fs::read_to_string(out_dir.join("process-extended.json")).unwrap();
    let extended: HashMap<String, String> = json::from_str(&extended).unwrap();
    assert_eq!(extended["bard"], exe);
    assert_eq!(extended["file_name"], "extended.html");
    assert_eq!(extended["file_stem"], "extended");
    let file = PathBuf::from(&extended["file"]);
    assert_eq!(file.file_name().unwrap(), "extended.html");
    assert_file_contains(&file, "Yippie yea");
}
