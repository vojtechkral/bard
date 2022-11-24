use handlebars::handlebars_helper;
use semver::Version;

use super::template::HbRender;
use super::tex_tools::TexTools;
use super::{Render, RenderContext};
use crate::app::App;
use crate::prelude::*;
use crate::project::{Output, Project};
use crate::render::tex_tools::TexRenderJob;

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

pub struct RPdf {
    hb: HbRender,
    toc_sort_key: Option<String>,
}

impl RPdf {
    pub fn new(project: &Project, output: &Output) -> Result<Self> {
        let mut hb = HbRender::new(project, output, &DEFAULT_TEMPLATE)?;

        // Setup Latex escaping
        hb.hb.register_escape_fn(hb_latex_escape);
        hb.hb.register_helper("pre", Box::new(hb_pre));

        let toc_sort_key = output
            .metadata
            .get("toc_sort_key")
            .and_then(|val| val.as_str().map(|s| s.to_string()));

        Ok(Self { hb, toc_sort_key })
    }
}

impl Render for RPdf {
    fn render(&self, app: &App, output: &Path, context: RenderContext) -> Result<()> {
        // Render TeX first
        let mut job = TexRenderJob::new(output, app.keep_interm(), self.toc_sort_key.as_deref())?;
        self.hb.render(&job.tex_file, context)?;

        if !app.post_process() {
            // TODO: test this
            job.tex_file.set_remove(false);
            return Ok(());
        }

        // Run TeX
        TexTools::get().render_pdf(app, job)
    }

    fn version(&self) -> Option<Version> {
        self.hb.version()
    }
}
