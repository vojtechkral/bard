use std::cell::RefCell;
use std::ffi::OsStr;
use std::fmt::Display;
use std::io::{self, Write};
use std::{env, fmt};

use term::color::{self, Color};
use term::{Attr, StderrTerminal};

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
pub struct App {
    post_process: bool,
    keep_interm: bool,

    // stdio stuff
    term: Option<RefCell<Box<StderrTerminal>>>,
    /// There are three levels: `0` = quiet, `1` = normal, `2` = verbose.
    verbosity: u32,
    use_color: bool,
    test_mode: bool,

    /// bard self exe binary path
    bard_exe: PathBuf,
}

impl App {
    pub fn new(opts: &MakeOpts) -> Self {
        let is_tty = atty::is(atty::Stream::Stderr);
        let term = is_tty.then(term::stderr).flatten();
        let use_color = opts.stdio.color.unwrap_or(true)
            && is_tty
            && term.as_ref().map_or(false, |t| t.supports_color()); // also checks for reset support

        Self {
            post_process: !opts.no_postprocess,
            keep_interm: opts.keep,
            term: term.map(RefCell::new),
            verbosity: opts.stdio.verbosity(),
            use_color,
            test_mode: false,
            bard_exe: env::current_exe().expect("Could not get path to bard self binary"),
        }
    }

    pub fn with_test_mode(post_process: bool, bard_exe: PathBuf) -> Self {
        Self {
            post_process,
            keep_interm: true,
            term: None,
            verbosity: 2,
            use_color: false,
            test_mode: true,
            bard_exe,
        }
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

    pub fn bard_exe(&self) -> &Path {
        self.bard_exe.as_path()
    }

    // stdio helpers

    fn with_term<F, R>(&self, f: F)
    where
        F: FnOnce(&mut Box<StderrTerminal>) -> R,
    {
        let _ = self.term.as_ref().map(|cell| f(&mut cell.borrow_mut()));
    }

    fn color_print(&self, color: Color, text: impl Display) {
        if self.use_color {
            self.with_term(|term| {
                let _ = term.fg(color);
                let _ = term.attr(Attr::Bold);
            });
        }

        eprint!("{}", text);

        if self.use_color {
            self.with_term(|term| term.reset());
        }
    }

    fn indent_line(line: &str) {
        eprintln!("             {}", line);
    }

    fn status_inner(&self, kind: impl Display, color: Color, status: impl Display) {
        if self.verbosity == 0 {
            return;
        }

        self.color_print(color, format!("{:>12}", kind));
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
        self.status_inner(verb, color::BRIGHT_CYAN, status);
    }

    pub fn success(&self, verb: impl Display) {
        self.status_inner(verb, color::BRIGHT_GREEN, "");
    }

    pub fn warning(&self, msg: impl Display) {
        self.status_inner("Warning", color::BRIGHT_YELLOW, msg);
    }

    pub fn error(&self, error: Error) {
        if self.verbosity == 0 {
            return;
        }

        self.status_inner("bard error", color::BRIGHT_RED, &error);

        let mut source = error.source();
        while let Some(err) = source {
            let err_str = format!("{}", err);
            for line in err_str.lines() {
                self.color_print(color::BRIGHT_RED, "  | ");
                eprintln!("{}", line);
            }

            source = err.source();
        }
    }

    pub fn rewind_line(&self) {
        if self.verbosity == 0 {
            return;
        }

        self.with_term(|term| {
            term.cursor_up()?;
            term.delete_line()
        });
    }

    pub fn subprocess_output(
        &self,
        ps_lines: &mut ProcessLines,
        program: impl AsRef<OsStr>,
    ) -> Result<()> {
        let program = program.as_ref();
        if self.verbosity == 0 {
            return Ok(());
        }

        let prog_name = Path::new(program).file_stem().unwrap();
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
                self.rewind_line();
                eprint!("{}: ", prog_name.to_string_lossy());
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
            self.rewind_line();
        }

        Ok(())
    }
}

impl fmt::Debug for App {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let term = self.term.is_some();
        f.debug_struct("App")
            .field("post_process", &self.post_process)
            .field("keep_interm", &self.keep_interm)
            .field("term", &term)
            .field("verbosity", &self.verbosity)
            .field("use_color", &self.use_color)
            .field("test_mode", &self.test_mode)
            .field("bard_exe", &self.bard_exe)
            .finish()
    }
}
