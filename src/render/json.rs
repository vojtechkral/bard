use std::fs::File;

use crate::project::{Project, OutputSpec, Metadata};
use crate::book::Song;
use crate::parser::ParsingDebug;
use crate::{ProgramMeta, PROGRAM_META};
use crate::error::*;
use super::Render;


#[derive(Serialize, Debug)]
struct JsonDoc<'a> {
    book: &'a Metadata,
    songs: &'a [Song],
    output: &'a Metadata,
    program: ProgramMeta,
    #[serde(skip_serializing_if = "Option::is_none")]
    debug: Option<&'a ParsingDebug>,
}

pub struct RJson;

impl Render for RJson {
    fn render<'a>(project: &'a Project, output: &'a OutputSpec) -> Result<&'a OutputSpec> {
        let json_doc = JsonDoc {
            book: project.metadata(),
            songs: project.songs(),
            output: &output.metadata,
            program: PROGRAM_META,
            debug: project.parsing_debug(),
        };

        let path = &output.file;

        let mut file = File::create(&path).map_err(|err| ErrorWritingFile(path.to_owned(), err))?;

        serde_json::to_writer_pretty(&mut file, &json_doc)
            .map_err(|err| ErrorWritingFile(path.to_owned(), err.into()))?;

        Ok(output)
    }
}
