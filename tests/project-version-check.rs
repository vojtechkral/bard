mod util;
pub use util::*;

#[test]
fn project_version_check() {
    let err = Builder::build(TEST_PROJECTS / "project-version-1").unwrap_err();
    assert!(format!("{:?}", err).contains("1.x"));

    let err = Builder::build(TEST_PROJECTS / "project-version-9001").unwrap_err();
    assert!(format!("{:?}", err).contains("9001.x"));
}
