# Zeus Soundness / False-Safe Audit — 01_soundness

Auditor: adversarial soundness review (read-only on `.rs`; only the existing
release binary was run; only this doc was written).
Binary under test: `/tmp/zeus_target/release/zeus_compiler` (built prior to this audit).
Repo: `/sessions/wonderful-exciting-archimedes/mnt/Zeus/Zeus` (== `C:\Zeus\Zeus`).
Scratch: `/tmp/scratch`. Date: 2026-06-09.

> Note on `build`/`run`: the `build`/`run`/`cert` commands invoke a C toolchain
> (`clang`/`gcc`) that hangs in this sandbox, so end-to-end `.zcert` emission could
> not complete here. All verdicts below come from `zeus audit` (and `audit --json`),
> which runs the *identical* ZIR / bounds / Lens analyses and the identical
> `pf.constant_time` / `reproducible` / `wcet` fields that `write_certificate`,
> the `@constant_time` build gate, and `run_with_policy` consume. Where a claim
> depends on the cert, the JSON from `audit --json` is shown as the load-bearing
> proof that the cert would carry the same (false) field.

---

## Executive summary

The core is **NOT sound for its headline claim** (constant-time / non-leakage on
Zeus source). There is a single, trivially-reachable, definitively-confirmed
**Critical false-safe**: any secret value laundered through a **struct field**
(`Struct { f: secret }` then read back `b.f`) **completely loses its taint** in the
ZIR pass, so a secret array index, secret branch, or secret division is reported
`constant_time: true` / **PROVED-SAFE**, the `@constant_time` build contract is
silently satisfied, and the emitted `.zcert` certifies `"constant_time":true` for a
function that has a real cache-timing / control-flow leak. Root cause: `StructInit`
is unhandled in `zir.rs::lower_expr` and falls into the `_ => Opaque {secret:false}`
arm. This is the existential failure for a proof tool: it affirms a property it
cannot back. The Lens (`.ll`) has a second, real false-safe: a **secret-dependent
variable shift count** (`shl x, secret`) is cleared as PROVED-SAFE / constant-time:
yes — a genuine variable-time channel on the embedded targets Zeus markets to.
Secondary holes: the policy gate never verifies the cert signature and passes
required properties vacuously when absent; "zero-heap" actually means "no libc
malloc" while the runtime allocates dynamically via `mmap` arenas. Determinism and
the WCET/overflow handling held up under attack (good). The compiler also **hangs
(infinite loop / DoS)** on variable reassignment (`x = 5;`) and on a field-access
used directly in index position (`table[b.v]`), because the AST has no assignment
node — a robustness bug that also blocks some analyses from ever running.

Bottom line: **do not make any "proved constant-time" / "PROVED-SAFE" product
claim** until the StructInit taint hole and the Lens variable-shift hole are closed.
The conservative scaffolding (extern degradation, `while`/recursion → unbounded,
checked WCET arithmetic, determinism-by-source-existence) is genuinely sound; the
leak it lets through is a missing transfer function, not a broken architecture.

---

## Finding 1 — CRITICAL — secret laundered through a struct field clears all taint (false constant-time / false PROVED-SAFE / false cert)

**Severity: Critical (false-safe / unsound).**

### Repro A — secret array index via struct field

