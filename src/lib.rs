//! `bard`, the Markdown-based songbook compiler.
//!
//! > ### <span style="font-variant: small-caps">**This is not a public API.** </span>
//! This library is an implementation detail of the `bard` CLI tool.
//! These APIs are internal and may break without notice.

#![allow(clippy::new_ret_no_self)]
#![allow(clippy::comparison_chain)]
#![allow(clippy::uninlined_format_args)]

use std::env;
use std::ffi::OsString;

use app::{App, MakeOpts, StdioOpts};
use clap::{CommandFactory as _, Parser as _};
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
use crate::project::{Project, Settings};
use crate::util_cmd::UtilCmd;
use crate::watch::{Watch, WatchEvent};

#[derive(Serialize, Clone, Debug)]
pub struct ProgramMeta {
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub homepage: &'static str,
    pub authors: &'static str,
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
    help_expected = true,
    disable_version_flag = true,
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Command>,

    /// Print program version in semver format
    #[arg(short = 'V', long, conflicts_with = "version_settings")]
    pub version: bool,
    /// Print project settings file version in semver format
    #[arg(long, conflicts_with = "version_ast")]
    pub version_settings: bool,
    /// Print project template AST version in semver format
    #[arg(long, conflicts_with = "version")]
    pub version_ast: bool,
}

impl Cli {
    fn print_version(&self) -> bool {
        if self.version {
            println!("{}", PROGRAM_META.version);
        }
        if self.version_settings {
            println!("{}", Settings::version());
        }
        if self.version_ast {
            println!("{}", book::version::current());
        }

        self.version || self.version_settings || self.version_ast
    }
}

#[derive(clap::Parser)]
enum Command {
    /// Initialize a new bard project skeleton in this directory
    Init {
        #[clap(flatten)]
        opts: StdioOpts,
    },
    /// Build the current project"
    Make {
        #[clap(flatten)]
        opts: MakeOpts,
    },
    /// Like make, but keep runing and rebuild each time there's a change in project files
    Watch {
        #[clap(flatten)]
        opts: MakeOpts,
    },
    /// Commandline utilities for postprocessing
    #[command(subcommand)]
    Util(UtilCmd),

    #[cfg(feature = "tectonic")]
    #[command(hide = true)]
    Tectonic(tectonic_embed::Tectonic),
}

impl Command {
    fn run(self, app: &App) -> Result<()> {
        use Command::*;

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
    if cli.print_version() {
        return 0;
    }

    let cmd = if let Some(cmd) = cli.cmd {
        cmd
    } else {
        let _ = Cli::command().print_help();
        return 0;
    };

    let app = match &cmd {
        Command::Init { opts } => App::new(&opts.clone().into()),
        Command::Make { opts } => App::new(opts),
        Command::Watch { opts } => App::new(opts),
        Command::Util(_) => App::new(&Default::default()),

        #[cfg(feature = "tectonic")]
        Command::Tectonic(_) => App::new_as_tectonic(),
    };

    if let Err(err) = cmd.run(&app) {
        app.error(err);
        1
    } else {
        0
    }
}
