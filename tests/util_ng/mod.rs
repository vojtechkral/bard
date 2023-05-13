//! New test project builder that supports defining projects from code.

use std::{
    fs, io, mem,
    ops::{Bound, RangeBounds},
    process::Command,
    thread::{self, JoinHandle},
};

use base64::{engine::general_purpose::STANDARD as BASE_64, Engine as _};
use regex::{Match, Regex};
use toml::Value as Toml;

use bard::{
    app::App,
    parser::DiagKind,
    prelude::*,
    project::Project,
    render::template::DefaultTemaplate,
    util::ExitStatusExt as _,
    watch::{Watch, WatchControl},
};

pub use indoc::{formatdoc, indoc};
pub use toml::toml;

pub struct TestProject {
    path: PathBuf,
    postprocess: bool,
    outputs: Vec<Toml>,
    modify_settings: Option<Box<dyn FnOnce(&mut toml::Table)>>,
    songs: Vec<(PathBuf, String)>,
    templates: Vec<Template>,
    scripts: Vec<Script>,
    assets: Vec<(PathBuf, Box<[u8]>)>,
}

impl TestProject {
    pub fn new(name: &str) -> Self {
        let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("test-projects")
            .join(name);

        Self {
            path,
            postprocess: false,
            outputs: vec![],
            modify_settings: None,
            songs: vec![],
            templates: vec![],
            scripts: vec![],
            assets: vec![],
        }
    }

    pub fn postprocess(mut self, postprocess: bool) -> Self {
        self.postprocess = postprocess;
        self
    }

    pub fn output(self, file: impl Into<String>) -> Self {
        let file = file.into();
        self.output_toml(toml! { file = file })
    }

    pub fn output_toml(mut self, output: impl Into<Toml>) -> Self {
        self.outputs.push(output.into());
        self
    }

    pub fn settings(mut self, f: impl FnOnce(&mut toml::Table) + 'static) -> Self {
        self.modify_settings = Some(Box::new(f));
        self
    }

    pub fn song(mut self, path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        let path = path.into();
        if !path.is_relative() {
            panic!("Song path must be relative: {:?}", path);
        }

        self.songs.push((path, content.into()));
        self
    }

    pub fn template(
        mut self,
        output: impl Into<String>,
        filename: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        self.templates.push(Template {
            output: output.into(),
            filename: filename.into(),
            content: content.into(),
        });
        self
    }

    /// Set output with filename `output` to use a template which is made up of
    /// a custom `prefix` followed by a `default` template content.
    pub fn template_prefix_default(
        self,
        output: impl Into<String>,
        filename: impl Into<String>,
        prefix: impl AsRef<str>,
        default: &DefaultTemaplate,
    ) -> Self {
        self.template(
            output,
            filename,
            format!("{}\n{}", prefix.as_ref(), default.content),
        )
    }

    pub fn script(
        mut self,
        output: impl Into<String>,
        name: impl Into<String>,
        content_sh: impl Into<String>,
        content_bat: impl Into<String>,
    ) -> Self {
        self.scripts.push(Script {
            output: output.into(),
            name: name.into(),
            content_sh: content_sh.into(),
            content_bat: content_bat.into(),
        });
        self
    }

    /// Add an asset file in the `output` directory, the `content` should be base64-formatted.
    pub fn binary_asset(mut self, path: impl Into<PathBuf>, content: impl AsRef<str>) -> Self {
        let path = path.into();
        if !path.is_relative() {
            panic!("Asset path must be relative: {:?}", path);
        }

        let bytes = content.as_ref().decode_base64();
        self.assets.push((path, bytes));

        self
    }

