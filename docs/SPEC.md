# The Rynix Language Specification — v0.1

Status: the **lexical structure** (section 2) is normative and matches the
implementation in `crates/rynix-lexer` exactly; the **syntactic grammar**
(section 3) is a draft implemented in Phase 2.

Rynix is canonical by design: for every construct there is exactly one
surface form. The grammar deliberately has no alternative spellings, no
optional semicolons, and no redundant loop/branch forms. This minimizes LLM
token consumption and removes degrees of freedom that models (and humans)
would otherwise have to choose between.

## 1. Source text

- Encoding: UTF-8. A leading BOM (`U+FEFF`) is stripped at load time.
- Total source per compilation session is capped at 4 GiB (spans are `u32`
  offsets into a global address space; see ADR-0003).
- Line terminators: `\n`, `\r\n`, or a lone `\r` — each produces exactly one
  `Newline` token.
- Whitespace: space (`U+0020`) and tab (`U+0009`) only. Any other control
  character is a lexical error (`RYX0001`).

## 2. Lexical structure (normative)

The lexer is *total*: every UTF-8 input produces a token stream that tiles
the input byte-exactly (every byte belongs to exactly one token; `Eof` is the
only empty token). Errors never abort lexing; they attach structured
diagnostics (`RYX0001..RYX0006`, see [diagnostics.md](diagnostics.md)) to
recovery tokens.

### 2.1 Comments

```
comment      = "#" { any-char-except-newline }
doc_comment  = "##" { any-char-except-newline }
```

`#` runs to the end of the line (`\n` or `\r`). `##` is a documentation
comment (attached to the following item in Phase 2+). There is no block
comment and no `//` form — one way only.

### 2.2 Identifiers and keywords

```
ident = ident_start { ident_continue }
ident_start    = "A".."Z" | "a".."z" | "_"
ident_continue = ident_start | "0".."9"
```

Identifiers are ASCII-only in v0.1 (ADR-0002). Non-ASCII text outside
strings/comments produces `RYX0003` with a recovery token.

Keywords (27, each lexed as its own token kind):

```
def end let mut if elif else loop for in break continue return
struct enum type import pub true false nil and or not as spawn region
match   # live; agent/signal/tensor remain reserved
struct enum type import pub true false nil and or not as spawn match
```

Reserved for future use (3): `agent signal tensor`.

Notes on canonical choices:

- Logical operators are the words `and`, `or`, `not` — there is no `&&`,
  `||`, or prefix `!` (but `!=` exists; a lone `!` gets a fix-it suggesting
  `not`).
- There is no `while`: iteration is `for x in iterable` and infinite loops
  are `loop`; both terminate with `end`.

### 2.3 Integer literals

```
int      = dec_int | hex_int | oct_int | bin_int
dec_int  = digit { digit | "_" }
hex_int  = "0x" hex_digit { hex_digit | "_" }
oct_int  = "0o" oct_digit { oct_digit | "_" }
bin_int  = "0b" bin_digit { bin_digit | "_" }
```

- Base prefixes are lowercase only; `0X`, `0O`, `0B` produce `RYX0004` with a
  high-confidence lowercase fix.
- Underscores must separate digits: each `_` must have a digit on both
  sides (`1_000` is valid; `1__0`, `1_`, `0x_1`, `1_.5` produce `RYX0004`).
- Type suffixes do not exist in v0.1 (`123abc` produces `RYX0004`); types
  come from inference or `as` casts.

### 2.4 Float literals

```
float = dec_int "." dec_int [ exponent ] | dec_int exponent
exponent = "e" [ "+" | "-" ] dec_int
```

- Digits are required on both sides of `.` (`1.` and `.5` are not floats;
  they lex as `IntLit Dot` / `Dot IntLit`).
- The exponent marker is lowercase `e` only; `1E5` produces `RYX0004` with a
  lowercase fix.

### 2.5 String literals

```
string = '"' { string_char | escape } '"'
escape = "\n" | "\t" | "\r" | "\0" | "\\" | "\""
       | "\x" hex_digit hex_digit
       | "\u{" hex_digit{1..6} "}"
```

