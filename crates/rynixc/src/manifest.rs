//! `rynix.toml` manifest + dependency resolution (Phase D3 + local index).
//!
//! Dependencies may be:
//! - `{ path = "..." }` — filesystem path (existing)
//! - `"x.y.z"` version string — resolved under `[registry] path = "..."` only
//!
//! There is **no** network registry. Missing local vendor dirs fail resolve/build.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

#[derive(Debug, Clone, Default)]
pub struct Manifest {
    pub dir: PathBuf,
    pub name: String,
    pub version: String,
    pub entry: Option<PathBuf>,
    /// Extra `.ryx` sources after `entry` (manifest order; SPEC §6.3).
    pub files: Vec<PathBuf>,
    pub runtime: Option<String>,
    pub optimize: Option<bool>,
    /// Local package index root (`[registry] path = "vendor"`).
    pub registry_path: Option<PathBuf>,
    pub deps: Vec<DepSpec>,
}

#[derive(Debug, Clone)]
pub struct DepSpec {
    pub name: String,
    pub kind: DepKind,
}

#[derive(Debug, Clone)]
pub enum DepKind {
    Path(PathBuf),
    /// Exact version string resolved under the local registry index.
    Version(String),
}

#[derive(Debug, Clone)]
pub struct ResolvedDep {
    pub name: String,
    pub kind: String,
    pub path: PathBuf,
    pub version: Option<String>,
    pub entry: Option<PathBuf>,
    /// Ordered sources: entry then `[package] files` (absolute).
    pub sources: Vec<PathBuf>,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct DepsReport {
    pub root_manifest: PathBuf,
    pub package: String,
    pub registry: Option<PathBuf>,
    pub deps: Vec<ResolvedDep>,
}

impl DepsReport {
    pub fn all_ok(&self) -> bool {
        self.deps.iter().all(|d| d.ok)
    }

    /// Unity compile units: `(package_name, ordered source paths)`.
    pub fn compile_units(&self) -> Result<Vec<(String, Vec<PathBuf>)>, String> {
        let mut out = Vec::new();
        for d in &self.deps {
            if !d.ok {
                return Err(format!("{}: {}", d.name, d.detail));
            }
            if d.sources.is_empty() {
                return Err(format!(
                    "dependency `{}` has no compile sources (need `[package].entry`)",
                    d.name
                ));
            }
            for p in &d.sources {
                if !p.is_file() {
                    return Err(format!(
                        "dependency `{}` source missing: {}",
                        d.name,
                        p.display()
                    ));
                }
            }
            out.push((d.name.clone(), d.sources.clone()));
        }
        Ok(out)
    }

    /// Entry `.ryx` paths (first source of each unit).
    #[allow(dead_code)]
    pub fn compile_entry_paths(&self) -> Result<Vec<PathBuf>, String> {
        Ok(self
            .compile_units()?
            .into_iter()
            .filter_map(|(_, paths)| paths.into_iter().next())
            .collect())
    }

