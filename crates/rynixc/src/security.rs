//! Line-oriented security scan (`rynixc security`) — subset of real CWEs.
//!
//! Honest scope: pattern matching on source text, not full taint / “100% CWE
//! eliminated”. Findings are advisory; exit 1 when any HIGH/CRITICAL hit.

use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct Finding {
    pub cwe: &'static str,
    pub title: &'static str,
    pub severity: &'static str,
    pub line: usize,
    pub snippet: String,
}

#[derive(Debug, Clone)]
pub struct SecurityReport {
    pub path: String,
    pub findings: Vec<Finding>,
}

impl SecurityReport {
    pub fn to_json(&self) -> Value {
        json!({
            "schema": "rynix.security.v1",
            "path": self.path,
            "finding_count": self.findings.len(),
            "blocking": self.blocking(),
            "findings": self.findings.iter().map(|f| json!({
                "cwe": f.cwe,
                "title": f.title,
                "severity": f.severity,
                "line": f.line,
                "snippet": f.snippet,
            })).collect::<Vec<_>>(),
            "disclaimer": "Pattern scan only — not a complete security audit.",
        })
    }

    pub fn blocking(&self) -> bool {
        self.findings
            .iter()
            .any(|f| f.severity == "CRITICAL" || f.severity == "HIGH")
    }
}

/// Known-bad substrings (CWE-798 class). Snippets are truncated; secrets not echoed fully.
pub fn scan_source(path: &str, source: &str) -> SecurityReport {
    let patterns: &[(&str, &str, &str)] = &[
        ("sk_live_", "Stripe live API key material", "CRITICAL"),
        ("AKIA", "AWS access key id prefix", "CRITICAL"),
        ("ghp_", "GitHub personal access token prefix", "CRITICAL"),
        ("-----BEGIN PRIVATE KEY-----", "PEM private key literal", "CRITICAL"),
        ("Bearer eyJ", "Hardcoded JWT bearer token", "HIGH"),
        ("password = \"", "Hardcoded password assignment", "HIGH"),
        ("password=\"", "Hardcoded password assignment", "HIGH"),
        ("secret_key = \"", "Hardcoded secret_key assignment", "HIGH"),
        ("api_key = \"", "Hardcoded api_key assignment", "HIGH"),
        ("api_key=\"", "Hardcoded api_key assignment", "HIGH"),
    ];

    let mut findings = Vec::new();
    for (idx, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("##") || trimmed.starts_with("//") {
            continue;
        }
        for (pat, title, sev) in patterns {
            if line.contains(pat) {
                findings.push(Finding {
                    cwe: "CWE-798",
                    title,
                    severity: sev,
                    line: idx + 1,
                    snippet: redact(line),
                });
            }
        }
    }

    SecurityReport {
        path: path.to_string(),
        findings,
    }
}

fn redact(line: &str) -> String {
    let mut s = line.trim().to_string();
    if s.len() > 80 {
        s.truncate(77);
        s.push_str("...");
    }
    // Blur quoted tails after `=`.
    if let Some(eq) = s.find('=') {
        let (head, tail) = s.split_at(eq + 1);
        let blurred: String = tail
            .chars()
            .map(|c| if c == '"' || c.is_whitespace() { c } else { '*' })
            .collect();
        return format!("{head}{blurred}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_sk_live() {
        let r = scan_source(
            "t.ryx",
            "def main() -> i64\n  let k = \"sk_live_TEST\"\n  return 0\nend\n",
        );
        assert!(r.blocking());
        assert_eq!(r.findings[0].cwe, "CWE-798");
    }

    #[test]
    fn clean_source_ok() {
        let r = scan_source("t.ryx", "def main() -> i64\n  return 1\nend\n");
        assert!(!r.blocking());
        assert!(r.findings.is_empty());
    }
}
