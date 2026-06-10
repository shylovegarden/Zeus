# Zeus: Concept to Reality — The Master Roadmap

*The prioritized plan for taking Zeus from where the compiler actually is today to
where it credibly could go. Every item is tagged and grounded in the real source
under `zeus_compiler/src`. Read `ZEUS_REAL_STATE.md` first for the verified status
table; this document turns that status into a build plan.*

**Version:** 0.1.0 · **Last grounded against source:** 2026-06-08

---

## Legend

| Tag | Meaning |
|---|---|
| **[DONE]** | Implemented and verified in the current compiler source. |
| **[NEXT]** | Highest-value near-term work. Small-to-medium effort, no research risk. |
| **[FUTURE]** | Real and buildable, but larger or dependent on [NEXT] items. |
| **[ASPIRATIONAL]** | Vision / marketing framing. Either research-grade or not started; do **not** claim it works. |

Effort notation: **S** ≈ days, **M** ≈ 1–3 weeks, **L** ≈ 1–3 months, **XL** ≈ quarters / research.

---

## What Zeus actually is (one paragraph, honest)

Zeus is a **source-to-source systems language**: you write `.zs`, the Rust compiler
emits **readable, freestanding C**, and `gcc`/`clang` produces an ordinary
user-space ELF. The emitted C is deliberately constrained — **zero heap**,
**side-channel-hardenable**, **optionally machine-checked by Z3**. Generated
binaries are normal OS processes (they call `mmap`, `fork`, `open`); Zeus is **not**
an OS, does **not** bypass the kernel, and does **not** run bare-metal-without-an-OS
today. The defensible, genuinely-novel core is the *bundle*: a hard zero-heap
guarantee + automatic `secret` wipe + opt-in oblivious memory + invisible
AoS→SoA vectorization + real Z3 contracts, all emitting auditable C.

---

## 0. Cross-cutting priorities (do these first)

1. **[NEXT · S] Pass `-O2`/`-O3` to the C compiler by default in `zeus build`.**
   Today `main.rs` invokes the C compiler with **no `-O` flag** (verified), so the
   SoA/`ivdep`/alignment work — the headline perf story — does nothing in a real
   `zeus build`. A one-to-three-line change unlocks the benefit the benchmarks
   already prove at `-O3`. Add `--release`/`--debug` to control it.
2. **[NEXT · S] Honest banner text.** The build prints "Native Clang Compilation"
   even when gcc is used, and the help text advertises bare-metal/GPU/WASM targets
   that are not wired up. Align user-facing strings with reality.
3. **[NEXT · S] CI that compiles every `tests/cases/*.zs` and the `bench/` suite,
   asserts exit codes + golden output, and runs the zero-heap enforcer on the
   emitted C.** Prevents regressions as the surface grows.

---

## 1. Language core

| Item | Status | Effort | Notes |
|---|---|---|---|
| Lex / parse / analyze / codegen pipeline | **[DONE]** | — | `main.rs`, full pipeline to C. |
| `let` / `let mut`, lexical scoping | **[DONE]** | — | Child scopes inherit `secret` bindings; no leak-out (`codegen.rs`, `oram.rs`). |
| `if` / `else`, `while`, `for i in a..b` | **[DONE]** | — | `parser.rs` `parse_for_statement` / `parse_parallel_block`. |
| Functions, recursion, `pub` | **[DONE]** | — | Emitted as C functions with correct return types. |
| Operator precedence (Pratt) | **[DONE]** | — | Verified correct precedence/associativity. |
| Structs + field access | **[DONE]** | — | Plus the SoA decomposition (§6). |
| Strings, `print`/`println` | **[DONE]** | — | |
| `secret`, `proof`, `enclave`, `parallel`, `target`, `cluster` blocks | **[DONE]** (parse+codegen) | — | Semantics vary; see §7/§8 for which are real vs no-op. |
| Compile-time array sizing `T[N]` (literal N) | **[DONE]** | — | N must be a literal today. |
| **Pattern matching / `match`, enums (sum types)** | **[NEXT]** | M | The single biggest expressiveness gap for a systems language. |
| **`break` / `continue`, `else if` chains as first-class** | **[NEXT]** | S | Common, currently awkward. |
| **Const generics / non-literal array sizes** | **[FUTURE]** | M | Needed for real library code. |
| **Closures / first-class functions** | **[FUTURE]** | L | Hard to reconcile with zero-heap; restrict to non-escaping. |
| **Modules / namespaces / real `import`** | **[FUTURE]** | M | `import` currently inlines AST (`codegen.rs` comment). |
| **Macros / `comptime` execution** | **[ASPIRATIONAL]** | XL | A `comptime` VM exists in skeleton (`src/comptime`, `src/vm`); real const-eval is a project. |

