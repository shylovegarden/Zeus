# Zeus — Known Drawbacks & Limitations (honest register)
*Updated 2026-06-10. Kept deliberately blunt: every entry is something a serious
evaluator would find, so we name it first. "Fixed" items are listed only where a
recent fix changed the story.*

## 1. Soundness / safety (the things that matter most for a proof tool)
- **Divide/modulo-by-zero is now guarded (FIXED).** Integer `a / b` and `a % b` with a
  non-constant divisor compile to a checked form that calls a clean, deterministic trap
  (`[ZEUS TRAP] ...`, exit 136) instead of crashing with SIGFPE -- in both normal and
  `parallel` code. Constant-nonzero and float divisors are emitted unguarded (no
  overhead; floats follow IEEE inf semantics). Residual note: the guard adds a small
  bounded branch the abstract-step WCET does not yet count, and a float `/0.0` still
  yields inf rather than trapping (by design).
- **Trusted base is not verified.** The proofs are only as trustworthy as the Zeus
  compiler (Rust) + Z3. We are a *high-assurance tool*, not a formally verified
  compiler (CompCert/Jasmin tier). A bug in our analysis or codegen could certify
  something false. Mitigation so far: unit tests on the analyses + the smoke suite.
- **Source-level constant-time is not yet auto-verified on the emitted binary.** `-O3`
  (or any C compiler) could in principle reintroduce a data-dependent branch the source
  didn't have. We prove CT at the Zeus/ZIR level. A harness now exists --
  `tools/verify_ct.sh` runs `clang -O2 -emit-llvm` on the generated C and re-audits the
  IR with the Lens -- but it REQUIRES clang/LLVM (absent in the current dev sandbox), so
  it has only been exercised on its clang-absent path here. A **non-blocking,
  EXPERIMENTAL CI job** (`binary-ct-experimental`) now installs clang and runs it on
  an LLVM runner. The honest gap that keeps it non-blocking: clang's IR drops Zeus's
  `secret`/`public` parameter distinction and the Lens defaults parameters to secret,
  so a whole-module IR audit is a NOISY SIGNAL (runtime helpers + public params show
  expected NOT-PROVENs). The auditor-grade version must carry the source's secret/public
  annotations into the IR audit (e.g. emit `; zeus.public` for non-secret params, or
  audit a single extracted function). That annotation-propagation is the real #1
  prerequisite for a FIPS constant-time claim.
- **The Lens models a subset of LLVM IR.** It is a dependency-free textual reader,
  not the real `llvm-ir` crate; loops, unknown opcodes, and pointer aliasing degrade
  to UNDECIDABLE (never a false PROVED-SAFE, by design — but coverage is partial).
- **Taint is per-function-parameter provenance.** Secrets entering via globals,
  through opaque extern calls, or via complex aliasing are handled conservatively
  (extern → tainted/UNDECIDABLE), but the model is not a full information-flow proof.

## 2. Type system / language semantics
- **The type checker is conservative and shallow.** It flags only *clearly* different
  known kinds (numeric vs str vs struct vs array). It does NOT distinguish among
  numeric types — `i32`/`i64`/`f64`/`bool` are mutually interchangeable (number
  literals are `f64` until annotated), so width/overflow/signedness mismatches and
  precision loss are not caught. No generics, no real inference, no borrow/lifetime
  checking (memory safety comes from zero-heap + arenas, not from the type system).
- **Integer overflow at runtime is wraparound (C semantics)**; only out-of-range
  *literals* are rejected at compile time.
- **`str` is `const char*`** with no operations (len/concat/compare/format). String
  arithmetic is rejected by the type checker, but there is no usable string type.
- **No enums/sum types, no `Result`/error-as-values (in AST but unwired), no modules,
  no generics, no closures.** These parse to clean errors, not features.

## 3. Tooling / ecosystem
- **Single-implementer prototype, no package ecosystem.** No package manager, no
  dependency resolution, no stdlib to speak of beyond a few math builtins.
- **LSP is minimal** (diagnostics only — no completion/hover/go-to-def); no debugger,
  no REPL; `zeus test` is a stub.
