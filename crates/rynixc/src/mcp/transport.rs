//! MCP stdio JSON-RPC framing.

use std::io::{self, BufRead, Write};

use serde_json::{Value, json};

pub(crate) fn rpc_error(code: i64, message: impl AsRef<str>) -> Value {
    json!({ "code": code, "message": message.as_ref() })
}

/// MCP tool result the model can read and correct (SEP-style), not a bare protocol error.
pub(crate) fn tool_result_error(message: impl AsRef<str>) -> Value {
    json!({
        "content": [{ "type": "text", "text": message.as_ref() }],
        "isError": true
    })
}

pub(crate) fn write_result(
    stdout: &mut impl Write,
    id: Option<Value>,
    result: Result<Value, Value>,
) {
    let mut body = json!({ "jsonrpc": "2.0" });
    if let Some(id) = id {
        body["id"] = id;
    }
    match result {
        Ok(v) => body["result"] = v,
        Err(e) => body["error"] = e,
    }
    let _ = write_message(stdout, &body);
}

pub(crate) fn read_message(reader: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Ok(None);
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            content_length = rest.trim().parse().ok();
        }
    }
    let len = content_length
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length"))?;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    let v: Value =
        serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(Some(v))
}

pub(crate) fn write_message(writer: &mut impl Write, value: &Value) -> io::Result<()> {
    let data = serde_json::to_vec(value)?;
    write!(writer, "Content-Length: {}\r\n\r\n", data.len())?;
    writer.write_all(&data)?;
    writer.flush()?;
    Ok(())
}