---

## 2. Type system

| Item | Status | Effort | Notes |
|---|---|---|---|
| Primitive types `i8..i64`/`u8..u64`/`bool`/`f32`/`f64` flow to C | **[DONE]** | — | `analyze_type`, `type_to_c`. |
| Struct types | **[DONE]** | — | |
| Basic type-directed codegen (e.g. `@atomic_add` targets → `int64_t`) | **[DONE]** | — | `collect_atomic_int_vars`. |
| **Real bidirectional type checking / inference** | **[NEXT]** | L | Today analysis is light; many errors surface only at the C stage. This is the credibility gate for a "safe" language. |
| **Sum types / generics / traits (bounded polymorphism)** | **[FUTURE]** | XL | Depends on §1 enums; keep monomorphized to preserve zero-heap + readable C. |
| **Ownership / borrow-style aliasing checks** | **[FUTURE]** | XL | Even a lightweight escape/lifetime analysis would materially raise the safety story (and pairs with FFI escape checks already present). |
| **Refinement types feeding Z3** | **[ASPIRATIONAL]** | XL | Natural synergy with §9; research-grade. |

---

## 3. Standard library

| Item | Status | Effort | Notes |
|---|---|---|---|
| `std/zeus/io.zs`, `std/zeus/hw.zs` (thin) | **[DONE]** | — | Minimal; mostly print/hw stubs. |
| Emitted C runtime preamble (arena, secure-wipe, oblivious read/write, fork-join) | **[DONE]** | — | The de-facto "runtime" lives in `codegen.rs` as emitted C. |
| **Core collections that respect zero-heap** (fixed/bounded `Vec`, ring buffer, bounded map) | **[NEXT]** | M | Arena- or capacity-backed; no hidden `malloc`. The library a real user needs first. |
| **`Option`/`Result` + ergonomic error propagation** | **[NEXT]** | M | Depends on enums (§1). Pairs with the existing FFI-escape/error tests. |
| **Math / fixed-point / SIMD-friendly helpers** | **[FUTURE]** | M | Lean on the SoA layout; expose vector intrinsics behind safe wrappers. |
| **String library beyond print** (bounded, no-alloc) | **[FUTURE]** | M | |
| **Embedded HAL (GPIO/UART/SPI) abstractions** | **[FUTURE]** | L | The real wedge for the embedded story (§11); currently `hw.zs` is a stub. |

---

## 4. Tooling

| Tool | Status | Effort | Notes |
|---|---|---|---|
| CLI (`build`/`run`/`test`/`fmt`/`doc`/`verify`/`lsp`/`init`) | **[DONE]** | — | `main.rs`. Real subcommands. |
| Formatter (`fmt`) | **[DONE]** | — | `formatter.rs` (~386 LoC), real. |
| Test runner (`test fn` blocks) | **[DONE]** (basic) | — | Executes `TestDeclaration` proof/asserts; no rich reporting/fixtures yet. |
| MISRA-C / safety audit trace (`doc`) | **[DONE]** (basic) | — | Emits a MISRA compliance report (`main.rs` ~611). Real but shallow. |
| LSP daemon | **[DONE]** (minimal) | — | `lsp.rs` (~168 LoC): `initialize`, didOpen/didChange, **publishDiagnostics only**. |
| VS Code extension scaffold | **[DONE]** | — | `vscode-zeus/`. |
| **LSP: completion, hover, go-to-def, signature help** | **[NEXT]** | M | Diagnostics-only today; these are the features developers actually feel. |
| **Test runner: assertions, fixtures, pass/fail summary, CI exit codes** | **[NEXT]** | S–M | Turn the existing block into a usable harness. |
| **Package manager / dependency resolution** | **[FUTURE]** | L | `init` scaffolds a project; there is **no** registry/resolver/lockfile yet. |
| **Debugger integration (DWARF passthrough / DAP)** | **[FUTURE]** | M | Since output is C→ELF, map `#line` directives (already emitted) to source for gdb/lldb; a DAP shim is then tractable. |
| **`strike` (clean/format/optimize) as a real, documented pass** | **[FUTURE]** | S | Currently a power-command; define exact behavior. |

