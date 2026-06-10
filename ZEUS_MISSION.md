# Zeus — Mission (the one-page north star)

> Anchor doc. Every build should map to one of the two edges below. If it doesn't, question it.
> Honest by rule: we say "machine-checked under a stated trusted base," never "unbreakable."

## The core, in one line
**Zeus is a high-assurance security platform for people who can't accept "it probably works" — defense, government, critical industry.** It has two edges from a single proof engine (the ZIR dataflow analysis):

### The Shield — build code that's *provably hard to exploit*
Write code; the compiler proves and certifies it is:
- **memory-safe** (zero-heap → no buffer-overflow/use-after-free exploit class),
- **timing-safe** (`@constant_time` → no side-channel leak),
- **bounded** (`@wcet` / `@stack` → no runaway / DoS),
- **reproducible** (no hidden nondeterminism).
Output: a single auditable C-derived binary + a certificate (`.zcert`), and a `zeus run --require` / `zeus.policy` gate that refuses to run anything that doesn't prove what you demand. **[REAL today.]**

### The Lens — audit *other* code and prove where it's unsafe
Point the same engine outward. `zeus audit <file>` runs the taint / leak / bounds passes and reports timing channels, unbounded paths, and secret leaks — and, uniquely, can say **"proved absent"** on bounded leak-free code, not just "no pattern matched." Authorized/defensive use (find bugs to fix them). **[Lens MVP REAL for Zeus source today; foreign-code ingestion is the next keystone.]**

## Honest scope (so we never get caught overclaiming)
- We did **not** invent new math. We package proven techniques (taint analysis, WCET, proof-carrying code) into one usable toolchain. That's a product/experience leap, not a science one — and that's enough.
- "Understands all languages" = analyzes whatever lowers to **C / LLVM-IR / WASM**, not magic.
- "Hard to attack" = provably fewer exploit classes, **not** unhackable. Anything a CPU runs can be observed.
- WCET is a sound bound in abstract **steps**, not nanoseconds yet. The certificate is content-**hashed**, not yet **signed**.
- Trusted base: the Zeus compiler + Z3 + the C compiler are trusted and unverified. The verified-compiler tier above us is **CompCert / Jasmin** — we are a high-assurance *tool*, not a verified compiler. Source-level constant-time can be undone by `-O2` (binary-level validation is a later step).

## Who it serves
Crypto engineers (no timing leaks), embedded/defense/aerospace (provable WCET/stack = the audit evidence they already pay for), blockchain (determinism + bounded execution), platform/CI teams gating AI-written code.

## Legacy & versatility — "supports existing systems with ease"
- Zeus → C: `extern fn` calls into existing C. **Works.**
- C → Zeus: `@ffi_export` exposes Zeus to C. **Works for simple signatures.**
- `zeus import <header.h>` generates bindings. **Works (pragmatic).**
- The deeper "understands legacy" story = ingest LLVM-IR/WASM into ZIR so the Lens can analyze compiled C/C++/Rust. **Next keystone, not done.**

## The 3 next moves (from grounded research)
1. **ZIR v2 — add a control-flow graph + inter-procedural call graph behind a `Lowerer` trait, then write an LLVM-IR front-end.** One investment strengthens the Shield (sound cross-function analysis) AND unlocks the Lens for C/C++/Rust at once (via `clang -emit-llvm`). *(Large, keystone.)*
2. **Cryptographically sign the `.zcert`** (Ed25519) + a reproducible-build manifest. Cheapest path from "compiler" to "defense-procurable." *(Small.)*
3. **Grow `zeus audit`** into a real analyzer: declarable sources/sinks, SARIF output, and honest PROVED-SAFE / LEAK-FOUND / UNDECIDABLE labels (lead with "proved absent" — the thing CodeQL/Semgrep can't say). *(Medium.)*

Everything else (Cranelift/LLVM backends, full C-source front-end, DO-178C evidence packs, binary-level constant-time) is real and valuable but *follows* these three.
