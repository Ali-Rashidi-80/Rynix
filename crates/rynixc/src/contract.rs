//! Agent contract evidence checker (`rynixc verify --contract=…`).
//!
//! See [ADR-0009](../../../docs/adr/0009-agent-contracts-toolchain.md): contracts are
//! toolchain TOML artifacts, not new `.ryx` keywords.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

#[derive(Debug, Clone, Default)]
pub struct ContractFile {
    pub name: String,
    pub evidence: Vec<EvidenceItem>,
}

#[derive(Debug, Clone)]
pub struct EvidenceItem {
    pub id: String,
    pub kind: EvidenceKind,
}

#[derive(Debug, Clone)]
pub enum EvidenceKind {
    /// Path must exist; optional substring must appear in file contents.
    File { path: String, contains: Option<String> },
    /// Named cargo test filter; static check unless `run_tests` is true.
    CargoTest {
        package: String,
        filter: String,
    },
}

#[derive(Debug, Clone)]
pub struct EvidenceResult {
    pub id: String,
    pub kind: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct VerifyReport {
    pub contract: String,
    pub status: String,
    pub evidence: Vec<EvidenceResult>,
    pub ran_tests: bool,
}

impl VerifyReport {
    pub fn to_json(&self) -> Value {
        json!({
            "schema": "rynix.verify.v1",
            "contract": self.contract,
            "status": self.status,
            "ran_tests": self.ran_tests,
            "passed": self.evidence.iter().filter(|e| e.ok).count(),
            "failed": self.evidence.iter().filter(|e| !e.ok).count(),
            "evidence": self.evidence.iter().map(|e| json!({
                "id": e.id,
                "kind": e.kind,
                "ok": e.ok,
                "detail": e.detail,
            })).collect::<Vec<_>>(),
        })
    }

    pub fn all_ok(&self) -> bool {
        self.evidence.iter().all(|e| e.ok)
    }
}

pub struct ContractEngine;

impl ContractEngine {
    pub fn load(path: &Path) -> Result<ContractFile, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        Ok(parse_contract_toml(&content))
    }

    pub fn verify(contract: &ContractFile, root: &Path, run_tests: bool) -> VerifyReport {
        let mut evidence = Vec::new();
        for item in &contract.evidence {
            evidence.push(check_evidence(item, root, run_tests));
        }
        let status = if evidence.iter().all(|e| e.ok) {
            "passed".into()
        } else {
            "failed".into()
        };
        VerifyReport {
            contract: contract.name.clone(),
            status,
            evidence,
            ran_tests: run_tests,
        }
    }
}

fn check_evidence(item: &EvidenceItem, root: &Path, run_tests: bool) -> EvidenceResult {
    match &item.kind {
        EvidenceKind::File { path, contains } => {
            let full = root.join(path);
            if !full.is_file() {
                return EvidenceResult {
                    id: item.id.clone(),
                    kind: "file".into(),
                    ok: false,
                    detail: format!("missing file: {path}"),
                };
            }
            if let Some(needle) = contains {
                let text = fs::read_to_string(&full).unwrap_or_default();
                if !text.contains(needle) {
                    return EvidenceResult {
                        id: item.id.clone(),
                        kind: "file".into(),
                        ok: false,
                        detail: format!("file `{path}` does not contain `{needle}`"),
                    };
                }
            }
            EvidenceResult {
                id: item.id.clone(),
                kind: "file".into(),
                ok: true,
                detail: format!("ok: {path}"),
            }
        }
        EvidenceKind::CargoTest { package, filter } => {
            if run_tests {
                return run_cargo_test(&item.id, package, filter, root);
            }
            // Static: filter string must appear under the package's tests/ or src/.
            let pkg_root = find_package_dir(root, package);
            let Some(pkg_dir) = pkg_root else {
                return EvidenceResult {
                    id: item.id.clone(),
                    kind: "cargo_test".into(),
                    ok: false,
                    detail: format!("package `{package}` not found under crates/"),
                };
            };
            let mut haystack = String::new();
            for sub in ["tests", "src"] {
                let dir = pkg_dir.join(sub);
                if dir.is_dir() {
                    collect_rs_text(&dir, &mut haystack);
                }
            }
            if haystack.contains(filter) {
                EvidenceResult {
                    id: item.id.clone(),
                    kind: "cargo_test".into(),
                    ok: true,
                    detail: format!("static: filter `{filter}` found in {package}"),
                }
            } else {
                EvidenceResult {
                    id: item.id.clone(),
                    kind: "cargo_test".into(),
                    ok: false,
                    detail: format!("static: filter `{filter}` not found in {package}"),
                }
            }
        }
    }
}

