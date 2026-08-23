//! CLI tests for AI agent commands (graph / slice / impact / eval).

use std::path::PathBuf;
use std::process::Command;

fn rynixc() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_rynixc"));
    c.current_dir(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root"),
    );
    c
}

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
        .canonicalize()
        .expect("example path")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

#[test]
fn graph_emits_schema_and_edges() {
    let path = example("02_match_loop.ryx");
    let out = rynixc()
        .arg("graph")
        .arg(&path)
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{:?}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("rynix.graph.v1"));
    assert!(text.contains("\"classify\""));
    assert!(text.contains("\"main\""));
    assert!(text.contains("\"edges\""));
}

#[test]
fn impact_lists_callers_callees() {
    let path = example("02_match_loop.ryx");
    let out = rynixc()
        .args(["impact", path.to_str().unwrap(), "--fn=main"])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{:?}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("rynix.impact.v1"));
    assert!(text.contains("main"));
    assert!(text.contains("\"nodes\""));
}

#[test]
fn eval_arith() {
    let out = rynixc()
        .args(["eval", "2 + 3 * 4"])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{:?}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "14");
}

#[test]
fn eval_json_schema() {
    let out = rynixc()
        .args(["eval", "--json", "10 + 5"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("rynix.eval.v1"));
    assert!(text.contains("15"));
}

#[test]
fn slice_human_outline() {
    let path = example("02_match_loop.ryx");
    let out = rynixc()
        .arg("slice")
        .arg(&path)
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("def classify"));
    assert!(text.contains("def main"));
}

#[test]
fn arch_check_json_on_repo() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("root");
    let out = rynixc()
        .args([
            "arch",
            "check",
            "--root",
            root.to_str().unwrap(),
            "--error-format=json",
        ])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{:?}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).expect("json");
    assert_eq!(v["schema"], "rynix.arch.v1");
    assert_eq!(v["status"], "passed");
}

#[test]
fn verify_wave1_contract_static() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("root");
    let contract = root.join("docs/contracts/wave1.contract.toml");
    let out = rynixc()
        .args([
            "verify",
            "--contract",
            contract.to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--error-format=json",
        ])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{:?}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).expect("json");
    assert_eq!(v["schema"], "rynix.verify.v1");
    assert_eq!(v["status"], "passed");
    assert_eq!(v["ran_tests"], false);
}

#[test]
fn precheck_json_write_gate() {
    let path = example("02_match_loop.ryx");
    let out = rynixc()
        .args([
            "precheck",
            path.to_str().unwrap(),
            "--fn=main",
            "--error-format=json",
        ])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{:?}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).expect("json");
    assert_eq!(v["schema"], "rynix.precheck.v1");
    assert_eq!(v["write_allowed"], false);
    assert!(v["impact"].is_object());
}

#[test]
fn context_respects_budget() {
    let path = example("02_match_loop.ryx");
    let out = rynixc()
        .args([
            "context",
            path.to_str().unwrap(),
            "--budget=20",
            "--error-format=json",
        ])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{:?}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).expect("json");
    assert_eq!(v["schema"], "rynix.context.v1");
    assert_eq!(v["budget"], 20);
    assert!(v["truncated"].as_bool().unwrap_or(false) || v["chars_used"].as_u64().unwrap() <= 20);
}

#[test]
fn security_flags_sk_live() {
    let dir = std::env::temp_dir().join("rynix_security_fixture");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("bad.ryx");
    std::fs::write(
        &path,
        "def main() -> i64\n  let k = \"sk_live_TESTKEY\"\n  return 0\nend\n",
    )
    .unwrap();
    let out = rynixc()
        .args(["security", path.to_str().unwrap(), "--error-format=json"])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "expected blocking findings");
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).expect("json");
    assert_eq!(v["schema"], "rynix.security.v1");
    assert_eq!(v["blocking"], true);
    assert!(v["finding_count"].as_u64().unwrap() >= 1);
}

#[test]
fn scope_defaults_deny_patch_write() {
    let out = rynixc()
        .args(["scope", "--error-format=json"])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{:?}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).expect("json");
    assert_eq!(v["schema"], "rynix.scope.v1");
    assert_eq!(v["permissions"]["patch_write"], false);
}

#[test]
fn patch_write_denied_without_scope() {
    let path = example("01_hello.ryx");
    let out = rynixc()
        .args(["patch", path.to_str().unwrap(), "--write"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("denied") || err.contains("patch_write"), "{err}");
}

#[test]
fn deps_resolves_path_package() {
    let root = repo_root();
    let app = root.join("testdata/pkg_app");
    let out = rynixc()
        .args([
            "deps",
            app.to_str().unwrap(),
            "--error-format=json",
        ])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).expect("json");
    assert_eq!(v["schema"], "rynix.deps.v1");
    assert_eq!(v["status"], "ok");
    assert_eq!(v["package"], "pkg_app");
    let deps = v["dependencies"].as_array().expect("deps");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0]["name"], "util");
    assert_eq!(deps[0]["kind"], "path");
    assert_eq!(deps[0]["ok"], true);
}

#[test]
fn deps_fails_missing_path() {
    let dir = std::env::temp_dir().join("rynix_deps_missing");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("rynix.toml"),
        r#"
[package]
name = "broken"
entry = "main.ryx"

[dependencies]
ghost = { path = "./nope" }
"#,
    )
    .unwrap();
    std::fs::write(dir.join("main.ryx"), "def main() -> i64\n  return 0\nend\n").unwrap();
    let out = rynixc()
        .args(["deps", dir.to_str().unwrap(), "--error-format=json"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).expect("json");
    assert_eq!(v["status"], "error");
}

#[test]
fn build_fails_broken_path_dep() {
    let dir = std::env::temp_dir().join("rynix_build_deps_missing");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("rynix.toml"),
        r#"
[package]
name = "broken_build"
entry = "main.ryx"

[dependencies]
ghost = { path = "./nope" }
"#,
    )
    .unwrap();
    let main = dir.join("main.ryx");
    std::fs::write(&main, "def main() -> i64\n  return 0\nend\n").unwrap();
    let out = rynixc()
        .args([
            "build",
            main.to_str().unwrap(),
            "-o",
            dir.join("out").to_str().unwrap(),
            "--runtime=portable",
        ])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("path dependency") || err.contains("ghost"),
        "{err}"
    );
}

#[test]
fn dna_emits_schema() {
    let root = repo_root();
    let out = rynixc()
        .args(["dna", root.to_str().unwrap(), "--error-format=json"])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{:?}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).expect("json");
    assert_eq!(v["schema"], "rynix.dna.v1");
    assert!(v["scanned_files"].as_u64().unwrap_or(0) > 0);
    assert!(v["naming"]["function_style"].is_string());
}

#[test]
fn new_scaffolds_package() {
    let parent = std::env::temp_dir().join("rynix_new_parent");
    let _ = std::fs::remove_dir_all(&parent);
    std::fs::create_dir_all(&parent).unwrap();
    let name = "demo_app";
    let out = rynixc()
        .args([
            "new",
            name,
            "--path",
            parent.to_str().unwrap(),
        ])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{:?}", String::from_utf8_lossy(&out.stderr));
    let root = parent.join(name);
    assert!(root.join("rynix.toml").is_file());
    assert!(root.join("src/main.ryx").is_file());
    let check = rynixc()
        .args(["check", root.join("src/main.ryx").to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        check.status.success(),
        "{:?}",
        String::from_utf8_lossy(&check.stderr)
    );
}
