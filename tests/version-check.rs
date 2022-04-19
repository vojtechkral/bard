use bard::book;
use bard::project::Project;
use bard::render::{Render, Renderer};

mod util;
use semver::{Comparator, Op, Prerelease, Version};
pub use util::*;

#[track_caller]
fn get_output_versions(project: &Project) -> Vec<Version> {
    // Imperative code so that track_caller works
    let mut res = vec![];
    for o in &project.settings.output {
        if let Some(ver) = Renderer::new(project, &o).load().unwrap() {
            res.push(ver);
        }
    }
    res
}

#[track_caller]
fn assert_project_compatible(project: &Project) {
    for v in get_output_versions(project) {
        let cmp = Comparator {
            op: Op::Caret,
            major: v.major,
            minor: Some(v.minor),
            patch: Some(v.patch),
            pre: Prerelease::EMPTY,
        };
        assert!(cmp.matches(&book::AST_VERSION));
    }
}

#[test]
fn version_check_load() {
    let build = Builder::build(TEST_PROJECTS / "version-check").unwrap();

    let expected = Version::new(1, 2, 3);
    for v in get_output_versions(&build.project) {
        assert_eq!(v, expected);
    }
}

#[test]
fn version_check_default_project() {
    let build =
        Builder::build_opts(ROOT / "default", "version-check-default", &OPTS_NO_PS).unwrap();
    assert_project_compatible(&build.project);
}

#[test]
fn version_check_example_project() {
    let build =
        Builder::build_opts(ROOT / "example", "version-check-example", &OPTS_NO_PS).unwrap();
    assert_project_compatible(&build.project);
}
