mod util;
use std::fs;

use bard::default_project::DEFAULT_PROJECT;
pub use util::*;

#[test]
fn init_and_build() {
    let _build = Builder::init_and_build("init").unwrap();
}

#[test]
fn init_doesnt_overwrite_1() {
    let test_content = "test\n";
    let work_dir = Builder::work_dir("init-overwrite-1", true).unwrap();
    let songs_dir = work_dir.join("songs");
    let test_file = songs_dir.join("yippie.md");
    let project_file = work_dir.join("bard.toml");
    fs::create_dir_all(&songs_dir).unwrap();
    fs::write(&test_file, test_content).unwrap();

    let app = Builder::app(false);
    bard::bard_init_at(&app, &work_dir).unwrap_err();

    let default_project = DEFAULT_PROJECT.resolve(&work_dir);
    default_project.files().find(|&f| f == test_file).unwrap();
    default_project
        .files()
        .find(|&f| f == project_file)
        .unwrap();

    let content_after = fs::read_to_string(&test_file).unwrap();
    assert_eq!(content_after, test_content);
    assert!(!project_file.exists());
}

#[test]
fn init_doesnt_overwrite_2() {
    let work_dir = Builder::work_dir("init-overwrite-2", true).unwrap();
    let out_dir = work_dir.join("output");
    let project_file = work_dir.join("bard.toml");
    fs::create_dir_all(&out_dir).unwrap();

    let app = Builder::app(false);
    bard::bard_init_at(&app, &work_dir).unwrap_err();

    let default_project = DEFAULT_PROJECT.resolve(&work_dir);
    default_project.dirs().find(|&d| d == out_dir).unwrap();
    assert!(out_dir.exists());
    assert!(!project_file.exists());
}
