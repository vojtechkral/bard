pub mod json;
pub mod template;

use anyhow::Result;
use semver::Version;
use serde::Serialize;

use crate::book::{Song, SongRef};
use crate::music::Notation;
use crate::project::{Metadata, Output, Project};
use crate::{ProgramMeta, PROGRAM_META};

#[derive(Serialize, Debug)]
pub struct RenderContext<'a> {
    book: &'a Metadata,
    songs: &'a [Song],
    songs_sorted: &'a [SongRef],
    notation: Notation,
    output: &'a Metadata,
    program: &'static ProgramMeta,
}

impl<'a> RenderContext<'a> {
    fn new(project: &'a Project, output: &'a Output) -> Self {
        RenderContext {
            book: project.metadata(),
            songs: project.songs(),
            songs_sorted: project.songs_sorted(),
            notation: project.settings.notation,
            output: &output.metadata,
            program: &PROGRAM_META,
        }
    }
}

pub trait Render<'a>: Sized {
    fn new(project: &'a Project, output: &'a Output) -> Self;
    /// Load the template file (if any) and return the AST version specified.
    fn load(&mut self) -> Result<Option<Version>>;
    fn render(&self) -> Result<()>;
}

pub use self::json::RJson;
pub use self::template::{DefaultTemaplate, RHovorka, RHtml, RTex};
