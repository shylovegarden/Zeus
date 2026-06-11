# ZEUS COMPILER - COMMAND REFERENCE

**Working Directory:** `cd /Users/shy/Developer/ZEUS/zeus_compiler`

---

## 🔨 BUILD COMMANDS

```bash
# Basic build
cargo run --release -- build <file.zs>

# Build with MLIR output
cargo run --release -- build <file.zs> --mlir

# Build with performance tuning
cargo run --release -- build <file.zs> --tune

# Cross-compile
cargo run --release -- build <file.zs> --target=<arch>
```

---

## ▶️ RUN COMMANDS

```bash
# Build and run
cargo run --release -- run <file.zs>

# Run with policy enforcement
cargo run --release -- run <file.zs> --require=zero-heap,constant-time,bounded
```

---

## 🔍 VERIFICATION COMMANDS

```bash
# Verify proofs only
cargo run --release -- verify <file.zs>

# Security audit (human-readable)
cargo run --release -- audit <file.zs>

# Security audit (JSON)
cargo run --release -- audit <file.zs> --json

# Audit LLVM IR (The Lens)
cargo run --release -- audit <file.ll>
```

---

## 📜 CERTIFICATE COMMANDS

```bash
# Generate certificate
cargo run --release -- cert <file.zs>

# Verify certificate
cargo run --release -- verify-cert <file.zcert>

# Verify provenance
cargo run --release -- verify-provenance <file.provenance.json>
```

---

## 🌐 WASM COMMANDS

```bash
# Compile to WebAssembly
cargo run --release -- wasm <file.zs>

# Save to file
cargo run --release -- wasm <file.zs> -o output.wat
```

---

## 🔗 FFI COMMANDS

```bash
# Import C header
cargo run --release -- import <header.h>
```

---

## 🛠️ DEVELOPMENT COMMANDS

```bash
# Format code
cargo run --release -- fmt <file.zs>

# Run tests
cargo run --release -- test <file.zs>

# Generate docs
cargo run --release -- doc <file.zs>

# Start LSP
cargo run --release -- lsp

# Init project
cargo run --release -- init <project_name>
```

---

## 🧪 TESTING COMMANDS

```bash
# Unit tests
cargo test

# Smoke tests (37 tests)
ZEUS_BIN=/Users/shy/Developer/ZEUS/zeus_compiler/target/release/zeus_compiler \
  bash ../tests/smoke_test.sh

# Feature tests
ZEUS_BIN=/Users/shy/Developer/ZEUS/zeus_compiler/target/release/zeus_compiler \
  bash ../tests/feature/run_feature_tests.sh

# Fuzz tests
ZEUS_BIN=/Users/shy/Developer/ZEUS/zeus_compiler/target/release/zeus_compiler \
  python3 ../tests/fuzz_analyses.py 100
```

---

## 📋 QUICK EXAMPLES

### Hello World
```bash
echo 'pub fn main() { println(42); }' > /tmp/hello.zs
cargo run --release -- run /tmp/hello.zs
```

### With Verification
```bash
echo '@verify fn add(a:i32,b:i32)->i32 { 
  proof{assert(a+b>=a);} 
  return a+b; 
}
pub fn main() { println(add(5,3)); }' > /tmp/proof.zs
cargo run --release -- run /tmp/proof.zs
```

### Constant-Time Crypto
```bash
echo '@constant_time fn secure(secret k:i32,m:i32)->i32{return k+m;}
pub fn main(){let secret key=42; println(secure(key,100));}' > /tmp/ct.zs
cargo run --release -- run /tmp/ct.zs
```

### Parallel Processing
```bash
echo 'pub fn main() { parallel(i in 0..8){println(i*i);} }' > /tmp/par.zs
cargo run --release -- run /tmp/par.zs
```

### Policy Enforcement
```bash
cargo run --release -- run /tmp/ct.zs --require=constant-time
```

---

## 🎯 COMMON WORKFLOWS

### 1. Quick Test
```bash
cargo run --release -- run <file.zs>
```

### 2. Production Build
```bash
cargo run --release -- build <file.zs> --tune
./<binary>
```

### 3. Security Audit
```bash
cargo run --release -- audit <file.zs> --json > audit.json
```

### 4. Verify Then Run
```bash
cargo run --release -- verify <file.zs>
cargo run --release -- run <file.zs> --require=zero-heap,constant-time
```

### 5. Cross-Platform
```bash
cargo run --release -- wasm <file.zs> -o app.wat
```

---

## 📁 OUTPUT FILES

- `<name>` - Native executable
- `<name>.c` - Generated C source
- `<name>.h` - C header
- `<name>.zcert` - Ed25519 signed certificate
- `<name>.provenance.json` - SLSA provenance
- `zeus_safety_report.txt` - Security analysis

---

## 🔑 ENVIRONMENT VARIABLES

- `ZEUS_KEY_DIR` - Directory for signing keys (~/.zeus)
- `ZEUS_SIGNING_KEY` - Override signing key (32-byte hex)
- `ZEUS_TRUSTED_PUB` - Trusted public key for verification

---

## 📊 POLICY PROPERTIES

Use with `--require=<properties>`:

- `zero-heap` - No dynamic allocation
- `constant-time` - No timing leaks
- `bounded` - Provable WCET
- `reproducible` - Deterministic execution
