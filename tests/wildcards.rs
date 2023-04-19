mod util_ng;
pub use util_ng::*;

#[test]
fn project_wildcards_1() {
    let build = TestProject::new("wildcards-1")
        .song(
            "yippied.md",
            indoc! {"
            # Song

            1. Yippie!
        "},
        )
        .output("songbook.html")
        .settings(|toml| {
            toml.set("songs", "*.md");
        })
        .build()
        .unwrap();

    assert!(build.read_output(".html").contains("Yippie"));
}

#[test]
fn project_wildcards_n() {
    let proj = TestProject::new("wildcards-n");
    let build = ['a', 'b', 'c']
        .iter()
        .flat_map(|c| (1..4).into_iter().map(move |i| (c, i)))
        .fold(proj, |proj, (c, i)| {
            proj.song(
                format!("{}-{}.md", c, i),
                formatdoc! {"
                # Song

                1. {}-{}
            ", c, i},
            )
        })
        .output("songbook.html")
        .settings(|toml| {
            toml.set("songs", vec!["a-*.md", "b-*.md", "c-*.md"]);
        })
        .build()
        .unwrap();

    let html = build.read_output(".html");

    for c in ['a', 'b', 'c'] {
        for i in 1..4 {
            assert!(html.contains(&format!("{}-{}", c, i)));
        }
    }
}
