# Zeus compiler regression test harness

A small, self-contained golden-file test suite for the Zeus compiler. It builds
each `.zs` case with `zeus_compiler build`, runs the produced binary, and checks
the result against a recorded golden file.

```
tests/
  run_tests.sh          # the runner
  README.md             # this file
  cases/
    <name>.zs           # a test program
    <name>.expected     # its golden output (positive tests only)
```

## How to run

```bash
# run the whole suite
tests/run_tests.sh

# run only specific cases (by base name, no extension)
tests/run_tests.sh verify_proof arith_precedence

# point at a different compiler binary
ZEUS_BIN=/path/to/zeus_compiler tests/run_tests.sh

# (re)generate / refresh all golden files from current compiler output
REFRESH=1 tests/run_tests.sh
```

The script prints a `PASS`/`FAIL` line per case and a summary. It **exits
non-zero if any case fails**, so it drops straight into CI.

### Environment variables

| Var       | Default                                      | Meaning                                   |
|-----------|----------------------------------------------|-------------------------------------------|
| `ZEUS_BIN`| `/tmp/zeus_target/release/zeus_compiler`     | path to the compiler binary               |
| `REFRESH` | `0`                                          | `1` = overwrite `.expected` golden files  |

### gcc -> clang shim

The compiler shells out to `clang` for native code generation. If `clang` is not
on `PATH` but `gcc` is, the runner transparently creates a temporary `clang`
shim that `exec`s `gcc`, prepends it to `PATH`, and removes it on exit. You do
not need to do anything; you'll just see a `[harness] ... using gcc shim` note.

## Why golden = compiler markers, not program output

The Zeus `print` builtin is currently a stub: it only ever emits
`Execution complete.`, and **every produced binary currently exits 0**. That
means a program's own stdout / exit code cannot encode a computed result, so we
cannot golden on program behavior yet.

Instead, each positive test goldens the **compiler's own deterministic stdout
markers**, captured after normalization:

1. Strip ANSI color escapes.
2. Keep only stable marker lines:
   - `[ZEUS VERIFIED] Mathematically proven: ...`  (discharged proofs)
   - `[ZEUS SMT-SOLVER] Formally verifying ...`     (verification entry)
   - ` 📦 Build Success: ... ./<name>`               (successful build)
   - any `Compilation failed/error` / `error[...]`  lines (for diagnosis)
3. Strip the volatile `(Total Time: ...)` suffix and trailing whitespace.

In addition, every positive test must (a) build successfully **and** (b) produce
a binary that runs and exits 0. So the suite checks both the compiler's proof /
build markers and that the emitted native binary is runnable.

This is the most robust signal available while `print` is a stub. Programs such
as `verify_proof` and `proof_secret` are especially strong: their `[ZEUS
VERIFIED]` lines encode the *actual* result of evaluating the asserted
expressions (e.g. `2 * 3 + 4 > 9` proven true), so precedence / arithmetic bugs
would change the golden.

> If/when `print` and exit codes become meaningful, the recommended upgrade is
> to switch those cases to **exit-code-encoded results** (have the program
> `return`/exit with the computed value and compare the run exit code) — the
> runner already captures and checks the binary's run exit code, so extending it
> is small.

## Negative tests

Any case whose name starts with `neg_` is a **negative test**: the compiler is
*expected to fail to compile it*. For these, a non-zero build exit code is a
`PASS` and a successful build is a `FAIL`. Negative cases have **no `.expected`
file** (their golden is simply "compilation must fail").

## Adding a case

### A positive case
1. Create `tests/cases/<name>.zs` exercising a currently-working feature.
2. Generate its golden:
   ```bash
   REFRESH=1 tests/run_tests.sh <name>
   ```
3. Eyeball the new `tests/cases/<name>.expected` to confirm the markers are what
   you expect, then commit both files.

### A negative case
1. Create `tests/cases/neg_<name>.zs` containing source the compiler should
   reject.
2. Run `tests/run_tests.sh neg_<name>` and confirm it reports `PASS` (i.e. the
   compiler really did refuse to build it). No `.expected` file is needed.

## First-run / refresh note

The golden files were generated against the compiler at
`/tmp/zeus_target/release/zeus_compiler`. If you run against a freshly built or
different compiler and see marker mismatches that are purely cosmetic (e.g. the
toolchain reworded a `Build Success` line), re-baseline with:

```bash
REFRESH=1 tests/run_tests.sh
```
and review the diff before committing.

## Current corpus

Positive (12):
`arith_precedence`, `paren_grouping`, `int_types`, `fn_call`, `mutable_var`,
`parallel_reduce`, `secret_var`, `secret_soa`, `soa_particles`, `proof_secret`,
`verify_proof`, `verify_attr`.

Negative (5):
`neg_symbol_garbage`, `neg_repeated_kw`, `neg_not_code`, `neg_bracket_soup`,
`neg_invalid_tokens`.