    pub fn build(mut self) -> Result<TestBuild> {
        // Create project directory
        if self.path.exists() {
            fs::remove_dir_all(&self.path).with_context(|| {
                format!("Couldn't remove previous test run data: {:?}", self.path)
            })?;
        }
        fs::create_dir_all(&self.path)
            .with_context(|| format!("Couldn't create directory: {:?}", self.path))?;

        // Instantiate App
        let bard_exe = option_env!("CARGO_BIN_EXE_bard")
            .expect("$CARGO_BIN_EXE_bard")
            .into();
        let app = App::with_test_mode(self.postprocess, bard_exe);

        // Init default project
        bard::bard_init_at(&app, &self.path)
            .with_context(|| format!("Failed to initialize project at: {:?}", self.path))?;

        let bard_toml_path = self.path.join("bard.toml");
        let mut bard_toml: toml::Table = fs::read_to_string(&bard_toml_path)
            .map_err(Error::from)
            .and_then(|toml| toml.parse().map_err(Error::from))
            .with_context(|| format!("Couldn't read bard.toml at {:?}", bard_toml_path))?;

        // Write predefined songs and update them in bard.toml
        // (default songs are removed)
        if !self.songs.is_empty() {
            let songs_dir = self.path.join("songs");
            fs::remove_dir_all(&songs_dir)
                .with_context(|| format!("Failed to remove default songs at: {:?}", songs_dir))?;
            fs::create_dir_all(&songs_dir)
                .with_context(|| format!("Couldn't create songs directory: {:?}", songs_dir))?;
            for (path, content) in self.songs.iter() {
                let path = songs_dir.join(path);
                fs::write(&path, content.as_bytes())
                    .with_context(|| format!("Couldn't write song file: {:?}", path))?;
            }
            let paths = self
                .songs
                .iter()
                .map(|(path, _)| Toml::String(path.to_string_lossy().into()))
                .collect();
            bard_toml.insert("songs".to_string(), Toml::Array(paths));
        }

        // Remove default outputs and apply configured ones
        bard_toml.set("output", mem::take(&mut self.outputs));

        // Write templates
        let tpl_dir = self.path.join("templates");
        if !self.templates.is_empty() {
            fs::create_dir_all(&tpl_dir)
                .with_context(|| format!("Couldn't create templates directory: {:?}", tpl_dir))?;
            for tpl in self.templates.iter() {
                let path = tpl_dir.join(&tpl.filename);
                fs::write(&path, tpl.content.as_bytes())
                    .with_context(|| format!("Couldn't write template file: {:?}", path))?;
            }
            for tpl in self.templates.iter() {
                bard_toml
                    .output_mut(&tpl.output)
                    .set("template", tpl.filename.as_str());
            }
        }

        // Write scripts
        let out_dir = self.path.join("output");
        if !self.scripts.is_empty() {
            fs::create_dir_all(&out_dir)
                .with_context(|| format!("Couldn't create output directory: {:?}", tpl_dir))?;
            for script in self.scripts.iter() {
                let path_sh = out_dir.join(&format!("{}.sh", script.name));
                let path_bat = out_dir.join(&format!("{}.bat", script.name));
                fs::write(&path_sh, script.content_sh.as_bytes())
                    .and_then(|_| path_sh.chmod(0o755))
                    .with_context(|| format!("Couldn't write script file: {:?}", path_sh))?;
                fs::write(&path_bat, script.content_bat.as_bytes())
                    .with_context(|| format!("Couldn't write script file: {:?}", path_bat))?;
            }
            for script in self.scripts.iter() {
                bard_toml
                    .output_mut(&script.output)
                    .set("script", script.name.as_str());
            }
        }

        // Write assets
        if !self.assets.is_empty() {
            fs::create_dir_all(&out_dir)
                .with_context(|| format!("Couldn't create output directory: {:?}", tpl_dir))?;
            for (path, content) in self.assets.iter() {
                let path = out_dir.join(path);
                fs::write(&path, content)
                    .with_context(|| format!("Couldn't write asset file: {:?}", path))?;
            }
        }

        // Modify project settings
        // This step goes last so that tests are able to modify settings applied by previous steps.
        if let Some(modify_settings) = self.modify_settings.take() {
            modify_settings(&mut bard_toml);
        }

        // Write back bard.toml
        toml::to_string_pretty(&bard_toml)
            .map_err(Error::from)
            .and_then(|toml| fs::write(&bard_toml_path, toml.as_bytes()).map_err(Error::from))
            .with_context(|| format!("Couldn't write bard.toml at {:?}", bard_toml_path))?;

        // Build project
        let result = bard::bard_make_at(&app, &self.path)
            .with_context(|| format!("Failed to build project at: {:?}", self.path));

        Ok(TestBuild { result, app })
    }
}

#[derive(Debug)]
pub struct TestBuild {
    result: Result<Project>,
    app: App,
}

impl TestBuild {
    #[track_caller]
    pub fn unwrap(&self) -> &Project {
        self.result.as_ref().unwrap()
    }

