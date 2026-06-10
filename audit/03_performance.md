# Zeus Performance Audit (03) — Compiler + Generated Code

Auditor scope: performance of the `zeus` toolchain (the Rust compiler) and the
quality/speed of the C it emits. Read-only on sources; all numbers below were
measured by running the release binary (`/tmp/zeus_target/release/zeus_compiler`)
on generated test programs. Host: x86_64 Linux, C backend = **gcc** (clang is not
installed, so `resolve_cc()` falls back to gcc — see main.rs:37-50).

---

## Executive summary

**Is it fast enough to be usable?** For the compiler's *own* work, yes — and then
some. Zeus's front-end and analysis passes (lexer, parser, ORAM, semantic
analysis, formal verifier, ZIR, bounds) are essentially free: **under 3 ms total
even for a 1000-function program**. Wall-clock build time is **~99% the C
compiler** (gcc) invoked on the emitted `.c`. So "compile speed" is really "gcc
speed" plus a fixed ~12.5 KB C prelude that gcc must re-parse every build.

But there are three real problems, two of them serious:

1. **A pathological O(?) memory blowup in the parser** triggered by a trivially
   common program: any function named `f32`, `f64`, `i32`, `i8`, `u64`, or
   `bool`. The lexer tokenizes those identifiers as *type keywords*, so
   `fn f32(...)` is malformed; the parser's error path fails to consume the bad
   token, `parse_program` spins pushing `LineDirective` statements, and the
   process tries a **5.6 GB single allocation and is OOM-killed**. One such
   function aborts the whole build. Severity: **High** — it's a correctness *and*
   resource bug hit by ordinary code (a math lib full of `f32`/`f64` helpers
   would never compile).

2. **`zeus build` ships `-O2`, not `-O3`** — and the headline SoA/vectorization
   feature only pays off at `-O3` with gcc. Measured: the SoA throughput kernel
   runs **35 ms at the -O2 Zeus uses vs 11 ms at -O3 (3.2x)**, because at -O2 gcc
   emits **zero** vector instructions for the loop; at -O3 it emits AVX
   (`vaddpd`/`vmovupd`). The SoA transform itself is genuine and excellent (SoA
   -O3 is ~7x faster than naive AoS -O3), but the default flag leaves most of that
   on the table. Severity: **High** (it silently undercuts the project's central
   performance claim).

3. **Every emitted program carries a ~250-line, ~12.5 KB fixed C prelude** (fiber
   scheduler, Chase-Lev work-stealing deque, W^X JIT supervisor, ORAM helpers,
   IOMMU/RDMA/garbled-circuit stubs) **plus an `__attribute__((constructor))` that
   `mmap`s a 64 MB arena + 3 more mappings at startup** — even for hello-world,
   which uses none of it. The C compiler dead-code-eliminates the unused `static
   inline` helpers (binary is only +3.9% vs hand C), but the constructor and its 4
   startup syscalls always survive and run. Severity: **Medium**.

**Biggest wins (in order):** fix the type-keyword parser OOM (tiny effort, huge
payoff); switch the default to `-O3` or add `-O3` for vectorizable kernels (tiny
effort, ~3x on the flagship benchmark); feature-gate the C prelude so trivial
programs don't drag 64 MB of arena + a constructor they never touch.

---

## Measured numbers

### Compile-time scaling (function count), SAFE function names (no keyword collision)

| Program            | Front-end (lex+parse+ORAM+sema+verify+ZIR+bounds) | gcc (`-O2 -march=native`) | Total wall | Peak RSS |
|--------------------|---------------------------------------------------|---------------------------|------------|----------|
| 10 functions       | tokenize 36 µs                                    | ~245 ms                   | 0.24 s     | 82 MB    |
| 100 functions      | tokenize 165 µs                                   | 299 ms                    | 0.30 s     | 85 MB    |
| 1000 functions     | tokenize 1571 µs; sema 435 µs; verify 189 µs; ZIR 1001 fns | 962 ms           | 0.97 s     | 116 MB   |
| 2000 stmts / 1 fn  | tokenize 1410 µs                                   | 264 ms                    | 0.27 s     | 82 MB    |

