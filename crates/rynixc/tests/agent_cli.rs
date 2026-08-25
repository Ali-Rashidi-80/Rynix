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
fn verify_manifest_build_evidence() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("root");
    let contract = root.join("docs/contracts/wave12_manifest.contract.toml");
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
    assert!(
        out.status.success(),
        "verify failed:\nstderr={}\nstdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).expect("json");
    assert_eq!(v["schema"], "rynix.verify.v1");
    assert_eq!(v["status"], "passed");
    assert_eq!(v["contract"], "wave12-manifest-build");
    assert_eq!(v["ran_tests"], false);
    assert!(v["passed"].as_u64().unwrap_or(0) >= 4);
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
    assert_eq!(deps.len(), 2, "transitive: core then util");
    assert_eq!(deps[0]["name"], "core");
    assert_eq!(deps[0]["kind"], "path");
    assert_eq!(deps[0]["ok"], true);
    assert_eq!(deps[1]["name"], "util");
    assert_eq!(deps[1]["kind"], "path");
    assert_eq!(deps[1]["ok"], true);
    assert_eq!(v["lock"]["present"], false);
    assert_eq!(v["lock"]["ok"], true);
}

#[test]
fn deps_resolves_local_registry_version() {
    let root = repo_root();
    let app = root.join("testdata/pkg_reg_app");
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
    assert_eq!(v["package"], "pkg_reg_app");
    assert_eq!(v["registry_index"], "scan");
    assert!(v["registry"].as_str().unwrap().contains("vendor"));
    let deps = v["dependencies"].as_array().expect("deps");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0]["name"], "util");
    assert_eq!(deps[0]["kind"], "registry");
    assert_eq!(deps[0]["version"], "0.1.0");
    assert_eq!(deps[0]["ok"], true);
}

#[test]
fn deps_resolves_sparse_local_index() {
    let root = repo_root();
    let app = root.join("testdata/pkg_sparse_app");
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
    assert_eq!(v["package"], "pkg_sparse_app");
    assert_eq!(v["registry_index"], "sparse");
    let deps = v["dependencies"].as_array().expect("deps");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0]["name"], "util");
    assert_eq!(deps[0]["kind"], "registry");
    assert_eq!(deps[0]["index"], "sparse");
    assert_eq!(deps[0]["version"], "0.2.0");
    assert_eq!(deps[0]["ok"], true);
    let path = deps[0]["path"].as_str().unwrap_or("");
    assert!(
        !path.contains("0.9.0") && !path.contains("0.8.0"),
        "sparse must ignore unlisted/yanked dirs, got {path}"
    );
}

#[test]
fn build_pkg_sparse_app_resolves_index() {
    let root = repo_root();
    let main = root.join("testdata/pkg_sparse_app/main.ryx");
    let out_dir = root.join("target/test-pkg-sparse-app");
    std::fs::create_dir_all(&out_dir).ok();
    let exe = out_dir.join(if cfg!(windows) {
        "pkg_sparse_app.exe"
    } else {
        "pkg_sparse_app"
    });
    let build = rynixc()
        .args([
            "build",
            main.to_str().unwrap(),
            "-o",
            exe.to_str().unwrap(),
            "--runtime=portable",
        ])
        .output()
        .expect("spawn build");
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&exe).output().expect("run");
    assert!(run.status.success(), "run failed");
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("42"),
        "expected util 0.2.0 answer=42 (not decoy 99), got: {stdout}"
    );
    assert!(
        !stdout.contains("99") && !stdout.contains("88"),
        "decoy/yanked util must not be linked, got: {stdout}"
    );
}

#[test]
fn build_pkg_reg_app_resolves_registry_deps() {
    let root = repo_root();
    let main = root.join("testdata/pkg_reg_app/main.ryx");
    let out_dir = root.join("target/test-pkg-reg-app");
    std::fs::create_dir_all(&out_dir).ok();
    let exe = out_dir.join(if cfg!(windows) {
        "pkg_reg_app.exe"
    } else {
        "pkg_reg_app"
    });
    let build = rynixc()
        .args([
            "build",
            main.to_str().unwrap(),
            "-o",
            exe.to_str().unwrap(),
            "--runtime=portable",
        ])
        .output()
        .expect("spawn build");
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&exe).output().expect("run");
    assert!(run.status.success(), "run failed");
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("42"),
        "expected util_answer()=42 on stdout, got: {stdout}"
    );
}

