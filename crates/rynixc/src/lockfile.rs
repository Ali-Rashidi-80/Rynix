//! `rynix.lock.toml` — pin resolved local deps with entry sha256 (no network).

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::manifest::DepsReport;

#[derive(Debug, Clone)]
pub struct LockPackage {
    pub name: String,
    pub version: Option<String>,
    pub kind: String,
    pub path: String,
    pub entry: Option<String>,
    pub sha256: String,
}

#[derive(Debug, Clone, Default)]
pub struct LockFile {
    pub packages: Vec<LockPackage>,
}

pub fn lock_path_for_manifest(manifest_path: &Path) -> PathBuf {
    manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("rynix.lock.toml")
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot hash {}: {e}", path.display()))?;
    let digest = Sha256::digest(&bytes);
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

pub fn sha256_sources(sources: &[PathBuf]) -> Result<String, String> {
    let mut hasher = Sha256::new();
    for p in sources {
        let bytes = fs::read(p).map_err(|e| format!("cannot hash {}: {e}", p.display()))?;
        hasher.update(p.display().to_string().as_bytes());
        hasher.update([0]);
        hasher.update(&bytes);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

pub fn lock_from_report(report: &DepsReport) -> Result<LockFile, String> {
    let mut packages = Vec::new();
    for d in &report.deps {
        if !d.ok {
            return Err(format!("cannot lock failed dep `{}`: {}", d.name, d.detail));
        }
        let sha = if d.sources.is_empty() {
            if let Some(e) = &d.entry {
                sha256_file(e)?
            } else {
                continue;
            }
        } else {
            sha256_sources(&d.sources)?
        };
        packages.push(LockPackage {
            name: d.name.clone(),
            version: d.version.clone(),
            kind: d.kind.clone(),
            path: d.path.display().to_string(),
            entry: d.entry.as_ref().map(|p| p.display().to_string()),
            sha256: sha,
        });
    }
    Ok(LockFile { packages })
}

pub fn write_lock(path: &Path, lock: &LockFile) -> Result<(), String> {
    let mut out = String::from("# rynix.lock.v1 — local resolve pin (no network CDN)\n");
    for p in &lock.packages {
        out.push_str("\n[[package]]\n");
        out.push_str(&format!("name = \"{}\"\n", p.name));
        if let Some(v) = &p.version {
            out.push_str(&format!("version = \"{v}\"\n"));
        }
        out.push_str(&format!("kind = \"{}\"\n", p.kind));
        out.push_str(&format!("path = \"{}\"\n", escape_toml(&p.path)));
        if let Some(e) = &p.entry {
            out.push_str(&format!("entry = \"{}\"\n", escape_toml(e)));
        }
        out.push_str(&format!("sha256 = \"{}\"\n", p.sha256));
    }
    fs::write(path, out).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn escape_toml(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn read_lock(path: &Path) -> Result<LockFile, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let mut packages = Vec::new();
    let mut cur: Option<LockPackage> = None;
    for raw in content.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line == "[[package]]" {
            if let Some(p) = cur.take() {
                packages.push(p);
            }
            cur = Some(LockPackage {
                name: String::new(),
                version: None,
                kind: String::new(),
                path: String::new(),
                entry: None,
                sha256: String::new(),
            });
            continue;
        }
        let Some(pkg) = cur.as_mut() else {
            continue;
        };
        if let Some(v) = parse_quoted(line, "name") {
            pkg.name = v;
        } else if let Some(v) = parse_quoted(line, "version") {
            pkg.version = Some(v);
        } else if let Some(v) = parse_quoted(line, "kind") {
            pkg.kind = v;
        } else if let Some(v) = parse_quoted(line, "path") {
            pkg.path = v.replace("\\\\", "\\");
        } else if let Some(v) = parse_quoted(line, "entry") {
            pkg.entry = Some(v.replace("\\\\", "\\"));
        } else if let Some(v) = parse_quoted(line, "sha256") {
            pkg.sha256 = v;
        }
    }
    if let Some(p) = cur {
        packages.push(p);
    }
    Ok(LockFile { packages })
}

fn parse_quoted(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{key} = \"");
    if !line.starts_with(&prefix) || !line.ends_with('"') {
        return None;
    }
    Some(line[prefix.len()..line.len() - 1].to_string())
}

/// Verify resolved report against an existing lock (name/kind/version/sha256).
pub fn verify_report(report: &DepsReport, lock: &LockFile) -> Result<(), String> {
    if report.deps.len() != lock.packages.len() {
        return Err(format!(
            "rynix.lock.toml out of date: resolve has {} deps, lock has {}",
            report.deps.len(),
            lock.packages.len()
        ));
    }
    for (d, lp) in report.deps.iter().zip(lock.packages.iter()) {
        if d.name != lp.name {
            return Err(format!(
                "lock name mismatch: resolve `{}` vs lock `{}`",
                d.name, lp.name
            ));
        }
        if d.kind != lp.kind {
            return Err(format!(
                "lock kind mismatch for `{}`: {} vs {}",
                d.name, d.kind, lp.kind
            ));
        }
        if d.version != lp.version {
            return Err(format!(
                "lock version mismatch for `{}`: {:?} vs {:?}",
                d.name, d.version, lp.version
            ));
        }
        let sha = if d.sources.is_empty() {
            d.entry
                .as_ref()
                .map(|e| sha256_file(e))
                .transpose()?
                .unwrap_or_default()
        } else {
            sha256_sources(&d.sources)?
        };
        if sha != lp.sha256 {
            return Err(format!(
                "lock sha256 mismatch for `{}`: sources changed (expected {}, got {})",
                d.name, lp.sha256, sha
            ));
        }
    }
    Ok(())
}

/// If `rynix.lock.toml` exists beside the manifest, verify; else Ok.
pub fn verify_if_present(report: &DepsReport) -> Result<(), String> {
    let path = lock_path_for_manifest(&report.root_manifest);
    if !path.is_file() {
        return Ok(());
    }
    let lock = read_lock(&path)?;
    verify_report(report, &lock)
}