`/tmp/scratch/attack_field.zs`:
```
struct Box {
    v: i32,
}
@constant_time
fn lookup(secret idx: i32) -> i32 {
    let table: [i32; 4] = [10, 20, 30, 40];
    let b: Box = Box { v: idx };
    let j: i32 = b.v;
    return table[j];
}
pub fn main() {
    let secret s: i32 = 2;
    println(lookup(s));
}
```
Command + real output (ANSI stripped):
```
$ /tmp/zeus_target/release/zeus_compiler audit attack_field.zs
== ZEUS AUDIT: The Lens ==  (static assurance report)
file: attack_field.zs

 fn lookup  [PROVED-SAFE]
    memory-safe:   zero-heap   constant-time: yes   reproducible: yes
    WCET: 15 steps   stack: 120 bytes
 fn main  [PROVED-SAFE]
    memory-safe:   zero-heap   constant-time: yes   reproducible: yes
    WCET: 24 steps   stack: 72 bytes

FINDINGS
  (none) -- no timing channels or unbounded functions detected.

VERDICT: 2 function(s) | 2 constant-time | 2 bounded | 0 finding(s)
         2 PROVED-SAFE | 0 NOT-PROVEN | 0 UNDECIDABLE
[ZEUS AUDIT GATE] PASSED
```
The **control** (same code, `return table[idx]` directly) is correctly caught:
```
$ /tmp/zeus_target/release/zeus_compiler audit base.zs
 fn lookup  [NOT-PROVEN]
    memory-safe:   zero-heap   constant-time: NO ...
  [!] fn lookup: secret value used as memory index -> cache-timing channel [UNMITIGATED ...]
[ZEUS AUDIT GATE] FAILED
```
So the *only* difference is routing `idx` through `Box{v:idx}` → `b.v`. The leak is
real (the generated C indexes `table[j]` with a secret-derived `j`), but Zeus says
constant-time / PROVED-SAFE.

### Repro B — secret-dependent branch via struct field
`/tmp/scratch/attack_branch.zs` (`if b.v == 1234 { return 1 }` with `b.v` secret):
```
 fn cmp  [PROVED-SAFE]
    constant-time: yes   reproducible: yes
VERDICT: ... 0 finding(s)
[ZEUS AUDIT GATE] PASSED
```

### Repro C — secret-dependent division via struct field
`/tmp/scratch/a_div.zs` (`return 1000000 / b.v`, `b.v` secret): `fn dv [PROVED-SAFE]
constant-time: yes`. The direct control `return 1000000 / k` is correctly
`[NOT-PROVEN] constant-time: NO`.

### Repro D — the cert / policy gate carries the false claim
`audit --json` uses the same `pf.constant_time` that `write_certificate` writes and
that `run_with_policy` / the `@constant_time` build gate read:
```
$ /tmp/zeus_target/release/zeus_compiler audit attack_field.zs --json
... "functions":[{"name":"lookup","verdict":"PROVED-SAFE", ... "constant_time":true ...
```
Therefore the signed `.zcert` would assert `"constant_time":true` for `lookup`, and
`zeus run attack_field.zs --require=constant-time` would pass its gate
(`run_with_policy`, main.rs:781-784: `!cert.contains("\"constant_time\":false")`)
and execute the leaking program. The Ed25519 signature then provides cryptographic
integrity over a **false** assertion.

### Root cause
`zeus_compiler/src/zir.rs`:
- `Expression::StructInit` is **not** matched in `lower_expr`; it falls through to
  `_ => self.push(ZInst::Opaque, ZType::Unknown, false, false)` (zir.rs:187). The
  struct value is therefore born **untainted**, regardless of secret initializers.
- Consequently `Expression::FieldAccess` (`b.v`, zir.rs:164-167) reads
  `self.sec(b) == false`, so `j`/`b.v` is public. The secret-index sink
  (zir.rs:155-163) and the secret-branch sink (`lower_cond`, zir.rs:191-196) and the
  secret-division sink (zir.rs:143-145) never fire.
- The build-time `@constant_time` contract (main.rs:1168-1174) and the certificate
  (`write_certificate`, main.rs:393) read this same `pf.constant_time`, so the lie
  propagates to the contract gate and the signed cert verbatim.

### Impact
Total bypass of Zeus's headline guarantee on Zeus source. Any AES/RSA/compare-MAC
style routine can be made to pass `@constant_time` + certify `constant_time:true` by
spilling the secret into a one-field struct first — exactly the idiom real crypto
code uses (key material in structs). This is a confidentiality break, not a cosmetic
miss.

