import * as path from 'path';
import * as vscode from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

export function activate(context: vscode.ExtensionContext): void {
  const config = vscode.workspace.getConfiguration('rynix');
  if (!config.get<boolean>('enableLsp', true)) {
    return;
  }

  const compilerPath = config.get<string>('compilerPath', 'rynixc');
  const serverOptions: ServerOptions = {
    command: compilerPath,
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
  context.subscriptions.push({ dispose: () => { void client?.stop(); } });
  void client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
