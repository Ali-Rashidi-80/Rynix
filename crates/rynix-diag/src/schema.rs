//! Structural validation of `rynix.diag.v1` objects against the frozen shape.
//!
//! The normative JSON Schema lives at `docs/schemas/rynix.diag.v1.json`. This
//! module mirrors that document so golden tests do not need an external
//! schema engine on the compiler's critical path.

use serde_json::Value;

/// Validates that `value` conforms to `rynix.diag.v1`.
pub fn validate_diag_v1(value: &Value) -> Result<(), String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "diagnostic must be a JSON object".to_string())?;

    for key in obj.keys() {
        if !matches!(
            key.as_str(),
            "schema" | "code" | "severity" | "stage" | "message" | "spans" | "fixes"
        ) {
            return Err(format!("unknown field `{key}`"));
        }
    }
    for req in [
        "schema", "code", "severity", "stage", "message", "spans", "fixes",
    ] {
        if !obj.contains_key(req) {
            return Err(format!("missing required field `{req}`"));
        }
    }

    if obj["schema"] != "rynix.diag.v1" {
        return Err(format!(
            "schema must be rynix.diag.v1, got {}",
            obj["schema"]
        ));
    }

    let code = obj["code"]
        .as_str()
        .ok_or_else(|| "code must be a string".to_string())?;
    if code.len() != 7 || !code.starts_with("RYX") || !code[3..].bytes().all(|b| b.is_ascii_digit())
    {
        return Err(format!("code `{code}` is not RYX####"));
    }

    let severity = obj["severity"]
        .as_str()
        .ok_or_else(|| "severity must be a string".to_string())?;
    if !matches!(severity, "error" | "warning" | "note" | "help") {
        return Err(format!("invalid severity `{severity}`"));
    }

    let stage = obj["stage"]
        .as_str()
        .ok_or_else(|| "stage must be a string".to_string())?;
    if !matches!(stage, "lex" | "parse" | "sema" | "ir" | "codegen") {
        return Err(format!("invalid stage `{stage}`"));
    }

    let message = obj["message"]
        .as_str()
        .ok_or_else(|| "message must be a string".to_string())?;
    if message.is_empty() {
        return Err("message must be non-empty".to_string());
    }

    let spans = obj["spans"]
        .as_array()
        .ok_or_else(|| "spans must be an array".to_string())?;
    if spans.is_empty() {
        return Err("spans must contain at least one entry".to_string());
    }
    let mut saw_primary = false;
    for (i, span) in spans.iter().enumerate() {
        validate_span(span).map_err(|e| format!("spans[{i}]: {e}"))?;
        if span["primary"].as_bool() == Some(true) {
            saw_primary = true;
        }
    }
    if !saw_primary {
        return Err("spans must include a primary span".to_string());
    }

    let fixes = obj["fixes"]
        .as_array()
        .ok_or_else(|| "fixes must be an array".to_string())?;
    for (i, fix) in fixes.iter().enumerate() {
        validate_fix(fix).map_err(|e| format!("fixes[{i}]: {e}"))?;
    }
    Ok(())
}

fn validate_span(value: &Value) -> Result<(), String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "span must be an object".to_string())?;
    for key in obj.keys() {
        if !matches!(
            key.as_str(),
            "file" | "lo" | "hi" | "line" | "col" | "end_line" | "end_col" | "primary" | "label"
        ) {
            return Err(format!("unknown field `{key}`"));
        }
    }
    for req in [
        "file", "lo", "hi", "line", "col", "end_line", "end_col", "primary", "label",
    ] {
        if !obj.contains_key(req) {
            return Err(format!("missing `{req}`"));
        }
    }
    require_nonempty_str(obj, "file")?;
    require_u64(obj, "lo")?;
    require_u64(obj, "hi")?;
    require_u64_min(obj, "line", 1)?;
    require_u64_min(obj, "col", 1)?;
    require_u64_min(obj, "end_line", 1)?;
    require_u64_min(obj, "end_col", 1)?;
    if !obj["primary"].is_boolean() {
        return Err("primary must be a boolean".to_string());
    }
    if !obj["label"].is_string() {
        return Err("label must be a string".to_string());
    }
    let lo = obj["lo"].as_u64().unwrap();
    let hi = obj["hi"].as_u64().unwrap();
    if hi < lo {
        return Err(format!("hi ({hi}) < lo ({lo})"));
    }
    Ok(())
}

