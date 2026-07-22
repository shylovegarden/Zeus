# Zeus

**A source-to-source systems language that compiles `.zs` to readable, zero-heap, side-channel-hardened C — then to a native binary.**

Zeus is a Rust-based compiler. You write `.zs`; Zeus emits constrained C (no heap, optional oblivious memory, optional Z3-checked assertions) and hands it to `clang`/`gcc` to build a normal native executable.

Zeus is **not** an operating system and does not bypass the kernel. Generated programs are ordinary user-space processes. For the full honest breakdown of what is real vs. aspirational, see **[`ZEUS_REAL_STATE.md`](ZEUS_REAL_STATE.md)**.

---

## Table of Contents

- [Quick Start](#quick-start)
- [What it does today](#what-it-does-today)
- [Build](#build)
- [Compile a `.zs` file](#compile-a-zs-file)
- [Example](#example)
- [Core Features](#core-features)
- [Project Structure](#project-structure)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License & Status](#license--status)

---

## Quick Start

```bash
# Install Rust (https://rustup.rs/), clang/gcc, and optionally z3
cd zeus_compiler
cargo build --release

# Compile and run a Zeus program
cargo run -- build ../hello_world.zs   # emit .c/.h and native binary
cargo run -- run ../hello_world.zs     # build and execute
```

---

## What it does today

- **Compiles `.zs` → C → native binary** via clang/gcc.
- **Zero-heap enforcer** — the build is *aborted* if the generated C contains `malloc`, `calloc`, `free`, `<pthread.h>`, or `<stdlib.h>`. All memory comes from a static `mmap`-backed arena.
- **`secret` keyword** — variables tagged `secret` get a `volatile` memory wipe plus a compiler memory barrier at scope exit (survives `-O3`).
- **Oblivious memory (opt-in)** — indexing a `secret` array compiles to a branchless, constant-time **full O(n) scan**, so the access pattern doesn't leak the index. Non-secret arrays stay fast.
- **Automatic SoA transform** — arrays of structs are decomposed into per-field arrays, each `aligned(32)` for AVX2 auto-vectorization; field accesses are rewritten automatically.
- **Multi-core `parallel` blocks** — fork-join across all cores using `fork()` + a `MAP_SHARED` arena; reductions are aggregated correctly after join.
- **`@verify` / `assert` with Z3** — constant assertions are folded at compile time; symbolic ones are proved by the real `z3` solver (a counterexample is printed on failure). Falls back to runtime checks without z3.
- **Typed locals** — `i32`, `u64`, `bool`, `f64` annotations flow through to C; correct operator precedence and lexical scoping.
- **Reproducible binaries** — compiler-enforced determinism for consistent build outputs.
- **Formal certificate signing** — Zeus signs proof claims in a `.zcert` (Ed25519) that can be verified independently.

---

## Build

### Requirements

- **Rust** toolchain (install via [rustup.rs](https://rustup.rs/))
- **C compiler** — `clang` or `gcc` on `PATH`
- **Optional**: `z3` theorem prover (for static formal verification; without it, assertions become runtime checks)

### Build the compiler

```bash
cd zeus_compiler
cargo build --release
```

The compiler is driven via `cargo run`.

---

## Compile a `.zs` file

### Commands

```bash
cd zeus_compiler

# Emit hello_world.c/.h and build a native binary
cargo run -- build ../hello_world.zs

# Build and execute in one step
cargo run -- run ../hello_world.zs

# Run Z3 verification only (if z3 is installed)
cargo run -- verify ../test_verify_z3.zs

# Run all test functions in the file
cargo run -- test ../hello_world.zs
```

### Output

The compiler writes:
- `<name>.c` — generated C source (human-readable, debuggable)
- `<name>.h` — generated C header
- `<name>` — native executable (via clang/gcc)
- `<name>.zcert` — signed proof certificate (with `--verify` flag)

---

## Example

```zeus
struct Entry { val: f64 }

pub fn main() {
    // `secret` => wiped at scope exit AND accessed obliviously (constant-time scan)
    let secret sbox = Entry[256];
    let i = 5;
    sbox[i].val = 42.0;          // oblivious write
    let got = sbox[i].val;       // oblivious read

    // typed locals + compile-time proof
    let counter: u64 = 0;
    proof { assert(counter >= 0); }   // proved by Z3 (or runtime-checked if z3 absent)

    // multi-core fork-join
    parallel (k in 0..1000) {
        let x = k * k;
    }
}
```

---

## Core Features

### Zero-Heap Guarantee

All dynamically allocated memory is forbidden. Memory comes from:
- **Stack** (fixed-size locals)
- **Static arena** (mmap-backed, allocated at startup)

This eliminates allocator side-channels and timing leaks.

### Side-Channel Hardening

- **`secret` variables**: automatic volatile wipe at scope exit + memory barriers
- **Oblivious indexing**: constant-time array access for secrets (O(n) full scan, not O(1) indexed)
- **Reproducible builds**: deterministic compilation enables formal verification

### Formal Verification

- **Z3 integration**: symbolic constants are proved at compile time; counterexamples are printed on failure
- **Fallback runtime checks**: assertions become no-ops or runtime panics if z3 is unavailable
- **Proof certificates**: `.zcert` files record which properties are proven; can be independently verified

### Machine-Binding (Simulation)

See [`attest/README.md`](attest/README.md) for details on the simulated **machine attestation** layer:
- Proof → Signed Certificate → Simulated Machine Binding
- Demonstrates policy hooks for future hardware TPM/PUF integration

---

## Project Structure

```
.
├── zeus_compiler/              # Main Zeus compiler (Rust)
│   ├── src/                    # Compiler frontend & code generator
│   ├── Cargo.toml
│   └── README.md
├── cloud/                      # Zeus cloud service (Axum + Tokio)
│   ├── src/                    # REST API, auth, job queue
│   └── Cargo.toml
├── pkg_manager/                # Package manager CLI
│   ├── src/
│   └── Cargo.toml
├── benchmarks/                 # Research benchmarks (Phase 1–4)
│   ├── research_suite.rs       # Foundational validation
│   ├── microbenchmarks.rs      # Performance characterization
│   └── README.md
├── attest/                     # Attestation simulator (shell + crypto)
│   ├── zeus-attested-run.sh    # Machine-binding wrapper
│   └── README.md
├── showcase/                   # Example Zeus programs
│   └── edge_ai/
│       └── mlp_infer.zs        # End-to-end ML inference example
├── ZEUS_REAL_STATE.md          # Honest status of features (real vs. aspirational)
└── README.md                   # This file
```

---

## Roadmap (not yet built)

The following appear in the vision/manifesto but are **stubs, honest no-ops, or aspirational** today — see [`ZEUS_REAL_STATE.md`](ZEUS_REAL_STATE.md) for specifics:

- OS-level / bare-metal execution and kernel bypass
- Hardware enclaves (SGX/SEV) — `enclave` blocks are currently compiler barriers
- `cluster` / RDMA distributed execution — currently runs in-process
- IOMMU/VFIO DMA firewalling and NVMe kernel-bypass storage
- GPU / Tensor-Core / AMX / MLIR hardware backends
- M:N fiber scheduler and single-digit-nanosecond task switching
- Machine-learning auto-tuning for optimization
- Indistinguishability obfuscation
- Full type checker, generics, strings, and standalone `zeus` binary

---

## Contributing

We welcome contributions! Please:

1. **Fork** the repository
2. **Create a feature branch** (`git checkout -b feature/your-feature`)
3. **Commit with clear messages** (describe the feature or fix)
4. **Submit a pull request** with a description of your changes

For major changes, please open an issue first to discuss.

### Development

- **Compiler internals**: see `zeus_compiler/src/` and add test cases in `zeus_compiler/tests/`
- **Cloud service**: see `cloud/src/` for REST API changes
- **Benchmarks**: add new scenarios to `benchmarks/`

---

## License & Status

**Prototype, v0.1.0.** Treat the verified core as usable-and-rare; treat the roadmap items as intent, not capability.

- **License**: See repository license file
- **Stability**: Pre-release; expect breaking changes
- **Feedback**: Open an issue for bugs, feature requests, or clarifications

---

## Resources

- **[ZEUS_REAL_STATE.md](ZEUS_REAL_STATE.md)** — Detailed feature status (real vs. aspirational)
- **[attest/README.md](attest/README.md)** — Machine attestation simulation details
- **[benchmarks/README.md](benchmarks/README.md)** — Research benchmark phases and results
- **[Showcase Examples](showcase/)** — End-to-end example programs

---

**Questions?** Open an issue or reach out on GitHub. 🚀
