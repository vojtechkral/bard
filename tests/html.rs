use std::fs;

use regex::Regex;

mod util;
pub use util::*;

#[test]
fn project_html() {
    let build = Builder::build(TEST_PROJECTS / "html").unwrap();
    let out_dir = build.project.settings.dir_output();

    let html = fs::read_to_string(out_dir.join("songbook.html"))
        .unwrap()
        .remove_newlines();
    let re = Regex::new("<foo>.*Yippie yea.*</foo>").unwrap();
    re.find(&html).unwrap();
    html.find(r#"<span style="color:red;">Yippie</span>"#)
        .unwrap();

    let tex = fs::read_to_string(out_dir.join("songbook.tex"))
        .unwrap()
        .remove_newlines();
    let re = Regex::new(r"\\begin\{foo\}.*Yippie.*yea.*\\end\{foo\}").unwrap();
    re.find(&tex).unwrap();
    tex.find(r"{\color{red}Yippie}").unwrap();
}
