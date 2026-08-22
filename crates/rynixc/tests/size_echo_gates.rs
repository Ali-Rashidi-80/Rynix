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
        cmd.arg("-fuse-ld=lld").arg("-lws2_32");
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

#[test]
fn tcp_echo_rps_c() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = std::env::temp_dir().join("rynix_tcp_echo_rps");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/tcp_echo_rps.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld").arg("-lws2_32");
    }
    let status = cmd.status().expect("clang tcp_echo");
    assert!(status.success(), "tcp_echo compile failed");
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    let status = Command::new(&exe).status().expect("run tcp_echo");
    assert!(status.success(), "tcp_echo_rps failed");
}

#[test]
fn collections_smoke_c() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let src = root.join("target/collections_smoke.c");
    std::fs::write(
        &src,
        r#"
#include "rynix_rt.h"
#include <stdio.h>
int main(void) {
  rynix_rt_region_create(0);
  void *v = rynix_rt_vec_i64_new(0);
  rynix_rt_vec_i64_push(v, 10);
  rynix_rt_vec_i64_push(v, 20);
  if (rynix_rt_vec_i64_len(v) != 2) return 1;
  if (rynix_rt_vec_i64_get(v, 1) != 20) return 1;
  void *m = rynix_rt_map_i64_new(0);
  rynix_rt_map_i64_insert(m, 7, 70);
  if (rynix_rt_map_i64_get(m, 7) != 70) return 1;
  puts("collections ok");
  return 0;
}
"#,
    )
    .unwrap();
    let out = root.join("target/collections_smoke");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg(&src)
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld").arg("-lws2_32");
    }
    assert!(cmd.status().unwrap().success());
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    assert!(Command::new(exe).status().unwrap().success());
}

#[test]
fn load_harness_rps_floor() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = root.join("target/load_harness_rt");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/load_harness.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld").arg("-lws2_32");
    }
    let status = cmd.status().expect("clang load_harness");
    assert!(status.success(), "load_harness compile failed");
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    let status = Command::new(&exe).status().expect("run load_harness");
    assert!(status.success(), "load_harness failed");
}

#[test]
fn uring_sqe_smoke_c() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = root.join("target/uring_sqe_smoke_rt");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/uring_sqe_smoke.c")
        .arg("-o")
        .arg(&out);
    // On non-Linux this exercises the stub path (-1 returns).
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld").arg("-lws2_32");
    }
    let status = cmd.status().expect("clang uring_sqe_smoke");
    assert!(status.success(), "uring_sqe_smoke compile failed");
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    let status = Command::new(&exe).status().expect("run uring_sqe_smoke");
    assert!(status.success(), "uring_sqe_smoke failed");
}

#[test]
fn fiber_park_smoke_c() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = root.join("target/fiber_park_smoke_rt");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/fiber_park_smoke.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld").arg("-lws2_32");
    }
    let status = cmd.status().expect("clang fiber_park_smoke");
    assert!(status.success(), "fiber_park_smoke compile failed");
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    let status = Command::new(&exe).status().expect("run fiber_park_smoke");
    assert!(status.success(), "fiber_park_smoke failed");
}

#[test]
fn json_smoke_c() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = root.join("target/json_smoke_rt");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/json_smoke.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld").arg("-lws2_32");
    }
    assert!(cmd.status().unwrap().success(), "json_smoke compile failed");
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    assert!(Command::new(exe).status().unwrap().success(), "json_smoke failed");
}

#[test]
fn json_unit_c() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = root.join("target/json_unit_rt");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/json_unit.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld").arg("-lws2_32");
    }
    assert!(cmd.status().unwrap().success(), "json_unit compile failed");
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    assert!(Command::new(exe).status().unwrap().success(), "json_unit failed");
}

#[test]
fn http_smoke_c() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = std::env::temp_dir().join("rynix_http_smoke_rt");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/http_smoke.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld").arg("-lws2_32");
    }
    assert!(cmd.status().unwrap().success(), "http_smoke compile failed");
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    assert!(Command::new(exe).status().unwrap().success(), "http_smoke failed");
}
