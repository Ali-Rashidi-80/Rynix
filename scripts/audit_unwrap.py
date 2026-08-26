"""Count .unwrap()/.expect( in crates/*/src excluding tests. Exit 1 if > budget."""
from __future__ import annotations

import re
import sys
from pathlib import Path

BUDGET = 60
PAT = re.compile(r"\.(unwrap|expect)\s*\(")


def count_root(root: Path) -> int:
    total = 0
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
            if PAT.search(line) and not line.strip().startswith("//"):
                total += 1
            i += 1
    return total


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    n = count_root(root)
    print(f"unwrap_expect_src={n} budget={BUDGET}")
    return 0 if n <= BUDGET else 1


if __name__ == "__main__":
    sys.exit(main())
