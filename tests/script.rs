use std::collections::HashMap;

mod util_ng;
pub use util_ng::*;

fn prepare_project(name: &str, postprocess: bool) -> TestProject {
    TestProject::new(name)
        .postprocess(postprocess)
        .output("songbook.html")
        .script(
            ".html",
            "script",
            indoc! {r#"
            #!/bin/sh

            echo "BARD = \"${BARD}\"
            OUTPUT = \"${OUTPUT}\"
            PROJECT_DIR = \"${PROJECT_DIR}\"
            OUTPUT_DIR = \"${OUTPUT_DIR}\"
            " > "${OUTPUT_STEM}.toml"

            "#},
            indoc! {r#"
            @ECHO OFF

            rem Windows paths contain backslashes - we need to be escape them for JSON:
            set BARD=%BARD:\=\\%
            set OUTPUT=%OUTPUT:\=\\%
            set PROJECT_DIR=%PROJECT_DIR:\=\\%
            set OUTPUT_DIR=%OUTPUT_DIR:\=\\%

            (
            echo BARD = "%BARD%"
            echo OUTPUT = "%OUTPUT%"
            echo PROJECT_DIR = "%PROJECT_DIR%"
            echo OUTPUT_DIR = "%OUTPUT_DIR%"
            ) > "%OUTPUT_STEM%.toml"

            "#},
        )
}

#[test]
fn project_script() {
    let build = prepare_project("script", true).build().unwrap();

    let out = build.read_output("songbook.toml");
    let out: HashMap<String, String> = toml::from_str(&out).unwrap();

    assert_eq!(out["BARD"], build.app().bard_exe().to_str().unwrap());
    assert_eq!(
        out["OUTPUT"],
        build.dir_output().join("songbook.html").to_str().unwrap()
    );
    assert_eq!(
        out["PROJECT_DIR"],
        build.unwrap().project_dir.to_str().unwrap()
    );
    assert_eq!(out["OUTPUT_DIR"], build.dir_output().to_str().unwrap());
}

#[test]
fn project_script_no_ps() {
    let build = prepare_project("script-no-ps", false).build().unwrap();
    build.try_read_output(".toml").unwrap_err();
}

#[test]
fn project_script_fail() {
    TestProject::new("script-fail")
        .postprocess(true)
        .output("songbook.html")
        .script(
            ".html",
            "script",
            indoc! {r#"
            #!/bin/sh
            kill $$
            "#},
            indoc! {r#"
            @ECHO OFF
            exit 1
            "#},
        )
        .build()
        .unwrap()
        .unwrap_err();
}
