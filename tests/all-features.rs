mod util;
pub use util::*;

#[test]
#[ignore = "requires TeX distribution due to post-processing"]
fn project_all_features() {
    let build =
        Builder::build_opts(TEST_PROJECTS / "all-features", "all-features", &OPTS_PS).unwrap();

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
}