### Recommended fix + effort
Model aggregates in ZIR: handle `Expression::StructInit` (taint = any field
initializer secret; ideally field-sensitive) and make `FieldAccess` propagate the
secret bit of the relevant field, with the safe default that **any** secret field
taints any field read whose field-sensitivity cannot be resolved. Mirror the
`.ll` Lens, which already has an alloca/GEP field model that resists this (see
Finding 5 — the Lens caught the equivalent). **Effort: M.**

---

## Finding 2 — CRITICAL — `.zcert` certifies properties the analysis didn't establish; policy gate trusts an unverified, vacuously-passing cert

**Severity: Critical (cert claims a property the analysis did not prove) — partly a
consequence of Finding 1, partly an independent gate weakness.**

### 2a — cert out-claims analysis (consequence of Finding 1)
Demonstrated in Finding 1 Repro D: `"constant_time":true` is emitted for a function
with a secret-dependent memory index. `write_certificate` (main.rs:374-409) copies
`pf.constant_time` straight into the signed body, so the cert asserts a property the
ZIR pass did not (and could not) establish.

### 2b — `run_with_policy` never verifies the embedded signature
`run_with_policy` (main.rs:774-795) reads the `.zcert`, does plain `cert.contains(..)`
substring checks (lines 781-784), then executes the binary. It **never** calls
`cert_sign::verify_cert_file`, unlike `cmd_cert` (main.rs:427) which does. Because
`run_with_policy` rebuilds first (main.rs:775), an externally-tampered cert is
overwritten — so this is not a forged-cert RCE on its own — but the gate's trust
decision is structurally divorced from signature verification, so any future path
that gates on a *pre-existing* cert (CI artifact, remote attestation) is unprotected.

