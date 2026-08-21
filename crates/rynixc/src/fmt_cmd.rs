//! `rynixc fmt` — canonical zero-config formatter.

use std::process::ExitCode;

use rynix_ast::{format_module, AstArena};
use rynix_diag::VecSink;
use rynix_span::{Interner, SourceMap};

use crate::cli::FmtOptions;
use crate::driver;

pub fn run(options: &FmtOptions) -> ExitCode {
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

    if sink.error_count() > 0 {
        return driver::emit_diagnostics(&sink, &sources, options.error_format);
    }

    let formatted = format_module(module, &interner, file.text(), file.start_pos());

    if options.check {
        if formatted != file.text() {
            eprintln!("{}: needs formatting", options.path.display());
            return ExitCode::from(1);
        }
        return ExitCode::SUCCESS;
    }

    if options.write {
        if let Err(e) = std::fs::write(&options.path, &formatted) {
            eprintln!("error: cannot write {}: {e}", options.path.display());
            return ExitCode::from(3);
        }
    } else {
        print!("{formatted}");
    }
    ExitCode::SUCCESS
}
