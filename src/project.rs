use std::collections::BTreeMap;
use std::fs;
use std::iter;
use std::str;

use serde::Deserialize;
use serde::Serialize;

use crate::book::{self, Book, Song, SongRef};
use crate::cli;
use crate::default_project::DEFAULT_PROJECT;
use crate::music::Notation;
use crate::prelude::*;
use crate::render::tex_tools::TexTools;
use crate::render::Renderer;
use crate::util::PathBufExt;

pub use toml::Value;

mod input;
use input::{InputSet, SongsGlobs};
mod output;
mod postprocess;
pub use output::Output;

use self::postprocess::PostProcessor;

fn dir_songs() -> PathBuf {
    "songs".into()
}

fn dir_templates() -> PathBuf {
    "templates".into()
}

fn dir_output() -> PathBuf {
    "output".into()
}

fn default_chorus_label() -> String {
    "Ch".into()
}

pub type Metadata = BTreeMap<Box<str>, Value>;

#[derive(Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Pdf,
    Html,
    Tex,
    Hovorka,
    Json,
    Xml,
    Auto,
}

impl Format {
    pub fn no_auto() -> ! {
        panic!("Output's Format should have been resolved at this point");
    }
}

impl Default for Format {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BookSection {
    #[serde(default = "default_chorus_label")]
    pub chorus_label: String,

    #[serde(flatten)]
    pub metadata: Metadata,
}

#[derive(Deserialize, Debug)]
pub struct Settings {
    #[serde(default = "dir_songs")]
    dir_songs: PathBuf,
    #[serde(default = "dir_templates")]
    dir_templates: PathBuf,
    #[serde(default = "dir_output")]
    dir_output: PathBuf,

    songs: SongsGlobs,
    pub output: Vec<Output>,

    #[serde(default)]
    pub notation: Notation,

    pub book: BookSection,
}

impl Settings {
    pub fn from_file(path: &Path, project_dir: &Path) -> Result<Settings> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read project file '{}'", path))?;

        let mut settings: Settings = toml::from_str(&contents)
            .with_context(|| format!("Could not parse project file '{}'", path))?;

        settings.resolve(project_dir)?;
        Ok(settings)
    }

    pub fn dir_output(&self) -> &Path {
        self.dir_output.as_ref()
    }

    fn resolve(&mut self, project_dir: &Path) -> Result<()> {
        self.dir_songs.resolve(project_dir);
        self.dir_templates.resolve(project_dir);
        self.dir_output.resolve(project_dir);

        for output in self.output.iter_mut() {
            output.resolve(&self.dir_templates, &self.dir_output)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Project {
    pub project_dir: PathBuf,
    pub settings: Settings,
    pub book: Book,

    project_file: PathBuf,
    input_paths: Vec<PathBuf>,
    post_process: bool,
}

impl Project {
    pub fn new<P: AsRef<Path>>(cwd: P) -> Result<Project> {
        let cwd = cwd.as_ref();
        let (project_file, project_dir) = Self::find_in_parents(cwd).ok_or_else(|| {
            anyhow!(
                "Could not find bard.toml file in current or parent directories\nCurrent directory: '{}'",
                cwd
            )
        })?;

        cli::status("Loading", &format!("project at {}", project_dir));

        let settings = Settings::from_file(&project_file, &project_dir)?;
        let book = Book::new(&settings);

        let mut project = Project {
            project_file,
            project_dir,
            settings,
            input_paths: vec![],
            book,
            post_process: true,
        };

        project.input_paths = project
            .collect_input_paths()
            .context("Failed to load input files")?;
        project.book.load_files(&project.input_paths)?;
        project.book.postprocess();

        Ok(project)
    }

    fn find_in_parents(start_dir: &Path) -> Option<(PathBuf, PathBuf)> {
        assert!(start_dir.is_dir());

        let mut parent = start_dir;
        loop {
            let project_file = parent.join("bard.toml");
            if project_file.exists() {
                return Some((project_file, parent.into()));
            }

            parent = parent.parent()?;
        }
    }

    pub fn init<P: AsRef<Path>>(project_dir: P) -> Result<()> {
        DEFAULT_PROJECT.resolve(project_dir.as_ref()).create()
    }

    fn collect_input_paths(&mut self) -> Result<Vec<PathBuf>> {
        let input_set = InputSet::new(&self.settings.dir_songs)?;

        self.settings
            .songs
            .iter()
            .try_fold(input_set, InputSet::apply_glob)?
            .finalize()
    }

    pub fn book_section(&self) -> &BookSection {
        &self.settings.book
    }

    pub fn songs(&self) -> &[Song] {
        &self.book.songs
    }

    pub fn songs_sorted(&self) -> &[SongRef] {
        &self.book.songs_sorted
    }

    pub fn render(&self) -> Result<()> {
        fs::create_dir_all(&self.settings.dir_output)?;
        let postprocessor = PostProcessor::new(&self.project_dir, self.settings.dir_output());

        if self.settings.output.iter().any(|o| o.format == Format::Pdf) {
            // Initialize Tex tools ahead of actual rendering so that
            // errors are reported early...
            TexTools::initialize().context("Could not initialize TeX tools.")?;
        }

        self.settings.output.iter().try_for_each(|output| {
            cli::status("Rendering", output.output_filename());
            let context = || format!("Could not render output file '{}'", output.file);

            let renderer = Renderer::new(self, output).with_context(context)?;
            let tpl_version = renderer.version();

            let res = renderer.render().with_context(context).and_then(|_| {
                if self.post_process {
                    postprocessor.run(output).with_context(|| {
                        format!("Could not postprocess output file '{}'", output.file)
                    })
                } else {
                    Ok(())
                }
            });

            // Perform version check of the template (if the Render supports it and there is a template file).
            // This is done after rendering and preprocessing so that the CLI messages are at the bottom of the log.
            // Otherwise they tend to be far behind eg. TeX output etc.
            if let Some((tpl_version, tpl_path)) = tpl_version.zip(output.template.as_ref()) {
                book::version::compat_check(tpl_path, &tpl_version);
            }

            res
        })
    }

    pub fn input_paths(&self) -> &Vec<PathBuf> {
        &self.input_paths
    }

    pub fn output_paths(&self) -> impl Iterator<Item = &Path> {
        self.settings.output.iter().map(|o| o.file.as_path())
    }

    pub fn watch_paths(&self) -> impl Iterator<Item = &Path> {
        let in_iter = self.input_paths.iter().map(PathBuf::as_path);

        let out_iter = self
            .settings
            .output
            .iter()
            .filter_map(Output::template_path);

        iter::once(self.project_file.as_path())
            .chain(in_iter)
            .chain(out_iter)
    }

    pub fn enable_postprocess(&mut self, enable: bool) {
        self.post_process = enable;
    }
}
