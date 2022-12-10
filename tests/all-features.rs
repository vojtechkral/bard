mod util;
use std::fs;

pub use util::*;

#[test]
fn project_all_features() {
    let build = Builder::build_with_ps(TEST_PROJECTS / "all-features", "all-features").unwrap();

    // Verify the list of songs
    let titles: Vec<_> = build.project.book.songs.iter().map(|s| &*s.title).collect();
    assert_eq!(
        titles,
        &[
            "Danny Boy",
            "Wildcard 1",
            "Wildcard 2",
            "Multiple Songs 1",
            "Multiple Songs 2",
        ]
    );

    // Verify script worked
    let out_dir = build.project.settings.dir_output();
    let html = fs::read_to_string(out_dir.join("songbook.html")).unwrap();
    let html_copy = fs::read_to_string(out_dir.join("copy-of-songbook.html")).unwrap();
    assert_eq!(html, html_copy);
}
