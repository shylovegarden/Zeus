# Zeus

**A source-to-source systems language that compiles `.zs` to readable, zero-heap, side-channel-hardened C — then to a native binary.**

Zeus is a Rust-based compiler. You write `.zs`; Zeus emits constrained C (no heap,
optional oblivious memory, optional Z3-checked assertions) and hands it to
`clang`/`gcc` to build a normal native executable.

Zeus is **not** an operating system and does not bypass the kernel. Generated
programs are ordinary user-space processes. For the full honest breakdown of what
is real vs. aspirational, see **[`ZEUS_REAL_STATE.md`](ZEUS_REAL_STATE.md)**.

---

## What it does today

- **Compiles `.zs` → C → native binary** via clang/gcc.
- **Zero-heap enforcer** — the build is *aborted* if the generated C contains `malloc`, `calloc`, `free`, `<pthread.h>`, or `<stdlib.h>`. All memory comes from a static `mmap`-backed arena.
- **`secret` keyword** — variables tagged `secret` get a `volatile` memory wipe plus a compiler memory barrier at scope exit (survives `-O3`).
- **Oblivious memory (opt-in)** — indexing a `secret` array compiles to a branchless, constant-time **full O(n) scan**, so the access pattern doesn't leak the index. Non-secret arrays stay fast and direct.
- **Automatic SoA transform** — arrays of structs are decomposed into per-field arrays, each `aligned(32)` for AVX2 auto-vectorization; field accesses are rewritten automatically.
- **Multi-core `parallel` blocks** — fork-join across all cores using `fork()` + a `MAP_SHARED` arena; reductions are aggregated correctly after join.
- **`@verify` / `assert` with Z3** — constant assertions are folded at compile time; symbolic ones are proved by the real `z3` solver (a counterexample is printed on failure). Falls back to a runtime check if `z3` isn't installed.
- **Typed locals** — `i32`, `u64`, `bool`, `f64` annotations flow through to C; correct operator precedence and lexical scoping.

## Build

The compiler lives in `zeus_compiler/`. Build and run it with Cargo:

```bash
cd zeus_compiler
cargo build --release
# the compiler is currently driven via cargo:
cargo run -- build ../hello_world.zs
```

Requirements: a Rust toolchain, plus `clang` or `gcc` on `PATH`. Install `z3` to
enable static formal verification (optional — without it, assertions become
runtime checks).

## Compile a `.zs` file

```bash
cd zeus_compiler
cargo run -- build  ../hello_world.zs   # emit hello_world.c/.h and build a native binary
cargo run -- run    ../hello_world.zs   # build and execute
cargo run -- verify ../test_verify_z3.zs  # run Z3 verification only
cargo run -- test   ../hello_world.zs   # run `test fn` blocks
```

The compiler writes `<name>.c` and `<name>.h` next to the input and produces the
native binary `<name>`.

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

## Roadmap (not yet built)

The following appear in the vision/manifesto but are **stubs, honest no-ops, or
aspirational** today — see `ZEUS_REAL_STATE.md` for specifics:

- OS-level / bare-metal-without-OS execution and kernel bypass.
- Hardware enclaves (SGX/SEV) — `enclave` blocks are currently just compiler barriers.
- `cluster` / RDMA distributed execution — currently runs in-process.
- IOMMU/VFIO DMA firewalling and NVMe kernel-bypass storage.
- GPU / Tensor-Core / AMX / MLIR hardware backends (`--mlir` emits a text dump only).
- M:N fiber scheduler and single-digit-nanosecond task switching (current model is process fork-join).
- Machine-learning auto-tuning (`--tune` uses mock weights; the "micro-AI" is a static heuristic).
- Indistinguishability obfuscation (the "garbled circuit" path is a labeled simulation).
- A full type checker, generics, strings, and a standalone shippable `zeus` binary.

## License / status

Prototype, v0.1.0. Treat the verified core as usable-and-rare; treat the roadmap
items as intent, not capability.
