use std::env;
use std::ffi::OsStr;
use std::fmt::Display;
use std::io::{self, Write};

use console::Color::{Cyan, Green, Red, Yellow};
use console::{Color, Style, Term};
use parking_lot::Mutex;

use crate::parser::Diagnostic;
use crate::prelude::*;
use crate::util::{ImgCache, ProcessLines};

#[derive(clap::Parser, Clone, Default)]
pub struct StdioOpts {
    /// Be more verbose
    #[arg(short, long)]
    pub verbose: bool,
    /// Suppress output
    #[arg(short, long)]
    pub quiet: bool,
    /// Whether to use colored output (auto-detected by default)
    #[arg(long)]
    pub color: Option<bool>,
}

impl StdioOpts {
    fn verbosity(&self) -> u8 {
        match (self.quiet, self.verbose) {
            (false, false) => 1,
            (false, true) => 2,
            (true, false) => 0,
            (true, true) => 1, // IDK but I think they cancel out back to default :)
        }
    }
}

#[derive(clap::Parser, Clone, Default)]
pub struct MakeOpts {
    /// Don't run post-processing steps, ie. TeX and scripts, if any
    #[arg(short = 'p', long)]
    pub no_postprocess: bool,
    /// Keep the TeX file when generating PDF. Use twice to keep TeX build directory as well.
    #[arg(short = 'k', long, action = clap::ArgAction::Count)]
    pub keep: u8,
    #[clap(flatten)]
    pub stdio: StdioOpts,
}

impl From<StdioOpts> for MakeOpts {
    fn from(stdio: StdioOpts) -> Self {
        Self {
            stdio,
            ..Default::default()
        }
    }
}

pub mod verbosity {
    pub const QUIET: u8 = 0;
    pub const NORMAL: u8 = 1;
    pub const VERBOSE: u8 = 2;
}

pub mod keeplevel {
    pub const NONE: u8 = 0;
    pub const TEX_ONLY: u8 = 1;
    pub const ALL: u8 = 2;
}

pub type ParserDiags = Mutex<Vec<Diagnostic>>;

/// Runtime config and stdio output fns.
#[derive(Debug)]
pub struct App {
    post_process: bool,
    /// See `keeplevel` for levels.
    keep_interm: u8,

    // stdio stuff
    term: Term,
    /// See `verbosity` for levels.
    verbosity: u8,
    test_mode: bool,

    /// bard self exe binary path
    bard_exe: PathBuf,
    /// bard self name for status reporting
    self_name: &'static str,

    /// Image dimensions cache, for `HbRender`.
    img_cache: ImgCache,

    /// Parser diagnostic messages, these are only collected in `test_mode`.
    parser_diags: ParserDiags,
}

impl App {
    pub fn new(opts: &MakeOpts) -> Self {
        Self {
            post_process: !opts.no_postprocess,
            keep_interm: opts.keep,
            term: Term::stderr(),
            verbosity: opts.stdio.verbosity(),
            test_mode: false,
            bard_exe: env::current_exe().expect("Could not get path to bard self binary"),
            self_name: "bard",
            img_cache: ImgCache::new(),
            parser_diags: Mutex::new(vec![]),
        }
    }

    pub fn with_test_mode(post_process: bool, bard_exe: PathBuf) -> Self {
        console::set_colors_enabled_stderr(false);

        Self {
            post_process,
            keep_interm: keeplevel::ALL,
            term: Term::stderr(),
            verbosity: 2,
            test_mode: true,
            bard_exe,
            self_name: "bard",
            img_cache: ImgCache::new(),
            parser_diags: Mutex::new(vec![]),
        }
    }

    #[cfg(feature = "tectonic")]
    pub fn new_as_tectonic() -> Self {
        let mut this = Self::new(&MakeOpts::default());
        this.verbosity = 1;
        this.self_name = "tectonic";
        this
    }

    pub fn post_process(&self) -> bool {
        self.post_process
    }

