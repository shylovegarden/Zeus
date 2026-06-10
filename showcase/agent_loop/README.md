# The AI <-> Zeus Self-Repair Loop (the "human-free CI loop")

This is the workflow that makes Zeus the **automated referee for AI-generated code**.
An autonomous agent submits a module; Zeus audits it and returns machine-readable
JSON diagnostics; the agent reads the exact findings, edits the source, and
resubmits -- looping until Zeus **proves** the code safe and emits an
Ed25519-signed certificate. No human reviews the code.

```
agent writes code
      |
      v
  zeus audit --json  --->  findings? --no-->  zeus build  -->  signed .zcert  (SHIP)
      ^                        |
      |                       yes
      +---- agent edits <------+   (or ESCALATE if no safe fix exists)
```

## Run it
```
# converges to a signed certificate (raises under-budget @wcet then @stack)
python3 zeus_agent_loop.py repair_demo.zs

# correctly REFUSES to certify a real timing leak and escalates
python3 zeus_agent_loop.py leak_demo.zs
```

## Verified output (this machine)
**Converging case** -- 3 iterations, no human (now driven by *structured* diagnostics):
```
[iter 1] accumulate:NOT-PROVEN   [wcet_exceeded] accumulate   distance-to-proof: 1538
         agent edit: raise @wcet(accumulate) -> 1543 (closes proof gap of 1538 steps)
[iter 2] accumulate:NOT-PROVEN   [stack_exceeded] accumulate  distance-to-proof: 80
         agent edit: raise @stack(accumulate) -> 88 (closes proof gap of 80 bytes)
[iter 3] accumulate:PROVED-SAFE  -> CERTIFIED (Ed25519 signature valid)
```
**Escalation case** -- the loop will not certify a leak:
```
[iter 1] pin_ok:NOT-PROVEN   secret value used as branch condition (timing channel)
         ESCALATE TO HUMAN -- no safe automated fix.   exit 2
```

## Structured diagnostics (machine-first)
`zeus audit --json` emits `audit:"v2"` with a **`findings_structured`** array -- typed
records an agent consumes without scraping prose:
```json
{ "function":"accumulate", "kind":"wcet_exceeded", "fixable":true,
  "observed_steps":1543, "budget_steps":5, "gap":1538,
  "suggested_action":"set @wcet(accumulate) >= 1543" }
```
`kind` classifies the failure (`wcet_exceeded`, `stack_exceeded`, `unbounded_wcet`,
`secret_branch`, `secret_index`, `secret_division`, `secret_return`); `gap` is the
**distance to a valid proof** (how far the budget is exceeded). `fixable:false`
findings (real logic/timing leaks) are exactly the ones the loop escalates. The
legacy `findings` string array is still emitted for backward compatibility.

## Plugging in a real LLM
The "agent" is a deterministic stub today (`propose_fix`) so the demo is
reproducible offline. A real LLM agent implements the **same interface**:
```python
propose_fix(findings: list[str], source: str) -> (new_source | None, action)
```
It returns `None` when it cannot produce a safe fix, so the loop escalates instead
of certifying. The orchestration, the JSON diagnostics, the proof, and the signed
certificate are all real and unchanged.

## Honest scope
Zeus does **not** "make code safe." It **refuses to certify** unsafe code, and the
loop cannot patch over a genuine logic/timing leak (it escalates). The two
auto-fixes shown are real source edits for *resource-budget* findings (raise the
proven `@wcet`/`@stack`); a logic flaw is exactly what gets escalated. Soundness
holds within Zeus's modeled subset and trusts the compiler + Z3 base -- this is a
high-assurance gate, not a verified compiler (CompCert/Jasmin tier).
