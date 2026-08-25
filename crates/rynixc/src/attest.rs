//! Local package digest attest (`rynix.attest.v1`) — offline SHA-256 of the lock.
//!
//! Not Sigstore Rekor/Fulcio/OIDC. A machine-readable digest bundle agents can
//! pin beside `rynix.lock.toml`.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::lockfile::{self, LockFile, sha256_file};
use crate::manifest::DepsReport;

const SCHEMA: &str = "rynix.attest.v1";
const KIND: &str = "local_digest";
const NOTE: &str =
    "offline SHA-256 of rynix.lock.toml plus package pins; not Sigstore Rekor/Fulcio";

#[derive(Debug, Clone)]
pub struct AttestFile {
    pub lock_path: String,
    pub lock_sha256: String,
    pub packages: Vec<(String, String)>,
}

pub fn attest_path_for_manifest(manifest_path: &Path) -> PathBuf {
    let lock = lockfile::lock_path_for_manifest(manifest_path);
    lock.with_file_name("rynix.attest.v1.json")
}

pub fn from_lock(lock_path: &Path, lock: &LockFile) -> Result<AttestFile, String> {
    let lock_sha256 = sha256_file(lock_path)?;
    let packages = lock
        .packages
        .iter()
        .map(|p| (p.name.clone(), p.sha256.clone()))
        .collect();
    Ok(AttestFile {
        lock_path: lock_path.display().to_string(),
        lock_sha256,
        packages,
    })
}

pub fn write_attest(path: &Path, attest: &AttestFile) -> Result<(), String> {
    let packages: Vec<Value> = attest
        .packages
        .iter()
        .map(|(name, sha256)| {
            serde_json::json!({
                "name": name,
                "sha256": sha256,
            })
        })
        .collect();
    let body = serde_json::json!({
        "schema": SCHEMA,
        "kind": KIND,
        "note": NOTE,
        "lock_path": attest.lock_path,
        "lock_sha256": attest.lock_sha256,
        "packages": packages,
    });
    let text = serde_json::to_string_pretty(&body)
        .map_err(|e| format!("cannot serialize attest: {e}"))?;
    fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

pub fn read_attest(path: &Path) -> Result<AttestFile, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let v: Value = serde_json::from_str(&text)
        .map_err(|e| format!("invalid attest JSON {}: {e}", path.display()))?;
    if v.get("schema").and_then(|s| s.as_str()) != Some(SCHEMA) {
        return Err(format!(
            "unexpected attest schema in {} (want {SCHEMA})",
            path.display()
        ));
    }
    if v.get("kind").and_then(|s| s.as_str()) != Some(KIND) {
        return Err(format!(
            "unexpected attest kind in {} (want {KIND})",
            path.display()
        ));
    }
    let lock_path = v
        .get("lock_path")
        .and_then(|s| s.as_str())
        .ok_or_else(|| format!("missing lock_path in {}", path.display()))?
        .to_string();
    let lock_sha256 = v
        .get("lock_sha256")
        .and_then(|s| s.as_str())
        .ok_or_else(|| format!("missing lock_sha256 in {}", path.display()))?
        .to_string();
    let mut packages = Vec::new();
    let arr = v
        .get("packages")
        .and_then(|p| p.as_array())
        .ok_or_else(|| format!("missing packages in {}", path.display()))?;
    for item in arr {
        let name = item
            .get("name")
            .and_then(|s| s.as_str())
            .ok_or_else(|| "attest package missing name".to_string())?
            .to_string();
        let sha256 = item
            .get("sha256")
            .and_then(|s| s.as_str())
            .ok_or_else(|| format!("attest package `{name}` missing sha256"))?
            .to_string();
        packages.push((name, sha256));
    }
    Ok(AttestFile {
        lock_path,
        lock_sha256,
        packages,
    })
}

pub fn verify_attest(
    report: &DepsReport,
    lock_path: &Path,
    lock: &LockFile,
    attest: &AttestFile,
) -> Result<(), String> {
    lockfile::verify_report(report, lock)?;
    let got = sha256_file(lock_path)?;
    if got != attest.lock_sha256 {
        return Err(format!(
            "attest lock_sha256 mismatch: expected {}, got {got}",
            attest.lock_sha256
        ));
    }
    if attest.packages.len() != lock.packages.len() {
        return Err(format!(
            "attest package count mismatch: {} vs {}",
            attest.packages.len(),
            lock.packages.len()
        ));
    }
    for (i, lp) in lock.packages.iter().enumerate() {
        let (name, sha) = &attest.packages[i];
        if name != &lp.name {
            return Err(format!(
                "attest package name mismatch at {i}: `{name}` vs `{}`",
                lp.name
            ));
        }
        if sha != &lp.sha256 {
            return Err(format!(
                "attest sha256 mismatch for `{name}`: expected {}, got {sha}",
                lp.sha256
            ));
        }
    }
    Ok(())
}

/// Attach `attest` object to a `rynix.deps.v1` JSON value.
pub fn enrich_attest_json(report: &DepsReport, mut v: Value) -> Value {
    let lock_path = lockfile::lock_path_for_manifest(&report.root_manifest);
    let path = attest_path_for_manifest(&report.root_manifest);
    let attest = if !path.is_file() {
        serde_json::json!({
            "present": false,
            "ok": true,
            "path": Value::Null,
            "detail": "no rynix.attest.v1.json",
        })
    } else if !lock_path.is_file() {
        serde_json::json!({
            "present": true,
            "ok": false,
            "path": path.display().to_string(),
            "detail": "attest present but rynix.lock.toml missing",
        })
    } else {
        match (lockfile::read_lock(&lock_path), read_attest(&path)) {
            (Ok(lock), Ok(attest)) => match verify_attest(report, &lock_path, &lock, &attest) {
                Ok(()) => serde_json::json!({
                    "present": true,
                    "ok": true,
                    "path": path.display().to_string(),
                    "detail": "verified",
                }),
                Err(e) => serde_json::json!({
                    "present": true,
                    "ok": false,
                    "path": path.display().to_string(),
                    "detail": e,
                }),
            },
            (Err(e), _) | (_, Err(e)) => serde_json::json!({
                "present": true,
                "ok": false,
                "path": path.display().to_string(),
                "detail": e,
            }),
        }
    };
    if let Some(obj) = v.as_object_mut() {
        obj.insert("attest".into(), attest);
    }
    v
}
