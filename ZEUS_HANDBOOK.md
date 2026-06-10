# Zeus Handbook — everything Zeus can do, and exactly how to run it

A plain-English guide for driving the whole toolchain. Zeus is a small systems
language whose compiler **proves things about your code** (no memory bloat, no
secret leaks, bounded run-time) and **refuses to certify** code it can't prove
safe. You write `.zs` files; Zeus turns them into fast native programs **and** a
signed certificate of what was proven.

> First time? Build the compiler once on your machine: in `zeus_compiler/` run
> `cargo build --release`. The binary is `target/release/zeus_compiler` — call it
> `zeus` below. (In this repo's CI sandbox it lives at
> `/tmp/zeus_target/release/zeus_compiler`.)

---

## The 60-second tour

```
zeus build  hello.zs        # compile -> native binary + signed hello.zcert
./hello                     # run it
zeus audit  hello.zs --json # machine-readable safety report (for tools/AI)
zeus cert   hello.zs        # human-readable "what did we prove?" report
zeus run    hello.zs --require=zero-heap,bounded   # only runs if proven
zeus wasm   hello.zs        # also emit WebAssembly (hello.wat)
zeus audit  foo.ll          # audit non-Zeus code (C/C++/Rust via LLVM IR)
```

---

## Commands, one by one

### `zeus build <file.zs>`
Compiles to a native binary and emits a **signed certificate** next to it.
```
zeus build examples/ex01_hello.zs
./ex01_hello            # -> 42
```
You also get `ex01_hello.zcert` (the proof certificate) and
`ex01_hello.provenance.json` (SLSA supply-chain record). Builds use
`-O2 -march=native`, so your code is genuinely fast.

### `zeus run <file.zs> --require=<props>`  (the policy gate)
Builds, then **refuses to run** unless the certificate proves the properties you
demand. Note the **`=`** sign.
```
zeus run examples/ex08_tiny_net.zs --require=zero-heap,reproducible,constant-time,bounded
# -> [ZEUS POLICY GATE] certificate satisfies [...] — executing.
#    12
```
Properties you can require: `zero-heap`, `reproducible`, `constant-time`, `bounded`.
You can also drop a `zeus.policy` file (one property per line) next to your code and
`zeus run` enforces it automatically with no flag.

### `zeus audit <file.zs> --json`  (machine-first safety report)
The report an automated tool or AI agent reads. Output is `audit:"v2"` with two
things: a `findings` list (human strings) and a **`findings_structured`** list of
typed records:
```json
{ "function":"accumulate", "kind":"wcet_exceeded", "fixable":true,
  "observed_steps":1543, "budget_steps":5, "gap":1538,
  "suggested_action":"set @wcet(accumulate) >= 1543" }
```
`kind` classifies the problem (`wcet_exceeded`, `stack_exceeded`, `unbounded_wcet`,
`secret_branch`, `secret_index`, `secret_division`, `secret_return`); `gap` is the
**distance to a valid proof**. `fixable:false` = a real logic/timing leak (a human
or agent must change the logic, not just a budget).

