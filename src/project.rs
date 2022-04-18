use std::collections::HashMap;
use std::fs;
use std::iter;
use std::process::Command;
use std::str;

use camino::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use handlebars::Handlebars;
use serde::Deserialize;

use crate::book::AST_VERSION;
use crate::book::{Book, Song, SongRef};
use crate::cli;
use crate::default_project::DEFAULT_PROJECT;
use crate::error::*;
use crate::music::Notation;
use crate::render::{RHovorka, RHtml, RJson, RTex, Render};
use crate::util::{ExitStatusExt, PathBufExt};

pub use toml::Value;

mod input;
use input::{InputSet, SongsGlobs};
mod postprocess;
use postprocess::{CmdSpec, PostProcessCtx};
mod output;
pub use output::Output;

pub const PROJECT_FILE: &str = "bard.toml";
pub const DIR_SONGS: &str = "songs";
pub const DIR_TEMPLATES: &str = "templates";
pub const DIR_OUTPUT: &str = "output";

const CHORUS_LABEL_KEY: &str = "chorus_label";
const CHORUS_LABEL_DEFAULT: &str = "Ch";

fn dir_songs() -> PathBuf {
    DIR_SONGS.to_string().into()
}

fn dir_templates() -> PathBuf {
    DIR_TEMPLATES.to_string().into()
}

fn dir_output() -> PathBuf {
    DIR_OUTPUT.to_string().into()
}

pub type Metadata = HashMap<Box<str>, Value>;

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    #[serde(alias = "xhtml")]
    Html,
    Tex,
    Hovorka,
    Json,
    Auto,
}

impl Format {
    fn no_auto() -> ! {
        panic!("Output's Format should have been resolved at this point");
    }
}

impl Default for Format {
    fn default() -> Self {
        Self::Auto
    }
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

    #[serde(rename = "book")]
    pub metadata: Metadata,
}

impl Settings {
    pub fn from_file(path: &Path, project_dir: &Path) -> Result<Settings> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read project file '{}'", path))?;

        let mut settings: Settings = toml::from_str(&contents)
            .with_context(|| format!("Could not parse project file '{}'", path))?;

        settings.resolve(project_dir)?;

        if !settings.metadata.contains_key(CHORUS_LABEL_KEY) {
            settings
                .metadata
                .insert(CHORUS_LABEL_KEY.into(), CHORUS_LABEL_DEFAULT.into());
        }

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
                "Could not find {} in current or parent directories\nCurrent directory: '{}'",
                PROJECT_FILE,
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
            let project_file = parent.join(PROJECT_FILE);
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

    pub fn metadata(&self) -> &Metadata {
        &self.settings.metadata
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

    fn call_render<'a, T>(&'a self, output: &'a Output) -> Result<()>
    where
        T: Render<'a>,
    {
        let mut render: T = Render::new(self, output);

        if let Some(version) = render.load()? {
            // This Render uses versioned templates, check the compatibility
            if AST_VERSION < version {
                cli::warning(
                    format!("The version of template `{}` is {}, which is newer than what this bard uses ({}).
Maybe this project was created with a newer bard version.
This may cause errors while rendering...",
                    output.template.as_ref().unwrap(), version, AST_VERSION,
                ))
            } else if AST_VERSION.major > version.major {
                cli::warning(
                    format!("The version of template `{}` is {}, which is from an older generation than what this bard uses ({}).
This may cause errors while rendering. It may be needed to convert the template to the newer format.",
                    output.template.as_ref().unwrap(), version, AST_VERSION,
                ))
            }
        }

        render.render()
    }

    pub fn render(&self) -> Result<()> {
        fs::create_dir_all(&self.settings.dir_output)?;

        self.settings.output.iter().try_for_each(|output| {
            use self::Format::*;

            cli::status("Rendering", output.output_filename());

            match output.format {
                Html => self.call_render::<RHtml>(output),
                Tex => self.call_render::<RTex>(output),
                Hovorka => self.call_render::<RHovorka>(output),
                Json => self.call_render::<RJson>(output),
                Auto => Format::no_auto(),
            }
            .with_context(|| format!("Could not render output file '{}'", output.file))?;

            if self.post_process {
                self.post_process(output).with_context(|| {
                    format!("Could not postprocess output file '{}'", output.file)
                })?;
            }

            Ok(())
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
