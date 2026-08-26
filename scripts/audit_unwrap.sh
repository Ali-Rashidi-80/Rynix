#!/usr/bin/env bash
# Count .unwrap() / .expect( in crates/*/src excluding tests.rs and cfg(test) modules.
# Budget: ≤ 60 (Phase 26 / GOLDEN_PATH).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
python3 - <<'PY' "$ROOT"
import re, sys
from pathlib import Path
root = Path(sys.argv[1])
pat = re.compile(r"\.(unwrap|expect)\s*\(")
count = 0
for p in (root / "crates").rglob("*.rs"):
    s = str(p).replace("\\", "/")
    if "/src/" not in s or p.name == "tests.rs" or "/tests/" in s:
        continue
    lines = p.read_text(encoding="utf-8", errors="replace").splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        if "#[cfg(test)]" in line:
            j = i + 1
            while j < len(lines) and "mod " not in lines[j] and "fn " not in lines[j]:
                j += 1
            if j < len(lines) and lines[j].lstrip().startswith("mod "):
                while j < len(lines) and "{" not in lines[j]:
                    j += 1
                depth = lines[j].count("{") - lines[j].count("}")
                j += 1
                while j < len(lines) and depth > 0:
                    depth += lines[j].count("{") - lines[j].count("}")
                    j += 1
                i = j
                continue
        if pat.search(line) and not line.strip().startswith("//"):
            count += 1
        i += 1
budget = 60
print(f"unwrap_expect_src={count} budget={budget}")
sys.exit(0 if count <= budget else 1)
PY
