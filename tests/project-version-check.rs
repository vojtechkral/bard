mod util;
pub use util::*;

#[test]
fn project_version_check_1_implicit() {
    let err = Builder::init_modify_build("project-version-1-implicit", |mut settings| {
        settings.remove("version");
        Ok(settings)
    })
    .unwrap_err();
    assert!(
        format!("{:?}", err).contains("1.x"),
        "actual error: {}",
        err
    );
}

#[test]
fn project_version_check_1_explicit() {
    let err = Builder::init_modify_build("project-version-1-explicit", |mut settings| {
        settings.insert("version".to_string(), 1.into());
        Ok(settings)
    })
    .unwrap_err();
    assert!(
        format!("{:?}", err).contains("1.x"),
        "actual error: {}",
        err
    );
}

#[test]
fn project_version_check_9001() {
    let err = Builder::init_modify_build("project-version-9001", |mut settings| {
        settings.insert("version".to_string(), 9001.into());
        // Also insert some junk key to verify the version check really only
        // about the version field:
        settings.insert(
            "some-nonsensical-unsupported-key-blablabla".to_string(),
            true.into(),
        );
        Ok(settings)
    })
    .unwrap_err();
    assert!(
        format!("{:?}", err).contains("9001.x"),
        "actual error: {}",
        err
    );
}
