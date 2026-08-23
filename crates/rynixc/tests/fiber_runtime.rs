//! Compiles and runs `rt/tests/fiber_smoke.c` when a MinGW/LLVM clang is available.

use std::path::PathBuf;
use std::process::Command;

fn find_clang() -> Option<PathBuf> {
    for name in [
        "x86_64-w64-mingw32-clang",
        "x86_64-w64-mingw32-clang.exe",
        "clang",
        "clang.exe",
    ] {
        if let Ok(o) = Command::new(name).arg("--version").output()
            && o.status.success()
        {
            return Some(PathBuf::from(name));
        }
    }
    None
}

#[test]
fn fiber_smoke_round_robin() {
    let Some(clang) = find_clang() else {
        eprintln!("skip fiber_smoke: clang not on PATH");
        return;
    };

    // Resolve workspace root without `\\?\` prefixes (breaks clang includes).
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = std::fs::canonicalize(&root).unwrap_or(root);
    let root = PathBuf::from(
        root.to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_string(),
    );

    let out = std::env::temp_dir().join("rynix_fiber_smoke.exe");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O2")
        .arg("-Irt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/fiber_smoke.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld").arg("-lws2_32").arg("-lsecur32").arg("-lcrypt32").arg("-lbcrypt");
    }
    let status = cmd.status().expect("spawn clang");
    assert!(status.success(), "clang failed: {status}");

    let run = Command::new(&out).output().expect("run fiber_smoke");
    assert!(
        run.status.success(),
        "fiber_smoke failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("fiber_smoke ok"), "{stdout}");
}
