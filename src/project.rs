use std::str;
use std::iter;
use std::path::{self, Path, PathBuf};
use std::ffi::OsStr;
use std::collections::HashMap;
use std::fs;
use std::process::Command;

use toml;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};

use crate::default_project::DEFAULT_PROJECT;
use crate::book::{Book, Song};
use crate::music::Notation;
use crate::render::{Render, RHtml, RHovorka, RTex, RJson};
use crate::cli;
use crate::util::*;
use crate::error::*;

pub use toml::Value;

pub const PROJECT_FILE: &'static str = "bard.toml";
pub const DIR_SONGS: &'static str = "songs";
pub const DIR_TEMPLATES: &'static str = "templates";
pub const DIR_OUTPUT: &'static str = "output";

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

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum SongsGlobs {
    One(String),
    Many(Vec<String>),
}

impl SongsGlobs {
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a str> {
        let mut pos = 0;

        iter::from_fn(move || match self {
            Self::One(s) => Some(s.as_str()),
            Self::Many(v) => v.get(pos).map(|s| {
                pos += 1;
                s.as_str()
            }),
        })
    }
}

impl Default for SongsGlobs {
    fn default() -> Self {
        Self::One("*.md".into())
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
pub struct Output {
    pub file: PathBuf,
    pub template: Option<PathBuf>,

    #[serde(default)]
    pub format: Format,

    #[serde(rename = "process")]
    pub post_process: Option<CmdSpec>,
    #[serde(rename = "process_win")]
    pub post_process_win: Option<CmdSpec>,

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

    fn resolve(&mut self, dir_templates: &Path, dir_output: &Path) -> Result<()> {
        // Check that filenames are valid UTF-8
        self.utf8_check()
            .map_err(|p| anyhow!("Filename cannot be decoded to UTF-8: {}", p))?;

        if let Some(template) = self.template.as_mut() {
            template.resolve(dir_templates);
        }
        self.file.resolve(dir_output);

        if !matches!(self.format, Format::Auto) {
            return Ok(());
        }

        let ext = self
            .file
            .extension()
            .and_then(OsStr::to_str)
            .map(str::to_lowercase);

        self.format = match ext.as_ref().map(String::as_str) {
            Some("html") | Some("xhtml") | Some("htm") | Some("xht") => Format::Html,
            Some("tex") => Format::Tex,
            Some("xml") => Format::Hovorka,
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
            Format::Html | Format::Tex | Format::Hovorka => {
                self.template.as_ref().map(PathBuf::as_path)
            }
            Format::Json => None,
            Format::Auto => Format::no_auto(),
        }
    }

    fn post_process(&self) -> Option<&CmdSpec> {
        if cfg!(windows) {
            if self.post_process_win.is_some() {
                return self.post_process_win.as_ref()
            }
        }

        self.post_process.as_ref()
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

    pub fn dpi(&self) -> f64 {
        const DEFAULT: f64 = 144.0;

        self.metadata
            .get("dpi")
            .and_then(|value| match value {
                Value::Integer(i) => Some(*i as f64),
                Value::Float(f) => Some(*f),
                _ => None,
            })
            .unwrap_or(DEFAULT)
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
            .with_context(|| format!("Failed to read project file '{}'", path.display()))?;

        let mut settings: Settings = toml::from_str(&contents)
            .with_context(|| format!("Could not parse project file '{}'", path.display()))?;

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

#[derive(Serialize, Debug)]
struct PostProcessCtx<'a> {
    file: &'a str,
    file_name: &'a str,
    file_stem: &'a str,
    project_dir: &'a str,
}

impl<'a> PostProcessCtx<'a> {
    fn new(file: &'a Path, project_dir: &'a Path) -> Self {
        // NOTE: Filenames should be known to be UTF-8-valid and canonicalized at this point
        let file_name = file.file_name().unwrap();
        let file_stem = file.file_stem().unwrap_or(file_name).to_str().unwrap();

        Self {
            file: file.to_str().unwrap(),
            file_name: file_name.to_str().unwrap(),
            file_stem,
            project_dir: project_dir.to_str().unwrap(),
        }
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
        let (project_file, project_dir) = Self::find_in_parents(cwd).ok_or(anyhow!(
            "Could not find {} in current or parent directories\nCurrent directory: '{}'",
            PROJECT_FILE,
            cwd.display()
        ))?;

        cli::status("Loading", &format!("project at {}", project_dir.display()));

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

        project.input_paths = project.collect_input_paths()?;
        project.book.load_files(&project.input_paths)?;

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
        // glob doesn't support setting a base for relative paths,
        // so we have to cd into the songs dir...
        let _cwd = CwdGuard::new(&self.settings.dir_songs)?;

        self.settings
            .songs
            .iter()
            .map(|g| (g, glob::glob(g)))
            .try_fold(vec![], |mut paths, (glob_src, glob)| {
                let glob =
                    glob.with_context(|| format!("Invalid input files pattern: '{}'", glob_src))?;

                let orig_idx = paths.len();
                for globres in glob {
                    let path = globres
                        .context("Could not locate input files")?
                        .resolved(&self.settings.dir_songs);

                    paths.push(path);
                }

                // Sort the entries collected for this glob.
                // This way, paths from one glob pattern are sorted alphabetically,
                // but order of globs as given in the input array is preserved.
                paths[orig_idx..].sort();

                Ok(paths)
            })
    }

    pub fn metadata(&self) -> &Metadata {
        &self.settings.metadata
    }

    pub fn songs(&self) -> &[Song] {
        &self.book.songs
    }

    fn post_process_one<'a>(
        &'a self, context: &'a PostProcessCtx<'a>, mut iter: impl Iterator<Item = &'a str>,
    ) -> Result<()> {
        let arg0 = match iter.next() {
            Some(arg0) => (arg0),
            None => return Ok(()), // No command does nothing
        };

        let hb = Handlebars::new();
        let arg0_r = hb.render_template(arg0, context).with_context(|| {
            format!("Could not substitute command: '{}'", arg0)
        })?;

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

        let context = PostProcessCtx::new(&output.file, &self.project_dir);

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
        fs::create_dir_all(&self.settings.dir_output)?;

        self.settings.output.iter().try_for_each(|output| {
            use self::Format::*;

            cli::status("Rendering", output.output_filename());

            match output.format {
                Html => RHtml::render(self, &output),
                Tex => RTex::render(self, &output),
                Hovorka => RHovorka::render(self, &output),
                Json => RJson::render(self, &output),
                Auto => Format::no_auto(),
            }
            .with_context(|| format!("Could render output file '{}'", output.file.display()))?;

            if self.post_process {
                self.post_process(&output)?;
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
