use std::fs::File;

use semver::Version;

use super::{Render, RenderContext};
use crate::error::*;
use crate::project::{Output, Project};

pub struct RJson<'a> {
    project: &'a Project,
    output: &'a Output,
}

impl<'a> Render<'a> for RJson<'a> {
    fn new(project: &'a Project, output: &'a Output) -> Self {
        Self { project, output }
    }

    fn load(&mut self) -> Result<Option<Version>> {
        Ok(None)
    }

    fn render(&self) -> Result<()> {
        let context = RenderContext::new(self.project, self.output);
        let path = &self.output.file;

        File::create(path)
            .map_err(Error::from)
            .and_then(|mut f| serde_json::to_writer_pretty(&mut f, &context).map_err(Error::from))
            .with_context(|| format!("Error writing output file: `{}`", path))
    }
}
