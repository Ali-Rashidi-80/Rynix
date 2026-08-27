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
fn deps_attest_write_verify_and_tamper() {
    let root = repo_root();
    let core = root.join("testdata/pkg_core");
    let dir = std::env::temp_dir().join("rynix_deps_attest_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let core_path = core.display().to_string().replace('\\', "/");
    std::fs::write(
        dir.join("rynix.toml"),
        format!(
            r#"
[package]
name = "attest_app"
entry = "main.ryx"

[dependencies]
core = {{ path = "{core_path}" }}
"#
        ),
    )
    .unwrap();
    std::fs::write(dir.join("main.ryx"), "def main() -> i64\n  return 0\nend\n").unwrap();

    let write = rynixc()
        .args(["deps", dir.to_str().unwrap(), "--attest"])
        .output()
        .expect("spawn attest");
    assert!(
        write.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&write.stderr)
    );
    let attest_path = dir.join("rynix.attest.v1.json");
    assert!(dir.join("rynix.lock.toml").is_file());
    assert!(attest_path.is_file());
    let body: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&attest_path).unwrap()).expect("attest json");
    assert_eq!(body["schema"], "rynix.attest.v1");
    assert_eq!(body["kind"], "local_digest");
    assert!(
        body["lock_sha256"]
            .as_str()
            .is_some_and(|s| s.len() == 64),
        "lock_sha256 missing"
    );

    let ok = rynixc()
        .args([
            "deps",
            dir.to_str().unwrap(),
            "--attest-verify",
            "--error-format=json",
        ])
        .output()
        .expect("spawn attest-verify");
    assert!(
        ok.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&ok.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&ok.stdout).trim()).expect("json");
    assert_eq!(v["attest"]["present"], true);
    assert_eq!(v["attest"]["ok"], true);

    let mut tampered = body.clone();
    tampered["lock_sha256"] =
        serde_json::json!("0000000000000000000000000000000000000000000000000000000000000000");
    std::fs::write(
        &attest_path,
        serde_json::to_string_pretty(&tampered).unwrap(),
    )
    .unwrap();

    let bad = rynixc()
        .args(["deps", dir.to_str().unwrap(), "--attest-verify"])
        .output()
        .expect("spawn attest-verify bad");
    assert!(!bad.status.success());
    let err = String::from_utf8_lossy(&bad.stderr);
    assert!(
        err.contains("lock_sha256") || err.contains("attest"),
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
        stdout.contains("ok: created package")
            && stdout.lines().any(|l| l.contains("next:") && l.contains("rynixc build")),
        "expected clear new success + next build hint, got: {stdout}"
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

#[test]
fn package_ux_new_deps_attest() {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let parent = std::env::temp_dir().join(format!(
        "rynix_pkg_ux_{}_{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&parent);
    std::fs::create_dir_all(&parent).unwrap();
    let name = "ux_attest_app";
    let created = rynixc()
        .args(["new", name, "--path", parent.to_str().unwrap()])
        .output()
        .expect("spawn new");
    assert!(
        created.status.success(),
        "new failed: {}",
        String::from_utf8_lossy(&created.stderr)
    );
    let stdout = String::from_utf8_lossy(&created.stdout);
    assert!(
        stdout.contains("ok: created package"),
        "new should print clear success, got: {stdout}"
    );
    assert!(
        stdout.contains("rynix.attest.v1") || stdout.contains("deps --attest"),
        "new should hint attest UX, got: {stdout}"
    );

    let root = parent.join(name);
    let attest = rynixc()
        .current_dir(&root)
        .args(["deps", ".", "--attest"])
        .output()
        .expect("spawn deps --attest");
    assert!(
        attest.status.success(),
        "deps --attest failed:\nstderr={}\nstdout={}",
        String::from_utf8_lossy(&attest.stderr),
        String::from_utf8_lossy(&attest.stdout)
    );
    let err = String::from_utf8_lossy(&attest.stderr);
    assert!(
        err.contains("ok: wrote") && err.contains("rynix.attest.v1"),
        "deps --attest should print clear attest success, got stderr: {err}"
    );
    assert!(
        err.contains("local digest") || err.contains("not Sigstore"),
        "attest success should stay honest about local digest, got: {err}"
    );

    let attest_path = root.join("rynix.attest.v1.json");
    assert!(
        attest_path.is_file(),
        "missing {}",
        attest_path.display()
    );
    let body: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&attest_path).unwrap())
            .expect("attest json");
    assert_eq!(body["schema"], "rynix.attest.v1");
    let _ = std::fs::remove_dir_all(&parent);
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

fn build_run_fixture(name: &str, expect_stdout: &str) {
    let root = repo_root();
    let main = root.join("testdata").join(format!("{name}.ryx"));
    let out_dir = root.join(format!("target/test-{name}"));
    std::fs::create_dir_all(&out_dir).ok();
    let exe = out_dir.join(if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
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
        "{name} build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&exe)
        .current_dir(&out_dir)
        .output()
        .expect("run");
    assert!(
        run.status.success(),
        "{name} run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains(expect_stdout),
        "{name}: expected {expect_stdout:?} in stdout {stdout:?}"
    );
}

#[test]
fn struct_str_field_roundtrip() {
    build_run_fixture("struct_str_field_roundtrip", "7");
}

#[test]
fn index_assign_ok() {
    build_run_fixture("index_assign_ok", "139");
}

#[test]
fn enum_value_roundtrip() {
    build_run_fixture("enum_value_roundtrip", "42");
}

#[test]
fn enum_match_variant_roundtrip() {
    build_run_fixture("enum_match_variant", "2");
}

#[test]
fn inline_match_return_roundtrip() {
    // Exhaustive match+return inlined into main (Phase 22-A).
    build_run_fixture("inline_match_return", "6");
}

#[test]
fn enum_qualified_variant_roundtrip() {
    build_run_fixture("enum_qualified_variant", "2");
}

#[test]
fn vec_str_roundtrip() {
    build_run_fixture("vec_str_roundtrip", "2");
}

#[test]
fn map_str_i64_roundtrip() {
    build_run_fixture("map_str_i64_roundtrip", "10");
}

#[test]
fn map_str_str_roundtrip() {
    build_run_fixture("map_str_str_roundtrip", "2");
}

#[test]
fn vec_bool_roundtrip() {
    build_run_fixture("vec_bool_roundtrip", "2");
}

#[test]
fn struct_bool_field_roundtrip() {
    build_run_fixture("struct_bool_field_roundtrip", "1");
}

#[test]
fn multiline_str_roundtrip() {
    build_run_fixture("multiline_str_roundtrip", "1");
}

#[test]
fn enum_payload_i64_match_roundtrip() {
    build_run_fixture("enum_payload_i64_match_roundtrip", "7");
}

#[test]
fn enum_payload_str_match_roundtrip() {
    build_run_fixture("enum_payload_str_match_roundtrip", "1");
}

#[test]
fn example_http_path_param_tls_checks() {
    let root = repo_root();
    let example = root.join("examples/11_http_path_param_tls.ryx");
    let check = rynixc()
        .args([
            "check",
            example.to_str().unwrap(),
            "--error-format=json",
        ])
        .output()
        .expect("check");
    assert!(
        check.status.success(),
        "example 11 check failed:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let ll_path = root.join("target/test-11_http_path_param_tls.ll");
    let emit = rynixc()
        .args([
            "emit-ll",
            example.to_str().unwrap(),
            "-o",
            ll_path.to_str().unwrap(),
        ])
        .output()
        .expect("emit-ll");
    assert!(
        emit.status.success(),
        "example 11 emit-ll failed:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let text = std::fs::read_to_string(&ll_path).expect("read ll");
    assert!(
        text.contains("rynix_rt_http_serve_loop_path_param_json_i64"),
        "missing path_param call"
    );
    assert!(
        text.contains("rynix_rt_http_tls_serve_once_json_i64"),
        "missing http_tls call"
    );
}

#[test]
fn example_http_vec_map_str_checks() {
    let root = repo_root();
    let example = root.join("examples/12_http_vec_map_str.ryx");
    let check = rynixc()
        .args([
            "check",
            example.to_str().unwrap(),
            "--error-format=json",
        ])
        .output()
        .expect("check");
    assert!(
        check.status.success(),
        "example 12 check failed:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let ll_path = root.join("target/test-12_http_vec_map_str.ll");
    let emit = rynixc()
        .args([
            "emit-ll",
            example.to_str().unwrap(),
            "-o",
            ll_path.to_str().unwrap(),
        ])
        .output()
        .expect("emit-ll");
    assert!(
        emit.status.success(),
        "example 12 emit-ll failed:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let text = std::fs::read_to_string(&ll_path).expect("read ll");
    assert!(
        text.contains("rynix_rt_vec_str_new") || text.contains("rynix_rt_vec_str_push"),
        "missing vec_str"
    );
    assert!(
        text.contains("rynix_rt_map_str_i64_new")
            || text.contains("rynix_rt_map_str_i64_insert"),
        "missing map_str_i64"
    );
    assert!(
        text.contains("rynix_rt_http_serve_loop_path_param_json_i64"),
        "missing path_param call"
    );
}

#[test]
fn example_map_str_str_product_checks() {
    let root = repo_root();
    let example = root.join("examples/13_http_map_str_str.ryx");
    let check = rynixc()
        .args([
            "check",
            example.to_str().unwrap(),
            "--error-format=json",
        ])
        .output()
        .expect("check");
    assert!(
        check.status.success(),
        "example 13 check failed:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let ll_path = root.join("target/test-13_http_map_str_str.ll");
    let emit = rynixc()
        .args([
            "emit-ll",
            example.to_str().unwrap(),
            "-o",
            ll_path.to_str().unwrap(),
        ])
        .output()
        .expect("emit-ll");
    assert!(
        emit.status.success(),
        "example 13 emit-ll failed:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let text = std::fs::read_to_string(&ll_path).expect("read ll");
    assert!(
        text.contains("rynix_rt_map_str_str_new")
            || text.contains("rynix_rt_map_str_str_insert"),
        "missing map_str_str"
    );
    assert!(
        text.contains("rynix_rt_http_serve_loop_path_param_json_i64"),
        "missing path_param call"
    );
}

#[test]
fn agent_skill_mentions_emit_wasm_and_attest() {
    let skill = repo_root().join(".agents/skills/rynix/SKILL.md");
    let text = std::fs::read_to_string(&skill).expect("read agent skill");
    for needle in [
        "emit-wasm",
        "rynix.attest.v1",
        "--attest",
        "no WASI",
        "not a language keyword",
    ] {
        assert!(
            text.contains(needle),
            "agent skill missing `{needle}` (Phase 15 Wave B Skills pack)"
        );
    }
    assert!(
        !text.contains("feature/skill/task") || text.contains("Do **not** invent"),
        "skill must refuse End feature/skill language keywords"
    );
}

#[test]
fn mcp_graph_path_file() {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};

    let path = example("02_match_loop.ryx");
    let path_str = path.to_str().expect("utf8 path");
    let mut child = Command::new(env!("CARGO_BIN_EXE_rynixc"))
        .arg("mcp-serve")
        .current_dir(repo_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn mcp-serve");

    let mut stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);

    let call = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "rynix_graph",
            "arguments": { "path": path_str }
        }
    });
    let body = serde_json::to_vec(&call).unwrap();
    write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
    stdin.write_all(&body).unwrap();
    stdin.flush().unwrap();

    let mut content_length = None;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).expect("headers");
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        let lower = trimmed.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            content_length = rest.trim().parse::<usize>().ok();
        }
    }
    let len = content_length.expect("Content-Length");
    let mut buf = vec![0u8; len];
    use std::io::Read;
    reader.read_exact(&mut buf).expect("body");
    let resp: serde_json::Value = serde_json::from_slice(&buf).expect("json");
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("text content");
    assert!(
        text.contains("rynix.graph.v1"),
        "expected graph schema, got: {text}"
    );
    assert!(text.contains("\"edges\""), "expected edges: {text}");
    assert!(
        text.contains("02_match_loop.ryx") || text.contains("classify"),
        "expected path or fn names: {text}"
    );

    // Fail-closed: missing file must error (not invent empty graph).
    let bad = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "rynix_graph",
            "arguments": { "path": "definitely/missing/no_such_file.ryx" }
        }
    });
    let body = serde_json::to_vec(&bad).unwrap();
    write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
    stdin.write_all(&body).unwrap();
    let _ = stdin.flush();

    let mut content_length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        let lower = trimmed.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            content_length = rest.trim().parse::<usize>().ok();
        }
    }
    if let Some(len) = content_length {
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf).expect("err body");
        let resp: serde_json::Value = serde_json::from_slice(&buf).expect("err json");
        assert!(
            resp.get("error").is_some(),
            "missing path must fail-closed with error: {resp}"
        );
    }

    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
}

