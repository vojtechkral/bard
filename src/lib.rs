//! `bard`, the Markdown-based songbook compiler.
//!
//! #### **This is not a public API.**
//! This library is an implementation detail of the `bard` CLI tool.
//! These APIs are internal and may break without notice.

use std::convert::TryFrom;
use std::env;
use std::ffi::OsString;

use camino::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use clap::Parser as _;
use serde::Serialize;

pub mod book;
pub mod cli;
pub mod default_project;
pub mod error;
pub mod music;
pub mod parser;
pub mod project;
pub mod render;
pub mod util;
pub mod util_cmd;
pub mod watch;

use crate::error::*;
use crate::project::Project;
use crate::util_cmd::UtilCmd;
use crate::watch::{Watch, WatchEvent};

#[derive(Serialize, Clone, Debug)]
pub struct ProgramMeta {
    name: &'static str,
    version: &'static str,
    description: &'static str,
    homepage: &'static str,
    authors: &'static str,
}

pub const PROGRAM_META: ProgramMeta = ProgramMeta {
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    description: env!("CARGO_PKG_DESCRIPTION"),
    homepage: env!("CARGO_PKG_HOMEPAGE"),
    authors: env!("CARGO_PKG_AUTHORS"),
};

#[derive(clap::Parser, Clone, Default, Debug)]
pub struct MakeOpts {
    #[clap(short = 'p', long, help = "Don't run outputs' postprocessing steps")]
    pub no_postprocess: bool,
}

#[derive(clap::Parser)]
#[clap(
    version = env!("CARGO_PKG_VERSION"),
    about = "bard: A Markdown-based songbook compiler",
)]
enum Bard {
    #[clap(about = "Initialize a new bard project skeleton in this directory")]
    Init,
    #[clap(about = "Build the current project")]
    Make {
        #[clap(flatten)]
        opts: MakeOpts,
    },
    #[clap(
        about = "Like make, but keep runing and rebuild each time there's a change in project files"
    )]
    Watch {
        #[clap(flatten)]
        opts: MakeOpts,
    },
    #[clap(subcommand, about = "Commandline utilities for postprocessing")]
    Util(UtilCmd),
}

impl Bard {
    fn run(self) -> Result<()> {
        use Bard::*;

        match self {
            Init => bard_init(),
            Make { opts } => bard_make(&opts),
            Watch { opts } => bard_watch(&opts),
            Util(cmd) => bard_util(cmd),
        }
    }
}

fn get_cwd() -> Result<PathBuf> {
    env::current_dir()
        .map_err(Error::from)
        .and_then(|p| PathBuf::try_from(p).map_err(Error::from))
        .context("Could not read current directory")
}

pub fn bard_init_at<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();

    cli::status("Initialize", &format!("new project at {}", path));
    Project::init(&path).context("Could not initialize a new project")?;
    cli::success("Done!");
    Ok(())
}

pub fn bard_init() -> Result<()> {
    let cwd = get_cwd()?;
    bard_init_at(&cwd)
}

pub fn bard_make_at<P: AsRef<Path>>(opts: &MakeOpts, path: P) -> Result<Project> {
    Project::new(path.as_ref())
        .and_then(|mut project| {
            project.enable_postprocess(!opts.no_postprocess);
            project.render()?;
            Ok(project)
        })
        .context("Could not make project")
}

pub fn bard_make(opts: &MakeOpts) -> Result<()> {
    let cwd = get_cwd()?;

    bard_make_at(opts, &cwd)?;
    cli::success("Done!");
    Ok(())
}

pub fn bard_watch_at<P: AsRef<Path>>(opts: &MakeOpts, path: P, mut watch: Watch) -> Result<()> {
    loop {
        let project = bard_make_at(opts, &path)?;

        eprintln!();
        cli::status("Watching", "for changes in the project ...");
        match watch.watch(&project)? {
            WatchEvent::Path(path) => {
                cli::status(
                    "",
                    &format!("Modification detected at '{}' ...", path.display()),
                );
            }
            WatchEvent::Pathless => cli::status("", "Modification detected ..."),
            WatchEvent::Cancel => break,
        }
    }

    Ok(())
}

pub fn bard_watch(opts: &MakeOpts) -> Result<()> {
    let cwd = get_cwd()?;
    let (watch, cancellation) = Watch::new()?;

    let _ = ctrlc::set_handler(move || {
        cancellation.cancel();
    });

    bard_watch_at(opts, &cwd, watch)
}

pub fn bard_util(cmd: UtilCmd) -> Result<()> {
    util_cmd::util_cmd(cmd)
}

pub fn bard(args: &[OsString]) -> Result<()> {
    Bard::parse_from(args).run()
}