---

## 5. Memory & safety

| Item | Status | Effort | Notes |
|---|---|---|---|
| **Zero-heap enforcer** | **[DONE]** | — | `enforcer.rs` scans **AST and emitted C text**; aborts build on `malloc(`/`calloc(`/`free(`/`<pthread.h>`/`<stdlib.h>`. This is real and load-bearing. |
| Static arena (64 MB mmap, atomic bump, OOM panic) | **[DONE]** | — | `codegen.rs` ~293–318. This *is* the no-malloc mechanism. |
| Phoenix mark/reset (scoped arena reclamation) | **[DONE]** | — | `__phoenix_mark` save/restore of `zeus_arena_offset`. |
| Secure wipe of `secret` at scope exit | **[DONE]** | — | `volatile` byte loop + `asm volatile(...:::"memory")`; survives `-O3`. |
| FFI-escape checks | **[DONE]** (basic) | — | `ffi_escape_test.zs` exercises it. |
| **Configurable arena size / multiple named arenas** | **[NEXT]** | S | 64 MB is hard-coded; embedded targets need control. |
| **Bounds checking on `T[N]` indexing** | **[NEXT]** | M | Today `p[i]` trusts the index. Optional checked mode (or Z3-proved-safe) is core to the safety pitch. |
| **Stack-overflow protection / large-array placement** | **[NEXT]** | S | SoA arrays land on the stack today (the benchmark hit this at 32 MB). Emit large arrays as `static`/arena-backed automatically. |
| **Escape/lifetime analysis (lightweight)** | **[FUTURE]** | L | See §2; complements FFI checks. |

---

## 6. Concurrency & parallelism

