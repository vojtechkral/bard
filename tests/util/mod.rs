use std::env;
use std::path::{Path, PathBuf};
use std::fs;

use anyhow::{Result, Context};
use fs_extra::dir::{self, CopyOptions};

use bard::cli;
use bard::project::Project;

const INT_DIR: &str = "int-test-workdirs";

pub fn assert_file_contains<P: AsRef<Path>>(path: P, what: &str) {
    let content = fs::read_to_string(&path).unwrap();
    let hit = content.find(what);
    assert!(hit.is_some(), "String `{}` not found in file: `{}`", what, path.as_ref().display());
}

#[derive(Debug)]
pub struct Builder {
    pub project: Project,
    pub dir: PathBuf,
}

impl Builder {
    fn source_dir(name: &str) -> PathBuf {
        let root = env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = format!("{}/tests/projects/{}", root, name);
        PathBuf::from(path)
    }

    fn work_dir(name: &str, rm: bool) -> Result<PathBuf> {
        let root = env::var("CARGO_MANIFEST_DIR").unwrap();

        // FIXME: `target` may be located elsewhere, this is brittle,
        // write a patch to cargo to pass `CARGO_TARGET_DIR` to tests/bins.
        let path = format!("{}/target/{}/{}", root, INT_DIR, name);
        let path = PathBuf::from(path);

        if rm {
            if path.exists() {
                fs::remove_dir_all(&path)
                    .with_context(|| format!("Couldn't remove previous test run data: `{}`", path.display()))?;
            }
        }

        Ok(path)
    }

    fn dir_copy<P1, P2>(src: P1, dest: P2) -> Result<()> where P1: AsRef<Path>, P2: AsRef<Path> {
        let src = src.as_ref();
        let dest = dest.as_ref();

        fs::create_dir_all(dest)
            .with_context(|| format!("Couldn't create directory: `{}`", dest.display()))?;

        let mut opts = CopyOptions::new();
        opts.content_only = true;
        dir::copy(src, dest, &opts)
            .with_context(|| format!("Couldn't copy directory `{}` to `{}`", src.display(), dest.display()))?;
        Ok(())
    }

    pub fn build(name: &str) -> Result<Self> {
        cli::use_stderr(true);

        let src_path = Self::source_dir(name);
        let work_dir = Self::work_dir(name, true)?;

        Self::dir_copy(src_path, &work_dir)?;
        let project = bard::bard_make_at(&work_dir)?;

        Ok(Self {
            project,
            dir: work_dir,
        })
    }

    pub fn init_and_build(name: &str) -> Result<Self> {
        cli::use_stderr(true);

        let work_dir = Self::work_dir(name, true)?;
        fs::create_dir_all(&work_dir).with_context(|| format!("Could create directory: `{}`", work_dir.display()))?;

        bard::bard_init_at(&work_dir).context("Failed to initialize")?;
        let project = bard::bard_make_at(&work_dir)?;

        Ok(Self {
            project,
            dir: work_dir,
        })
    }
}
