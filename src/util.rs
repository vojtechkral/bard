use std::fmt;
use std::ops;
use std::cmp;
use std::convert::TryFrom;
use std::process::ExitStatus;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;

use pulldown_cmark::{InlineStr, CowStr};
use serde::ser;

use crate::error::*;


// SmallStr

/// Like pulldown_cmark's CowStr but without
/// the referencing variant.
#[derive(Clone, Eq)]
pub enum SmallStr {
    Boxed(Box<str>),
    Inlined(InlineStr),
}

impl SmallStr {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl Default for SmallStr {
    fn default() -> Self {
        let s = InlineStr::try_from("").expect("Internal error: SmallStr constructor");
        Self::Inlined(s)
    }
}

impl ops::Deref for SmallStr {
    type Target = str;

    fn deref(&self) -> &str {
        match self {
            SmallStr::Boxed(b) => &*b,
            SmallStr::Inlined(s) => s.deref(),
        }
    }
}

impl AsRef<str> for SmallStr {
    fn as_ref(&self) -> &str {
        &*self
    }
}

impl cmp::PartialEq for SmallStr {
    fn eq(&self, other: &Self) -> bool {
        cmp::PartialEq::eq(self.as_str(), other.as_str())
    }
}

impl<'a> From<CowStr<'a>> for SmallStr {
    fn from(cowstr: CowStr<'a>) -> Self {
        match cowstr {
            CowStr::Boxed(b) => Self::Boxed(b),
            CowStr::Borrowed(s) => Self::from(s),
            CowStr::Inlined(s) => Self::Inlined(s),
        }
    }
}

impl<'a> From<&'a str> for SmallStr {
    fn from(s: &'a str) -> Self {
        if let Ok(inlined) = InlineStr::try_from(s) {
            Self::Inlined(inlined)
        } else {
            Self::Boxed(s.to_owned().into())
        }
    }
}

impl From<String> for SmallStr {
    fn from(s: String) -> Self {
        if let Ok(inlined) = InlineStr::try_from(s.as_str()) {
            Self::Inlined(inlined)
        } else {
            Self::Boxed(s.into())
        }
    }
}

impl fmt::Display for SmallStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SmallStr::Boxed(b) => fmt::Display::fmt(b, f),
            SmallStr::Inlined(s) => fmt::Display::fmt(s, f),
        }
    }
}

impl fmt::Debug for SmallStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SmallStr::Boxed(_) => write!(f, "Boxed(")?,
            SmallStr::Inlined(_) => write!(f, "Inlined(")?,
        }
        fmt::Debug::fmt(self.as_str(), f)?;
        write!(f, ")")
    }
}

impl ser::Serialize for SmallStr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            SmallStr::Boxed(b) => serializer.serialize_str(&*b),
            SmallStr::Inlined(s) => serializer.serialize_str(s.as_ref()),
        }
    }
}

// ExitStatus extension

pub trait ExitStatusExt {
    fn into_result(self) -> Result<()>;
}

impl ExitStatusExt for ExitStatus {
    fn into_result(self) -> Result<()> {
        if self.success() {
            return Ok(());
        }

        #[cfg(unix)]
        {
            if let Some(signal) = self.signal() {
                bail!("Process killed by signal: {}", signal);
            }
        }

        match self.code() {
            Some(code) => bail!("Process exited with code: {}", code),
            None => bail!("Process failed with unknown error"),
        }
    }
}