    pub fn keep_interm(&self) -> u8 {
        self.keep_interm
    }

    pub fn verbosity(&self) -> u8 {
        self.verbosity
    }

    pub fn use_color(&self) -> bool {
        console::colors_enabled_stderr()
    }

    pub fn bard_exe(&self) -> &Path {
        self.bard_exe.as_path()
    }

    pub fn img_cache(&self) -> &ImgCache {
        &self.img_cache
    }

    pub fn parser_diags(&self) -> &ParserDiags {
        &self.parser_diags
    }

    // stdio helpers

    fn color(&self, color: Color) -> Style {
        self.term.style().fg(color).bright().bold()
    }

    fn indent_line(line: &str) {
        eprintln!("             {}", line);
    }

    fn status_inner(&self, kind: impl Display, style: &Style, status: impl Display) {
        if self.verbosity == 0 {
            return;
        }

        eprint!("{:>12}", style.apply_to(kind));
        let status = format!("{}", status);
        let mut lines = status.lines();
        let first = lines.next().unwrap_or("");
        eprintln!(" {}", first);
        lines.for_each(Self::indent_line);
    }

    pub fn indent(&self, status: impl Display) {
        if self.verbosity == 0 {
            return;
        }

        let status = format!("{}", status);
        status.lines().for_each(Self::indent_line);
    }

    pub fn status(&self, verb: &str, status: impl Display) {
        self.status_inner(verb, &self.color(Cyan), status);
    }

    /// Like `status()`, but no newline
    pub fn status_bare(&self, verb: &str, status: impl Display) {
        if self.verbosity == 0 {
            return;
        }

        eprint!("{:>12} {}", self.color(Cyan).apply_to(verb), status);
    }

    pub fn success(&self, verb: impl Display) {
        self.status_inner(verb, &self.color(Green), "");
    }

    pub fn warning(&self, msg: impl Display) {
        self.status_inner("Warning", &self.color(Yellow), msg);
    }

    pub fn error(&self, error: Error) {
        if self.verbosity == 0 {
            return;
        }

        let color = self.color(Red);
        self.status_inner(format!("{} error", self.self_name), &color, &error);

        let mut source = error.source();
        while let Some(err) = source {
            let err_str = format!("{}", err);
            for line in err_str.lines() {
                eprintln!("  {} {}", color.apply_to("|"), line);
            }

            source = err.source();
        }
    }

    pub fn error_generic(&self, msg: impl Display) {
        self.status_inner("Error", &self.color(Red), msg);
    }

    pub fn parser_diag(&self, diag: Diagnostic) {
        if self.test_mode {
            self.parser_diags.lock().push(diag.clone());
        }

        if diag.is_error() {
            self.error_generic(diag);
        } else {
            self.warning(diag);
        }
    }

    pub fn subprocess_output(
        &self,
        ps_lines: &mut ProcessLines,
        program: impl AsRef<OsStr>,
        status: &str,
    ) -> Result<()> {
        let program = program.as_ref();
        if self.verbosity == 0 {
            return Ok(());
        }

        let stderr = io::stderr();
        let mut stderr = stderr.lock();

        if self.verbosity == 1 {
            eprintln!()
        }
        while let Some(line) = ps_lines
            .read_line()
            .with_context(|| format!("Error reading output of program {:?}", program))?
        {
            if self.verbosity == 1 {
                let _ = self.term.clear_last_lines(1);
                eprint!("{}: ", status);
            }

            if !self.test_mode {
                stderr.write_all(&line).unwrap();
            } else {
                // Workaround for https://github.com/rust-lang/rust/issues/90785
                let mut line = String::from_utf8_lossy(&line).to_string();
                line.retain(|c| !c.is_control());
                eprintln!("{}", line);
            }
        }
        if self.verbosity == 1 {
            let _ = self.term.clear_last_lines(1);
        }

        Ok(())
    }
}