- Strings are single-line: a raw line terminator ends the token with
  `RYX0002` (fix: insert `"` before the newline). End of file inside a string
  produces `RYX0006`.
- `\u{...}` must encode a Unicode scalar value (rejects surrogates and values
  above `0x10FFFF`) — `RYX0005` otherwise.
- Unknown escapes produce `RYX0005`; lexing continues.
- There is exactly one string form in the shipping grammar: single-line
  `"..."` with the escapes above. (No multiline/raw string literal form.)

### 2.6 Punctuation and operators (30)

```
( ) [ ] { }            grouping, indexing, struct literals (named fields)
, . .. ..= : :: ->     separators, ranges, paths, return type
= == != < <= > >=      assignment and comparison
+ - * / %              arithmetic
+= -= *= /= %=         compound assignment
```

### 2.7 Newlines and statement termination

`Newline` is a real token: statements terminate at newlines, and there are no
semicolons. The parser (not the lexer) ignores `Newline` tokens inside
`( ... )` and `[ ... ]` and `{ ... }` groups, so long expressions wrap
naturally. The lexer stays context-free.

### 2.8 Token kinds (implementation reference)

70 kinds: `IntLit FloatLit StrLit Ident`, 30 keyword kinds, 30 punctuation
kinds, and the structural kinds `Newline Whitespace Comment DocComment
Unknown Eof`. `Whitespace` and `Comment` are trivia (skipped by the parser);
`Newline` and `DocComment` are significant.

## 3. Syntactic grammar (draft — Phase 2)

Blocks are keyword-delimited: a header line opens a block and `end` closes
it. Indentation is not significant.

```
module      = { Newline } { item { Newline } }
item        = fn_def | struct_def | enum_def | type_alias | import

import      = "import" path Newline
type_alias  = "type" Ident "=" type Newline

fn_def      = [ "pub" ] "def" Ident "(" [ params ] ")" [ "->" type ] Newline
              block "end"
params      = param { "," param }
param       = Ident ":" type

struct_def  = [ "pub" ] "struct" Ident Newline { field Newline } "end"
field       = Ident ":" type

enum_def    = [ "pub" ] "enum" Ident Newline { Ident [ "(" type ")" ] Newline } "end"

block       = { stmt }
stmt        = let_stmt | return_stmt | break_stmt | continue_stmt
            | loop_stmt | for_stmt | if_stmt | match_stmt | expr_stmt
            | assign_stmt
let_stmt    = "let" [ "mut" ] Ident [ ":" type ] "=" expr Newline
assign_stmt = place "=" expr Newline
            | place ("+=" | "-=" | "*=" | "/=" | "%=") expr Newline
place       = path | place "." Ident | place "[" expr "]"
return_stmt = "return" [ expr ] Newline
break_stmt  = "break" Newline
continue_stmt = "continue" Newline
loop_stmt   = "loop" Newline block "end"
for_stmt    = "for" Ident "in" expr Newline block "end"
if_stmt     = "if" expr Newline block { "elif" expr Newline block }
              [ "else" Newline block ] "end"
match_stmt  = "match" expr Newline { match_arm } [ "else" Newline block ] "end"
match_arm   = match_pat Newline block
match_pat   = IntLit | "true" | "false" | "_" | path
              ; path = bare nullary variant or `Enum::Variant` (ADR-0015);
              ; arm header must end the line (not `Ident(...)` / `Ident = …`)
expr_stmt   = expr Newline

expr        = … | struct_lit | …
struct_lit  = path "{" [ field_init { "," field_init } [ "," ] ] "}"
field_init  = Ident ":" expr

type        = path [ "[" type { "," type } "]" ] | "[" type "]"
path        = Ident { "::" Ident }
```

**Struct literals (v1):** named fields only — `Point { x: 1, y: 2 }`.
Field types in a literal are **`i64` or `str`** (Phase 17-A); other field types
are rejected. Enum *nullary* values are supported as discriminant `i64`s
(Phase 17-C); payload constructors remain deferred.

Field assignment `p.x = …` is allowed when `p` is a `mut` binding.
Index assignment (`a[i] = …`) is allowed when `a` is a `mut` array/slice
binding (Phase 17-B).

Reference types (`&T`) are not part of the shipping type grammar; ownership
is inferred via escape analysis, not surface `&` syntax.

