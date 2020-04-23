pub mod template;
pub mod json;
pub mod txt;

use anyhow::Result;

use crate::project::{Project, OutputSpec};


pub trait Render {
    fn render<'a>(project: &'a Project, output: &'a OutputSpec) -> Result<&'a OutputSpec>;
}

pub use self::template::{DefaultTemaplate, RHtml, RTex};
pub use self::json::RJson;
pub use self::txt::RTxt;
