# Zeus FEATURE test suite

End-to-end hardening tests for Zeus's newer capabilities. Every check runs the
**real** `zeus_compiler` binary (plus `wasmtime` and the agent-loop driver) and
asserts behavior that was observed empirically -- nothing is mocked.

## Running

```sh
export PATH=/tmp/cargo/bin:$PATH CARGO_HOME=/tmp/cargo CARGO_TARGET_DIR=/tmp/zeus_target
ZEUS_BIN=/tmp/zeus_target/release/zeus_compiler \
WT=/tmp/wasmtime-v25.0.0-x86_64-linux/wasmtime \
bash tests/feature/run_feature_tests.sh
```

* `ZEUS_BIN` (required) -- path to the `zeus_compiler` binary. The suite uses the
  prebuilt binary; it never invokes `cargo`.
* `WT` (optional) -- path to a `wasmtime` binary. If unset/missing, the WASM
  round-trip check is **SKIPPED** (not failed).
* `PY` (optional) -- python3 interpreter for JSON parsing (default `python3`).

The script prints `PASS`/`FAIL`/`SKIP` per check with a one-line description, a
final `RESULT: X passed, Y failed`, and exits non-zero if any check FAILs.

## Artifact hygiene

`zeus build/run/cert/wasm` and the agent loop drop artifacts
(`.c/.h/.zcert/.provenance.json/.wat/.work.*`/native binaries) into the **current
working directory** -- and `zeus wasm` writes the `.wat` next to the **source**.
To keep the repo clean, the suite copies every source it must compile into a
private `mktemp -d` scratch dir under `/tmp` and runs all file-emitting commands
from there. The scratch dir is removed on exit.

## What it covers

| # | Check | Asserts |
|---|-------|---------|
| 1 | audit v2 structured JSON (wcet) | `audit=="v2"`; `findings_structured` has `kind=="wcet_exceeded"` with integer `gap>0` and `fixable==true` (fixture: `fixtures/wcet_low.zs`) |
| 2 | audit v2 structured JSON (secret branch) | `findings_structured` has `kind=="secret_branch"`, `fixable==false` (fixture: `fixtures/secret_branch.zs`) |
| 3 | WASM round-trip | `zeus wasm showcase/wasm/math.zs` emits a `.wat`; `wasmtime --invoke neuron4 3 1 2 1` prints `12`; native `zeus run` of the same source also prints `12` |
| 4 | edge-AI build/run | `zeus build`+`run` of `showcase/edge_ai/mlp_infer.zs` prints `16` |
| 5 | edge-AI certificate | `zeus cert mlp_infer.zs` verdict shows reproducible + constant-time + bounded all **PROVEN** |
| 6 | The Lens: `multi_fn.ll` | both a `PROVED-SAFE` and a `NOT-PROVEN` function verdict; overall exit code `1` |
| 7 | The Lens: `interproc.ll` | exit `1` and a finding mentioning `into @inner` (interprocedural taint) |
| 8 | The Lens: `public_add.ll` | clean `PROVED-SAFE`; exit `0` |
| 9 | agent loop converges | `zeus_agent_loop.py repair_demo.zs` exits `0` (auto-repairs under-budget `@wcet` and ships a signed cert) |
| 10 | agent loop escalates | `zeus_agent_loop.py leak_demo.zs` exits `2` (refuses to certify a secret-dependent branch) |

## Fixtures

* `fixtures/wcet_low.zs` -- a 256-iteration loop under a deliberately tiny
  `@wcet(5)` budget. The `@wcet` attribute sits *immediately* above `fn`
  (required: an intervening attribute line breaks attachment in the parser).
* `fixtures/secret_branch.zs` -- a `@constant_time` function that branches on a
  `secret` parameter (an unmitigated, non-auto-fixable timing channel).

Fixtures are ASCII-only by design (the lexer rejects non-ASCII bytes).