#[test]
fn build_pkg_app_calls_path_dep() {
    let root = repo_root();
    let main = root.join("testdata/pkg_app/main.ryx");
    let out_dir = root.join("target/test-pkg-app");
    std::fs::create_dir_all(&out_dir).ok();
    let exe = out_dir.join(if cfg!(windows) {
        "pkg_app.exe"
    } else {
        "pkg_app"
    });
    let build = rynixc()
        .args([
            "build",
            main.to_str().unwrap(),
            "-o",
            exe.to_str().unwrap(),
            "--runtime=portable",
        ])
        .output()
        .expect("spawn build");
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&exe).output().expect("run");
    assert!(run.status.success(), "run failed");
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("42"),
        "expected util_answer()=42 on stdout, got: {stdout}"
    );
}

#[test]
fn emit_ll_pkg_app_includes_dep_fn() {
    let root = repo_root();
    let main = root.join("testdata/pkg_app/main.ryx");
    let out_dir = root.join("target/test-pkg-app-ll");
    std::fs::create_dir_all(&out_dir).ok();
    let ll = out_dir.join("pkg_app.ll");
    let emit = rynixc()
        .args([
            "emit-ll",
            main.to_str().unwrap(),
            "-o",
            ll.to_str().unwrap(),
        ])
        .output()
        .expect("spawn emit-ll");
    assert!(
        emit.status.success(),
        "emit-ll failed:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let text = std::fs::read_to_string(&ll).expect("read .ll");
    // Dep may stay as @util_answer / @core_id or fold to 42.
    assert!(
        text.contains("42")
            || text.contains("util_answer")
            || text.contains("core_id")
            || text.contains("rynix_rt_print_i64"),
        "expected dep call or print in IR, got snippet:\n{}",
        &text[..text.len().min(800)]
    );
}

#[test]
fn build_pkg_import_app_qualified_call() {
    let root = repo_root();
    let main = root.join("testdata/pkg_import_app/main.ryx");
    let out_dir = root.join("target/test-pkg-import-app");
    std::fs::create_dir_all(&out_dir).ok();
    let exe = out_dir.join(if cfg!(windows) {
        "pkg_import_app.exe"
    } else {
        "pkg_import_app"
    });
    let build = rynixc()
        .args([
            "build",
            main.to_str().unwrap(),
            "-o",
            exe.to_str().unwrap(),
            "--runtime=portable",
        ])
        .output()
        .expect("spawn build");
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&exe).output().expect("run");
    assert!(run.status.success(), "run failed");
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("42"),
        "expected util.util_answer()=42, got: {stdout}"
    );
}

#[test]
fn deps_resolves_transitive_core_before_util() {
    let root = repo_root();
    let app = root.join("testdata/pkg_app");
    let out = rynixc()
        .args(["deps", app.to_str().unwrap(), "--error-format=json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).expect("json");
    let deps = v["dependencies"].as_array().expect("deps");
    assert_eq!(deps.len(), 2);
    assert_eq!(deps[0]["name"], "core");
    assert_eq!(deps[1]["name"], "util");
}

#[test]
fn build_pkg_semver_caret_picks_highest() {
    let root = repo_root();
    let main = root.join("testdata/pkg_semver_app/main.ryx");
    let out_dir = root.join("target/test-pkg-semver-app");
    std::fs::create_dir_all(&out_dir).ok();
    let exe = out_dir.join(if cfg!(windows) {
        "pkg_semver_app.exe"
    } else {
        "pkg_semver_app"
    });
    let build = rynixc()
        .args([
            "build",
            main.to_str().unwrap(),
            "-o",
            exe.to_str().unwrap(),
            "--runtime=portable",
        ])
        .output()
        .expect("spawn build");
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&exe).output().expect("run");
    assert!(run.status.success(), "run failed");
    assert!(
        String::from_utf8_lossy(&run.stdout).contains("42"),
        "expected 42 from >=0.1.0 → 0.2.0"
    );
}

#[test]
fn build_pkg_std_app_loads_math() {
    let root = repo_root();
    let main = root.join("testdata/pkg_std_app/main.ryx");
    let out_dir = root.join("target/test-pkg-std-app");
    std::fs::create_dir_all(&out_dir).ok();
    let exe = out_dir.join(if cfg!(windows) {
        "pkg_std_app.exe"
    } else {
        "pkg_std_app"
    });
    let build = rynixc()
        .args([
            "build",
            main.to_str().unwrap(),
            "-o",
            exe.to_str().unwrap(),
            "--runtime=portable",
        ])
        .output()
        .expect("spawn build");
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&exe).output().expect("run");
    assert!(run.status.success(), "run failed");
    assert!(
        String::from_utf8_lossy(&run.stdout).contains("42"),
        "expected 42 from math.add3"
    );
}

