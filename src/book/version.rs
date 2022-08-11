use std::fmt;

use camino::Utf8Path as Path;
use semver::Version;

use crate::cli;

pub struct AstVersion {
    pub ver: Version,
    pub description: &'static str,
}

impl AstVersion {
    pub const fn new(ver_maj: u32, ver_min: u32, description: &'static str) -> Self {
        Self {
            ver: Version::new(ver_maj as u64, ver_min as u64, 0),
            description,
        }
    }
}

impl fmt::Display for AstVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}.", self.ver, self.description)
    }
}

pub static AST_VERSION_LOG: &[AstVersion] = &[
    AstVersion::new(1, 0, "Initial version"),
    AstVersion::new(
        1,
        1,
        "New style, added support for HTML snippets, TTF font files, and baseline chords",
    ),
];

pub fn current() -> &'static Version {
    AST_VERSION_LOG
        .iter()
        .last()
        .as_ref()
        .map(|v| &v.ver)
        .unwrap()
}

fn log_changes(since: &Version) {
    cli::status("", format!("Changes since version {}:", since));

    for ver in AST_VERSION_LOG.iter().skip_while(|v| &v.ver <= since) {
        cli::status("", ver);
    }
}

pub fn compat_check(tpl_path: &Path, tpl_version: &Version) {
    let current = current();
    if current < tpl_version {
        // Template's AST is newer than this bard's AST
        cli::warning(format!(
            "The version of template `{}` is {}, which is newer than what this bard uses ({}).
Maybe this project was created with a newer bard version.
This may cause errors while rendering...",
            tpl_path, tpl_version, current,
        ));
    } else if current.major > tpl_version.major {
        // Template's AST major version is older than this bard's AST, incompatibly
        cli::warning(
            format!("The version of template `{}` is {}, which is from an older generation than what this bard uses ({}).
This may cause errors while rendering. It may be needed to convert the template to the newer format.",
            tpl_path, tpl_version, current,
        ));
        log_changes(tpl_version);
    } else if current > tpl_version {
        // Template's AST version is older than this bard's AST, compatibly
        cli::status(
            "Notice",
            format!(
                "The version of template `{}` is {}. This version of bard supports {}.
This is not a problem, but the new version may offer improvements.",
                tpl_path, tpl_version, current,
            ),
        );
        log_changes(tpl_version);
    }
}
