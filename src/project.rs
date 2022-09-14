use std::collections::BTreeMap;
use std::fs;
use std::iter;
use std::process::Command;
use std::str;

use camino::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use handlebars::Handlebars;
use serde::Deserialize;
use serde::Serialize;

use crate::book::{self, Book, Song, SongRef};
use crate::cli;
use crate::default_project::DEFAULT_PROJECT;
use crate::error::*;
use crate::music::Notation;
use crate::render::{Render, Renderer};
use crate::util::{ExitStatusExt, PathBufExt};

pub use toml::Value;

mod input;
use input::{InputSet, SongsGlobs};
mod postprocess;
use postprocess::{CmdSpec, PostProcessCtx};
mod output;
pub use output::Output;

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

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Html,
    Tex,
    Pdf,
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

    fn post_process_one<'a>(
        &'a self,
        context: &'a PostProcessCtx<'a>,
        mut iter: impl Iterator<Item = &'a str>,
    ) -> Result<()> {
        let arg0 = match iter.next() {
            Some(arg0) => (arg0),
            None => return Ok(()), // No command does nothing
        };

        let hb = Handlebars::new();
        let arg0_r = hb
            .render_template(arg0, context)
            .with_context(|| format!("Could not substitute command: '{}'", arg0))?;

        let mut cmd = Command::new(arg0_r.clone());
        let mut cmd_src = arg0_r;

        for arg in iter {
            // Accumulate args here for error reporting:
            cmd_src.push(' ');
            cmd_src.push_str(arg);

            let arg_r = hb.render_template(arg, context).with_context(|| {
                format!("Could not substitute command arguments: '{}'", cmd_src)
            })?;

            // Replace the arg with the interpolated content after succesful render
            cmd_src.truncate(cmd_src.len() - arg.len());
            cmd_src.push_str(&arg_r);

            cmd.arg(&arg_r);
        }

        cmd.current_dir(&self.settings.dir_output);

        cli::status("Postprocess", &cmd_src);

        let status = cmd
            .status()
            .with_context(|| format!("Failed to run processing command '{}'", cmd_src))?;

        status
            .into_result()
            .with_context(|| format!("Processing command '{}' failed", cmd_src))
    }

    fn post_process(&self, output: &Output) -> Result<()> {
        let cmds = match output.post_process() {
            Some(cmds) if !cmds.is_empty() => cmds,
            _ => return Ok(()),
        };

        let context = PostProcessCtx::new(&output.file, &self.project_dir)?;

        match cmds {
            CmdSpec::Basic(cmd) => self.post_process_one(&context, cmd.split_whitespace())?,
            CmdSpec::Multiple(vec) => {
                for cmd in vec.iter() {
                    self.post_process_one(&context, cmd.split_ascii_whitespace())?;
                }
            }
            CmdSpec::Extended(vec) => {
                for cmd in vec.iter() {
                    self.post_process_one(&context, cmd.iter().map(String::as_str))?;
                }
            }
        }

        Ok(())
    }

    pub fn render(&self) -> Result<()> {
        fs::create_dir_all(&self.settings.dir_output)?;

        self.settings.output.iter().try_for_each(|output| {
            cli::status("Rendering", output.output_filename());
            let context = || format!("Could not render output file '{}'", output.file);

            let mut renderer = Renderer::new(self, output);
            let tpl_version = renderer.load().with_context(&context)?;

            let res = renderer.render().with_context(&context).and_then(|_| {
                if self.post_process {
                    self.post_process(output).with_context(|| {
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
