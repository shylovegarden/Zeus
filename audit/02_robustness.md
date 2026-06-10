# Zeus Compiler — Robustness Audit

**Scope:** fuzzing the `zeus_compiler` binary for crashes (panic / OOM / hang) and
silently-wrong behavior. Read-only on source; binary run read-only from
`/tmp/zeus_target/release/zeus_compiler`. All repros below were run from `/tmp/scratch`
with `export PATH=/tmp/cargo/bin:$PATH CARGO_HOME=/tmp/cargo CARGO_TARGET_DIR=/tmp/zeus_target`.

Exit-code legend: `101` = Rust panic, `134` = abort (memory-allocation failure / OOM),
`137` = OOM-killed, `124` = timeout/hang, `1` = clean handled error, `0` = accepted.

---

## Executive summary

Zeus is **fragile at both ends of its pipeline**: the CLI front door (file I/O) and the
parser→analysis core. The lexer/parser accept many malformed inputs gracefully (good),
but a handful of *tiny, almost-plausible* inputs detonate into multi-gigabyte allocations
that abort the process (exit 134) or wedge the machine. Separately, **6 of 9 file-taking
subcommands panic on the single most common user mistake — a path that does not exist or
is not UTF-8.**

The two worst, most alarming findings:

1. **`pub fn main() {{}}` (a function body that is one nested block) makes the compiler
   try to allocate ~1.4–2.8 GB and abort.** This reproduces on `audit`, `verify`, and
   `build` — it is in the shared front-end, not one command. A nested empty block is a
   trivially reachable input; this is the headline crash.
2. **A missing or non-UTF-8 source file panics** (`build/run/fmt/test/doc/verify/cert`),
   because the file is read with `.expect()` / `.unwrap()` instead of a handled error.

The same runaway-allocation bug class is reachable from several *malformed-but-ordinary*
inputs: `if {`, `while {` (header with no condition), `@wcet(` (unterminated attribute),
keyword soup, and `let }`. These are exactly the kinds of half-typed fragments an editor /
LSP / CI would feed the compiler mid-edit, so they are not purely adversarial.

The `.ll` "Lens" path, the `wasm`/`cert`/`verify-cert`/`import` subcommands, recursion,
mutual recursion, deeply nested calls, large files, large identifiers/numbers, integer/float
overflow literals, and most malformed syntax are all handled **cleanly**. The robustness gap
is concentrated, not pervasive — but the items in it are severe.

### Category scoreboard (crashed vs handled cleanly)

| Category | Result |
|---|---|
| Empty / whitespace / only-comments / block-comment | clean (exit 0) |
| Huge file (100k lines), huge identifier (100k), huge number literal (100k digits) | clean |
| i64-overflow / f64-overflow literals | clean (accepted, see Silently-wrong note) |
| Deep nested parens / deep nested calls (depth-128 guard fires) | clean (exit 1) |
| **Nested bare block `{{}}` / deeply nested blocks** | **CRASH (OOM abort 134)** |
| Unbalanced parens/brackets, missing semicolons, stray operators, incomplete fn/struct | clean |
| **Keyword soup** | **CRASH (OOM 134)** |
| **`if {` no-cond, `while {` no-cond** | **CRASH (OOM 134)** |
| `if x` no-body, `for .. {}` no-body | clean |
| **`let }` (incomplete let)** | **CRASH (abort 134)** |
| SoA zero size `P[0]`, huge size `P[999999999999]`, dup fields, empty struct, undef call | clean |
| **SoA negative size `P[-5]`** | **CRASH / HANG (OOM, outran KILL timeout)** |
| `@wcet()` / `@wcet(abc)` / `@wcet(-100)` / `@wcet(huge)` / dup / on non-fn | clean |
| **`@wcet(` (unterminated paren)** | **CRASH (OOM 134)** |
| `.ll` Lens: empty / garbage / truncated / NUL bytes / weird / huge | clean |
| `wasm`/`cert`/`verify-cert`/`import` on garbage / binary / missing | clean |
| **Missing file / non-UTF-8 file on build/run/fmt/test/doc/verify/cert** | **CRASH (panic 101)** |

Roughly **8 of ~24 probed categories crashed**; the remaining ~16 were handled cleanly.
The crashes cluster into 3 root-cause classes (below), so they are fixable with a small
number of targeted changes.

---

## Findings (ranked by severity)

### F1 — CRITICAL: nested block `{{}}` causes multi-GB allocation → abort
A function whose body is a single nested block triggers a ~1.4–2.8 GB allocation and the
process aborts. Reproduces across the whole front-end.

