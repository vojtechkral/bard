mod util;
pub use util::*;

#[test]
fn project_smart_punctuation_default() {
    let build = Builder::build(TEST_PROJECTS / "smart-punctuation").unwrap();
    let html = build.project.output_paths().next().unwrap();
    assert_file_contains(html, "‘Hello,’ “world” …");
}

#[test]
fn project_smart_punctuation_off() {
    let app = Builder::app(false);
    let project_dir =
        prepare_project(TEST_PROJECTS / "smart-punctuation", "smart-punctuation-off").unwrap();
    modify_settings(&project_dir, |mut settings| {
        settings.insert("smart_punctuation".to_string(), false.into());
        Ok(settings)
    })
    .unwrap();

    let project = bard::bard_make_at(&app, &project_dir).unwrap();
    let html = project.output_paths().next().unwrap();
    assert_file_contains(html, "&#x27;Hello,&#x27; &quot;world&quot; ...");
}
