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

/// Prefer a clang that can compile `--target=wasm32-unknown-unknown`.
fn clang_with_wasm() -> Option<String> {
    for c in ["clang", "clang.exe", "x86_64-w64-mingw32-clang"] {
        let probe_c = std::env::temp_dir().join("rynix_wasm_clang_probe.c");
        let probe_o = std::env::temp_dir().join("rynix_wasm_clang_probe.o");
        let _ = std::fs::write(&probe_c, "int main(void){return 0;}\n");
        let ok = Command::new(c)
            .args([
                "--target=wasm32-unknown-unknown",
                "-c",
                probe_c.to_str()?,
                "-o",
                probe_o.to_str()?,
            ])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            return Some(c.into());
        }
    }
    None
}

/// Prefer a clang that can **link** a freestanding wasm32 module to `.wasm`.
fn clang_with_wasm_link() -> Option<String> {
    for c in ["clang", "clang.exe", "x86_64-w64-mingw32-clang"] {
        let probe_ll = std::env::temp_dir().join("rynix_wasm_link_probe.ll");
        let probe_wasm = std::env::temp_dir().join("rynix_wasm_link_probe.wasm");
        let _ = std::fs::write(
            &probe_ll,
            "target triple = \"wasm32-unknown-unknown\"\ndefine i32 @main() {\nentry:\n  ret i32 0\n}\n",
        );
        let ok = Command::new(c)
            .args([
                "--target=wasm32-unknown-unknown",
                "-nostdlib",
                "-Wl,--no-entry",
                "-Wl,--export-all",
                "-o",
                probe_wasm.to_str()?,
                probe_ll.to_str()?,
            ])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            let _ = std::fs::remove_file(&probe_wasm);
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
        cmd.arg("-fuse-ld=lld").arg("-lws2_32").arg("-lsecur32").arg("-lcrypt32").arg("-lbcrypt");
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
        cmd.arg("-fuse-ld=lld").arg("-lws2_32").arg("-lsecur32").arg("-lcrypt32").arg("-lbcrypt");
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
        cmd.arg("-fuse-ld=lld").arg("-lws2_32").arg("-lsecur32").arg("-lcrypt32").arg("-lbcrypt");
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
        cmd.arg("-fuse-ld=lld").arg("-lws2_32").arg("-lsecur32").arg("-lcrypt32").arg("-lbcrypt");
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
        cmd.arg("-fuse-ld=lld").arg("-lws2_32").arg("-lsecur32").arg("-lcrypt32").arg("-lbcrypt");
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
        cmd.arg("-fuse-ld=lld").arg("-lws2_32").arg("-lsecur32").arg("-lcrypt32").arg("-lbcrypt");
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
        cmd.arg("-fuse-ld=lld").arg("-lws2_32").arg("-lsecur32").arg("-lcrypt32").arg("-lbcrypt");
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
        cmd.arg("-fuse-ld=lld").arg("-lws2_32").arg("-lsecur32").arg("-lcrypt32").arg("-lbcrypt");
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
        cmd.arg("-fuse-ld=lld").arg("-lws2_32").arg("-lsecur32").arg("-lcrypt32").arg("-lbcrypt");
    }
    assert!(cmd.status().unwrap().success(), "http_smoke compile failed");
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    assert!(Command::new(exe).status().unwrap().success(), "http_smoke failed");
}

#[test]
fn http_serve_once_smoke_c() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = std::env::temp_dir().join("rynix_http_serve_once_rt");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/http_serve_once_smoke.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld").arg("-lws2_32").arg("-lsecur32").arg("-lcrypt32").arg("-lbcrypt");
    }
    assert!(
        cmd.status().unwrap().success(),
        "http_serve_once smoke compile failed"
    );
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    assert!(
        Command::new(exe).status().unwrap().success(),
        "http_serve_once smoke failed"
    );
}

