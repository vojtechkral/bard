use std::fs;

use bard::render::{DefaultTemaplate, RHovorka, RHtml, RTex};

mod util;
pub use util::*;

#[test]
fn project_default() {
    let _build = Builder::build(ROOT / "default").unwrap();
}

#[test]
fn project_example() {
    let _build = Builder::build(ROOT / "example").unwrap();
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
    assert_eq!(html, RHtml::TPL_CONTENT);

    let tex = fs::read_to_string(templates.join("pdf.hbs")).unwrap();
    assert_eq!(tex, RTex::TPL_CONTENT);

    let hovorka = fs::read_to_string(templates.join("hovorka.hbs")).unwrap();
    assert_eq!(hovorka, RHovorka::TPL_CONTENT);
}
