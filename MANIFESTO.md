# The Zeus Manifesto
*Vision: the language that runs everywhere, from your browser to bare metal.*

> **How to read this document.** Zeus has a big, deliberate North Star. It also has
> a real, working core that is much narrower than that North Star. To stay honest,
> this manifesto is split into two clearly labeled parts:
>
> - **§ Vision / North Star** — where we *want* Zeus to go. Aspirational. Not built.
> - **§ Implemented Today** — what the compiler *actually* does, verified against source.
>
> For a no-spin status report, see **[`ZEUS_REAL_STATE.md`](ZEUS_REAL_STATE.md)**.

---

# Part I — Vision / North Star (Aspirational)

*Everything in Part I is a goal, not a current capability. Treat it as a roadmap.*

To dominate the next decades of computing, a language must solve real bottlenecks
across the whole software lifecycle — from the lab to physical hardware. Here is
the blueprint of why we believe Zeus can win.

### 1. For AI Scientists (The Lab) — *aspirational*
Kill the two-language problem. The dream: a scientist writes `matmul(A, B)` and an
MLIR middle-end maps it straight onto the silicon — Tensor-Core PTX on NVIDIA, AMX
on Apple Silicon, AVX-512 on x86 — with `zeus build --tune` auto-searching
micro-variants on the real chip. *(Today: `--mlir` emits a small text dump; `--tune`
uses mock weights; there is no GPU/Tensor backend.)*

### 2. For Bare-Metal & Automotive Engineers (The Factory) — *partly real*
Write simple, readable code; let the compiler invisibly choose cache-optimal
layouts and forbid dynamic memory. *(The SoA transform and the Zero-Heap Enforcer in
this bullet are **real today** — see Part II. "Runs on bare metal without an OS" is
**not** — generated code is a normal user-space process.)*

### 3. For Security & Defense (The Vault) — *partly real*
- **`proof {}` blocks** that mathematically prove properties before compilation. *(Real today via Z3.)*
- **The `secret` keyword** that wipes sensitive data from RAM at scope exit. *(Real today.)*
- **Oblivious memory** so access patterns don't leak secrets. *(Real today, opt-in, for `secret` arrays.)*
- Hermetic dependency sandboxing, and one day hardware enclaves (SGX/SEV) and
  indistinguishability obfuscation. *(Enclaves are compiler-barrier no-ops today; iO does not exist.)*

### 4. For Developers (The Trenches) — *partly real*
One binary for build/format/test/verify/LSP, zero config, and a `comptime` bytecode
VM. *(The CLI subcommands exist and are driven via `cargo run --` today; the
comptime VM and single shipped `zeus` binary are still maturing.)*

### 5. The Grand Systems Vision — *aspirational*
Out-of-band sentinel cores that snoop cycles via `__rdtsc` and vaporize runaway
fibers; an M:N fiber scheduler with single-digit-nanosecond task switching;
kernel-bypass I/O (NVMe polling, RDMA `cluster`); IOMMU DMA firewalls; photonic and
quantum targets via `.zeus_arch` blueprints; a Zeus-built OS handling sensor buses
in place at line rate.

**Status of Part I, §5: none of this is the way Zeus works today.** Parallelism is
process **fork-join**, not fibers. `cluster` runs in-process. NVMe is an `mmap`, not a
polling driver. IOMMU is a comment. Enclaves are barriers. These are the things we
are reaching for — and we will say plainly when each one becomes real.

---

# Part II — Implemented Today (Verified Against Source)

*Everything in Part II was checked against `zeus_compiler/src`. This is what Zeus
genuinely is right now: a **source-to-source compiler** that turns `.zs` into
constrained, readable C and then a native binary.*

The combination below — emitting auditable C — is the real, and genuinely rare,
heart of the project:

- **Zero-Heap Enforcer.** The build is aborted if the generated C contains
  `malloc`/`calloc`/`free`/`<pthread.h>`/`<stdlib.h>`. All memory comes from a
  static, `mmap`-backed bump arena. *(`enforcer.rs`, `codegen.rs`)*
- **`secret` wipe.** A `volatile` byte-clear plus an `__asm__ volatile(... "memory")`
  barrier is emitted at scope exit, surviving `-O3`. *(`codegen.rs`)*
- **Oblivious memory (opt-in).** Indexing a `secret` array compiles to a branchless,
  constant-time **full O(n) scan** with a masked select, so the memory access pattern
  is independent of the secret index. Non-secret arrays remain direct and fast.
  *(`oram.rs`, `codegen.rs`)*
- **Invisible SoA transform.** Arrays of structs become per-field arrays, each
  `__attribute__((aligned(32)))` for AVX2; field accesses are auto-rewritten.
  *(`codegen.rs`)*
- **Multi-core `parallel` (fork-join).** Forks N = core-count workers over a chunked
  index range using a `MAP_SHARED` arena; reductions aggregate correctly after
  `waitpid`. *(`codegen.rs`)*
- **`@verify` / `assert` via real Z3.** Constant assertions fold at compile time;
  symbolic ones are emitted as SMT and solved by the `z3` binary — `unsat` proves the
  property, `sat` prints a counterexample, results cache to `.zeus_verify_cache`.
  Without `z3`, a runtime check is injected instead. *(`formal_verifier.rs`)*
- **Typed locals, precedence, scoping.** `i32`/`u64`/`bool`/`f64` flow to C; correct
  operator precedence; lexical scoping that inherits `secret` bindings without leaking
  declarations. *(`parser.rs`, `codegen.rs`, `oram.rs`)*

## The honest wedge

Zeus today is best understood as a **friendlier front end that emits verified,
side-channel-hardened, zero-heap C** for safety-critical, embedded, and
security-sensitive work. That is a real, defensible niche. The OS-level and
hardware-bypass ambitions in Part I are the direction of travel — labeled as such so
no one is misled about what runs today.

*We would rather under-promise in the manifesto and over-deliver in the compiler than
the reverse.*
