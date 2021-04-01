use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use clap::{App, SubCommand, AppSettings};
use serde::Serialize;

pub mod project;
pub mod default_project;
pub mod music;
pub mod parser;
pub mod book;
pub mod render;
pub mod watch;
pub mod cli;
pub mod util;
pub mod error;

use crate::project::Project;
use crate::watch::{Watch, WatchEvent};
use crate::error::*;

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

fn get_cwd() -> Result<PathBuf> {
    let cwd = env::current_dir().context("Could not read current directory")?;
    ensure!(
        cwd.as_path().to_str().is_some(),
        format!("Path is not valid unicode: '{}'", cwd.display())
    );
    Ok(cwd)
}

pub fn bard_init() -> Result<()> {
    let cwd = get_cwd()?;

    cli::status("Initialize", &format!("new project at {}", cwd.display()));
    Project::init(&cwd).context("Could not initialize a new project")?;
    cli::success("Done!");
    Ok(())
}

pub fn bard_make_at(path: &Path) -> Result<Project> {
    Project::new(path)
        .and_then(|project| {
            project.render()?;
            Ok(project)
        })
        .context("Could not make project")
}

pub fn bard_make() -> Result<Project> {
    let cwd = get_cwd()?;

    let project = bard_make_at(&cwd)?;
    cli::success("Done!");
    Ok(project)
}

pub fn bard_watch() -> Result<()> {
    let cwd = get_cwd()?;
    let (mut watch, cancellation) = Watch::new()?;

    let _ = ctrlc::set_handler(move || {
        cancellation.cancel();
    });

    loop {
        let project = bard_make_at(&cwd)?;

        eprintln!("");
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

pub fn bard(args: &[OsString]) -> Result<()> {
    let args = App::new("bard")
        .version("0.3")
        .author("Vojtech Kral <vojtech@kral.hk>")
        .about("bard: Songbook compiler")
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("init")
                .about("Initialize and empty project with default settings"),
        )
        .subcommand(
            SubCommand::with_name("make")
                .about("Process the current project and generate output files"),
        )
        .subcommand(SubCommand::with_name("watch").about(
            "Watch the current project and its input files for changes, and re-make the project \
             each time there's a change",
        ))
        .get_matches_from(args.iter());

    if let Some(_args) = args.subcommand_matches("init") {
        bard_init()?;
    } else if let Some(_args) = args.subcommand_matches("make") {
        bard_make()?;
    } else if let Some(_args) = args.subcommand_matches("watch") {
        bard_watch()?;
    }

    Ok(())
}
