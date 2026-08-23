//! The `rynixc check` subcommand: lex + parse + sema (+ optional escape explain).

use std::process::ExitCode;

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_rir::{
    analyze_escape, explain_alloc_human, explain_alloc_json, lower_module,
};
use rynix_sema::analyze_with_source;
use rynix_span::{Interner, SourceMap};

use crate::cli::{CheckOptions, ErrorFormat};
use crate::driver;

pub fn run(options: &CheckOptions) -> ExitCode {
    let mut sources = SourceMap::new();
    let file_id = match sources.load_file(&options.path) {
        Ok(id) => id,
        Err(error) => {
            eprintln!("error: cannot read {}: {error}", options.path.display());
            return ExitCode::from(3);
        }
    };

    let file = sources.file(file_id);
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = rynix_parser::parse(
        &arena,
        &mut interner,
        file.text(),
        file.start_pos(),
        &mut sink,
    );

    // Always run sema on the (possibly recovered) tree so AI agents see the
    // full diagnostic set for a single `check` invocation.
    let analysis = analyze_with_source(
        module,
        &mut interner,
        &mut sink,
        Some(file.text()),
        file.start_pos(),
    );

    let code = driver::emit_diagnostics(&sink, &sources, options.error_format);
    if code != ExitCode::SUCCESS || !options.explain_alloc {
        return code;
    }

    let rir = lower_module(
        module,
        &analysis,
        &mut interner,
        file.text(),
        file.start_pos(),
    );
    let report = analyze_escape(&rir, &interner);
    match options.error_format {
        ErrorFormat::Human => print!("{}", explain_alloc_human(&rir, &report, &interner)),
        ErrorFormat::Json => print!("{}", explain_alloc_json(&rir, &report, &interner)),
    }
    ExitCode::SUCCESS
}
