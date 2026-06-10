# Zeus Security Showcase — Access-Pattern-Oblivious Secret Table Lookup

This showcase demonstrates Zeus's headline security property: a **secret-indexed
table lookup whose memory access pattern does not depend on the secret index** —
the classic AES S-box / cache-timing attack target — done correctly *by the
compiler*, not by hand.

## Files

| File              | What it is                                                            |
|-------------------|----------------------------------------------------------------------|
| `sbox_secure.zs`  | Zeus program: a `secret` 256-entry table with secret-index lookups.  |
| `sbox_naive.c`    | Plain C doing the ordinary, vulnerable `table[secret_index]` lookup. |
| `compare.sh`      | Builds both, shows identical results, and proves the access patterns differ. |
| `README.md`       | This file.                                                           |

## Quick start

```sh
# Uses /tmp/zeus_target/release/zeus_compiler by default; override with ZEUS=...
cd showcase
./compare.sh
```

If your Zeus compiler shells out to `clang` and only `gcc` is installed,
`compare.sh` transparently shims `clang -> gcc` on `PATH` for its run.

## The threat: cache-timing on secret table lookups

Block ciphers like AES are commonly implemented with lookup tables (S-boxes)
indexed by **key-dependent** bytes: `S[key_byte ^ state_byte]`. A normal lookup
`table[k]` computes an address from `k` and dereferences it. The CPU then pulls
the corresponding **cache line** into L1/L2.

An attacker sharing the same physical core (a co-tenant VM, another process, or
even JavaScript in another tab) cannot read the table's *contents*, but can
observe **which cache lines were touched** using techniques like:

- **Prime+Probe** — fill the cache, let the victim run, measure which lines got
  evicted.
- **Flush+Reload** — flush a shared line, let the victim run, time how fast it
  reloads.

Because the line touched depends on `k`, the timing leaks `k` — and `k` is
derived from the secret key. This is a real, published class of attacks against
naive AES implementations.

`sbox_naive.c` is exactly this vulnerable pattern:

```c
int32_t v1 = table[k1];   /* address depends on secret k1 -> cache leak */
```

## What Zeus does

In `sbox_secure.zs`, the table is declared `secret`:

```zeus
let secret sbox = Entry[256];
...
let v1 = sbox[k1].val;   // compiles to an OBLIVIOUS full scan
```

The `secret` keyword changes how the Zeus compiler lowers array indexing. Instead
of emitting `table[k]`, it emits a call to a constant-time, branchless primitive
that **touches every entry in the table** and uses a bit-mask to select the one
at the secret index. From the compiler's emitted C (`sbox_secure.c`):

```c
static inline void __zeus_oread_bytes(void* dst, const void* base,
                                      size_t n, size_t esz, size_t idx) {
    unsigned char* d = (unsigned char*)dst;
    const unsigned char* b = (const unsigned char*)base;
    for (size_t j = 0; j < esz; j++) d[j] = 0;
    for (size_t k = 0; k < n; k++) {                 // <-- scans ALL n entries
        unsigned char m = (unsigned char)0 - (unsigned char)(k == idx);  // 0x00 or 0xFF
        const unsigned char* e = b + k * esz;
        for (size_t j = 0; j < esz; j++)
            d[j] |= (unsigned char)(e[j] & m);       // branchless masked select
    }
}
```

And each secret lookup in `main` lowers to:

```c
int32_t v1 = (int32_t)(({ int32_t _zo;
    __zeus_oread_bytes(&_zo, sbox_val, (size_t)(256), sizeof(sbox_val[0]), (size_t)(k1));
    _zo; }));
```

Key properties of this lowering:

- **Every index reads the same 256 locations**, in the same order, regardless of
  `k`. The cache footprint is identical for `k=42`, `k=200`, `k=7`, or any other
  index — so Prime+Probe / Flush+Reload learn nothing about `k`.
- **No secret-dependent branch.** The selector `m = 0 - (k == idx)` is `0x00` or
  `0xFF` and is applied with `&`/`|`, never an `if`, so there is no
  branch-predictor leak either.
- Writes use the symmetric `__zeus_owrite_bytes`, which also scans all entries.

## Measured cost

The protection is **O(n) per access** instead of O(1). Concretely, for this
256-entry table of 4-byte (`i32`) entries:

| Operation                | Naive C            | Zeus `secret`                      |
|--------------------------|--------------------|------------------------------------|
| Locations touched / read | 1 entry (4 bytes)  | **256 entries (1024 bytes)**       |
| Inner work / read        | 1 load             | 256 iterations × 4-byte mask/merge |
| Asymptotic cost          | O(1)               | **O(n)** (n = table length)        |
| Branches on secret       | address-dependent  | **none** (branchless mask)         |

So the obliviousness costs roughly a **256× factor** in memory work per access
for this table size (the full-scan factor equals the table length). This is the
standard, expected price of access-pattern obliviousness for a linear-scan
implementation — you pay O(n) to make the pattern independent of the index. It is
opt-in: only arrays you mark `secret` pay it.

`compare.sh` quantifies this directly from the emitted code (table length `256`
and the number of oblivious reads/writes it found).

## What is and isn't protected — honest scope

**Protected (demonstrated here):**

- **Memory access pattern of secret-indexed table reads/writes.** The set and
  order of addresses touched is independent of the secret index, defeating
  cache-line-granularity access-pattern attacks (Prime+Probe, Flush+Reload) on
  *that table*.
- **Secret-dependent branching on the index** for these accesses — there is none;
  selection is branchless.

**NOT protected / out of scope (no overclaiming):**

- **This is not a proof of constant-time at the hardware level.** The C is
  written to be branchless and full-scan, but a sufficiently aggressive compiler
  or microarchitecture (e.g. data-dependent vectorization, hardware prefetchers,
  data memory-dependent timing on some CPUs) could in principle reintroduce
  timing variation. Use a hardened backend / DIT-mode CPU for hard guarantees.
- **It does not hide the table length or that a lookup happens** — only *which*
  entry within a fixed-size secret table.
- **It does not protect non-`secret` data**, control flow elsewhere in your
  program, power/EM side channels, speculative-execution attacks (Spectre), or
  the secrecy of values you `println`.
- **It is not encryption and not "unbreakable."** It removes one specific,
  well-known leak (the data-dependent access pattern of a table lookup). Other
  side channels and the rest of your program remain your responsibility.

**Bottom line:** Zeus turns a notoriously easy-to-get-wrong, hand-written
constant-time table lookup into a one-keyword (`secret`) compiler guarantee, with
identical results and a clear, quantifiable O(n) cost — and nothing more is
claimed.

## Reproducing the proof

Running `./compare.sh` shows, against the **real compiler output**:

1. Both programs print `49 / 131 / 60` — identical results.
2. `sbox_naive.c` contains direct `table[k1]` / `table[k2]` / `table[k3]` accesses.
3. Zeus's emitted `sbox_secure.c` contains `__zeus_oread_bytes(...)` calls for
   each of those lookups, plus the full-scan primitive itself — proving the
   access-pattern hiding is actually present in the compiled binary, not just a
   source-level annotation.
