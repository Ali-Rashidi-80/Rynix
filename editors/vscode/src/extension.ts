import * as path from 'path';
import * as vscode from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from 'vscode-languageclient/node';
import { spawn } from 'child_process';

let client: LanguageClient | undefined;
let output: vscode.OutputChannel | undefined;

function compilerPath(): string {
  return vscode.workspace.getConfiguration('rynix').get<string>('compilerPath', 'rynixc');
}

function runRynixc(args: string[], cwd?: string): Promise<{ code: number; out: string; err: string }> {
  return new Promise((resolve) => {
    const child = spawn(compilerPath(), args, {
      cwd,
      shell: process.platform === 'win32',
    });
    let out = '';
    let err = '';
    child.stdout.on('data', (d) => {
      out += String(d);
    });
    child.stderr.on('data', (d) => {
      err += String(d);
    });
    child.on('close', (code) => {
      resolve({ code: code ?? 1, out, err });
    });
    child.on('error', (e) => {
      resolve({ code: 1, out, err: String(e) });
    });
  });
}

async function showToolResult(title: string, args: string[], doc: vscode.TextDocument): Promise<void> {
  if (!output) {
    output = vscode.window.createOutputChannel('Rynix');
  }
  output.clear();
  output.show(true);
  output.appendLine(`$ rynixc ${args.join(' ')}`);
  const cwd = path.dirname(doc.uri.fsPath);
  const res = await runRynixc(args, cwd);
  if (res.out) {
    output.appendLine(res.out.trimEnd());
  }
  if (res.err) {
    output.appendLine(res.err.trimEnd());
  }
  output.appendLine(res.code === 0 ? `[ok] ${title}` : `[exit ${res.code}] ${title}`);
}

/** Hover soft-builtin / def names — SURPASS editor depth. */
class RynixHoverProvider implements vscode.HoverProvider {
  provideHover(
    document: vscode.TextDocument,
    position: vscode.Position,
  ): vscode.ProviderResult<vscode.Hover> {
    if (!vscode.workspace.getConfiguration('rynix').get<boolean>('enableHover', true)) {
      return undefined;
    }
    const range = document.getWordRangeAtPosition(position, /[A-Za-z_][A-Za-z0-9_]*/);
    if (!range) {
      return undefined;
    }
    const word = document.getText(range);
    const soft: Record<string, string> = {
      print_i64: 'soft: print_i64(n) — write integer to stdout',
      opaque_i64: 'soft: opaque_i64(x) — optimizer barrier (Suite5)',
      http_get_json_i64: 'soft: HTTP GET + JSON field extract',
      tls_client_echo: 'soft: TLS client echo (SChannel/OpenSSL)',
      sha256_first_i64: 'soft: SHA-256 first 8 bytes BE',
      hmac_sha256_first_i64: 'soft: HMAC-SHA256 first 8 bytes BE',
      ws_accept_key_eq: 'soft: RFC 6455 Sec-WebSocket-Accept check',
      kv_new: 'soft: arena string→i64 map',
      frame_client_echo: 'soft: length-prefixed frame echo client',
    };
    if (soft[word]) {
      return new vscode.Hover(new vscode.MarkdownString(`**${word}**\n\n${soft[word]}`), range);
    }
    const line = document.lineAt(position.line).text;
    if (/^\s*def\s+/.test(line) && line.includes(word)) {
      return new vscode.Hover(
        new vscode.MarkdownString(`**def ${word}** — use CodeLens for check / alloc / impact`),
        range,
      );
    }
    return undefined;
  }
}

/** CodeLens over `def name` — check / alloc / impact (SURPASS E1). */
class RynixCodeLensProvider implements vscode.CodeLensProvider {
  private readonly _onDidChange = new vscode.EventEmitter<void>();
  readonly onDidChangeCodeLenses = this._onDidChange.event;

  refresh(): void {
    this._onDidChange.fire();
  }

  provideCodeLenses(document: vscode.TextDocument): vscode.CodeLens[] {
    if (!vscode.workspace.getConfiguration('rynix').get<boolean>('enableCodeLens', true)) {
      return [];
    }
    const lenses: vscode.CodeLens[] = [];
    const re = /^\s*def\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/;
    for (let i = 0; i < document.lineCount; i++) {
      const line = document.lineAt(i);
      const m = re.exec(line.text);
      if (!m) {
        continue;
      }
      const fn = m[1];
      const range = new vscode.Range(i, 0, i, line.text.length);
      const file = document.uri.fsPath;
      lenses.push(
        new vscode.CodeLens(range, {
          title: 'Rynix: check',
          command: 'rynix.codeLens.check',
          arguments: [file],
        }),
        new vscode.CodeLens(range, {
          title: 'alloc',
          command: 'rynix.codeLens.alloc',
          arguments: [file],
        }),
        new vscode.CodeLens(range, {
          title: `impact ${fn}`,
          command: 'rynix.codeLens.impact',
          arguments: [file, fn],
        }),
      );
    }
    return lenses;
  }
}

export function activate(context: vscode.ExtensionContext): void {
  const lenses = new RynixCodeLensProvider();
  context.subscriptions.push(
    vscode.languages.registerCodeLensProvider({ language: 'rynix', scheme: 'file' }, lenses),
    vscode.languages.registerHoverProvider({ language: 'rynix', scheme: 'file' }, new RynixHoverProvider()),
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration('rynix.enableCodeLens')) {
        lenses.refresh();
      }
    }),
    vscode.commands.registerCommand('rynix.codeLens.check', async (file: string) => {
      const doc = await vscode.workspace.openTextDocument(file);
      await showToolResult('check', ['check', file, '--error-format=json'], doc);
    }),
    vscode.commands.registerCommand('rynix.codeLens.alloc', async (file: string) => {
      const doc = await vscode.workspace.openTextDocument(file);
      await showToolResult(
        'explain-alloc',
        ['check', file, '--explain-alloc', '--error-format=json'],
        doc,
      );
    }),
    vscode.commands.registerCommand('rynix.codeLens.impact', async (file: string, fn: string) => {
      const doc = await vscode.workspace.openTextDocument(file);
      await showToolResult('impact', ['impact', file, '--fn', fn, '--error-format=json'], doc);
    }),
  );

  const config = vscode.workspace.getConfiguration('rynix');
  if (!config.get<boolean>('enableLsp', true)) {
    return;
  }

  const serverOptions: ServerOptions = {
    command: compilerPath(),
    args: ['lsp-serve'],
    transport: TransportKind.stdio,
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'rynix' }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher('**/*.ryx'),
    },
  };

  client = new LanguageClient('rynix-lsp', 'Rynix Language Server', serverOptions, clientOptions);
  context.subscriptions.push({
    dispose: () => {
      void client?.stop();
    },
  });
  void client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
