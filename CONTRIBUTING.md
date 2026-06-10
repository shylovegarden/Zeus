# Contributing to Zeus

Thanks for your interest. Zeus is an early, single-implementer research compiler; the
bar for changes is **soundness first, then correctness, then features**.

## Ground rules
- **Never introduce a false PROVED-SAFE.** Any change to an analysis (`zir.rs`,
  `bounds.rs`, `llvm_ingest.rs`, `cert_sign.rs`) must add a regression test and must
  degrade to UNDECIDABLE/NOT-PROVEN rather than over-claim.
- **Every change stays green.** Before opening a PR, run the full suite (below) — all
  must pass, with zero new false positives on the examples/showcase.
- **Honest scope.** Don't add a feature that only "works" by emitting a comment or a
  stub. If it's partial, label it partial.

## Run the tests
```
cd zeus_compiler && cargo build --release && cargo test --release
ZEUS_BIN="$(pwd)/zeus_compiler/target/release/zeus_compiler" bash tests/run_tests.sh
ZEUS_BIN=... bash tests/feature/run_feature_tests.sh
ZEUS_BIN=... bash tests/smoke_test.sh
```
CI runs all of these plus the (experimental, non-blocking) binary-CT job.

## Conventions
- Shell scripts and `.zs`/`.rs` files are **LF** (enforced via `.gitattributes`).
- `.zs` sources are **ASCII**; filenames must not start with a digit; don't name a
  function after a C type (`double`, `int`). See `ZEUS_HANDBOOK.md` "Gotchas".
- Large compiler files are edited carefully (codegen/parser/main are ~1.5–2k lines and
  are overdue for splitting — a welcome contribution).

## Good first issues
Fix `zeus fmt` (it currently emits `x Plus y`); wire `Result`/error-values; add LSP
completion; split a god-file into modules.
