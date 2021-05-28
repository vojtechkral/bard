pub mod json;
pub mod template;

use anyhow::Result;
use serde::Serialize;

use crate::book::Song;
use crate::project::{Metadata, Output, Project};
use crate::ProgramMeta;

#[derive(Serialize, Debug)]
pub struct RenderContext<'a> {
    book: &'a Metadata,
    songs: &'a [Song],
    output: &'a Metadata,
    program: ProgramMeta,
}

pub trait Render {
    fn render<'a>(project: &'a Project, output: &'a Output) -> Result<&'a Output>;
}

pub use self::json::RJson;
pub use self::template::{DefaultTemaplate, RHovorka, RHtml, RTex};
