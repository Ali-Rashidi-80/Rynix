//! Argument parsing.
//!
//! Hand-written on purpose: the driver has one canonical invocation form per
//! task, so a parser generator would add a dependency and a second way to
//! express everything.

use std::path::PathBuf;

pub const USAGE: &str = "\
Usage: rynixc <command> [options]

Commands:
  lex <file.ryx>      Tokenize a source file
  parse <file.ryx>    Parse a source file into an arena AST
  check <file.ryx>    Lex + parse + sema; report diagnostics
  dump-rir <file.ryx> Lower to RIR and print textual form
  emit-ll <file.ryx>  Emit textual LLVM IR (.ll)
  emit-wasm <file.ryx> Emit a real .wasm via clang (wasm32; no WASI/rt)
  build [path]        Emit .ll and link with clang + rt/portable.c
  run [path]          Build then execute (path: .ryx, dir, rynix.toml, or cwd)
  test [paths...]     Run #^ directive tests (default: testdata/)
  fmt <file.ryx>      Canonical-format a source file
  graph <file.ryx>    Emit rynix.graph.v1 JSON (functions + call edges)
  slice <file.ryx>    Compact interface view (human or JSON)
  impact <file.ryx>   Blast-radius: callers/callees (rynix.impact.v1)
  eval <expr>         Micro-evaluator via RIR interpreter
  patch <file.ryx>    Apply best compiler-suggested fix
  verify              Evidence-gate a contract TOML (ADR-0009)
  precheck <file.ryx> Blast-radius + write gate (rynix.precheck.v1)
  context <file.ryx>  Slice packed to a char budget (rynix.context.v1)
  security <file.ryx> Pattern CWE-798-class scan (rynix.security.v1)
  scope               Show agent permissions (rynix.scope.v1)
  deps [path]         Resolve local path deps from rynix.toml (rynix.deps.v1)
  dna [path]          Heuristic project conventions (rynix.dna.v1)
  new <name>          Scaffold local package (rynix.toml + src/main.ryx)
  mcp-serve           JSON-RPC 2.0 MCP server on stdio
  lsp-serve           Language Server Protocol on stdio
  arch check          Validate Architecture.toml layer rules

Options for `arch`:
  --config <path>     Path to Architecture.toml (default: ./Architecture.toml)
  --root <path>       Project root to scan (default: .)

Options for `verify`:
  --contract <path>   Contract TOML (required)
  --root <path>       Project root (default: .)
  --run               Actually run cargo_test evidence (slow)
  --error-format=FMT  human (default) or json (rynix.verify.v1)

Options for `precheck`:
  --fn NAME           Limit impact to one function
  --allow-write       Set write_allowed=true in the report
  --error-format=FMT  human or json

Options for `context`:
  --budget=N          Max characters in packed outline (default: 2000)
  --error-format=FMT  human or json

Options for `security`:
  --error-format=FMT  human or json (rynix.security.v1)

Options for `scope`:
  --config <path>     Path to rynix.scope.toml (optional)
  --error-format=FMT  human or json

Options for `deps`:
  --lock              Write rynix.lock.toml beside the manifest
  --locked            Require rynix.lock.toml and verify sha256 pins
  --error-format=FMT  human or json (rynix.deps.v1)

Options for `dna`:
  --prompt            Emit a short agent-facing conventions blurb
  --error-format=FMT  human or json (rynix.dna.v1)

Options for `new`:
  --path DIR          Parent directory (default: .)

Options for `patch`:
  --write             Write fix (requires scope patch_write or --force-write)
  --force-write       Override deny-by-default scope for this invocation
  --scope <path>      Scope config for write gate

Options for `lex`:
  --dump-tokens       Print one line per token: span, kind, text

Options for `parse`:
  --dump-ast          Print the AST as an s-expression

Options for `check`:
  --explain-alloc     After a clean check, print allocation placement
                      (stack|region|heap) for every RIR alloc site