- **Certificate provenance is self-signed.** A persistent per-machine Ed25519 key
  signs the `.zcert`; there is no PKI/CA/transparency-log, so "who signed it" is only
  as strong as key custody. `ZEUS_TRUSTED_PUB` lets a verifier pin a key, but there
  is no revocation or chain of trust.

## 4. Backends / portability
- **WASM backend covers only the integer/control-flow subset** (no arrays/structs/
  floats/strings); out-of-subset functions are honestly `skipped`, not mis-compiled.
- **The compiler shells out to gcc/clang** for the final native build; cross-compile
  via `--target` requires clang (passing it to gcc fails).
- **Every binary carries a ~12 KB fixed C prelude + a 64 MB-arena constructor** even
  for hello-world; gcc DCEs unused parts but the arena/startup cost is always paid.
- **"zero-heap" means "no libc malloc."** The runtime still reserves memory via a
  static `mmap` arena; it is deterministic and bounded, but it is not "no allocation
  at all." Labeling is now honest in output.

## 5. Operational / environment
- **WCET is reported in abstract "steps," not nanoseconds** — a sound relative bound,
  not yet calibrated to a target CPU's cycle timing.
- **Large source files are edited via scripts** in this dev setup (the editor
  truncates >~1500-line files on the mount); the three "god files" (codegen ~1977,
  parser ~1500, main ~1500) are overdue for splitting.

## Recently CLOSED (so the register stays accurate)
- Secret-dependent `/=` / `%=` are now flagged variable-time (were a false-"safe").
- Compound assignment (`+= -= *= /= %=`) now emits code (was silently dropped).
- Constant `/0` / `%0` rejected at compile time (incl. in `return` position).
- WASM no longer emits invalid WAT (cert section is escaped).
- Parser OOM crash class, lexer non-ASCII crash, file-read panics, attribute-drop,
  and the struct-field constant-time false-"safe" — all fixed and regression-tested.

**Bottom line:** the *core proof pipeline* (prove → sign → gate → audit) is sound and
tested on its modeled subset; the honest gaps are (a) binary-level CT validation,
(b) a real (deep) type system, (c) the verified-compiler trust story, and
(d) language/ecosystem completeness. None are hidden; all are on the roadmap.

## 6. Newly catalogued (2026-06-10, from the 360 gap review)
- **`-march=native` makes binaries non-portable** (tied to the build host's ISA) -- a
  SIGILL waiting to happen on older CPUs. Fix: conservative baseline default + native opt-in.
- **`zeus fmt` is broken** -- emits operator Debug names (`x Plus y`) into source. Fix the
  formatter's expression printer; add a round-trip test.
- **No LICENSE / SECURITY.md / CONTRIBUTING.md** -- SECURITY/CONTRIBUTING added 2026-06-10;
  LICENSE is the owner's call (recommend Apache-2.0).
- **Constant-time is logical, not microarchitectural** (no cache/speculation/power/EM/
  fault model) -- never claim "timing-attack-proof," only "no logical timing channel."
- **The compiler's own build has no SBOM/SLSA provenance** (we attest user code, not us).
- **No standalone cert verifier** for users without the full toolchain.
- See `GAP_ANALYSIS.md` for the full cross-functional breakdown + how to address each.

## Recently CLOSED (cont.)
- The certificate now binds to the **compiled binary** (`binary_sha256`, signed); a
  tampered/swapped binary is rejected by `zeus verify-cert`.

## Recently CLOSED (2026-06-10, from the must-do list)
- **Differential/property fuzzer** (`tests/fuzz_analyses.py`, in CI): ~1000 checks/run
  asserting analysis ground-truth (secret-sink => not constant-time; while/recursion =>
  unbounded; etc.) + crash-fuzz. Guards against a silent false PROVED-SAFE.
- **`zeus fmt` fixed**: emits real operators (`a * b + a % b - 1`), idempotent, round-trips.
- **`-march=native` no longer the default**: portable for any target by default; native is
  opt-in via `--tune`.