### `zeus audit <file.ll>`  ("The Lens" — audit other people's code)
Point Zeus at LLVM IR produced from C/C++/Rust (`clang -emit-llvm -S foo.c`).
It runs the same secret-leak analysis across all functions and call boundaries.
```
zeus audit showcase/llvm/public_add.ll     # -> PROVED-SAFE (exit 0)
zeus audit showcase/llvm/sbox.ll           # -> NOT-PROVEN (secret-indexed lookup, exit 1)
zeus audit showcase/llvm/leaky.ll --sarif  # emit SARIF 2.1.0 for GitHub code-scanning
zeus audit showcase/llvm/counter.ll --strict   # UNDECIDABLE also fails the build
```
Verdicts: **PROVED-SAFE** / **NOT-PROVEN** / **UNDECIDABLE** (it never claims safe
on code it can't fully model).

### `zeus cert <file.zs>`  (human-readable trust report)
```
zeus cert examples/ex08_tiny_net.zs
# -> per-function reproducible / constant-time / WCET / stack, signature check,
#    and a Verdict line (PROVEN / NOT-PROVEN per property).
```

### `zeus verify-cert <file.zcert>` / `zeus verify-provenance <file.provenance.json>`
Re-check a certificate or provenance file's Ed25519 signature (fails on tamper).
```
zeus verify-cert ex08_tiny_net.zcert
```

### `zeus wasm <file.zs>`  (run anywhere — browser, edge, agent sandboxes)
Emits WebAssembly text (`.wat`) for the integer/control-flow subset. Functions
outside the subset (arrays/structs/float/println) are listed as `skipped`, never
mis-compiled.
```
zeus wasm showcase/wasm/math.zs            # -> showcase/wasm/math.wat (next to the source)
wasmtime run --invoke neuron4 showcase/wasm/math.wat 3 1 2 1   # -> 12
```

### `zeus import <header.h>`  (drop into existing C systems)
Reads a C header and generates Zeus `extern fn` bindings so you can call legacy C.
```
zeus import showcase/engine.h
```

---

## Demos you can run right now

| Folder | What it shows | Try it |
|--------|---------------|--------|
| `examples/` | Bite-size, verified programs (one per feature) | `zeus build examples/ex01_hello.zs && ./ex01_hello` |
| `showcase/flagship/` | One demo per market (crypto, contract, embedded, AI task) | `zeus cert showcase/flagship/control_loop.zs` |
| `showcase/edge_ai/` | Fixed-weight neural net, **certified** bounded+deterministic | `zeus build showcase/edge_ai/mlp_infer.zs && ./mlp_infer` (-> 16) |
| `showcase/wasm/` | Zeus -> WebAssembly, run in Wasmtime | see `zeus wasm` above |
| `showcase/llvm/` | The Lens auditing non-Zeus LLVM IR | `zeus audit showcase/llvm/multi_fn.ll` |
| `showcase/agent_loop/` | **AI fixes its own code** until Zeus certifies it | `python3 showcase/agent_loop/zeus_agent_loop.py showcase/agent_loop/repair_demo.zs` |
| `ci/` | GitHub Action + script that gates a build on `zeus audit` | `bash ci/zeus-audit.sh showcase/llvm/` |
| `attest/` | Simulated machine-binding on top of the cert (honest PUF stand-in) | `bash attest/zeus-attested-run.sh --show-token` |
| `tests/feature/` | Automated checks for all the above | `bash tests/feature/run_feature_tests.sh` |

---

## Gotchas (save yourself an hour)

- **Attributes stack freely.** `@wcet(...)`, `@stack(...)`, `@constant_time`,
  `@deterministic`, etc. can be listed in any order above a `fn` and all attach.
  A misspelled/unknown attribute (e.g. `@wceet(5)`) now **fails the build loudly**
  with a clear message instead of being silently ignored.
- **ASCII is safest.** Non-ASCII bytes (em-dash, smart quotes) are now handled
  cleanly (a clear error, never a crash), but plain ASCII `-`/`"` avoids surprises.
- **Don't start a file name with a digit.** `01_foo.zs` fails to build (the
  generated C header guard becomes an invalid macro). Use `ex01_foo.zs`.
- **Don't name a function `double`** (or other C type names) — it collides in the
  generated C bridge.
- **`--require` needs the `=`**: `--require=bounded`, not `--require bounded`.
- **Integer behavior needs explicit types.** `let x: i32 = 3;` is an integer; a
  bare `3` is a float. Type your loop counters and budgets.
- **Calls are arity-checked.** Calling a function with the wrong number of
  arguments now fails with a clean Zeus error (e.g. "call to 'add' has 3
  argument(s) but it is defined with 2"), not an obscure C error.

---

## What Zeus does and does NOT claim (read this)

- Zeus does **not** make code safe. It **refuses to certify** code it cannot prove
  safe within its modeled subset — and the AI repair loop escalates a real leak to
  a human rather than papering over it.
- WCET ("worst-case execution time") is reported in **abstract steps**, a sound
  relative bound — calibrate to nanoseconds per target CPU before quoting real time.
- The certificate proves the **properties listed**, signed with Ed25519 by a
  **persistent identity**: the keypair lives in one stable place (`$ZEUS_KEY_DIR`,
  else `$HOME/.zeus`) and is reused across builds, so a cert verifies on any
  machine/dir. `verify-cert` always checks the signature against the cert's embedded
  key; to enforce a *specific* trusted signer, set `ZEUS_TRUSTED_PUB` (hex or a
  path to a `.pub`) and a mismatch hard-fails. Override the seed with
  `ZEUS_SIGNING_KEY` (32-byte hex) for reproducible/CI signing.
- Zeus is a **high-assurance tool**, not a formally verified compiler (the
  CompCert/Jasmin tier). It trusts its own Rust toolchain + the Z3 prover.

That honesty is the point: every claim above is something you can run and check.
