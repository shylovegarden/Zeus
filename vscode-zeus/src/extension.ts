import * as path from 'path';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    Executable
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    console.log('Zeus extension activated. Starting Language Server...');

    // In a real release, this would point to the bundled binary or a globally installed `zeus`.
    // For local development, we point it to the Cargo target directory.
    const compilerPath = path.join(
        context.extensionPath,
        '..',
        'zeus_compiler',
        'target',
        'release',
        'zeus_compiler.exe' // Windows path
    );

    const run: Executable = {
        command: compilerPath,
        args: ['lsp'],
        options: { env: { ...process.env, RUST_BACKTRACE: "1" } }
    };

    const serverOptions: ServerOptions = {
        run,
        debug: run
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'zeus' }],
        synchronize: {
            // Notify the server about file changes to '.clientrc files contained in the workspace
            fileEvents: vscode.workspace.createFileSystemWatcher('**/.clientrc')
        }
    };

    client = new LanguageClient(
        'zeusLanguageServer',
        'Zeus Language Server',
        serverOptions,
        clientOptions
    );

    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
