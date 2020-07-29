use std::str;
use std::iter;
use std::path::{self, Path, PathBuf};
use std::ffi::OsStr;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::process::Command;

use toml;
use serde::{Deserialize, Deserializer};
use tera::{self, Tera, Context};

use crate::default_project::DEFAULT_PROJECT;
use crate::book::{Book, Song};
use crate::music::Notation;
use crate::parser::ParsingDebug;
use crate::render::{Render, RHtml, RTex, RJson, RTxt};
use crate::cli;
use crate::util::ExitStatusExt as _;
use crate::error::*;

pub use toml::Value;

pub const PROJECT_FILE: &'static str = "bard.toml";


pub type Metadata = HashMap<Box<str>, Value>;

fn deserialize_inputs<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize, Debug)]
    #[serde(untagged)]
    enum DeInput {
        One(String),
        Many(Vec<String>),
    }

    let input = DeInput::deserialize(deserializer)?;

    Ok(match input {
        DeInput::One(glob) => vec![glob],
        DeInput::Many(vec) => vec,
    })
}

trait PathBufExt {
    fn resolve(&mut self, project_dir: &Path);
    fn resolved(self, project_dir: &Path) -> Self;
    fn utf8_check(&self) -> Result<(), path::Display>;
}

impl PathBufExt for PathBuf {
    fn resolve(&mut self, project_dir: &Path) {
        if self.is_relative() {
            *self = project_dir.join(&self);
        }
    }

    fn resolved(mut self, project_dir: &Path) -> Self {
        self.resolve(project_dir);
        self
    }

