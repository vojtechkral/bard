use std::convert::TryFrom;
use std::env;
use std::process::Command;

use camino::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};

use crate::project::Output;
use crate::util::ExitStatusExt;
use crate::{cli, error::*};

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum CmdSpec {
    Basic(String),
    Multiple(Vec<String>),
    Extended(Vec<Vec<String>>),
}

impl CmdSpec {
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Basic(s) => s.is_empty(),
            Self::Multiple(v) => v.is_empty(),
            Self::Extended(v) => v.is_empty(),
        }
    }
}

#[derive(Serialize, Debug)]
struct PostProcessCtx<'a> {
    bard: String,
    file: &'a str,
    file_name: &'a str,
    file_stem: &'a str,
    project_dir: &'a str,
}

impl<'a> PostProcessCtx<'a> {
    pub fn new(file: &'a Path, project_dir: &'a Path) -> Result<Self> {
        let bard = env::current_exe()
            .map_err(Error::from)
            .and_then(|p| PathBuf::try_from(p).map_err(Error::from))
            .map(|p| p.to_string())
            .context("Could not read path to bard executable")?;

        // NOTE: Filenames should be canonicalized at this point
        let file_name = file.file_name().unwrap();
        let file_stem = file.file_stem().unwrap_or(file_name);

        Ok(Self {
            bard,
            file: file.as_str(),
            file_name,
            file_stem,
            project_dir: project_dir.as_str(),
        })
    }
}

pub struct PostProcessor<'a> {
    project_dir: &'a Path,
    output_dir: &'a Path,
}

impl<'a> PostProcessor<'a> {
    pub fn new(project_dir: &'a Path, output_dir: &'a Path) -> Self {
        Self {
            project_dir,
            output_dir,
        }
    }

    fn post_process_one(
        &self,
        context: &PostProcessCtx<'a>,
        mut args: impl Iterator<Item = &'a str>,
    ) -> Result<()> {
        let arg0 = match args.next() {
            Some(arg0) => arg0,
            None => return Ok(()), // No command does nothing
        };

        let hb = Handlebars::new();
        let arg0_r = hb
            .render_template(arg0, context)
            .with_context(|| format!("Could not substitute command: '{}'", arg0))?;

        let mut cmd = Command::new(arg0_r.clone());
        let mut cmd_src = arg0_r;

        for arg in args {
            // Accumulate args here for error reporting:
            cmd_src.push(' ');
            cmd_src.push_str(arg);

            let arg_r = hb.render_template(arg, context).with_context(|| {
                format!("Could not substitute command arguments: '{}'", cmd_src)
            })?;

            // Replace the arg with the interpolated content after succesful render
            cmd_src.truncate(cmd_src.len() - arg.len());
            cmd_src.push_str(&arg_r);

            cmd.arg(&arg_r);
        }

        cmd.current_dir(self.output_dir);

        cli::status("Postprocess", &cmd_src);

        let status = cmd
            .status()
            .with_context(|| format!("Failed to run processing command '{}'", cmd_src))?;

        status
            .into_result()
            .with_context(|| format!("Processing command '{}' failed", cmd_src))
    }

    pub fn run(&self, output: &'a Output) -> Result<()> {
        let cmds = match output.post_process() {
            Some(cmds) if !cmds.is_empty() => cmds,
            _ => return Ok(()),
        };

        let context = PostProcessCtx::new(&output.file, self.project_dir)?;

        match cmds {
            CmdSpec::Basic(cmd) => self.post_process_one(&context, cmd.split_whitespace())?,
            CmdSpec::Multiple(vec) => {
                for cmd in vec.iter() {
                    self.post_process_one(&context, cmd.split_ascii_whitespace())?;
                }
            }
            CmdSpec::Extended(vec) => {
                for cmd in vec.iter() {
                    self.post_process_one(&context, cmd.iter().map(String::as_str))?;
                }
            }
        }

        Ok(())
    }
}
