use std::fs;

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
