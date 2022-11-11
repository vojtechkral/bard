use std::fs;
use std::thread;
use std::time::Duration;

use bard::app::App;
use bard::watch::Watch;

mod util;
pub use util::*;

#[test]
fn watch() {
    const DELAY: Duration = Duration::from_millis(1250);
    const TEST_STR: &str = "test test test";

    let build = Builder::build(TEST_PROJECTS / "watch").unwrap();

    // Start bard watch in another thread
    let dir2 = build.dir.clone();
    let (watch, cancellation) = Watch::new().unwrap();
    let watch_thread = thread::spawn(move || {
        let app = App::with_test_mode(false);
        bard::bard_watch_at(&app, &dir2, watch)
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
