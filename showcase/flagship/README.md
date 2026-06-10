# Zeus Flagship Suite — one proof engine, four audiences

Every demo here is built by the **same** Zeus compiler and emits the **same** kind of
machine-checkable certificate (`<name>.zcert`). The only thing that changes between
markets is which guarantee you put front-and-center. That is the whole thesis:
**the artifact proves itself, and you set the policy for what you'll run.**

Build any demo: `zeus build <file>.zs`   ·   Inspect its proof: `zeus cert <file>.zs`
Gate execution: `zeus run <file>.zs --require zero-heap,reproducible,constant-time,bounded`

## The four faces

| Demo | Audience | Proves | Output |
|------|----------|--------|--------|
| `crypto_sbox.zs` | **Cryptography** | `@constant_time` — secret table, no timing channel; contents wiped at scope exit | 32729 |
| `token_contract.zs` | **Smart contracts** | `@deterministic` + `@wcet(2000)` — byte-identical across nodes, gas/time bounded | 191 |
| `control_loop.zs` | **Safety-critical embedded** | `@wcet(5000)` + `@stack(4096)` — provable worst-case time & stack, zero-heap (MISRA 21.3) | 320 |
| `ai_task_good.zs` / `ai_task_bad.zs` | **AI supply-chain** | the gate runs the bounded task and **refuses** the unbounded one | 2017 / refused |

## The new approach: proof travels with the code

1. **`zeus cert <file>`** renders the certificate as a human-readable trust report
   (per-function reproducible / constant-time / WCET / stack + a verdict line).
2. **`zeus.policy`** — drop a file like:
   ```
   # org-wide proof policy
   bounded
   constant-time
   zero-heap
   ```
   and `zeus run` enforces it automatically: any binary whose certificate doesn't prove
   every listed property is **refused before execution**. Proof-as-policy, supply-chain ready.

## Honest scope (v0.x)

- WCET is a sound bound in abstract **steps**, not nanoseconds yet (calibratable per CPU).
- Stack is a conservative estimate; WCET is the rigorous `Option` bound (UNBOUNDED = refused).
- The certificate is **content-hashed (SHA-256)**, not yet cryptographically *signed* (integrity, not provenance — signing is the next hardening step).
- The analyses are **sound but conservative**: never a false "safe," occasionally strict.
- Known compiler bug: non-ASCII bytes (e.g. an em-dash in a comment) currently crash the lexer — keep source ASCII for now.