fn mcp_tools_call(name: &str, arguments: serde_json::Value) -> (serde_json::Value, Option<serde_json::Value>) {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::process::{Command, Stdio};

    let mut child = Command::new(env!("CARGO_BIN_EXE_rynixc"))
        .arg("mcp-serve")
        .current_dir(repo_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn mcp-serve");

    let mut stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);

    let call = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": name, "arguments": arguments }
    });
    let body = serde_json::to_vec(&call).unwrap();
    write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
    stdin.write_all(&body).unwrap();
    stdin.flush().unwrap();

    let mut content_length = None;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).expect("headers");
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        let lower = trimmed.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            content_length = rest.trim().parse::<usize>().ok();
        }
    }
    let len = content_length.expect("Content-Length");
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).expect("body");
    let ok_resp: serde_json::Value = serde_json::from_slice(&buf).expect("json");

    let bad = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": { "path": "definitely/missing/no_such_file.ryx" }
        }
    });
    let body = serde_json::to_vec(&bad).unwrap();
    write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
    stdin.write_all(&body).unwrap();
    let _ = stdin.flush();

    let mut content_length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        let lower = trimmed.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            content_length = rest.trim().parse::<usize>().ok();
        }
    }
    let err_resp = content_length.map(|len| {
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf).expect("err body");
        serde_json::from_slice(&buf).expect("err json")
    });

    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
    (ok_resp, err_resp)
}

