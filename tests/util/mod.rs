use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::ops;
use std::process::Command;
use std::process::Stdio;

use bard::app::App;
use bard::util::Apply;
use bard::util::ExitStatusExt;
use fs_extra::dir::{self, CopyOptions};

use bard::prelude::*;
use bard::project::Project;

/// Project source root (where `Cargo.toml` is)
pub const ROOT: ProjectPath = ProjectPath { path: &[] };

/// `$ROOT/tests/test-projects`
pub const TEST_PROJECTS: ProjectPath = ProjectPath {
    path: &["tests", "test-projects"],
};

#[derive(Clone, Copy, Debug)]
pub struct ProjectPath {
    path: &'static [&'static str],
}

impl<'rhs> ops::Div<&'rhs str> for ProjectPath {
    type Output = PathBuf;

    fn div(self, rhs: &'rhs str) -> Self::Output {
        let mut res = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for c in self.path.iter() {
            res.push(c);
        }
        res.push(rhs);
        res
    }
}

#[track_caller]
pub fn assert_file_contains(path: impl AsRef<Path>, what: &str) {
    let path = path.as_ref();
    let content = fs::read_to_string(path).unwrap();
    let hit = content.find(what);
    assert!(
        hit.is_some(),
        "String '{}' not found in file: {:?}\nFile contents:\n{}",
        what,
        path,
        content
    );
}

#[track_caller]
pub fn assert_first_line_contains(path: impl AsRef<Path>, what: &str) {
    let path = path.as_ref();
    let file = BufReader::new(File::open(path).unwrap());
    let line = file.lines().next().unwrap().unwrap();
    let hit = line.find(what);
    assert!(
        hit.is_some(),
        "String '{}' not found in first line of file: {:?}\nFirst line: {}",
        what,
        path,
        line
    );
}

pub fn tmp_dir() -> PathBuf {
    // Cargo support for tmpdir merged yay https://github.com/rust-lang/cargo/pull/9375
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
}

#[derive(Debug)]
pub struct Builder {
    pub project: Project,
    pub dir: PathBuf,
    pub app: App,
}

impl Builder {
    pub fn work_dir(name: &str, rm: bool) -> Result<PathBuf> {
        let path = tmp_dir().join(name);

        if rm && path.exists() {
            fs::remove_dir_all(&path)
                .with_context(|| format!("Couldn't remove previous test run data: {:?}", path))?;
        }

        Ok(path)
    }

    fn dir_copy(src: impl AsRef<Path>, dest: impl AsRef<Path>) -> Result<()> {
        let src = src.as_ref();
        let dest = dest.as_ref();

        fs::create_dir_all(dest)
            .with_context(|| format!("Couldn't create directory: {:?}", dest))?;

        let mut opts = CopyOptions::new();
        opts.content_only = true;
        dir::copy(src, dest, &opts)
            .with_context(|| format!("Couldn't copy directory {:?} to {:?}", src, dest))?;
        Ok(())
    }

    pub fn prepare(src_path: impl AsRef<Path>, name: &str) -> Result<PathBuf> {
        let src_path = src_path.as_ref();
        let work_dir = Self::work_dir(name, true)?;

        Self::dir_copy(src_path, &work_dir)?;
        Ok(work_dir)
    }

    pub fn app(post_process: bool) -> App {
        let bard_exe = option_env!("CARGO_BIN_EXE_bard")
            .expect("$CARGO_BIN_EXE_bard")
            .into();
        App::with_test_mode(post_process, bard_exe)
    }

    fn build_inner(src_path: impl AsRef<Path>, name: &str, post_process: bool) -> Result<Self> {
        let app = Self::app(post_process);

        let work_dir = Self::prepare(src_path, name)?;
        let project = bard::bard_make_at(&app, &work_dir)?;

        Ok(Self {
            project,
            dir: work_dir,
            app,
        })
    }

