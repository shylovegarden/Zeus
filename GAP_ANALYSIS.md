# Zeus — 360° Gap Analysis (what an advanced cross-functional team would flag)
*2026-06-10. Organized by the people who would review it — silicon up to end user.
Each item: the gap, why it matters, and how to address. Items already tracked in
DRAWBACKS.md/ROADMAP are referenced, not repeated; this doc adds what those missed.*

> Just closed (this pass): the certificate now binds to the **compiled binary**
> (`binary_sha256`, signed), and `verify-cert` rejects a tampered binary — a real
> supply-chain hole that no prior list named.

---

## 1. Silicon / hardware / circuit-board engineers
**These are the gaps a hardware-security or RTOS engineer raises first.**
- **"Constant-time" is logical, not microarchitectural.** We prove no secret-dependent
  branch/index/division at source+IR. We do NOT model the CPU: cache behavior, branch
  predictor, SMT/hyperthreading, speculative execution (Spectre/Meltdown class),
  variable-latency instructions (some multipliers/dividers), or store-to-load
  forwarding. CT on paper ≠ CT on silicon. → *Address:* integrate `dudect`/`ctgrind`-
  style statistical timing tests on the built binary; document the microarchitectural
  threat model explicitly; never claim "timing-attack-proof," only "no logical timing
  channel in the modeled subset."
- **No power / EM side-channel (DPA/SPA) story.** Smartcard/embedded-secure teams need
  masking, and we offer none. → *Address:* out of scope for now — say so; a `@masked`
  pass is a research item, not a near-term claim.
- **No fault-injection / glitch resistance** (redundancy, control-flow integrity).
  → *Address:* document as out of scope; for ISO-26262/DO-178 later, add CFI + dual
  computation behind a flag.
- **WCET is in abstract "steps," not CPU cycles.** A hard-real-time engineer needs a
  per-core pipeline/cache/memory model to get nanoseconds. → *Address:* calibrate steps
  to cycles for one target CPU; cite the model; keep "steps" labeled as relative.
- **`-march=native` → non-portable binaries** (tied to the build host's ISA). Ship a
  binary built on a Zen4 box, crash with SIGILL on an older CPU. → *Address:* default to
  a conservative baseline (`-march=x86-64-v2`) and make `native` opt-in; add a
  cross-arch (ARM/RISC-V) CI matrix — currently only x86-64 Linux is tested.

## 2. Compiler / PL researchers
- **The analyses have no formal soundness proof.** Taint/CT/WCET/determinism are
  *believed* sound and unit-tested, not proven against a formal semantics. A PL referee
  wants a theorem. → *Address:* write the small-step semantics + a paper soundness
  argument for the CT analysis first (the sellable one); long-term, mechanize in Coq/Lean.
- **Trusted base is unverified** (Rust compiler + our code + Z3). Not CompCert tier.
  (Tracked.) → *Address:* the verify-CT-on-IR loop reduces reliance on our codegen;
  long-term, self-host or mechanize.
- **Numeric type collapse may make analyses unsound.** i8/i32/u64/f64/bool are treated
  as interchangeable; a WCET/overflow/bit-width assumption could be wrong if a value is
  actually i64. → *Address:* a real typed IR (the never-built "ZIR v2") with width-aware
  types; this is the single biggest correctness debt.
- **ZIR is shallow** (no CFG, no dominator tree, no real dataflow framework); the
  keystone "ZIR v2 + LLVM front-end" was never built. → *Address:* it remains the #1
  architectural investment (unlocks the real Lens + sound cross-fn analysis).
- **No termination proof** beyond "constant-bounded loops"; recursion → UNBOUNDED but
  not proven terminating. → *Address:* honest labeling (done); a ranking-function check
  is a later feature.

## 3. Security / cryptography engineers
- **Source-level CT not yet verified on the optimized binary.** (Tracked; harness +
  experimental CI exist; the real fix is carrying secret/public annotations into the IR
  audit.) → *Address:* annotation propagation — the #1 prerequisite for a FIPS claim.
- **No differential testing of the analyses against a ground-truth tool.** We test for
  crashes and a few hand cases; we don't cross-check CT verdicts vs `ctgrind`/`dudect`
  or fuzz the analysis for false PROVED-SAFE at scale. → *Address:* a differential/fuzz
  harness that mutates programs and compares verdicts — the highest-value testing work.
- **No written threat model / security policy.** A reviewer can't tell what we claim to
  defend against. → *Address:* SECURITY.md (adding now) + a THREAT_MODEL.md.
- **Provenance is self-signed; no PKI / transparency log / revocation.** (Tracked.)
  → *Address:* integrate Sigstore/Rekor for keyless signing + transparency.
- **`secret` coverage gaps:** floats, aggregates-by-value, secrets via globals/opaque
  FFI are conservative-or-missed. → *Address:* extend taint seeds; FFI capability
  contracts (the "quarantined FFI" pillar).

## 4. Systems / infra / DevOps / build engineers
- **We attest user code but NOT the compiler's own build** (no SBOM/SLSA provenance for
  the Zeus binary itself — ironic for a supply-chain tool). → *Address:* generate SLSA
  provenance + SBOM for the compiler release in CI.
