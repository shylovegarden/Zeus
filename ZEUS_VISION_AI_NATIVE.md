# Zeus — The AI-Authored, Verified Systems Language

*Status: living strategy doc. Grounded in the current codebase and June 2026 research. Separates what is REAL today, what is NEXT, and what is the honest North Star.*

---

## 1. The one-sentence position

**Zeus is the systems language where an AI writes zero-heap, formally-verified, constant-time code against a machine-friendly form — the verifier is the trust gate that lets humans rely on AI-written software — and it drops into existing C/C++ systems out of the gate.**

Three pillars, none of them magic:
1. **Speed** — compiles to readable C, then native. Real numbers today: SoA hot loops ~9× faster than naive array-of-structs; arena allocation ~3.4× faster than malloc.
2. **Security** — opt-in oblivious memory + secret-wipe + Z3-proven assertions/contracts, all emitted as auditable C.
3. **Interop** — `zeus import engine.h` generates `extern fn` bindings, so Zeus plugs into a legacy C/C++ codebase module-by-module.

---

## 2. Why this is realistic and not hype (the research)

**The security wedge is a respected field — and unoccupied at the "general-purpose + ergonomic" end.**
Constant-time / side-channel-safe languages are real and prestigious: **FaCT** (a constant-time DSL, PLDI'19), **Jasmin** (a low-level language whose compiler is formally verified in Coq), and **HACL\*** (verified C crypto, used in Firefox/Linux). 2025 work even extends this to *speculative* constant-time (Spectre-safe). But every one of these is research-grade, crypto-only, and painful. **Nobody offers constant-time + verification in a general-purpose language with first-class C interop that an AI can write fluently.** That gap is Zeus's opening.

**AI-native languages are a live 2026 frontier.**
Vercel Labs shipped **Zero**, a systems language built so AI agents can read, repair, and ship native programs (<10 KiB binaries, JSON diagnostics). The research framing of an ideal LLM-native language is striking: *"a rich IR no human would want to write, paired with a human-facing projection, with formal verification at every step — programs as a history of semantic transformations, not text files."* That is precisely the "language mostly AI writes, few human inputs" idea — and the missing ingredient that makes it safe is **verification as the gate**. Zeus already has the verification half.

So the futuristic vision is real *if* framed correctly: not "AI writes incomprehensible code," but **"AI writes it, the prover certifies it, a human-readable projection + JSON diagnostics keep it auditable."**

---

## 3. What is REAL today (verified in the codebase)

- Source-to-source compiler: `.zs` → readable C → native binary. 36/36 sample programs build; 17/17 golden tests pass.
- Language: i32/u64/f64/bool/str types, arrays `[T; N]`, structs, functions/recursion, if/else/while/for, `&&`/`||`, unary ops, correct operator precedence, lexical scoping, `print`/`println` of values, a math stdlib (min/max/clamp/abs/sqrt/pow), comptime constant folding.
- Security: `secret` keyword (RAM wipe at scope exit), **opt-in oblivious memory** (constant-time full-scan reads/writes for `secret` arrays — defeats the cache-timing table-lookup leak), zero-heap enforcement (no malloc/free in output — MISRA C:2012 Rule 21.3 by construction).
- Verification: `@verify` and `@requires`/`@ensures` contracts; real **Z3** integration that proves properties and returns counterexamples; auto-generated `zeus_safety_report.txt`.
- Parallelism: multi-core fork-join `parallel` blocks with correct cross-process reductions.
- Tooling: line-numbered diagnostics, **`--json` machine-readable diagnostics** (AI-repair signal), **`zeus import` C-header → bindings**, `-O2 -march=native` real optimization.
- Honest docs: README/MANIFESTO separate vision from implemented; `ZEUS_CONCEPT_TO_REALITY.md` is the master roadmap.

## 4. What is NEXT (the adoption-critical gaps, in order)

1. **Type checker with real errors + parser error recovery** — report many errors, not die on the first; reject mismatches in Zeus, not in C.
2. **Bidirectional C FFI** — not just `extern fn` in; let C call Zeus too (the real "module-by-module migration" story). Harden `zeus import` toward a fuller C parser.
3. **Modules / imports + a package manifest** — projects with dependencies.
4. **Static proof of contracts + `@invariant`** — make `@requires`/`@ensures` *proven*, not only runtime-checked (Z3 path already exists).
5. **`zeus verify --constant-time`** — prove a function's timing is data-independent. Almost no language ships this; it's the killer security feature.
6. **Sum types + pattern matching + `Result` error handling** — table-stakes for real programs.
7. **LSP + formatter + debugger + tutorial** — the difference between "interesting" and "adopted."

## 5. The North Star (honest, ambitious, NOT magic)

- **AI-authored loop:** AI proposes a change as a semantic edit → Zeus type-checks + Z3-proves the safety contracts → emits `--json` diagnostics on failure for the AI to repair → on success, produces a native binary + a proof/safety certificate. Humans review the certificate and the readable projection, not raw IR. *This is the "few human inputs" vision, made trustworthy by proof.*
- **Verified edge AI (the right "micro-AI"):** a deterministic, zero-heap, constant-time tiny-tensor inference runtime — fixed weights, bounded worst-case execution time, certifiable. This is the realistic, defensible version of "AI baked in," and it fits the safety/embedded wedge (today's PyTorch stack can't be certified for it). **Not** a self-learning binary — that would break determinism, verification, and MISRA, and add an attack surface.
- **Certification artifact:** `zeus cert` → a MISRA / ISO-26262 evidence pack (zero-heap proof, contract proofs, constant-time proofs). This is what a defense/medical/automotive buyer actually pays for.

## 6. What we will STOP claiming (because it costs credibility)

- "Replaces the OS / runs bare metal / bypasses the kernel" — Zeus binaries are normal user-space processes (mmap/fork).
- "Indistinguishability obfuscation / unbreakable / can't be reverse-engineered" — no practical iO exists; anything a CPU runs can be observed.
- "Self-learning AI baked into every binary" — conflicts with determinism, verification, and security.

The honest pitch is **stronger** than the hype: *a systems language that compiles to zero-heap, formally-verified, constant-time C, with provable side-channel protection and first-class C interop — that an AI can write and a prover can certify.* That is genuinely new, and it is buildable.
