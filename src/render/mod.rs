pub mod template;
pub mod json;
pub mod txt;

use anyhow::Result;

use crate::project::{Project, Output};


pub trait Render {
    fn render<'a>(project: &'a Project, output: &'a Output) -> Result<&'a Output>;
}

pub use self::template::{DefaultTemaplate, RHtml, RTex};
pub use self::json::RJson;
pub use self::txt::RTxt;
