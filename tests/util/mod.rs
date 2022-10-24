use std::env;
use std::fs;
use std::ops;

use fs_extra::dir::{self, CopyOptions};

use bard::cli;
use bard::prelude::*;
use bard::project::Project;
use bard::MakeOpts;

const INT_DIR: &str = "int-test-workdirs";

pub const OPTS_NO_PS: MakeOpts = MakeOpts {
    no_postprocess: true,
};
pub const OPTS_PS: MakeOpts = MakeOpts {
    no_postprocess: false,
};

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

pub fn assert_file_contains<P: AsRef<Path>>(path: P, what: &str) {
    let content = fs::read_to_string(path.as_ref()).unwrap();
    let hit = content.find(what);
    assert!(
        hit.is_some(),
        "String `{}` not found in file: `{}`\nFile contents:\n{}",
        what,
        path.as_ref(),
        content
    );
}

pub fn int_dir() -> PathBuf {
    // Cargo suppor for tmpdir merged yay https://github.com/rust-lang/cargo/pull/9375
    // but we should still support old cargos, better to use option_env:
    option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .unwrap_or(
            [env!("CARGO_MANIFEST_DIR"), "target", INT_DIR]
                .iter()
                .collect(),
        )
}

#[derive(Debug)]
pub struct Builder {
    pub project: Project,
    pub dir: PathBuf,
}

impl Builder {
    pub fn work_dir(name: &str, rm: bool) -> Result<PathBuf> {
        let path = int_dir().join(name);

        if rm {
            if path.exists() {
                fs::remove_dir_all(&path).with_context(|| {
                    format!("Couldn't remove previous test run data: `{}`", path)
                })?;
            }
        }

        Ok(path)
    }

    fn dir_copy(src: impl AsRef<Path>, dest: impl AsRef<Path>) -> Result<()> {
        let src = src.as_ref();
        let dest = dest.as_ref();

        fs::create_dir_all(dest)
            .with_context(|| format!("Couldn't create directory: `{}`", dest))?;

        let mut opts = CopyOptions::new();
        opts.content_only = true;
        dir::copy(src, dest, &opts)
            .with_context(|| format!("Couldn't copy directory `{}` to `{}`", src, dest))?;
        Ok(())
    }

    pub fn prepare(src_path: impl AsRef<Path>, name: &str) -> Result<PathBuf> {
        cli::use_stderr(true);

        let src_path = src_path.as_ref();
        let work_dir = Self::work_dir(name, true)?;

        Self::dir_copy(src_path, &work_dir)?;
        Ok(work_dir)
    }

    pub fn build(src_path: PathBuf) -> Result<Self> {
        Self::build_opts(&src_path, src_path.file_name().unwrap(), &OPTS_NO_PS)
    }

    pub fn build_opts(src_path: impl AsRef<Path>, name: &str, opts: &MakeOpts) -> Result<Self> {
        cli::use_stderr(true);

        let work_dir = Self::prepare(src_path, name)?;
        let project = bard::bard_make_at(opts, &work_dir)?;

        Ok(Self {
            project,
            dir: work_dir,
        })
    }

    pub fn init_and_build(name: &str, opts: &MakeOpts) -> Result<Self> {
        cli::use_stderr(true);

        let work_dir = Self::work_dir(name.as_ref(), true)?;
        fs::create_dir_all(&work_dir)
            .with_context(|| format!("Could create directory: `{}`", work_dir))?;

        bard::bard_init_at(&work_dir).context("Failed to initialize")?;
        let project = bard::bard_make_at(opts, &work_dir)?;

        Ok(Self {
            project,
            dir: work_dir,
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
