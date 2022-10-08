use anyhow::Result;
use semver::Version;
use serde::Serialize;

use crate::book::{Song, SongRef};
use crate::music::Notation;
use crate::project::{BookSection, Format, Metadata, Output, Project};
use crate::{ProgramMeta, PROGRAM_META};

pub mod json;
pub mod template;
pub mod xml;

pub use self::json::RJson;
pub use self::template::*;
pub use self::xml::RXml;

#[derive(Serialize, Debug)]
pub struct RenderContext<'a> {
    book: &'a BookSection,
    songs: &'a [Song],
    songs_sorted: &'a [SongRef],
    notation: Notation,
    output: &'a Metadata,
    program: &'static ProgramMeta,
}

impl<'a> RenderContext<'a> {
    fn new(project: &'a Project, output: &'a Output) -> Self {
        RenderContext {
            book: project.book_section(),
            songs: project.songs(),
            songs_sorted: project.songs_sorted(),
            notation: project.settings.notation,
            output: &output.metadata,
            program: &PROGRAM_META,
        }
    }
}

trait Render {
    /// Render the output file based on `project` and `output`.
    fn render(&self, project: &Project, output: &Output) -> Result<()>;

    /// Returns the AST version specified in the template, if any.
    fn version(&self) -> Option<Version> {
        None
    }
}

pub struct Renderer<'a> {
    project: &'a Project,
    output: &'a Output,
    render: Box<dyn Render>,
}

impl<'a> Renderer<'a> {
    pub fn new(project: &'a Project, output: &'a Output) -> Result<Self> {
        let render: Box<dyn Render> = match output.format {
            Format::Html => Box::new(RHtml::new(project, output)?),
            Format::Tex => Box::new(RTex::new(project, output)?),
            Format::Hovorka => Box::new(RHovorka::new(project, output)?),
            Format::Json => Box::new(RJson::new()),
            Format::Xml => Box::new(RXml::new()),
            Format::Auto => Format::no_auto(),
        };

        Ok(Self {
            project,
            output,
            render,
        })
    }

    pub fn version(&self) -> Option<Version> {
        self.render.version()
    }

    pub fn render(&self) -> Result<()> {
        self.render.render(self.project, self.output)
    }
}
