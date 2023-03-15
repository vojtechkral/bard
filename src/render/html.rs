use semver::Version;

use super::template::{DpiHelper, HbRender};
use super::{Render, RenderContext};
use crate::app::App;
use crate::prelude::*;
use crate::project::{Output, Project};
use crate::util::ImgCache;

default_template!(DEFAULT_TEMPLATE, "html.hbs");

pub struct RHtml(HbRender);

impl RHtml {
    pub fn new(project: &Project, output: &Output, img_cache: &ImgCache) -> Result<Self> {
        let mut hb = HbRender::new(project, output, &DEFAULT_TEMPLATE, img_cache)?;

        // Setup HTML-specific helpers
        hb.hb
            .register_helper("scale", DpiHelper::new(output, "scale"));

        Ok(Self(hb))
    }
}

impl Render for RHtml {
    fn render(&self, _app: &App, output: &Path, context: RenderContext) -> Result<()> {
        self.0.render(output, context)
    }

    fn version(&self) -> Option<Version> {
        self.0.version()
    }
}
