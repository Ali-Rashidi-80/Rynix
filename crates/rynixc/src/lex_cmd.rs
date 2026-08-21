//! The `rynixc lex` subcommand.

use std::io::{BufWriter, Write};
use std::process::ExitCode;

use rynix_diag::{VecSink, render_human, render_json};
use rynix_lexer::Lexer;
use rynix_span::SourceMap;

use crate::cli::{ErrorFormat, LexOptions};

pub fn run(options: &LexOptions) -> ExitCode {
    let mut sources = SourceMap::new();
    let file_id = match sources.load_file(&options.path) {
        Ok(id) => id,
        Err(error) => {
            eprintln!("error: cannot read {}: {error}", options.path.display());
            return ExitCode::from(3);
        }
    };

    let mut sink = VecSink::new();
    // Buffered: token dumps are large, and one write syscall per token would
    // dominate the measurement.
    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    {
        let file = sources.file(file_id);
        let mut lexer = Lexer::from_file(file);
        loop {
            let token = lexer.next_token(&mut sink);
            if options.dump_tokens {
                let _ = writeln!(
                    out,
                    "{:>7}..{:<7} {:<12?} {:?}",
                    token.span.lo(),
                    token.span.hi(),
                    token.kind,
                    &file.text()[(token.span.lo() - file.start_pos()) as usize
                        ..(token.span.hi() - file.start_pos()) as usize]
                );
            }
            if token.is_eof() {
                break;
            }
        }
    }
    let _ = out.flush();

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
