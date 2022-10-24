use std::fs;

use bard::render;

mod util;
pub use util::*;

#[test]
fn project_default() {
    let _build = Builder::build(ROOT / "default").unwrap();
}

#[test]
#[ignore = "requires TeX distribution due to post-processing"]
fn project_default_postproess() {
    let _build = Builder::build_opts(ROOT / "default", "default-postprocess", &OPTS_PS).unwrap();
}

#[test]
fn project_example() {
    let _build = Builder::build(ROOT / "example").unwrap();
}

#[test]
#[ignore = "requires TeX distribution due to post-processing"]
fn project_example_postproess() {
    let _build = Builder::build_opts(ROOT / "example", "example-postprocess", &OPTS_PS).unwrap();
}

#[test]
fn project_default_templates() {
    let _build = Builder::build(TEST_PROJECTS / "default-templates").unwrap();
}

#[test]
fn project_default_templates_save() {
    let build = Builder::build(TEST_PROJECTS / "default-templates-save").unwrap();
    let templates = build.dir.join("templates");

    let html = fs::read_to_string(templates.join("html.hbs")).unwrap();
    assert_eq!(html, render::html::DEFAULT_TEMPLATE.content);

    let tex = fs::read_to_string(templates.join("pdf.hbs")).unwrap();
    assert_eq!(tex, render::tex::DEFAULT_TEMPLATE.content);

    let hovorka = fs::read_to_string(templates.join("hovorka.hbs")).unwrap();
    assert_eq!(hovorka, render::hovorka::DEFAULT_TEMPLATE.content);
}
