#!/usr/bin/env python3
"""Train LLVM PGO profiles for Rynix Suite5 binaries (one profdata per workload).

Usage (repo root, after release rynixc):
  python benchmarks/suite5/pgo_train.py
  python benchmarks/suite5/run_suite5.py --langs rynix --pgo-use target/suite5/pgo
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SUITE = Path(__file__).resolve().parent
OUT = ROOT / "target" / "suite5"
PGO_DIR = OUT / "pgo"

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


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    PGO_DIR.mkdir(parents=True, exist_ok=True)
    rynixc = ROOT / "target" / "release" / (
        "rynixc.exe" if os.name == "nt" else "rynixc"
    )
    if not rynixc.is_file():
        subprocess.check_call(
            ["cargo", "build", "-p", "rynixc", "--release"], cwd=str(ROOT)
        )

    llvm_profdata = (
        "llvm-profdata"
        if os.name != "nt"
        else "llvm-profdata.exe"
    )

    for name in CHALLENGES:
        src = SUITE / f"{name}.ryx"
        dst = OUT / f"{name}_pgo_train"
        profraw = PGO_DIR / f"{name}.profraw"
        profdata = PGO_DIR / f"{name}.profdata"
        if profdata.is_file():
            profdata.unlink()

        subprocess.check_call(
            [
                str(rynixc),
                "build",
                str(src.relative_to(ROOT)).replace("\\", "/"),
                "-o",
                str(dst.relative_to(ROOT)).replace("\\", "/"),
                "--runtime=portable",
                "--bench",
                "--pgo-gen",
            ],
            cwd=str(ROOT),
        )
        exe = dst.with_suffix(".exe") if dst.with_suffix(".exe").is_file() else dst
        env = os.environ.copy()
        env["SUITE5_BENCH"] = "1"
        profraws: list[Path] = []
        train_runs = int(os.environ.get("SUITE5_PGO_TRAIN_RUNS", "3"))
        print(f"train run: {name} ({train_runs}× SUITE5_BENCH=1)", flush=True)
        for run in range(train_runs):
            profraw = PGO_DIR / f"{name}.{run}.profraw"
            if profraw.is_file():
                profraw.unlink()
            env["LLVM_PROFILE_FILE"] = str(profraw)
            subprocess.check_call([str(exe)], cwd=str(ROOT), env=env)
            if not profraw.is_file():
                print(f"error: no profraw for {name} run {run}", file=sys.stderr)
                return 1
            profraws.append(profraw)

        subprocess.check_call(
            [llvm_profdata, "merge", "-output", str(profdata), *[str(p) for p in profraws]],
            cwd=str(ROOT),
        )
        for p in profraws:
            p.unlink(missing_ok=True)
        print(f"  wrote {profdata.relative_to(ROOT)}")

    print(f"\nPGO profiles: {PGO_DIR.relative_to(ROOT)}/<workload>.profdata")
    print(
        "Benchmark: python benchmarks/suite5/run_suite5.py --langs rynix "
        f"--pgo-use {PGO_DIR.relative_to(ROOT)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
