use std::fs::File;

use super::{Render, RenderContext};
use crate::app::App;
use crate::prelude::*;

#[derive(Debug, Default)]
pub struct RJson;

impl RJson {
    pub fn new() -> Self {
        Self
    }
}

impl Render for RJson {
    fn render(&self, _app: &App, output: &Path, context: RenderContext) -> Result<()> {
        File::create(output)
            .map_err(Error::from)
            .and_then(|mut f| serde_json::to_writer_pretty(&mut f, &context).map_err(Error::from))
            .with_context(|| format!("Error writing output file: {:?}", output))
    }
}
