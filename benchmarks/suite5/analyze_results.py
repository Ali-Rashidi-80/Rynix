#!/usr/bin/env python3
import json
from pathlib import Path

d = json.loads(Path("benchmarks/suite5/suite5_results.json").read_text())
all_langs = ["c", "rust", "go", "zig", "rynix", "end"]
langs = [l for l in all_langs if any(x["lang"] == l for x in d["rows"])]
challenges = d["challenges"]

# Suite5 trip counts go through opaque_* so timed binaries keep real loops.
# Literal-bound fold still exists in the compiler (see rynix-rir fold_fixtures tests).


def row_for(rows, ch: str, lang: str):
    return next((x for x in rows if x["challenge"] == ch and x["lang"] == lang), None)


print("### CoV (stdev/median) — lower = more stable")
print("| Challenge | " + " | ".join(langs) + " |")
print("|---|" + "|".join(["---:"] * len(langs)) + "|")
for ch in challenges:
    row = []
    for lang in langs:
        r = row_for(d["rows"], ch, lang)
        if not r or not r.get("ok") or "timing" not in r:
            row.append("-")
            continue
        t = r["timing"]
        cv = 100 * t["ms_stdev"] / t["ms_median"] if t["ms_median"] else 0
        row.append(f"{cv:.0f}%")
    print(f"| {ch} | " + " | ".join(row) + " |")

print("\n### Full median ms matrix (runtime loops; opaque trip counts)")
header_langs = ["c", "rust", "go", "zig", "rynix", "end"]
print(
    "| Workload | "
    + " | ".join(x.capitalize() if x != "rynix" else "Rynix" for x in header_langs)
    + " | Best |"
)
print("|---|" + "|".join(["--:"] * len(header_langs)) + "|---|")
for ch in challenges:
    cells = []
    best = ("", 1e9)
    for lang in all_langs:
        r = row_for(d["rows"], ch, lang)
        ms = r.get("ms") if r and r.get("ok") else None
        if ms is None:
            cells.append("-")
            continue
        cells.append(f"{ms:.2f}")
        if ms < best[1]:
            best = (lang, ms)
    best_s = f"{best[0]} {best[1]:.2f}" if best[0] else "-"
    print(f"| {ch} | " + " | ".join(cells) + f" | {best_s} |")

print("\n### Rynix vs C ratio (same-work runtime)")
for ch in challenges:
    c_row = row_for(d["rows"], ch, "c")
    r_row = row_for(d["rows"], ch, "rynix")
    if not c_row or c_row.get("ms") is None:
        print(f"{ch:8} rynix/c = - (no C row)")
        continue
    r_ms = r_row.get("ms") if r_row else None
    if r_ms is None:
        print(f"{ch:8} rynix/c = - (build fail)")
        continue
    print(f"{ch:8} rynix/c = {r_ms/c_row['ms']:.3f}x")

print("\n### Rynix rank & gap to fastest")
for ch in challenges:
    times = []
    for lang in langs:
        r = row_for(d["rows"], ch, lang)
        if r and r.get("ok"):
            times.append((r["ms"], lang, r.get("timing", {})))
    if not times:
        continue
    ryn = next(((ms, t) for ms, lang, t in times if lang == "rynix"), None)
    if ryn is None:
        print(f"{ch:8} rank -/{len(langs)}  rynix missing")
        continue
    ryn_ms, _ = ryn
    times.sort(key=lambda x: x[0])
    rank = next(i + 1 for i, (_, l, _) in enumerate(times) if l == "rynix")
    n = len(times)
    best_ms, best_lang, _ = times[0]
    gap = (ryn_ms / best_ms - 1) * 100
    print(
        f"{ch:8} rank {rank}/{n}  rynix={ryn_ms:7.2f}  "
        f"best={best_lang:5} {best_ms:7.2f}  gap={gap:+5.1f}%"
    )

pgo_path = Path("benchmarks/suite5/suite5_results_pgo.json")
if pgo_path.is_file():
    pgo = json.loads(pgo_path.read_text())
    print("\n### PGO delta (rynix baseline -> pgo-use)")
    print("| Workload | baseline | pgo | delta |")
    print("|---|--:|--:|--:|")
    for ch in challenges:
        base = next(
            (x for x in d["rows"] if x["challenge"] == ch and x["lang"] == "rynix" and x.get("ok")),
            None,
        )
        opt = next(
            (x for x in pgo["rows"] if x["challenge"] == ch and x["lang"] == "rynix" and x.get("ok")),
            None,
        )
        if base and opt:
            delta = (opt["ms"] / base["ms"] - 1) * 100
            print(f"| {ch} | {base['ms']:.2f} | {opt['ms']:.2f} | {delta:+.1f}% |")