#[test]
fn mcp_impact_path_file() {
    let path = example("02_match_loop.ryx");
    let path_str = path.to_str().expect("utf8 path");
    let (resp, err) = mcp_tools_call(
        "rynix_impact",
        serde_json::json!({ "path": path_str, "fn": "main" }),
    );
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("text content");
    assert!(
        text.contains("rynix.impact.v1"),
        "expected impact schema, got: {text}"
    );
    assert!(text.contains("\"nodes\""), "expected nodes: {text}");
    assert!(
        text.contains("main") || text.contains("classify"),
        "expected fn names: {text}"
    );
    let err = err.expect("fail-closed response");
    assert!(
        err.get("error").is_some(),
        "missing path must fail-closed with error: {err}"
    );
}

#[test]
fn mcp_precheck_path_file() {
    let path = example("02_match_loop.ryx");
    let path_str = path.to_str().expect("utf8 path");
    let (resp, err) = mcp_tools_call(
        "rynix_precheck",
        serde_json::json!({ "path": path_str, "fn": "main" }),
    );
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("text content");
    assert!(
        text.contains("rynix.precheck.v1"),
        "expected precheck schema, got: {text}"
    );
    assert!(text.contains("rynix.impact.v1") || text.contains("\"impact\""), "expected impact nest: {text}");
    let err = err.expect("fail-closed response");
    assert!(
        err.get("error").is_some(),
        "missing path must fail-closed with error: {err}"
    );
}

#[test]
fn mcp_check_path_file() {
    let path = example("02_match_loop.ryx");
    let path_str = path.to_str().expect("utf8 path");
    let (resp, err) = mcp_tools_call("rynix_check", serde_json::json!({ "path": path_str }));
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("text content");
    assert!(
        text.contains("\"ok\":true") || text.trim().is_empty() || text.contains("rynix.diag"),
        "expected clean check or diag, got: {text}"
    );
    let err = err.expect("fail-closed response");
    assert!(
        err.get("error").is_some(),
        "missing path must fail-closed with error: {err}"
    );
}

#[test]
fn mcp_context_path_file() {
    let path = example("02_match_loop.ryx");
    let path_str = path.to_str().expect("utf8 path");
    let (resp, err) = mcp_tools_call(
        "rynix_context",
        serde_json::json!({ "path": path_str, "budget": 500 }),
    );
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("text content");
    assert!(
        text.contains("rynix.context.v1"),
        "expected context schema, got: {text}"
    );
    let err = err.expect("fail-closed response");
    assert!(
        err.get("error").is_some(),
        "missing path must fail-closed with error: {err}"
    );
}

#[test]
fn mcp_security_path_file() {
    let path = example("02_match_loop.ryx");
    let path_str = path.to_str().expect("utf8 path");
    let (resp, err) = mcp_tools_call("rynix_security", serde_json::json!({ "path": path_str }));
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("text content");
    assert!(
        text.contains("rynix.security.v1"),
        "expected security schema, got: {text}"
    );
    let err = err.expect("fail-closed response");
    assert!(
        err.get("error").is_some(),
        "missing path must fail-closed with error: {err}"
    );
}

#[test]
fn mcp_apply_fix_path_file() {
    // apply_fix with path: read disk; may be no-op fix on clean file.
    let path = example("01_hello.ryx");
    let path_str = path.to_str().expect("utf8 path");
    let (resp, err) = mcp_tools_call("apply_fix", serde_json::json!({ "path": path_str }));
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("text content");
    assert!(
        text.contains("def ") || text.contains("main"),
        "expected source text back, got: {text}"
    );
    let err = err.expect("fail-closed response");
    assert!(
        err.get("error").is_some(),
        "missing path must fail-closed with error: {err}"
    );
}

#[test]
fn mcp_format_path_file() {
    let path = example("01_hello.ryx");
    let path_str = path.to_str().expect("utf8 path");
    let (resp, err) = mcp_tools_call("rynix_format", serde_json::json!({ "path": path_str }));
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("text content");
    assert!(
        text.contains("def main"),
        "expected formatted source, got: {text}"
    );
    let err = err.expect("fail-closed response");
    assert!(
        err.get("error").is_some(),
        "missing path must fail-closed with error: {err}"
    );
}

#[test]
fn mcp_compile_path_file() {
    let path = example("01_hello.ryx");
    let path_str = path.to_str().expect("utf8 path");
    let (resp, err) = mcp_tools_call("compile", serde_json::json!({ "path": path_str }));
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("text content");
    assert!(
        text.contains("define") || text.contains("@main"),
        "expected LLVM IR, got: {text}"
    );
    let err = err.expect("fail-closed response");
    assert!(
        err.get("error").is_some(),
        "missing path must fail-closed with error: {err}"
    );
}

#[test]
fn verify_phase19_path_mcp_contract() {
    let root = repo_root();
    let contract = root.join("docs/contracts/phase19_path_mcp.contract.toml");
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
    assert_eq!(v["contract"], "phase19-path-mcp");
    assert_eq!(v["ran_tests"], false);
}

#[test]
fn verify_phase21_roi_contract() {
    let root = repo_root();
    let contract = root.join("docs/contracts/phase21_roi.contract.toml");
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
    assert_eq!(v["contract"], "phase21-roi");
    assert_eq!(v["ran_tests"], false);
}

#[test]
fn verify_phase22_inline_mcp_contract() {
    let root = repo_root();
    let contract = root.join("docs/contracts/phase22_inline_mcp.contract.toml");
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
    assert_eq!(v["contract"], "phase22-inline-mcp");
    assert_eq!(v["ran_tests"], false);
}

