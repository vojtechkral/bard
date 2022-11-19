use std::io::{BufRead, Write};
use std::ops::Deref;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::time::Duration;
use std::{env, fmt, fs, io, iter, thread};

use parking_lot::{const_mutex, Mutex, MutexGuard};
use serde::de::Error as _;
use serde::Deserialize;

use crate::app::App;
use crate::prelude::*;
use crate::util::{ExitStatusExt, ProcessLines};
use crate::util_cmd;

static TEX_TOOLS: Mutex<Option<TexTools>> = const_mutex(None);

#[derive(Clone, Debug)]
pub enum TexConfig {
    TexLive { program: Option<String> },
    Tectonic { program: Option<String> },
    None,
}

impl FromStr for TexConfig {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (kind, program) = input
            .split_once(':')
            .map(|(a, b)| (a, Some(b.to_string())))
            .unwrap_or((input, None));

        let res = if kind.eq_ignore_ascii_case("texlive") {
            Self::TexLive { program }
        } else if kind.eq_ignore_ascii_case("tectonic") {
            Self::Tectonic { program }
        } else if kind.eq_ignore_ascii_case("none") {
            Self::None
        } else {
            bail!("Unexpected TeX distro type: '{}', possible choices are: 'texlive', 'tectonic', or 'none'. The syntax for TeX config is 'distro' or 'distro:program'.", kind);
        };

        Ok(res)
    }
}

impl<'de> Deserialize<'de> for TexConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let input: &'de str = Deserialize::deserialize(deserializer)?;
        input.parse().map_err(D::Error::custom)
    }
}

impl fmt::Display for TexConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TexConfig::TexLive { program } => {
                write!(f, "texlive")?;
                if let Some(p) = program {
                    write!(f, ":{}", p)?;
                }
            }
            TexConfig::Tectonic { program } => {
                write!(f, "tectonic")?;
                if let Some(p) = program {
                    write!(f, ":{}", p)?;
                }
            }
            TexConfig::None => write!(f, "none")?,
        }

        Ok(())
    }
}

/// Run a command and get first line from stdout, if any
fn test_program(program: &str, arg1: &str) -> Result<String> {
    let mut child = Command::new(program)
        .arg(arg1)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    // Crude way to wait for the subprocess with a timeout.
    for _ in 0..30 {
        if let Some(status) = child.try_wait()? {
            status.into_result()?;
            break;
        }

        thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();

    let stdout = child.stdout.take().map(io::BufReader::new).unwrap();
    let first_line = stdout
        .lines()
        .next()
        .ok_or_else(|| anyhow!("No output from program {}", program))??;
    if first_line.is_empty() || first_line.chars().all(|c| c.is_ascii_whitespace()) {
        bail!("No output from program {}", program);
    }
    Ok(first_line)
}

fn run_program(app: &App, program: &str, args: &[&str], cwd: &Path) -> Result<()> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Could not run program '{}'", program))?;

    let mut ps_lines =
        ProcessLines::new(child.stdout.take().unwrap(), child.stderr.take().unwrap());

    app.subprocess_output(&mut ps_lines, program)?;

    let status = child
        .wait()
        .with_context(|| format!("Error running program '{}'", program))?;

    if !status.success() && app.verbosity() == 1 {
        let cmdline =
            iter::once(&program)
                .chain(args.iter())
                .fold(String::new(), |mut cmdline, arg| {
                    cmdline.push_str(arg);
                    cmdline.push(' ');
                    cmdline
                });
        eprintln!("{}", cmdline);

        let stderr = io::stderr();
        let mut stderr = stderr.lock();
        for line in ps_lines.collected_lines() {
            let _ = stderr.write_all(line);
        }
    }

    status.into_result()
}

#[derive(Debug)]
pub struct TexRenderJob<'a> {
    pub tex_file: &'a Path,
    pub out_dir: &'a Path,
    pub pdf_path: &'a Path,
    pub toc_sort_key: Option<&'a str>,
}

impl<'a> TexRenderJob<'a> {
    fn cwd(&self) -> &'a Path {
        self.tex_file.parent().unwrap()
    }

    fn sort_toc(&self) -> Result<()> {
        let key = match self.toc_sort_key {
            Some(key) => key,
            None => return Ok(()),
        };

        let tex_stem = self.tex_file.file_stem().unwrap();
        let toc = self.out_dir.join(format!("{}.toc", tex_stem));

        if toc.exists() {
            util_cmd::sort_lines(key, &toc)
                .with_context(|| format!("Could not sort TOC file '{}'", toc))?;
        }

        Ok(())
    }

    fn move_pdf(&self) -> Result<()> {
        let tex_stem = self.tex_file.file_stem().unwrap();
        let out_pdf = self.out_dir.join(format!("{}.pdf", tex_stem));
        fs::rename(&out_pdf, self.pdf_path)
            .with_context(|| format!("Could not move to output file '{}'", self.pdf_path))
    }
}

