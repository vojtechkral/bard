mod util_ng;
pub use util_ng::*;

#[test]
fn project_nonexistent_file() {
    let build = TestProject::new("nonexistent-file")
        .settings(|toml| toml.set("songs", vec!["no-such-file.md"]))
        .build()
        .unwrap();
    let err = build.unwrap_err();

    let cause = format!("{}", err.root_cause());
    cause.find("no-such-file.md").unwrap();
}
