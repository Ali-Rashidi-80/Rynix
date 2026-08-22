#!/usr/bin/env python3
"""One-shot Suite5 PGO workflow: train → baseline (C+Rynix) → PGO Rynix → analyze.

Usage (repo root, release rynixc built):
  python benchmarks/suite5/run_pgo_suite.py
  python benchmarks/suite5/run_pgo_suite.py --full   # all 5 langs for baseline JSON
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SUITE = Path(__file__).resolve().parent


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--full",
        action="store_true",
        help="baseline run uses all langs (slower); default c,rynix only",
    )
    ap.add_argument(
        "--skip-train",
        action="store_true",
        help="reuse existing target/suite5/pgo/*.profdata",
    )
    args = ap.parse_args()

    if not args.skip_train:
        print("==> PGO train (12 workloads)", flush=True)
        subprocess.check_call(
            [sys.executable, str(SUITE / "pgo_train.py")],
            cwd=str(ROOT),
        )

    langs = "c,rust,go,zig,rynix" if args.full else "c,rynix"
    print(f"==> Baseline benchmark ({langs})", flush=True)
    subprocess.check_call(
        [
            sys.executable,
            str(SUITE / "run_suite5.py"),
            "--langs",
            langs,
            "--json-out",
            str(SUITE / "suite5_results.json"),
        ],
        cwd=str(ROOT),
    )

    print("==> Rynix + PGO benchmark", flush=True)
    subprocess.check_call(
        [
            sys.executable,
            str(SUITE / "run_suite5.py"),
            "--langs",
            "rynix",
            "--pgo-use",
            "target/suite5/pgo",
            "--json-out",
            str(SUITE / "suite5_results_pgo.json"),
        ],
        cwd=str(ROOT),
    )

    print("==> Analysis", flush=True)
    subprocess.check_call(
        [sys.executable, str(SUITE / "analyze_results.py")],
        cwd=str(ROOT),
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