```
printf 'pub fn main() {{}}\n' > nb.zs
( ulimit -c 0; ulimit -v 1000000; timeout -s KILL 6 zeus_compiler audit  nb.zs )  # exit 134, "memory allocation of 2818572288 bytes failed"
( ulimit -c 0; ulimit -v 1000000; timeout -s KILL 6 zeus_compiler verify nb.zs )  # exit 134, "memory allocation of 1409286144 bytes failed"
```
Smallest trigger confirmed: body must contain a **nested** block. `pub fn main() {}` is
fine (exit 0); `pub fn main() {{}}` and `{{{}}}` abort. `{ let x: i32 = 0; }` is fine.
Without the `ulimit -v` cap the allocation thrashes real memory and **hangs the host**
(observed repeatedly — the bare `{`/nested-block inputs were what wedged the test
sandbox for minutes at a time).

**Likely root cause:** a bare `{` inside a function body is parsed as an expression
statement; the primary-expression parser has no case for `Token::LBrace` and returns
`None` without the body loop making correct progress, after which downstream traversal
(ZIR/bounds for `audit`, codegen-style emission for `build`) drives an indent/size counter
that is fed to `"    ".repeat(indent)` in `codegen.rs` (`generate_statement`, line 688;
also line 1676). With a corrupted/huge `indent` (or a usize underflow from `indent - 1`),
`repeat` requests gigabytes. See also the `parse_statement` catch-all
(`parser.rs:261` → `parse_expression_statement`) and the function-body loop
(`parser.rs:1003-1010`). Root files: `src/parser.rs`, `src/codegen.rs`.

### F2 — CRITICAL: missing / non-UTF-8 source file panics on 6+ subcommands
The most common real-world input error — a wrong path — crashes the compiler with a Rust
panic instead of a clean error message.

```
zeus_compiler build /tmp/nope.zs   # exit 101, panicked at src/main.rs:944:49
zeus_compiler run   /tmp/nope.zs   # exit 101
zeus_compiler fmt   /tmp/nope.zs   # exit 101, panicked at src/main.rs:1303:49
zeus_compiler test  /tmp/nope.zs   # exit 101
zeus_compiler doc   /tmp/nope.zs   # exit 101
zeus_compiler verify /tmp/nope.zs  # exit 101
zeus_compiler cert  /tmp/nope.zs   # exit 101, panicked at src/main.rs:944:49
```
Non-UTF-8 content hits the same line (read succeeds at the OS level but `read_to_string`
returns `Err`):
```
printf '\xff\xfe\x00\x01 fn main' > nonutf8.zs
zeus_compiler build nonutf8.zs     # exit 101, panicked at src/main.rs:944:49
```
By contrast `audit`, `wasm`, `import`, and `verify-cert` handle the missing file cleanly
(exit 1) — so the fix pattern already exists in the codebase, it just isn't applied uniformly.

**Root cause:** `fs::read_to_string(...).expect(...)` / `.unwrap()` at
`src/main.rs:944` (build/cert), `:1303` (fmt), `:1315` (test), `:1394` (doc),
`:1429`/`:1441` (verify), and `Path::new(...).file_stem().unwrap().to_str().unwrap()` at
`:940`. Root file: `src/main.rs`.

### F3 — HIGH: malformed-but-ordinary fragments trigger the same multi-GB OOM
Several inputs that an editor/LSP/CI would routinely produce mid-edit hit the same
runaway-allocation class as F1:

```
printf 'fn struct let if for while return pub fn main if else' > c.zs
zeus_compiler audit c.zs           # exit 134, "memory allocation of 1409286144 bytes failed"   (keyword soup)

printf 'pub fn main() { if { return 1; } }' > c.zs
zeus_compiler audit c.zs           # exit 134, 1.4 GB  (if with no condition)

printf 'pub fn main() { while { } }' > c.zs
zeus_compiler audit c.zs           # exit 134, 1.4 GB  (while with no condition)

printf '@wcet( fn foo() { }' > c.zs
zeus_compiler audit c.zs           # exit 134, 1.4 GB  (unterminated @wcet paren)

printf 'pub fn main() { let }' > c.zs
zeus_compiler audit c.zs           # exit 134 (abort during allocation; "let" with no binding)
```
The recurring constant `1409286144` (0x54000000) and `2818572288` (0xA8000000, exactly
2x) strongly indicate one shared bad allocation site (same `repeat`/capacity computation as
F1), reached when the parser fails to make progress and a header (`if`/`while`/attribute)
or block is consumed without its body. Root files: `src/parser.rs`, `src/codegen.rs`.
Severity High rather than Critical only because each individual fragment is malformed; the
*aggregate* risk (any IDE/CI feeding partial source) is effectively Critical.

### F4 — HIGH: negative SoA size `P[-5]` hangs / OOMs and outran the KILL timeout
```
printf 'struct P { x: i32, } pub fn main() { let p = P[-5]; }' > c.zs
( ulimit -c 0; ulimit -v 1200000; timeout -s KILL 6 zeus_compiler audit c.zs )
# Did not return cleanly; repeatedly wedged the sandbox. P[0] and P[999999999999] are fine (exit 0).
```
A negative array/SoA size almost certainly becomes a huge `usize` (sign-cast) that drives a
buffer allocation or a `0..n` traversal, producing an allocation/loop large enough to
escape even a `timeout -s KILL`. That `P[0]` and `P[999999999999]` are accepted but `P[-5]`
is catastrophic shows size validation is incomplete. Root files: `src/analyzer.rs`
(`Type::Array(inner, size)` handling, line 247), `src/codegen.rs`.