    #[track_caller]
    pub fn unwrap_err(&self) -> &Error {
        self.result.as_ref().unwrap_err()
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    #[track_caller]
    pub fn assert_parser_diag(&self, kind: DiagKind) {
        self.app
            .parser_diags()
            .lock()
            .iter()
            .find(|diag| diag.kind == kind)
            .unwrap();
    }

    pub fn dir_songs(&self) -> &Path {
        self.unwrap().settings.dir_songs()
    }

    pub fn dir_output(&self) -> &Path {
        self.unwrap().settings.dir_output()
    }

    pub fn output_path(&self, suffix: &str) -> Result<PathBuf> {
        fs::read_dir(self.dir_output())?
            .map(|entry| entry.unwrap().path())
            .find(|p| p.file_ends_with(suffix))
            .ok_or_else(|| anyhow!("No file in output dir with suffix `{}`", suffix))
    }

    pub fn try_read_output(&self, suffix: &str) -> Result<String> {
        // Reading output dir rather than iterating project outputs,
        // to support .tex files, files generated by script etc.
        let file = self.output_path(suffix)?;
        fs::read_to_string(&file).map_err(Error::from)
    }

    pub fn read_output(&self, suffix: &str) -> String {
        self.try_read_output(suffix).unwrap()
    }

    /// Convert a PDF to text using the Poppler `pdftotext` tool.
    ///
    /// `pages` is a 1-indexed range, ie. `1..3` means pages 1 and 2 (and is the same as `..3`).
    pub fn pdf_to_text(
        &self,
        output_suffix: &str,
        pages: impl RangeBounds<usize>,
    ) -> Result<String> {
        let mut cmd = Command::new("pdftotext");
        cmd.arg("-layout");
        cmd.arg("-enc").arg("UTF-8");

        if let Some(f) = match pages.start_bound() {
            Bound::Included(&f) => Some(f),
            Bound::Excluded(&f) => Some(f + 1),
            Bound::Unbounded => None,
        } {
            cmd.arg("-f".to_string()).arg(format!("{}", f));
        };

        if let Some(l) = match pages.end_bound() {
            Bound::Included(&i) => Some(i),
            Bound::Excluded(&i) => Some(i - 1),
            Bound::Unbounded => None,
        } {
            cmd.arg("-l".to_string()).arg(format!("{}", l));
        };

        let output = self.output_path(output_suffix)?;
        cmd.arg(output).arg("-");

        let output = cmd.output()?;
        output.status.into_result()?;
        let stdout = String::from_utf8_lossy(&output.stdout).into();
        Ok(stdout)
    }

    /// Start bard watch in another thread.
    pub fn watch(&self) -> (JoinHandle<()>, WatchControl) {
        let dir_output = self.dir_output().to_owned();
        let app = self.app.clone();
        let (watch, control) = Watch::new(true).unwrap();

        let watch_thread = thread::spawn(move || {
            bard::bard_watch_at(&app, &dir_output, watch).unwrap();
        });

        (watch_thread, control)
    }
}

struct Template {
    output: String,
    filename: String,
    content: String,
}

struct Script {
    output: String,
    name: String,
    content_sh: String,
    content_bat: String,
}

pub trait TomlTableExt {
    fn set(&mut self, key: impl Into<String>, value: impl Into<Toml>);
    fn output(&self, suffix: &str) -> &toml::Table;
    fn output_mut(&mut self, suffix: &str) -> &mut toml::Table;
}

impl TomlTableExt for toml::Table {
    fn set(&mut self, key: impl Into<String>, value: impl Into<Toml>) {
        self.insert(key.into(), value.into());
    }

    fn output(&self, suffix: &str) -> &toml::Table {
        self.get("output")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .find(|o| {
                o.get("file")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .strip_suffix(suffix)
                    .is_some()
            })
            .unwrap()
            .as_table()
            .unwrap()
    }

    fn output_mut(&mut self, suffix: &str) -> &mut toml::Table {
        self.get_mut("output")
            .unwrap()
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|o| {
                o.get("file")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .strip_suffix(suffix)
                    .is_some()
            })
            .unwrap()
            .as_table_mut()
            .unwrap()
    }
}

pub trait StrExt {
    fn find_re<'s>(&'s self, re: &str) -> Option<Match<'s>>;
    fn decode_base64(&self) -> Box<[u8]>;
}

impl StrExt for str {
    fn find_re<'s>(&'s self, re: &str) -> Option<Match<'s>> {
        let re = Regex::new(re).unwrap();
        re.find(self)
    }

    fn decode_base64(&self) -> Box<[u8]> {
        BASE_64.decode(self).unwrap().into()
    }
}

pub trait StringExt {
    fn remove_newlines(self) -> Self;
}

impl StringExt for String {
    fn remove_newlines(mut self) -> Self {
        self.retain(|c| c != '\n' && c != '\r');
        self
    }
}

trait PathExt {
    fn chmod(&self, mode: u32) -> io::Result<()>;
}

#[cfg(unix)]
impl PathExt for Path {
    fn chmod(&self, mode: u32) -> io::Result<()> {
        use std::fs::Permissions;
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(self, Permissions::from_mode(mode))
    }
}

#[cfg(not(unix))]
impl PathExt for Path {
    fn chmod(&self, _mode: u32) -> io::Result<()> {
        Ok(())
    }
}
