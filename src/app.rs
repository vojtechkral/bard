use std::env;
use std::ffi::OsStr;
use std::fmt::Display;
use std::io::{self, Write};

use console::Color::{Cyan, Green, Red, Yellow};
use console::{Color, Style, Term};

use crate::prelude::*;
use crate::util::ProcessLines;

#[derive(clap::Parser, Clone, Default, Debug)]
pub struct StdioOpts {
    #[arg(short, long, help = "Be more verbose")]
    pub verbose: bool,
    #[arg(short, long, help = "Suppress output")]
    pub quiet: bool,
    #[arg(
        long,
        help = "Whether to use colored output (auto-detected by default)"
    )]
    pub color: Option<bool>,
}

impl StdioOpts {
    fn verbosity(&self) -> u32 {
        match (self.quiet, self.verbose) {
            (false, false) => 1,
            (false, true) => 2,
            (true, false) => 0,
            (true, true) => 1, // IDK but I think they cancel out back to default :)
        }
    }
}

#[derive(clap::Parser, Clone, Default, Debug)]
pub struct MakeOpts {
    #[arg(
        short = 'p',
        long,
        help = "Don't run post-processing steps, ie. TeX and scripts, if any"
    )]
    pub no_postprocess: bool,
    #[arg(
        short = 'k',
        long,
        help = "Keep intermediate output files in the output directory"
    )]
    pub keep: bool,
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

/// Runtime config and stdio output fns.
#[derive(Debug)]
pub struct App {
    post_process: bool,
    keep_interm: bool,

    // stdio stuff
    term: Term,
    /// There are three levels: `0` = quiet, `1` = normal, `2` = verbose.
    verbosity: u32,
    test_mode: bool,

    /// bard self exe binary path
    bard_exe: PathBuf,
    /// bard self name for status reporting
    self_name: &'static str,
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
        }
    }

    pub fn with_test_mode(post_process: bool, bard_exe: PathBuf) -> Self {
        console::set_colors_enabled_stderr(false);

        Self {
            post_process,
            keep_interm: true,
            term: Term::stderr(),
            verbosity: 2,
            test_mode: true,
            bard_exe,
            self_name: "bard",
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

    pub fn keep_interm(&self) -> bool {
        self.keep_interm
    }

    pub fn verbosity(&self) -> u32 {
        self.verbosity
    }

    pub fn use_color(&self) -> bool {
        console::colors_enabled_stderr()
    }

    pub fn bard_exe(&self) -> &Path {
        self.bard_exe.as_path()
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