    fn utf8_check(&self) -> Result<(), path::Display> {
        self.to_str().map(|_| ()).ok_or(self.display())
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum CmdSpec {
    Basic(String),
    Extended(Vec<Vec<String>>),
}

impl CmdSpec {
    fn is_empty(&self) -> bool {
        match self {
            Self::Basic(s) => s.is_empty(),
            Self::Extended(v) => v.is_empty(),
        }
    }
}

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    #[serde(alias = "xhtml")]
    #[serde(alias = "xml")]
    Html,
    Latex,
    Txt,
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
pub struct Output {
    pub file: PathBuf,
    pub template: Option<PathBuf>,

    #[serde(default)]
    pub format: Format,

    #[serde(rename = "process")]
    pub post_process: Option<CmdSpec>,

    #[serde(flatten)]
    pub metadata: Metadata,
}

impl Output {
    fn utf8_check(&self) -> Result<(), path::Display> {
        if let Some(template) = self.template.as_ref() {
            template.utf8_check()?;
        }
        self.file.utf8_check()
    }

    fn resolve(&mut self, project_dir: &Path) -> Result<()> {
        // Check that filenames are valid UTF-8
        self.utf8_check()
            .map_err(|p| anyhow!("Filename cannot be decoded to UTF-8: {}", p))?;

        if let Some(template) = self.template.as_mut() {
            template.resolve(project_dir);
        }
        self.file.resolve(project_dir);

        if !matches!(self.format, Format::Auto) {
            return Ok(());
        }

        let ext = self
            .file
            .extension()
            .and_then(OsStr::to_str)
            .map(str::to_lowercase);

        self.format = match ext.as_ref().map(String::as_str) {
            Some("html") | Some("xhtml") | Some("htm") | Some("xml") | Some("xht") => Format::Html,
            Some("tex") => Format::Latex,
            Some("txt") => Format::Txt,
            Some("json") => Format::Json,
            _ => bail!(
                "Unknown or unsupported format of output file: {}\nHint: Specify format with  \
                 'format = ...'",
                self.file.display()
            ),
        };

        Ok(())
    }

    fn output_filename(&self) -> &str {
        self.file
            .file_name()
            .map(|name| {
                name.to_str()
                    .expect("OutputSpec: template path must be valid utf-8")
                    .into()
            })
            .expect("OutputSpec: Invalid filename")
    }

    fn template_path(&self) -> Option<&Path> {
        match self.format {
            Format::Html | Format::Latex => self.template.as_ref().map(PathBuf::as_path),
            Format::Txt | Format::Json => None,
            Format::Auto => Format::no_auto(),
        }
    }

    pub fn template_filename(&self) -> String {
        self.template
            .as_ref()
            .map(|p| {
                p.to_str()
                    .expect("OutputSpec: template path must be valid utf-8")
                    .into()
            })
            .unwrap_or(String::from("<builtin>"))
    }
}

#[derive(Deserialize, Debug)]
pub struct Settings {
    #[serde(deserialize_with = "deserialize_inputs")]
    pub input: Vec<String>,
    pub output: Vec<Output>,

    #[serde(default)]
    pub notation: Notation,
    #[serde(default = "Settings::default_chorus_label")]
    pub chorus_label: String,

    #[serde(default)]
    pub debug: bool,

    #[serde(rename = "book")]
    pub metadata: Metadata,
}

impl Settings {
    pub fn from_file(path: &Path, project_dir: &Path) -> Result<Settings> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read project file '{}'", path.display()))?;

        let mut settings: Settings = toml::from_str(&contents)
            .with_context(|| format!("Could not parse project file '{}'", path.display()))?;

        settings.resolve(project_dir)?;
        Ok(settings)
    }

    fn default_chorus_label() -> String {
        String::from("Ch.")
    }

    fn resolve(&mut self, project_dir: &Path) -> Result<()> {
        for output in self.output.iter_mut() {
            output.resolve(project_dir)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Project {
    project_file: PathBuf,
    pub project_dir: PathBuf,
    pub settings: Settings,
    input_paths: Vec<PathBuf>,
    pub book: Book,
}

impl Project {
    pub fn new<P: AsRef<Path>>(cwd: P) -> Result<Project> {
        let cwd = cwd.as_ref();
        let (project_file, project_dir) = Self::find_in_parents(cwd).ok_or(anyhow!(
            "Could not find 'bard.toml' in current or parent directories\nCurrent directory: '{}'",
            cwd.display()
        ))?;

        cli::status("Loading", &format!("project at {}", project_dir.display()));

        // cd into the project dir, this ensures globbing and
        // template and output file relative paths work
        env::set_current_dir(&project_dir).context("Could not read project directory")?;

        let settings = Settings::from_file(&project_file, &project_dir)?;
        let book = Book::new(settings.notation, &settings.chorus_label, settings.debug);

        let mut project = Project {
            project_file,
            project_dir,
            settings,
            input_paths: vec![],
            book,
        };

        project.collect_input_paths()?;
        project.book.load_files(&project.input_paths)?;

        Ok(project)
    }

    fn find_in_parents(start_dir: &Path) -> Option<(PathBuf, PathBuf)> {
        assert!(start_dir.is_dir());

        let mut parent = start_dir;
        loop {
            let bard_toml = parent.join(PROJECT_FILE);
            if bard_toml.exists() {
                return Some((bard_toml, parent.into()));
            }

            parent = parent.parent()?;
        }
    }

    pub fn init<P: AsRef<Path>>(project_dir: P) -> Result<()> {
        let project_dir = project_dir.as_ref();

        if let Some(path) = DEFAULT_PROJECT
            .iter()
            .map(|entry| entry.path(project_dir))
            .find(|path| path.exists())
        {
            bail!("File already exists: '{}'", path.display());
        }

        for entry in DEFAULT_PROJECT {
            entry.create(project_dir)?;
        }

        Ok(())
    }

    fn collect_input_paths(&mut self) -> Result<()> {
        self.input_paths = self
            .settings
            .input
            .iter()
            .map(|g| (g, glob::glob(g)))
            .try_fold(vec![], |mut paths, (glob_src, glob)| {
                let glob =
                    glob.with_context(|| format!("Invalid input files pattern: '{}'", glob_src))?;

                let mut matched = false;
                let orig_idx = paths.len();
                for globres in glob {
                    matched = true;

                    let path = globres
                        .context("Could not locate input files")?
                        .resolved(&self.project_dir);

                    paths.push(path);
                }

                if !matched {
                    // Pattern matched no files
                    bail!("No file(s) found for input pattern: '{}'", glob_src);
                } else {
                    // Sort the entries collected for this glob.
                    // This way, paths from one glob pattern are sorted alphabetically,
                    // but order of globs as given in the input array is preserved.
                    paths[orig_idx..].sort();

                    Ok(paths)
                }
            })?;

        Ok(())
    }

    pub fn metadata(&self) -> &Metadata {
        &self.settings.metadata
    }

    pub fn songs(&self) -> &[Song] {
        &self.book.songs
    }

    pub fn parsing_debug(&self) -> Option<&ParsingDebug> {
        if self.settings.debug {
            self.book.parsing_debug.as_ref()
        } else {
            None
        }
    }

    fn post_process_one<'a>(
        &'a self, context: &Context, mut iter: impl Iterator<Item = &'a str>,
    ) -> Result<()> {
        let arg0 = match iter.next() {
            Some(arg0) => (arg0),
            None => return Ok(()), // No command does nothing
        };

        let mut cmd = Command::new(arg0);
        let mut cmd_src = arg0.to_string();

        for arg in iter {
            // Accumulate args here for error reporting:
            cmd_src.push(' ');
            cmd_src.push_str(arg);

            let arg_interp = Tera::one_off(arg, context, false).with_context(|| {
                format!("Could not substitute command arguments: '{}'", cmd_src)
            })?;

            // Replace the arg with the interpolated content after succesful Tera
            // interpolation: (the space stays)
            cmd_src.truncate(cmd_src.len() - arg.len());
            cmd_src.push_str(&arg_interp);

            cmd.arg(&arg_interp);
        }

        cmd.current_dir(&self.project_dir);

        let status = cmd
            .status()
            .with_context(|| format!("Failed to run processing command '{}'", cmd_src))?;

        status
            .into_result()
            .with_context(|| format!("Processing command '{}' failed", cmd_src))
    }

    fn post_process(&self, output: &Output) -> Result<()> {
        let cmds = match output.post_process.as_ref() {
            Some(cmds) if !cmds.is_empty() => cmds,
            _ => return Ok(()),
        };

        // NOTE: Filenames should be known to be UTF-8-valid and canonicalized at this
        // point
        let mut context = Context::new();
        context.insert("file", output.file.to_str().unwrap());
        let filename = output.file.file_name().unwrap();
        context.insert("file_name", filename.to_str().unwrap());
        let stem = output
            .file
            .file_stem()
            .unwrap_or(filename)
            .to_str()
            .unwrap();
        context.insert("file_stem", stem);
        context.insert("project_dir", self.project_dir.to_str().unwrap());
        let context = context;

        match cmds {
            CmdSpec::Basic(s) => self.post_process_one(&context, s.split_whitespace())?,
            CmdSpec::Extended(vec) => {
                for cmd in vec.iter() {
                    self.post_process_one(&context, cmd.iter().map(String::as_str))?
                }
            }
        }

        Ok(())
    }

    pub fn render(&self) -> Result<()> {
        self.settings.output.iter().try_for_each(|output| {
            use self::Format::*;

            cli::status("Rendering", output.output_filename());

            match output.format {
                Html => RHtml::render(self, &output),
                Latex => RTex::render(self, &output),
                Json => RJson::render(self, &output),
                Txt => RTxt::render(self, &output),
                Auto => Format::no_auto(),
            }
            .with_context(|| format!("Could render output file '{}'", output.file.display()))?;

            self.post_process(&output)
        })
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
}
