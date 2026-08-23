//! `rynix.toml` manifest + path dependency resolution (Phase D3).
//!
//! No registry: only `{ path = "..." }` deps. Missing paths fail resolve/build.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

#[derive(Debug, Clone, Default)]
pub struct Manifest {
    pub dir: PathBuf,
    pub name: String,
    pub version: String,
    pub entry: Option<PathBuf>,
    pub runtime: Option<String>,
    pub optimize: Option<bool>,
    pub deps: Vec<PathDep>,
}

#[derive(Debug, Clone)]
pub struct PathDep {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ResolvedDep {
    pub name: String,
    pub path: PathBuf,
    pub entry: Option<PathBuf>,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct DepsReport {
    pub root_manifest: PathBuf,
    pub package: String,
    pub deps: Vec<ResolvedDep>,
}

impl DepsReport {
    pub fn all_ok(&self) -> bool {
        self.deps.iter().all(|d| d.ok)
    }

    pub fn to_json(&self) -> Value {
        json!({
            "schema": "rynix.deps.v1",
            "manifest": self.root_manifest.display().to_string(),
            "package": self.package,
            "status": if self.all_ok() { "ok" } else { "error" },
            "dependencies": self.deps.iter().map(|d| json!({
                "name": d.name,
                "kind": "path",
                "path": d.path.display().to_string(),
                "entry": d.entry.as_ref().map(|p| p.display().to_string()),
                "ok": d.ok,
                "detail": d.detail,
            })).collect::<Vec<_>>(),
            "note": "Local path deps only — no package registry (SURPASS D3).",
        })
    }
}

/// Walk parents from `start` for `rynix.toml`.
pub fn find_manifest(start: &Path) -> Option<PathBuf> {
    let mut cur = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        let cand = cur.join("rynix.toml");
        if cand.is_file() {
            return Some(cand);
        }
        if !cur.pop() {
            return None;
        }
    }
}

pub fn load_manifest(path: &Path) -> Result<Manifest, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let dir = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let mut m = parse_manifest_toml(&content);
    m.dir = dir;
    if m.name.is_empty() {
        m.name = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unnamed".into());
    }
    Ok(m)
}

pub fn resolve_deps(manifest: &Manifest) -> DepsReport {
    let mut out = Vec::new();
    for dep in &manifest.deps {
        let abs = if dep.path.is_absolute() {
            dep.path.clone()
        } else {
            manifest.dir.join(&dep.path)
        };
        let abs = fs::canonicalize(&abs).unwrap_or(abs);
        let dep_toml = abs.join("rynix.toml");
        if !abs.is_dir() {
            out.push(ResolvedDep {
                name: dep.name.clone(),
                path: abs,
                entry: None,
                ok: false,
                detail: "path is not a directory".into(),
            });
            continue;
        }
        if !dep_toml.is_file() {
            out.push(ResolvedDep {
                name: dep.name.clone(),
                path: abs,
                entry: None,
                ok: false,
                detail: "missing rynix.toml in dependency path".into(),
            });
            continue;
        }
        match load_manifest(&dep_toml) {
            Ok(dm) => {
                let entry = dm.entry.as_ref().map(|e| {
                    let p = dm.dir.join(e);
                    fs::canonicalize(&p).unwrap_or(p)
                });
                let (ok, detail) = match &entry {
                    Some(p) if p.is_file() => (true, format!("entry {}", p.display())),
                    Some(p) => (false, format!("entry missing: {}", p.display())),
                    None => (
                        true,
                        "manifest ok (no entry — check-only dep)".into(),
                    ),
                };
                out.push(ResolvedDep {
                    name: dep.name.clone(),
                    path: abs,
                    entry,
                    ok,
                    detail,
                });
            }
            Err(e) => out.push(ResolvedDep {
                name: dep.name.clone(),
                path: abs,
                entry: None,
                ok: false,
                detail: e,
            }),
        }
    }
    DepsReport {
        root_manifest: manifest.dir.join("rynix.toml"),
        package: manifest.name.clone(),
        deps: out,
    }
}

