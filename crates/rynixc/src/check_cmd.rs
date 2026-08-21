//! The `rynixc check` subcommand: lex + parse + sema, report diagnostics.

use std::process::ExitCode;

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_sema::analyze;
use rynix_span::{Interner, SourceMap};

use crate::cli::CheckOptions;
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
    let _analysis = analyze(module, &mut interner, &mut sink);

    driver::emit_diagnostics(&sink, &sources, options.error_format)
}
