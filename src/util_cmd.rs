use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::str::FromStr;

use lexical_sort::{lexical_cmp, StringSort};
use regex::Regex;

use crate::error::*;

#[derive(Debug)]
struct SortableLine {
    line: String,
    key: String,
}

impl AsRef<str> for SortableLine {
    fn as_ref(&self) -> &str {
        self.key.as_str()
    }
}

fn line_read(
    mut lines: Vec<SortableLine>,
    line: io::Result<String>,
    regex: &Regex,
) -> Result<Vec<SortableLine>> {
    let line = line?;
    let caps = regex
        .captures(&line)
        .with_context(|| format!("No match for line {}: {}", lines.len(), line))?;
    let key = caps
        .get(1)
        .map(|m| m.as_str().to_owned())
        .with_context(|| {
            format!(
                "No capture group in regex: `{}`, the sort key has to be in a capture group",
                regex
            )
        })?;

    lines.push(SortableLine { line, key });

    Ok(lines)
}

pub fn sort_lines(path: &str, regex: &str) -> Result<()> {
    let regex = Regex::from_str(regex).with_context(|| format!("Invalid regex: `{}`", regex))?;

    let path = PathBuf::from(path);
    let file =
        File::open(&path).with_context(|| format!("Could not open file `{}`", path.display()))?;
    let reader = BufReader::new(file);
    let mut lines = reader
        .lines()
        .try_fold(Vec::new(), |lines, line| line_read(lines, line, &regex))
        .with_context(|| format!("Could not sort file `{}`", path.display()))?;

    lines.string_sort(lexical_cmp);

    let write_err = || format!("Could not write file `{}`", path.display());
    let mut file = File::create(&path)
        .map(BufWriter::new)
        .with_context(write_err)?;
    for line in &lines[..] {
        writeln!(&mut file, "{}", &line.line).with_context(write_err)?;
    }
    file.flush().with_context(write_err)?;

    Ok(())
}
