//! Differential: compiled binary exit code vs RIR interpreter for small programs.

use std::path::PathBuf;
use std::process::Command;

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{interpret_module, lower_module, run_pipeline, InterpValue};
use rynix_sema::analyze;
use rynix_span::Interner;

fn clang_present() -> bool {
    for c in ["x86_64-w64-mingw32-clang", "clang", "clang.exe"] {
        if Command::new(c).arg("--version").output().is_ok() {
            return true;
        }
    }
    false
}

fn interp_i64(src: &str) -> i64 {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let mut rir = lower_module(module, &analysis, &mut interner, src, 0);
    assert!(run_pipeline(&mut rir).is_empty());
    match interpret_module(&rir, &interner).expect("interp") {
        InterpValue::I64(n) => n,
        other => panic!("expected i64, got {other:?}"),
    }
}

fn build_and_run(root: &std::path::Path, src_path: &str, out_stem: &str) -> i32 {
    let out = root.join("target").join(out_stem);
    let status = Command::new(env!("CARGO_BIN_EXE_rynixc"))
        .current_dir(root)
        .args([
            "build",
            src_path,
            "-o",
            out.to_str().unwrap(),
            "--runtime=portable",
        ])
        .status()
        .expect("rynixc build");
    assert!(status.success(), "build failed for {src_path}");
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    let out = Command::new(&exe).output().expect("run binary");
    // `main() -> i64` becomes the process exit code (non-zero is normal here).
    out.status.code().unwrap_or(0)
}

#[test]
fn llvm_matches_interp_arith() {
    if !clang_present() {
        eprintln!("skip: no clang");
        return;
    }
    let src = "def main() -> i64\n  return 2 + 3 * 4\nend\n";
    let expected = interp_i64(src);
    assert_eq!(expected, 14);

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let path = root.join("target/diff_arith.ryx");
    std::fs::write(&path, src).unwrap();
    let code = build_and_run(&root, "target/diff_arith.ryx", "diff_arith_bin");
    assert_eq!(code, (expected as i32) & 0xff);
}

#[test]
fn llvm_matches_interp_match() {
    if !clang_present() {
        eprintln!("skip: no clang");
        return;
    }
    let src = r"
def main() -> i64
  let x = 2
  match x
    1
      return 11
    2
      return 22
    else
      return 0
  end
  return -1
end
";
    let expected = interp_i64(src);
    assert_eq!(expected, 22);

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let path = root.join("target/diff_match.ryx");
    std::fs::write(&path, src).unwrap();
    let code = build_and_run(&root, "target/diff_match.ryx", "diff_match_bin");
    assert_eq!(code, (expected as i32) & 0xff);
}

#[test]
fn llvm_matches_interp_match_bool() {
    if !clang_present() {
        eprintln!("skip: no clang");
        return;
    }
    let src = r"
def main() -> i64
  let b = true
  match b
    false
      return 1
    true
      return 9
  end
  return 0
end
";
    let expected = interp_i64(src);
    assert_eq!(expected, 9);

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let path = root.join("target/diff_match_bool.ryx");
    std::fs::write(&path, src).unwrap();
    let code = build_and_run(&root, "target/diff_match_bool.ryx", "diff_match_bool_bin");
    assert_eq!(code, (expected as i32) & 0xff);
}

#[test]
fn llvm_matches_interp_bool_and_or() {
    if !clang_present() {
        eprintln!("skip: no clang");
        return;
    }
    let src = r"
def main() -> i64
  if true and false
    return 1
  end
  if false or true
    return 5
  end
  return 0
end
";
    let expected = interp_i64(src);
    assert_eq!(expected, 5);

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let path = root.join("target/diff_bool_and.ryx");
    std::fs::write(&path, src).unwrap();
    let code = build_and_run(&root, "target/diff_bool_and.ryx", "diff_bool_and_bin");
    assert_eq!(code, (expected as i32) & 0xff);
}

#[test]
fn llvm_vec_methods_end_to_end() {
    if !clang_present() {
        eprintln!("skip: no clang");
        return;
    }
    // Interpreter stubs CallExt — this is an honest LLVM+runtime check.
    let src = r"
def main() -> i64
  let v: Vec[i64] = vec_new(0)
  v.push(10)
  v.push(20)
  return v.len() + v.get(0)
end
";
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let path = root.join("target/diff_vec_methods.ryx");
    std::fs::write(&path, src).unwrap();
    let code = build_and_run(&root, "target/diff_vec_methods.ryx", "diff_vec_methods_bin");
    assert_eq!(code, 12, "len(2)+get(0)=10 → 12");
}

#[test]
fn llvm_map_methods_end_to_end() {
    if !clang_present() {
        eprintln!("skip: no clang");
        return;
    }
    let src = r"
def main() -> i64
  let m: Map[i64, i64] = map_new(0)
  m.insert(1, 100)
  m.insert(2, 200)
  return m.len() + m.get(1)
end
";
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let path = root.join("target/diff_map_methods.ryx");
    std::fs::write(&path, src).unwrap();
    let code = build_and_run(&root, "target/diff_map_methods.ryx", "diff_map_methods_bin");
    assert_eq!(code, 102, "len(2)+get(1)=100 → 102");
}
