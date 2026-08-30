//! Dual-era MCP discovery (stdio primary; stateless-ready metadata).

use serde_json::{Value, json};

pub(crate) fn server_discover() -> Value {
    json!({
        "protocolVersions": ["2024-11-05", "2025-03-26", "2025-06-18"],
        "capabilities": {
            "tools": { "listChanged": false }
        },
        "serverInfo": {
            "name": "rynixc",
            "version": env!("CARGO_PKG_VERSION")
        },
        "transport": ["stdio"],
        "stateless": true
    })
}
