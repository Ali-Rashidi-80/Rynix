"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const path = __importStar(require("path"));
const vscode = __importStar(require("vscode"));
const node_1 = require("vscode-languageclient/node");
const child_process_1 = require("child_process");
let client;
let output;
function compilerPath() {
    return vscode.workspace.getConfiguration('rynix').get('compilerPath', 'rynixc');
}
function runRynixc(args, cwd) {
    return new Promise((resolve) => {
        const child = (0, child_process_1.spawn)(compilerPath(), args, {
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
async function showToolResult(title, args, doc) {
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
class RynixHoverProvider {
    provideHover(document, position) {
        if (!vscode.workspace.getConfiguration('rynix').get('enableHover', true)) {
            return undefined;
        }
        const range = document.getWordRangeAtPosition(position, /[A-Za-z_][A-Za-z0-9_]*/);
        if (!range) {
            return undefined;
        }
        const word = document.getText(range);
        const soft = {
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
            return new vscode.Hover(new vscode.MarkdownString(`**def ${word}** — use CodeLens for check / alloc / impact`), range);
        }
        return undefined;
    }
}
/** CodeLens over `def name` — check / alloc / impact (SURPASS E1). */
class RynixCodeLensProvider {
    constructor() {
        this._onDidChange = new vscode.EventEmitter();
        this.onDidChangeCodeLenses = this._onDidChange.event;
    }
    refresh() {
        this._onDidChange.fire();
    }
    provideCodeLenses(document) {
        if (!vscode.workspace.getConfiguration('rynix').get('enableCodeLens', true)) {
            return [];
        }
        const lenses = [];
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
            lenses.push(new vscode.CodeLens(range, {
                title: 'Rynix: check',
                command: 'rynix.codeLens.check',
                arguments: [file],
            }), new vscode.CodeLens(range, {
                title: 'alloc',
                command: 'rynix.codeLens.alloc',
                arguments: [file],
            }), new vscode.CodeLens(range, {
                title: `impact ${fn}`,
                command: 'rynix.codeLens.impact',
                arguments: [file, fn],
            }));
        }
        return lenses;
    }
}
function activate(context) {
    const lenses = new RynixCodeLensProvider();
    context.subscriptions.push(vscode.languages.registerCodeLensProvider({ language: 'rynix', scheme: 'file' }, lenses), vscode.languages.registerHoverProvider({ language: 'rynix', scheme: 'file' }, new RynixHoverProvider()), vscode.workspace.onDidChangeConfiguration((e) => {
        if (e.affectsConfiguration('rynix.enableCodeLens')) {
            lenses.refresh();
        }
    }), vscode.commands.registerCommand('rynix.codeLens.check', async (file) => {
        const doc = await vscode.workspace.openTextDocument(file);
        await showToolResult('check', ['check', file, '--error-format=json'], doc);
    }), vscode.commands.registerCommand('rynix.codeLens.alloc', async (file) => {
        const doc = await vscode.workspace.openTextDocument(file);
        await showToolResult('explain-alloc', ['check', file, '--explain-alloc', '--error-format=json'], doc);
    }), vscode.commands.registerCommand('rynix.codeLens.impact', async (file, fn) => {
        const doc = await vscode.workspace.openTextDocument(file);
        await showToolResult('impact', ['impact', file, '--fn', fn, '--error-format=json'], doc);
    }));
    const config = vscode.workspace.getConfiguration('rynix');
    if (!config.get('enableLsp', true)) {
        return;
    }
    const serverOptions = {
        command: compilerPath(),
        args: ['lsp-serve'],
        transport: node_1.TransportKind.stdio,
    };
    const clientOptions = {
        documentSelector: [{ scheme: 'file', language: 'rynix' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.ryx'),
        },
    };
    client = new node_1.LanguageClient('rynix-lsp', 'Rynix Language Server', serverOptions, clientOptions);
    context.subscriptions.push({
        dispose: () => {
            void client?.stop();
        },
    });
    void client.start();
}
function deactivate() {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
//# sourceMappingURL=extension.js.map