use std::env;
use std::path::PathBuf;
use std::fs;

use anyhow::{Result, Context};
use fs_extra::dir::copy as dir_copy;
use fs_extra::dir::CopyOptions;

use bard::project::Project;

#[derive(Debug)]
pub struct Builder {
    pub project: Project,
    pub dir: PathBuf,
}

impl Builder {
    pub fn build(name: &str) -> Result<Self> {
        let root = env::var("CARGO_MANIFEST_DIR").unwrap();
        let src_path = format!("{}/tests/projects/{}", root, name);
        let src_path = PathBuf::from(src_path);

        // FIXME: `target` may be located else where, this is brittle,
        // write a patch to cargo to pass `CARGO_TARGET_DIR` to tests/bins.
        let test_dir = format!("{}/target/tests-workdir", root);
        let mut test_dir = PathBuf::from(test_dir);
        fs::create_dir_all(&test_dir)
            .with_context(|| format!("Couldn't create test work dir: `{}`", test_dir.display()))?;

        let prev_run = test_dir.join(name);
        if prev_run.exists() {
            fs::remove_dir_all(&prev_run)
                .with_context(|| format!("Couldn't remove previous test run data: `{}`", prev_run.display()))?;
        }

        dir_copy(src_path, &test_dir, &CopyOptions::new())
            .with_context(|| format!("Couldn't copy test project `{}` into test work dir: `{}`", name, test_dir.display()))?;

        test_dir.push(name);

        let project = bard::bard_make_at(&test_dir)?;

        Ok(Self {
            project,
            dir: test_dir,
        })
    }
}