    pub fn to_json(&self) -> Value {
        json!({
            "schema": "rynix.deps.v1",
            "manifest": self.root_manifest.display().to_string(),
            "package": self.package,
            "registry": self.registry.as_ref().map(|p| p.display().to_string()),
            "status": if self.all_ok() { "ok" } else { "error" },
            "dependencies": self.deps.iter().map(|d| json!({
                "name": d.name,
                "kind": d.kind,
                "path": d.path.display().to_string(),
                "version": d.version,
                "entry": d.entry.as_ref().map(|p| p.display().to_string()),
                "sources": d.sources.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                "ok": d.ok,
                "detail": d.detail,
            })).collect::<Vec<_>>(),
            "note": "Local path deps and optional filesystem package index — no network registry.",
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

fn resolve_one_dir(name: &str, kind: &str, abs: PathBuf, version: Option<String>) -> ResolvedDep {
    let dep_toml = abs.join("rynix.toml");
    if !abs.is_dir() {
        return ResolvedDep {
            name: name.into(),
            kind: kind.into(),
            path: abs,
            version,
            entry: None,
            sources: Vec::new(),
            ok: false,
            detail: "path is not a directory".into(),
        };
    }
    if !dep_toml.is_file() {
        return ResolvedDep {
            name: name.into(),
            kind: kind.into(),
            path: abs,
            version,
            entry: None,
            sources: Vec::new(),
            ok: false,
            detail: "missing rynix.toml in dependency path".into(),
        };
    }
    match load_manifest(&dep_toml) {
        Ok(dm) => {
            let entry = dm.entry.as_ref().map(|e| {
                let p = dm.dir.join(e);
                fs::canonicalize(&p).unwrap_or(p)
            });
            let mut sources = Vec::new();
            if let Some(e) = &entry {
                sources.push(e.clone());
            }
            for rel in &dm.files {
                let p = dm.dir.join(rel);
                let p = fs::canonicalize(&p).unwrap_or(p);
                if entry.as_ref().is_some_and(|e| e == &p) {
                    continue;
                }
                sources.push(p);
            }
            let (ok, detail) = match &entry {
                Some(p) if p.is_file() => {
                    let missing: Vec<_> = sources.iter().filter(|s| !s.is_file()).collect();
                    if missing.is_empty() {
                        (
                            true,
                            format!("entry {} ({} source(s))", p.display(), sources.len()),
                        )
                    } else {
                        (
                            false,
                            format!("source missing: {}", missing[0].display()),
                        )
                    }
                }
                Some(p) => (false, format!("entry missing: {}", p.display())),
                None => (true, "manifest ok (no entry — check-only dep)".into()),
            };
            ResolvedDep {
                name: name.into(),
                kind: kind.into(),
                path: abs,
                version,
                entry,
                sources,
                ok,
                detail,
            }
        }
        Err(e) => ResolvedDep {
            name: name.into(),
            kind: kind.into(),
            path: abs,
            version,
            entry: None,
            sources: Vec::new(),
            ok: false,
            detail: e,
        },
    }
}

fn resolve_registry_version(manifest: &Manifest, name: &str, ver_req: &str) -> ResolvedDep {
    let Some(reg_rel) = &manifest.registry_path else {
        return ResolvedDep {
            name: name.into(),
            kind: "registry".into(),
            path: manifest.dir.clone(),
            version: Some(ver_req.into()),
            entry: None,
            sources: Vec::new(),
            ok: false,
            detail: "version dep requires [registry] path = \"…\" (local index only)".into(),
        };
    };
    let reg = if reg_rel.is_absolute() {
        reg_rel.clone()
    } else {
        manifest.dir.join(reg_rel)
    };
    let reg = fs::canonicalize(&reg).unwrap_or(reg);

    // Exact directory match only when the req is a plain version (not a range).
    if semver::Version::parse(ver_req).is_ok() {
        let exact_candidates = [
            reg.join(name).join(ver_req),
            reg.join(format!("{name}-{ver_req}")),
        ];
        for cand in &exact_candidates {
            if cand.is_dir() && cand.join("rynix.toml").is_file() {
                let abs = fs::canonicalize(cand).unwrap_or_else(|_| cand.clone());
                return resolve_one_dir(name, "registry", abs, Some(ver_req.into()));
            }
        }
    }

    // Semver range: ^1.2.3, >=1.2.3, =1.2.3 — pick highest matching hierarchical version.
    let req = match semver::VersionReq::parse(ver_req) {
        Ok(r) => r,
        Err(e) => {
            return ResolvedDep {
                name: name.into(),
                kind: "registry".into(),
                path: reg.clone(),
                version: Some(ver_req.into()),
                entry: None,
                sources: Vec::new(),
                ok: false,
                detail: format!(
                    "invalid version req `{ver_req}` for `{name}`: {e} (use exact dir, ^x.y.z, or >=x.y.z)"
                ),
            };
        }
    };

    let pkg_dir = reg.join(name);
    let mut best: Option<(semver::Version, PathBuf)> = None;
    if pkg_dir.is_dir()
        && let Ok(rd) = fs::read_dir(&pkg_dir)
    {
        for ent in rd.flatten() {
            let path = ent.path();
            if !path.is_dir() || !path.join("rynix.toml").is_file() {
                continue;
            }
            let Some(ver_s) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let Ok(ver) = semver::Version::parse(ver_s) else {
                continue;
            };
            if !req.matches(&ver) {
                continue;
            }
            match &best {
                None => best = Some((ver, path)),
                Some((cur, _)) if ver > *cur => best = Some((ver, path)),
                _ => {}
            }
        }
    }

    if let Some((ver, path)) = best {
        let abs = fs::canonicalize(&path).unwrap_or(path);
        return resolve_one_dir(name, "registry", abs, Some(ver.to_string()));
    }

    ResolvedDep {
        name: name.into(),
        kind: "registry".into(),
        path: reg.clone(),
        version: Some(ver_req.into()),
        entry: None,
        sources: Vec::new(),
        ok: false,
        detail: format!(
            "package `{name}` req `{ver_req}` not found under local registry (exact dir or semver range under {}/{{name}}/)",
            reg.display()
        ),
    }
}

pub fn resolve_deps(manifest: &Manifest) -> DepsReport {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut stack = std::collections::HashSet::new();
    resolve_deps_rec(manifest, &mut out, &mut seen, &mut stack);
    let registry = manifest.registry_path.as_ref().map(|p| {
        let abs = if p.is_absolute() {
            p.clone()
        } else {
            manifest.dir.join(p)
        };
        fs::canonicalize(&abs).unwrap_or(abs)
    });
    DepsReport {
        root_manifest: manifest.dir.join("rynix.toml"),
        package: manifest.name.clone(),
        registry,
        deps: out,
    }
}

/// Depth-first post-order: transitive deps appear before their dependents.
fn resolve_deps_rec(
    manifest: &Manifest,
    out: &mut Vec<ResolvedDep>,
    seen: &mut std::collections::HashSet<PathBuf>,
    stack: &mut std::collections::HashSet<PathBuf>,
) {
    for dep in &manifest.deps {
        let resolved = match &dep.kind {
            DepKind::Path(p) => {
                let abs = if p.is_absolute() {
                    p.clone()
                } else {
                    manifest.dir.join(p)
                };
                let abs = fs::canonicalize(&abs).unwrap_or(abs);
                resolve_one_dir(&dep.name, "path", abs, None)
            }
            DepKind::Version(ver) => resolve_registry_version(manifest, &dep.name, ver),
        };
        if !resolved.ok {
            out.push(resolved);
            continue;
        }
        let key = resolved.path.clone();
        if stack.contains(&key) {
            out.push(ResolvedDep {
                name: resolved.name,
                kind: resolved.kind,
                path: resolved.path,
                version: resolved.version,
                entry: resolved.entry,
                sources: Vec::new(),
                ok: false,
                detail: format!("cyclic dependency involving `{}`", dep.name),
            });
            continue;
        }
        if seen.contains(&key) {
            continue;
        }
        stack.insert(key.clone());
        let child_toml = key.join("rynix.toml");
        if child_toml.is_file()
            && let Ok(child) = load_manifest(&child_toml)
        {
            // Child registry is relative to the child package; inherit root
            // registry only when child has none and parent is the app with registry.
            resolve_deps_rec(&child, out, seen, stack);
        }
        stack.remove(&key);
        seen.insert(key);
        out.push(resolved);
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
            if let Some(eq) = line.find('=') {
                let name = line[..eq].trim().to_string();
                let rhs = line[eq + 1..].trim();
                if let Some(path) = extract_path_from_inline_table(rhs) {
                    m.deps.push(DepSpec {
                        name,
                        kind: DepKind::Path(PathBuf::from(path)),
                    });
                } else if let Some(ver) = parse_bare_string(rhs) {
                    m.deps.push(DepSpec {
                        name,
                        kind: DepKind::Version(ver),
                    });
                } else if rhs == "{" || rhs.starts_with('{') {
                    pending_dep = Some(name);
                }
            } else if let Some(name) = pending_dep.clone() {
                if let Some(path) = parse_string_assign(line, "path") {
                    m.deps.push(DepSpec {
                        name,
                        kind: DepKind::Path(PathBuf::from(path)),
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
                } else if let Some(files) = parse_string_array_assign(line, "files") {
                    m.files = files.into_iter().map(PathBuf::from).collect();
                }
            }
            "[registry]" => {
                if let Some(v) = parse_string_assign(line, "path") {
                    m.registry_path = Some(PathBuf::from(v));
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

fn parse_bare_string(rhs: &str) -> Option<String> {
    let rhs = rhs.trim();
    if rhs.starts_with('{') {
        return None;
    }
    let v = rhs.trim_matches(|c| c == '"' || c == '\'');
    // Reject empty / unquoted inline junk; allow semver ops (`>=`, `<=`, `=`).
    if v.is_empty() {
        None
    } else {
        Some(v.to_string())
    }
}

fn extract_path_from_inline_table(rhs: &str) -> Option<String> {
    let rhs = rhs.trim();
    if !rhs.starts_with('{') {
        return None;
    }
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

/// `files = ["a.ryx", "b.ryx"]` — inline string array only.
fn parse_string_array_assign(line: &str, key: &str) -> Option<Vec<String>> {
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
    if !val.starts_with('[') || !val.ends_with(']') {
        return None;
    }
    let inner = &val[1..val.len() - 1];
    let mut out = Vec::new();
    for part in inner.split(',') {
        let s = part.trim().trim_matches(|c| c == '"' || c == '\'');
        if !s.is_empty() {
            out.push(s.to_string());
        }
    }
    Some(out)
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
        match &m.deps[0].kind {
            DepKind::Path(p) => assert_eq!(p, &PathBuf::from("../util")),
            _ => panic!("expected path dep"),
        }
    }

    #[test]
    fn parses_registry_version_dep() {
        let m = parse_manifest_toml(
            r#"
[package]
name = "app"

[registry]
path = "vendor"

[dependencies]
util = "0.1.0"
"#,
        );
        assert_eq!(m.registry_path, Some(PathBuf::from("vendor")));
        assert_eq!(m.deps.len(), 1);
        match &m.deps[0].kind {
            DepKind::Version(v) => assert_eq!(v, "0.1.0"),
            _ => panic!("expected version dep"),
        }
    }

    #[test]
    fn resolves_local_registry_layout() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/pkg_reg_app");
        let root = root.canonicalize().expect("testdata");
        let m = load_manifest(&root.join("rynix.toml")).expect("manifest");
        let report = resolve_deps(&m);
        assert!(report.all_ok(), "{:?}", report.deps);
        assert_eq!(report.deps.len(), 1);
        assert_eq!(report.deps[0].kind, "registry");
        assert_eq!(report.deps[0].version.as_deref(), Some("0.1.0"));
    }

    #[test]
    fn resolves_transitive_path_deps_postorder() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/pkg_app");
        let root = root.canonicalize().expect("testdata");
        let m = load_manifest(&root.join("rynix.toml")).expect("manifest");
        let report = resolve_deps(&m);
        assert!(report.all_ok(), "{:?}", report.deps);
        assert_eq!(report.deps.len(), 2, "core then util: {:?}", report.deps);
        assert_eq!(report.deps[0].name, "core");
        assert_eq!(report.deps[1].name, "util");
        let entries = report.compile_entry_paths().expect("entries");
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn resolves_semver_caret_to_highest() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/pkg_semver_app");
        let root = root.canonicalize().expect("testdata");
        let m = load_manifest(&root.join("rynix.toml")).expect("manifest");
        let report = resolve_deps(&m);
        assert!(report.all_ok(), "{:?}", report.deps);
        assert_eq!(report.deps.len(), 1);
        assert_eq!(report.deps[0].name, "util");
        assert_eq!(report.deps[0].version.as_deref(), Some("0.2.0"));
    }

    #[test]
    fn resolves_semver_caret_stays_on_minor_0() {
        let dir = std::env::temp_dir().join("rynix_semver_caret");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("vendor/util/0.1.0")).unwrap();
        std::fs::create_dir_all(dir.join("vendor/util/0.2.0")).unwrap();
        for ver in ["0.1.0", "0.2.0"] {
            std::fs::write(
                dir.join(format!("vendor/util/{ver}/rynix.toml")),
                format!("[package]\nname = \"util\"\nversion = \"{ver}\"\nentry = \"lib.ryx\"\n"),
            )
            .unwrap();
            std::fs::write(
                dir.join(format!("vendor/util/{ver}/lib.ryx")),
                "def util_answer() -> i64\n  return 1\nend\n",
            )
            .unwrap();
        }
        std::fs::write(
            dir.join("rynix.toml"),
            r#"
[package]
name = "app"
entry = "main.ryx"
[registry]
path = "vendor"
[dependencies]
util = "^0.1.0"
"#,
        )
        .unwrap();
        std::fs::write(dir.join("main.ryx"), "def main() -> i64\n  return 0\nend\n").unwrap();
        let m = load_manifest(&dir.join("rynix.toml")).expect("manifest");
        let report = resolve_deps(&m);
        assert!(report.all_ok(), "{:?}", report.deps);
        assert_eq!(report.deps[0].version.as_deref(), Some("0.1.0"));
    }
}
