#!/bin/bash
# Build Linux .deb package for Zeus
set -e

echo "Building Zeus .deb package..."

VERSION="0.1.0"
ARCH="amd64"
PKG_NAME="zeus_${VERSION}_${ARCH}"
BUILD_DIR="/tmp/${PKG_NAME}"

# Clean previous build
rm -rf "${BUILD_DIR}"
mkdir -p "${BUILD_DIR}/DEBIAN"
mkdir -p "${BUILD_DIR}/usr/bin"
mkdir -p "${BUILD_DIR}/usr/share/zeus"
mkdir -p "${BUILD_DIR}/usr/share/doc/zeus"

# Build release binary
echo "Building release binary..."
cd /Users/shy/Developer/ZEUS/zeus_compiler
cargo build --release 2>&1 | tail -5

# Copy binary
cp target/release/zeus "${BUILD_DIR}/usr/bin/"
chmod +x "${BUILD_DIR}/usr/bin/zeus"

# Copy docs
cp README.md "${BUILD_DIR}/usr/share/doc/zeus/"
cp LICENSE* "${BUILD_DIR}/usr/share/doc/zeus/" 2>/dev/null || true

# Create control file
cat > "${BUILD_DIR}/DEBIAN/control" << CONTROL_EOF
Package: zeus
Version: ${VERSION}
Section: devel
Priority: optional
Architecture: ${ARCH}
Depends: llvm-14, libz3-dev
Maintainer: Zeus Team <team@zeus-lang.org>
Description: Systems language with formal verification
 Zeus is a systems programming language with built-in
 formal verification for zero-heap, constant-time,
 and bounded execution guarantees.
CONTROL_EOF

# Build package
echo "Building .deb package..."
dpkg-deb --build "${BUILD_DIR}"

# Move to output
mkdir -p /Users/shy/Developer/ZEUS/dist
cp "/tmp/${PKG_NAME}.deb" /Users/shy/Developer/ZEUS/dist/

echo "✅ .deb package created: dist/${PKG_NAME}.deb"
echo "Install with: sudo dpkg -i dist/${PKG_NAME}.deb"
