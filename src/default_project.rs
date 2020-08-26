use std::path::{Path, PathBuf};
use std::fs;
use std::iter;

use lazy_static::lazy_static;

use crate::project::{PROJECT_FILE, DIR_SONGS, DIR_TEMPLATES};
use crate::render::{DefaultTemaplate, RHtml, RTex};
use crate::util::PathBufExt as _;
use crate::error::*;

/// File: Contains a filename and the file's content
#[derive(Debug)]
struct File {
    path: PathBuf,
    content: &'static [u8],
}

impl File {
    fn new(path: &str, content: &'static str) -> Self {
        Self {
            path: path.into(),
            content: content.as_bytes(),
        }
    }

    fn exists(&self) -> bool {
        self.path.exists()
    }

    fn resolved(&self, base: &Path) -> Self {
        Self {
            path: self.path.clone().resolved(base),
            content: self.content,
        }
    }

    fn create(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Could not create directory `{}`", parent.display()))?;
        }

        fs::write(&self.path, self.content)
            .with_context(|| format!("Could not initialize file `{}`", self.path.display()))
    }
}


#[derive(Debug)]
pub struct DefaultProject {
    project_file: File,
    songs: Box<[File]>,
    templates: Box<[File]>,
}

impl DefaultProject {
    fn new() -> Self {
        let project_file = File::new(PROJECT_FILE, include_str!("../default/bard.toml"));

        let songs = vec![File::new(
            "yippie.md",
            include_str!("../default/songs/yippie.md"),
        )]
        .into();

        let templates = vec![
            File::new(RTex::TPL_NAME, RTex::TPL_CONTENT),
            File::new(RHtml::TPL_NAME, RHtml::TPL_CONTENT),
        ]
        .into();

        Self {
            project_file,
            songs,
            templates,
        }
    }

    pub fn resolve(&self, project_dir: &Path) -> DefaultProjectResolved {
        let dir_songs = project_dir.join(DIR_SONGS);
        let dir_templates = project_dir.join(DIR_TEMPLATES);

        let project_file = self.project_file.resolved(project_dir);
        let songs = self.songs.iter().map(|f| f.resolved(&dir_songs)).collect();
        let templates = self
            .templates
            .iter()
            .map(|f| f.resolved(&dir_templates))
            .collect();

        DefaultProjectResolved(Self {
            project_file,
            songs,
            templates,
        })
    }

    fn files<'a>(&'a self) -> impl Iterator<Item = &'a File> {
        iter::once(&self.project_file)
            .chain(self.songs.iter())
            .chain(self.templates.iter())
    }

    fn any_exists<'a>(&'a self) -> Option<&'a File> {
        self.files().find(|&f| f.exists())
    }
}

pub struct DefaultProjectResolved(DefaultProject);

impl DefaultProjectResolved {
    pub fn create(self) -> Result<()> {
        let project = self.0;

        if let Some(existing) = project.any_exists() {
            bail!("File already exists: '{}'", existing.path.display());
        }

        for file in project.files() {
            file.create()?;
        }

        Ok(())
    }
}

lazy_static! {
    pub static ref DEFAULT_PROJECT: DefaultProject = DefaultProject::new();
}
