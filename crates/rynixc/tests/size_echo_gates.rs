//! Binary size gate + fiber echo smoke (M7/M8 acceptance).

use std::path::PathBuf;
use std::process::Command;

fn clang() -> Option<String> {
    for c in [
        "x86_64-w64-mingw32-clang",
        "clang",
        "clang.exe",
    ] {
        if Command::new(c).arg("--version").output().is_ok() {
            return Some(c.into());
        }
    }
    None
}

#[test]
fn hello_binary_under_300kb() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..");
    let root = root.canonicalize().unwrap();
    let out = root.join("target/size_gate_hello");
    let status = Command::new(env!("CARGO_BIN_EXE_rynixc"))
        .current_dir(&root)
        .args([
            "build",
            "testdata/lexer/hello.ryx",
            "-o",
            out.to_str().unwrap(),
            "--runtime=portable",
        ])
        .status()
        .expect("rynixc build");
    assert!(status.success(), "build failed");
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    let meta = std::fs::metadata(&exe).expect("stat binary");
    let kb = meta.len() / 1024;
    eprintln!("hello binary {kb} KiB (clang={clang})");
    assert!(
        meta.len() < 300 * 1024,
        "hello binary is {} bytes; gate is < 300KiB",
        meta.len()
    );
}

#[test]
fn fiber_echo_smoke_c() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = root.join("target/echo_smoke_rt");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/echo_smoke.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld");
    }
    let status = cmd.status().expect("clang echo");
    assert!(status.success(), "echo_smoke compile failed");
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    let status = Command::new(&exe).status().expect("run echo");
    assert!(status.success(), "echo_smoke failed");
}