Linear values (`Vec`, `Map`, user `struct`, slices, opaque `ptr`)
move on `let` binding from a path, assignment from a path, or call/pipe
argument. Using a moved binding is `RYX2011`. Scalars (`i64`, `f64`, `bool`,
`str`) and nullary enum discriminants copy. Exclusive borrow conflicts
(`&` + mutate) are deferred until reference types are specified.

Expression precedence (weakest to strongest, comparisons non-associative):

```
1  or
2  and
3  not (prefix)
4  == != < <= > >=
5  .. ..=
6  + -
7  * / %
8  unary -
9  as
10 call () | index [] | field . | method .name()
```

## 3.2 Pipeline `|>`

```
let y = 21 |> double
let z = 1 |> add(2)   # desugars to add(1, 2)
```

`lhs |> name` becomes `name(lhs)`.  
`lhs |> name(args…)` becomes `name(lhs, args…)`.  
Right-hand side must be a path or call (not an arbitrary expression).

## 3.3 Effect annotations (`#^ effect:`)

Functions may declare purity with a same-line directive:

```
def add(a: i64, b: i64) -> i64  #^ effect: pure
  return a + b
end
```

`#^ effect: pure` (alias `#^ effects: pure`) means the function must not
transitively perform `io` or `network`. Soft builtins such as `print`,
`http_*`, `tcp_*`, and `kv_*` are impure; `json_*` / arithmetic stay pure.
Violations emit `RYX2012`. OS sandboxing is out of scope; this is a static
toolchain check (also exercised by `rynixc check` and verify contracts).


## 4. Design pillars (context)

1. Rust compiler core, zero-allocation front-end (mmap + arenas).
2. AI-native canonical syntax; structured JSON/MCP diagnostics with
   confidence-scored fixes.
3. Deterministic Zero-GC memory: interprocedural escape analysis + implicit
   region inference + compiler-injected frees.
4. Colorless concurrency: fibers on a thread-per-core io_uring scheduler; no
   async/await.
5. Direct LLVM IR emission, whole-program DCE, LTO; binaries under 1MB.

## 5. Soft std surface (v0.1)

The compiler recognizes these **soft builtins** (no `import` required). They
lower to `rynix_rt_*` symbols documented in [abi.md](abi.md):

| Builtin | Role |
|---------|------|
| `print` / `print_i64` | stdout |
| `sleep_ms`, `yield`, `now_ms`, `fiber_run` | fibers / time |
| `vec_new`, `vec_push`, `vec_get`, `vec_len` | mono `Vec[i64]` |
| `map_new`, `map_insert`, `map_get`, `map_len` | mono `Map[i64,i64]` |
| `tcp_listen`, `tcp_accept`, `tcp_connect`, `tcp_recv`, `tcp_send`, `tcp_close` | TCP |
| `json_get_i64(body, key)` | minimal JSON int field |
| `json_has_i64(body, key)` | 1 if int field present |
| `http_get_json_i64(host, port, path, field)` | HTTP GET + JSON field |
| `http_post_json_i64(host, port, path, body, field)` | HTTP POST JSON + response field |
| `http_serve_once_json_i64(port, path, value)` | one-shot HTTP JSON server |
| `http_serve_once_echo_json_i64(port, path, field)` | one-shot echo request JSON field |
| `http_serve_loop_json_i64(port, path, value, max_reqs)` | bounded: exactly `max_reqs` matching GETs → `0` |
| `http_serve_loop_2paths_json_i64(port, path_a, val_a, path_b, val_b, max_reqs)` | dual-path bounded loop (either path counts) |
| `http_serve_loop_3paths_json_i64(port, path_a, val_a, path_b, val_b, path_c, val_c, max_reqs)` | triple-path bounded loop (any listed path counts) |
| `http_serve_loop_path_param_json_i64(port, prefix, max_reqs)` | GET `{prefix}{digits}` → JSON value = parsed i64 |
| `http_serve_loop_header_json_i64(port, path, header, max_reqs)` | GET `path` + header decimal → JSON value |
| `http_serve_loop_post_echo_json_i64(port, path, field, max_reqs, max_body)` | bounded POST echo; body > max_body → 400 |
| `http_serve_loop_keepalive_json_i64(port, path, value, max_reqs)` | one accept; up to max_reqs GETs on same conn |
| `http_tls_serve_once_json_i64` / `http_tls_get_json_i64` | HTTP JSON over TLS (SChannel/OpenSSL; else `-2`) |
| `frame_serve_once_echo` / `frame_client_echo` | length-prefixed binary frame echo |
| `tls_serve_once_echo` / `tls_client_echo` | TLS echo (real SChannel/OpenSSL) |
| `sha256_first_i64(data)` | SHA-256 → first 8 bytes as i64 |
| `hmac_sha256_first_i64(key, data)` | HMAC-SHA256 → first 8 bytes as i64 |
| `aes128_gcm_nist_empty_tag_first_i64()` | AES-GCM NIST KAT helper |
| `ws_accept_key_eq` / `ws_accept_sha1_first_i64` / `ws_frame_roundtrip_ok` | WebSocket handshake/frame helpers |
| `kv_new` / `kv_put` / `kv_get` / `kv_len` | arena string→i64 map |
| `fs_write_file(path, data)` | write whole file (0 / -1) |
| `fs_read_file(path)` | read whole file as `str` (or fail) |
| `fs_read_file_eq(path, expect)` | compare file to string (0 / -1) |
| `fs_exists(path)` | `1` if readable file, else `0` |
| `fs_remove_file(path)` | unlink (`0`; missing path is ok) |

