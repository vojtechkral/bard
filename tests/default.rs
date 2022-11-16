use std::fs;

use bard::render;

mod util;
pub use util::*;

#[test]
fn project_default() {
    let _build = Builder::build(ROOT / "default").unwrap();
}

#[test]
#[ignore = "requires TeX distribution"]
fn project_default_postproess() {
    let _build = Builder::build_with_ps(ROOT / "default", "default-postprocess").unwrap();
}

#[test]
fn project_example() {
    let _build = Builder::build(ROOT / "example").unwrap();
}

#[test]
#[ignore = "requires TeX distribution"]
fn project_example_postproess() {
    let _build = Builder::build_with_ps(ROOT / "example", "example-postprocess").unwrap();
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

    let pdf = fs::read_to_string(templates.join("pdf.hbs")).unwrap();
    assert_eq!(pdf, render::pdf::DEFAULT_TEMPLATE.content);

    let hovorka = fs::read_to_string(templates.join("hovorka.hbs")).unwrap();
    assert_eq!(hovorka, render::hovorka::DEFAULT_TEMPLATE.content);
}
