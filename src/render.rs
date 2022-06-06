use anyhow::Result;
use semver::Version;
use serde::Serialize;

use crate::book::{Song, SongRef};
use crate::music::Notation;
use crate::project::{Format, Metadata, Output, Project};
use crate::{ProgramMeta, PROGRAM_META};

pub mod json;
pub mod template;
pub mod xml;

pub use self::json::RJson;
pub use self::template::*;
pub use self::xml::RXml;

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

pub enum Renderer<'a> {
    Html(RHtml<'a>),
    Tex(RTex<'a>),
    Hovorka(RHovorka<'a>),
    Json(RJson<'a>),
    Xml(RXml<'a>),
}

impl<'a> Render<'a> for Renderer<'a> {
    fn new(project: &'a Project, output: &'a Output) -> Self {
        match output.format {
            Format::Html => Self::Html(RHtml::new(project, output)),
            Format::Tex => Self::Tex(RTex::new(project, output)),
            Format::Hovorka => Self::Hovorka(RHovorka::new(project, output)),
            Format::Json => Self::Json(RJson::new(project, output)),
            Format::Xml => Self::Xml(RXml::new(project, output)),
            Format::Auto => Format::no_auto(),
        }
    }

    fn load(&mut self) -> Result<Option<Version>> {
        match self {
            Self::Html(r) => r.load(),
            Self::Tex(r) => r.load(),
            Self::Hovorka(r) => r.load(),
            Self::Json(r) => r.load(),
            Self::Xml(r) => r.load(),
        }
    }

    fn render(&self) -> Result<()> {
        match self {
            Self::Html(r) => r.render(),
            Self::Tex(r) => r.render(),
            Self::Hovorka(r) => r.render(),
            Self::Json(r) => r.render(),
            Self::Xml(r) => r.render(),
        }
    }
}