#[test]
fn build_fs_via_std_import() {
    let root = repo_root();
    let main = root.join("testdata/pkg_std_fs/main.ryx");
    let out_dir = root.join("target/test-pkg-std-fs");
    std::fs::create_dir_all(&out_dir).ok();
    let exe = out_dir.join(if cfg!(windows) {
        "pkg_std_fs.exe"
    } else {
        "pkg_std_fs"
    });
    let build = rynixc()
        .args([
            "build",
            main.to_str().unwrap(),
            "-o",
            exe.to_str().unwrap(),
            "--runtime=portable",
        ])
        .output()
        .expect("spawn build");
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&exe)
        .current_dir(&out_dir)
        .output()
        .expect("run");
    assert!(
        run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        String::from_utf8_lossy(&run.stdout).contains('0'),
        "expected 0 from fs via std import"
    );
}

#[test]
fn build_crypto_sha_via_std() {
    let root = repo_root();
    let main = root.join("testdata/pkg_std_crypto/main.ryx");
    let out_dir = root.join("target/test-pkg-std-crypto");
    std::fs::create_dir_all(&out_dir).ok();
    let exe = out_dir.join(if cfg!(windows) {
        "pkg_std_crypto.exe"
    } else {
        "pkg_std_crypto"
    });
    let build = rynixc()
        .args([
            "build",
            main.to_str().unwrap(),
            "-o",
            exe.to_str().unwrap(),
            "--runtime=portable",
        ])
        .output()
        .expect("spawn build");
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&exe).output().expect("run");
    assert!(run.status.success(), "run failed");
    assert!(
        String::from_utf8_lossy(&run.stdout).contains('0'),
        "expected 0 from crypto.sha256_first_i64 NIST abc"
    );
}

