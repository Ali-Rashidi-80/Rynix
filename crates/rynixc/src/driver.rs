//! Shared helpers for rynixc subcommands.

use std::io::{BufWriter, Write};
use std::process::ExitCode;

use rynix_diag::{VecSink, render_human, render_json};
use rynix_span::SourceMap;

use crate::cli::ErrorFormat;

pub fn emit_diagnostics(
    sink: &VecSink,
    sources: &SourceMap,
    error_format: ErrorFormat,
) -> ExitCode {
    let mut stderr = BufWriter::new(std::io::stderr().lock());
    for diag in &sink.diags {
        let rendered = match error_format {
            ErrorFormat::Human => render_human(diag, sources),
            ErrorFormat::Json => render_json(diag, sources),
        };
        let _ = writeln!(stderr, "{rendered}");
    }
    let errors = sink.error_count();
    if errors > 0 && error_format == ErrorFormat::Human {
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
