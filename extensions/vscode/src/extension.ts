import * as vscode from 'vscode';
import * as path from 'path';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

export function activate(context: vscode.ExtensionContext) {
    console.log('Zeus extension activated');

    // Register build command
    let buildCmd = vscode.commands.registerCommand('zeus.build', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active Zeus file');
            return;
        }
        
        const filePath = editor.document.fileName;
        const terminal = vscode.window.createTerminal('Zeus Build');
        terminal.sendText(`zeus build "${filePath}"`);
        terminal.show();
    });

    // Register verify command
    let verifyCmd = vscode.commands.registerCommand('zeus.verify', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active Zeus file');
            return;
        }
        
        const filePath = editor.document.fileName;
        try {
            const { stdout } = await execAsync(`zeus verify "${filePath}"`);
            vscode.window.showInformationMessage('Zeus verification passed!');
        } catch (error) {
            vscode.window.showErrorMessage('Zeus verification failed');
        }
    });

    // Register run command
    let runCmd = vscode.commands.registerCommand('zeus.run', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active Zeus file');
            return;
        }
        
        const filePath = editor.document.fileName;
        const terminal = vscode.window.createTerminal('Zeus Run');
        terminal.sendText(`zeus run "${filePath}"`);
        terminal.show();
    });

    // Register document formatter
    let formatter = vscode.languages.registerDocumentFormattingEditProvider(
        'zeus',
        {
            async provideDocumentFormattingEdits(document: vscode.TextDocument): Promise<vscode.TextEdit[]> {
                const filePath = document.fileName;
                try {
                    await execAsync(`zeus fmt "${filePath}"`);
                    const formatted = await execAsync(`cat "${filePath}"`);
                    const fullRange = new vscode.Range(
                        document.positionAt(0),
                        document.positionAt(document.getText().length)
                    );
                    return [vscode.TextEdit.replace(fullRange, formatted.stdout)];
                } catch (error) {
                    vscode.window.showErrorMessage('Zeus format failed');
                    return [];
                }
            }
        }
    );

    // Add status bar item
    let statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    statusBar.text = "$(shield) Zeus";
    statusBar.tooltip = "Zeus Language Support";
    statusBar.command = 'zeus.build';
    statusBar.show();

    context.subscriptions.push(buildCmd, verifyCmd, runCmd, formatter, statusBar);
}

export function deactivate() {
    console.log('Zeus extension deactivated');
}