#[test]
fn deps_reports_multifile_sources() {
    let root = repo_root();
    let app = root.join("testdata/pkg_app");
    let out = rynixc()
        .args(["deps", app.to_str().unwrap(), "--error-format=json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).expect("json");
    let deps = v["dependencies"].as_array().expect("deps");
    let util = deps.iter().find(|d| d["name"] == "util").expect("util");
    let sources = util["sources"].as_array().expect("sources");
    assert_eq!(sources.len(), 2, "entry + extra.ryx");
    let joined = sources
        .iter()
        .map(|s| s.as_str().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("|");
    assert!(joined.contains("lib.ryx"), "{joined}");
    assert!(joined.contains("extra.ryx"), "{joined}");
}

#[test]
fn deps_resolves_workspace_member() {
    let root = repo_root();
    let app = root.join("testdata/ws_monorepo/app");
    let out = rynixc()
        .args(["deps", app.to_str().unwrap(), "--error-format=json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).expect("json");
    assert_eq!(v["package"], "ws_app");
    assert!(v["workspace"].as_str().unwrap().contains("ws_monorepo"));
    let deps = v["dependencies"].as_array().expect("deps");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0]["name"], "util");
    assert_eq!(deps[0]["kind"], "workspace");
    assert_eq!(deps[0]["ok"], true);
}

#[test]
fn build_ws_monorepo_app() {
    let root = repo_root();
    let main = root.join("testdata/ws_monorepo/app/main.ryx");
    let out_dir = root.join("target/test-ws-monorepo");
    std::fs::create_dir_all(&out_dir).ok();
    let exe = out_dir.join(if cfg!(windows) {
        "ws_app.exe"
    } else {
        "ws_app"
    });
    let build = rynixc()
        .args([
            "build",
            main.to_str().unwrap(),
            "-o",
            exe.to_str().unwrap(),
            "--runtime=portable",
        ])
        .output()
        .expect("spawn build");
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&exe).output().expect("run");
    assert!(run.status.success(), "run failed");
    assert!(
        String::from_utf8_lossy(&run.stdout).contains("42"),
        "expected 42 from workspace util"
    );
}

#[test]
fn deps_lock_writes_at_workspace_root() {
    let root = repo_root();
    let app = root.join("testdata/ws_monorepo/app");
    let ws_root = root.join("testdata/ws_monorepo");
    let lock = ws_root.join("rynix.lock.toml");
    let _ = std::fs::remove_file(&lock);
    let out = rynixc()
        .args(["deps", app.to_str().unwrap(), "--lock"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(lock.is_file(), "lock should be at workspace root");
    let _ = std::fs::remove_file(&lock);
}

#[test]
fn deps_lock_write_verify_and_tamper() {
    let root = repo_root();
    let core = root.join("testdata/pkg_core");
    let dir = std::env::temp_dir().join("rynix_deps_lock_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let core_path = core.display().to_string().replace('\\', "/");
    std::fs::write(
        dir.join("rynix.toml"),
        format!(
            r#"
[package]
name = "lock_app"
entry = "main.ryx"

[dependencies]
core = {{ path = "{core_path}" }}
"#
        ),
    )
    .unwrap();
    std::fs::write(dir.join("main.ryx"), "def main() -> i64\n  return 0\nend\n").unwrap();

    let lock_write = rynixc()
        .args(["deps", dir.to_str().unwrap(), "--lock"])
        .output()
        .expect("spawn lock");
    assert!(
        lock_write.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&lock_write.stderr)
    );
    assert!(dir.join("rynix.lock.toml").is_file());

    let locked_ok = rynixc()
        .args([
            "deps",
            dir.to_str().unwrap(),
            "--locked",
            "--error-format=json",
        ])
        .output()
        .expect("spawn locked");
    assert!(
        locked_ok.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&locked_ok.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&locked_ok.stdout).trim()).expect("json");
    assert_eq!(v["lock"]["present"], true);
    assert_eq!(v["lock"]["ok"], true);

    // Tamper lock sha so verify fails.
    let lock_path = dir.join("rynix.lock.toml");
    let lock_text = std::fs::read_to_string(&lock_path).unwrap();
    let mut rebuilt = String::new();
    for line in lock_text.lines() {
        if line.starts_with("sha256 = ") {
            rebuilt.push_str(
                "sha256 = \"0000000000000000000000000000000000000000000000000000000000000000\"\n",
            );
        } else {
            rebuilt.push_str(line);
            rebuilt.push('\n');
        }
    }
    std::fs::write(&lock_path, rebuilt).unwrap();

    let locked_bad = rynixc()
        .args(["deps", dir.to_str().unwrap(), "--locked"])
        .output()
        .expect("spawn locked bad");
    assert!(!locked_bad.status.success());
    let err = String::from_utf8_lossy(&locked_bad.stderr);
    assert!(
        err.contains("sha256") || err.contains("lock"),
        "{err}"
    );
}

#[test]
fn build_fs_roundtrip() {
    let root = repo_root();
    let main = root.join("testdata/fs_roundtrip.ryx");
    let out_dir = root.join("target/test-fs-roundtrip");
    std::fs::create_dir_all(&out_dir).ok();
    let exe = out_dir.join(if cfg!(windows) {
        "fs_roundtrip.exe"
    } else {
        "fs_roundtrip"
    });
    let build = rynixc()
        .args([
            "build",
            main.to_str().unwrap(),
            "-o",
            exe.to_str().unwrap(),
            "--runtime=portable",
        ])
        .output()
        .expect("spawn build");
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&exe)
        .current_dir(&out_dir)
        .output()
        .expect("run");
    assert!(run.status.success(), "run failed: {}", String::from_utf8_lossy(&run.stderr));
    assert!(
        String::from_utf8_lossy(&run.stdout).contains('0'),
        "expected 0 from fs round-trip"
    );
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
fn build_from_manifest_entry() {
    let root = repo_root();
    let app = root.join("testdata/pkg_app");
    let out_dir = root.join("target/test-pkg-app-manifest");
    std::fs::create_dir_all(&out_dir).ok();
    let exe = out_dir.join(if cfg!(windows) {
        "pkg_app_manifest.exe"
    } else {
        "pkg_app_manifest"
    });
    // No file arg; cwd = package dir. Manifest runtime = portable (no --runtime=).
    let build = rynixc()
        .current_dir(&app)
        .args(["build", "-o", exe.to_str().unwrap()])
        .output()
        .expect("spawn build");
    assert!(
        build.status.success(),
        "build from manifest entry failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&exe).output().expect("run");
    assert!(run.status.success(), "run failed");
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("42"),
        "expected util_answer()=42 on stdout, got: {stdout}"
    );
}

#[test]
fn run_from_manifest_entry() {
    let root = repo_root();
    let app = root.join("testdata/pkg_app");
    let out = rynixc()
        .current_dir(&app)
        .args(["run"])
        .output()
        .expect("spawn run");
    assert!(
        out.status.success(),
        "run from manifest failed:\nstderr={}\nstdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("42"),
        "expected util_answer()=42 on stdout, got: {stdout}"
    );
}

#[test]
fn build_missing_entry_diag() {
    let dir = std::env::temp_dir().join("rynix_missing_entry");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("rynix.toml"),
        "[package]\nname = \"no_entry\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let out = rynixc()
        .current_dir(&dir)
        .args(["build", "--error-format=json"])
        .output()
        .expect("spawn build");
    assert!(
        !out.status.success(),
        "expected failure for missing entry"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("[package].entry") || err.contains("missing"),
        "expected entry-missing message, got: {err}"
    );
    let v: serde_json::Value = serde_json::from_str(err.trim()).expect("json resolve error");
    assert!(
        v["error"].as_str().unwrap_or("").contains("entry"),
        "json error should mention entry: {v}"
    );
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
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.lines().any(|l| l.trim() == "next: rynixc build"),
        "expected next: rynixc build, got: {stdout}"
    );
    let build = rynixc()
        .current_dir(&root)
        .args(["build"])
        .output()
        .expect("spawn build");
    assert!(
        build.status.success(),
        "scaffold build without path failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}

/// Phase 12 Wave 1b: negative memory corpus asserts diagnostic *codes* only.
#[test]
fn compile_fail_memory_corpus() {
    let dir = repo_root().join("testdata/compile_fail/memory");
    let cases = [
        ("use_after_move.ryx", "RYX2011"),
        ("pure_violation.ryx", "RYX2012"),
        ("stub_reserved.ryx", "RYX2013"),
    ];
    let entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "ryx"))
        .collect();
    assert_eq!(
        entries.len(),
        3,
        "expected exactly 3 fixtures in {}, found {}",
        dir.display(),
        entries.len()
    );
    for (name, expect_code) in cases {
        let path = dir.join(name);
        assert!(path.is_file(), "missing fixture {}", path.display());
        let out = rynixc()
            .args([
                "check",
                path.to_str().unwrap(),
                "--error-format=json",
            ])
            .output()
            .expect("spawn check");
        assert!(
            !out.status.success(),
            "{} should fail check",
            path.display()
        );
        let err = String::from_utf8_lossy(&out.stderr);
        let codes: Vec<String> = err
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(|line| {
                let v: serde_json::Value = serde_json::from_str(line).ok()?;
                v.get("code")?.as_str().map(|s| s.to_string())
            })
            .collect();
        assert!(
            codes.iter().any(|c| c == expect_code),
            "{}: expected {expect_code} in codes {codes:?};\nstderr={err}",
            path.display()
        );
    }
}

#[test]
fn struct_literal_field() {
    let root = repo_root();
    let main = root.join("testdata/struct_literal_field.ryx");
    let out_dir = root.join("target/test-struct-literal-field");
    std::fs::create_dir_all(&out_dir).ok();
    let exe = out_dir.join(if cfg!(windows) {
        "struct_literal_field.exe"
    } else {
        "struct_literal_field"
    });
    let build = rynixc()
        .args([
            "build",
            main.to_str().unwrap(),
            "-o",
            exe.to_str().unwrap(),
            "--runtime=portable",
        ])
        .output()
        .expect("spawn build");
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&exe)
        .current_dir(&out_dir)
        .output()
        .expect("run");
    assert!(
        run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("12"),
        "expected printed 12 (10+2), got {stdout:?}"
    );
}
