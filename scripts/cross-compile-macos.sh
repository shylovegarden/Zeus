#!/bin/bash
# Cross-compile for macOS (Intel + Apple Silicon)
set -e

echo "Cross-compiling Zeus for macOS..."

cd /Users/shy/Developer/ZEUS/zeus_compiler

# Install cross tool if needed
cargo install cross 2>/dev/null || true

# Build for Intel (x86_64)
echo "Building for macOS Intel (x86_64)..."
cargo build --release --target x86_64-apple-darwin 2>&1 | tail -5 || echo "Note: Cross-compilation requires proper toolchain"

# Build for Apple Silicon (ARM64)
echo "Building for macOS Apple Silicon (aarch64)..."
cargo build --release --target aarch64-apple-darwin 2>&1 | tail -5 || echo "Note: Cross-compilation requires proper toolchain"

# Create universal binary (if both succeed)
if [ -f "target/x86_64-apple-darwin/release/zeus" ] && [ -f "target/aarch64-apple-darwin/release/zeus" ]; then
    echo "Creating universal binary..."
    mkdir -p target/universal-apple-darwin/release
    lipo -create \
        target/x86_64-apple-darwin/release/zeus \
        target/aarch64-apple-darwin/release/zeus \
        -output target/universal-apple-darwin/release/zeus
    echo "✅ Universal binary created"
fi

# Package
mkdir -p /Users/shy/Developer/ZEUS/dist
if [ -f "target/universal-apple-darwin/release/zeus" ]; then
    tar -czf /Users/shy/Developer/ZEUS/dist/zeus-v0.1.0-macos-universal.tar.gz \
        -C target/universal-apple-darwin/release zeus
    echo "✅ macOS universal tarball created"
else
    # Package individual architectures
    for arch in x86_64 aarch64; do
        if [ -f "target/${arch}-apple-darwin/release/zeus" ]; then
            tar -czf "/Users/shy/Developer/ZEUS/dist/zeus-v0.1.0-macos-${arch}.tar.gz" \
                -C "target/${arch}-apple-darwin/release" zeus
            echo "✅ macOS ${arch} tarball created"
        fi
    done
fi

echo "macOS cross-compile script complete"
