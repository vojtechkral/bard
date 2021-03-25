use std::fs::File;

use crate::project::{Project, Output};
use crate::PROGRAM_META;
use crate::error::*;
use super::{Render, RenderContext};

pub struct RJson;

impl Render for RJson {
    fn render<'a>(project: &'a Project, output: &'a Output) -> Result<&'a Output> {
        let context = RenderContext {
            book: project.metadata(),
            songs: project.songs(),
            output: &output.metadata,
            program: PROGRAM_META,
        };

        let path = &output.file;

        let mut file = File::create(&path).map_err(|err| ErrorWritingFile(path.to_owned(), err))?;

        serde_json::to_writer_pretty(&mut file, &context)
            .map_err(|err| ErrorWritingFile(path.to_owned(), err.into()))?;

        Ok(output)
    }
}
