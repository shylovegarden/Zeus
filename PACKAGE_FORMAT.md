# Zeus Package Format

## Overview

Zeus packages are distributed as `.zpkg` files containing source code, metadata, and dependencies. The package manager (`zeus get`, `zeus publish`) handles installation and publishing.

## Package Structure

```
my_package/
├── zeus_pkg.toml    # Package manifest
├── src/              # Source files
│   ├── main.zeus
│   └── lib.zeus
├── tests/            # Test files
│   └── test.zeus
└── README.md         # Documentation
```

## Package Manifest (zeus_pkg.toml)

```toml
[package]
name = "my_package"
version = "0.1.0"
description = "A useful Zeus library"
authors = ["Author Name <email@example.com>"]
zeus_version = ">=0.1.0"

[dependencies]
other_package = "^0.2.0"

[dev-dependencies]
test_package = "^0.1.0"

[lib]
name = "my_lib"
path = "src/lib.zeus"

[[bin]]
name = "my_tool"
path = "src/main.zeus"
```

## Manifest Fields

### [package]
- `name`: Package name (required, kebab-case)
- `version`: SemVer version (required)
- `description`: Short description (optional)
- `authors`: List of author names/emails (optional)
- `zeus_version`: Minimum Zeus compiler version (optional)
- `license`: SPDX license identifier (optional)
- `repository`: Git repository URL (optional)
- `homepage`: Project homepage URL (optional)

### [dependencies]
- Package dependencies with version constraints
- Format: `package_name = "^version"`
- Version constraints: `^1.0.0`, `~1.2.0`, `>=1.0.0`, `=1.0.0`

### [dev-dependencies]
- Development-only dependencies (tests, benchmarks)

### [lib]
- `name`: Library name (optional, defaults to package name)
- `path`: Path to library entry point (optional, defaults to `src/lib.zeus`)

### [[bin]]
- `name`: Binary name (required for each binary)
- `path`: Path to binary entry point (required)

## Version Constraints

- `^1.2.3`: Compatible with >=1.2.3, <2.0.0
- `~1.2.3`: Compatible with >=1.2.3, <1.3.0
- `>=1.2.3`: Minimum version 1.2.3
- `=1.2.3`: Exact version 1.2.3
- `*`: Any version

## Package Registry

The default registry is `https://zeus.pkg.dev`. This can be overridden with the `ZEUS_REGISTRY_URL` environment variable.

## Package Commands

### Install a package
```bash
zeus get username/package
zeus get username/package --version 1.0.0
```

### Publish a package
```bash
zeus publish
zeus publish --path /path/to/package
```

### Search for packages
```bash
zeus search crypto
```

### List installed packages
```bash
zeus list
```

### Remove a package
```bash
zeus remove username/package
```

## Package Installation Location

Packages are installed to `~/.zeus/packages/` by default:
```
~/.zeus/
├── packages/
│   └── username/
│       └── package/
│           ├── zeus_pkg.toml
│           └── src/
└── cache/
```

## Package Naming Convention

Package names should:
- Use kebab-case (lowercase with hyphens)
- Be unique
- Be descriptive
- Avoid reserved names: `std`, `core`, `alloc`

## Package Versioning

Follow Semantic Versioning (SemVer):
- MAJOR: Incompatible API changes
- MINOR: Backwards-compatible functionality additions
- PATCH: Backwards-compatible bug fixes

Example: `1.2.3` (MAJOR.MINOR.PATCH)

## Dependency Resolution

The package manager uses a simple dependency resolver:
1. Collect all dependencies from manifest
2. Resolve version constraints
3. Check for conflicts
4. Download and install in dependency order

## Security

Package integrity is verified using SHA-256 hashes:
- Each package has a checksum in the registry
- Downloads are verified against the checksum
- Tampered packages are rejected

## Future Enhancements

- [ ] Private package registries
- [ ] Package signing and verification
- [ ] Dependency lock file (zeus_pkg.lock)
- [ ] Workspace support (monorepos)
- [ ] Virtual workspace support
- [ ] Package features (optional dependencies)
- [ ] Build scripts
- [ ] Native library support
