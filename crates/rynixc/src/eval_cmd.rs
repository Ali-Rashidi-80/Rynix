//! `rynixc eval` — micro-evaluator via RIR interpreter (oracle-backed).

use std::process::ExitCode;

use crate::agent_lib::eval_snippet;
use crate::cli::EvalOptions;

pub fn run(options: &EvalOptions) -> ExitCode {
    match eval_snippet(&options.expr) {
        Ok(v) => {
            if options.json {
                println!("{}", serde_json::json!({ "schema": "rynix.eval.v1", "result": v }));
            } else {
                match v.get("value") {
                    Some(n) => println!("{n}"),
                    None => println!("{v}"),
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("eval error: {e}");
            ExitCode::from(1)
        }
    }
}