### F5 — MEDIUM: integer/float overflow literals are silently accepted
```
printf 'pub fn main() { let x: i64 = 99999999999999999999999999; }' > c.zs   # exit 0, no diagnostic
printf 'pub fn main() { let x: f64 = 1e400; }'                       > c.zs   # exit 0, no diagnostic
printf 'pub fn main() { let x: i64 = <100000 nines>; }'              > c.zs   # exit 0
```
Literals that cannot fit i64/f64 are accepted with no warning. For a compiler that markets
WCET/safety/MISRA guarantees, silently truncating or wrapping an out-of-range constant is a
**silently-wrong-output** hazard (the emitted C will not hold the literal the source
states). Worth a parse/analysis-time range check. Root files: `src/lexer.rs` (number
tokenization), `src/analyzer.rs`.

### F6 — LOW/INFO: huge SoA size accepted without bound
`P[999999999999]` is accepted at exit 0 during `audit` (static analysis only — it doesn't
allocate the buffer). For `build`/`run` this would emit C that allocates ~terabytes; a
static cap would be friendlier. Lower severity because it only bites at compile/runtime of
the generated artifact, not the compiler itself.

---

## What was robust (passed cleanly — for completeness)

- Empty file, whitespace-only, comment-only (`//` and `/* */`) → exit 0.
- 100k-line file, 100k-char identifier, 100k-digit number → exit 0.
- Deeply nested parens/expressions: parser enforces an `expression_depth > 128` guard
  (`parser.rs:1216`) and returns a clean error (exit 1) — no stack overflow.
- Deeply nested function calls `f(f(...(1)...))` (depth 500) → exit 1, clean.
- Self-recursion, mutual recursion → exit 0 (analyzed, not executed; no infinite descent).
- Unbalanced `(((`, `[[[`, missing semicolons, stray operators, `if x` (no body),
  `for i in 0..8` (no body), incomplete `fn foo(`, incomplete `struct S {` → exit 0/1.
- `@wcet()`, `@wcet(abc)`, `@wcet(-100)`, `@wcet(99999...)`, duplicate `@wcet`,
  `@wcet(10)` on a struct, bare `@`, `@@@@` → exit 0/1.
- SoA `P[0]`, duplicate struct fields, empty struct `struct P {}`, call to undefined
  function → exit 0/1.
- `.ll` Lens audit on empty / garbage / truncated (`define i32 @main() {`) / embedded NUL /
  malformed-LLVM / 200k-line huge `.ll` → exit 0/1, no crash.
- `wasm` on garbage source, `cert` on valid source, `verify-cert` on garbage `.zcert`,
  `import` on malformed `.h` and on raw binary → exit 0/1, no panic.

---

## Top-5 robustness fixes (ranked)

1. **Fix the runaway allocation (F1/F3/F4).** Audit every `"    ".repeat(indent)` /
   `Vec::with_capacity` / `vec![x; n]` in `codegen.rs` and the bounds/ZIR traversal for
   (a) `usize` underflow (`indent - 1` when `indent == 0`) and (b) unbounded/unchecked
   counts. Cap indent depth and clamp any size derived from a literal. This single class is
   behind the worst crashes (`{{}}`, `if {`, `while {`, `@wcet(`, keyword soup, `P[-5]`).

2. **Make the parser guarantee forward progress and handle bare `{`.** Add a
   `Token::LBrace` case to the primary-expression / statement parser (treat as a block or a
   clean error), and ensure the function-body loop (`parser.rs:1003-1010`) always advances
   even when `parse_statement` returns `None` — so malformed fragments produce errors, not
   runaway downstream work.

3. **Replace file-read `.expect()` / `.unwrap()` with handled errors (F2).** Apply the
   pattern already used by `audit`/`wasm`/`import` to `build/run/fmt/test/doc/verify/cert`
   (`src/main.rs:940, 944, 1303, 1315, 1394, 1429, 1441`). A missing or non-UTF-8 file
   should print a diagnostic and `exit(1)`, never panic.

4. **Validate array/SoA sizes (F4/F6).** Reject negative sizes outright and impose a sane
   upper bound on `Name[N]` at analysis time (`analyzer.rs:247`), before any allocation or
   `0..N` traversal is derived from the value.

5. **Range-check numeric literals (F5).** Diagnose integer literals that don't fit their
   declared/inferred type and float literals that overflow f64, instead of silently
   accepting them — important for a toolchain claiming safety/MISRA assurance.

---

*Methodology note:* OOM repros were run under `ulimit -v` with core dumps disabled
(`ulimit -c 0`) and `timeout -s KILL`, because un-capped they exhaust host memory and hang
the machine — which is itself part of finding F1/F3/F4's severity. All exit codes above are
the observed values from those contained runs.