#[test]
fn http_loop_three_gets() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();

    // Soft builtin: sema + lower + LLVM decl (Wave 2.2).
    let ryx = root.join("target/http_loop_check.ryx");
    std::fs::write(
        &ryx,
        r#"def main() -> i64
  let rc = http_serve_loop_json_i64(0, "/api", 7, 3)
  return rc
end
"#,
    )
    .unwrap();
    let check = Command::new(env!("CARGO_BIN_EXE_rynixc"))
        .args(["check", ryx.to_str().unwrap()])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "http_serve_loop_json_i64 check failed:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let ll = Command::new(env!("CARGO_BIN_EXE_rynixc"))
        .args(["emit-ll", ryx.to_str().unwrap()])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(ll.status.success(), "emit-ll failed");
    let text = String::from_utf8_lossy(&ll.stdout);
    assert!(
        text.contains("rynix_rt_http_serve_loop_json_i64"),
        "missing serve_loop runtime call in IR"
    );

    let out = std::env::temp_dir().join("rynix_http_loop_three_gets_rt");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/http_loop_three_gets_smoke.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld").arg("-lws2_32").arg("-lsecur32").arg("-lcrypt32").arg("-lbcrypt");
    }
    assert!(
        cmd.status().unwrap().success(),
        "http_loop_three_gets smoke compile failed"
    );
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    assert!(
        Command::new(exe).status().unwrap().success(),
        "http_loop_three_gets smoke failed"
    );
}

#[test]
fn http_loop_2paths() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();

    let ryx = root.join("target/http_loop_2paths_check.ryx");
    std::fs::write(
        &ryx,
        r#"def main() -> i64
  let rc = http_serve_loop_2paths_json_i64(0, "/a", 11, "/b", 22, 3)
  return rc
end
"#,
    )
    .unwrap();
    let check = Command::new(env!("CARGO_BIN_EXE_rynixc"))
        .args(["check", ryx.to_str().unwrap()])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "http_serve_loop_2paths_json_i64 check failed:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let ll = Command::new(env!("CARGO_BIN_EXE_rynixc"))
        .args(["emit-ll", ryx.to_str().unwrap()])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(ll.status.success(), "emit-ll failed");
    let text = String::from_utf8_lossy(&ll.stdout);
    assert!(
        text.contains("rynix_rt_http_serve_loop_2paths_json_i64"),
        "missing 2paths serve_loop runtime call in IR"
    );

    let out = std::env::temp_dir().join("rynix_http_loop_2paths_rt");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/http_loop_2paths_smoke.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld").arg("-lws2_32").arg("-lsecur32").arg("-lcrypt32").arg("-lbcrypt");
    }
    assert!(
        cmd.status().unwrap().success(),
        "http_loop_2paths smoke compile failed"
    );
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    assert!(
        Command::new(exe).status().unwrap().success(),
        "http_loop_2paths smoke failed"
    );
}

#[test]
fn http_post_echo_smoke_c() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = std::env::temp_dir().join("rynix_http_post_echo_rt");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/http_post_echo_smoke.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld").arg("-lws2_32").arg("-lsecur32").arg("-lcrypt32").arg("-lbcrypt");
    }
    assert!(
        cmd.status().unwrap().success(),
        "http_post_echo smoke compile failed"
    );
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    assert!(
        Command::new(exe).status().unwrap().success(),
        "http_post_echo smoke failed"
    );
}

#[test]
fn frame_echo_smoke_c() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = std::env::temp_dir().join("rynix_frame_echo_rt");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/frame_echo_smoke.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld").arg("-lws2_32").arg("-lsecur32").arg("-lcrypt32").arg("-lbcrypt");
    }
    assert!(
        cmd.status().unwrap().success(),
        "frame_echo smoke compile failed"
    );
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    assert!(
        Command::new(exe).status().unwrap().success(),
        "frame_echo smoke failed"
    );
}

#[test]
fn crypto_kv_smoke_c() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = std::env::temp_dir().join("rynix_crypto_kv_smoke_rt");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/crypto_kv_smoke.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld").arg("-lws2_32").arg("-lsecur32").arg("-lcrypt32").arg("-lbcrypt");
    }
    assert!(
        cmd.status().unwrap().success(),
        "crypto_kv smoke compile failed"
    );
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    assert!(
        Command::new(exe).status().unwrap().success(),
        "crypto_kv smoke failed"
    );
}

#[test]
fn fs_smoke_c() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = std::env::temp_dir().join("rynix_fs_smoke_rt");
    let mut cmd = Command::new(&clang);
    // `fs.c` is self-contained (stdio); avoid linking full portable + Winsock.
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/src/fs.c")
        .arg("rt/tests/fs_smoke.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld");
    }
    assert!(cmd.status().unwrap().success(), "fs smoke compile failed");
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    assert!(
        Command::new(exe).status().unwrap().success(),
        "fs smoke failed"
    );
}

