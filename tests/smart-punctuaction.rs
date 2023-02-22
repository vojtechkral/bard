use std::fs;

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
    let work_dir =
        Builder::prepare(TEST_PROJECTS / "smart-punctuation", "smart-punctuation-off").unwrap();
    let bard_toml = work_dir.join("bard.toml");
    let mut settings = fs::read_to_string(&bard_toml).unwrap();
    settings.insert_str(0, "smart_punctuation = false\n");
    fs::write(&bard_toml, settings.as_bytes()).unwrap();
    let project = bard::bard_make_at(&app, &work_dir).unwrap();
    let html = project.output_paths().next().unwrap();
    assert_file_contains(html, "&#x27;Hello,&#x27; &quot;world&quot; ...");
}