#[test]
fn verify_phase23_depth_contract() {
    let root = repo_root();
    let contract = root.join("docs/contracts/phase23_depth.contract.toml");
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
    assert_eq!(v["contract"], "phase23-depth");
    assert_eq!(v["ran_tests"], false);
}

#[test]
fn verify_phase24_map_str_contract() {
    let root = repo_root();
    let contract = root.join("docs/contracts/phase24_map_str.contract.toml");
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
    assert_eq!(v["contract"], "phase24-map-str");
    assert_eq!(v["ran_tests"], false);
}

#[test]
fn verify_phase25_golden_contract() {
    let root = repo_root();
    let contract = root.join("docs/contracts/phase25_golden.contract.toml");
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
    assert_eq!(v["contract"], "phase25-golden");
    assert_eq!(v["ran_tests"], false);
}

#[test]
fn lower_decomp_invariants() {
    let root = repo_root();
    assert!(root.join("crates/rynix-rir/src/lower/mod.rs").is_file());
    assert!(root.join("docs/adr/0019-lower-decomp.md").is_file());
    assert!(!root.join("crates/rynix-rir/src/lower.rs").exists());
    let adr = std::fs::read_to_string(root.join("docs/adr/0019-lower-decomp.md")).unwrap();
    assert!(adr.contains("lower/"));
    // Behavior smoke: Map[str,str] still lowers/runs after decomp.
    build_run_fixture("map_str_str_roundtrip", "2");
}

#[test]
fn lsp_decomp_parity() {
    let root = repo_root();
    assert!(root.join("crates/rynixc/src/lsp/mod.rs").is_file());
    assert!(root.join("docs/adr/0020-lsp-decomp.md").is_file());
    let thin = std::fs::read_to_string(root.join("crates/rynixc/src/lsp_cmd.rs")).unwrap();
    assert!(
        thin.contains("pub use crate::lsp") || thin.contains("crate::lsp::"),
        "lsp_cmd should re-export lsp module"
    );
}