fn run_cargo_test(id: &str, package: &str, filter: &str, root: &Path) -> EvidenceResult {
    let out = Command::new("cargo")
        .args(["test", "-p", package, filter, "--", "--exact"])
        .current_dir(root)
        .output();
    match out {
        Ok(o) if o.status.success() => EvidenceResult {
            id: id.into(),
            kind: "cargo_test".into(),
            ok: true,
            detail: format!("cargo test -p {package} {filter} passed"),
        },
        Ok(o) => {
            // Retry without --exact (filter substring match).
            let out2 = Command::new("cargo")
                .args(["test", "-p", package, filter])
                .current_dir(root)
                .output();
            match out2 {
                Ok(o2) if o2.status.success() => EvidenceResult {
                    id: id.into(),
                    kind: "cargo_test".into(),
                    ok: true,
                    detail: format!("cargo test -p {package} {filter} passed"),
                },
                Ok(o2) => EvidenceResult {
                    id: id.into(),
                    kind: "cargo_test".into(),
                    ok: false,
                    detail: format!(
                        "cargo test failed: {}",
                        String::from_utf8_lossy(&o2.stderr)
                            .lines()
                            .rev()
                            .take(3)
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .collect::<Vec<_>>()
                            .join(" | ")
                    ),
                },
                Err(e) => EvidenceResult {
                    id: id.into(),
                    kind: "cargo_test".into(),
                    ok: false,
                    detail: format!("cargo spawn failed after first attempt ({e}); first: {}", o.status),
                },
            }
        }
        Err(e) => EvidenceResult {
            id: id.into(),
            kind: "cargo_test".into(),
            ok: false,
            detail: format!("cannot spawn cargo: {e}"),
        },
    }
}

fn find_package_dir(root: &Path, package: &str) -> Option<PathBuf> {
    let direct = root.join("crates").join(package);
    if direct.is_dir() {
        return Some(direct);
    }
    // Scan crates/*/Cargo.toml for name =
    let crates = root.join("crates");
    let entries = fs::read_dir(crates).ok()?;
    for ent in entries.flatten() {
        let cargo = ent.path().join("Cargo.toml");
        if let Ok(text) = fs::read_to_string(&cargo) {
            if text.contains(&format!("name = \"{package}\""))
                || text.contains(&format!("name = '{package}'"))
            {
                return Some(ent.path());
            }
        }
    }
    None
}

fn collect_rs_text(dir: &Path, out: &mut String) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for ent in entries.flatten() {
        let p = ent.path();
        if p.is_dir() {
            collect_rs_text(&p, out);
        } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
            if let Ok(t) = fs::read_to_string(&p) {
                out.push_str(&t);
                out.push('\n');
            }
        }
    }
}

