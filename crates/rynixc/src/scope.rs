//! Agent permission scope (`rynixc scope` + gate for `patch --write`).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct ScopeConfig {
    pub patch_write: bool,
    pub source: String,
}

impl Default for ScopeConfig {
    fn default() -> Self {
        Self {
            patch_write: false,
            source: "default(deny)".into(),
        }
    }
}

impl ScopeConfig {
    pub fn to_json(&self) -> Value {
        json!({
            "schema": "rynix.scope.v1",
            "permissions": {
                "patch_write": self.patch_write,
            },
            "source": self.source,
            "note": "Deny-by-default: patch --write requires patch_write=true or --force-write.",
        })
    }
}

pub fn load_scope(config: Option<&Path>) -> ScopeConfig {
    let candidates: Vec<PathBuf> = if let Some(p) = config {
        vec![p.to_path_buf()]
    } else {
        vec![
            PathBuf::from("rynix.scope.toml"),
            PathBuf::from(".rynix/scope.toml"),
        ]
    };
    for path in candidates {
        if path.is_file() {
            if let Ok(text) = fs::read_to_string(&path) {
                return parse_scope_toml(&text, &path.display().to_string());
            }
        }
    }
    ScopeConfig::default()
}

fn parse_scope_toml(content: &str, source: &str) -> ScopeConfig {
    let mut cfg = ScopeConfig {
        patch_write: false,
        source: source.into(),
    };
    let mut in_permissions = false;
    for line in content.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_permissions = line.trim_matches(['[', ']']) == "permissions";
            continue;
        }
        if !in_permissions {
            continue;
        }
        if let Some(rest) = line.strip_prefix("patch_write") {
            let rest = rest.trim().trim_start_matches('=').trim();
            cfg.patch_write = rest == "true" || rest == "1";
        }
    }
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_denies_write() {
        assert!(!ScopeConfig::default().patch_write);
    }

    #[test]
    fn parses_allow() {
        let c = parse_scope_toml("[permissions]\npatch_write = true\n", "t.toml");
        assert!(c.patch_write);
    }
}
