# Zeus -> WebAssembly ("The Reach")

`zeus wasm <file.zs>` lowers the **verifiable integer/control-flow subset** of Zeus
to WebAssembly text (`.wat`) so the same code runs in any WASM runtime -- Wasmtime,
Node, browsers, and edge/agent sandboxes like Microsoft **Wassette**. This is the
deployment bridge: a Zeus function that has been *proven* bounded/deterministic can
be shipped as a portable, sandboxable Wasm module.

## Honest scope
Lowered today: functions over integer/bool values (as `i32`), `let`/assignment
locals, arithmetic + comparison, `if/else`, **constant-bounded `for` loops**,
`return`, and calls between defined functions. A function is exported only if it
**and all its callees** are in the subset -- otherwise it is listed as `skipped`
with a reason. The emitter never produces an invalid module or silently-wrong code.
Out of scope (skipped): structs/SoA arrays, tensors, `secret`, `while`, floats,
`parallel`, FFI, and `println` (needs a host import).

## Build + run
```
zeus wasm showcase/wasm/math.zs            # -> showcase/wasm/math.wat
wasmtime run --invoke neuron4 showcase/wasm/math.wat 3 1 2 1
```

## Verified cross-checked output (native Zeus vs. Wasmtime)
| call | native Zeus | Wasmtime (.wat) |
|------|-------------|-----------------|
| `neuron4(3,1,2,1)` | 12 | **12** |
| `relu(12)` / `relu(-5)` | 12 / 0 | **12 / 0** |
| `clampv(50,0,10)` | 10 | **10** |
| `ramp(0)` (sum 0..9) | 45 | **45** |

Same results in both -- the loop, branch, and call lowering are correct. (`main`
is skipped because `println` requires a host import; invoke the pure functions
directly, which is exactly how a Wasm host/agent-sandbox calls an exported tool.)