/// Minimal TOML subset for contract files (no external crate).
pub fn parse_contract_toml(content: &str) -> ContractFile {
    let mut file = ContractFile::default();
    let mut section = String::new();
    let mut current: Option<EvidenceDraft> = None;

    for line in content.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            flush_evidence(&mut file, &mut current);
            let name = line.trim_matches(['[', ']']);
            if name == "contract" {
                section = "contract".into();
            } else if name == "evidence" || name == "[evidence]" || name.starts_with("evidence") {
                // [[evidence]] becomes "evidence" after trim of one bracket pair;
                // handle both [evidence] and evidence from [[evidence]]
                section = "evidence".into();
                current = Some(EvidenceDraft::default());
            } else {
                section = name.to_string();
            }
            // Fix: line `[[evidence]]` → trim_matches once → `[evidence]` still has brackets
            continue;
        }
        // Handle `[[evidence]]` — after one trim_matches(['[',']']) we get `[evidence]`
        // Actually `"[[evidence]]".trim_matches(['[', ']'])` => `"evidence"` because
        // trim_matches strips all leading/trailing chars in the set.
        // Good.

        if section == "contract" {
            if let Some(v) = parse_string_assign(line, "name") {
                file.name = v;
            }
            continue;
        }
        if section == "evidence" {
            let draft = current.get_or_insert_with(EvidenceDraft::default);
            if let Some(v) = parse_string_assign(line, "id") {
                draft.id = v;
            } else if let Some(v) = parse_string_assign(line, "kind") {
                draft.kind = v;
            } else if let Some(v) = parse_string_assign(line, "path") {
                draft.path = Some(v);
            } else if let Some(v) = parse_string_assign(line, "contains") {
                draft.contains = Some(v);
            } else if let Some(v) = parse_string_assign(line, "package") {
                draft.package = Some(v);
            } else if let Some(v) = parse_string_assign(line, "filter") {
                draft.filter = Some(v);
            }
        }
    }
    flush_evidence(&mut file, &mut current);
    if file.name.is_empty() {
        file.name = "unnamed".into();
    }
    file
}

#[derive(Default)]
struct EvidenceDraft {
    id: String,
    kind: String,
    path: Option<String>,
    contains: Option<String>,
    package: Option<String>,
    filter: Option<String>,
}

fn flush_evidence(file: &mut ContractFile, current: &mut Option<EvidenceDraft>) {
    let Some(d) = current.take() else {
        return;
    };
    if d.id.is_empty() {
        return;
    }
    let kind = match d.kind.as_str() {
        "cargo_test" => EvidenceKind::CargoTest {
            package: d.package.unwrap_or_else(|| "rynixc".into()),
            filter: d.filter.unwrap_or_default(),
        },
        _ => EvidenceKind::File {
            path: d.path.unwrap_or_default(),
            contains: d.contains,
        },
    };
    file.evidence.push(EvidenceItem { id: d.id, kind });
}

fn parse_string_assign(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{key} ");
    let eq = format!("{key}=");
    let rest = if let Some(r) = line.strip_prefix(&format!("{key} = ")) {
        r
    } else if let Some(r) = line.strip_prefix(&eq) {
        r.trim()
    } else if line.starts_with(&prefix) && line.contains('=') {
        line.split_once('=')?.1.trim()
    } else {
        return None;
    };
    let rest = rest.trim();
    if (rest.starts_with('"') && rest.ends_with('"'))
        || (rest.starts_with('\'') && rest.ends_with('\''))
    {
        Some(rest[1..rest.len() - 1].to_string())
    } else {
        Some(rest.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_file_and_cargo_evidence() {
        let c = parse_contract_toml(
            r#"
[contract]
name = "demo"

[[evidence]]
id = "gates"
kind = "file"
path = "crates/rynixc/tests/phase10_gates.rs"
contains = "arch"

[[evidence]]
id = "cli"
kind = "cargo_test"
package = "rynixc"
filter = "graph_emits_schema"
"#,
        );
        assert_eq!(c.name, "demo");
        assert_eq!(c.evidence.len(), 2);
        assert!(matches!(c.evidence[0].kind, EvidenceKind::File { .. }));
        assert!(matches!(c.evidence[1].kind, EvidenceKind::CargoTest { .. }));
    }

    #[test]
    fn verify_missing_file_fails() {
        let c = parse_contract_toml(
            r#"
[contract]
name = "x"
[[evidence]]
id = "nope"
kind = "file"
path = "does/not/exist.rs"
"#,
        );
        let report = ContractEngine::verify(&c, Path::new("."), false);
        assert!(!report.all_ok());
        assert_eq!(report.status, "failed");
    }
}
