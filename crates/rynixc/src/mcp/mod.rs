//! `rynixc mcp-serve` — JSON-RPC 2.0 over stdio (Content-Length framing).

mod discover;
mod tools;
mod transport;

use std::io;
use std::process::ExitCode;

use serde_json::{Value, json};

use crate::mcp::discover::server_discover;
use crate::mcp::tools::{tool_defs, tools_call};
use crate::mcp::transport::{read_message, rpc_error, write_result};

pub fn run() -> ExitCode {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut stdout = io::stdout();

    loop {
        let msg = match read_message(&mut reader) {
            Ok(Some(v)) => v,
            Ok(None) => return ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("mcp: read error: {e}");
                return ExitCode::from(1);
            }
        };

        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(Value::Null);

        if method == "notifications/initialized" || method.starts_with("notifications/") {
            continue;
        }

        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "rynixc", "version": env!("CARGO_PKG_VERSION") }
            })),
            "server/discover" => Ok(server_discover()),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tool_defs() })),
            "tools/call" => tools_call(&params),
            "shutdown" => {
                write_result(&mut stdout, id, Ok(Value::Null));
                return ExitCode::SUCCESS;
            }
            "" if msg.get("result").is_some() => continue,
            other => Err(rpc_error(-32601, format!("method not found: {other}"))),
        };

        if id.is_some() {
            write_result(&mut stdout, id, result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::tool_defs;
    use crate::mcp::discover::server_discover;

    #[test]
    fn mcp_tool_count_honest() {
        let tools = tool_defs();
        let arr = tools.as_array().expect("tool_defs array");
        assert_eq!(
            arr.len(),
            19,
            "MCP must stay at 19 real tools (no theater ≥20); got {}",
            arr.len()
        );
        let names: Vec<&str> = arr.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(
            names.contains(&"rynix_slice"),
            "missing rynix_slice: {names:?}"
        );
        assert!(names.contains(&"apply_fix"), "missing apply_fix: {names:?}");
    }

    #[test]
    fn mcp_dual_era_smoke() {
        let d = server_discover();
        let versions = d["protocolVersions"].as_array().expect("versions");
        assert!(versions.iter().any(|v| v.as_str() == Some("2024-11-05")));
        assert_eq!(d["stateless"].as_bool(), Some(true));
    }

    #[test]
    fn mcp_annotations_smoke() {
        let tools = tool_defs();
        let arr = tools.as_array().expect("tools");
        let graph = arr
            .iter()
            .find(|t| t["name"].as_str() == Some("rynix_graph"))
            .expect("rynix_graph");
        assert_eq!(graph["annotations"]["readOnlyHint"].as_bool(), Some(true));
        let fix = arr
            .iter()
            .find(|t| t["name"].as_str() == Some("apply_fix"))
            .expect("apply_fix");
        assert_eq!(fix["annotations"]["destructiveHint"].as_bool(), Some(true));
    }
}
