use std::fs;
use std::path::MAIN_SEPARATOR;

use crate::prelude::*;
use crate::util::PathBufExt as _;

/// A filesystem node, either a file (with content), or a directory.
#[derive(Debug)]
enum Node {
    File {
        path: &'static str,
        content: &'static [u8],
    },
    Dir {
        path: &'static str,
    },
}

impl Node {
    const fn dir(path: &'static str) -> Self {
        Self::Dir { path }
    }

    fn path_buf(&self) -> PathBuf {
        match self {
            Self::File { path, .. } => path,
            Self::Dir { path } => path,
        }
        .replace('/', &String::from(MAIN_SEPARATOR))
        .into() // MAIN_SEPARATOR_STR isn't stable :-|
    }

    fn resolve(&self, base: &Path) -> NodeResolved {
        let mut path = self.path_buf();
        path.resolve(base);
        match self {
            Self::File { content, .. } => NodeResolved::File { path, content },
            Self::Dir { .. } => NodeResolved::Dir { path },
        }
    }
}

enum NodeResolved {
    File {
        path: PathBuf,
        content: &'static [u8],
    },
    Dir {
        path: PathBuf,
    },
}

impl NodeResolved {
    fn path(&self) -> &Path {
        match self {
            Self::File { path, .. } => path.as_ref(),
            Self::Dir { path } => path.as_ref(),
        }
    }

    fn create(&self) -> Result<()> {
        let dir_path = match self {
            Self::File { path, .. } => path.parent(),
            Self::Dir { path } => Some(path.as_ref()),
        };
        if let Some(dir_path) = dir_path {
            fs::create_dir_all(dir_path)
                .with_context(|| format!("Could not create directory {:?}", dir_path))?;
        }

        if let Self::File { path, content } = self {
            fs::write(path, content)
                .with_context(|| format!("Could not initialize file {:?}", path))?;
        }

        Ok(())
    }
}

macro_rules! node_file {
    ($path:literal) => {
        Node::File {
            path: $path,
            content: include_bytes!(concat!("../default/", $path)),
        }
    };
}

#[derive(Debug)]
pub struct DefaultProject {
    nodes: &'static [Node],
}

impl DefaultProject {
    pub fn resolve(&self, project_dir: &Path) -> DefaultProjectResolved {
        let nodes = self.nodes.iter().map(|n| n.resolve(project_dir)).collect();
        DefaultProjectResolved { nodes }
    }
}

pub const DEFAULT_PROJECT: DefaultProject = DefaultProject {
    nodes: &[
        // Project file:
        node_file!("bard.toml"),
        // Song:
        node_file!("songs/yippie.md"),
        // Output dir:
        Node::dir("output"),
        // Fonts:
        Node::dir("output/fonts"),
        node_file!("output/fonts/BardSerif-Regular.ttf"),
        node_file!("output/fonts/BardSerif-BoldItalic.ttf"),
        node_file!("output/fonts/BardSerif-Bold.ttf"),
        node_file!("output/fonts/BardSerif-Italic.ttf"),
        node_file!("output/fonts/BardSerif-Regular.ttf"),
        node_file!("output/fonts/BardSans-BoldItalic.ttf"),
        node_file!("output/fonts/BardSans-Bold.ttf"),
        node_file!("output/fonts/BardSans-Italic.ttf"),
        node_file!("output/fonts/BardSans-Regular.ttf"),
        node_file!("output/fonts/fonts.css"),
        node_file!("output/fonts/fonts.tex"),
    ],
};

pub struct DefaultProjectResolved {
    nodes: Vec<NodeResolved>,
}

impl DefaultProjectResolved {
    pub fn create(self) -> Result<()> {
        let existing = self.nodes.iter().find(|n| n.path().exists());
        if let Some(existing) = existing {
            bail!("File already exists: {:?}", existing.path());
        }

        for node in &self.nodes[..] {
            node.create()?;
        }

        Ok(())
    }

    pub fn files(&self) -> impl Iterator<Item = &Path> {
        self.nodes.iter().filter_map(|node| match node {
            NodeResolved::File { path, .. } => Some(path.as_path()),
            NodeResolved::Dir { .. } => None,
        })
    }

    pub fn dirs(&self) -> impl Iterator<Item = &Path> {
        self.nodes.iter().filter_map(|node| match node {
            NodeResolved::Dir { path } => Some(path.as_path()),
            NodeResolved::File { .. } => None,
        })
    }
}
