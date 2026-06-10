# Zeus — Audit Synthesis & Roadmap to a Viable Product
*Synthesis of six parallel deep-audit reports (audit/01–06), 2026-06-09.*

## Executive verdict
Zeus is a strong prototype with a genuinely defensible wedge (prove + sign + gate
+ audit-foreign-code), but the six-dimension audit found exactly the gaps that
separate "impressive demo" from "product a regulated buyer trusts." The good news:
the gaps cluster into a small number of root causes, and the **two most dangerous
were fixed today**. The honest news: the headline "proved constant-time" claim was
*not* sound before today, and a class of inputs could OOM the host — both now closed,
but it shows the core needs hardening before any external claim.

Detailed evidence is in `audit/01_soundness.md` … `audit/06_market.md`.

---

## FIXED TODAY (verified, regression-tested — now 23/23 golden + 10/10 feature)
- **[CRITICAL · soundness] Secret laundered through a struct field → false constant-time.**
  `Box{v:key}` then `b.v` cleared all taint, so a function branching on a secret reported
  `constant_time:true / PROVED-SAFE` and the signed cert copied that lie. Root cause:
  `StructInit` unhandled in `zir.rs` (fell to `Opaque{secret:false}`). Fix: a struct value
  is secret if any field initializer is secret (sound over-approximation). Locked by
  `neg_secret_launder`. *(01_soundness #1 — the worst finding.)*
- **[CRITICAL · robustness] Parser OOM/hang crash class.** `fn f32(...)`, bare `{}`, `if {`,
  `while {`, `@wcet(`, `P[-5]`, `match`/`enum` tokens, keyword soup — all drove multi-GB
  allocations that wedged the host. Root cause: sub-parsers could fail without consuming a
  token, so `parse_program` spun pushing `LineDirective`s. Fix: a forward-progress guard in
  `parse_program`. All 8 repros now exit cleanly (rc=1). Locked by `neg_typename_fn`,
  `neg_bare_block`. *(02_robustness F1/F3/F4, 03 #1, 04 #1 — one fix, whole class.)*

---

## OPEN — ranked by priority

### P0 — DONE (2026-06-09, verified, unit-tested)
*All four cleared: gate verifies the Ed25519 signature + per-function checks + fail-closed;
Lens flags secret shifts; file-read panics replaced with clean errors; `#[test]` 0 -> 11.
Also landed: `-O3` (3.2x SoA) and numeric-overflow rejection. 24/24 golden + 11/11 unit.*

#### (historical) P0 list — must fix before ANY external "we prove it" claim
1. **Policy gate doesn't verify the signature.** `run_with_policy` checks properties with
   substring matching and never validates the Ed25519 signature; vacuously passes for empty
   function sets. *(01 #2.)* Effort: S–M.
2. **Lens false-safe on secret-dependent shift.** `shl i32 1, %s` (variable-time on the
   embedded targets Zeus markets) is cleared PROVED-SAFE — `shl/lshr/ashr` by a secret amount
   must be a timing sink (or degrade to UNDECIDABLE), never safe. *(01 #3.)* Effort: S.
3. **File-read panics.** Missing/non-UTF-8 source panics on build/run/fmt/test/doc/cert
   (`unwrap`/`expect` not applied uniformly). *(02 F2, 05 #5.)* Effort: S.
4. **No unit tests on the prove-safe analyses.** `#[test]` count = 0. The core claim is itself
   untested; a regression ships a false PROVED-SAFE silently. Add `#[cfg(test)]` for
   zir/bounds/llvm_ingest/cert_sign (esp. a cert sign→verify→tamper round-trip). *(05 #1.)* Effort: M.

### P1 — correctness & trust
5. **Real type checker.** Today wrong programs compile: `let s: str = i32_fn()` only warns; a
   missing `}` can still produce an exit-0 binary; type errors leak raw gcc/ld text. The arity
   check added earlier is only the first slice. *(04 #4.)* Effort: L.
6. **`-O3` for SoA/vectorization.** `zeus build` hard-codes `-O2`; measured −O3 gives 3.2× on
   the SoA kernel (0 → 12 AVX insns). Easiest high-value win. *(03 #2.)* Effort: S.
7. **Numeric-literal range checks.** `1e400` / 26-digit ints accepted silently — unacceptable
   for a MISRA/safety pitch. *(02 F5.)* Effort: S.
8. **Honest "zero-heap" labeling.** It means "no libc malloc"; the runtime uses `mmap` arenas.
   Either say "no dynamic heap allocation (static arena)" or gate the arena too. *(01 #4, 03.)* Effort: S.

### P2 — engineering health & scale
9. **Duplicated parallel codegen (~300 lines forked).** New AST forms silently emit wrong
   (un-rewritten) variables inside `parallel` blocks. Unify. *(05 #2.)* Effort: M.
10. **Split the god-files** (codegen 1977 / parser 1512 / main 1464) — also removes the
    mount-truncation fragility (large-file edits currently go through brittle patch scripts).
    *(05 #3.)* Effort: M.
11. **Compiler CI + `rust-toolchain.toml`**; clear the 17 build / 92 clippy warnings; delete
    dead `backend.rs`. *(05 #4/#6.)* Effort: S–M.
12. **ZIR interprocedural fixpoint** re-lowers all functions each pass (latent O(N²)). *(03.)* Effort: M.

### P3 — language completeness (to write real programs)
13. Enums + pattern matching; wire `Result`/`Ok`/`Err` (in AST/codegen but unusable); real
    strings (len/concat/cmp/format); modules/imports; generics. *(04.)* Effort: L (each).

---

## Market reality (06_market.md)
- **Beachhead: FIPS-140-3 / constant-time crypto tooling.** Uses what's built, has a dated
  regulatory trigger, and the incumbents (Jasmin/HACL*/FaCT) are expert-only — the competitor
  is *unusability*, not a funded rival. (AI-code CI gating is the bigger market but depends on
  the not-yet-built real LLVM front-end.)
- **Single highest-leverage move:** land one FIPS/crypto design partner and get an independent
  (CST-lab or Jasmin/HACL*-literate) party to confirm *one* Zeus constant-time certificate.
  That converts "prototype" to "auditor-acceptable." **Today's struct-field soundness fix is
  directly on this critical path** — you could not have offered that cert yesterday.
- **The bar:** CompCert was formally qualified for the ATR 42/72 (March 2026) — the first
  DO-178C/DO-330 credit from compiler usage. Proof the door is open, and the standard Zeus's
  evidence-pack ambition will be judged against.
- **Load-bearing risk:** trusted-not-verified base + `-O2`/`-O3` can void *source-level*
  constant-time at the binary. For a real FIPS claim you need IR/binary-level constant-time
  validation (or to sit on a verified base). Make that an explicit gating milestone.

## Realistic path (12–24 months)
- **Phase 0 (done today):** kill the crash class; close the worst false-safe.
- **Phase 1 (weeks):** P0 list — gate signature verification, Lens `shl`, file-panic cleanup,
  unit tests on the analyses; plus the `-O3` win.
- **Phase 2 (months):** real type checker; binary/IR-level constant-time validation (the FIPS
  blocker); split god-files + CI; honest zero-heap labeling.
- **Phase 3:** one FIPS/crypto design partner; independent confirmation of one certificate;
  evidence-pack tooling. That is the moment "vision" becomes "viable product."
