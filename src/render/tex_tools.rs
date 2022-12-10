use std::convert::{TryFrom, TryInto};
use std::ffi::{OsStr, OsString};
use std::io::{BufRead, Write};
use std::ops::Deref;
use std::process::{Command, Stdio};
use std::time::Duration;
use std::{env, fmt, fs, io, iter, thread};

use parking_lot::{const_mutex, Mutex, MutexGuard};
use serde::de::Error as _;
use serde::Deserialize;
use strum::{Display, EnumString, EnumVariantNames, VariantNames as _};

use crate::app::App;
use crate::prelude::*;
use crate::util::{ExitStatusExt, ProcessLines, TempPath};
use crate::util_cmd;

static TEX_TOOLS: Mutex<Option<TexTools>> = const_mutex(None);

#[derive(EnumString, EnumVariantNames, Display, Clone, Copy, PartialEq, Eq, Debug)]
#[strum(ascii_case_insensitive, serialize_all = "lowercase")]
pub enum TexDistro {
    TexLive,
    Tectonic,
    None,
}

impl TexDistro {
    fn default_program(&self) -> Option<OsString> {
        match self {
            Self::TexLive => Some("xelatex".to_string().into()),
            Self::Tectonic => Some("tectonic".to_string().into()),
            Self::None => None,
        }
    }

    fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl<'de> Deserialize<'de> for TexDistro {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let input: &'de str = Deserialize::deserialize(deserializer)?;
        input.parse().map_err(D::Error::custom)
    }
}

#[derive(Clone, Debug)]
pub struct TexConfig {
    distro: TexDistro,
    program: Option<OsString>,
}

impl TexConfig {
    fn try_from_env() -> Result<Option<Self>> {
        env::var_os("BARD_TEX")
            .map(|var| Self::try_from(var.as_ref()))
            .transpose()
    }

    fn with_distro(distro: TexDistro) -> Self {
        Self {
            distro,
            program: None,
        }
    }

    fn probe(&mut self, app: &App) -> Result<()> {
        if self.distro.is_none() {
            return Ok(());
        }

        if self.program.is_none() {
            self.program = self.distro.default_program();
        }

        let version = match self.distro {
            TexDistro::TexLive => test_program(self.program.as_ref().unwrap(), "-version")?,
            TexDistro::Tectonic => test_program(self.program.as_ref().unwrap(), "--version")?,
            TexDistro::None => unreachable!(),
        };

        app.indent(version);
        Ok(())
    }

    fn render_args<'j, 's: 'j>(&'s self, job: &'j TexRenderJob) -> Vec<&'j OsStr> {
        let mut args = match self.distro {
            TexDistro::TexLive => vec![
                "-interaction=nonstopmode".as_ref(),
                "-output-directory".as_ref(),
                job.out_dir.as_os_str(),
            ],
            TexDistro::Tectonic => vec![
                "-k".as_ref(),
                "-r".as_ref(),
                "0".as_ref(),
                "-o".as_ref(),
                job.out_dir.as_os_str(),
            ],
            TexDistro::None => unreachable!(),
        };

        args.extend(["--".as_ref(), job.tex_file.as_os_str()]);
        args
    }
}

#[cfg(unix)]
impl<'a> TryFrom<&'a OsStr> for TexConfig {
    type Error = Error;

    fn try_from(input: &'a OsStr) -> Result<Self, Self::Error> {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let input = input.as_bytes();
        let mut split = input.splitn(2, |&c| c == b':');
        let distro = OsStr::from_bytes(split.next().unwrap()).to_string_lossy();
        let program = split.next().map(|p| OsString::from_vec(p.to_owned()));
        let distro: TexDistro = distro.parse().map_err(|_| {
            anyhow!(
                "Unexpected TeX distro type: '{}', possible choices are: {:?}.",
                distro,
                TexDistro::VARIANTS,
            )
        })?;

        Ok(Self { distro, program })
    }
}
#[cfg(windows)]
impl<'a> TryFrom<&'a OsStr> for TexConfig {
    type Error = Error;

    fn try_from(input: &'a OsStr) -> Result<Self, Self::Error> {
        use std::os::windows::ffi::{OsStrExt, OsStringExt};

        const COLON: u16 = u16::from_le_bytes([b':', 0]);

        let input: Vec<_> = input.encode_wide().collect();
        let mut split = input.splitn(2, |&c| c == COLON);
        let distro = OsString::from_wide(split.next().unwrap());
        let distro = distro.to_string_lossy();
        let program = split.next().map(|p| OsString::from_wide(p));
        let distro: TexDistro = distro.parse().map_err(|_| {
            anyhow!(
                "Unexpected TeX distro type: '{}', possible choices are: {:?}.",
                distro,
                TexDistro::VARIANTS,
            )
        })?;

        Ok(Self { distro, program })
    }
}

impl<'de> Deserialize<'de> for TexConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let input: &'de str = Deserialize::deserialize(deserializer)?;
        OsStr::new(input).try_into().map_err(D::Error::custom)
    }
}

impl fmt::Display for TexConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.distro)?;

        if let Some(program) = self.program.as_ref() {
            write!(f, ":{}", program.to_string_lossy())?;
        }

        Ok(())
    }
}

/// Run a command and get first line from stdout, if any
fn test_program(program: impl AsRef<OsStr>, arg1: &str) -> Result<String> {
    let program = program.as_ref();
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
        .ok_or_else(|| anyhow!("No output from program {:?}", program))??;
    if first_line.is_empty() || first_line.chars().all(|c| c.is_ascii_whitespace()) {
        bail!("No output from program {:?}", program);
    }
    Ok(first_line)
}