trait TexDistro {
    fn render_pdf(&self, app: &App, job: TexRenderJob) -> Result<()>;
}

type DynTexDistro = Box<dyn TexDistro + Send + Sync + 'static>;

pub struct TexTools {
    distro: DynTexDistro,
}

impl TexTools {
    pub fn initialize(app: &App, settings_tex: Option<&TexConfig>) -> Result<()> {
        app.status("Locating", "TeX tools...");

        // First see if there's an explicit config...
        if let Some((config, source)) = Self::explicit_config(settings_tex)? {
            let distro = match config.clone() {
                TexConfig::TexLive { program } => TexLive::probe(app, program),
                TexConfig::Tectonic { program } => Tectonic::probe(app, program),
                TexConfig::None => Ok(TexNoop::new()),
            }
            .with_context(|| {
                format!(
                    "Error using TeX distribution '{}' configured from {}.",
                    config, source
                )
            })?;

            return Self::set(distro);
        }

        // No explicit config, try to probe automatically...

        if let Ok(texlive) = TexLive::probe(app, None) {
            return Self::set(texlive);
        }

        if let Ok(tectonic) = Tectonic::probe(app, None) {
            return Self::set(tectonic);
        }

        bail!("No TeX distribution found. FIXME: link doc.")
    }

    fn explicit_config(
        settings_tex: Option<&TexConfig>,
    ) -> Result<Option<(TexConfig, &'static str)>> {
        // Env var takes priority:
        if let Ok(cfg) = env::var("BARD_TEX") {
            return cfg
                .parse()
                .map(|cfg| Some((cfg, "the BARD_TEX environment variable")))
                .context("Invalid config in BARD_TEX environment variable");
        }

        // Then comes explicit setting from project settings:
        Ok(settings_tex
            .cloned()
            .map(|cfg| (cfg, "the 'tex' option in bard.toml")))
    }

    pub fn get() -> impl Deref<Target = Self> {
        struct Guard(MutexGuard<'static, Option<TexTools>>);

        impl Deref for Guard {
            type Target = TexTools;

            fn deref(&self) -> &Self::Target {
                self.0.as_ref().expect("TexTools not initialized")
            }
        }

        Guard(TEX_TOOLS.lock())
    }

    fn set(distro: Box<dyn TexDistro + Send + Sync + 'static>) -> Result<()> {
        let this = Self { distro };
        *TEX_TOOLS.lock() = Some(this);
        Ok(())
    }

    pub fn render_pdf(&self, app: &App, job: TexRenderJob) -> Result<()> {
        self.distro.render_pdf(app, job)
    }
}

struct TexLive {
    program: String,
}

impl TexLive {
    fn probe(app: &App, program: Option<String>) -> Result<DynTexDistro> {
        let program = program.unwrap_or_else(|| "xelatex".to_string());
        let first_line = test_program(&program, "-version")?;
        app.indent(first_line);
        Ok(Box::new(Self { program }))
    }
}

impl TexDistro for TexLive {
    fn render_pdf(&self, app: &App, job: TexRenderJob) -> Result<()> {
        let args = [
            "-interaction=nonstopmode",
            "-output-directory",
            job.out_dir.as_str(),
            job.tex_file.as_str(),
        ];

        run_program(app, &self.program, &args, job.cwd())?;
        job.sort_toc()?;
        run_program(app, &self.program, &args, job.cwd())?;
        job.move_pdf()?;

        Ok(())
    }
}

struct Tectonic {
    program: String,
}

impl Tectonic {
    fn probe(app: &App, program: Option<String>) -> Result<DynTexDistro> {
        let program = program.unwrap_or_else(|| "tectonic".to_string());
        let first_line = test_program(&program, "--version")?;
        app.indent(first_line);
        Ok(Box::new(Self { program }))
    }
}

impl TexDistro for Tectonic {
    fn render_pdf(&self, app: &App, job: TexRenderJob) -> Result<()> {
        let args = [
            "-k",
            "-r",
            "0",
            "-o",
            job.out_dir.as_str(),
            job.tex_file.as_str(),
        ];

        run_program(app, &self.program, &args, job.cwd())?;
        job.sort_toc()?;
        run_program(app, &self.program, &args, job.cwd())?;
        job.move_pdf()?;

        Ok(())
    }
}

struct TexNoop;

impl TexNoop {
    fn new() -> DynTexDistro {
        Box::new(Self)
    }
}

impl TexDistro for TexNoop {
    fn render_pdf(&self, _: &App, _: TexRenderJob) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn test_test_program() {
        assert_eq!(test_program("echo", "hello").unwrap(), "hello");
        test_program("xxx-surely-this-doesnt-exist", "").unwrap_err();
        test_program("false", "").unwrap_err();
        test_program("sleep", "9800").unwrap_err();
    }
}
