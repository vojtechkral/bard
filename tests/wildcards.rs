mod util;
use std::fs;

pub use util::*;

#[test]
fn project_wildcards_1() {
    let build = Builder::build(TEST_PROJECTS / "wildcards-1").unwrap();
    let html = build.project.output_paths().next().unwrap();
    assert_file_contains(html, "Yippie");
}

#[test]
fn project_wildcards_n() {
    let build = Builder::build(TEST_PROJECTS / "wildcards-n").unwrap();

    let html = build.project.output_paths().next().unwrap();
    let html = fs::read_to_string(html).unwrap();

    for c in ['a', 'b', 'c'] {
        for i in 1..4 {
            assert!(html.contains(&format!("{}-{}", c, i)));
        }
    }
}