- **No real distribution.** `install.sh` is untested cross-platform; no signed binary
  releases, no semver of the language, no package registry, no reproducible-build
  attestation across machines. → *Address:* release automation (GitHub Releases +
  checksums + signing), pin semver, ship a standalone verifier.
- **CI is minimal:** single job, no OS/arch matrix, no caching, no release, no fuzzing,
  no ASan/UBSan on the generated C or the Rust. → *Address:* add a matrix, sanitizer
  runs, fuzz job, and a cron differential-CT job.
- **No standalone cert verifier for users without the toolchain.** A consumer who
  receives a `.zcert` + binary must install all of Zeus to check it. → *Address:* a tiny
  standalone `zeus-verify` binary (sig + binary-hash only, no compiler).

## 5. Application developers (people writing Zeus)
- **`zeus fmt` is BROKEN** — it round-trips operator *Debug names* into source
  (`x Plus y` instead of `x + y`), producing files that then fail to compile. Concrete,
  reproducible, unaddressed. → *Address:* fix the formatter's expression printer to emit
  real operators (map Plus→`+`, Star→`*`, …); add a fmt round-trip regression test.
- **Language is incomplete:** no enums/sum types, `Result`/error-values (in AST, unwired),
  real strings, modules/imports, generics, closures; tiny stdlib. (Tracked.) → *Address:*
  `Result`/error-values first (highest dev value), then strings, then enums+match.
- **Errors still leak raw gcc/ld text** for unmodeled constructs. → *Address:* a pre-codegen
  "unsupported construct" check that emits clean Zeus errors instead of letting C fail.
- **Tooling thin:** LSP is diagnostics-only (no completion/hover/goto); no debugger, no
  REPL, no test runner (`zeus test` is a stub). → *Address:* LSP completion + a real
  `zeus test` are the two highest-leverage DX items.

## 6. End users (running Zeus-built software)
- **Binary↔cert binding (JUST FIXED).** The cert now includes `binary_sha256` and
  `verify-cert` rejects a swapped/tampered binary.
- **Running native code from a gate is still a trust step** — `zeus run --require` exec's
  `./binary` after checking the cert; there is no sandbox. → *Address:* document that the
  gate trusts the (now binary-bound, signed) cert; optionally run inside a Wasm/microVM
  sandbox for untrusted producers.
- **No revocation / expiry on certs.** A once-valid cert is valid forever. → *Address:*
  add an issued-at/expiry field and an optional revocation list.

## 7. Project / product / legal hygiene
- **No LICENSE, SECURITY.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md.** Blocks any serious
  OSS adoption or contribution. → *Address:* adding SECURITY.md + CONTRIBUTING now;
  **LICENSE is a decision for the owner** — recommend Apache-2.0 (patent grant suits a
  systems/security tool) or MIT; pick before any public release.
- **"Zeus" is a heavily-used name** (Aave's Zeus, others). → *Address:* trademark/name
  search before launch; have a fallback name.

---

## The short list — what MUST be done next, in order
1. **Annotation-propagated binary-CT verification** (carry secret/public into the IR
   audit) — turns the experimental CI signal into a real, auditor-grade FIPS check.
2. **Differential/fuzz testing of the analyses** — protects against a silent false
   PROVED-SAFE, the existential failure.
3. **A real typed IR (ZIR v2)** — fixes the numeric-collapse soundness debt AND unlocks
   the real LLVM Lens.
4. **Fix `zeus fmt`** (concrete bug) + clean "unsupported construct" errors.
5. **Distribution & hygiene:** LICENSE/SECURITY/CONTRIBUTING, standalone verifier, SLSA
   provenance for the compiler itself, conservative `-march` default + cross-arch CI.
6. **`Result`/error-values** — the highest-value language-completeness step.

Everything above is named so none of it is a hidden surprise. The core proof pipeline
is sound and tested on its modeled subset; these are the gaps between "verified
prototype" and "a product a silicon/security/dev team trusts end to end."
