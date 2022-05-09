use std::fs;
use std::path::MAIN_SEPARATOR;

use camino::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use lazy_static::lazy_static;

use crate::error::*;
use crate::project::{DIR_SONGS, PROJECT_FILE};
use crate::util::PathBufExt as _;

/// A filesystem node, either a file (with content), or a directory.
#[derive(Debug)]
enum Node {
    File {
        path: PathBuf,
        content: &'static [u8],
    },
    Dir {
        path: PathBuf,
    },
}

impl Node {
    fn file(path: impl Into<PathBuf>, content: &'static str) -> Self {
        Self::File {
            path: path.into(),
            content: content.as_bytes(),
        }
    }

    fn dir(path: impl Into<PathBuf>) -> Self {
        Self::Dir { path: path.into() }
    }

    fn path(&self) -> &Path {
        match self {
            Self::File { path, .. } => path,
            Self::Dir { path } => path,
        }
    }

    fn exists(&self) -> bool {
        self.path().exists()
    }

    fn resolved(&self, base: &Path) -> Self {
        match self {
            Self::File { path, content } => Self::File {
                path: path.clone().resolved(base),
                content,
            },
            Self::Dir { path } => Self::Dir {
                path: path.clone().resolved(base),
            },
        }
    }

    fn create(&self) -> Result<()> {
        let dir_path = match self {
            Self::File { path, .. } => path.parent(),
            Self::Dir { path } => Some(path.as_ref()),
        };
        if let Some(dir_path) = dir_path {
            fs::create_dir_all(dir_path)
                .with_context(|| format!("Could not create directory `{}`", dir_path))?;
        }

        if let Self::File { path, content } = self {
            fs::write(path, content)
                .with_context(|| format!("Could not initialize file `{}`", path))?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct DefaultProject {
    nodes: Vec<Node>,
}

impl DefaultProject {
    fn new() -> Self {
        let nodes = vec![
            // Project file:
            Node::file(PROJECT_FILE, include_str!("../default/bard.toml")),
            // Song:
            Node::file(
                format!("{}{}yippie.md", DIR_SONGS, MAIN_SEPARATOR),
                include_str!("../default/songs/yippie.md"),
            ),
            // Output dir:
            Node::dir("output"),
        ];

        Self { nodes }
    }

    pub fn resolve(&self, project_dir: &Path) -> DefaultProjectResolved {
        let nodes = self
            .nodes
            .iter()
            .map(|f| f.resolved(&project_dir))
            .collect();

        DefaultProjectResolved(Self { nodes })
    }

    fn any_exists(&self) -> Option<&Node> {
        self.nodes.iter().find(|&f| f.exists())
    }
}

pub struct DefaultProjectResolved(DefaultProject);

impl DefaultProjectResolved {
    pub fn create(self) -> Result<()> {
        let project = self.0;

        if let Some(existing) = project.any_exists() {
            bail!("File already exists: '{}'", existing.path());
        }

        for node in &project.nodes[..] {
            node.create()?;
        }

        Ok(())
    }

    pub fn files(&self) -> impl Iterator<Item = &Path> {
        self.0.nodes.iter().filter_map(|node| match node {
            Node::File { path, .. } => Some(path.as_path()),
            Node::Dir { .. } => None,
        })
    }

    pub fn dirs(&self) -> impl Iterator<Item = &Path> {
        self.0.nodes.iter().filter_map(|node| match node {
            Node::Dir { path } => Some(path.as_path()),
            Node::File { .. } => None,
        })
    }
}

lazy_static! {
    pub static ref DEFAULT_PROJECT: DefaultProject = DefaultProject::new();
}