| Item | Status | Effort | Notes |
|---|---|---|---|
| **`parallel (i in a..b)` fork-join** | **[DONE]** | — | Forks N = core-count workers over a chunked range; shared/reduction vars in `MAP_SHARED` arena, copied back after `waitpid`. **Reductions are correct.** `codegen.rs` ~779. |
| `@atomic_add` reductions | **[DONE]** | — | Targets typed `int64_t` so `__atomic_fetch_add` compiles cleanly. |
| Auto SoA + 32-byte alignment + `#pragma GCC ivdep` | **[DONE]** | — | `T[N]` → per-field `aligned(32)` arrays; verified ~9x over naive AoS at `-O3` (see `bench/RESULTS.md`). |
| **Persistent fork worker pool (fork once, reuse across blocks)** | **[NEXT]** | M | Today every `parallel` block forks fresh — fork+teardown overhead per block. See the worker-pool patch spec (separate deliverable) for the exact codegen change: fork N workers at the first block, dispatch subsequent blocks via a generation counter in the `MAP_SHARED` arena. |
| **`parallel` over non-range data / nested parallelism** | **[FUTURE]** | M | Generalize the dispatch beyond a single index range. |
| **Threads / shared-memory threading model** | **[FUTURE]** | L | Currently *process* fork-join (chosen partly because pthreads is enforcer-banned). A safe in-process model needs design that doesn't reopen the heap. |
| M:N fiber scheduler / "10 ns task switch" | **[ASPIRATIONAL]** | XL | The old `ucontext` fiber model was **removed** (it `_exit()`'d and lost results); the manifesto's nanosecond-switch claims are vision, not code. |

---

## 7. Security

| Item | Status | Effort | Notes |
|---|---|---|---|
| `secret` keyword + auto wipe | **[DONE]** | — | §5. |
| **Opt-in oblivious memory for `secret` arrays** | **[DONE]** | — | Element access compiles to a **branchless full O(n) scan** with masked select, so the access pattern is independent of the secret index. Non-secret arrays stay direct/fast. `oram.rs`, `__zeus_oread_bytes`/`__zeus_owrite_bytes`. Showcased in `showcase/sbox_secure.zs`. |
| Constant-time intent (no secret-dependent branches in hardened paths) | **[DONE]** (for the oblivious path) | — | |
| **Documented threat model + a leakage test in CI** | **[NEXT]** | S–M | Measure access-pattern invariance under the oblivious path; pin it in CI so a refactor can't silently break the property. |
| **`enclave` hardware backing (SGX / SEV / TrustZone)** | **[ASPIRATIONAL]** | XL | Today `enclave` emits only a compiler memory barrier; the source says "no hardware enclave on this target." Real attestation is a research/integration effort. |
| **Side-channel hardening beyond memory (timing of arithmetic, etc.)** | **[FUTURE]** | L | Constant-time multiply/select primitives, verified. |
| iO / garbled circuits | **[ASPIRATIONAL]** | XL | The "garbled circuit" routine is a labeled **simulation** (XOR with `__rdtsc`). iO does not exist in any usable form, here or in the literature, at practical speed. **Never claim this works.** |

---

## 8. Hardware / kernel-bypass surface (honesty section)

These exist as named blocks that emit honest no-ops or thin wrappers. They are
**not** what their names suggest, and the source generally says so.

| Item | Status | Reality |
|---|---|---|
| NVMe "direct" access | **[ASPIRATIONAL]** (thin) | Emits a real `open(O_DIRECT)` + `mmap` — but there is **no** kernel-bypass polling driver. It's an mmap, not SPDK. |
| RDMA / Infiniband / `cluster` | **[ASPIRATIONAL]** | `cluster` runs the block **in-process**; no distributed transport is built. |
| IOMMU / VFIO DMA firewall | **[ASPIRATIONAL]** | Comment-only function body; no DMA isolation is configured. |
| "Micro-AI" / adaptive `@adaptive` | **[ASPIRATIONAL]** | A **static linear score** over fixed weights — source labels it "NOT an ML model." `--tune` swaps in mock weights. Does not learn. |
| OS replacement / bare-metal / "runs everywhere" | **[ASPIRATIONAL]** | Output is a normal user-space process using syscalls. Real `none-eabi`/`riscv-none` bring-up (no libc, crt0, linker script) is genuine work (§10). |

**Recommendation:** keep these blocks (they're useful syntax placeholders) but
gate the marketing. The honest, fundable story does not need them.

---

## 9. Verification

| Item | Status | Effort | Notes |
|---|---|---|---|
| **`@verify` / `assert` via real Z3** | **[DONE]** | — | Constant comparisons folded at compile time; symbolic ones emitted as SMT and run through the real `z3` binary. `unsat` = proven, `sat` = counterexample printed, cached in `.zeus_verify_cache`. Falls back to a runtime check if `z3` is absent. `formal_verifier.rs`. |
| `proof { }` blocks (compile-time, elided at runtime) | **[DONE]** | — | |
| MISRA-C report (`doc`) | **[DONE]** (basic) | — | §4. |
| `--medical` verification mode | **[DONE]** (flag) | — | Present in CLI; deepen the rule set. |
| **Richer contract language (preconditions/postconditions, loop invariants)** | **[NEXT]** | M | Today contracts are predicate asserts. Invariants would let Z3 prove loops, not just straight-line code. |
| **Prove bounds-safety of `T[N]` indexing with Z3** | **[NEXT]** | M | Ties §5 bounds-checking to §9 — "verified zero-heap, bounds-proved C" is a sharp claim. |
| **Memory-safety / no-UB obligations discharged to Z3** | **[FUTURE]** | L | |
| **Full functional verification (Frama-C/CBMC-class)** | **[ASPIRATIONAL]** | XL | Long-horizon; partner with existing tools rather than rebuild. |

---

## 10. Backends / targets

| Target | Status | Effort | Notes |
|---|---|---|---|
| **C → gcc/clang → x86-64 Linux ELF** | **[DONE]** | — | The real, working backend. |
| `--target=<triple>` passthrough to the C compiler | **[DONE]** (passthrough) | — | Forwards `-target`; only works where the C toolchain + sysroot exist. |
| MLIR emission (`--mlir`) | **[DONE]** (skeleton) | M to finish | `mlir_codegen.rs` (~165 LoC) emits a middle-end sketch; not a real lowering pipeline yet. |
| **Bare-metal `armv7a-none-eabi` / `riscv64-none-elf` (no libc)** | **[FUTURE]** | L | The advertised targets need crt0, linker scripts, and a libc-free runtime. The zero-heap arena is already a great fit; this is the credible path to the "embedded" story. |
| **WASM (`wasm32`)** | **[FUTURE]** | M | Emit C and use Emscripten/clang-wasm; mostly runtime-shimming work. |
| **GPU (`nvptx64`)** | **[ASPIRATIONAL]** | XL | Advertised but unwired; real GPU lowering is a major effort. |
| **eBPF backend** | **[ASPIRATIONAL]** | XL | Attractive (verified, bounded, no-heap maps to eBPF's constraints well) but research-grade. |
| **FPGA / HLS** | **[ASPIRATIONAL]** | XL | Vision only. |

---

## 11. Ecosystem & adoption

| Item | Status | Effort | Notes |
|---|---|---|---|
| README / MANIFESTO / honest status docs | **[DONE]** | — | `README.md`, `ZEUS_REAL_STATE.md`, `HONEST_IMPLEMENTATION_PLAN.md`. |
| Example programs / showcases | **[DONE]** | — | `showcase/`, `benchmarks/`, `tests/cases/`. |
| **This benchmark suite (`bench/`) with reproducible numbers** | **[DONE]** | — | SoA ~9x, arena ~3.4x, honestly caveated. |
| **Tutorial + "why Zeus" landing doc aimed at embedded/security devs** | **[NEXT]** | S | Lead with the real wedge, not the aspirational blocks. |
| **Package registry + community libraries** | **[FUTURE]** | L | Depends on §1 modules + §4 package manager. |
| **Certification story (DO-178C / IEC 62304 / ISO 26262 evidence generation)** | **[FUTURE]** | L | Extend `doc`/MISRA into real audit artifacts — this is monetizable in the embedded/medical space. |

---

## 12. What makes Zeus genuinely novel (the defensible wedge)

Strip away the aspirational hardware/OS framing and a sharp, real, *differentiated*
product remains:

> **A safety- and security-focused systems language that emits auditable,
> verified, zero-heap C.**

The novelty is the **combination in one toolchain that outputs plain C**:

1. **Hard zero-heap guarantee** — the build is *killed* if `malloc`/`free`/`pthread`
   ever appear in source or emitted C. Not a lint; a gate. (`enforcer.rs`)
2. **Automatic `secret` lifecycle** — scope-exit wipe that survives `-O3`, plus
   **opt-in oblivious (constant-time) memory** for secret arrays. (`codegen.rs`, `oram.rs`)
3. **Invisible AoS->SoA vectorization** — write natural structs, get aligned
   Structure-of-Arrays C for free (~9x measured vs naive AoS at `-O3`).
4. **Real Z3-backed contracts** — `@verify`/`assert` that *prove* properties or
   hand back a counterexample. (`formal_verifier.rs`)
5. **Readable, freestanding C output** — auditable by existing certification and
   review processes; drops into any C toolchain.

No mainstream language ships *all five* and emits inspectable C. Rust gives memory
safety but not zero-heap-by-construction, not oblivious memory, not built-in Z3
contracts, and not C output for audit. SPARK/Ada gives verification but not the
security primitives or the SoA ergonomics. That gap is the wedge.

### The honest defensible pitch

**"Write high-level systems code; ship verified, zero-heap, side-channel-aware C
that passes a MISRA audit — targeted at embedded, automotive, medical, and
security-critical software."**

That is true *today* (modulo turning on `-O2` by default), demonstrable with the
benchmarks, and large enough to matter. The OS/bare-metal/GPU/iO framing should be
explicitly held back as long-term vision so it never undercuts the credibility of
the parts that already work.

---

## 13. Suggested execution order (the first two quarters)

**Q1 — make the real story bulletproof:**
1. `-O2/-O3` by default + `--release` (§0.1) — *the* highest-ROI change.
2. CI: compile+run every example/test/bench, enforce zero-heap on output (§0.3).
3. Honest user-facing strings (§0.2).
4. Bounds checking (optional/Z3-proved) + auto static/arena placement for large arrays (§5).
5. Persistent fork worker pool (§6 / patch spec).

**Q2 — close the expressiveness + safety gaps:**
6. `match` / enums / `Option`/`Result` (§1, §3).
7. Real type checking pass (§2).
8. Bounded zero-heap collections (§3).
9. LSP completion/hover/go-to-def (§4).
10. Contract invariants + Z3-proved indexing safety (§9).

Everything in §8 and the XL items in §7/§10 stay clearly labeled **[ASPIRATIONAL]**
until someone funds the research.
