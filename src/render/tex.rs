use handlebars::handlebars_helper;
use semver::Version;

use super::template::HbRender;
use super::Render;
use crate::error::*;
use crate::project::{Output, Project};

default_template!(DEFAULT_TEMPLATE, "pdf.hbs");

fn latex_escape(input: &str, pre_spaces: bool) -> String {
    let mut res = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            ' ' if pre_spaces => res.push('~'),
            '&' | '%' | '$' | '#' | '_' | '{' | '}' => {
                res.push('\\');
                res.push(c);
            }
            '[' => res.push_str("{\\lbrack}"),
            ']' => res.push_str("{\\rbrack}"),
            '~' => res.push_str("{\\textasciitilde}"),
            '^' => res.push_str("{\\textasciicircum}"),
            '\\' => res.push_str("{\\textbackslash}"),
            c => res.push(c),
        }
    }

    res
}

fn hb_latex_escape(input: &str) -> String {
    latex_escape(input, false)
}

handlebars_helper!(hb_pre: |input: str| {
    latex_escape(input, true)
});

pub struct RTex(HbRender);

impl RTex {
    pub fn new(project: &Project, output: &Output) -> Result<Self> {
        let mut render = HbRender::new(project, output, &DEFAULT_TEMPLATE)?;

        // Setup Latex escaping
        render.hb.register_escape_fn(hb_latex_escape);
        render.hb.register_helper("pre", Box::new(hb_pre));

        Ok(Self(render))
    }
}

impl Render for RTex {
    fn render(&self, project: &Project, output: &Output) -> Result<()> {
        self.0.render(project, output)
    }

    fn version(&self) -> Option<Version> {
        self.0.version()
    }
}
