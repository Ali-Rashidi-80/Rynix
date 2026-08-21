//! The `rynixc parse` subcommand.

use std::io::{BufWriter, Write};
use std::process::ExitCode;

use rynix_ast::{AstArena, dump_module};
use rynix_diag::{VecSink, render_human, render_json};
use rynix_span::{Interner, SourceMap};

use crate::cli::{ErrorFormat, ParseOptions};

pub fn run(options: &ParseOptions) -> ExitCode {
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

    if options.dump_ast {
        let mut out = BufWriter::new(std::io::stdout().lock());
        let _ = writeln!(out, "{}", dump_module(module, &interner));
        let _ = out.flush();
    }

    let mut stderr = BufWriter::new(std::io::stderr().lock());
    for diag in &sink.diags {
        let rendered = match options.error_format {
            ErrorFormat::Human => render_human(diag, &sources),
            ErrorFormat::Json => render_json(diag, &sources),
        };
        let _ = writeln!(stderr, "{rendered}");
    }
    let errors = sink.error_count();
    if errors > 0 && options.error_format == ErrorFormat::Human {
        let _ = writeln!(
            stderr,
            "\n{errors} error{} reported",
            if errors == 1 { "" } else { "s" }
        );
    }
    let _ = stderr.flush();

    if errors > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
