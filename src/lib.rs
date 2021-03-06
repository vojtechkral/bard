use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::Serialize;
use structopt::clap::AppSettings;
use structopt::StructOpt;

pub mod book;
pub mod cli;
pub mod default_project;
pub mod error;
pub mod music;
pub mod parser;
pub mod project;
pub mod render;
pub mod util;
pub mod watch;

use crate::error::*;
use crate::project::Project;
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

#[derive(StructOpt, Clone, Default, Debug)]
pub struct MakeOpts {
    #[structopt(short = "p", long)]
    pub no_postprocess: bool,
}

#[derive(StructOpt)]
#[structopt(
    version = env!("CARGO_PKG_VERSION"),
    about = "bard: A Markdown-based songbook compiler",
)]
enum Bard {
    Init,
    Make {
        #[structopt(flatten)]
        opts: MakeOpts,
    },
    Watch {
        #[structopt(flatten)]
        opts: MakeOpts,
    },
}

impl Bard {
    fn run(self) -> Result<()> {
        use Bard::*;

        match self {
            Init => bard_init(),
            Make { opts } => bard_make(&opts),
            Watch { opts } => bard_watch(&opts),
        }
    }
}

fn get_cwd() -> Result<PathBuf> {
    let cwd = env::current_dir().context("Could not read current directory")?;
    ensure!(
        cwd.as_path().to_str().is_some(),
        format!("Path is not valid unicode: '{}'", cwd.display())
    );
    Ok(cwd)
}

pub fn bard_init_at<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();

    cli::status("Initialize", &format!("new project at {}", path.display()));
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

pub fn bard(args: &[OsString]) -> Result<()> {
    Bard::from_clap(
        &Bard::clap()
            .setting(AppSettings::VersionlessSubcommands)
            .setting(AppSettings::ArgRequiredElseHelp)
            .get_matches_from(args.iter()),
    )
    .run()
}
