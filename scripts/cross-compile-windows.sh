#!/bin/bash
# Cross-compile for Windows
set -e

echo "Cross-compiling Zeus for Windows..."

cd /Users/shy/Developer/ZEUS/zeus_compiler

# Install cross tool if needed
cargo install cross 2>/dev/null || true

# Build for Windows x64
echo "Building for Windows x64..."
cargo build --release --target x86_64-pc-windows-msvc 2>&1 | tail -5 || echo "Note: Windows cross-compile requires MSVC toolchain"

# Alternative: Use MinGW
echo "Trying MinGW target..."
cargo build --release --target x86_64-pc-windows-gnu 2>&1 | tail -5 || echo "Note: MinGW may require: rustup target add x86_64-pc-windows-gnu"

# Package
mkdir -p /Users/shy/Developer/ZEUS/dist

for target in x86_64-pc-windows-msvc x86_64-pc-windows-gnu; do
    if [ -f "target/${target}/release/zeus.exe" ]; then
        zip -j "/Users/shy/Developer/ZEUS/dist/zeus-v0.1.0-windows-${target}.zip" \
            "target/${target}/release/zeus.exe"
        echo "✅ Windows ${target} zip created"
    fi
done

echo "Windows cross-compile script complete"
echo "Note: Windows builds typically require:"
echo "  - MSVC target: Visual Studio build tools"
echo "  - GNU target: MinGW-w64"