Reserved keywords (not soft callables; calls → `RYX2013`): `tensor`, `signal`, `agent`.

Notes in `std/*.ryx` that contain **no** `def` remain documentation for soft
builtins (e.g. `std/http.ryx`, `std/tls.ryx`). Modules with real `def`s
(`std/math.ryx`, `std/fs.ryx`, `std/crypto.ryx` SHA only) load via
`import std::<module>` (SPEC §6.5).

## 6. Packages & local index (v0.1)

Packages are directories with a `rynix.toml` manifest. There is **no** network
registry ([ADR-0010](adr/0010-local-package-index.md)).

### 6.1 Path dependencies

```toml
[dependencies]
util = { path = "../util" }
```

Resolved relative to the manifest directory. Missing dirs / `rynix.toml` fail
`rynixc deps` and `rynixc build`.

### 6.2 Local filesystem index

```toml
[registry]
path = "vendor"

[dependencies]
util = "0.1.0"
```

Exact version strings resolve to, in order:

1. `{registry}/{name}/{version}/` (must contain `rynix.toml`)
2. `{registry}/{name}-{version}/`

Semver ranges for the local index (`^`, `>=`, `=` — Cargo-compatible; for
`0.y.z`, `^0.y.z` stays on the `0.y` line), downloads, and mirrors remain
out of scope for CDN. Exact directory names still work; ranges pick the
**highest** matching `{registry}/{name}/{version}/` folder
([ADR-0010](adr/0010-local-package-index.md)).

### 6.2.1 Local sparse index (no CDN)

When `{registry}/index/config.json` exists (or `[registry] sparse = true`),
resolve versions from a Cargo-style **local** crate file instead of scanning
every `{name}/{version}/` directory:

```text
vendor/index/config.json          # presence selects sparse mode
vendor/index/ut/il/util           # NDJSON: {"name","vers", optional "yanked"}
vendor/util/0.2.0/                # package sources (must exist for the chosen vers)
```

Prefix rules match crates.io sparse: 1-char `1/{name}`, 2-char `2/{name}`,
3-char `3/{a}/{name}`, else `{aa}/{bb}/{name}`. A `.json` suffix on the crate
file is accepted. `yanked: true` lines are skipped. Unlisted version
directories (even if they contain `rynix.toml`) must not be selected.
`config.json` is not fetched over the network; `dl` / `api` keys are ignored.

`rynixc deps --error-format=json` reports `registry_index: "sparse" | "scan"`.

Evidence: `testdata/pkg_sparse_app`, `deps_resolves_sparse_local_index`,
`build_pkg_sparse_app_resolves_index`.

### 6.3 Unity compile of dependency sources