/// Resolve deps for a source file's project; `Ok(None)` if no manifest.
pub fn resolve_for_source(source: &Path) -> Result<Option<DepsReport>, String> {
    let Some(m_path) = find_manifest(source) else {
        return Ok(None);
    };
    let m = load_manifest(&m_path)?;
    Ok(Some(resolve_deps(&m)))
}

fn parse_manifest_toml(content: &str) -> Manifest {
    let mut m = Manifest::default();
    let mut section = "";
    let mut pending_dep: Option<String> = None;

    for raw in content.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            pending_dep = None;
            section = line;
            continue;
        }
        if section == "[dependencies]" {
            // foo = { path = "..." }
            if let Some(eq) = line.find('=') {
                let name = line[..eq].trim().to_string();
                let rhs = line[eq + 1..].trim();
                if let Some(path) = extract_path_from_inline_table(rhs) {
                    m.deps.push(PathDep {
                        name,
                        path: PathBuf::from(path),
                    });
                } else if rhs == "{" || rhs.starts_with('{') {
                    pending_dep = Some(name);
                }
            } else if let Some(name) = pending_dep.clone() {
                if let Some(path) = parse_string_assign(line, "path") {
                    m.deps.push(PathDep {
                        name,
                        path: PathBuf::from(path),
                    });
                    pending_dep = None;
                }
            }
            continue;
        }
        pending_dep = None;
        match section {
            "[package]" => {
                if let Some(v) = parse_string_assign(line, "name") {
                    m.name = v;
                } else if let Some(v) = parse_string_assign(line, "version") {
                    m.version = v;
                } else if let Some(v) = parse_string_assign(line, "entry") {
                    m.entry = Some(PathBuf::from(v));
                }
            }
            "[build]" => {
                if let Some(v) = parse_string_assign(line, "runtime") {
                    m.runtime = Some(v);
                } else if let Some(v) = parse_bool_assign(line, "optimize") {
                    m.optimize = Some(v);
                }
            }
            _ => {}
        }
    }
    m
}

fn extract_path_from_inline_table(rhs: &str) -> Option<String> {
    let rhs = rhs.trim();
    if !rhs.starts_with('{') {
        return None;
    }
    // { path = "foo" } or { path = 'foo' }
    for part in rhs.trim_matches(|c| c == '{' || c == '}').split(',') {
        let part = part.trim();
        if let Some(v) = parse_string_assign(part, "path") {
            return Some(v);
        }
    }
    None
}

fn parse_string_assign(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}");
    let line = line.trim();
    if !line.starts_with(&prefix) {
        return None;
    }
    let rest = line[prefix.len()..].trim_start();
    if !rest.starts_with('=') {
        return None;
    }
    let val = rest[1..].trim();
    let val = val.trim_matches(|c| c == '"' || c == '\'');
    if val.is_empty() {
        None
    } else {
        Some(val.to_string())
    }
}

fn parse_bool_assign(line: &str, key: &str) -> Option<bool> {
    let prefix = format!("{key}");
    let line = line.trim();
    if !line.starts_with(&prefix) {
        return None;
    }
    let rest = line[prefix.len()..].trim_start();
    if !rest.starts_with('=') {
        return None;
    }
    let val = rest[1..].trim().trim_matches(|c| c == '"' || c == '\'');
    match val {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_path_dep_inline() {
        let m = parse_manifest_toml(
            r#"
[package]
name = "app"
entry = "main.ryx"

[dependencies]
util = { path = "../util" }
"#,
        );
        assert_eq!(m.name, "app");
        assert_eq!(m.deps.len(), 1);
        assert_eq!(m.deps[0].name, "util");
        assert_eq!(m.deps[0].path, PathBuf::from("../util"));
    }
}