    pub fn build(src_path: PathBuf) -> Result<Self> {
        Self::build_inner(
            &src_path,
            src_path.file_name().unwrap().to_str().unwrap(),
            false,
        )
    }

    pub fn build_with_name(src_path: PathBuf, name: &str) -> Result<Self> {
        Self::build_inner(src_path, name, false)
    }

    pub fn build_with_ps(src_path: impl AsRef<Path>, name: &str) -> Result<Self> {
        Self::build_inner(src_path, name, true)
    }

    pub fn init(app: &App, name: &str) -> Result<PathBuf> {
        let work_dir = Self::work_dir(name, true)?;
        fs::create_dir_all(&work_dir)
            .with_context(|| format!("Could create directory: {:?}", work_dir))?;
        bard::bard_init_at(app, &work_dir).context("Failed to initialize")?;
        Ok(work_dir)
    }

    pub fn init_and_build(name: &str) -> Result<Self> {
        let app = Self::app(false);
        let work_dir = Self::init(&app, name)?;
        let project = bard::bard_make_at(&app, &work_dir)?;

        Ok(Self {
            project,
            dir: work_dir,
            app,
        })
    }
}

pub fn bard_exe() -> PathBuf {
    env!("CARGO_BIN_EXE_bard").into()
}

pub struct ExeBuilder {
    pub work_dir: PathBuf,
    bin_dir: PathBuf,
    bard_exe: PathBuf,
    custom_path: bool,
    env: HashMap<String, String>,
}

impl ExeBuilder {
    pub fn tex_mock_exe() -> PathBuf {
        env!("CARGO_BIN_EXE_tex-mock").into()
    }

    pub fn init(name: &str) -> Result<Self> {
        let app = Builder::app(false);
        let work_dir = Builder::init(&app, name)?;

        let bin_dir = work_dir.join("bin");
        fs::create_dir(&bin_dir).context("Could not create bin subdir")?;

        Ok(Self {
            work_dir,
            bin_dir,
            bard_exe: bard_exe(),
            custom_path: false,
            env: HashMap::new(),
        })
    }

    pub fn with_xelatex_bin(mut self) -> Self {
        let mock_exe = Self::tex_mock_exe();
        let mut target = self.bin_dir.join("xelatex");
        if let Some(ext) = mock_exe.extension() {
            target.set_extension(ext);
        }
        fs::copy(&mock_exe, &target).unwrap();
        self.custom_path = true;
        self
    }

    pub fn with_tectonic_bin(mut self) -> Self {
        let mock_exe = Self::tex_mock_exe();
        let mut target = self.bin_dir.join("tectonic");
        if let Some(ext) = mock_exe.extension() {
            target.set_extension(ext);
        }
        fs::copy(&mock_exe, &target).unwrap();
        self.custom_path = true;
        self
    }

    pub fn custom_path(mut self, custom_path: bool) -> Self {
        self.custom_path = custom_path;
        self
    }

    pub fn with_env(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.env.insert(k.into(), v.into());
        self
    }

    pub fn run(self, args: &[&str]) -> Result<Self> {
        Command::new(&self.bard_exe)
            .apply(|mut cmd| {
                dbg!(self.custom_path);
                if self.custom_path {
                    cmd.env_clear().env("PATH", &self.bin_dir);
                }
                cmd
            })
            .envs(self.env.iter())
            .args(args)
            .current_dir(&self.work_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .context("Failed to run bard")?
            .into_result()
            .context("bard exited with failed status")?;

        Ok(self)
    }

    pub fn out_dir(&self) -> PathBuf {
        self.work_dir.join("output")
    }

    pub fn output(&self, filename: &str) -> PathBuf {
        self.out_dir().join(filename)
    }

    pub fn find_tmp_dir(&self, output: &str) -> Option<PathBuf> {
        let prefix = format!("{}.", output);
        self.out_dir()
            .read_dir()
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_str()
                    .map(|name| name.starts_with(&prefix))
                    == Some(true)
            })
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
