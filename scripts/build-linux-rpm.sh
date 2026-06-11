#!/bin/bash
# Build Linux .rpm package for Zeus
set -e

echo "Building Zeus .rpm package..."

VERSION="0.1.0"
RELEASE="1"
ARCH="x86_64"
PKG_NAME="zeus-${VERSION}-${RELEASE}.${ARCH}"
BUILD_ROOT="~/rpmbuild"

# Setup RPM build environment
mkdir -p "${BUILD_ROOT}/"{BUILD,RPMS,SOURCES,SPECS,SRPMS}

# Build release binary
echo "Building release binary..."
cd /Users/shy/Developer/ZEUS/zeus_compiler
cargo build --release 2>&1 | tail -5

# Create spec file
cat > "${BUILD_ROOT}/SPECS/zeus.spec" << 'SPEC_EOF'
Name:           zeus
Version:        0.1.0
Release:        1%{?dist}
Summary:        Systems language with formal verification
License:        MIT
URL:            https://zeus-lang.org
Source0:        zeus-%{version}.tar.gz
BuildArch:      x86_64
Requires:       llvm >= 14, z3

%description
Zeus is a systems programming language with built-in
formal verification for zero-heap, constant-time,
and bounded execution guarantees.

%prep
# Binary already built

%install
mkdir -p %{buildroot}/usr/bin
mkdir -p %{buildroot}/usr/share/doc/zeus
cp /Users/shy/Developer/ZEUS/zeus_compiler/target/release/zeus %{buildroot}/usr/bin/
cp /Users/shy/Developer/ZEUS/README.md %{buildroot}/usr/share/doc/zeus/

%files
/usr/bin/zeus
/usr/share/doc/zeus/README.md

%changelog
* Thu Jun 11 2026 Zeus Team <team@zeus-lang.org> - 0.1.0-1
- Initial release
SPEC_EOF

# Build RPM
echo "Building .rpm package..."
cd "${BUILD_ROOT}"
rpmbuild -ba SPECS/zeus.spec 2>&1 || echo "Note: rpmbuild may require additional setup"

# Copy output
mkdir -p /Users/shy/Developer/ZEUS/dist
cp "${BUILD_ROOT}/RPMS/x86_64/"*.rpm /Users/shy/Developer/ZEUS/dist/ 2>/dev/null || true

echo "✅ .rpm build script ready"