Tokenizing scales 36 → 165 → 1571 µs across 10 → 100 → 1000 functions: **clean
linear** (≈10x per 10x). All Zeus-internal passes combined stay <3 ms at N=1000.
gcc dominates total wall time (~99%). **The compiler's own algorithms are
linear and fast.** The C compiler is the bottleneck, and the 12.5 KB prelude it
must re-parse on every build is a fixed tax (~245 ms floor even for hello-world).

### The parser OOM (type-keyword-named functions) — High

| Test program                                  | Result                          |
|-----------------------------------------------|---------------------------------|
| 28 functions `func$i`, body `x+1`             | OK, 82 MB, 59 µs                |
| 1 function named `f99`                         | OK                              |
| **1 function named `f32`**                     | **Abort — "memory allocation of 5,637,144,576 bytes failed"** |
| 32 functions named `f$i` (f1..f32)             | OOM-killed (>6 GB), dies before "Tokenizing" prints |
| 100 functions named `func$i`                   | OK (proves it is the *name*, not the count) |

Deterministic. The trigger is purely the function *name* colliding with a type
keyword; the constant and the count are irrelevant.

### Generated-C quality / binary size

| Artifact                          | Size           | Notes                                            |
|-----------------------------------|----------------|--------------------------------------------------|
| hello-world `.c` (1 fn)           | 12,769 bytes   | ~250-line fixed prelude; actual logic = 6 lines  |
| `.c` for 20 functions             | 14,053 bytes   | +1283 B / 20 fns ≈ **64 B per function** (linear) |
| hello-world Zeus binary           | 16,584 bytes   | gcc `-O2 -march=native`                           |
| hand-written C hello, `-O2 -march=native` | 15,968 bytes | Zeus is **+616 B (+3.9%)** — DCE strips unused inlines |
| hello-world runtime RSS           | 1,408 KB       | same as hand C (64 MB arena is lazily faulted)   |
| **startup `mmap`s**               | 64 MB arena + 3 small maps | run unconditionally via `__attribute__((constructor))` |

### Runtime: SoA throughput kernel (`bench/soa_throughput.zs`, 131072 bodies × 512 steps)

| Build                                   | Vector insns | Time / run |
|-----------------------------------------|--------------|------------|
| SoA, **`-O2 -march=native` (what `zeus build` uses)** | **0**    | **35 ms**  |
| SoA, `-O3 -march=native`                | 12 (AVX)     | **11 ms**  |
| naive AoS hand-C, `-O2 -march=native`   | 0            | 75 ms      |
| naive AoS hand-C, `-O3 -march=native`   | 5            | 110 ms     |

SoA -O3 (11 ms) is **~3.2x faster than SoA -O2** and **~7x faster than AoS -O3**.
The SoA transform is real and effective; the `-O2` default is what's holding it
back.

### WASM / cross-compile

`zeus build hello.zs --target=wasm32-unknown-unknown` **fails**:
`gcc: error: unrecognized command-line option '-target'`. Cross-compilation
(including the documented WASM baseline target) depends on clang, but the build
unconditionally falls back to gcc when clang is absent and then passes
clang-only flags. No preflight check. (No `.wasm` size could be measured.)

---

## Findings

### F1 — Type-keyword-named functions cause a multi-GB allocation / OOM-kill — **HIGH**

**Evidence:** A single `fn f32(x: i32) -> i32 { ... }` aborts with
`memory allocation of 5,637,144,576 bytes failed`. 32 such functions get
OOM-killed at >6 GB before the "Tokenizing" line even prints. `f99` and `func$i`
names are fine at any count (100 OK).

**Root cause:**
- `lexer.rs:284-289` maps the identifiers `i8`, `i32`, `u64`, `f32`, `f64`, `bool`
  to *type* tokens (`Token::I32`, `Token::F32`, …), so the lexer never produces an
  `Identifier("f32")`.
- `parser.rs:939-942` (`parse_function_declaration`): after consuming `fn`, the
  name must be `Token::Identifier`; for `Token::F32` it pushes "Expected function
  name" and **`return None` without consuming the bad token**.
- `parser.rs:71-88` (`parse_program`): each loop iteration unconditionally pushes
  a `Statement::LineDirective` (line 78) *before* attempting a statement. When the
  cursor gets stuck on the malformed `fn f32 ( ...`, `advance_after_statement`
  (parser.rs:46-69) cannot make reliable forward progress to EOF, so the loop
  spins, growing `program.statements` until a `Vec` reallocation requests ~5.6 GB.

