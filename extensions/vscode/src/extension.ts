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

    // Register proof visualization command
    let proofVizCmd = vscode.commands.registerCommand('zeus.proofViz', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active Zeus file');
            return;
        }
        
        const filePath = editor.document.fileName;
        const outputPath = filePath.replace('.zs', '_proof.html');
        
        try {
            await execAsync(`zeus proof-viz "${filePath}" -o "${outputPath}"`);
            vscode.window.showInformationMessage(`Proof visualization saved to ${outputPath}`);
            
            // Open in browser
            const panel = vscode.window.createWebviewPanel(
                'zeusProof',
                'Zeus Proof Visualization',
                vscode.ViewColumn.Two,
                { enableScripts: true }
            );
            
            const htmlContent = await execAsync(`cat "${outputPath}"`);
            panel.webview.html = htmlContent.stdout;
        } catch (error) {
            vscode.window.showErrorMessage('Proof visualization failed');
        }
    });

    // Register certificate command
    let certCmd = vscode.commands.registerCommand('zeus.certificate', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active Zeus file');
            return;
        }
        
        const filePath = editor.document.fileName;
        const baseName = path.basename(filePath, '.zs');
        const certPath = path.join(path.dirname(filePath), baseName + '.zcert');
        
        try {
            const { stdout } = await execAsync(`zeus cert "${filePath}"`);
            vscode.window.showInformationMessage('Certificate generated successfully!');
            
            // Show certificate summary
            const panel = vscode.window.createWebviewPanel(
                'zeusCert',
                'Zeus Certificate',
                vscode.ViewColumn.Two,
                {}
            );
            
            panel.webview.html = `
                <html>
                <head><style>
                    body { font-family: sans-serif; padding: 20px; background: #f5f5f5; }
                    .cert { background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }
                    .badge { display: inline-block; padding: 5px 10px; margin: 5px; border-radius: 4px; background: #4caf50; color: white; }
                </style></head>
                <body>
                    <div class="cert">
                        <h2>Zeus Security Certificate</h2>
                        <pre>${stdout.replace(/</g, '&lt;')}</pre>
                        <div class="badge">Verified</div>
                        <div class="badge">Zero-Heap</div>
                        <div class="badge">Constant-Time</div>
                    </div>
                </body>
                </html>
            `;
        } catch (error) {
            vscode.window.showErrorMessage('Certificate generation failed');
        }
    });

    // Register live verification
    let liveVerifyCmd = vscode.commands.registerCommand('zeus.liveVerify', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active Zeus file');
            return;
        }
        
        const filePath = editor.document.fileName;
        vscode.window.showInformationMessage('Starting live verification...');
        
        // Run verification on save
        const disposable = vscode.workspace.onDidSaveTextDocument(async (doc) => {
            if (doc.fileName === filePath) {
                try {
                    await execAsync(`zeus verify "${filePath}"`);
                    vscode.window.showInformationMessage('✓ Verification passed', { timeout: 2000 });
                } catch (error) {
                    vscode.window.showWarningMessage('✗ Verification failed', { timeout: 5000 });
                }
            }
        });
        
        context.subscriptions.push(disposable);
    });

    // Add certificate status to status bar
    let updateCertStatus = async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return;
        
        const filePath = editor.document.fileName;
        const baseName = path.basename(filePath, '.zs');
        const certPath = path.join(path.dirname(filePath), baseName + '.zcert');
        
        try {
            await execAsync(`test -f "${certPath}"`);
            statusBar.text = "$(shield) Zeus ✓";
            statusBar.tooltip = "Certificate verified";
        } catch {
            statusBar.text = "$(shield) Zeus";
            statusBar.tooltip = "No certificate";
        }
    };
    
    // Update status on file change
    vscode.window.onDidChangeActiveTextEditor(updateCertStatus);
    updateCertStatus();

    context.subscriptions.push(
        buildCmd, verifyCmd, runCmd, formatter, 
        proofVizCmd, certCmd, liveVerifyCmd, statusBar
    );
}

export function deactivate() {
    console.log('Zeus extension deactivated');
}