Options for `dump-rir`:
  --opt               Run DCE / const-fold / simplify-cfg before dump
  --escape            Run escape analysis and inject region/free markers

Options for `emit-ll` / `emit-wasm` / `build` / `run`:
  -o <path>           Output path (emit-wasm defaults to <stem>.wasm)
  --opt / --no-opt    RIR optimize pipeline (emit-ll/emit-wasm defaults off; build
                      defaults on unless [build].optimize / these flags say otherwise)
  --target=TRIPLE     (emit-ll) v1: `wasm32-unknown-unknown` only (Phase 13);
                      emit-wasm always uses wasm32-unknown-unknown
  --keep-ll           (build) Keep the intermediate .ll next to the binary
  --runtime=KIND      `portable`, `uring` (Linux), or `iocp` (Windows);
                      if omitted, use [build].runtime then portable
  --bench             (build) Define RYNIX_BENCH — print_i64 becomes a sink (Suite5 timing)
  --pgo-gen           (build) Clang `-fprofile-instr-generate` (training build)
  --pgo-use=PATH      (build) Clang `-fprofile-use=PATH` (optimized build)

Options for `fmt`:
  --write             Write result back to the file
  --check             Exit 1 if the file is not already formatted

Options for `graph` / `slice` / `impact`:
  --error-format=FMT  Same as shared options (JSON schemas: graph/slice/impact v1)

Options for `impact`:
  --fn NAME           Limit to one function's blast radius

Options for `eval`:
  --json              Emit rynix.eval.v1 JSON

Options for `patch`:
  --write             Write fixed source back to the file

Shared options:
  --error-format=FMT  Diagnostic rendering: `human` (default) or `json`
                      (`json` emits one rynix.diag.v1 object per line;
                       schema: docs/schemas/rynix.diag.v1.json)

Global options:
  -h, --help          Print this help
  -V, --version       Print the compiler version

Exit codes: 0 success, 1 diagnostics/build failure, 2 bad invocation, 3 I/O error
";

/// How diagnostics are rendered.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ErrorFormat {
    Human,
    Json,
}

