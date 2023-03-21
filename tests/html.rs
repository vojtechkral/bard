use bard::{
    parser::{html::RESERVED_TAGS, DiagKind},
    render::{html, pdf},
};

mod util_ng;
pub use util_ng::*;

#[test]
fn project_html() {
    #[rustfmt::skip]
    let build = TestProject::new("html")
        .output("songbook.pdf")
        .output("songbook.html")
        .song("song.md", indoc! {r#"
            # Song

            <foo>

            1. `Am`Yippie yea `C`oh!</foo>
            <bar color="red">Yippie</bar> yea `Am`yay!
            "#},
        )
        .template_prefix_default("songbook.html", "html.hbs", indoc! {r#"
            {{#*inline "h-foo"}}<foo>{{/inline}}
            {{#*inline "h-/foo"}}</foo>{{/inline}}

            {{#*inline "h-bar"}}<span style="color:{{ color }};">{{/inline}}
            {{#*inline "h-/bar"}}</span>{{/inline}}
            "#},
            &html::DEFAULT_TEMPLATE,
        )
        .template_prefix_default("songbook.pdf", "pdf.hbs", indoc! {r#"
            \newenvironment{foo}{}{}
            {{#*inline "h-foo"}}\begin{foo}{{/inline}}
            {{#*inline "h-/foo"}}\end{foo}{{/inline}}

            {{#*inline "h-bar"}}{\color{ {{~ color ~}} }{{/inline}}
            {{#*inline "h-/bar"}}}{{/inline}}
            "#},
            &pdf::DEFAULT_TEMPLATE,
        )
        .build()
        .unwrap();

    let html = build.read_output(".html").remove_newlines();
    html.find_re("<foo>.*Yippie yea.*</foo>").unwrap();
    html.find(r#"<span style="color:red;">Yippie</span>"#)
        .unwrap();

    let tex = build.read_output(".tex").remove_newlines();
    eprintln!("{}", tex);
    tex.find_re(r"\\begin\{foo\}.*Yippie.*yea.*\\end\{foo\}")
        .unwrap();
    tex.find(r"{\color{red}Yippie}").unwrap();
}

#[test]
fn project_html_reserved_tags() {
    let song = RESERVED_TAGS
        .iter()
        .enumerate()
        .fold("# Song\n".to_string(), |mut s, (i, tag)| {
            s.push_str(&format!("\n{}. <{}>text.\n", i + 1, tag));
            s
        });

    let build = TestProject::new("html-reserved-tags")
        .output("songbook.html")
        .song("song.md", song)
        .build()
        .unwrap();

    build.unwrap_err();
    build.assert_parser_diag(DiagKind::HtmlReservedTag { tag: "html".into() });
    build.assert_parser_diag(DiagKind::HtmlReservedTag { tag: "tex".into() });
}
