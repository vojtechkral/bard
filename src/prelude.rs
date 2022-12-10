//! Crate-wide definitions.

pub use std::path::{Path, PathBuf};

pub use anyhow::{anyhow, bail, Context as _, Error, Result};

pub use crate::util::{PathBufExt as _, PathExt as _};