fn validate_fix(value: &Value) -> Result<(), String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "fix must be an object".to_string())?;
    for key in obj.keys() {
        if !matches!(key.as_str(), "message" | "confidence" | "edits") {
            return Err(format!("unknown field `{key}`"));
        }
    }
    for req in ["message", "confidence", "edits"] {
        if !obj.contains_key(req) {
            return Err(format!("missing `{req}`"));
        }
    }
    require_nonempty_str(obj, "message")?;
    let confidence = obj["confidence"]
        .as_f64()
        .ok_or_else(|| "confidence must be a number".to_string())?;
    if !(0.0..=1.0).contains(&confidence) {
        return Err(format!("confidence {confidence} out of range"));
    }
    let edits = obj["edits"]
        .as_array()
        .ok_or_else(|| "edits must be an array".to_string())?;
    if edits.is_empty() {
        return Err("edits must be non-empty".to_string());
    }
    for (i, edit) in edits.iter().enumerate() {
        validate_edit(edit).map_err(|e| format!("edits[{i}]: {e}"))?;
    }
    Ok(())
}

fn validate_edit(value: &Value) -> Result<(), String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "edit must be an object".to_string())?;
    for key in obj.keys() {
        if !matches!(key.as_str(), "file" | "lo" | "hi" | "replacement") {
            return Err(format!("unknown field `{key}`"));
        }
    }
    for req in ["file", "lo", "hi", "replacement"] {
        if !obj.contains_key(req) {
            return Err(format!("missing `{req}`"));
        }
    }
    require_nonempty_str(obj, "file")?;
    require_u64(obj, "lo")?;
    require_u64(obj, "hi")?;
    if !obj["replacement"].is_string() {
        return Err("replacement must be a string".to_string());
    }
    Ok(())
}

fn require_nonempty_str(obj: &serde_json::Map<String, Value>, key: &str) -> Result<(), String> {
    match obj[key].as_str() {
        Some(s) if !s.is_empty() => Ok(()),
        Some(_) => Err(format!("`{key}` must be non-empty")),
        None => Err(format!("`{key}` must be a string")),
    }
}

fn require_u64(obj: &serde_json::Map<String, Value>, key: &str) -> Result<(), String> {
    if obj[key].as_u64().is_some() {
        Ok(())
    } else {
        Err(format!("`{key}` must be a non-negative integer"))
    }
}

fn require_u64_min(
    obj: &serde_json::Map<String, Value>,
    key: &str,
    min: u64,
) -> Result<(), String> {
    let n = obj[key]
        .as_u64()
        .ok_or_else(|| format!("`{key}` must be a non-negative integer"))?;
    if n < min {
        Err(format!("`{key}` must be >= {min}"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_minimal_valid_object() {
        let v = json!({
            "schema": "rynix.diag.v1",
            "code": "RYX0001",
            "severity": "error",
            "stage": "lex",
            "message": "unknown character",
            "spans": [{
                "file": "a.ryx",
                "lo": 0, "hi": 1,
                "line": 1, "col": 1,
                "end_line": 1, "end_col": 2,
                "primary": true,
                "label": ""
            }],
            "fixes": []
        });
        validate_diag_v1(&v).unwrap();
    }

    #[test]
    fn rejects_wrong_schema() {
        let mut v = json!({
            "schema": "rynix.diag.v0",
            "code": "RYX0001",
            "severity": "error",
            "stage": "lex",
            "message": "x",
            "spans": [{
                "file": "a.ryx", "lo": 0, "hi": 1,
                "line": 1, "col": 1, "end_line": 1, "end_col": 2,
                "primary": true, "label": ""
            }],
            "fixes": []
        });
        assert!(validate_diag_v1(&v).unwrap_err().contains("schema"));
        v["schema"] = json!("rynix.diag.v1");
        v.as_object_mut().unwrap().remove("spans");
        assert!(validate_diag_v1(&v).unwrap_err().contains("spans"));
    }
}
