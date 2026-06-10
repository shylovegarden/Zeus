# Zeus Benchmark Results

**Machine:** Intel(R) Core(TM) i9-10885H @ 2.40GHz (AVX2), Linux
**Compiler:** `gcc (Ubuntu 11.4.0-1ubuntu1~22.04.3) 11.4.0`
**Flags:** `-O3 -march=native -fno-math-errno`
**Date captured:** 2026-06-08
**Zeus compiler:** `/tmp/zeus_target/release/zeus_compiler` (v0.1.0)

---

## THE HONEST CAVEAT — read this first

Zeus is a **source-to-source compiler**: `.zs -> readable C -> gcc -> ELF`. Every
number below is **gcc's performance on the C that Zeus emits**, not the result of
a bespoke Zeus optimizer or backend. Zeus's contribution to performance is purely
the *shape of the C it emits* (Structure-of-Arrays layout, 32-byte alignment,
`#pragma GCC ivdep`), which lets gcc's existing auto-vectorizer do more.

Two more things to be honest about:

1. **`zeus build` itself compiles the emitted C at `-O0`** (it invokes the C
   compiler with no `-O` flag — see `main.rs`). So a binary produced by
   `zeus build` today is **not** vectorized. To measure the layout benefit, the
   harness recompiles the *emitted* `.c` at `-O3 -march=native` — the **same**
   flags applied to the naive C counterpart. The comparison is therefore strictly
   "same compiler, same flags, **SoA layout vs AoS layout**." Making `zeus build`
   pass `-O2/-O3` by default is a one-line change tracked in the roadmap.
2. The SoA win is a **data-layout** win, not magic. Any C programmer who hand-wrote
   SoA would get the same result. Zeus's value is that you write the natural
   AoS-style `struct` + `array[N]` and get the SoA layout *for free, invisibly*.

---

## Benchmark 1 — SoA throughput (Zeus-emitted SoA vs naive AoS C)

Tight unit-stride loop integrating position by velocity over `N = 131,072` bodies
for `STEPS = 512` (≈ 67.1M element-iterations), 2 doubles updated per element.

- `soa_throughput.zs` — Zeus source. `let bodies = Body[N]` is lowered to four
  separate `double bodies_x[N] __attribute__((aligned(32)))` arrays
  (verified in the emitted `soa_throughput.c`).
- `soa_naive.c` — the same algorithm written the natural C way: one
  `Body bodies[N]` array of structs (interleaved fields, 32-byte stride).

Both binaries print `1537` (= 1 + 512·3), confirming the loop does real,
identical work and was not optimized away.

| Variant            | best wall | ns / element |
|--------------------|-----------|--------------|
| Zeus SoA (`-O3`)   | **12 ms** | **0.179**    |
| Naive AoS (`-O3`)  | 111 ms    | 1.654        |
| **Speedup (AoS/SoA)** | **~9.2x** |          |

(best of 7 runs each; re-runs were stable: SoA 12–13 ms, AoS 111–127 ms.)

**Interpretation.** The AoS loop strides 32 bytes between consecutive `.x`
values, so each cache line drags along `.y/.vx/.vy` the inner update path
touches only partially, and the vectorizer can't cleanly pack 4 contiguous
doubles per AVX2 lane. The SoA loop is unit-stride and 32-byte aligned, so gcc
emits packed AVX2 loads/stores. ~9x is consistent with a memory-layout-bound
kernel going from gather-ish AoS to clean vectorized SoA on this CPU. This is a
real, reproducible layout effect — not an inflated claim.

---

## Benchmark 2 — Arena bump-allocator vs malloc/free

`arena_vs_malloc.c` reproduces the exact generated `__zeus_arena_alloc` pattern
(align-up + bump an offset in a pre-mmap'd region) and times it against
`malloc`+`free`, 5,000,000 allocations of 32 bytes, best of 5 trials.

| Allocator        | best total | ns / alloc |
|------------------|------------|------------|
| Zeus arena bump  | 11.99 ms   | **2.398**  |
| malloc + free    | 41.25 ms   | 8.250      |
| **Speedup (malloc/arena)** | **~3.4x** |     |

**Interpretation.** A bump allocator is a couple of instructions (align, add,
return pointer); glibc malloc maintains size classes, free lists, and arenas.
The ~3.4x gap is expected and honest. The trade Zeus makes: **no individual
`free`** — memory is reclaimed by resetting the whole arena (the Phoenix
mark/reset pattern in codegen). That is the standard arena/region trade-off and
is exactly why Zeus can emit **zero `malloc`** in user output while still
supporting dynamic-looking allocation. This bench measures allocation cost only,
not fragmentation behavior (where arenas also win for the workloads Zeus targets).

---

## Reproduce

```bash
cd bench
bash run_bench.sh        # builds + runs everything, prints ns/element and ratios
```

Override the compiler if needed: `CC=clang bash run_bench.sh`.
Numbers will vary by CPU; the *ratios* are the portable signal.
