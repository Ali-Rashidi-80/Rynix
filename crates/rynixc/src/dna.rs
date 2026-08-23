//! Project “DNA” heuristics — conventions report for agents (SURPASS B6).
//!
//! Honest scope: naming / architecture / soft-stdlib signals from `.ryx` text.
//! Not “80 Stable layers”. Schema: `rynix.dna.v1`.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

#[derive(Debug, Clone, Default)]
pub struct NamingStats {
    pub fn_snake: usize,
    pub fn_camel: usize,
    pub fn_other: usize,
    pub struct_pascal: usize,
    pub struct_other: usize,
}

#[derive(Debug, Clone)]
pub struct DnaReport {
    pub root: PathBuf,
    pub project_name: String,
    pub scanned_files: usize,
    pub scanned_defs: usize,
    pub naming: NamingStats,
    pub architecture_toml: bool,
    pub rynix_toml: bool,
    pub uses_region: bool,
    pub uses_pipe: bool,
    pub uses_fibers: bool,
    pub uses_http: bool,
    pub uses_tls: bool,
    pub effect_pure_attrs: usize,
    pub confidence: f64,
}

impl DnaReport {
    pub fn function_style(&self) -> &'static str {
        if self.naming.fn_snake >= self.naming.fn_camel {
            "snake_case"
        } else {
            "camelCase"
        }
    }

    pub fn struct_style(&self) -> &'static str {
        if self.naming.struct_pascal > 0 {
            "PascalCase"
        } else {
            "unknown"
        }
    }

    pub fn concurrency_model(&self) -> &'static str {
        if self.uses_fibers {
            "cooperative_fibers"
        } else {
            "single_thread_default"
        }
    }

    pub fn memory_strategy(&self) -> &'static str {
        if self.uses_region {
            "explicit_region_scopes"
        } else {
            "escape_inferred"
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "schema": "rynix.dna.v1",
            "root": self.root.display().to_string(),
            "project_name": self.project_name,
            "scanned_files": self.scanned_files,
            "scanned_defs": self.scanned_defs,
            "naming": {
                "function_style": self.function_style(),
                "struct_style": self.struct_style(),
                "fn_snake": self.naming.fn_snake,
                "fn_camel": self.naming.fn_camel,
                "fn_other": self.naming.fn_other,
                "struct_pascal": self.naming.struct_pascal,
                "struct_other": self.naming.struct_other,
            },
            "signals": {
                "architecture_toml": self.architecture_toml,
                "rynix_toml": self.rynix_toml,
                "region": self.uses_region,
                "pipe": self.uses_pipe,
                "fibers": self.uses_fibers,
                "http": self.uses_http,
                "tls": self.uses_tls,
                "effect_pure_attrs": self.effect_pure_attrs,
            },
            "architecture_style": if self.architecture_toml {
                "Architecture.toml_enforced"
            } else {
                "ad_hoc_or_flat"
            },
            "concurrency_model": self.concurrency_model(),
            "memory_strategy": self.memory_strategy(),
            "confidence": self.confidence,
            "disclaimer": "Heuristic conventions report — not a formal architecture proof.",
        })
    }

    pub fn to_prompt(&self) -> String {
        format!(
            "Rynix project DNA (heuristic)\n\
             - name: {}\n\
             - functions: {} (snake={}, camel={})\n\
             - structs: {}\n\
             - memory: {}\n\
             - concurrency: {}\n\
             - arch file: {}\n\
             - signals: region={} pipe={} fibers={} http={} tls={} pure_attrs={}\n\
             - confidence: {:.0}%\n\
             Prefer matching these conventions when editing.\n",
            self.project_name,
            self.function_style(),
            self.naming.fn_snake,
            self.naming.fn_camel,
            self.struct_style(),
            self.memory_strategy(),
            self.concurrency_model(),
            self.architecture_toml,
            self.uses_region,
            self.uses_pipe,
            self.uses_fibers,
            self.uses_http,
            self.uses_tls,
            self.effect_pure_attrs,
            self.confidence * 100.0,
        )
    }
}

fn is_snake(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && (name.contains('_') || name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()))
}

fn is_camel(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => name.chars().any(|c| c.is_ascii_uppercase()),
        _ => false,
    }
}

fn is_pascal(name: &str) -> bool {
    match name.chars().next() {
        Some(c) if c.is_ascii_uppercase() => true,
        _ => false,
    }
}

fn collect_ryx(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for ent in rd.flatten() {
        let path = ent.path();
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if name == "target" || name == "node_modules" || name == ".git" {
            continue;
        }
        if path.is_dir() {
            collect_ryx(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("ryx") {
            out.push(path);
        }
    }
}

fn mine_file(text: &str, naming: &mut NamingStats, report: &mut DnaReport) {
    if text.contains("region ") {
        report.uses_region = true;
    }
    if text.contains("|>") {
        report.uses_pipe = true;
    }
    if text.contains("spawn") || text.contains("fiber_run") || text.contains("yield(") {
        report.uses_fibers = true;
    }
    if text.contains("http_") {
        report.uses_http = true;
    }
    if text.contains("tls_") {
        report.uses_tls = true;
    }
    report.effect_pure_attrs += text.matches("#^ effect: pure").count();

    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("def ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if name.is_empty() {
                continue;
            }
            report.scanned_defs += 1;
            if is_snake(&name) {
                naming.fn_snake += 1;
            } else if is_camel(&name) {
                naming.fn_camel += 1;
            } else {
                naming.fn_other += 1;
            }
        } else if let Some(rest) = t.strip_prefix("struct ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if name.is_empty() {
                continue;
            }
            report.scanned_defs += 1;
            if is_pascal(&name) {
                naming.struct_pascal += 1;
            } else {
                naming.struct_other += 1;
            }
        }
    }
}

/// Mine conventions under `root` (recursive `.ryx` scan).
pub fn mine_dna(root: &Path) -> DnaReport {
    let root = root
        .canonicalize()
        .unwrap_or_else(|_| root.to_path_buf());
    let project_name = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("rynix-project")
        .to_string();
    let mut report = DnaReport {
        root: root.clone(),
        project_name,
        scanned_files: 0,
        scanned_defs: 0,
        naming: NamingStats::default(),
        architecture_toml: root.join("Architecture.toml").is_file(),
        rynix_toml: root.join("rynix.toml").is_file(),
        uses_region: false,
        uses_pipe: false,
        uses_fibers: false,
        uses_http: false,
        uses_tls: false,
        effect_pure_attrs: 0,
        confidence: 0.0,
    };
    let mut files = Vec::new();
    collect_ryx(&root, &mut files);
    let mut naming = NamingStats::default();
    for path in &files {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        report.scanned_files += 1;
        mine_file(&text, &mut naming, &mut report);
    }
    report.naming = naming;
    let signals = (report.architecture_toml as usize)
        + (report.rynix_toml as usize)
        + (report.uses_region as usize)
        + (report.uses_pipe as usize)
        + (report.uses_fibers as usize)
        + report.scanned_defs.min(20) / 5;
    report.confidence = (0.25 + 0.1 * signals as f64).min(0.95);
    if report.scanned_files == 0 {
        report.confidence = 0.1;
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn mines_snake_and_region() {
        let dir = std::env::temp_dir().join("rynix_dna_unit");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("main.ryx"),
            "def hello_world() -> i64\n  region r\n    return 1\n  end\nend\n",
        )
        .unwrap();
        let r = mine_dna(&dir);
        assert!(r.uses_region);
        assert_eq!(r.function_style(), "snake_case");
        assert!(r.scanned_defs >= 1);
    }
}
