//! `bard`, the Markdown-based songbook compiler.
//!
//! > ### <span style="font-variant: small-caps">**This is not a public API.** </span>
//! This library is an implementation detail of the `bard` CLI tool.
//! These APIs are internal and may break without notice.

#![allow(clippy::new_ret_no_self)]
#![allow(clippy::comparison_chain)]

use std::env;
use std::ffi::OsString;

use app::{App, MakeOpts, StdioOpts};
use clap::Parser as _;
use serde::Serialize;

pub mod app;
pub mod book;
pub mod default_project;
pub mod music;
pub mod parser;
pub mod prelude;
pub mod project;
pub mod render;
#[cfg(feature = "tectonic")]
pub mod tectonic_embed;
pub mod util;
pub mod util_cmd;
pub mod watch;

use crate::prelude::*;
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

#[derive(clap::Parser)]
#[command(
    version = env!("CARGO_PKG_VERSION"),
    about = "bard: A Markdown-based songbook compiler",
)]
enum Cli {
    #[command(about = "Initialize a new bard project skeleton in this directory")]
    Init {
        #[clap(flatten)]
        opts: StdioOpts,
    },
    #[command(about = "Build the current project")]
    Make {
        #[clap(flatten)]
        opts: MakeOpts,
    },
    #[command(
        about = "Like make, but keep runing and rebuild each time there's a change in project files"
    )]
    Watch {
        #[clap(flatten)]
        opts: MakeOpts,
    },
    #[command(subcommand, about = "Commandline utilities for postprocessing")]
    Util(UtilCmd),

    #[cfg(feature = "tectonic")]
    #[command(hide = true)]
    Tectonic(tectonic_embed::Tectonic),
}

impl Cli {
    fn run(self, app: &App) -> Result<()> {
        use Cli::*;

        match self {
            Init { .. } => bard_init(app),
            Make { .. } => bard_make(app),
            Watch { .. } => bard_watch(app),
            Util(cmd) => cmd.run(app),

            #[cfg(feature = "tectonic")]
            Tectonic(tectonic) => tectonic.run(app),
        }
    }
}

fn get_cwd() -> Result<PathBuf> {
    env::current_dir().context("Could not read current directory")
}

pub fn bard_init_at<P: AsRef<Path>>(app: &App, path: P) -> Result<()> {
    let path = path.as_ref();

    app.status("Initialize", format!("new project at {:?}", path));
    Project::init(path).context("Could not initialize a new project")?;
    app.success("Done!");
    Ok(())
}

pub fn bard_init(app: &App) -> Result<()> {
    let cwd = get_cwd()?;
    bard_init_at(app, cwd)
}

pub fn bard_make_at<P: AsRef<Path>>(app: &App, path: P) -> Result<Project> {
    Project::new(app, path.as_ref())
        .and_then(|project| {
            project.render(app)?;
            Ok(project)
        })
        .context("Could not make project")
}

pub fn bard_make(app: &App) -> Result<()> {
    let cwd = get_cwd()?;

    bard_make_at(app, cwd)?;
    app.success("Done!");
    Ok(())
}

pub fn bard_watch_at<P: AsRef<Path>>(app: &App, path: P, mut watch: Watch) -> Result<()> {
    loop {
        let project = bard_make_at(app, &path)?;

        eprintln!();
        app.status("Watching", "for changes in the project ...");
        match watch.watch(&project)? {
            WatchEvent::Change(paths) if paths.len() == 1 => {
                app.indent(format!("Change detected at {:?} ...", paths[0]))
            }
            WatchEvent::Change(..) => app.indent("Change detected ..."),
            WatchEvent::Cancel => break,
            WatchEvent::Error(err) => return Err(err),
        }
    }

    Ok(())
}

pub fn bard_watch(app: &App) -> Result<()> {
    let cwd = get_cwd()?;
    let (watch, cancellation) = Watch::new()?;

    let _ = ctrlc::set_handler(move || {
        cancellation.cancel();
    });

    bard_watch_at(app, cwd, watch)
}

pub fn bard(args: &[OsString]) -> i32 {
    let cli = Cli::parse_from(args);
    let app = match &cli {
        Cli::Init { opts } => App::new(&opts.clone().into()),
        Cli::Make { opts } => App::new(opts),
        Cli::Watch { opts } => App::new(opts),
        Cli::Util(_) => App::new(&Default::default()),

        #[cfg(feature = "tectonic")]
        Cli::Tectonic(_) => App::new_as_tectonic(),
    };

    if let Err(err) = cli.run(&app) {
        app.error(err);
        1
    } else {
        0
    }
}
