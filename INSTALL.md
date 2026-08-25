# Install Rynix

**Languages:** [English](INSTALL.md) (default) · [فارسی](INSTALL.fa.md)

Install **`rynixc`** from source or GitHub Release. Every step below has a matching
verify command — same gates as CI where noted.

---

## Prerequisites

| Requirement | Notes |
|-------------|-------|
| **Rust** | MSRV **1.98** (`Cargo.toml`); [`rust-toolchain.toml`](rust-toolchain.toml) pins the `stable` channel |
| **clang** | Required for `build` / `run` / `emit-wasm` (one-path setup below) |
| **Python 3** | Optional — Suite5 harness only |
| **Node.js 18+** | Optional — VS Code extension; also runs `emit-wasm` Node smokes |

### One-path clang (Win + Linux)

Rynix links through **clang** only — no second compiler path. Install once, put it on
`PATH`, then `rynixc build` / `run` / `emit-wasm` use the same driver.

| Platform | One-path clang setup |
|----------|----------------------|
| **Windows** | Install **LLVM/clang** or **MinGW-w64 clang** (`x86_64-w64-mingw32-clang`). Add the `bin` directory to `PATH`. Prefer cargo target `x86_64-pc-windows-gnu`. Verify: `clang --version` (or `x86_64-w64-mingw32-clang --version`). Runtime default: `--runtime=portable` (optional `--runtime=iocp`). |
| **Linux** | Install distro **clang** (e.g. `sudo apt install clang` / `sudo dnf install clang`). Verify: `clang --version`. Runtime: `--runtime=portable` or `--runtime=uring` when built with `RYNIX_RT_URING`. |

```sh
# Both platforms — confirm clang is the single link driver on PATH:
clang --version
rynixc build examples/01_hello.ryx -o target/hello --runtime=portable
```

### Platform toolchains

| OS | Toolchain | Runtime flag |
|----|-----------|----------------|
| **Windows** | MinGW `x86_64-w64-mingw32-clang`, target `x86_64-pc-windows-gnu` | `--runtime=portable` |
| **Linux** | System `clang` | `--runtime=portable` or `--runtime=uring` |
| **macOS** | System `clang` | `--runtime=portable` |

```text
  prerequisites                install                    verify
  ─────────────                ───────                    ──────
  Rust + clang        ──▶   INSTALL.sh / install.ps1  ──▶  rynixc --version
       │                  or cargo install                  cargo test
       │                                                       arch check
       └──────────────────────────────────────────────────▶  build + run hello
```

---

## Quick install (from source)

### Windows (PowerShell)

```powershell
.\install.ps1
```

### Unix (Linux / macOS)

```sh
chmod +x INSTALL.sh
./INSTALL.sh
```

Both scripts run:

```sh
cargo install --path crates/rynixc --force
```

and print the install path of `rynixc`.

### Manual install

```sh
cargo install --path crates/rynixc --force
rynixc --version
```

Ensure `~/.cargo/bin` (or `%USERPROFILE%\.cargo\bin`) is on your `PATH`.

---

## Install from GitHub Release

On tagged releases (`v*`), CI publishes prebuilt binaries:

| Artifact | Platform |
|----------|----------|
| `rynixc-linux-x86_64` | Linux x86_64 |
| `rynixc-windows-x86_64.exe` | Windows x86_64 |

1. Download from GitHub Releases (tag `v*`) when published for your fork/org.
2. Verify SHA256 against `SHA256SUMS.txt` in the release assets.
3. If `SHA256SUMS.txt.asc` is present (optional GPG secret configured on the
   publisher), verify the signature:

```sh
gpg --verify SHA256SUMS.txt.asc SHA256SUMS.txt
```

4. Place the binary on `PATH` as `rynixc`.

Local packaging (SHA256SUMS + optional detach-sign):

```powershell
# Windows
.\scripts\build_release.ps1   # set RYNIX_GPG_KEY_ID to also sign
```

```sh
# Ephemeral sign/verify smoke (no production key required):
bash scripts/gpg_sign_smoke.sh
```

> Release binaries still need **clang** on `PATH` for `rynixc build` / `run`.

---

## First run

```sh
rynixc run examples/01_hello.ryx
rynixc check examples/03_vec.ryx
rynixc graph examples/02_match_loop.ryx
```

Build a native binary:

```sh
rynixc build examples/03_vec.ryx -o target/ex_vec --runtime=portable
./target/ex_vec          # Unix
.\target\ex_vec.exe      # Windows
```

JSON example (no network):

```sh
rynixc run examples/05_http_json.ryx    # prints 42
```

---

## Verify installation

### Minimal smoke

```sh
rynixc --version
rynixc arch check
rynixc run examples/01_hello.ryx
```

### Full developer verify (matches CI intent)

```sh
cargo test --workspace
cargo clippy -p rynixc -p rynix-rir -p rynix-codegen -p rynix-sema -- -D warnings
rynixc arch check --error-format=json
rynixc build examples/03_vec.ryx -o target/ex_vec --runtime=portable
python benchmarks/suite5/run_suite5.py --langs c,rynix
```

Expected: all tests pass; arch JSON `status: passed`; Suite5 reports checksum OK
for all ten C + Rynix rows.

---

## Editor (optional)

```sh
cd editors/vscode
npm install
npm run compile
```

Windows shortcut:

```powershell
.\editors\vscode\install_extension.ps1
```

VS Code settings:

| Key | Value |
|-----|-------|
| `rynix.compilerPath` | Full path to `rynixc` if not on PATH |
| `rynix.enableLsp` | `true` |

See [`editors/vscode/README.md`](editors/vscode/README.md).

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `clang not found` on Windows | Install MinGW-w64 / LLVM clang (see **One-path clang** above); add `bin` to PATH |
| `clang not found` on Linux | Install distro clang (`apt`/`dnf`/`pacman`); confirm `clang --version` |
| `linker failed` on Windows | Use `x86_64-pc-windows-gnu` target; `-fuse-ld=lld` if needed |
| `Architecture.toml not found` | Run `arch check` from repo root, or pass `--root /path/to/Rynix` |
| `build` succeeds but run hangs on TCP | Standalone smokes use blocking connect; fiber apps need `rynix_rt_run` |
| Suite5 checksum mismatch | Run `cargo build -p rynixc`; compare C vs Rynix source in `benchmarks/suite5/` |

---

## Next steps

| Goal | Doc |
|------|-----|
| Language reference | [`docs/SPEC.md`](docs/SPEC.md) |
| Runtime ABI | [`docs/abi.md`](docs/abi.md) |
| AI / MCP workflows | [`AGENTS.md`](AGENTS.md) |
| Roadmap & phases | [`docs/ROADMAP.md`](docs/ROADMAP.md) |
| Contributing | [`CONTRIBUTING.md`](CONTRIBUTING.md) |

Back to [`README.md`](README.md).
