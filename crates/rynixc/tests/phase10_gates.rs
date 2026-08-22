//! Phase 10 acceptance gates — honest integration tests (no mocked passes).

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rynixc() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_rynixc"));
    c.current_dir(repo_root());
    c
}

#[test]
fn arch_check_passes_on_repo() {
    let root = repo_root();
    let out = rynixc()
        .args(["arch", "check", "--root"])
        .arg(&root)
        .output()
        .expect("spawn rynixc arch check");
    assert!(
        out.status.success(),
        "arch check failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn arch_check_json_schema() {
    let root = repo_root();
    let out = rynixc()
        .args([
            "arch",
            "check",
            "--root",
            root.to_str().expect("utf8"),
            "--error-format=json",
        ])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).expect("json");
    assert_eq!(v["schema"], "rynix.arch.v1");
    assert_eq!(v["status"], "passed");
}

#[test]
fn json_get_i64_example_runs() {
    let root = repo_root();
    let example = root.join("examples/05_http_json.ryx");
    let out_dir = root.join("target/test-json-http");
    std::fs::create_dir_all(&out_dir).ok();
    let exe = out_dir.join(if cfg!(windows) {
        "05_http_json.exe"
    } else {
        "05_http_json"
    });
    let build = rynixc()
        .args([
            "build",
            example.to_str().unwrap(),
            "-o",
            exe.to_str().unwrap(),
            "--runtime=portable",
        ])
        .output()
        .expect("build");
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&exe).output().expect("run");
    assert!(run.status.success(), "run failed");
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("42"), "expected 42 on stdout, got: {stdout}");
}

#[test]
fn http_get_json_i64_sema_and_lower() {
    let root = repo_root();
    let src = root.join("target/phase10_http_check.ryx");
    std::fs::write(
        &src,
        r#"def main() -> i64
  let x = http_get_json_i64("127.0.0.1", 9, "/api", "value")
  return x
end
"#,
    )
    .unwrap();
    let check = rynixc().args(["check", src.to_str().unwrap()]).output().unwrap();
    assert!(
        check.status.success(),
        "check failed:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let ll = rynixc()
        .args(["emit-ll", src.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(ll.status.success(), "emit-ll failed");
    let text = String::from_utf8_lossy(&ll.stdout);
    assert!(
        text.contains("rynix_rt_http_get_json_i64"),
        "missing http runtime call in IR"
    );
}

#[test]
fn suite5_twelve_workloads_checksum_gate() {
    let root = repo_root();
    let py = if cfg!(windows) { "python" } else { "python3" };
    let out = Command::new(py)
        .args([
            "benchmarks/suite5/run_suite5.py",
            "--langs",
            "c,rynix",
            "--json-out",
            "target/suite5/phase10_gate.json",
        ])
        .current_dir(&root)
        .output()
        .expect("suite5");
    assert!(
        out.status.success(),
        "suite5 failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let path = root.join("target/suite5/phase10_gate.json");
    let text = std::fs::read_to_string(path).expect("results json");
    let v: serde_json::Value = serde_json::from_str(&text).expect("parse");
    let rows = v["rows"].as_array().expect("rows");
    let mut c_ok = 0;
    let mut rynix_ok = 0;
    for row in rows {
        let lang = row["lang"].as_str().unwrap_or("");
        let ok = row.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
        if lang == "c" && ok {
            c_ok += 1;
        }
        if lang == "rynix" && ok {
            rynix_ok += 1;
        }
        if (lang == "c" || lang == "rynix") && !ok {
            panic!("checksum mismatch: {row}");
        }
    }
    assert_eq!(c_ok, 12, "expected 12 passing C workloads");
    assert_eq!(rynix_ok, 12, "expected 12 passing Rynix workloads");
}

#[test]
fn cli_lists_phase10_commands() {
    let out = rynixc().arg("--help").output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("lsp-serve"));
    assert!(text.contains("arch check"));
    assert!(text.contains("graph"));
}

#[test]
fn vscode_extension_bundle_exists() {
    let root = repo_root();
    assert!(root.join("editors/vscode/package.json").is_file());
    assert!(root.join("editors/vscode/dist/extension.js").is_file());
    assert!(root.join("editors/vscode/syntaxes/rynix.tmLanguage.json").is_file());
}
