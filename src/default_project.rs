use std::path::{Path, PathBuf};
use std::io;
use std::fs;

use crate::project::PROJECT_FILE;
use crate::render::{DefaultTemaplate, RHtml, RTex};

#[derive(Debug)]
pub struct FileContent {
    name: &'static str,
    content: &'static str,
}

impl FileContent {
    pub fn path(&self, base: &Path) -> PathBuf {
        base.join(self.name)
    }

    pub fn create(&self, base: &Path) -> io::Result<()> {
        let path = self.path(base);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(path, self.content.as_bytes())
    }
}

pub static DEFAULT_PROJECT: &'static [FileContent] = &[
    FileContent {
        name: PROJECT_FILE,
        content: include_str!("../default/bard.toml"),
    },
    FileContent {
        name: RTex::TPL_NAME,
        content: RTex::TPL_CONTENT,
    },
    FileContent {
        name: RHtml::TPL_NAME,
        content: RHtml::TPL_CONTENT,
    },
    FileContent {
        name: "songs/yippie.md",
        content: include_str!("../default/songs/yippie.md"),
    },
];