#[test]
fn tls_echo_smoke_c() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = std::env::temp_dir().join("rynix_tls_echo_rt");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/tls_echo_smoke.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld")
            .arg("-lws2_32")
            .arg("-lsecur32")
            .arg("-lcrypt32");
    } else {
        // Opt-in OpenSSL backend: `clang -DRYNIX_RT_OPENSSL … -lssl -lcrypto`
        // Default Linux builds use the stub (-2) so CI needs no libssl.
    }
    let compiled = cmd.status().unwrap().success();
    if !compiled {
        if cfg!(windows) {
            panic!("tls_echo smoke compile failed");
        }
        eprintln!("skip: TLS backend not available for link");
        return;
    }
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    let status = Command::new(exe).status().unwrap();
    let code = status.code().unwrap_or(1);
    if code == 77 {
        eprintln!("skip: tls_echo unsupported on this host");
        return;
    }
    assert!(status.success(), "tls_echo smoke failed code={code}");
}

fn suite12_checksum_gate(src: &str, out_name: &str, want: &str) {
    suite12_checksum_gate_ex(src, out_name, want, &[]);
}

fn suite12_checksum_gate_ex(src: &str, out_name: &str, want: &str, extra_args: &[&str]) {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = std::env::temp_dir().join(out_name);
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O3")
        .arg(src)
        .arg("-o")
        .arg(&out);
    for a in extra_args {
        cmd.arg(a);
    }
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld");
    } else {
        cmd.arg("-lm");
    }
    assert!(
        cmd.status().unwrap().success(),
        "suite12 {src} compile failed"
    );
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    let output = Command::new(exe).output().expect("run");
    assert!(output.status.success(), "{src} failed");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains(want),
        "{src}: unexpected checksum: {text} (want {want})"
    );
}

#[test]
fn suite12_alu_reduction_checksum() {
    suite12_checksum_gate(
        "benchmarks/suite12/alu_reduction.c",
        "rynix_suite12_alu",
        "checksum=3370198876750320971",
    );
}

#[test]
fn suite12_hft_engine_checksum() {
    suite12_checksum_gate(
        "benchmarks/suite12/hft_engine.c",
        "rynix_suite12_hft",
        "checksum=552829538",
    );
}

#[test]
fn suite12_json_serializer_checksum() {
    suite12_checksum_gate(
        "benchmarks/suite12/json_serializer.c",
        "rynix_suite12_json",
        "checksum=5588438541400559045",
    );
}

#[test]
fn suite12_fsm_lexer_checksum() {
    suite12_checksum_gate(
        "benchmarks/suite12/fsm_lexer.c",
        "rynix_suite12_fsm",
        "checksum=-103069600432064540",
    );
}

#[test]
fn suite12_dna_levenshtein_checksum() {
    suite12_checksum_gate(
        "benchmarks/suite12/dna_levenshtein.c",
        "rynix_suite12_dna",
        "checksum=525912",
    );
}

#[test]
fn suite12_gemm_matrix_checksum() {
    suite12_checksum_gate(
        "benchmarks/suite12/gemm_matrix.c",
        "rynix_suite12_gemm",
        "checksum=6422836",
    );
}

#[test]
fn suite12_monte_carlo_bs_checksum() {
    suite12_checksum_gate(
        "benchmarks/suite12/monte_carlo_bs.c",
        "rynix_suite12_mc",
        "checksum=10440246",
    );
}

#[test]
fn suite12_binary_trees_checksum() {
    suite12_checksum_gate(
        "benchmarks/suite12/binary_trees.c",
        "rynix_suite12_trees",
        "checksum=407713",
    );
}

#[test]
fn suite12_sha256_blocks_checksum() {
    suite12_checksum_gate(
        "benchmarks/suite12/sha256_blocks.c",
        "rynix_suite12_sha256",
        "checksum=-4721506799343634759",
    );
}

fn suite12_ryx_checksum_gate(src: &str, out_name: &str, want: &str) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = std::env::temp_dir().join(out_name);
    let status = Command::new(env!("CARGO_BIN_EXE_rynixc"))
        .current_dir(&root)
        .args([
            "build",
            src,
            "-o",
            out.to_str().unwrap(),
            "--runtime=portable",
        ])
        .status()
        .expect("rynixc build");
    assert!(status.success(), "suite12 ryx {src} build failed");
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    let output = Command::new(exe).output().expect("run");
    assert!(
        output.status.success(),
        "{src} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains(want),
        "{src}: unexpected checksum: {text} (want {want})"
    );
}

#[test]
fn suite12_alu_ryx_checksum() {
    suite12_ryx_checksum_gate(
        "benchmarks/suite12/alu_reduction.ryx",
        "rynix_suite12_alu_ryx",
        "3370198876750320971",
    );
}

