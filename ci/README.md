# Zeus Audit Gate — a trust gate for AI-generated code

Large models write a lot of code. Most of it is fine; some of it leaks secrets
through timing side channels in ways a human reviewer will not spot in a diff.
The Zeus Audit Gate puts a **proof obligation** between "an agent wrote this" and
"this runs in production":

```
agent writes C  ->  clang -emit-llvm  ->  zeus audit  ->  SARIF + pass/fail gate
                                                              |
                                            only PROVED-SAFE code merges / ships
```

Zeus's "Lens" ingests the LLVM-IR that `clang` emits and computes secret-taint
with a **monotone fixpoint** over every instruction — sound through `phi` nodes
and across loop back-edges. For each function it returns one of three verdicts:

| Verdict       | Meaning                                                              | Gate (default) | Gate (`--strict`) |
|---------------|---------------------------------------------------------------------|:--------------:|:-----------------:|
| `PROVED-SAFE` | No secret-dependent branch, memory index, or division — on the modeled subset. | PASS | PASS |
| `UNDECIDABLE` | Outside the modeled subset (loop it can't bound, unknown opcode, aliasing). **Never reported as safe.** | PASS (with caveats) | **FAIL** |
| `NOT-PROVEN`  | A real finding: a secret value reaches a branch, an index, a division, or a public return. | **FAIL** | **FAIL** |

**Honesty note.** The Lens models a *subset* of LLVM-IR. Anything it cannot fully
reason about is `UNDECIDABLE` — it is never silently promoted to `PROVED-SAFE`.
A green gate means "proven safe on the modeled subset," not "proven safe for all
possible behaviors." Use `--strict` when you want UNDECIDABLE to block the build.

---

## What's in this folder

| File | Purpose |
|------|---------|
| `zeus-audit.sh` | Portable POSIX wrapper. Audits a list of `.ll` files or a directory, merges per-file SARIF into one report, prints a summary table, exits non-zero on any failure. |
| `action.yml` | Composite GitHub Action ("Zeus Audit Gate"). Inputs: `path`, `strict`, `sarif-out`. Delegates to `zeus-audit.sh`. |
| `example-workflow.yml` | Ready-to-paste workflow: `clang -emit-llvm` → Zeus Audit Gate → upload SARIF to code scanning. |
| `README.md` | This file. |

---

## Installing the zeus binary

The gate assumes `zeus` (or `zeus_compiler`) is already on `PATH`. Provide it in
CI any way you like; common options:

```sh
# (a) Download a pinned release artifact:
curl -sSL -o /usr/local/bin/zeus \
  https://example.com/zeus/releases/v0.1.0/zeus-linux-x86_64
chmod +x /usr/local/bin/zeus

# (b) Build from source:
cargo build --release --manifest-path zeus_compiler/Cargo.toml
sudo install -m 0755 target/release/zeus_compiler /usr/local/bin/zeus
```

The wrapper looks for `$ZEUS_BIN`, then `zeus`, then `zeus_compiler` on `PATH`.
You can also point it explicitly: `zeus-audit.sh --bin /path/to/zeus_compiler ...`.

---

## Quick start (local)

Audit a directory of `.ll` files and fail on any unproven function:

```sh
# point the wrapper at your zeus binary (or put it on PATH)
export ZEUS_BIN=/path/to/zeus_compiler

# audit every .ll under showcase/llvm/, write merged SARIF, gate the result
sh ci/zeus-audit.sh --sarif-out zeus-audit.sarif showcase/llvm/
echo "exit code: $?"     # 0 = all passed, 1 = at least one failed
```

Add `--strict` to also fail on `UNDECIDABLE`:

```sh
sh ci/zeus-audit.sh --strict --sarif-out zeus-audit.sarif showcase/llvm/
```

---

## Verified local test (real output)

These are the exact commands run against `showcase/llvm/` and their **real**
output. `clang -emit-llvm` produces exactly this shape of IR for real C, so this
is the same path AI-generated C takes in CI.

### 1. Audit the whole showcase directory (default mode)

```
$ export ZEUS_BIN=/path/to/zeus_compiler
$ sh ci/zeus-audit.sh --sarif-out zeus-audit.sarif showcase/llvm/

================================ ZEUS AUDIT GATE ================================
mode: default (UNDECIDABLE passes with caveats)
binary: /tmp/zeus_target/release/zeus_compiler
--------------------------------------------------------------------------------
FILE                                      VERDICT       FINDINGS  GATE
--------------------------------------------------------------------------------
showcase/llvm/counter.ll                  UNDECIDABLE   1         PASS
showcase/llvm/ct_mix.ll                   NOT-PROVEN    1         FAIL
showcase/llvm/insecure_cmp.ll             NOT-PROVEN    1         FAIL
showcase/llvm/leaky.ll                    NOT-PROVEN    1         FAIL
showcase/llvm/public_add.ll               PROVED-SAFE   0         PASS
showcase/llvm/route_pub.ll                PROVED-SAFE   0         PASS
showcase/llvm/sbox.ll                     NOT-PROVEN    2         FAIL
--------------------------------------------------------------------------------
totals: 7 file(s)  |  PROVED-SAFE 2  UNDECIDABLE 1  NOT-PROVEN 4
merged SARIF -> zeus-audit.sarif
--------------------------------------------------------------------------------
[ZEUS AUDIT GATE] FAILED -- at least one file did not pass.
================================================================================

$ echo $?
1
```

The build correctly fails: four functions leak a secret through a branch, a
memory index, or a public return.

### 2. The leaky function fails the gate

```
$ sh ci/zeus-audit.sh --no-sarif showcase/llvm/leaky.ll

================================ ZEUS AUDIT GATE ================================
mode: default (UNDECIDABLE passes with caveats)
binary: /tmp/zeus_target/release/zeus_compiler
--------------------------------------------------------------------------------
FILE                                      VERDICT       FINDINGS  GATE
--------------------------------------------------------------------------------
showcase/llvm/leaky.ll                    NOT-PROVEN    1         FAIL
--------------------------------------------------------------------------------
totals: 1 file(s)  |  PROVED-SAFE 0  UNDECIDABLE 0  NOT-PROVEN 1
--------------------------------------------------------------------------------
[ZEUS AUDIT GATE] FAILED -- at least one file did not pass.
================================================================================

$ echo $?
1
```

### 3. The proven-safe function passes the gate

```
$ sh ci/zeus-audit.sh --no-sarif showcase/llvm/public_add.ll

================================ ZEUS AUDIT GATE ================================
mode: default (UNDECIDABLE passes with caveats)
binary: /tmp/zeus_target/release/zeus_compiler
--------------------------------------------------------------------------------
FILE                                      VERDICT       FINDINGS  GATE
--------------------------------------------------------------------------------
showcase/llvm/public_add.ll               PROVED-SAFE   0         PASS
--------------------------------------------------------------------------------
totals: 1 file(s)  |  PROVED-SAFE 1  UNDECIDABLE 0  NOT-PROVEN 0
--------------------------------------------------------------------------------
[ZEUS AUDIT GATE] PASSED -- all 1 file(s) cleared the gate.
================================================================================

$ echo $?
0
```

### 4. `--strict` blocks UNDECIDABLE too

In default mode `counter.ll` (an unbounded loop back-edge) is `UNDECIDABLE` and
PASSes with caveats. Under `--strict` it FAILs:

```
$ sh ci/zeus-audit.sh --strict --no-sarif showcase/llvm/counter.ll

...
showcase/llvm/counter.ll                  UNDECIDABLE   1         FAIL
...
[ZEUS AUDIT GATE] FAILED -- at least one file did not pass.

$ echo $?
1
```

### Merged SARIF (the findings that tripped the gate)

`zeus-audit.sh` merges every file's `zeus audit --sarif` output into one valid
SARIF 2.1.0 document. From the run in step 1 (`jq` over `zeus-audit.sarif`):

```json
{ "ruleId": "ZEUS-UNDECIDABLE",   "level": "note",  "text": "fn counter: analysis UNDECIDABLE (loop / unknown opcode / unresolved aliasing)." }
{ "ruleId": "ZEUS-SECRET-RETURN", "level": "error", "text": "fn ct_mix: returns a secret-tainted value to a public caller" }
{ "ruleId": "ZEUS-SECRET-BRANCH", "level": "error", "text": "fn insecure_cmp: secret value used as branch condition -> control-flow timing channel [UNMITIGATED]" }
{ "ruleId": "ZEUS-SECRET-BRANCH", "level": "error", "text": "fn leaky: secret value used as branch condition -> control-flow timing channel [UNMITIGATED]" }
{ "ruleId": "ZEUS-SECRET-INDEX",  "level": "error", "text": "fn sbox_lookup: secret value used as memory index -> cache-timing channel [UNMITIGATED]" }
{ "ruleId": "ZEUS-SECRET-RETURN", "level": "error", "text": "fn sbox_lookup: returns a secret-tainted value to a public caller" }
```

Six results total: one `note` (UNDECIDABLE) and five `error`s (the real leaks).
`PROVED-SAFE` functions produce no SARIF results — a clean file is a quiet file.

---

## Using it in GitHub Actions

The composite action lives in `ci/action.yml`. Reference it with `uses: ./ci`
after checkout (the action and the wrapper must sit together in `ci/`):

```yaml
- name: Zeus Audit Gate
  uses: ./ci
  with:
    path: build/ir        # file, dir, or space-separated list of .ll files
    strict: "false"       # "true" to also fail on UNDECIDABLE
    sarif-out: zeus-audit.sarif

- name: Upload SARIF to code scanning
  if: always()
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: zeus-audit.sarif
    category: zeus-audit
```

See `ci/example-workflow.yml` for a complete, paste-ready pipeline that compiles
`src/ai-generated/*.c` with `clang -O1 -S -emit-llvm`, runs the gate, and uploads
the SARIF so findings appear in the repository's **Security** tab.

---

## Notes on the real binary's behavior (verified)

The wrapper is built around the **observed** behavior of `zeus_compiler audit`,
not assumptions:

- `zeus audit <file.ll> --sarif` writes SARIF 2.1.0 to **stdout** (the human
  banner is suppressed) and sets the exit code to the gate verdict.
- There is **no `--sarif-out` flag** on the binary; the wrapper captures stdout
  itself and merges the per-file documents.
- **Flag order matters:** `--strict` must come *before* `--sarif`. The wrapper
  always invokes `zeus audit <file> --strict --sarif` accordingly.
- Exit code is the source of truth: `1` on `NOT-PROVEN` (and on `UNDECIDABLE`
  under `--strict`), `0` otherwise. The wrapper never overrides it.