fn run_program(app: &App, program: impl AsRef<OsStr>, args: &[&OsStr], cwd: &Path) -> Result<()> {
    let program = program.as_ref();
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Could not run program {:?}", program))?;

    let mut ps_lines =
        ProcessLines::new(child.stdout.take().unwrap(), child.stderr.take().unwrap());

    app.subprocess_output(&mut ps_lines, program)?;

    let status = child
        .wait()
        .with_context(|| format!("Error running program {:?}", program))?;

    if !status.success() && app.verbosity() == 1 {
        let cmdline =
            iter::once(&program)
                .chain(args.iter())
                .fold(String::new(), |mut cmdline, arg| {
                    cmdline.push_str(&arg.to_string_lossy());
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
    pub tex_file: TempPath,
    out_dir: TempPath,
    pdf_path: &'a Path,
    toc_sort_key: Option<&'a str>,
}

impl<'a> TexRenderJob<'a> {
    pub fn new(pdf_path: &'a Path, keep: bool, toc_sort_key: Option<&'a str>) -> Result<Self> {
        Ok(Self {
            tex_file: TempPath::new_file(pdf_path.with_extension("tex"), !keep),
            out_dir: TempPath::make_temp_dir(pdf_path, !keep)?,
            pdf_path,
            toc_sort_key,
        })
    }
}

impl<'a> TexRenderJob<'a> {
    fn cwd(&self) -> &'a Path {
        self.pdf_path.parent().unwrap()
    }

    fn sort_toc(&self) -> Result<()> {
        let key = match self.toc_sort_key {
            Some(key) => key,
            None => return Ok(()),
        };

        let tex_stem = self.tex_file.file_stem().unwrap();
        let toc = self.out_dir.join_stem(tex_stem, ".toc");

        if toc.exists() {
            util_cmd::sort_lines(key, &toc)
                .with_context(|| format!("Could not sort TOC file {:?}", toc))?;
        }

        Ok(())
    }

    fn move_pdf(&self) -> Result<()> {
        let tex_stem = self.tex_file.file_stem().unwrap();
        let out_pdf = self.out_dir.join_stem(tex_stem, ".pdf");
        fs::rename(&out_pdf, self.pdf_path)
            .with_context(|| format!("Could not move to output file {:?}", self.pdf_path))
    }
}

pub struct TexTools {
    config: TexConfig,
}

impl TexTools {
    pub fn initialize(app: &App, from_settings: Option<&TexConfig>) -> Result<()> {
        app.status("Locating", "TeX tools...");

        // 1. Priority: BARD_TEX env var
        if let Some(mut config) = TexConfig::try_from_env()? {
            config.probe(app).with_context(|| {
                format!(
                    "Error using TeX distribution '{}' configured from the BARD_TEX environment variable.", config)})?;
            return Self::set(config);
        }

        // 2. Config from bard.toml
        if let Some(mut config) = from_settings.cloned() {
            config.probe(app).with_context(|| {
                format!(
                    "Error using TeX distribution '{}' configured from the bard.toml project file.",
                    config
                )
            })?;
            return Self::set(config);
        }

        // 3. No explicit config, try to probe automatically...

        for kind in [TexDistro::TexLive, TexDistro::Tectonic] {
            let mut config = TexConfig::with_distro(kind);
            if config.probe(app).is_ok() {
                return Self::set(config);
            }
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

    fn set(config: TexConfig) -> Result<()> {
        let this = Self { config };
        *TEX_TOOLS.lock() = Some(this);
        Ok(())
    }

    pub fn render_pdf(&self, app: &App, mut job: TexRenderJob) -> Result<()> {
        if self.config.distro.is_none() {
            // TODO: test this:
            job.tex_file.set_remove(false);
            return Ok(());
        }

        app.status("Running", "TeX...");

        let args = self.config.render_args(&job);
        let program = self.config.program.as_ref().unwrap();

        if app.verbosity() >= 2 {
            app.status("Command", format!("{:?} {:?}", program, args));
        }

        run_program(app, program, &args, job.cwd())?;
        job.sort_toc()?;
        run_program(app, program, &args, job.cwd())?;
        job.move_pdf()?;

        Ok(())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn tex_config_parse(input: impl AsRef<OsStr>) -> Result<TexConfig> {
        input.as_ref().try_into()
    }

    #[test]
    fn tex_config_parsing() {
        let config = tex_config_parse("texlive").unwrap();
        assert_eq!(config.distro, TexDistro::TexLive);
        assert_eq!(config.program, None);

        let config = tex_config_parse("tectonic").unwrap();
        assert_eq!(config.distro, TexDistro::Tectonic);
        assert_eq!(config.program, None);

        let config = tex_config_parse("texlive:foo:bar").unwrap();
        assert_eq!(config.distro, TexDistro::TexLive);
        assert_eq!(config.program, Some("foo:bar".to_string().into()));

        let config = tex_config_parse("tectonic:foo:bar").unwrap();
        assert_eq!(config.distro, TexDistro::Tectonic);
        assert_eq!(config.program, Some("foo:bar".to_string().into()));

        tex_config_parse("xxx").unwrap_err();
    }

    #[test]
    fn test_test_program() {
        assert_eq!(test_program("echo", "hello").unwrap(), "hello");
        test_program("xxx-surely-this-doesnt-exist", "").unwrap_err();
        test_program("false", "").unwrap_err();
        test_program("sleep", "9800").unwrap_err();
    }
}