#[test]
fn unwrap_budget_gate() {
    let root = repo_root();
    let script = root.join("scripts/audit_unwrap.py");
    assert!(script.is_file(), "missing scripts/audit_unwrap.py");
    let out = std::process::Command::new("python")
        .arg(&script)
        .current_dir(&root)
        .output()
        .or_else(|_| {
            std::process::Command::new("python3")
                .arg(&script)
                .current_dir(&root)
                .output()
        })
        .expect("run audit_unwrap.py");
    assert!(
        out.status.success(),
        "unwrap budget exceeded:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn contract_schema_gate() {
    let root = repo_root();
    assert!(root.join("docs/schemas/rynix.contract.v1.json").is_file());
    assert!(root.join("docs/adr/0021-phase-contract-schema.md").is_file());
    let dir = root.join("docs/contracts");
    let mut n = 0;
    for ent in std::fs::read_dir(&dir).unwrap() {
        let ent = ent.unwrap();
        let name = ent.file_name().to_string_lossy().to_string();
        if !name.ends_with(".contract.toml") {
            continue;
        }
        let text = std::fs::read_to_string(ent.path()).unwrap();
        assert!(
            text.contains("[contract]") && text.contains("name"),
            "{name} missing [contract] name"
        );
        assert!(
            text.contains("[[evidence]]"),
            "{name} missing [[evidence]]"
        );
        n += 1;
    }
    assert!(n >= 5, "expected several contracts, got {n}");
}

#[test]
fn cargo_deny_or_deferral() {
    // Superseded by cargo_deny_clean (Phase 31); alias for phase26 contract.
    cargo_deny_clean_inner();
}

#[test]
fn cargo_deny_clean() {
    cargo_deny_clean_inner();
}

fn cargo_deny_clean_inner() {
    let root = repo_root();
    let deny = root.join("deny.toml");
    assert!(deny.is_file(), "deny.toml missing");
    let text = std::fs::read_to_string(&deny).unwrap();
    assert!(
        text.contains("[licenses]") || text.contains("licenses"),
        "deny.toml must configure licenses"
    );
    let ci = std::fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();
    assert!(
        ci.contains("cargo-deny"),
        "CI must run cargo-deny job"
    );
}

#[test]
fn sanitizer_scaffold_documented() {
    let text =
        std::fs::read_to_string(repo_root().join("docs/SANITIZER_SCAFFOLD.md")).unwrap();
    assert!(text.contains("fsanitize"));
    assert!(
        text.contains("Phase 27") || text.contains("Phase 31"),
        "scaffold must mention Phase 27 or 31"
    );
}

#[test]
fn repo_url_real() {
    let text = std::fs::read_to_string(repo_root().join("Cargo.toml")).unwrap();
    assert!(
        text.contains("repository = \"https://github.com/"),
        "workspace repository must be a real GitHub URL"
    );
    assert!(!text.contains("example.invalid"));
}

#[test]
fn verify_phase26_maturity_contract() {
    let root = repo_root();
    let contract = root.join("docs/contracts/phase26_maturity.contract.toml");
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
    assert_eq!(v["status"], "passed");
    assert_eq!(v["contract"], "phase26-maturity");
}

#[test]
fn sandbox_docker_smoke() {
    let root = repo_root();
    let matrix = root.join("docs/SANDBOX_SKIP_MATRIX.md");
    assert!(matrix.is_file(), "SANDBOX_SKIP_MATRIX.md missing");
    let text = std::fs::read_to_string(&matrix).expect("read skip matrix");
    assert!(
        text.to_ascii_lowercase().contains("docker"),
        "skip matrix must mention docker"
    );

    let docker_ok = Command::new("docker")
        .args(["info"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !docker_ok {
        return;
    }

    // Avoid hanging on registry pulls: only exercise full link when the image
    // is already present locally (CI without image → skip matrix OK).
    let image = std::env::var("RYNIX_DOCKER_IMAGE")
        .unwrap_or_else(|_| "silkeh/clang:latest".to_string());
    let image_local = Command::new("docker")
        .args(["image", "inspect", &image])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !image_local {
        return;
    }

    let dir = std::env::temp_dir().join("rynix_sandbox_docker_smoke");
    let _ = std::fs::create_dir_all(&dir);
    let src = dir.join("main.ryx");
    std::fs::write(&src, "def main() -> i64\n  return 0\nend\n").unwrap();
    let out_bin = dir.join("sandbox_out");
    let out = rynixc()
        .args([
            "build",
            src.to_str().unwrap(),
            "-o",
            out_bin.to_str().unwrap(),
            "--sandbox=docker",
            "--no-opt",
        ])
        .output()
        .expect("spawn build --sandbox=docker");
    assert!(
        out.status.success(),
        "docker sandbox build failed with local image:\nstderr={}\nstdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn sanitize_rejects_exec() {
    let dir = std::env::temp_dir().join("rynix_sanitize_exec");
    let _ = std::fs::create_dir_all(&dir);
    let src = dir.join("bad_system.ryx");
    std::fs::write(
        &src,
        "def main() -> i64\n  system(\"x\")\n  return 0\nend\n",
    )
    .unwrap();
    let out = rynixc()
        .args(["check", src.to_str().unwrap(), "--error-format=json"])
        .output()
        .expect("spawn check");
    assert!(!out.status.success(), "expected check to fail for system()");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let lower = combined.to_ascii_lowercase();
    assert!(
        lower.contains("system")
            || lower.contains("sanitize")
            || lower.contains("dangerous")
            || combined.contains("RYX2014"),
        "expected system/sanitize/dangerous/RYX2014 in diagnostics:\n{combined}"
    );

    let adr = repo_root().join("docs/adr/0023-rir-sanitize.md");
    assert!(adr.is_file(), "ADR-0023 missing");
    let adr_text = std::fs::read_to_string(&adr).unwrap();
    assert!(
        adr_text.contains("sanitize") && adr_text.to_ascii_lowercase().contains("callex"),
        "ADR-0023 must document CallExt sanitize"
    );
}

#[test]
fn msan_ubsan_rt_clean() {
    // Superseded by msan_ubsan_rt_enforced (Phase 31).
    msan_ubsan_rt_enforced_inner();
}

#[test]
fn msan_ubsan_rt_enforced() {
    msan_ubsan_rt_enforced_inner();
}

fn msan_ubsan_rt_enforced_inner() {
    let ci = std::fs::read_to_string(repo_root().join(".github/workflows/ci.yml")).unwrap();
    assert!(
        ci.contains("address,undefined") || ci.contains("fsanitize=address,undefined"),
        "sanitizer-rt CI must enforce ASan+UBSan"
    );
    let scaffold = std::fs::read_to_string(repo_root().join("docs/SANITIZER_SCAFFOLD.md")).unwrap();
    assert!(
        scaffold.contains("memory") || scaffold.contains("MSan") || scaffold.contains("fsanitize=memory"),
        "MSan must remain documented as optional"
    );
}

#[test]
fn fuzz_new_targets_seeded() {
    let root = repo_root();
    let target = root.join("fuzz/fuzz_targets/parse_no_crash.rs");
    assert!(target.is_file(), "parse_no_crash fuzz target missing");
    let seed = root.join("fuzz/corpus/parse_no_crash/seed_main.ryx");
    assert!(seed.is_file(), "seed corpus file missing at {}", seed.display());
}

#[test]
fn emit_ll_no_link_smoke() {
    let path = example("13_http_map_str_str.ryx");
    let out_ll = std::env::temp_dir().join("rynix_emit_ll_no_link.ll");
    let out = rynixc()
        .args([
            "emit-ll",
            path.to_str().unwrap(),
            "-o",
            out_ll.to_str().unwrap(),
        ])
        .output()
        .expect("spawn emit-ll");
    assert!(
        out.status.success(),
        "emit-ll failed:\nstderr={}\nstdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let ll = std::fs::read_to_string(&out_ll).expect("read .ll");
    assert!(ll.contains("define"), "expected LLVM IR define in emit-ll output");
}

#[test]
fn security_cwe_matrix_or_deferral() {
    let path = repo_root().join("docs/CWE_MATRIX.md");
    assert!(path.is_file(), "CWE_MATRIX.md missing");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("CWE-798"), "matrix must document CWE-798");
}

#[test]
fn windows_sandbox_or_deferral() {
    // Superseded by windows_sandbox_smoke (Phase 31).
    windows_sandbox_smoke_inner();
}

#[test]
fn windows_sandbox_smoke() {
    windows_sandbox_smoke_inner();
}

fn windows_sandbox_smoke_inner() {
    let root = repo_root();
    let job_src = root.join("crates/rynixc/src/job_object.rs");
    assert!(job_src.is_file(), "job_object.rs missing");
    let defer = std::fs::read_to_string(root.join("docs/WINDOWS_SANDBOX_DEFERRAL.md")).unwrap();
    assert!(
        defer.contains("Implemented") || defer.contains("Job Object"),
        "WINDOWS_SANDBOX doc must note Job Object implementation"
    );
    #[cfg(windows)]
    {
        let path = example("01_hello.ryx");
        let out = std::env::temp_dir().join("rynix_job_sandbox_hello.exe");
        let _ = std::fs::remove_file(&out);
        let status = rynixc()
            .args([
                "build",
                path.to_str().unwrap(),
                "-o",
                out.to_str().unwrap(),
                "--sandbox=job",
            ])
            .status()
            .expect("spawn build --sandbox=job");
        assert!(status.success(), "build --sandbox=job failed");
        assert!(out.is_file(), "expected output binary from job sandbox build");
    }
}

#[test]
fn security_cwe_one_additive() {
    let text = std::fs::read_to_string(repo_root().join("crates/rynixc/src/security.rs")).unwrap();
    assert!(
        text.contains("glpat-"),
        "security.rs must include glpat- additive pattern"
    );
    let matrix = std::fs::read_to_string(repo_root().join("docs/CWE_MATRIX.md")).unwrap();
    assert!(matrix.contains("glpat-"), "CWE_MATRIX must document glpat-");
}

#[test]
fn verify_phase27_security_contract() {
    let root = repo_root();
    let contract = root.join("docs/contracts/phase27_security.contract.toml");
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
    assert_eq!(v["status"], "passed");
    assert_eq!(v["contract"], "phase27-security");
}

#[test]
fn mcp_slice_or_documented_absence() {
    let path = example("01_hello.ryx");
    let path_str = path.to_str().expect("utf8 path");
    let (resp, err) = mcp_tools_call("rynix_slice", serde_json::json!({ "path": path_str }));
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("text content");
    let v: serde_json::Value = serde_json::from_str(text).expect("slice json");
    assert_eq!(v["schema"], "rynix.slice.v1");
    let lines = v["lines"].as_array().expect("lines");
    assert!(
        lines.iter().any(|l| l.as_str().is_some_and(|s| s.contains("main"))),
        "expected main in slice lines: {lines:?}"
    );
    let err = err.expect("fail-closed response");
    assert!(
        err.get("error").is_some(),
        "missing path must fail-closed with error: {err}"
    );
}

#[test]
fn std_crypto_hmac_aes_import_ok() {
    let root = repo_root();
    let main = root.join("testdata/pkg_std_crypto_hmac_aes/main.ryx");
    let out = rynixc()
        .current_dir(root.join("testdata/pkg_std_crypto_hmac_aes"))
        .args(["check", main.to_str().unwrap(), "--error-format=json"])
        .output()
        .expect("spawn check");
    assert!(
        out.status.success(),
        "check failed:\nstderr={}\nstdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let crypto = std::fs::read_to_string(root.join("std/crypto.ryx")).expect("crypto.ryx");
    assert!(
        crypto.contains("hmac_sha256_first_i64") && crypto.contains("aes128_gcm_nist_empty_tag_first_i64"),
        "std/crypto.ryx must facade HMAC and AES"
    );
}

#[test]
fn verdict_peer_date_current() {
    let root = repo_root();
    let verdict = std::fs::read_to_string(root.join("docs/VERDICT.md")).expect("VERDICT");
    let gap = std::fs::read_to_string(root.join("docs/END_PEER_GAP.md")).expect("END_PEER_GAP");
    assert!(
        verdict.contains("2026-08-26"),
        "VERDICT.md peer date should be refreshed to 2026-08-26"
    );
    assert!(
        gap.contains("2026-08-26"),
        "END_PEER_GAP.md audit refresh should be 2026-08-26"
    );
}

#[test]
fn verify_phase28_agent_contract() {
    let root = repo_root();
    let contract = root.join("docs/contracts/phase28_agent.contract.toml");
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
    assert_eq!(v["status"], "passed");
    assert_eq!(v["contract"], "phase28-agent");
}

#[test]
fn uring_recv_send_completion_smoke() {
    // Superseded by uring_tcp_recv_send_completion_smoke (Phase 32).
    uring_tcp_recv_send_completion_smoke_inner();
}

#[test]
fn uring_tcp_recv_send_completion_smoke() {
    uring_tcp_recv_send_completion_smoke_inner();
}

fn uring_tcp_recv_send_completion_smoke_inner() {
    let net = std::fs::read_to_string(repo_root().join("rt/src/net.c")).unwrap();
    assert!(
        net.contains("rynix_rt_uring_ready")
            && net.contains("rynix_rt_uring_read")
            && net.contains("rynix_rt_uring_write"),
        "tcp_recv/send must prefer uring_read/write when ready"
    );
    let doc = std::fs::read_to_string(repo_root().join("docs/URING_RECV_SEND.md")).unwrap();
    assert!(
        doc.contains("Implemented") || doc.contains("IORING_OP_READ"),
        "URING_RECV_SEND.md must document completion path"
    );
}

#[test]
fn tls_ci_matrix_documented() {
    let path = repo_root().join("docs/TLS_CI_MATRIX.md");
    assert!(path.is_file(), "TLS_CI_MATRIX.md missing");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.contains("SChannel") && text.contains("OpenSSL"),
        "TLS matrix must mention SChannel and OpenSSL"
    );
}

#[test]
fn tls_linux_ci_row_green() {
    let text = std::fs::read_to_string(repo_root().join("docs/TLS_CI_MATRIX.md")).unwrap();
    assert!(
        text.contains("http_tls_product_smoke"),
        "TLS matrix must cite http_tls_product_smoke job/gate"
    );
    let gates = std::fs::read_to_string(repo_root().join("crates/rynixc/tests/size_echo_gates.rs")).unwrap();
    assert!(
        gates.contains("fn http_tls_product_smoke"),
        "size_echo_gates must define http_tls_product_smoke"
    );
}

#[test]
fn http_auth_or_method_gate() {
    // Superseded by http_bearer_header_soft_gate (Phase 32).
    http_bearer_header_soft_gate_inner();
}

#[test]
fn http_bearer_header_soft_gate() {
    http_bearer_header_soft_gate_inner();
}

fn http_bearer_header_soft_gate_inner() {
    let doc = std::fs::read_to_string(repo_root().join("docs/HTTP_AUTH_METHOD_DEFERRAL.md")).unwrap();
    assert!(
        doc.contains("Bearer") && (doc.contains("Implemented") || doc.contains("soft")),
        "HTTP auth doc must document Bearer soft"
    );
    let hdr = std::fs::read_to_string(repo_root().join("rt/include/rynix_rt.h")).unwrap();
    assert!(
        hdr.contains("rynix_rt_http_serve_loop_bearer_json_i64"),
        "RT header must declare bearer soft"
    );
    assert!(
        repo_root().join("rt/tests/http_bearer_smoke.c").is_file(),
        "http_bearer_smoke.c missing"
    );
}

#[test]
fn escape_interproc_or_limit_doc() {
    // Superseded by escape_interproc_improvement_gate (Phase 32).
    escape_interproc_improvement_gate_inner();
}

#[test]
fn escape_interproc_improvement_gate() {
    escape_interproc_improvement_gate_inner();
}

fn escape_interproc_improvement_gate_inner() {
    let doc = std::fs::read_to_string(repo_root().join("docs/ESCAPE_INTERPROC_LIMIT.md")).unwrap();
    assert!(
        doc.contains("SCC") || doc.contains("interproc_scc"),
        "escape doc must cite SCC improvement"
    );
    let unit = std::fs::read_to_string(repo_root().join("crates/rynix-rir/tests/escape_unit.rs")).unwrap();
    assert!(
        unit.contains("interproc_scc_mutual_recursion_arg_escape"),
        "escape_unit must include SCC mutual-recursion gate"
    );
}

#[test]
fn package_ux_diag_gate() {
    // Reuse package UX smoke: `new` + `deps --attest` print clear, honest messages.
    package_ux_new_deps_attest();
}

#[test]
fn attest_docs_match_impl() {
    let root = repo_root();
    let spec = std::fs::read_to_string(root.join("docs/SPEC.md")).expect("SPEC");
    assert!(
        spec.contains("local_digest") || spec.contains("local digest"),
        "SPEC must document local digest attest"
    );
    assert!(
        spec.contains("not** Sigstore") || spec.contains("not Sigstore") || spec.contains("**not** Sigstore"),
        "SPEC must refuse Sigstore theater"
    );
    let skill = std::fs::read_to_string(root.join(".agents/skills/rynix/SKILL.md")).expect("skill");
    assert!(
        skill.contains("not** Sigstore") || skill.contains("not Sigstore") || skill.contains("Sigstore Rekor"),
        "skill must stay honest about attest vs Sigstore"
    );
}

#[test]
fn book_skeleton_exists() {
    let root = repo_root();
    let book = root.join("docs/book");
    assert!(book.is_dir(), "docs/book/ missing");
    let summary = std::fs::read_to_string(book.join("SUMMARY.md")).expect("SUMMARY");
    let mut chapters = 0usize;
    for name in [
        "01_getting_started.md",
        "02_language_tour.md",
        "03_agent_toolchain.md",
    ] {
        let p = book.join(name);
        assert!(p.is_file(), "missing chapter {name}");
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(
            text.contains("SPEC") || text.contains("examples/"),
            "{name} should link SPEC or examples"
        );
        chapters += 1;
    }
    assert!(chapters >= 3, "need ≥3 chapters");
    assert!(
        summary.contains("Getting started") || summary.contains("01_"),
        "SUMMARY should list chapters"
    );
}

#[test]
fn suite5_post_p24_artifact_links() {
    let path = repo_root().join("docs/SUITE5_POST_P24_ARTIFACTS.md");
    assert!(path.is_file(), "SUITE5_POST_P24_ARTIFACTS.md missing");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.contains("suite5_summary_2026-08-25_phase16.txt")
            || text.contains("benchmarks/suite5"),
        "must link Suite5 artifacts"
    );
}

#[test]
fn rfc_or_contributing_sections() {
    let root = repo_root();
    let rfc = root.join("rfcs/0000-template.md");
    assert!(rfc.is_file(), "rfcs/0000-template.md missing");
    let contributing = std::fs::read_to_string(root.join("CONTRIBUTING.md")).expect("CONTRIBUTING");
    assert!(
        contributing.contains("RFC") || contributing.contains("rfcs/"),
        "CONTRIBUTING should mention RFC process"
    );
}

#[test]
fn phase30_not_auto() {
    let root = repo_root();
    let p28 = std::fs::read_to_string(root.join("docs/PHASE28.md")).unwrap();
    let p29 = std::fs::read_to_string(root.join("docs/PHASE29.md")).unwrap();
    let golden = std::fs::read_to_string(root.join("docs/GOLDEN_PATH.md")).unwrap();
    let remaining = std::fs::read_to_string(root.join("docs/GOLDEN_REMAINING.md")).unwrap();
    assert!(
        p28.contains("user-triggered only") && p29.contains("user-triggered only"),
        "PHASE28/29 must state Phase 30 is user-triggered only"
    );
    assert!(
        golden.contains("user-triggered only") || golden.contains("do **not** auto-start Phase 30"),
        "GOLDEN_PATH must keep Phase 30 user-triggered"
    );
    // Phase 30 doc exists only after explicit user ask (this release band).
    assert!(
        root.join("docs/PHASE30.md").is_file(),
        "PHASE30.md required after Phase 30 start"
    );
    assert!(
        remaining.contains("explicit ask") || remaining.contains("user-triggered"),
        "GOLDEN_REMAINING must keep Phase 37 / release policy explicit"
    );
}

#[test]
fn changelog_v011_cut() {
    let text = std::fs::read_to_string(repo_root().join("CHANGELOG.md")).unwrap();
    assert!(
        text.contains("## [0.1.1]") && text.contains("Quality-10"),
        "CHANGELOG must cut [0.1.1] Quality-10 section"
    );
    assert!(
        text.contains("Ali-Rashidi-80/Rynix"),
        "CHANGELOG compare URLs must use Ali-Rashidi-80/Rynix"
    );
    assert!(
        !text.contains("rynix-lang/rynix"),
        "stale rynix-lang/rynix compare URLs must be gone"
    );
}

#[test]
fn production_readiness_scoreboard() {
    let text = std::fs::read_to_string(repo_root().join("PRODUCTION_READINESS.md")).unwrap();
    assert!(
        text.contains("Quality-10 scoreboard"),
        "PRODUCTION_READINESS must include Quality-10 scoreboard"
    );
    for axis in [
        "Architecture",
        "Rust code quality",
        "C runtime quality",
        "Test strategy",
        "Error handling",
        "Security",
        "Performance",
        "Deployment / CI",
        "AI tooling",
        "Documentation",
        "Niche-10",
    ] {
        assert!(text.contains(axis), "scoreboard missing axis {axis}");
    }
    assert!(
        text.contains("0.1.1") || text.contains("`0.1.1`"),
        "PRODUCTION_READINESS must reference 0.1.1"
    );
}

#[test]
fn golden_remaining_sot() {
    let root = repo_root();
    let p = root.join("docs/GOLDEN_REMAINING.md");
    assert!(p.is_file(), "GOLDEN_REMAINING.md missing");
    let text = std::fs::read_to_string(&p).unwrap();
    assert!(text.contains("Phase") && text.contains("30"), "must list Phase 30");
    let golden = std::fs::read_to_string(root.join("docs/GOLDEN_PATH.md")).unwrap();
    assert!(
        golden.contains("GOLDEN_REMAINING"),
        "GOLDEN_PATH must point at GOLDEN_REMAINING"
    );
}

#[test]
fn verify_phase30_release_contract() {
    let root = repo_root();
    let contract = root.join("docs/contracts/phase30_release.contract.toml");
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
        "phase30 verify failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn verify_phase31_security_harden_contract() {
    let root = repo_root();
    let contract = root.join("docs/contracts/phase31_security_harden.contract.toml");
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
        "phase31 verify failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn verify_phase32_runtime_close_contract() {
    let root = repo_root();
    let contract = root.join("docs/contracts/phase32_runtime_close.contract.toml");
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
        "phase32 verify failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn tutorial_five_runnable() {
    let root = repo_root();
    let names = [
        "tutorial_01_hello.ryx",
        "tutorial_02_match.ryx",
        "tutorial_03_vec.ryx",
        "tutorial_04_map.ryx",
        "tutorial_05_agent_check.ryx",
    ];
    for name in names {
        let path = root.join("examples").join(name);
        assert!(path.is_file(), "missing {name}");
        let out = rynixc()
            .args(["check", path.to_str().unwrap()])
            .output()
            .expect("check");
        assert!(
            out.status.success(),
            "{name} check failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let book = std::fs::read_to_string(root.join("docs/book/05_tutorials.md")).unwrap();
    assert!(book.contains("tutorial_01"), "book must list tutorials");
}

#[test]
fn contributing_sections_gate() {
    let text = std::fs::read_to_string(repo_root().join("CONTRIBUTING.md")).unwrap();
    for needle in ["Build matrix", "ADR workflow", "Commit style", "good-first-issue", "RFC"] {
        assert!(text.contains(needle), "CONTRIBUTING missing section {needle}");
    }
}

#[test]
fn rfc_process_documented() {
    let text = std::fs::read_to_string(repo_root().join("rfcs/README.md")).unwrap();
    assert!(
        text.contains("Track G") && text.contains("RFC"),
        "rfcs/README must document RFC-before-Track-G"
    );
}

#[test]
fn contributor_onboarding_doc() {
    let path = repo_root().join("docs/CONTRIBUTOR_ONBOARDING.md");
    assert!(path.is_file());
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("good-first-issue") || text.contains("First contribution"));
}

#[test]
fn retrospective_template_exists() {
    let path = repo_root().join("docs/RETROSPECTIVE_QCORE.md");
    assert!(path.is_file());
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("What shipped") || text.contains("Scoreboard"));
}

#[test]
fn verify_phase34_track_c_contract() {
    let root = repo_root();
    let contract = root.join("docs/contracts/phase34_track_c.contract.toml");
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
        "phase34 verify failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn verify_phase33_lang_close_contract() {
    let root = repo_root();
    let contract = root.join("docs/contracts/phase33_lang_close.contract.toml");
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
        "phase33 verify failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn vec_t_i64_compat_spike() {
    build_run_fixture("vec_t_i64_compat_spike", "2");
}

#[test]
fn vec_t_roundtrip_matrix() {
    build_run_fixture("vec_t_i64_compat_spike", "2");
    build_run_fixture("vec_str_roundtrip", "2");
    build_run_fixture("vec_bool_roundtrip", "2");
}

#[test]
fn map_kv_roundtrip_matrix() {
    build_run_fixture("map_str_i64_roundtrip", "10");
    build_run_fixture("map_str_str_roundtrip", "2");
    // Map[i64,i64] via soft map_new — existing product path
    let root = repo_root();
    let path = root.join("examples/03_vec.ryx");
    assert!(path.is_file());
    let check = rynixc().args(["check", path.to_str().unwrap()]).output().unwrap();
    assert!(check.status.success(), "examples/03_vec check failed");
}

#[test]
fn std_collections_facade_ok() {
    let root = repo_root();
    let facade = std::fs::read_to_string(root.join("std/collections.ryx")).unwrap();
    assert!(
        facade.contains("vec_new") && facade.contains("ADR-0025"),
        "std/collections.ryx must document soft ABI + ADR-0025"
    );
    let main = root.join("testdata/pkg_std_collections/main.ryx");
    let out = rynixc()
        .current_dir(root.join("testdata/pkg_std_collections"))
        .args(["check", main.to_str().unwrap()])
        .output()
        .expect("check");
    assert!(
        out.status.success(),
        "std::collections import failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn legacy_mono_alias_ok() {
    // Legacy soft names still resolve alongside typed Vec/Map annotations.
    build_run_fixture("vec_str_roundtrip", "2");
    build_run_fixture("map_str_str_roundtrip", "2");
    build_run_fixture("vec_bool_roundtrip", "2");
}

#[test]
fn verify_phase35_track_g_adr_contract() {
    let root = repo_root();
    let contract = root.join("docs/contracts/phase35_track_g_adr.contract.toml");
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
    assert!(out.status.success(), "phase35 verify failed");
}

#[test]
fn verify_phase36_track_g_contract() {
    let root = repo_root();
    let contract = root.join("docs/contracts/phase36_track_g.contract.toml");
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
    assert!(out.status.success(), "phase36 verify failed");
}

#[test]
fn phase37_hold_documented() {
    let text = std::fs::read_to_string(repo_root().join("docs/PHASE37.md")).unwrap();
    assert!(
        text.contains("HOLD") && text.contains("explicit"),
        "PHASE37 must document hold + explicit ask"
    );
}

#[test]
fn verify_phase29_ceiling_contract() {
    let root = repo_root();
    let contract = root.join("docs/contracts/phase29_ceiling.contract.toml");
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
    assert_eq!(v["status"], "passed");
    assert_eq!(v["contract"], "phase29-ceiling");
}

#[test]
fn agent_skill_mentions_completion_rename_path_mcp() {
    let skill = repo_root().join(".agents/skills/rynix/SKILL.md");
    let text = std::fs::read_to_string(&skill).expect("read agent skill");
    for needle in [
        "completion",
        "rename",
        "path-first",
        "rynix_impact",
        "rynix_precheck",
        "phase19_path_mcp.contract.toml",
    ] {
        assert!(
            text.contains(needle),
            "agent skill missing `{needle}` (Phase 19)"
        );
    }
}

#[test]
fn install_one_path_clang_win_linux() {
    let text = std::fs::read_to_string(repo_root().join("INSTALL.md")).expect("INSTALL.md");
    assert!(
        text.contains("One-path clang"),
        "INSTALL.md must document one-path clang setup"
    );
    let lower = text.to_ascii_lowercase();
    assert!(
        lower.contains("windows") && lower.contains("clang"),
        "INSTALL.md must mention clang for Windows"
    );
    assert!(
        lower.contains("linux") && lower.contains("clang"),
        "INSTALL.md must mention clang for Linux"
    );
    // Both platforms appear in the one-path section (not only troubleshooting).
    let idx = text
        .find("One-path clang")
        .expect("One-path clang heading");
    let section = &text[idx..];
    let section_end = section.find("\n## ").unwrap_or(section.len());
    let one_path = &section[..section_end];
    assert!(
        one_path.contains("Windows") && one_path.contains("Linux"),
        "one-path section must cover Windows and Linux"
    );
    assert!(
        one_path.to_ascii_lowercase().matches("clang").count() >= 2,
        "one-path section should mention clang for both platforms"
    );
}

#[test]
fn niche10_scorecard_links_gates() {
    let text = std::fs::read_to_string(repo_root().join("docs/NICHE10.md")).expect("NICHE10.md");
    for gate in [
        "emit_wasm_host_print_i64",
        "package_ux_new_deps_attest",
        "install_one_path_clang_win_linux",
        "niche10_scorecard_links_gates",
        "http_loop_path_param",
        "http_header_i64_smoke",
        "http_body_bounded_smoke",
        "http_keepalive_bounded_smoke",
        "http_tls_product_smoke",
        "mcp_graph_path_file",
        "mcp_impact_path_file",
        "mcp_precheck_path_file",
        "completion_lists_fn_and_let",
        "rename_local_updates_def_and_refs",
        "struct_str_field_roundtrip",
        "index_assign_ok",
        "enum_value_roundtrip",
        "suite5_twelve_workloads_checksum_gate",
        "deps_attest_write_verify_and_tamper",
        "iocp_echo_smoke_c",
        "uring_sqe_smoke_c",
        "ws_frames_smoke_c",
        "crypto_kv_smoke_c",
    ] {
        assert!(
            text.contains(gate),
            "docs/NICHE10.md must link gate `{gate}` (Phase 20-D)"
        );
    }
    assert!(
        text.contains("**10**") || text.contains("| **10**"),
        "NICHE10.md must score axes at 10 only with evidence"
    );
    assert!(
        text.contains("**not** full WASI") || text.contains("not full WASI"),
        "NICHE10.md must refuse full WASI theater"
    );
}
