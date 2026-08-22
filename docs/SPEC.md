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
( ) [ ] { }            grouping, indexing, struct literals
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
let_stmt    = "let" [ "mut" ] Ident [ ":" type ] "=" expr Newline
return_stmt = "return" [ expr ] Newline
break_stmt  = "break" Newline
continue_stmt = "continue" Newline
loop_stmt   = "loop" Newline block "end"
for_stmt    = "for" Ident "in" expr Newline block "end"
if_stmt     = "if" expr Newline block { "elif" expr Newline block }
              [ "else" Newline block ] "end"
match_stmt  = "match" expr Newline { match_arm } [ "else" Newline block ] "end"
match_arm   = match_pat Newline block
match_pat   = IntLit | "true" | "false" | "_"
expr_stmt   = expr Newline

type        = path [ "[" type { "," type } "]" ] | "[" type "]"
path        = Ident { "::" Ident }
```

Reference types (`&T`) are not part of the shipping type grammar; ownership
is inferred via escape analysis, not surface `&` syntax.

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

Assignment (`=`, `+=`, `-=`, `*=`, `/=`, `%=`) is a statement, not an
expression — canonical, and eliminates a whole class of bugs and model
confusion.

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
| `http_get_json_i64(host, port, path, field)` | HTTP GET + JSON field |
| `tensor`, `signal`, `agent` | smart primitives (stubs / hooks) |

Notes in `std/*.ryx` are documentation only until a module loader ships.
