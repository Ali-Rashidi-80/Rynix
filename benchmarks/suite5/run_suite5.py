#!/usr/bin/env python3
"""Reproducible Suite5 cross-language microbenchmarks.

Identical algorithms in Rynix / C / Rust / Go / Zig / End (optional) / End lang optional.
Each program prints one integer checksum on stdout. The harness verifies
checksum once (full I/O), then times with SUITE5_BENCH=1 (volatile sink, no printf).

Timing: warmup + robust median of N runs (defaults: 3 warmup, 9 timed; trims min/max when N≥5).

Usage (from repo root):
  python benchmarks/suite5/run_suite5.py
  python benchmarks/suite5/run_suite5.py --langs c,rust,rynix --summary
  python benchmarks/suite5/run_suite5.py --warmup 3 --runs 9
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SUITE = Path(__file__).resolve().parent
OUT = ROOT / "target" / "suite5"
CHALLENGES = (
    "alu",
    "nested",
    "fib",
    "hash",
    "prime",
    "sum",
    "bits",
    "matrix",
    "scan",
    "powmod",
    "gcd",
    "reduce",
)


def which(cmd: str) -> str | None:
    return shutil.which(cmd)


def run_once(cmd: list[str], *, bench: bool) -> tuple[str, float, int]:
    env = os.environ.copy()
    if bench:
        env["SUITE5_BENCH"] = "1"
    else:
        env.pop("SUITE5_BENCH", None)
    t0 = time.perf_counter()
    p = subprocess.run(cmd, capture_output=True, text=True, cwd=str(ROOT), env=env)
    dt = (time.perf_counter() - t0) * 1000.0
    out = (p.stdout or "").strip().splitlines()
    checksum = out[-1].strip() if out else ""
    return checksum, dt, p.returncode


def robust_median(samples: list[float]) -> tuple[float, float]:
    """Return (raw_median, trimmed_median). Trim min/max when len >= 5."""
    raw = statistics.median(samples)
    if len(samples) >= 5:
        s = sorted(samples)
        trimmed = statistics.median(s[1:-1])
    else:
        trimmed = raw
    return raw, trimmed


def run_timed_median(
    cmd: list[str],
    *,
    warmup: int,
    runs: int,
    check_cmd: list[str] | None = None,
) -> tuple[str, float, int, dict]:
    """Verify checksum without bench, then median-ms with SUITE5_BENCH.

    ``check_cmd`` defaults to ``cmd``. Rynix uses a non-``--bench`` binary for
    checksum (printf) and a ``--bench`` binary for timing (always-sink RT).
    """
    checksum, _, code = run_once(check_cmd or cmd, bench=False)
    if code != 0 or not checksum:
        return checksum, 0.0, code, {}

    for _ in range(warmup):
        run_once(cmd, bench=True)

    samples: list[float] = []
    for _ in range(runs):
        _, ms, code = run_once(cmd, bench=True)
        if code != 0:
            return checksum, ms, code, {}
        samples.append(ms)

    samples.sort()
    raw_median, median_ms = robust_median(samples)
    stats = {
        "ms_median": round(median_ms, 3),
        "ms_median_raw": round(raw_median, 3),
        "ms_min": round(samples[0], 3),
        "ms_max": round(samples[-1], 3),
        "ms_mean": round(statistics.mean(samples), 3),
        "ms_stdev": round(statistics.pstdev(samples), 3) if len(samples) > 1 else 0.0,
        "ms_samples": [round(x, 3) for x in samples],
        "warmup": warmup,
        "runs": runs,
        "trimmed": len(samples) >= 5,
    }
    return checksum, median_ms, 0, stats


def build_c(name: str) -> Path | None:
    clang = which("x86_64-w64-mingw32-clang") or which("clang") or which("gcc")
    if not clang:
        return None
    src = SUITE / f"{name}.c"
    dst = OUT / f"{name}_c"
    if os.name == "nt":
        dst = dst.with_suffix(".exe")
    native = [] if os.environ.get("CI") or os.environ.get("GITHUB_ACTIONS") else ["-march=native"]
    subprocess.check_call(
        [clang, "-O3", *native, "-I", str(SUITE), "-o", str(dst), str(src)],
        cwd=str(ROOT),
    )
    return dst


def build_rust(name: str) -> Path | None:
    rustc = which("rustc")
    if not rustc:
        return None
    src = SUITE / f"{name}.rs"
    dst = OUT / f"{name}_rs"
    if os.name == "nt":
        dst = dst.with_suffix(".exe")
    subprocess.check_call(
        [rustc, "-O", "-C", "lto=thin", "-o", str(dst), str(src)],
        cwd=str(SUITE),
    )
    return dst


def build_go(name: str) -> Path | None:
    go = which("go")
    if not go:
        return None
    src = SUITE / f"{name}.go"
    support = SUITE / "bench_runtime.go"
    dst = OUT / f"{name}_go"
    if os.name == "nt":
        dst = dst.with_suffix(".exe")
    env = os.environ.copy()
    env["CGO_ENABLED"] = "0"
    subprocess.check_call(
        [go, "build", "-ldflags=-s -w", "-o", str(dst), str(src), str(support)],
        cwd=str(SUITE),
        env=env,
    )
    return dst


def build_zig(name: str) -> Path | None:
    zig = which("zig")
    if not zig:
        return None
    src = SUITE / f"{name}.zig"
    dst = OUT / f"{name}_zig"
    if os.name == "nt":
        dst = dst.with_suffix(".exe")
    subprocess.check_call(
        [
            zig,
            "build-exe",
            "-O",
            "ReleaseFast",
            "-lc",
            f"-femit-bin={dst}",
            str(src),
        ],
        cwd=str(SUITE),
    )
    return dst


def resolve_pgo_use(pgo_use: Path | None, name: str) -> Path | None:
    """Per-workload profdata when ``pgo_use`` is a directory."""
    if pgo_use is None:
        return None
    if pgo_use.is_dir():
        candidate = pgo_use / f"{name}.profdata"
        if candidate.is_file():
            return candidate
        print(
            f"warning: missing PGO profile {candidate}, building without --pgo-use",
            file=sys.stderr,
        )
        return None
    return pgo_use


def build_rynix(name: str, *, bench: bool, pgo_use: Path | None) -> Path | None:
    rynixc = ROOT / "target" / "release" / ("rynixc.exe" if os.name == "nt" else "rynixc")
    if not rynixc.is_file():
        debug = ROOT / "target" / "debug" / ("rynixc.exe" if os.name == "nt" else "rynixc")
        rynixc = debug if debug.is_file() else None
    if rynixc is None:
        subprocess.check_call(["cargo", "build", "-p", "rynixc", "--release"], cwd=str(ROOT))
        rynixc = ROOT / "target" / "release" / ("rynixc.exe" if os.name == "nt" else "rynixc")
    src = SUITE / f"{name}.ryx"
    # Separate outputs: check binary prints; bench binary always-sinks (no getenv).
    dst = OUT / (f"{name}_rynix_bench" if bench else f"{name}_rynix_check")
    cmd = [
        str(rynixc),
        "build",
        str(src.relative_to(ROOT)).replace("\\", "/"),
        "-o",
        str(dst.relative_to(ROOT)).replace("\\", "/"),
        "--runtime=portable",
    ]
    if bench:
        cmd.append("--bench")
    profile = resolve_pgo_use(pgo_use, name)
    if profile is not None:
        cmd.append(f"--pgo-use={profile}")
    subprocess.check_call(cmd, cwd=str(ROOT))
    exe = dst.with_suffix(".exe") if (dst.with_suffix(".exe")).is_file() else dst
    return exe if exe.is_file() else None


def build_end(name: str) -> Path | None:
    """Build End peer when `endc`/`end` and `{name}.end` exist (see END_INTEGRATION.md).

    Copies the source into ``target/suite5/`` first so End's C11 emit does not
    overwrite ``benchmarks/suite5/{name}.c``.
    """
    endc = which("endc") or which("end")
    if not endc:
        return None
    src = SUITE / f"{name}.end"
    if not src.is_file():
        return None
    OUT.mkdir(parents=True, exist_ok=True)
    work_end = OUT / f"{name}_end_src.end"
    shutil.copy2(src, work_end)
    dst = OUT / f"{name}_end"
    if os.name == "nt":
        dst = dst.with_suffix(".exe")
    cmd_try = [endc, "build", str(work_end), "--strip", "-o", str(dst)]
    cmd = [endc, "build", str(work_end), "-o", str(dst)]
    try:
        subprocess.check_call(cmd_try, cwd=str(ROOT))
    except subprocess.CalledProcessError:
        subprocess.check_call(cmd, cwd=str(ROOT))
    # End writes sibling .c next to the .end input — remove scratch artifacts.
    for junk in (
        OUT / f"{name}_end_src.c",
        OUT / f"{name}_end_src.end",
    ):
        if junk.is_file():
            junk.unlink()
    exe = dst.with_suffix(".exe") if dst.with_suffix(".exe").is_file() else dst
    if not exe.is_file():
        raise RuntimeError(f"endc reported success but missing binary: {exe}")
    return exe


BUILDERS = {
    "c": lambda name, **_kw: build_c(name),
    "rust": lambda name, **_kw: build_rust(name),
    "go": lambda name, **_kw: build_go(name),
    "zig": lambda name, **_kw: build_zig(name),
    "rynix": build_rynix,
    "end": lambda name, **_kw: build_end(name),
}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--langs",
        default="c,rust,go,zig,rynix,end",
        help="comma list of languages to run",
    )
    ap.add_argument("--json-out", default=str(SUITE / "suite5_results.json"))
    ap.add_argument(
        "--summary",
        action="store_true",
        help="print cross-language matrix (median ms + vs-C ratio) at end",
    )
    ap.add_argument("--warmup", type=int, default=int(os.environ.get("SUITE5_WARMUP", "3")))
    ap.add_argument("--runs", type=int, default=int(os.environ.get("SUITE5_RUNS", "9")))
    pgo_env = os.environ.get("RYNIX_PGO_PROFDATA")
    ap.add_argument(
        "--pgo-use",
        type=Path,
        default=Path(pgo_env) if pgo_env else None,
        help="Rynix only: LLVM profile dir (…/pgo) or single .profdata (or RYNIX_PGO_PROFDATA)",
    )
    args = ap.parse_args()
    langs = [x.strip() for x in args.langs.split(",") if x.strip()]
    OUT.mkdir(parents=True, exist_ok=True)

    rows: list[dict] = []
    print("| Challenge | Lang | checksum | ms (median) |")
    print("|---|---|---:|---:|")
    for challenge in CHALLENGES:
        ref: str | None = None
        for lang in langs:
            builder = BUILDERS.get(lang)
            if not builder:
                print(f"skip unknown lang {lang}", file=sys.stderr)
                continue
            try:
                check_exe = None
                if lang == "rynix":
                    check_exe = builder(
                        challenge,
                        bench=False,
                        pgo_use=args.pgo_use,
                    )
                    exe = builder(
                        challenge,
                        bench=True,
                        pgo_use=args.pgo_use,
                    )
                else:
                    exe = builder(challenge)
            except Exception as e:  # noqa: BLE001 — report and continue
                print(f"| {challenge} | {lang} | BUILD_FAIL | — |  # {e}")
                rows.append(
                    {
                        "challenge": challenge,
                        "lang": lang,
                        "ok": False,
                        "error": str(e),
                    }
                )
                continue
            if exe is None or (lang == "rynix" and check_exe is None):
                print(f"| {challenge} | {lang} | SKIP (toolchain missing) | — |")
                rows.append(
                    {"challenge": challenge, "lang": lang, "ok": False, "skipped": True}
                )
                continue
            checksum, ms, code, stats = run_timed_median(
                [str(exe)],
                warmup=args.warmup,
                runs=args.runs,
                check_cmd=[str(check_exe)] if check_exe is not None else None,
            )
            ok = code == 0 and bool(checksum)
            if ref is None and ok:
                ref = checksum
            match = ok and checksum == ref
            mark = "OK" if match else ("MISMATCH" if ok else "FAIL")
            print(f"| {challenge} | {lang} | {checksum} {mark} | {ms:.2f} |")
            row = {
                "challenge": challenge,
                "lang": lang,
                "checksum": checksum,
                "ms": round(ms, 3),
                "ok": ok and match,
                "exit": code,
            }
            if stats:
                row["timing"] = stats
            rows.append(row)

    payload = {
        "schema": "rynix.suite5.v2",
        "challenges": list(CHALLENGES),
        "note": (
            "Checksum verified with full stdout (Rynix: non-`--bench` binary); "
            "timed runs use SUITE5_BENCH=1 and Rynix `--bench` always-sink RT "
            f"(warmup={args.warmup}, runs={args.runs}, reported ms=trimmed median when runs>=5)."
        ),
        "rows": rows,
    }
    Path(args.json_out).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"\nWrote {args.json_out}")
    print("TCP echo bake-off (separate): see docs/bakeoff.md")

    if args.summary:
        summary_langs = ["c", "rust", "go", "zig", "rynix", "end"]
        header = "| Challenge | " + " | ".join(summary_langs) + " | Rynix/C |"
        sep = "|---|" + "|".join(["---:"] * len(summary_langs)) + "|---:|"
        print(f"\n### Cross-language summary (median ms, warmup={args.warmup}, "
              f"runs={args.runs}; ratio vs C when present)\n")
        print(header)
        print(sep)
        by_challenge: dict[str, dict[str, dict]] = {}
        for row in rows:
            if not row.get("ok"):
                continue
            by_challenge.setdefault(row["challenge"], {})[row["lang"]] = row

        for challenge in CHALLENGES:
            bucket = by_challenge.get(challenge, {})

            def ms(lang: str) -> str:
                r = bucket.get(lang)
                return f"{r['ms']:.1f}" if r else "—"

            c_ms = bucket.get("c", {}).get("ms")
            r_ms = bucket.get("rynix", {}).get("ms")
            ratio = f"{r_ms / c_ms:.2f}×" if c_ms and r_ms else "—"
            cells = " | ".join(ms(lang) for lang in summary_langs)
            print(f"| {challenge} | {cells} | {ratio} |")

    critical = [
        r
        for r in rows
        if r["lang"] in ("c", "rynix") and not r.get("ok")
    ]
    if critical:
        print(
            f"\nWARNING: {len(critical)} required row(s) failed (C/Rynix checksum)",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