**Recommended fix:** Either (a) allow type keywords as identifiers in name
position (a contextual-keyword tweak in `parse_function_declaration` /
`parse_let_statement`), or (b) on the "Expected function name" error path, consume
the offending token and bail to a real error report instead of returning `None`
with no advance, and (c) make `parse_program` guard against zero-progress
iterations (if `current_token` is unchanged across an iteration, force-advance or
abort). Any one of (b)/(c) stops the OOM; (a) also makes the language usable for
the obvious `f32`/`f64`-heavy math code. **Effort: ~1–2 hours.**

### F2 — `zeus build` uses `-O2`; the SoA/vectorization feature needs `-O3` — **HIGH**

**Evidence:** Same SoA kernel: -O2 → 0 vector insns, 35 ms; -O3 → 12 AVX insns,
11 ms (3.2x). The benchmark file's own comment claims "-O2 ... vectorizes the
SoA/ivdep hot loops" — that is **false for gcc** (and gcc is the fallback C
compiler). `bench/soa_throughput.zs` even contradicts itself, noting elsewhere
that vectorization "only appears when the emitted .c is recompiled at -O3."

**Root cause:** `main.rs:1081` hard-codes `clang_cmd.arg("-O2")`. The aligned,
unit-stride SoA C that `codegen.rs` works hard to emit (32-byte-aligned field
arrays, straight-line loop body) is exactly what the auto-vectorizer wants, but
gcc's loop vectorizer is off at `-O2`.

**Recommended fix:** Default to `-O3 -march=native` (the codegen is already shaped
for it), or at minimum add `-ftree-vectorize` / `-O3` when the program contains a
lowered `Struct[N]` SoA / parallel hot loop. Guard with a flag if deterministic
codegen matters. **Effort: ~30 min for the default flip; ~half a day if you want
per-kernel opt selection.**

### F3 — Unconditional ~12.5 KB C prelude + 64 MB arena constructor in every program — **MEDIUM**

**Evidence:** hello-world emits a 12,769-byte `.c` whose real content is 6 lines;
the rest is a fixed prelude (`codegen.rs:64+`, all `push_str` with no
feature-gating: fiber scheduler at :269, Chase-Lev deque, W^X JIT at the
`memfd_create` block, ORAM `__zeus_oread/owrite`, IOMMU/RDMA/garbled-circuit
stubs). `__attribute__((constructor)) __zeus_init_shared_memory` (codegen.rs:~304)
`mmap`s a 64 MB `MAP_SHARED|MAP_ANON` arena + 3 more maps. `strace` confirms a
hello-world process issues `mmap(NULL, 67108864, ...)` + 3 small mmaps at startup.
The binary is only +3.9% vs hand C (gcc DCEs the unused inlines), but the
constructor and its 4 syscalls always run, and gcc must re-parse the full 12.5 KB
on **every** build (this is the ~245 ms compile floor).

**Root cause:** No usage analysis gates the prelude. The compiler already computes
exactly what's needed — ZIR's `uses_heap` / `reaches_extern` (zir.rs) and the
presence of parallel/cluster/tensor/ORAM nodes — but codegen emits everything
regardless.

**Recommended fix:** Emit prelude sections on demand. Only emit the fiber
scheduler + deque if a `parallel`/`cluster` block exists; only the ORAM helpers if
an `OramAccess` exists; only the arena constructor if `uses_heap` is true. For
pure scalar programs the prelude collapses to a few includes. This also cuts the
per-build gcc-parse floor. **Effort: ~1 day (mechanical: wrap each `push_str`
block in a `if program_uses_X` and thread the flags from the existing analyses).**

### F4 — ZIR interprocedural-taint fixpoint re-lowers all functions each pass — **MEDIUM**

**Evidence/root cause:** `zir.rs:280-292`. The outer `loop` re-runs
`lower_function` for **every** function on each iteration until the
`returns_secret` set stops growing. Worst case (a chain where each pass discovers
one more secret-returning function) is O(N) passes × O(N) functions × O(body) =
**O(N²·body) re-lowering**. In practice N is small and chains are short (measured
ZIR cost was negligible at N=1000 with no secrets), so it doesn't bite today — but
it's a latent quadratic on adversarial/large secret-propagation graphs.

