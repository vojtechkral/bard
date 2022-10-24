use semver::Version;

use super::template::HbRender;
use super::{Render, RenderContext};
use crate::prelude::*;
use crate::project::{Output, Project};

default_template!(DEFAULT_TEMPLATE, "hovorka.hbs");

pub struct RHovorka(HbRender);

impl RHovorka {
    pub fn new(project: &Project, output: &Output) -> Result<Self> {
        Ok(Self(HbRender::new(project, output, &DEFAULT_TEMPLATE)?))
    }
}

impl Render for RHovorka {
    fn render(&self, output: &Path, context: RenderContext) -> Result<()> {
        self.0.render(output, context)
    }

    fn version(&self) -> Option<Version> {
        self.0.version()
    }
}
