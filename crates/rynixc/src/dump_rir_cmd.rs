//! The `rynixc dump-rir` subcommand: lower to RIR and print textual form.

use std::process::ExitCode;

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_rir::{
    analyze_escape, inject_regions, lower_module, print_module, run_pipeline,
};
use rynix_sema::analyze;
use rynix_span::{Interner, SourceMap};

use crate::cli::DumpRirOptions;
use crate::driver;

pub fn run(options: &DumpRirOptions) -> ExitCode {
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
    let analysis = analyze(module, &mut interner, &mut sink);

    if sink.error_count() > 0 {
        return driver::emit_diagnostics(&sink, &sources, options.error_format);
    }

    let mut rir = lower_module(
        module,
        &analysis,
        &mut interner,
        file.text(),
        file.start_pos(),
    );
    if options.optimize {
        let errs = run_pipeline(&mut rir);
        for e in &errs {
            eprintln!("rir verifier: {e}");
        }
        if !errs.is_empty() {
            return ExitCode::from(1);
        }
    }
    if options.escape {
        let report = analyze_escape(&rir, &interner);
        inject_regions(&mut rir, &report);
    }

    print!("{}", print_module(&rir, &interner));
    ExitCode::SUCCESS
}
