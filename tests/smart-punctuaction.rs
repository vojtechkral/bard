mod util_ng;
pub use util_ng::*;

#[rustfmt::skip]
fn prepare_project(name: &str) -> TestProject {
    TestProject::new(name)
        .song("punctuation.md", indoc! {r#"
            # Smart Punctuation

            1. 'Hello,' "world" ...
        "#},
        )
        .output("songbook.html")
        .output("songbook.json")
}

#[test]
fn project_smart_punctuation_default() {
    let build = prepare_project("smart-punctuation-default")
        .build()
        .unwrap();

    assert!(build.read_output(".html").contains("‘Hello,’ “world” …"));
    assert!(build.read_output(".json").contains("‘Hello,’ “world” …"));
}

#[test]
fn project_smart_punctuation_off() {
    let build = prepare_project("smart-punctuation-off")
        .settings(|toml| toml.set("smart_punctuation", false))
        .build()
        .unwrap();

    assert!(build
        .read_output(".html")
        .contains("&#x27;Hello,&#x27; &quot;world&quot; ..."));
    assert!(build
        .read_output(".json")
        .contains(r#"'Hello,' \"world\" ..."#));
}
