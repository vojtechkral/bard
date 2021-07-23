mod util;
pub use util::*;

#[test]
fn project_nonexistent_file() {
    let err = Builder::build(TEST_PROJECTS / "nonexistent-file").unwrap_err();
    let cause = format!("{}", err.root_cause());
    cause.find("no-such-file.md").unwrap();
}
