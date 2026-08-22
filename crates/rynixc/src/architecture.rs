//! Architecture.toml layer boundary checker (`rynixc arch check`).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

#[derive(Debug, Clone, Default)]
pub struct ArchitectureConfig {
    pub invariants: HashMap<String, RuleInvariant>,
    pub layout: LayoutRules,
}

#[derive(Debug, Clone, Default)]
pub struct LayoutRules {
    pub required_dirs: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RuleInvariant {
    pub cannot_import: Vec<String>,
    pub forbidden_call: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ArchitectureViolation {
    pub rule_pattern: String,
    pub file: String,
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ArchitectureCheckReport {
    pub status: String,
    pub rules_checked: usize,
    pub files_scanned: usize,
    pub violations_count: usize,
    pub violations: Vec<ArchitectureViolation>,
}

impl ArchitectureCheckReport {
    pub fn to_json(&self) -> Value {
        json!({
            "schema": "rynix.arch.v1",
            "status": self.status,
            "rules_checked": self.rules_checked,
            "files_scanned": self.files_scanned,
            "violations_count": self.violations_count,
            "violations": self.violations.iter().map(|v| json!({
                "rule_pattern": v.rule_pattern,
                "file": v.file,
                "line": v.line,
                "message": v.message,
            })).collect::<Vec<_>>(),
        })
    }
}

pub struct ArchitectureEngine;

impl ArchitectureEngine {
    pub fn load_config(config_path: Option<&Path>) -> Result<ArchitectureConfig, String> {
        let candidates: Vec<PathBuf> = if let Some(p) = config_path {
            vec![p.to_path_buf()]
        } else {
            vec![
                PathBuf::from("Architecture.toml"),
                PathBuf::from("architecture.toml"),
            ]
        };
        for path in candidates {
            if path.is_file() {
                let content = fs::read_to_string(&path)
                    .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
                return Ok(parse_architecture_toml(&content));
            }
        }
        Err("Architecture.toml not found".into())
    }

    pub fn check_project(config: &ArchitectureConfig, root: &Path) -> ArchitectureCheckReport {
        let mut violations = Vec::new();
        let mut files_scanned = 0usize;

        for dir in &config.layout.required_dirs {
            let p = root.join(dir);
            if !p.is_dir() {
                violations.push(ArchitectureViolation {
                    rule_pattern: "layout".into(),
                    file: dir.clone(),
                    line: 0,
                    message: format!("required directory missing: {dir}"),
                });
            }
        }

        let mut ryx_files = Vec::new();
        collect_ryx_files(root, &mut ryx_files);

        for file_path in &ryx_files {
            files_scanned += 1;
            let rel = file_path
                .strip_prefix(root)
                .unwrap_or(file_path)
                .to_string_lossy()
                .replace('\\', "/");
            let source = fs::read_to_string(file_path).unwrap_or_default();
            let lines: Vec<&str> = source.lines().collect();

            for (pattern, rule) in &config.invariants {
                if !match_glob(pattern, &rel) {
                    continue;
                }
                for (idx, line) in lines.iter().enumerate() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("import ") {
                        let target = trimmed
                            .trim_start_matches("import ")
                            .trim_matches(|c: char| c == '"' || c == ';' || c.is_whitespace())
                            .replace('\\', "/")
                            .replace("::", "/");
                        for forbidden in &rule.cannot_import {
                            if match_glob(forbidden, &target)
                                || target.starts_with(forbidden.trim_end_matches("/**"))
                            {
                                violations.push(ArchitectureViolation {
                                    rule_pattern: pattern.clone(),
                                    file: rel.clone(),
                                    line: idx + 1,
                                    message: format!(
                                        "layer '{pattern}' cannot import '{target}' (forbidden by {forbidden})"
                                    ),
                                });
                            }
                        }
                    }
                    for call in &rule.forbidden_call {
                        if trimmed.contains(call) {
                            violations.push(ArchitectureViolation {
                                rule_pattern: pattern.clone(),
                                file: rel.clone(),
                                line: idx + 1,
                                message: format!(
                                    "layer '{pattern}' forbids call pattern '{call}'"
                                ),
                            });
                        }
                    }
                }
            }
        }

        let status = if violations.is_empty() {
            "passed".into()
        } else {
            "violation_detected".into()
        };

        ArchitectureCheckReport {
            status,
            rules_checked: config.invariants.len(),
            files_scanned,
            violations_count: violations.len(),
            violations,
        }
    }
}

fn flush_rule(config: &mut ArchitectureConfig, key: Option<&String>, rule: &RuleInvariant) {
    if let Some(k) = key {
        config.invariants.insert(k.clone(), rule.clone());
    }
}

/// Minimal TOML subset for Architecture.toml (no external crate).
fn parse_architecture_toml(content: &str) -> ArchitectureConfig {
    let mut config = ArchitectureConfig::default();
    let mut section = String::new();
    let mut current_key: Option<String> = None;
    let mut current_rule = RuleInvariant::default();

    for line in content.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            flush_rule(&mut config, current_key.as_ref(), &current_rule);
            current_key = None;
            current_rule = RuleInvariant::default();
            section = line.trim_matches(['[', ']']).to_string();
            continue;
        }
        if section == "layout" {
            if let Some(val) = parse_string_list(line, "required_dirs") {
                config.layout.required_dirs = val;
            }
            continue;
        }
        if section == "invariants" {
            if let Some((pat, rest)) = line.split_once('=') {
                let pat = pat.trim().trim_matches('"').to_string();
                current_key = Some(pat);
                current_rule = RuleInvariant::default();
                let rest = rest.trim();
                if rest.starts_with('{') {
                    parse_rule_inline(rest, &mut current_rule);
                }
            } else if line.contains('=') {
                parse_rule_field(line, &mut current_rule);
            }
        }
    }
    flush_rule(&mut config, current_key.as_ref(), &current_rule);
    config
}

