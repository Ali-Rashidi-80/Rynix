# Rynix VS Code extension

Syntax highlighting and LSP (`rynixc lsp-serve`) for `.ryx` files.

## Install (development)

```powershell
cd editors/vscode
npm install
npm run compile
```

Then **Extensions: Install from VSIX** or open this folder in VS Code and **Run Extension**.

## Settings

| Key | Default | Description |
|-----|---------|-------------|
| `rynix.compilerPath` | `rynixc` | Compiler on PATH |
| `rynix.enableLsp` | `true` | Start language server (diag / def / completion / rename) |
| `rynix.enableCodeLens` | `true` | check / alloc / impact CodeLens |
| `rynix.enableHover` | `true` | Soft-builtin hover tips |

## Features

- TextMate grammar for Rynix v0.1
- LSP via `vscode-languageclient` → `rynixc lsp-serve`:
  - diagnostics (`textDocument/publishDiagnostics`)
  - hover / go-to-definition
  - **completion** / **rename** / **references** / **workspace/symbol**
- CodeLens: check / explain-alloc / impact (CLI spawn, not LSP)