#[derive(Debug)]
pub struct LexOptions {
    pub path: PathBuf,
    pub dump_tokens: bool,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub struct ParseOptions {
    pub path: PathBuf,
    pub dump_ast: bool,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub struct CheckOptions {
    pub path: PathBuf,
    pub error_format: ErrorFormat,
    pub explain_alloc: bool,
}

#[derive(Debug)]
pub struct DumpRirOptions {
    pub path: PathBuf,
    pub optimize: bool,
    pub escape: bool,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub struct EmitLlOptions {
    pub path: PathBuf,
    pub output: Option<PathBuf>,
    pub optimize: bool,
    /// Phase 13: `Some("wasm32-unknown-unknown")` when `--target=` set.
    pub target: Option<String>,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub struct EmitWasmOptions {
    pub path: PathBuf,
    pub output: Option<PathBuf>,
    pub optimize: bool,
    pub error_format: ErrorFormat,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RuntimeKind {
    Portable,
    Uring,
    Iocp,
}

#[derive(Debug, Clone)]
pub enum PgoMode {
    None,
    Generate,
    Use(PathBuf),
}

#[derive(Debug)]
pub struct BuildOptions {
    /// Source `.ryx`, package directory, `rynix.toml`, or `None` for cwd.
    pub path: Option<PathBuf>,
    pub output: Option<PathBuf>,
    pub keep_ll: bool,
    /// `Some` only when `--runtime=` was present on the CLI (L4).
    pub runtime: Option<RuntimeKind>,
    /// `Some` only when `--opt` / `--no-opt` was present (P13-L5).
    pub optimize: Option<bool>,
    pub bench: bool,
    pub pgo: PgoMode,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub struct RunOptions {
    /// Source `.ryx`, package directory, `rynix.toml`, or `None` for cwd.
    pub path: Option<PathBuf>,
    pub output: Option<PathBuf>,
    /// `Some` only when `--runtime=` was present on the CLI (L4).
    pub runtime: Option<RuntimeKind>,
    /// `Some` only when `--opt` / `--no-opt` was present (P13-L5).
    pub optimize: Option<bool>,
    pub bench: bool,
    pub pgo: PgoMode,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub struct TestOptions {
    pub paths: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct FmtOptions {
    pub path: PathBuf,
    pub write: bool,
    pub check: bool,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub struct AgentOptions {
    pub path: PathBuf,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub struct ImpactOptions {
    pub path: PathBuf,
    pub function: Option<String>,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub struct EvalOptions {
    pub expr: String,
    pub json: bool,
}

#[derive(Debug)]
pub struct PatchOptions {
    pub path: PathBuf,
    pub write: bool,
    pub force_write: bool,
    pub scope: Option<PathBuf>,
}

#[derive(Debug)]
pub struct SecurityOptions {
    pub path: PathBuf,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub struct ScopeOptions {
    pub config: Option<PathBuf>,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub struct DepsOptions {
    pub path: Option<PathBuf>,
    /// Write `rynix.lock.toml` after a successful resolve.
    pub lock: bool,
    /// Fail unless `rynix.lock.toml` exists and matches resolve.
    pub locked: bool,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub struct DnaOptions {
    pub path: Option<PathBuf>,
    pub prompt: bool,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub struct NewOptions {
    pub name: String,
    pub path: Option<PathBuf>,
}

#[derive(Debug)]
pub struct ArchCheckOptions {
    pub config: Option<PathBuf>,
    pub root: Option<PathBuf>,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub struct VerifyOptions {
    pub contract: PathBuf,
    pub root: Option<PathBuf>,
    pub run_tests: bool,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub struct PrecheckOptions {
    pub path: PathBuf,
    pub function: Option<String>,
    pub allow_write: bool,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub struct ContextOptions {
    pub path: PathBuf,
    pub budget: usize,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub enum Command {
    Help,
    Version,
    Lex(LexOptions),
    Parse(ParseOptions),
    Check(CheckOptions),
    DumpRir(DumpRirOptions),
    EmitLl(EmitLlOptions),
    EmitWasm(EmitWasmOptions),
    Build(BuildOptions),
    Run(RunOptions),
    Test(TestOptions),
    Fmt(FmtOptions),
    Graph(AgentOptions),
    Slice(AgentOptions),
    Impact(ImpactOptions),
    Eval(EvalOptions),
    Patch(PatchOptions),
    Verify(VerifyOptions),
    Precheck(PrecheckOptions),
    Context(ContextOptions),
    Security(SecurityOptions),
    Scope(ScopeOptions),
    Deps(DepsOptions),
    Dna(DnaOptions),
    New(NewOptions),
    McpServe,
    LspServe,
    ArchCheck(ArchCheckOptions),
}

/// Parses command-line arguments (without the program name).
pub fn parse(args: &[String]) -> Result<Command, String> {
    let Some(first) = args.first() else {
        return Ok(Command::Help);
    };
    match first.as_str() {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "-V" | "--version" | "version" => Ok(Command::Version),
        "lex" => parse_lex(&args[1..]),
        "parse" => parse_parse(&args[1..]),
        "check" => parse_check(&args[1..]),
        "dump-rir" => parse_dump_rir(&args[1..]),
        "emit-ll" => parse_emit_ll(&args[1..]),
        "emit-wasm" => parse_emit_wasm(&args[1..]),
        "build" => parse_build(&args[1..]),
        "run" => parse_run(&args[1..]),
        "test" => parse_test(&args[1..]),
        "fmt" => parse_fmt(&args[1..]),
        "graph" => parse_agent(&args[1..], AgentCmd::Graph),
        "slice" => parse_agent(&args[1..], AgentCmd::Slice),
        "impact" => parse_impact(&args[1..]),
        "eval" => parse_eval(&args[1..]),
        "patch" => parse_patch(&args[1..]),
        "verify" => parse_verify(&args[1..]),
        "precheck" => parse_precheck(&args[1..]),
        "context" => parse_context(&args[1..]),
        "security" => parse_security(&args[1..]),
        "scope" => parse_scope(&args[1..]),
        "deps" => parse_deps(&args[1..]),
        "dna" => parse_dna(&args[1..]),
        "new" => parse_new(&args[1..]),
        "mcp-serve" => Ok(Command::McpServe),
        "lsp-serve" => Ok(Command::LspServe),
        "arch" => parse_arch(&args[1..]),
        other => Err(format!("unknown command `{other}`")),
    }
}

fn parse_error_format(arg: &str) -> Result<ErrorFormat, String> {
    match arg {
        "--error-format=human" => Ok(ErrorFormat::Human),
        "--error-format=json" => Ok(ErrorFormat::Json),
        other if other.starts_with("--error-format") => Err(format!(
            "invalid `{other}`: expected --error-format=human or --error-format=json"
        )),
        _ => unreachable!(),
    }
}

fn parse_lex(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut dump_tokens = false;
    let mut error_format = ErrorFormat::Human;

    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--dump-tokens" => dump_tokens = true,
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }

    let path = path.ok_or_else(|| "missing input file".to_string())?;
    Ok(Command::Lex(LexOptions {
        path,
        dump_tokens,
        error_format,
    }))
}

fn parse_parse(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut dump_ast = false;
    let mut error_format = ErrorFormat::Human;

    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--dump-ast" => dump_ast = true,
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }

    let path = path.ok_or_else(|| "missing input file".to_string())?;
    Ok(Command::Parse(ParseOptions {
        path,
        dump_ast,
        error_format,
    }))
}

fn parse_check(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut error_format = ErrorFormat::Human;
    let mut explain_alloc = false;

    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--explain-alloc" => explain_alloc = true,
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }

    let path = path.ok_or_else(|| "missing input file".to_string())?;
    Ok(Command::Check(CheckOptions {
        path,
        error_format,
        explain_alloc,
    }))
}

fn parse_dump_rir(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut optimize = false;
    let mut escape = false;
    let mut error_format = ErrorFormat::Human;

    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--opt" => optimize = true,
            "--escape" => escape = true,
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }

    let path = path.ok_or_else(|| "missing input file".to_string())?;
    Ok(Command::DumpRir(DumpRirOptions {
        path,
        optimize,
        escape,
        error_format,
    }))
}

fn parse_emit_ll(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut output = None;
    let mut optimize = false;
    let mut target = None;
    let mut error_format = ErrorFormat::Human;
    let mut expect_o = false;

    for arg in args {
        if expect_o {
            output = Some(PathBuf::from(arg));
            expect_o = false;
            continue;
        }
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--opt" => optimize = true,
            "-o" => expect_o = true,
            other if other.starts_with("-o=") => {
                output = Some(PathBuf::from(&other[3..]));
            }
            other if other.starts_with("--target=") => {
                let t = &other[9..];
                if t != "wasm32-unknown-unknown" {
                    return Err(format!(
                        "unsupported `--target={t}` (v1 allows only wasm32-unknown-unknown)"
                    ));
                }
                target = Some(t.to_string());
            }
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }
    if expect_o {
        return Err("missing path after -o".into());
    }

    let path = path.ok_or_else(|| "missing input file".to_string())?;
    Ok(Command::EmitLl(EmitLlOptions {
        path,
        output,
        optimize,
        target,
        error_format,
    }))
}

fn parse_emit_wasm(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut output = None;
    let mut optimize = false;
    let mut error_format = ErrorFormat::Human;
    let mut expect_o = false;

    for arg in args {
        if expect_o {
            output = Some(PathBuf::from(arg));
            expect_o = false;
            continue;
        }
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--opt" => optimize = true,
            "-o" => expect_o = true,
            other if other.starts_with("-o=") => {
                output = Some(PathBuf::from(&other[3..]));
            }
            other if other.starts_with("--target=") => {
                let t = &other[9..];
                if t != "wasm32-unknown-unknown" {
                    return Err(format!(
                        "unsupported `--target={t}` (emit-wasm is always wasm32-unknown-unknown)"
                    ));
                }
                // Accepted for symmetry with emit-ll; always wasm32.
            }
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }
    if expect_o {
        return Err("missing path after -o".into());
    }

    let path = path.ok_or_else(|| "missing input file".to_string())?;
    Ok(Command::EmitWasm(EmitWasmOptions {
        path,
        output,
        optimize,
        error_format,
    }))
}

fn parse_build(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut output = None;
    let mut keep_ll = false;
    let mut runtime = None;
    let mut optimize = None;
    let mut bench = false;
    let mut pgo = PgoMode::None;
    let mut error_format = ErrorFormat::Human;
    let mut expect_o = false;

    for arg in args {
        if expect_o {
            output = Some(PathBuf::from(arg));
            expect_o = false;
            continue;
        }
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--keep-ll" => keep_ll = true,
            "--bench" => bench = true,
            "--opt" => optimize = Some(true),
            "--no-opt" => optimize = Some(false),
            "--pgo-gen" => pgo = PgoMode::Generate,
            "--runtime=portable" => runtime = Some(RuntimeKind::Portable),
            "--runtime=uring" => runtime = Some(RuntimeKind::Uring),
            "--runtime=iocp" => runtime = Some(RuntimeKind::Iocp),
            other if other.starts_with("--pgo-use=") => {
                pgo = PgoMode::Use(PathBuf::from(&other[10..]));
            }
            other if other.starts_with("--runtime") => {
                return Err(
                    "invalid `--runtime`: expected --runtime=portable, --runtime=uring, or --runtime=iocp".into(),
                );
            }
            "-o" => expect_o = true,
            other if other.starts_with("-o=") => {
                output = Some(PathBuf::from(&other[3..]));
            }
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }
    if expect_o {
        return Err("missing path after -o".into());
    }

    Ok(Command::Build(BuildOptions {
        path,
        output,
        keep_ll,
        runtime,
        optimize,
        bench,
        pgo,
        error_format,
    }))
}

fn parse_run(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut output = None;
    let mut runtime = None;
    let mut optimize = None;
    let mut bench = false;
    let mut pgo = PgoMode::None;
    let mut error_format = ErrorFormat::Human;
    let mut expect_o = false;

    for arg in args {
        if expect_o {
            output = Some(PathBuf::from(arg));
            expect_o = false;
            continue;
        }
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--bench" => bench = true,
            "--opt" => optimize = Some(true),
            "--no-opt" => optimize = Some(false),
            "--pgo-gen" => pgo = PgoMode::Generate,
            "--runtime=portable" => runtime = Some(RuntimeKind::Portable),
            "--runtime=uring" => runtime = Some(RuntimeKind::Uring),
            "--runtime=iocp" => runtime = Some(RuntimeKind::Iocp),
            other if other.starts_with("--pgo-use=") => {
                pgo = PgoMode::Use(PathBuf::from(&other[10..]));
            }
            other if other.starts_with("--runtime") => {
                return Err(
                    "invalid `--runtime`: expected --runtime=portable, --runtime=uring, or --runtime=iocp".into(),
                );
            }
            "-o" => expect_o = true,
            other if other.starts_with("-o=") => {
                output = Some(PathBuf::from(&other[3..]));
            }
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }
    if expect_o {
        return Err("missing path after -o".into());
    }

    Ok(Command::Run(RunOptions {
        path,
        output,
        runtime,
        optimize,
        bench,
        pgo,
        error_format,
    }))
}

fn parse_test(args: &[String]) -> Result<Command, String> {
    let mut paths = Vec::new();
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other => paths.push(PathBuf::from(other)),
        }
    }
    Ok(Command::Test(TestOptions { paths }))
}

fn parse_fmt(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut write = false;
    let mut check = false;
    let mut error_format = ErrorFormat::Human;

    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--write" => write = true,
            "--check" => check = true,
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }
    let path = path.ok_or_else(|| "missing input file".to_string())?;
    Ok(Command::Fmt(FmtOptions {
        path,
        write,
        check,
        error_format,
    }))
}

#[derive(Clone, Copy)]
enum AgentCmd {
    Graph,
    Slice,
}

fn parse_agent(args: &[String], cmd: AgentCmd) -> Result<Command, String> {
    let mut path = None;
    let mut error_format = ErrorFormat::Human;
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }
    let path = path.ok_or_else(|| "missing input file".to_string())?;
    let opts = AgentOptions {
        path,
        error_format,
    };
    Ok(match cmd {
        AgentCmd::Graph => Command::Graph(opts),
        AgentCmd::Slice => Command::Slice(opts),
    })
}

fn parse_impact(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut function = None;
    let mut error_format = ErrorFormat::Human;
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            other if other.starts_with("--fn=") => {
                function = Some(other.trim_start_matches("--fn=").to_string());
            }
            "--fn" => {
                return Err("--fn requires a name (use --fn=name)".into());
            }
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }
    let path = path.ok_or_else(|| "missing input file".to_string())?;
    Ok(Command::Impact(ImpactOptions {
        path,
        function,
        error_format,
    }))
}

fn parse_eval(args: &[String]) -> Result<Command, String> {
    let mut json = false;
    let mut parts = Vec::new();
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--json" => json = true,
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other => parts.push(other.to_string()),
        }
    }
    if parts.is_empty() {
        return Err("missing expression".into());
    }
    Ok(Command::Eval(EvalOptions {
        expr: parts.join(" "),
        json,
    }))
}

fn parse_patch(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut write = false;
    let mut force_write = false;
    let mut scope = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--write" => write = true,
            "--force-write" => force_write = true,
            "--scope" => {
                i += 1;
                let Some(val) = args.get(i) else {
                    return Err("missing value for --scope".into());
                };
                scope = Some(PathBuf::from(val));
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
        i += 1;
    }
    let path = path.ok_or_else(|| "missing input file".to_string())?;
    Ok(Command::Patch(PatchOptions {
        path,
        write,
        force_write,
        scope,
    }))
}

fn parse_security(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut error_format = ErrorFormat::Human;
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }
    let path = path.ok_or_else(|| "missing input file".to_string())?;
    Ok(Command::Security(SecurityOptions {
        path,
        error_format,
    }))
}

fn parse_scope(args: &[String]) -> Result<Command, String> {
    let mut config = None;
    let mut error_format = ErrorFormat::Human;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            "--config" => {
                i += 1;
                let Some(val) = args.get(i) else {
                    return Err("missing value for --config".into());
                };
                config = Some(PathBuf::from(val));
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other => return Err(format!("unexpected argument `{other}`")),
        }
        i += 1;
    }
    Ok(Command::Scope(ScopeOptions {
        config,
        error_format,
    }))
}

fn parse_deps(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut lock = false;
    let mut locked = false;
    let mut error_format = ErrorFormat::Human;
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--lock" => lock = true,
            "--locked" => locked = true,
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }
    Ok(Command::Deps(DepsOptions {
        path,
        lock,
        locked,
        error_format,
    }))
}

fn parse_dna(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut prompt = false;
    let mut error_format = ErrorFormat::Human;
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--prompt" => prompt = true,
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }
    Ok(Command::Dna(DnaOptions {
        path,
        prompt,
        error_format,
    }))
}

fn parse_new(args: &[String]) -> Result<Command, String> {
    let mut name = None;
    let mut path = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--path" => {
                i += 1;
                let Some(p) = args.get(i) else {
                    return Err("--path requires a directory".into());
                };
                path = Some(PathBuf::from(p));
            }
            other if other.starts_with("--path=") => {
                path = Some(PathBuf::from(other.trim_start_matches("--path=")));
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if name.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => name = Some(other.to_string()),
        }
        i += 1;
    }
    let Some(name) = name else {
        return Err("`new` requires a package name".into());
    };
    Ok(Command::New(NewOptions { name, path }))
}

fn parse_verify(args: &[String]) -> Result<Command, String> {
    let mut contract = None;
    let mut root = None;
    let mut run_tests = false;
    let mut error_format = ErrorFormat::Human;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--run" => run_tests = true,
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            "--contract" => {
                i += 1;
                let Some(val) = args.get(i) else {
                    return Err("missing value for --contract".into());
                };
                contract = Some(PathBuf::from(val));
            }
            other if other.starts_with("--contract=") => {
                contract = Some(PathBuf::from(
                    other.trim_start_matches("--contract="),
                ));
            }
            "--root" => {
                i += 1;
                let Some(val) = args.get(i) else {
                    return Err("missing value for --root".into());
                };
                root = Some(PathBuf::from(val));
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other => return Err(format!("unexpected argument `{other}`")),
        }
        i += 1;
    }
    let contract = contract.ok_or_else(|| "missing --contract <path>".to_string())?;
    Ok(Command::Verify(VerifyOptions {
        contract,
        root,
        run_tests,
        error_format,
    }))
}

fn parse_precheck(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut function = None;
    let mut allow_write = false;
    let mut error_format = ErrorFormat::Human;
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--allow-write" => allow_write = true,
            other if other.starts_with("--fn=") => {
                function = Some(other.trim_start_matches("--fn=").to_string());
            }
            "--fn" => {
                return Err("use --fn=NAME (equals form)".into());
            }
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }
    let path = path.ok_or_else(|| "missing input file".to_string())?;
    Ok(Command::Precheck(PrecheckOptions {
        path,
        function,
        allow_write,
        error_format,
    }))
}

