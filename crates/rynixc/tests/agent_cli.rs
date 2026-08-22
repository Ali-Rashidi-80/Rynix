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
    assert!(text.contains("classify"));
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