fn parse_string_list(line: &str, key: &str) -> Option<Vec<String>> {
    let (k, v) = line.split_once('=')?;
    if k.trim() != key {
        return None;
    }
    let v = v.trim().trim_start_matches('[').trim_end_matches(']');
    Some(
        v.split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    )
}

fn parse_rule_inline(block: &str, rule: &mut RuleInvariant) {
    let inner = block.trim().trim_matches(['{', '}']);
    for part in inner.split(',') {
        parse_rule_field(part, rule);
    }
}

fn parse_rule_field(line: &str, rule: &mut RuleInvariant) {
    let Some((k, v)) = line.split_once('=') else {
        return;
    };
    let k = k.trim();
    let v = v.trim().trim_start_matches('[').trim_end_matches(']');
    let items: Vec<String> = v
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect();
    match k {
        "cannot_import" => rule.cannot_import = items,
        "forbidden_call" => rule.forbidden_call = items,
        _ => {}
    }
}

fn collect_ryx_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = fs::read_dir(dir) else { return };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "target" || name.starts_with('.') {
                continue;
            }
            collect_ryx_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("ryx") {
            out.push(path);
        }
    }
}

fn match_glob(pattern: &str, text: &str) -> bool {
    let pat = pattern.replace('\\', "/");
    let text = text.replace('\\', "/");
    if pat.ends_with("/**") {
        let prefix = pat.trim_end_matches("/**");
        return text.starts_with(prefix);
    }
    if pat.contains('*') {
        let parts: Vec<&str> = pat.split('*').collect();
        if parts.len() == 2 {
            return text.starts_with(parts[0]) && text.ends_with(parts[1]);
        }
    }
    pat == text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_repo_architecture_toml() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let cfg_path = root.join("Architecture.toml");
        let text = fs::read_to_string(cfg_path).expect("Architecture.toml");
        let cfg = parse_architecture_toml(&text);
        assert!(!cfg.layout.required_dirs.is_empty());
        assert!(!cfg.invariants.is_empty());
    }

    #[test]
    fn import_paths_normalize_colons() {
        let dir = std::env::temp_dir().join("rynix_arch_colon_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("examples/bad")).unwrap();
        fs::write(
            dir.join("Architecture.toml"),
            r#"
[invariants]
"examples/**" = { cannot_import = ["std/fs"] }
"#,
        )
        .unwrap();
        fs::write(dir.join("examples/bad/evil.ryx"), "import std::fs\n").unwrap();
        let cfg = ArchitectureEngine::load_config(Some(&dir.join("Architecture.toml")))
            .expect("load");
        let report = ArchitectureEngine::check_project(&cfg, &dir);
        assert_eq!(report.violations_count, 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn detects_forbidden_import_violation() {
        let dir = std::env::temp_dir().join("rynix_arch_violation_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("layer/bad")).unwrap();
        fs::write(
            dir.join("Architecture.toml"),
            r#"
[invariants]
"layer/**" = { cannot_import = ["std/net"] }
"#,
        )
        .unwrap();
        fs::write(dir.join("layer/bad/evil.ryx"), "import std::net\n").unwrap();
        let cfg = ArchitectureEngine::load_config(Some(&dir.join("Architecture.toml")))
            .expect("load");
        let report = ArchitectureEngine::check_project(&cfg, &dir);
        assert_eq!(report.violations_count, 1, "{:?}", report.violations);
        let _ = fs::remove_dir_all(&dir);
    }
}
