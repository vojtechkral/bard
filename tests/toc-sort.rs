mod util_ng;
pub use util_ng::*;

#[rustfmt::skip]
fn prepare_project(name: &str) -> TestProject {
    TestProject::new(name)
        .song("č.md", indoc! {"
        # Song Č

        Foo bar.
        "},
        )
        .song("c.md", indoc! {"
        # Song C

        Foo bar.
        "},
        )
        .song("b.md", indoc! {"
        # Song B

        Foo bar.
        "},
        )
        .song("a.md", indoc! {"
        # Song A

        Foo bar.
        "},
        )
}

#[test]
fn project_toc_sort_off() {
    let build = prepare_project("toc-sort-off")
        .output("songbook.html")
        .build()
        .unwrap();
    let html = build.read_output(".html");
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

#[test]
fn project_toc_sort_html() {
    let build = prepare_project("toc-sort-html")
        .output_toml(toml! {
            file = "songbook.html"
            toc_sort = true
        })
        .build()
        .unwrap();
    let html = build.read_output(".html");
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
    let build = prepare_project("toc-sort-pdf")
        .postprocess(true)
        .output_toml(toml! {
            file = "songbook.pdf"
            toc_sort = true
        })
        .build()
        .unwrap();
    let pdf_text = build.pdf_to_text(".pdf", ..3).unwrap();

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
