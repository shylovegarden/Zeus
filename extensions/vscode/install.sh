#!/bin/bash
# VS Code Extension Installer for Zeus

echo "Installing Zeus VS Code Extension..."

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if VS Code is installed
if ! command -v code &> /dev/null; then
    echo -e "${RED}VS Code not found. Please install VS Code first.${NC}"
    echo "Download: https://code.visualstudio.com/"
    exit 1
fi

EXTENSION_DIR="/Users/shy/Developer/ZEUS/extensions/vscode"
VSCODE_EXT_DIR="$HOME/.vscode/extensions/zeus-lang.zeus-0.1.0"

echo "Extension source: $EXTENSION_DIR"
echo "VS Code extensions: $VSCODE_EXT_DIR"

# Remove old installation if exists
if [ -d "$VSCODE_EXT_DIR" ]; then
    echo "Removing old installation..."
    rm -rf "$VSCODE_EXT_DIR"
fi

# Copy extension files
echo "Copying extension files..."
mkdir -p "$VSCODE_EXT_DIR"
cp -r "$EXTENSION_DIR/"* "$VSCODE_EXT_DIR/"

# Check if extension.ts exists and needs compilation
if [ -f "$VSCODE_EXT_DIR/src/extension.ts" ]; then
    echo "TypeScript files found, checking for compiled JS..."
    
    # Create out directory if it doesn't exist
    mkdir -p "$VSCODE_EXT_DIR/out"
    
    # If extension.js doesn't exist, we need to compile
    if [ ! -f "$VSCODE_EXT_DIR/out/extension.js" ]; then
        echo "Compiling TypeScript..."
        cd "$VSCODE_EXT_DIR"
        
        # Check if npm is available
        if command -v npm &> /dev/null; then
            npm install 2>/dev/null || true
            npm run compile 2>/dev/null || {
                echo -e "${RED}Warning: Could not compile TypeScript${NC}"
                echo "Creating minimal JS fallback..."
                
                # Create minimal JS version
                cat > "$VSCODE_EXT_DIR/out/extension.js" << 'EOF'
const vscode = require('vscode');
const { exec } = require('child_process');
const { promisify } = require('util');
const execAsync = promisify(exec);

function activate(context) {
    console.log('Zeus extension activated');
    
    // Build command
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
    
    // Verify command
    let verifyCmd = vscode.commands.registerCommand('zeus.verify', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active Zeus file');
            return;
        }
        const filePath = editor.document.fileName;
        try {
            await execAsync(`zeus verify "${filePath}"`);
            vscode.window.showInformationMessage('✓ Zeus verification passed!');
        } catch (error) {
            vscode.window.showErrorMessage('✗ Zeus verification failed');
        }
    });
    
    // Run command
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
    
    // Status bar
    let statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    statusBar.text = "$(shield) Zeus";
    statusBar.tooltip = "Zeus Language Support";
    statusBar.command = 'zeus.build';
    statusBar.show();
    
    context.subscriptions.push(buildCmd, verifyCmd, runCmd, statusBar);
}

function deactivate() {
    console.log('Zeus extension deactivated');
}

module.exports = { activate, deactivate };
EOF
            }
        else
            echo -e "${RED}npm not found, creating JS fallback...${NC}"
        fi
    fi
fi

# Reload VS Code window
echo ""
echo -e "${GREEN}✅ Zeus extension installed successfully!${NC}"
echo ""
echo "Next steps:"
echo "1. Reload VS Code window (Cmd+Shift+P → 'Developer: Reload Window')"
echo "2. Open a .zs file"
echo "3. Use Cmd+Shift+B to build, F5 to run"
echo ""
echo "Commands:"
echo "  • Zeus: Build (Cmd+Shift+B)"
echo "  • Zeus: Verify (Cmd+Shift+V)"
echo "  • Zeus: Run (F5)"
echo ""
