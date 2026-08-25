//! `rynixc new` — scaffold a local package (no registry).

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::cli::NewOptions;

pub fn run(options: &NewOptions) -> ExitCode {
    let name = options.name.trim();
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        eprintln!("error: package name must be [A-Za-z0-9_-]+");
        return ExitCode::from(2);
    }
    let parent = options
        .path
        .clone()
        .unwrap_or_else(|| PathBuf::from("."));
    let root = parent.join(name);
    if root.exists() {
        eprintln!("error: {} already exists", root.display());
        return ExitCode::from(1);
    }
    if let Err(e) = fs::create_dir_all(root.join("src")) {
        eprintln!("error: cannot create {}: {e}", root.display());
        return ExitCode::from(3);
    }
    let toml = format!(
        r#"# Scaffolded by `rynixc new` — local packages only (no registry).

[package]
name = "{name}"
version = "0.1.0"
entry = "src/main.ryx"

[build]
runtime = "portable"
optimize = true
"#
    );
    let main = format!(
        r#"## {name} — entry point
def main() -> i64
  print_i64(0)
  return 0
end
"#
    );
    if let Err(e) = fs::write(root.join("rynix.toml"), toml) {
        eprintln!("error: write rynix.toml: {e}");
        return ExitCode::from(3);
    }
    if let Err(e) = fs::write(root.join("src/main.ryx"), main) {
        eprintln!("error: write main.ryx: {e}");
        return ExitCode::from(3);
    }
    println!("ok: created package `{name}` at {}", root.display());
    println!("  rynix.toml");
    println!("  src/main.ryx");
    println!("next: cd {} && rynixc build", root.display());
    println!("hint: local packages only (no CDN registry); rynixc deps --attest for rynix.attest.v1");
    ExitCode::SUCCESS
}
