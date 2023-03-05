mod util;
use std::{fs, path::PathBuf};

pub use util::*;

#[test]
fn project_toc_sort_off() {
    let build = Builder::build(TEST_PROJECTS / "toc-sort").unwrap();
    let html = build.project.settings.dir_output().join("songbook.html");
    let html = fs::read_to_string(&html).unwrap();
    let (pos1, pos2, pos3, pos4) = (
        html.find("Song A").unwrap(),
        html.find("Song B").unwrap(),
        html.find("Song C").unwrap(),
        html.find("Song Č").unwrap(),
    );

    // the order in bard.toml is descending
    assert!(pos1 > pos2);
    assert!(pos2 > pos3);
    assert!(pos3 > pos4);
}

fn project_toc_sort(name: &str) -> PathBuf {
    let project_dir = prepare_project(TEST_PROJECTS / "toc-sort", name).unwrap();
    modify_settings(&project_dir, |mut settings| {
        let outputs = settings["output"].as_array_mut().unwrap();
        for output in outputs.iter_mut() {
            let output = output.as_table_mut().unwrap();
            output.insert("toc_sort".to_string(), true.into());
        }
        Ok(settings)
    })
    .unwrap();

    project_dir
}

#[test]
fn project_toc_sort_html() {
    let app = Builder::app(false);
    let project_dir = project_toc_sort("toc-sort-html");
    let project = bard::bard_make_at(&app, &project_dir).unwrap();
    let html = project.settings.dir_output().join("songbook.html");
    let html = fs::read_to_string(&html).unwrap();
    let (pos1, pos2, pos3, pos4) = (
        html.find("Song A").unwrap(),
        html.find("Song B").unwrap(),
        html.find("Song C").unwrap(),
        html.find("Song Č").unwrap(),
    );

    // order should now be ascending
    assert!(pos1 < pos2);
    assert!(pos2 < pos3);
    assert!(pos3 < pos4);
}

#[test]
#[ignore = "requires poppler/pdftotext"]
fn project_toc_sort_pdf() {
    let app = Builder::app(true);
    let project_dir = project_toc_sort("toc-sort-pdf");
    let project = bard::bard_make_at(&app, &project_dir).unwrap();
    let pdf = project.settings.dir_output().join("songbook.pdf");
    let pdf_text = pdf_to_text(&pdf, ..3).unwrap();
    let (pos1, pos2, pos3, pos4) = (
        pdf_text.find("Song A").unwrap(),
        pdf_text.find("Song B").unwrap(),
        pdf_text.find("Song C").unwrap(),
        pdf_text.find("Song Č").unwrap(),
    );

    // order should now be ascending
    assert!(pos1 < pos2);
    assert!(pos2 < pos3);
    assert!(pos3 < pos4);
}
