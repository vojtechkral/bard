use std::fs::File;
use std::io::Write as _;

mod util_ng;
pub use util_ng::*;

const SMOL_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAQAAAAEAAQMAAABmvDolAAAAA1BMVEW10NBjBBbqAAAAH0lEQVRoge3BAQ0AAADCoPdPbQ43oAAAAAAAAAAAvg0hAAABmmDh1QAAAABJRU5ErkJggg==";

#[test]
fn watch() {
    const TEST_STR: &str = "watch test watch test";

    let build = TestProject::new("watch")
        .song(
            "watch.md",
            indoc! {r#"
            # Watch Test

            1. `C`Watch.
            ![smol](smol.png "center")
        "#},
        )
        .binary_asset("smol.png", SMOL_PNG)
        .output("songbook.html")
        .build()
        .unwrap();

    // Start bard watch in another thread
    let (watch_thread, control) = build.watch();

    // Wait for the watch to actually start watching files
    // after the initial render pass:
    control.wait_watching();

    // Modify a source file:
    let md_file = build.dir_songs().join("watch.md");
    File::options()
        .append(true)
        .open(&md_file)
        .unwrap()
        .write_all(TEST_STR.as_bytes())
        .unwrap();

    // Wait for the watching to resume after the 1st triggered render pass:
    control.wait_watching();

    // Modify an image file:
    let img_file = build.dir_output().join("smol.png");
    let content = SMOL_PNG.decode_base64();
    File::create(&img_file)
        .unwrap()
        .write_all(&content)
        .unwrap();

    // Wait for the watching to resume after the 2nd triggered render pass:
    control.wait_watching();

    // Cancel watching:
    control.cancel();

    // Check that output contains test string:
    let html = build.read_output(".html");
    assert!(html.contains(TEST_STR));

    watch_thread.join().unwrap();
}