`rynixc build` / `emit-ll` load each resolved dependency’s sources **before**
the app and parse them as **one** compilation unit. Sources are:

1. `[package].entry` (required)
2. Optional `[package].files = ["a.ryx", …]` extras in manifest order

```toml
[package]
name = "util"
entry = "lib.ryx"
files = ["extra.ryx"]
```

There is no blind `**/*.ryx` scan. Dependency `def` names are mangled to
`pkg__fn` (double underscore). App bare calls to unique exports are rewritten
to the mangled name; `pkg.fn(...)` is rewritten the same way. Soft builtins
stay unmangled.

Rules:

- Every declared dependency must have a resolvable `entry` file at compile time
- Extra `files` paths are relative to the package directory and must exist
- Dependency sources must **not** define `def main`
- Duplicate bare `def` names across packages are a compile error
- Transitive deps are included; network registries stay out of scope
- Soft `std` builtins remain injected by sema

Evidence: `testdata/pkg_app`, `testdata/pkg_util` (`files = ["extra.ryx"]`),
`testdata/pkg_reg_app`, `testdata/pkg_sparse_app`, `build_pkg_app_calls_path_dep`,
`build_pkg_reg_app_resolves_registry_deps`, `build_pkg_sparse_app_resolves_index`.

### 6.3.1 Local lockfile (`rynix.lock.toml`)

`rynixc deps --lock` writes `rynix.lock.toml` beside the root manifest with
per-dep `sha256` over ordered sources (no network CDN). If a lockfile is
present, `deps` / `build` / `emit-ll` verify pins. `deps --locked` fails when
the lockfile is missing.

Evidence: `deps_lock_write_verify_and_tamper`.

### 6.3.2 Local digest attest (`rynix.attest.v1.json`)

`rynixc deps --attest` writes `rynix.lock.toml` and a sibling
`rynix.attest.v1.json` with `kind: "local_digest"`: SHA-256 of the lock file
plus the same per-package pins. `deps --attest-verify` fails if the file is
missing or `lock_sha256` / pins do not match.

This is **not** Sigstore (no Rekor, Fulcio, OIDC, or transparency log). Schema:
`docs/schemas/rynix.attest.v1.json`.

Evidence: `deps_attest_write_verify_and_tamper`.

### 6.4 `import` and qualified calls

```ryx
import util

def main() -> i64
  return util.util_answer()
end
```

`import name` binds `name` as a module. A call `name.fn(...)` resolves to the
mangled `name__fn` symbol in the unity unit.

Evidence: `testdata/pkg_import_app`, `build_pkg_import_app_qualified_call`.

Transitive path/registry dependencies are resolved depth-first (dependents after
their deps) and included in the unity unit. Cycles fail resolve.

Evidence: `testdata/pkg_core` ← `pkg_util` ← `pkg_app`,
`deps_resolves_transitive_core_before_util`.

### 6.5 Real `std/*.ryx` loader

`import std::<module>` loads `{toolchain}/std/<module>.ryx` into the unity unit
when that file contains at least one `def`. Modules that are docs-only (no
`def`) are skipped — soft builtins (§5) remain the ABI for those symbols.

```ryx
import std::math

def main() -> i64
  return math.add3(40, 1, 1)
end
```

`import std::fs` / `import std::crypto` load thin `def` wrappers over soft
`fs_*` / `sha256_first_i64` (HMAC/AES stay soft-only; no facade this phase).

Evidence: `std/math.ryx`, `testdata/pkg_std_app`, `build_pkg_std_app_loads_math`;
`std/fs.ryx`, `testdata/pkg_std_fs`, `build_fs_via_std_import`;
`std/crypto.ryx`, `testdata/pkg_std_crypto`, `build_crypto_sha_via_std`.

### 6.6 Workspace monorepo

A workspace root may declare sibling packages:

```toml
[workspace]
members = ["app", "lib"]
```

Members reference each other by `[package].name`:

```toml
[dependencies]
util = { workspace = true }
```

Resolution is local only (no CDN). `rynix.lock.toml` lives beside the workspace
root manifest and applies to all members. Path and registry deps still work.

Evidence: `testdata/ws_monorepo`, `build_ws_monorepo_app`, `deps_resolves_workspace_member`.


