//! Open buffer state.

use std::path::PathBuf;

pub(crate) struct Document {
    pub(crate) path: PathBuf,
    pub(crate) text: String,
    pub(crate) version: i64,
}

