use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde_json as json;

use bard::MakeOpts;
use bard::render::{RHtml, RTex, RHovorka, DefaultTemaplate};

mod util;
use util::{Builder, assert_file_contains, ROOT, TEST_PROJECTS};

#[test]
fn init_and_build() {
    let _build = Builder::init_and_build("init").unwrap();
}

#[test]
fn project_default() {
    let _build = Builder::build(ROOT / "default").unwrap();
}

#[test]
fn project_example() {
    let _build = Builder::build(ROOT / "example").unwrap();
}

#[test]
fn project_default_templates () {
    let _build = Builder::build(TEST_PROJECTS / "default-templates").unwrap();
}

#[test]
fn project_default_templates_save () {
    let build = Builder::build(TEST_PROJECTS / "default-templates-save").unwrap();
    let templates = build.dir.join("templates");

    let html = fs::read_to_string(templates.join("html.hbs")).unwrap();
    assert_eq!(html, RHtml::TPL_CONTENT);

    let tex = fs::read_to_string(templates.join("pdf.hbs")).unwrap();
    assert_eq!(tex, RTex::TPL_CONTENT);

    let hovorka = fs::read_to_string(templates.join("hovorka.hbs")).unwrap();
    assert_eq!(hovorka, RHovorka::TPL_CONTENT);
}

#[cfg(not(windows))]  // FIXME: make this work on windows
#[test]
fn watch() {
    use std::thread;
    use std::time::Duration;
    use util::OPTS_NO_PS;

    use bard::watch::Watch;

    const DELAY: Duration = Duration::from_millis(1250);
    const TEST_STR: &str = "test test test";

    let build = Builder::build(TEST_PROJECTS / "watch").unwrap();

    // Start bard watch in another thread
    let dir2 = build.dir.clone();
    let (watch, cancellation) = Watch::new().unwrap();
    let watch_thread = thread::spawn(move || {
        bard::bard_watch_at(&OPTS_NO_PS, &dir2, watch)
    });

    thread::sleep(DELAY);

    // Modify a song:
    let song_path = build.project.input_paths()[0].clone();
    let mut song = fs::read_to_string(&song_path).unwrap();
    song.push_str(TEST_STR);
    song.push('\n');
    fs::write(&song_path, song.as_bytes()).unwrap();

    thread::sleep(DELAY);

    // Cancel watching:
    cancellation.cancel();

    // Check if the change was picked up:
    let html = build.project.output_paths().next().unwrap();
    assert_file_contains(html, TEST_STR);

    watch_thread.join().unwrap().unwrap();
}

#[test]
fn project_postprocess () {
    let build = Builder::build_opts(TEST_PROJECTS / "postprocess", &MakeOpts::default()).unwrap();
    let out_dir = build.project.settings.dir_output();

    let basic = fs::read_to_string(out_dir.join("process-basic.json")).unwrap();
    let basic: HashMap<String, String> = json::from_str(&basic).unwrap();
    assert_eq!(basic["file_name"], "basic.html");
    assert_eq!(basic["file_stem"], "basic");

    let extended = fs::read_to_string(out_dir.join("process-extended.json")).unwrap();
    let extended: HashMap<String, String> = json::from_str(&extended).unwrap();
    assert_eq!(extended["file_name"], "extended.html");
    assert_eq!(extended["file_stem"], "extended");
    let file = PathBuf::from(&extended["file"]);
    assert_eq!(file.file_name().unwrap(), "extended.html");
    assert_file_contains(&file, "Yippie yea");
}
