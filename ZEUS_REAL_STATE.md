# Zeus — Real State (Honest Status Report)

*For a technical evaluator. Every claim below was checked against the compiler source in `zeus_compiler/src`. Version 0.1.0.*

---

## What Zeus actually is

Zeus is a **source-to-source systems language**. You write `.zs`, and the Zeus
compiler (written in Rust) emits **readable, freestanding C** which is then handed
to `clang`/`gcc` to produce a normal native binary. The emitted C is deliberately
constrained: **zero heap**, **side-channel-hardened**, and **optionally machine-checked**.

The genuinely novel part is the *combination* in one toolchain that emits plain C:

- a hard **zero-heap** guarantee (the build is killed if `malloc`/`free`/`pthread` ever appear),
- automatic **`secret` memory wiping** at scope exit,
- **opt-in oblivious memory** (constant-time full-scan) for `secret` arrays,
- automatic **Array-of-Structs → Struct-of-Arrays** transform with AVX alignment, and
- real **Z3-backed `@verify`/`assert`** that proves properties or returns a counterexample.

That bundle, emitting auditable C, is rare. It is the defensible core.

It is **not** an operating system, it does **not** bypass the kernel, and it does
**not** run on bare metal without an OS today. Generated binaries are ordinary
user-space processes that call `mmap`, `fork`, and `open` — i.e. they use the OS.

---

## What's real today (verified against source)

| Feature | Reality | Where |
|---|---|---|
| `.zs` → C → native binary | **Real.** Full lex/parse/analyze/codegen, then `clang`/`gcc`. | `main.rs`, `codegen.rs` |
| Zero-heap enforcer | **Real.** Scans both the AST and the generated C text; aborts the build on `malloc(`, `calloc(`, `free(`, `<pthread.h>`, `<stdlib.h>`. | `enforcer.rs` |
| `secret` memory wipe | **Real.** Emits a `volatile` byte loop plus an `__asm__ volatile(... "memory")` barrier at scope exit, so the wipe survives `-O3`. | `codegen.rs::generate_secure_wipe` |
| Oblivious memory for `secret` arrays | **Real, opt-in.** A `secret` array's element access compiles to a branchless **full O(n) scan** with a masked select, so the access pattern is independent of the secret index. Non-secret arrays stay direct/fast. | `oram.rs`, `codegen.rs::__zeus_oread_bytes`/`__zeus_owrite_bytes` |
| Auto SoA + AVX alignment | **Real.** `Particle[32]` is decomposed into per-field arrays, each `__attribute__((aligned(32)))` (256-bit AVX2), and `p[i].x` is rewritten to the field array. | `codegen.rs` (SoA pass) |
| Multi-core `parallel` blocks | **Real, fork-join.** Forks *N* = core-count worker processes over a chunked index range; captured/reduction variables live in a `MAP_SHARED` arena and are copied back after `waitpid`. Reductions are correct. | `codegen.rs` (fork-join dispatch) |
| `@verify` / `assert` via Z3 | **Real.** Constant comparisons are folded at compile time; symbolic ones are emitted as SMT and run through the real `z3` binary (`unsat` = proven, `sat` = counterexample printed, results cached in `.zeus_verify_cache`). Falls back to a runtime check if `z3` is absent. | `formal_verifier.rs` |
| Integer type annotations | **Real.** `i32`/`u64`/`bool`/`f64` flow through to the emitted C types. | `parser.rs`, `codegen.rs` |
| Operator precedence | **Real.** Correct Pratt-style precedence. | `parser.rs` |
| Lexical scoping | **Real.** Child scopes inherit `secret` bindings without leaking declarations back out. | `oram.rs`, `codegen.rs` |
| Static arena allocator | **Real.** 64 MB `mmap`-backed arena with an atomic bump pointer and OOM panic — this *is* the "no malloc" mechanism. | `codegen.rs` |

---

## What's stubbed or aspirational (clearly labeled)

These exist as honest placeholders, comments, or compatibility no-ops. They do
**not** do what their names might suggest, and the source generally says so.

- **No OS replacement / no kernel bypass.** Output is a normal process that uses syscalls. The "bare-metal / runs everywhere / replaces the OS" framing is vision, not fact.
- **`enclave` blocks** emit only a compiler memory barrier (`asm volatile("" ::: "memory")`). **No SGX / SEV** hardware enclave is used; the code comment says "no hardware enclave on this target."
- **`cluster` / RDMA / Infiniband** runs the block **in-process**; the distributed RDMA backend is not built. Honest comment in codegen.
- **IOMMU / VFIO segmentation** is a **comment-only** function body — no DMA firewall is configured.
- **NVMe direct access** emits a real `open(O_DIRECT)` + `mmap`, but there is no NVMe polling driver; it's an mmap, not a kernel-bypass storage stack.
- **The "micro-AI" / adaptive heuristic** is a **static linear score** over fixed weights. The source literally labels it "NOT an ML model." It does not learn and is not self-tuning. `--tune` swaps in **mock** weights.
- **iO (indistinguishability obfuscation)** does not exist. The "garbled-circuit" routine is a labeled **simulation** (XOR with an `__rdtsc` value), not cryptographic obfuscation.
- **M:N fiber scheduler / single-digit-nanosecond fiber switching** (from the manifesto and the stale `CURRENT_STATUS.md`) is **not** how parallelism works. The current model is **process fork-join**. The "10 ns task switch" numbers are aspirational.
- **Photonic / quantum / `.zeus_arch` blueprints**, MLIR-to-Tensor-Core PTX, AMX mapping: aspirational. `--mlir` emits a small MLIR-ish text dump, not a tensor-core backend.

---

## Honest positioning (the defensible wedge)

Zeus is most credible as a **safety-critical / embedded / security-sensitive C
generator** — a higher-level front end that emits MISRA-friendly, heap-free,
side-channel-aware C with optional formal checks:

- **Embedded / automotive / aerospace:** zero-heap by construction → no fragmentation, predictable footprint; readable C drops into existing C toolchains and review processes.
- **Security-sensitive code:** `secret` wipe + oblivious access give cold-boot and cache-timing hardening exactly where you ask for it, and nowhere else (you pay the O(n) cost only on `secret` data).
- **Verification-curious teams:** real Z3 proofs on assertions, with counterexamples, at compile time.

It is **not** an OS, a kernel-bypass runtime, a GPU compiler, or a cryptographic
obfuscator. Positioning it that way invites easy disproof; positioning it as
"verified, hardened, zero-heap C from a friendlier language" is honest and strong.

---

## Realistic completion estimate (framed by goal)

- **As a hardened embedded-C generator (the defensible wedge):** ~**70–80% there.** The core passes work end to end on the included tests. Remaining work is breadth: a real type checker beyond annotations, broader language surface (generics, richer control flow, strings), error-message quality, a test suite, and packaging the compiler as a shippable `zeus` binary (today it's `cargo run --`).
- **As the manifesto's vision (OS-level, kernel-bypass, hardware enclaves, RDMA cluster, GPU/Tensor-Core backend, iO, fiber scheduler):** **single-digit percent.** These are largely stubs or honest no-ops and represent multi-year, multi-specialist efforts each.

**Bottom line:** the rare, real core (zero-heap + secret-wipe + oblivious memory +
SoA + Z3, all emitting readable C) works and is worth taking seriously. The
grand-systems claims are a roadmap, not a current property.
