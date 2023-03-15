use handlebars::handlebars_helper;
use semver::Version;

use super::template::{DpiHelper, HbRender};
use super::tex_tools::TexTools;
use super::{Render, RenderContext};
use crate::app::App;
use crate::prelude::*;
use crate::project::{Output, Project};
use crate::render::tex_tools::TexRenderJob;
use crate::util::ImgCache;

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
    tex_runs: u32,
}

impl RPdf {
    pub fn new(project: &Project, output: &Output, img_cache: &ImgCache) -> Result<Self> {
        let mut hb = HbRender::new(project, output, &DEFAULT_TEMPLATE, img_cache)?;

        // Setup TeX escaping and TeX-specific helpers
        hb.hb.register_escape_fn(hb_latex_escape);
        hb.hb.register_helper("pre", Box::new(hb_pre));
        hb.hb
            .register_helper("px2mm", DpiHelper::new(output, "px2mm"));

        Ok(Self {
            hb,
            toc_sort_key: output.toc_sort.then(|| output.toc_sort_key.clone()),
            tex_runs: output.tex_runs,
        })
    }
}

impl Render for RPdf {
    fn render(&self, app: &App, output: &Path, context: RenderContext) -> Result<()> {
        // Render TeX first
        let tex_file = output.with_extension("tex");
        self.hb.render(&tex_file, context)?;
        if self.tex_runs == 0 || !app.post_process() {
            // TODO: test this
            return Ok(());
        }

        // Run TeX
        let job = TexRenderJob::new(
            tex_file,
            output,
            app.keep_interm(),
            self.toc_sort_key.as_deref(),
            self.tex_runs - 1,
        )?;
        TexTools::get().render_pdf(app, job)
    }

    fn version(&self) -> Option<Version> {
        self.hb.version()
    }
}
