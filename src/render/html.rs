use semver::Version;
use camino::Utf8Path as Path;

use super::template::HbRender;
use super::{Render, RenderContext};
use crate::error::*;
use crate::project::{Output, Project};

default_template!(DEFAULT_TEMPLATE, "html.hbs");

pub struct RHtml(HbRender);

impl RHtml {
    pub fn new(project: &Project, output: &Output) -> Result<Self> {
        Ok(Self(HbRender::new(project, output, &DEFAULT_TEMPLATE)?))
    }
}

impl Render for RHtml {
    fn render(&self, output: &Path, context: RenderContext) -> Result<()> {
        self.0.render(output, context)
    }

    fn version(&self) -> Option<Version> {
        self.0.version()
    }
}