#[test]
fn suite12_json_ryx_checksum() {
    suite12_ryx_checksum_gate(
        "benchmarks/suite12/json_serializer.ryx",
        "rynix_suite12_json_ryx",
        "5588438541400559045",
    );
}

#[test]
fn suite12_sha256_ryx_checksum() {
    suite12_ryx_checksum_gate(
        "benchmarks/suite12/sha256_blocks.ryx",
        "rynix_suite12_sha256_ryx",
        "-4721506799343634759",
    );
}

#[test]
fn ws_accept_smoke_c() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = std::env::temp_dir().join("rynix_ws_accept_rt");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/ws_accept_smoke.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld")
            .arg("-lws2_32")
            .arg("-lsecur32")
            .arg("-lcrypt32")
            .arg("-lbcrypt");
    }
    assert!(
        cmd.status().unwrap().success(),
        "ws_accept smoke compile failed"
    );
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    assert!(
        Command::new(exe).status().unwrap().success(),
        "ws_accept smoke failed"
    );
}

#[test]
fn ws_frames_smoke_c() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = std::env::temp_dir().join("rynix_ws_frames_rt");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/ws_frames_smoke.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld")
            .arg("-lws2_32")
            .arg("-lsecur32")
            .arg("-lcrypt32")
            .arg("-lbcrypt");
    }
    assert!(
        cmd.status().unwrap().success(),
        "ws_frames smoke compile failed"
    );
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    assert!(
        Command::new(exe).status().unwrap().success(),
        "ws_frames smoke failed"
    );
}

#[test]
fn ws_large_echo_smoke_c() {
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = std::env::temp_dir().join("rynix_ws_large_echo_rt");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/ws_large_echo_smoke.c")
        .arg("-o")
        .arg(&out);
    if cfg!(windows) {
        cmd.arg("-fuse-ld=lld")
            .arg("-lws2_32")
            .arg("-lsecur32")
            .arg("-lcrypt32")
            .arg("-lbcrypt");
    }
    assert!(
        cmd.status().unwrap().success(),
        "ws_large_echo smoke compile failed"
    );
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    assert!(
        Command::new(exe).status().unwrap().success(),
        "ws_large_echo smoke failed"
    );
}

