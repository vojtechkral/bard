use std::io::{BufRead, Write};
use std::ops::Deref;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::time::Duration;
use std::{env, fs, io, iter, thread};

use parking_lot::{const_mutex, Mutex, MutexGuard};
use serde::de::Error as _;
use serde::Deserialize;

use crate::cli::{self, TerminalExt as _};
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
            bail!("Unexpected TeX distro type: `{}`, possible choices are: `texlive`, `tectonic`, or `none`", kind);
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

/// Run a command and get first line from stdout, if any
fn test_program(program: &str, arg1: &str) -> Result<Option<String>, ()> {
    fn unit<E>(_: E) {}

    let mut child = match Command::new(program)
        .arg(arg1)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.kind())
    {
        Ok(child) => child,
        Err(io::ErrorKind::NotFound) => return Ok(None),
        Err(_) => return Err(()),
    };

    // Crude way to wait for the subprocess with a timeout.
    for _ in 0..30 {
        if let Some(status) = child.try_wait().map_err(unit)? {
            status.into_result().map_err(unit)?;
            break;
        }

        thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();

    let stdout = child.stdout.take().map(io::BufReader::new).ok_or(())?;
    let first_line = stdout.lines().next().ok_or(())?.map_err(unit)?;
    Ok(Some(first_line))
}

fn run_program(program: &str, args: &[&str], cwd: &Path) -> Result<()> {
    let prog_name = Path::new(program).file_stem().unwrap();

    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Could not run program `{}`", program))?;

    let mut ps_lines =
        ProcessLines::new(child.stdout.take().unwrap(), child.stderr.take().unwrap());
    let stderr = io::stderr();
    let mut stderr = stderr.lock();

    let mut term = term::stderr().unwrap(); // TODO: App context

    // FIXME: This messes up test harness output https://github.com/rust-lang/rust/issues/90785
    // This should probably be solved by stdio through app-context

    eprintln!();
    while let Some(line) = ps_lines
        .read_line()
        .with_context(|| format!("Error reading output of program `{}`", program))?
    {
        term.rewind_line().unwrap();
        eprint!("{}: ", prog_name);
        stderr.write_all(&line).unwrap();
    }
    term.rewind_line().unwrap();

    let status = child
        .wait()
        .with_context(|| format!("Error running program `{}`", program))?;
    if !status.success() {
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
            // TODO: sort_lines warns on cli - is that ok?
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
    fn render_pdf(&self, job: TexRenderJob) -> Result<()>;
}

pub struct TexTools {
    distro: Box<dyn TexDistro + Send + Sync + 'static>,
}

impl TexTools {
    pub fn initialize() -> Result<()> {
        cli::status("Locating", "TeX tools...");

        if let Ok(tex_config) = env::var("BARD_TEX") {
            let tex_config: TexConfig = tex_config
                .parse()
                .context("Invalid config in BARD_TEX environment variable")?;

            // When tech tools are configured explicitly, we don't probe them...

            return match tex_config {
                TexConfig::TexLive { program } => Self::set(TexLive::new(
                    program.unwrap_or_else(|| "xelatex".to_string()),
                )),
                TexConfig::Tectonic { program } => Self::set(Tectonic::new(
                    program.unwrap_or_else(|| "tectonic".to_string()),
                )),
                TexConfig::None => Self::set(TexNoop),
            };
        }

        let program = "xelatex".to_string();
        let version = test_program(&program, "-version");
        if let Ok(Some(version)) = version {
            cli::indent(version);
            return Self::set(TexLive::new(program));
        }

        let program = "tectonic".to_string();
        let version = test_program(&program, "--version");
        if let Ok(Some(version)) = version {
            cli::indent(version);
            return Self::set(Tectonic::new(program));
        }

        bail!("No TeX distribution found. FIXME: link doc.")
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

    fn set<D>(distro: D) -> Result<()>
    where
        D: TexDistro + Send + Sync + 'static,
    {
        let this = Self {
            distro: Box::new(distro),
        };
        *TEX_TOOLS.lock() = Some(this);
        Ok(())
    }

    pub fn render_pdf(&self, job: TexRenderJob) -> Result<()> {
        self.distro.render_pdf(job)
    }
}

struct TexLive {
    program: String,
}

impl TexLive {
    fn new(program: String) -> Self {
        Self { program }
    }
}

impl TexDistro for TexLive {
    fn render_pdf(&self, job: TexRenderJob) -> Result<()> {
        let args = [
            "-interaction=nonstopmode",
            "-output-directory",
            job.out_dir.as_str(),
            job.tex_file.as_str(),
        ];

        run_program(&self.program, &args, job.cwd())?;
        job.sort_toc()?;
        run_program(&self.program, &args, job.cwd())?;
        job.move_pdf()?;

        Ok(())
    }
}

struct Tectonic {
    program: String,
}

impl Tectonic {
    fn new(program: String) -> Self {
        Self { program }
    }
}

impl TexDistro for Tectonic {
    fn render_pdf(&self, job: TexRenderJob) -> Result<()> {
        let args = [
            "-k",
            "-r",
            "0",
            "-o",
            job.out_dir.as_str(),
            job.tex_file.as_str(),
        ];

        run_program(&self.program, &args, job.cwd())?;
        job.sort_toc()?;
        run_program(&self.program, &args, job.cwd())?;
        job.move_pdf()?;

        Ok(())
    }
}

struct TexNoop;

impl TexDistro for TexNoop {
    fn render_pdf(&self, _job: TexRenderJob) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn test_test_program() {
        assert_eq!(test_program("echo", "hello").unwrap().unwrap(), "hello");
        assert!(test_program("xxx-surely-this-doesnt-exist", "")
            .unwrap()
            .is_none());
        test_program("false", "").unwrap_err();
        test_program("sleep", "9800").unwrap_err();
    }
}
