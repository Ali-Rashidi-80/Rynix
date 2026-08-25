#!/usr/bin/env python3
"""Regenerate Suite5 .end ports for End@cf5bef3.

End peer HEAD breaks statement-form `if cond { ... }` (even suite12_end.end
fails to parse). Working forms: `while`, `for`, and expression
`a if cond else b`. This script only updates Rynix-owned ports — never End sources.
"""
from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "benchmarks" / "suite5"

HELPERS = '''@import_c("<stdio.h>")
@import_c("<stdlib.h>")

// End@cf5bef3 regressions (do not edit End): statement `if { }` fails to parse
// (suite12_end.end broken); expression `a if c else b` always yields 0.
// Ports use for/while + getenv-as-i64 only.
pub fn suite5_print_i64(n: i64) void {
  val env: *u8 = getenv("SUITE5_BENCH")
  mut has: i64 = env as i64
  while has == 0 {
    printf("%lld\\n", n)
    has = 1
  }
  val sink: i64 = n
  ret
}

pub fn opaque_i64(x: i64) i64 {
  val env: *u8 = getenv("SUITE5_OPAQUE")
  val touch: i64 = env as i64
  ret x + touch - touch
}
'''

BODIES: dict[str, str] = {
    "alu": '''
pub fn main() i32 {
  val n: i64 = opaque_i64(2000000)
  mut acc: i64 = 0
  for i_idx in n {
    val i: i64 = i_idx as i64
    acc = acc + i * 3 - i / 2 + i % 7
  }
  suite5_print_i64(acc)
  ret 0
}
''',
    "nested": '''
pub fn main() i32 {
  val n: i64 = opaque_i64(450)
  mut s: i64 = 0
  for i_idx in n {
    val i: i64 = i_idx as i64
    for j_idx in n {
      val j: i64 = j_idx as i64
      s = s + (i * j + i) % 97
    }
  }
  suite5_print_i64(s)
  ret 0
}
''',
    "fib": '''
pub fn main() i32 {
  val n: i64 = opaque_i64(5000000)
  mut a: i64 = 0
  mut b: i64 = 1
  for _i in n {
    val t: i64 = a + b
    a = b
    b = t
  }
  suite5_print_i64(a)
  ret 0
}
''',
    "hash": '''
pub fn main() i32 {
  val n: i64 = opaque_i64(3000000)
  mut h: i64 = 0
  for i_idx in n {
    val i: i64 = i_idx as i64
    h = (h * 31 + i) % 1000000007
  }
  suite5_print_i64(h)
  ret 0
}
''',
    "prime": '''
pub fn main() i32 {
  val lim: i64 = opaque_i64(99999)
  mut count: i64 = 0
  for i_raw in lim {
    val i: i64 = (i_raw as i64) + 2
    mut prime: i64 = 1
    mut j: i64 = 2
    while j * j <= i {
      mut rem: i64 = i % j
      while rem == 0 {
        prime = 0
        rem = 1
      }
      j = j + 1
    }
    count = count + prime
  }
  suite5_print_i64(count)
  ret 0
}
''',
    "sum": '''
pub fn main() i32 {
  val n: i64 = opaque_i64(1500000)
  mut acc: i64 = 0
  for i_idx in n {
    val i: i64 = i_idx as i64
    acc = acc + i * i
  }
  suite5_print_i64(acc)
  ret 0
}
''',
    "bits": '''
pub fn popcount(x: i64) i64 {
  mut v: i64 = x
  mut c: i64 = 0
  for _bit in 64 {
    mut bit: i64 = v & 1
    mut live: i64 = v
    // add bit only while v still nonzero (same as early-break popcount)
    while live != 0 {
      c = c + bit
      live = 0
    }
    v = v / 2
  }
  ret c
}

pub fn main() i32 {
  val n: i64 = opaque_i64(25000000)
  mut x: i64 = 1
  mut acc: i64 = 0
  for i_idx in n {
    val i: i64 = i_idx as i64
    x = (x * 31 + i) % 1000000007
    acc = acc + popcount(x)
  }
  suite5_print_i64(acc)
  ret 0
}
''',
    "matrix": '''
pub fn cell(i: i64, j: i64) i64 {
  mut s: i64 = 0
  for k_idx in 4 {
    val k: i64 = k_idx as i64
    val av: i64 = i + k
    val bv: i64 = k * j + 1
    s = s + av * bv
  }
  ret s
}

pub fn main() i32 {
  val c00: i64 = cell(0, 0)
  val c11: i64 = cell(1, 1)
  val c22: i64 = cell(2, 2)
  val c33: i64 = cell(3, 3)
  val per: i64 = opaque_i64(225000)
  val trace: i64 = per * (c00 + c11 + c22 + c33)
  suite5_print_i64(trace)
  ret 0
}
''',
    "scan": '''
pub fn main() i32 {
  val n: i64 = opaque_i64(8000000)
  mut acc: i64 = 0
  for i_idx in n {
    val i: i64 = i_idx as i64
    mut rem3: i64 = i % 3
    mut rem7: i64 = i % 7
    mut add: i64 = 0
    while rem3 == 0 {
      add = 1
      rem3 = 1
    }
    while rem7 == 0 {
      add = 1
      rem7 = 1
    }
    acc = acc + add
  }
  suite5_print_i64(acc)
  ret 0
}
''',
    "powmod": '''
pub fn main() i32 {
  val n: i64 = opaque_i64(2500000)
  mut acc: i64 = 1
  val base: i64 = 3
  for _i in n {
    acc = (acc * base) % 1000000007
  }
  suite5_print_i64(acc)
  ret 0
}
''',
    "gcd": '''
pub fn gcd64(a: i64, b: i64) i64 {
  mut x: i64 = a
  mut y: i64 = b
  while y != 0 {
    val t: i64 = x % y
    x = y
    y = t
  }
  ret x
}

pub fn main() i32 {
  val n: i64 = opaque_i64(2500000)
  mut acc: i64 = 0
  for i_raw in n {
    val i: i64 = (i_raw as i64) + 1
    val a: i64 = i * 9973
    val b: i64 = i * 1237 + 42
    acc = acc + gcd64(a, b)
  }
  suite5_print_i64(acc)
  ret 0
}
''',
    "reduce": '''
pub fn main() i32 {
  val n: i64 = opaque_i64(10000000)
  mut acc: i64 = 0
  for i_idx in n {
    val i: i64 = i_idx as i64
    acc = acc + i * 31 - i / 8 + i % 13
  }
  suite5_print_i64(acc)
  ret 0
}
''',
}


def main() -> None:
    for name, body in BODIES.items():
        path = OUT / f"{name}.end"
        text = HELPERS + body
        path.write_text(text, encoding="utf-8", newline="\n")
        print(f"wrote {path.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