fn parse_context(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut budget = 2000usize;
    let mut error_format = ErrorFormat::Human;
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            other if other.starts_with("--budget=") => {
                let raw = other.trim_start_matches("--budget=");
                budget = raw
                    .parse()
                    .map_err(|_| format!("invalid --budget={raw}"))?;
                if budget == 0 {
                    return Err("--budget must be >= 1".into());
                }
            }
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }
    let path = path.ok_or_else(|| "missing input file".to_string())?;
    Ok(Command::Context(ContextOptions {
        path,
        budget,
        error_format,
    }))
}

fn parse_arch(args: &[String]) -> Result<Command, String> {
    let Some(sub) = args.first() else {
        return Err("arch requires a subcommand (check)".into());
    };
    if sub != "check" {
        return Err(format!("unknown arch subcommand `{sub}` (expected check)"));
    }
    let mut config = None;
    let mut root = None;
    let mut error_format = ErrorFormat::Human;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            "--config" => {
                i += 1;
                let Some(val) = args.get(i) else {
                    return Err("missing value for --config".into());
                };
                config = Some(PathBuf::from(val));
            }
            "--root" => {
                i += 1;
                let Some(val) = args.get(i) else {
                    return Err("missing value for --root".into());
                };
                root = Some(PathBuf::from(val));
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other => return Err(format!("unexpected argument `{other}`")),
        }
        i += 1;
    }
    Ok(Command::ArchCheck(ArchCheckOptions {
        config,
        root,
        error_format,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn no_arguments_prints_help() {
        assert!(matches!(parse(&[]).unwrap(), Command::Help));
        assert!(matches!(parse(&args(&["--help"])).unwrap(), Command::Help));
        assert!(matches!(parse(&args(&["-V"])).unwrap(), Command::Version));
    }

    #[test]
    fn lex_defaults_to_human_errors_without_token_dump() {
        let Command::Lex(options) = parse(&args(&["lex", "a.ryx"])).unwrap() else {
            panic!("expected lex");
        };
        assert_eq!(options.path.to_str(), Some("a.ryx"));
        assert!(!options.dump_tokens);
        assert_eq!(options.error_format, ErrorFormat::Human);
    }

    #[test]
    fn lex_accepts_all_flags_in_any_order() {
        let Command::Lex(options) = parse(&args(&[
            "lex",
            "--error-format=json",
            "b.ryx",
            "--dump-tokens",
        ]))
        .unwrap() else {
            panic!("expected lex");
        };
        assert_eq!(options.path.to_str(), Some("b.ryx"));
        assert!(options.dump_tokens);
        assert_eq!(options.error_format, ErrorFormat::Json);
    }

    #[test]
    fn parse_accepts_dump_ast() {
        let Command::Parse(options) = parse(&args(&[
            "parse",
            "a.ryx",
            "--dump-ast",
            "--error-format=json",
        ]))
        .unwrap() else {
            panic!("expected parse");
        };
        assert!(options.dump_ast);
        assert_eq!(options.error_format, ErrorFormat::Json);
    }

    #[test]
    fn dump_rir_prints_func() {
        let Command::DumpRir(options) = parse(&args(&[
            "dump-rir",
            "a.ryx",
            "--opt",
            "--escape",
            "--error-format=json",
        ]))
        .unwrap() else {
            panic!("expected dump-rir");
        };
        assert_eq!(options.path.to_str(), Some("a.ryx"));
        assert!(options.optimize);
        assert!(options.escape);
        assert_eq!(options.error_format, ErrorFormat::Json);
    }

    #[test]
    fn check_accepts_json_and_explain() {
        let Command::Check(options) = parse(&args(&[
            "check",
            "a.ryx",
            "--explain-alloc",
            "--error-format=json",
        ]))
        .unwrap() else {
            panic!("expected check");
        };
        assert_eq!(options.path.to_str(), Some("a.ryx"));
        assert!(options.explain_alloc);
        assert_eq!(options.error_format, ErrorFormat::Json);
    }

    #[test]
    fn build_allows_omitted_path_and_tracks_runtime_flag() {
        let Command::Build(options) = parse(&args(&["build"])).unwrap() else {
            panic!("expected build");
        };
        assert!(options.path.is_none());
        assert!(options.runtime.is_none());

        let Command::Build(options) =
            parse(&args(&["build", "pkg/", "--runtime=iocp"])).unwrap()
        else {
            panic!("expected build");
        };
        assert_eq!(options.path.as_deref().and_then(|p| p.to_str()), Some("pkg/"));
        assert_eq!(options.runtime, Some(RuntimeKind::Iocp));
    }

    #[test]
    fn run_allows_omitted_path() {
        let Command::Run(options) = parse(&args(&["run", "--runtime=portable"])).unwrap() else {
            panic!("expected run");
        };
        assert!(options.path.is_none());
        assert_eq!(options.runtime, Some(RuntimeKind::Portable));
    }

    #[test]
    fn invocation_errors_are_specific() {
        assert!(
            parse(&args(&["lex"]))
                .unwrap_err()
                .contains("missing input")
        );
        assert!(
            parse(&args(&["nope"]))
                .unwrap_err()
                .contains("unknown command")
        );
        assert!(
            parse(&args(&["lex", "a", "--nope"]))
                .unwrap_err()
                .contains("unknown option")
        );
        assert!(
            parse(&args(&["lex", "a", "b"]))
                .unwrap_err()
                .contains("extra argument")
        );
        assert!(
            parse(&args(&["lex", "a", "--error-format=xml"]))
                .unwrap_err()
                .contains("expected --error-format")
        );
    }
}
