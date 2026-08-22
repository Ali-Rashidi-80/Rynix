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
| `rynix.enableLsp` | `true` | Start language server |

## Features

- TextMate grammar for Rynix v0.1
- Diagnostics from `rynixc check` pipeline
- Go-to-definition via sema `path_resolution`
