//! LSP framing, URIs, and request envelope.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

pub(crate) struct LspRequest {
    pub(crate) id: Option<Value>,
    pub(crate) method: String,
    pub(crate) params: Option<Value>,
}

pub(crate) fn write_message(out: &mut impl Write, value: &Value) -> io::Result<()> {
    let body = serde_json::to_string(value).unwrap_or_else(|_| "{}".into());
    write!(out, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    out.flush()
}

pub(crate) fn uri_to_path(uri: &str) -> PathBuf {
    if let Some(rest) = uri.strip_prefix("file://") {
        let path = rest.trim_start_matches('/');
        if rest.starts_with('/') && path.len() >= 2 && path.as_bytes()[1] == b':' {
            // Windows file:///C:/...
            PathBuf::from(path)
        } else if cfg!(windows) && path.contains(':') {
            PathBuf::from(path)
        } else {
            PathBuf::from(format!("/{path}"))
        }
    } else {
        PathBuf::from(uri)
    }
}

pub(crate) fn path_to_uri(path: &Path) -> String {
    let abs = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let s = abs.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        format!("file:///{s}")
    } else {
        format!("file://{s}")
    }
}

pub(crate) fn pos_from_line_col(file: &rynix_span::SourceFile, line: u32, col: u32) -> u32 {
    let line = line.max(1);
    let col = col.max(1);
    let mut local = 0u32;
    for l in 1..line {
        local += file.line_text(l).len() as u32 + 1;
    }
    local += col.saturating_sub(1).min(file.line_text(line).len() as u32);
    file.start_pos().saturating_add(local)
}