### 2c — required properties pass *vacuously*
The gate checks `!cert.contains("\"constant_time\":false")` (main.rs:782) and
`!cert.contains("\"reproducible\":false")` / `!...wcet_steps":null` (781,783). A
certificate with **zero functions**, or whose only functions are trivial, contains
no `:false`/`:null` substring at all, so `--require=constant-time,reproducible,bounded`
**passes vacuously**. There is no positive proof obligation ("every function is
proven constant_time"), only an absence-of-negation check.

### Root cause
main.rs:781-784 (substring matching instead of structured per-function checks);
main.rs:774-795 (no `verify_cert_file` call in the gate path); Finding 1 root cause
feeds 2a.

### Impact
The certificate — Zeus's entire trust artifact — can assert `constant_time:true`,
`reproducible:true`, `zero_heap:true`, `bounded` for programs/functions that were
never proven to have them, and the policy gate will execute them.

### Recommended fix + effort
(1) Parse the cert as JSON and require, per property, that **every** function
positively carries `"<prop>":true` (and `wcet_steps` is a number) — fail closed on a
missing/empty functions array. (2) Call `cert_sign::verify_cert_file` inside
`run_with_policy` before trusting any field. (3) Fix Finding 1 so the field is true.
**Effort: S** for (1)+(2); the real fix is Finding 1.

---

## Finding 3 — HIGH — the Lens reports PROVED-SAFE / constant-time:yes for a secret-dependent variable shift (`shl x, secret`)

**Severity: High (false constant-time on a real variable-time channel; the Lens
affirmatively clears it rather than degrading to UNDECIDABLE).**

### Repro
`/tmp/scratch/L3c.ll`:
```
define void @sh3(i32 %s) {
entry:
  %r = shl i32 1, %s
  ret void
}
```
Here `%s` is the secret (params are secret by default; no `; zeus.public:`), and it
is the **shift amount**. Real output:
```
$ /tmp/zeus_target/release/zeus_compiler audit L3c.ll
== ZEUS AUDIT: The Lens (LLVM-IR, multi-block, interprocedural) ==  (non-Zeus code)
file: L3c.ll   functions: 1
 fn sh3  (2 basic block(s))  [PROVED-SAFE]
    constant-time: yes   fully-modeled: yes
    (none) -- no secret-dependent branch, index, or division detected.

[ZEUS AUDIT GATE] PASSED -- PROVED-SAFE on the modeled subset.
```
For comparison, the Lens *does* correctly flag secret branch (`L?`/`ctrl.ll`),
secret GEP index (`L2.ll`), and secret division (`L1.ll`) — all NOT-PROVEN. Only the
variable-shift channel is missed.

### Root cause
`zeus_compiler/src/llvm_ingest.rs`: the sink set in the report pass (lines 304-352)
recognizes only `sdiv/udiv/srem/urem/fdiv/frem` (division), `getelementptr` (index),
`br i1`/`switch` (control flow), and `ret` (data return). `shl`/`lshr`/`ashr` are in
`RECOGNIZED` (so they don't trigger UNDECIDABLE) but are **never treated as timing
sinks**. A data-dependent shift count is variable-latency on numerous targets Zeus
explicitly markets to ("Bare-metal Automotive ECUs / Drones", "RISC-V IoT",
Cortex-M0/M0+). The Lens thus clears (PROVED-SAFE) an unmodeled-as-sink real channel
instead of declining to claim safety.

### Impact
Non-Zeus crypto audited "through the Lens" can present `constant-time: yes` /
PROVED-SAFE while leaking via secret-dependent shifts (a classic side channel, e.g.
in big-integer / Montgomery code). The Lens's own docstring promise — "It never
reports PROVED-SAFE on code it could not fully reason about" — is violated: it *did*
reason about the shift (recognized opcode) but reasoned incompletely.

### Recommended fix + effort
Add `shl/lshr/ashr` (and arguably `mul`/`udiv` already covered) with a **secret
shift-amount** check to the sink scan: if the second operand's origin set is
non-empty, emit a `variable-time` finding (or at minimum degrade to UNDECIDABLE on
targets where shifts aren't constant-time). **Effort: S.**

---

## Finding 4 — MEDIUM — "zero-heap" means "no libc malloc"; the runtime allocates dynamically via `mmap` arenas, which the enforcer does not ban

**Severity: Medium (misleading guarantee / label, not a clean false-safe of the
stated MISRA rule).**

### Evidence (source, read-only)
- The enforcer (`enforcer.rs`) bans only the textual patterns `malloc(`, `calloc(`,
  `free(`, `#include <pthread.h>`, `#include <stdlib.h>` (enforcer.rs:36-42) and the
  extern names containing those (enforcer.rs:7-13). It does **not** ban `mmap`.
- Codegen's allocator is `__zeus_arena_alloc`, backed by `mmap(... MAP_ANON ...)`
  (codegen.rs:299-322), and tensors/NVMe/parallel/cluster allocate from it
  (codegen.rs:844-847, 1197, 1839). On Windows it is `VirtualAlloc` (codegen.rs:102).
- ZIR's `uses_heap` (the input to the cert's `zero_heap`) is set **only** for
  `TensorDefinition`, `NvmeDmaMap`, `ParallelBlock`, `ClusterBlock`
  (zir.rs:176-180, 219-223). The program-level `zero_heap` is then "no reachable
  `uses_heap` and no reachable extern" (zir.rs:347-351).

### Impact
A program that uses tensors/parallel blocks is dynamically allocating memory at
runtime (arena over `mmap`), yet "zero-heap" is the marketed memory-safety pillar and
the cert reports `zero_heap` based on a definition that equates heap with libc
`malloc`. For a safety/medical audience ("IEC 62304", "MISRA 21.3") this is a
material mislabel: `mmap`/`VirtualAlloc` *is* dynamic OS allocation. It is consistent
with Zeus's internal definition, so it is not a clean contradiction of its own gate —
hence Medium, not Critical — but it will not survive an external safety reviewer.

### Recommended fix + effort
Either (a) rename the property to "no-libc-malloc / static-arena" and document that
arena memory is reserved up-front, or (b) make `zero_heap` require that the arena is
never instantiated (no tensor/parallel/cluster/NVMe reachable) AND add `mmap`/
`VirtualAlloc` to the enforcer's banned set for true-zero-heap builds. **Effort: S
(rename/doc) / M (real enforcement).**

---

## Finding 5 — MEDIUM — secret-tainted *return value* laundered through a struct field is not reported (confidentiality false-negative)

**Severity: Medium (missed data-flow leak; not the headline timing claim, but a real
non-leakage false-negative and a second symptom of the Finding-1 root cause).**

### Repro
`/tmp/scratch/a_ret.zs`:
```
struct Box { v: i32, }
fn leak(secret k: i32) -> i32 {
    let b: Box = Box { v: k };
    return b.v;
}
pub fn main(){ let secret s: i32 = 42; println(leak(s)); }
```
Output: `fn leak [PROVED-SAFE]`, `FINDINGS (none)`, `GATE PASSED`. The direct
control (`return k`) fires the ZIR sink "returns a secret-tainted value to a public
caller" (zir.rs:208) and the interprocedural `returns_secret` summary. Through the
struct field, the return is laundered to public and the summary never marks `leak`
as secret-returning, so downstream callers are also under-tainted.

### Root cause
Same as Finding 1 (`StructInit` unmodeled in zir.rs:187 ⇒ `b.v` public). Also defeats
the interprocedural taint fixpoint (zir.rs:279-292), since `returns_secret_flag` is
never set for `leak`.

### Impact
Secret data can be exfiltrated to a public return / caller while Zeus certifies the
function leak-free. Weakens the "non-leakage" half of the ZIR pass, not only the
timing half.

### Recommended fix + effort
Fixed by the Finding-1 aggregate-taint fix. **Effort: included in Finding 1 (M).**

---

## Finding 6 — MEDIUM — compiler hangs (infinite loop / DoS) on variable reassignment and on field-access-as-index

**Severity: Medium (availability / robustness; also blocks analyses from running, so
it indirectly hides issues rather than failing safe).**

### Repro A — reassignment
`/tmp/scratch/assign.zs`:
```
pub fn main() {
    let x: i32 = 0;
    x = 5;
    println(x);
}
```
`timeout -s KILL 8 zeus_compiler audit assign.zs` produced **no output** and was
killed by the timeout (the process never returned). The AST has **no assignment
statement** (confirmed by reading `ast.rs:11-118` — there is `Let`, `If`, `For`,
`While`, `ExpressionStatement`, etc., but nothing for `lhs = rhs`), and the parser
loops instead of emitting a clean error.

### Repro B — field access in index position
`/tmp/scratch/a_direct.zs` (`return table[b.v];`) likewise hung the analyzer with no
output until killed.

### Impact
A `.zs` that uses ordinary reassignment (the most common statement form in any real
program) hangs `zeus audit`/`build`. Beyond DoS, it means whole categories of
programs can never reach the analyses, and a reviewer cannot even test taint
laundering via mutation (the canonical way to break a flow-insensitive taint pass).

### Recommended fix + effort
Add an `Assignment { target, value }` statement to the AST/parser with a real parse
error if unsupported, and a ZIR transfer function (re-tainting the target). Add an
input-size/step guard so the parser/analyzer can never loop unbounded. **Effort: M.**

---

## Things that held up under attack (genuinely sound — credit where due)

- **Determinism / `@deterministic` / reproducible.** Laundering `rand()` through a
  struct field (`/tmp/scratch/a_nd.zs`) did **not** produce `reproducible:true`: the
  function-level determinism verdict checks for the *existence* of any nondet-tainted
  SSA value (zir.rs:242), and `rand()` is also an unknown call → `reaches_extern` →
  constant_time withheld. Reported `reproducible: NO` (correct). `is_nondet_source`
  (zir.rs:93-98) covers rand/time/clock/IO/getpid/etc.
- **WCET conservatism.** `while` and recursion → `None` ("UNBOUNDED"); a declared
  `@wcet` over a `while`/recursion FAILS (bounds.rs:103,137,231). A 1e6-iteration
  `for` against `@wcet(50)` is correctly flagged "EXCEEDS" (`/tmp/scratch/w2.zs`).
  Loop-bound and per-iteration arithmetic use `checked_*`/`saturating_*`
  (bounds.rs:41-58, 104-118), so a low-WCET-via-overflow attack saturates to MAX
  (the safe direction) rather than wrapping to a small passing number.
- **Lens core sinks.** Secret branch, secret GEP index, secret `sdiv`, and secret
  alloca→load→div are all correctly NOT-PROVEN (`ctrl.ll`, `L2.ll`, `L1.ll`); the
  alloca/GEP memory model propagates taint through stack slots and struct-field GEPs,
  and non-alloca pointer stores degrade to UNDECIDABLE (llvm_ingest.rs:282-296).
- **Extern degradation.** Reachable `extern fn` / unknown calls withhold constant-time
  and force `wcet=null` / not-zero-heap (zir.rs:300-304,340; main.rs:387-393).

---

## Ranked top-5 "fix before any product claim"

1. **(Finding 1, Critical) Model aggregate taint in ZIR.** Handle `StructInit` and
   propagate field taint in `FieldAccess`; secret-through-struct is a one-line idiom
   that defeats constant-time, the `@constant_time` gate, and the cert today.
   Effort M. *Until this lands, every "proved constant-time" claim on Zeus source is
   false-safe-able.*
2. **(Finding 2, Critical) Make the cert/policy gate prove, not assume.** Parse the
   cert as JSON; require every function to positively carry `"<prop>":true`; fail
   closed on empty/missing function arrays; call `verify_cert_file` in
   `run_with_policy`. Effort S.
3. **(Finding 3, High) Add variable-shift (`shl/lshr/ashr` by secret) as a Lens
   timing sink** (and audit other recognized-but-non-sink opcodes), or degrade to
   UNDECIDABLE. Effort S.
4. **(Finding 6, Medium) Eliminate the parser/analyzer hangs** on reassignment and
   field-as-index; add a real assignment node with a ZIR re-taint transfer function
   and a hard step/size guard. Effort M. *Also required so taint-laundering-via-
   mutation can even be tested.*
5. **(Finding 4, Medium) Stop calling mmap-arena builds "zero-heap"** — rename to
   "no-libc-malloc / static-arena", or enforce real zero-heap (ban mmap/VirtualAlloc
   and reject tensor/parallel/cluster) for that label. Effort S–M.

---

### Reproduction index (all under `/tmp/scratch`, all run with the existing binary)
| file | attack | verdict observed | expected |
|------|--------|------------------|----------|
| base.zs | secret index (direct) | NOT-PROVEN (correct) | NOT-PROVEN |
| attack_field.zs | secret index via struct field | **PROVED-SAFE / ct:true** | NOT-PROVEN |
| attack_branch.zs | secret branch via struct field | **PROVED-SAFE** | NOT-PROVEN |
| a_div.zs | secret division via struct field | **PROVED-SAFE** | NOT-PROVEN |
| a_div_ctrl.zs | secret division (direct) | NOT-PROVEN (correct) | NOT-PROVEN |
| a_arr.zs | secret index via array element | NOT-PROVEN (correct) | NOT-PROVEN |
| a_nd.zs | rand via struct field | reproducible:NO (correct) | non-reproducible |
| a_ret.zs | secret return via struct field | **PROVED-SAFE** | NOT-PROVEN |
| ctrl.ll | secret branch (Lens) | NOT-PROVEN (correct) | NOT-PROVEN |
| L1.ll | secret alloca→div (Lens) | NOT-PROVEN (correct) | NOT-PROVEN |
| L2.ll | secret GEP index (Lens) | NOT-PROVEN (correct) | NOT-PROVEN |
| L3c.ll | secret variable shift (Lens) | **PROVED-SAFE / ct:yes** | NOT-PROVEN/UNDECIDABLE |
| w2.zs | 1e6 loop vs @wcet(50) | EXCEEDS (correct) | EXCEEDS |
| assign.zs | reassignment | **HANG (DoS)** | clean parse/error |
| a_direct.zs | field-access as index | **HANG (DoS)** | NOT-PROVEN |
