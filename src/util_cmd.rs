use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::str::FromStr;

use regex::Regex;

use crate::app::App;
use crate::cli;
use crate::prelude::*;
use crate::util::sort_lexical_by;

#[derive(clap::Parser)]
pub enum UtilCmd {
    #[command(name = "cp", about = "Copy a file")]
    Copy {
        #[arg(help = "Source path")]
        src: String,
        #[arg(help = "Destination path")]
        dest: String,
    },
    #[command(about = "Alphabetically sorts lines of a file in-place")]
    SortLines {
        #[arg(
            help = "Regular expression that extracts the sort key from each line via a capture group"
        )]
        regex: String,
        #[arg(help = "The file whose lines to sort, in-place")]
        file: String,
    },
}

impl UtilCmd {
    pub fn run(self, app: &App) -> Result<()> {
        use UtilCmd::*;

        match self {
            SortLines { regex, file } => sort_lines(&regex, &file).map(|_| ()),
            Copy { src, dest } => copy(&src, &dest),
        }
    }
}

#[derive(Debug)]
struct Line {
    line: String,
    key: Option<String>,
}

fn line_read(mut lines: Vec<Line>, line: io::Result<String>, regex: &Regex) -> Result<Vec<Line>> {
    let line = line?;
    let key = if let Some(caps) = regex.captures(&line) {
        caps.get(1)
            .map(|m| Some(m.as_str().to_owned()))
            .with_context(|| {
                format!(
                    "No capture group in regex: `{}`, the sort key has to be in a capture group",
                    regex
                )
            })?
    } else {
        None
    };

    lines.push(Line { line, key });

    Ok(lines)
}

pub fn sort_lines(regex: &str, path: impl Into<PathBuf>) -> Result<usize> {
    let regex = Regex::from_str(regex).with_context(|| format!("Invalid regex: `{}`", regex))?;

    let path = path.into();
    let file = File::open(&path).with_context(|| format!("Could not open file `{}`", path))?;
    let reader = BufReader::new(file);

    let mut lines = reader
        .lines()
        .try_fold(Vec::new(), |lines, line| line_read(lines, line, &regex))
        .with_context(|| format!("Could not sort file `{}`", path))?;

    let count = lines
        .as_mut_slice()
        .split_mut(|line| line.key.is_none())
        .map(|slice| {
            sort_lexical_by(slice, |line| line.key.as_ref().unwrap());
            slice.len()
        })
        .sum();

    let write_err = || format!("Could not write file `{}`", path);
    let mut file = File::create(&path)
        .map(BufWriter::new)
        .with_context(write_err)?;
    for line in &lines[..] {
        writeln!(&mut file, "{}", &line.line).with_context(write_err)?;
    }
    file.flush().with_context(write_err)?;

    if count == 0 {
        cli::warning("sort-lines: No lines matched the regex.");
    }

    Ok(count)
}

pub fn copy(src: &str, dest: &str) -> Result<()> {
    fs::copy(src, dest)?;
    Ok(())
}
