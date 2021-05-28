use std::env;
use std::ffi::OsStr;
use std::fs;
use std::ops;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fs_extra::dir::{self, CopyOptions};

use bard::cli;
use bard::project::Project;
use bard::MakeOpts;

const INT_DIR: &str = "int-test-workdirs";

pub const OPTS_NO_PS: MakeOpts = MakeOpts {
    no_postprocess: true,
};

pub const ROOT: ProjectPath = ProjectPath { path: &[] };

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
    let content = fs::read_to_string(&path).unwrap();
    let hit = content.find(what);
    assert!(
        hit.is_some(),
        "String `{}` not found in file: `{}`\nFile contents:\n{}",
        what,
        path.as_ref().display(),
        content
    );
}

#[derive(Debug)]
pub struct Builder {
    pub project: Project,
    pub dir: PathBuf,
}

impl Builder {
    fn work_dir(name: &OsStr, rm: bool) -> Result<PathBuf> {
        // Cargo suppor for tmpdir merged yay https://github.com/rust-lang/cargo/pull/9375
        // but we should still support old cargos, better to use option_env:
        let path = option_env!("CARGO_TARGET_TMPDIR")
            .map(|tmpdir| PathBuf::from(tmpdir).join(name))
            .unwrap_or(
                [env!("CARGO_MANIFEST_DIR"), "target", INT_DIR]
                    .iter()
                    .collect(),
            )
            .join(name);

        if rm {
            if path.exists() {
                fs::remove_dir_all(&path).with_context(|| {
                    format!(
                        "Couldn't remove previous test run data: `{}`",
                        path.display()
                    )
                })?;
            }
        }

        Ok(path)
    }

    fn dir_copy<P1, P2>(src: P1, dest: P2) -> Result<()>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        let src = src.as_ref();
        let dest = dest.as_ref();

        fs::create_dir_all(dest)
            .with_context(|| format!("Couldn't create directory: `{}`", dest.display()))?;

        let mut opts = CopyOptions::new();
        opts.content_only = true;
        dir::copy(src, dest, &opts).with_context(|| {
            format!(
                "Couldn't copy directory `{}` to `{}`",
                src.display(),
                dest.display()
            )
        })?;
        Ok(())
    }

    pub fn build(src_path: PathBuf) -> Result<Self> {
        Self::build_opts(src_path, &OPTS_NO_PS)
    }

    pub fn build_opts(src_path: PathBuf, opts: &MakeOpts) -> Result<Self> {
        cli::use_stderr(true);

        // let src_path = Self::source_dir(name);
        let name = src_path.file_name().unwrap();
        let work_dir = Self::work_dir(name, true)?;

        Self::dir_copy(src_path, &work_dir)?;
        let project = bard::bard_make_at(opts, &work_dir)?;

        Ok(Self {
            project,
            dir: work_dir,
        })
    }

    pub fn init_and_build(name: &str) -> Result<Self> {
        cli::use_stderr(true);

        let work_dir = Self::work_dir(name.as_ref(), true)?;
        fs::create_dir_all(&work_dir)
            .with_context(|| format!("Could create directory: `{}`", work_dir.display()))?;

        bard::bard_init_at(&work_dir).context("Failed to initialize")?;
        let project = bard::bard_make_at(&OPTS_NO_PS, &work_dir)?;

        Ok(Self {
            project,
            dir: work_dir,
        })
    }
}
