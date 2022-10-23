use std::fs::File;

use super::{Render, RenderContext};
use crate::error::*;
use crate::project::{Output, Project};

#[derive(Debug, Default)]
pub struct RJson;

impl RJson {
    pub fn new() -> Self {
        Self
    }
}

impl Render for RJson {
    fn render(&self, project: &Project, output: &Output) -> Result<()> {
        let context = RenderContext::new(project, output);
        let path = &output.file;

        File::create(path)
            .map_err(Error::from)
            .and_then(|mut f| serde_json::to_writer_pretty(&mut f, &context).map_err(Error::from))
            .with_context(|| format!("Error writing output file: `{}`", path))
    }
}