#[test]
fn iocp_echo_smoke_c() {
    if !cfg!(windows) {
        eprintln!("skip: IOCP is Windows-only");
        return;
    }
    let Some(clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let out = std::env::temp_dir().join("rynix_iocp_echo_rt");
    let mut cmd = Command::new(&clang);
    cmd.current_dir(&root)
        .arg("-O1")
        .arg("-DRYNIX_RT_IOCP")
        .arg("-I")
        .arg("rt/include")
        .arg("rt/portable.c")
        .arg("rt/tests/iocp_echo_smoke.c")
        .arg("-o")
        .arg(&out)
        .arg("-fuse-ld=lld")
        .arg("-lws2_32")
        .arg("-lsecur32")
        .arg("-lcrypt32")
        .arg("-lbcrypt");
    assert!(
        cmd.status().unwrap().success(),
        "iocp_echo smoke compile failed"
    );
    let exe = if out.with_extension("exe").is_file() {
        out.with_extension("exe")
    } else {
        out
    };
    assert!(
        Command::new(exe).status().unwrap().success(),
        "iocp_echo smoke failed"
    );
}

#[test]
fn gpg_detach_sign_smoke() {
    let has_gpg = Command::new("gpg")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !has_gpg {
        eprintln!("skip: gpg not on PATH");
        return;
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();

    // Prefer bash script; fall back to PowerShell on Windows.
    let bash_script = root.join("scripts/gpg_sign_smoke.sh");
    let ps_script = root.join("scripts/gpg_sign_smoke.ps1");

    let output = if Command::new("bash").arg("--version").output().is_ok() && bash_script.is_file()
    {
        Command::new("bash")
            .arg(&bash_script)
            .current_dir(&root)
            .output()
            .expect("bash gpg smoke")
    } else if cfg!(windows) && ps_script.is_file() {
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                ps_script.to_str().unwrap(),
            ])
            .current_dir(&root)
            .output()
            .expect("powershell gpg smoke")
    } else {
        eprintln!("skip: no bash/powershell runner for gpg smoke");
        return;
    };

    let code = output.status.code().unwrap_or(1);
    if code == 77 {
        eprintln!("skip: gpg_sign_smoke returned 77");
        return;
    }
    assert!(
        output.status.success(),
        "gpg_sign_smoke failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn emit_ll_wasm32_clang_accepts() {
    let Some(clang) = clang_with_wasm() else {
        eprintln!("skip: no clang with wasm32-unknown-unknown on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    std::fs::create_dir_all(root.join("target")).ok();

    let ll = root.join("target/wasm_arith.ll");
    let obj = root.join("target/wasm_arith.o");
    let status = Command::new(env!("CARGO_BIN_EXE_rynixc"))
        .current_dir(&root)
        .args([
            "emit-ll",
            "testdata/wasm_arith.ryx",
            "--target=wasm32-unknown-unknown",
            "-o",
            ll.to_str().unwrap(),
        ])
        .status()
        .expect("emit-ll wasm");
    assert!(status.success(), "emit-ll --target=wasm32 failed");
    let text = std::fs::read_to_string(&ll).expect("read ll");
    assert!(
        text.contains("target triple = \"wasm32-unknown-unknown\""),
        "missing wasm32 triple in .ll"
    );
    let status = Command::new(&clang)
        .current_dir(&root)
        .args([
            "--target=wasm32-unknown-unknown",
            "-c",
            ll.to_str().unwrap(),
            "-o",
            obj.to_str().unwrap(),
            "-Wno-override-module",
        ])
        .status()
        .expect("clang wasm -c");
    assert!(status.success(), "clang --target=wasm32 -c rejected .ll");
    assert!(obj.is_file(), "wasm object missing");
}

#[test]
fn emit_wasm_clang_produces_wasm() {
    let Some(_clang) = clang_with_wasm_link() else {
        eprintln!("skip: no clang that can link wasm32 freestanding .wasm");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    std::fs::create_dir_all(root.join("target")).ok();

    let wasm = root.join("target/wasm_arith.wasm");
    let _ = std::fs::remove_file(&wasm);
    let status = Command::new(env!("CARGO_BIN_EXE_rynixc"))
        .current_dir(&root)
        .args([
            "emit-wasm",
            "testdata/wasm_arith.ryx",
            "-o",
            wasm.to_str().unwrap(),
        ])
        .status()
        .expect("emit-wasm");
    assert!(status.success(), "emit-wasm failed");
    assert!(wasm.is_file(), "emit-wasm did not write {}", wasm.display());
    let bytes = std::fs::read(&wasm).expect("read wasm");
    assert!(
        bytes.len() >= 4 && bytes[0] == 0 && &bytes[1..4] == b"asm",
        "missing \\0asm magic (got {} bytes)",
        bytes.len()
    );
}

#[test]
fn build_respects_manifest_optimize() {
    let Some(_clang) = clang() else {
        eprintln!("skip: no clang on PATH");
        return;
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..").canonicalize().unwrap();
    let dir = root.join("target/pkg_opt_gate");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("rynix.toml"),
        r#"[package]
name = "opt_gate"
version = "0.1.0"
entry = "main.ryx"

[build]
optimize = false
runtime = "portable"
"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("main.ryx"),
        "def main() -> i64\n  return 1 + 1\nend\n",
    )
    .unwrap();
    let out = dir.join("opt_gate_bin");
    let ll = out.with_extension("ll");
    let status = Command::new(env!("CARGO_BIN_EXE_rynixc"))
        .current_dir(&dir)
        .args([
            "build",
            "--keep-ll",
            "-o",
            out.to_str().unwrap(),
            "--runtime=portable",
        ])
        .status()
        .expect("build");
    assert!(status.success(), "build with optimize=false failed");
    // With RIR opt off, the add of two literals should still appear in .ll
    // (const-fold lives in the optimize pipeline).
    let text = std::fs::read_to_string(&ll).expect("read keep-ll");
    assert!(
        text.contains("add i64") || text.contains("add nsw i64") || text.contains("add nuw i64"),
        "expected unoptimized add in .ll when [build].optimize=false; got:\n{text}"
    );

    // CLI --opt overrides manifest false.
    let out2 = dir.join("opt_gate_bin_opt");
    let ll2 = out2.with_extension("ll");
    let status = Command::new(env!("CARGO_BIN_EXE_rynixc"))
        .current_dir(&dir)
        .args([
            "build",
            "--keep-ll",
            "--opt",
            "-o",
            out2.to_str().unwrap(),
            "--runtime=portable",
        ])
        .status()
        .expect("build --opt");
    assert!(status.success(), "build --opt failed");
    let text2 = std::fs::read_to_string(&ll2).expect("read keep-ll opt");
    assert!(
        !text2.contains("add i64 1, 1") && !text2.contains("add nsw i64 1, 1"),
        "expected const-fold under --opt; got:\n{text2}"
    );
}