**Recommended fix:** Lower each function once into ZIR, then run the secret-return
taint as a fixpoint *over the cached SSA / call-graph summaries* instead of
re-lowering source. Standard worklist over def-use. **Effort: ~half a day.**

### F5 — Cross-compile / WASM silently requires clang; no preflight — **MEDIUM (usability)**

**Evidence:** `--target=wasm32-unknown-unknown` fails with gcc's
`unrecognized command-line option '-target'`. **Root cause:** `main.rs:1082-1088`
adds clang-style `-target <triple>` but `resolve_cc()` (main.rs:37-50) may have
returned gcc. There's no check that clang exists before accepting a `--target`.

**Recommended fix:** If a cross target is requested and clang isn't found, fail
fast with a clear message ("cross-compilation requires clang"); or translate to
gcc cross-toolchain invocation where one exists. **Effort: ~1–2 hours.**

### F6 — Avoidable subtree clones in hot AST passes — **LOW / nit**

`oram.rs:159` does `expr.clone()` (clones the whole indexed subtree) only to
repackage `IndexAccess` → `OramAccess`; a `std::mem::replace`/destructure would
avoid the copy. `oram.rs:41` clones the scope `HashSet` per nested block.
`parser.rs` clones `current_token` on essentially every statement
(`prev_token = self.current_token.clone()`). `analyzer.rs:71` clones all struct
fields. None are measurable today (passes are <1 ms at N=1000); listed for
completeness. **Effort: trivial, low value.**

### F7 — `bounds.rs` and `analyzer.rs` and `oram.rs` are clean — *(no finding)*

For the record: `bounds.rs` memoizes WCET per function (`Cost::func`, bounds.rs:135-145)
so it is O(total AST nodes), not quadratic. `analyzer.rs` is a single-pass O(AST)
walk with scoped HashMaps. `oram.rs` is a single-pass O(AST) walk. The reachability
fixpoint in `zir.rs:312-326` iterates over tiny call graphs and does not re-lower.
The previously-fixed lexer `chars().nth` quadratic is confirmed gone — the lexer
now uses `chars: Vec<char>` with O(1) indexed access (lexer.rs:95, 116-131) and
measures linear (36/165/1571 µs at 10/100/1000 functions). No *other* front-end
super-linear pattern was found.

---

## Ranked top-5 optimization opportunities

1. **Fix the type-keyword parser OOM (F1).** A single `f32`-named function aborts
   the build with a 5.6 GB allocation. Make type keywords usable as identifiers in
   name position, and/or guarantee forward progress in `parse_program`. ~1–2 h.
   Highest payoff: turns "won't compile / crashes the box" into "works."

2. **Default to `-O3` (or `-O3` for vectorizable kernels) (F2).** Recovers a
   measured **3.2x** on the flagship SoA benchmark (35 ms → 11 ms) and stops the
   project from silently undershipping its central performance claim. ~30 min.

3. **Feature-gate the C prelude + arena constructor (F3).** Stop emitting the
   fiber scheduler / ORAM / JIT / IOMMU boilerplate and the unconditional 64 MB
   `mmap` constructor for programs that don't use them. Cuts emitted-C size from
   ~12.5 KB to a few hundred bytes for scalar programs, removes 4 startup syscalls,
   and lowers the per-build gcc-parse floor. ~1 day.

4. **De-quadratic the ZIR taint fixpoint (F4).** Lower each function to SSA once
   and iterate the secret-return fixpoint over cached summaries, not source.
   Removes a latent O(N²) on large/adversarial secret-propagation graphs.
   ~half a day.

5. **Preflight cross-compile / WASM toolchain (F5).** Detect that `--target`
   needs clang and fail fast (or use a gcc cross toolchain) instead of emitting a
   confusing `gcc: unrecognized option '-target'`. Makes the advertised WASM /
   bare-metal targets actually reachable. ~1–2 h.

*Method note:* numbers were taken with `/usr/bin/time -v` on generated `.zs`
programs under a `ulimit -v` cap to contain the OOM bug; the SoA kernel was timed
by recompiling the Zeus-emitted `.c` at -O2 vs -O3 and counting AVX instructions
via `objdump`. The C backend in this environment is gcc (clang absent), which is
also the documented fallback in `main.rs`.
