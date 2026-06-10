# Zeus Attested Run — SOFTWARE SIMULATION of Machine-Bound Execution

> **Honesty notice (read first).** This directory is a **software simulation**
> of a hardware-bound execution attestation ("PUF/TPM-style binding"). It is a
> **wrapper** around the existing `zeus` binary — it does **not** modify the
> compiler. Its purpose is to demonstrate the **policy hook**: *refuse to run
> unless the binary is bound to THIS machine's token.*
>
> The "machine token" here is derived **in software** (a SHA-256 of
> `/etc/machine-id`, or of `hostname` + a local salt file). It is **NOT** read
> from silicon, it is **NOT** unclonable, and it provides **NO** hardware
> security. A real implementation requires actual hardware — a **TPM 2.0 quote**
> over PCRs, or an **SRAM-PUF challenge-response** from the device. See
> [Limitations](#limitations).

---

## The trust chain

Zeus already gives you the first two links. This wrapper adds the third
(simulated) link:

```
  1. PROOF          code is formally proven (zero-heap, reproducible,
                    constant-time, bounded WCET) at build time
        |
        v
  2. SIGNATURE      those proofs are recorded in a certificate (<stem>.zcert)
                    that is Ed25519-signed; `zeus verify-cert` checks it
        |
        v
  3. MACHINE        ...AND this binary is only allowed to run on a machine
     BINDING        whose token matches the one it was bound to.
     (SIMULATED)    <-- this layer is what attest/ adds, in software
```

Links **1** and **2** are real Zeus behavior. Link **3** is a **simulation** of
what a TPM/PUF would enforce in hardware.

---

## What is REAL vs. what is SIMULATED

| Aspect | Status | Notes |
|---|---|---|
| Property proofs in the certificate | **REAL** | Produced by `zeus build`. |
| Ed25519 signature on the certificate | **REAL** | Checked by `zeus verify-cert`; a tampered cert is rejected. |
| Property gate before execution | **REAL** | `zeus run --require=...` refuses to run if a required property is not proven. |
| The wrapper refusing to run on a non-matching token | **REAL** | The script exits non-zero (code 4) and never invokes the program. |
| The "machine token" itself | **SIMULATED** | SHA-256 of `/etc/machine-id` (or `hostname`+salt). Software-derived, **clonable**, **not** silicon-bound. |
| "Inert on other silicon" | **SIMULATED** | We *model* this by comparing a software token; we do not cryptographically bind the binary to hardware. |

### Threats this DOES help with (in the simulation)

- An operator accidentally deploying the binary to the **wrong host**: the
  wrapper refuses to run there (token mismatch).
- Demonstrating and testing the **policy/control flow** of a future real
  attested-execution system, end to end, against the real Zeus toolchain.

### Threats this does NOT stop

- A motivated attacker who can **read `/etc/machine-id`** (or the salt file) and
  **replay** the token on another machine — the token is not a secret bound to
  silicon.
- Anyone who simply **runs the proven binary directly** (the `./mlp_infer` that
  `zeus build` drops) without going through this wrapper. The wrapper is a
  *gate you choose to put in front of execution*, not a property of the binary.
- Any hardware-level attack, cloning, or extraction. **No hardware root of
  trust is involved.** For that you need a TPM quote or a real PUF.

---

## Usage

```sh
# Set the path to the zeus binary (or put `zeus` on your PATH).
export ZEUS_BIN=/path/to/zeus_compiler

# 1) Capture THIS machine's simulated token:
attest/zeus-attested-run.sh --show-token

# 2) Run a module, bound to that token, gated on proven properties:
attest/zeus-attested-run.sh <file.zs> \
    --bind <expected_token> \
    --require=zero-heap,reproducible,constant-time,bounded
```

**Exit codes**

| Code | Meaning |
|---|---|
| 0 | Attested **and** ran (build ok, cert valid, token matched, gate passed). |
| 1 | Usage / environment error. |
| 2 | Build failed. |
| 3 | Certificate verification failed (bad signature/hash). |
| 4 | **ATTESTATION FAILED** — machine token mismatch (refused). |
| 5 | Policy gate refused execution (a required property was not proven). |

The wrapper performs, in order: **(a)** build the module in a `/tmp` scratch dir
to obtain the signed `.zcert`; **(b)** `zeus verify-cert` it; **(c)** compute
this machine's simulated token and compare to `--bind`; **(d)** if all pass,
`zeus run --require=...` to gate on properties and execute.

---

## Worked example (real captured output)

The demo module is `showcase/edge_ai/mlp_infer.zs`, which builds and proves all
four properties. The output below was captured on this host
(`/etc/machine-id` present).

### (i) Get this machine's simulated token

```
$ attest/zeus-attested-run.sh --show-token
[ NOTE ] This is a SIMULATED, software-derived machine token (not silicon-bound).
8e525279af9264f8d017d4f552a7a6e5ab3e650377ff0f045c6a149d947df54e
```

### (ii) Run with the CORRECT token — succeeds (exit 0)

```
$ attest/zeus-attested-run.sh showcase/edge_ai/mlp_infer.zs \
      --bind 8e525279af9264f8d017d4f552a7a6e5ab3e650377ff0f045c6a149d947df54e \
      --require=zero-heap,reproducible,constant-time,bounded

=== Zeus Attested Run (SOFTWARE SIMULATION) ===
Trust chain:  proof  ->  signed certificate  ->  machine binding
Module:       .../showcase/edge_ai/mlp_infer.zs

[ .... ] Step 1/4: building module to produce a signed certificate...
[ PASS ] build succeeded; certificate emitted: mlp_infer.zcert
[ .... ] Step 2/4: verifying Ed25519 certificate signature...
[ PASS ] certificate signature valid (proof chain intact)
[ .... ] Step 3/4: checking SIMULATED machine binding...
        expected (--bind): 8e525279af9264f8d017d4f552a7a6e5ab3e650377ff0f045c6a149d947df54e
        this machine:      8e525279af9264f8d017d4f552a7a6e5ab3e650377ff0f045c6a149d947df54e
[ PASS ] machine token matches --bind (simulated binding satisfied)
[ .... ] Step 4/4: gating on proven properties and executing...
        gate: zeus run --require=zero-heap,reproducible,constant-time,bounded
 ... (zeus build/verify pipeline output) ...
 📜 Certificate: mlp_infer.zcert  [sha256 + per-fn reproducible/constant_time/wcet/stack]
[ZEUS POLICY GATE] certificate satisfies [zero-heap, reproducible, constant-time, bounded] — executing.
16

[ PASS ] ATTESTED: proof verified, machine binding satisfied, policy gate passed, program executed.

$ echo $?
0
```

The `16` is the program's own output; the trailing exit code `0` confirms it
was attested and ran.

### (iii) Run with a WRONG token (`--bind deadbeef`) — refused (exit 4)

```
$ attest/zeus-attested-run.sh showcase/edge_ai/mlp_infer.zs \
      --bind deadbeef \
      --require=zero-heap,reproducible,constant-time,bounded

=== Zeus Attested Run (SOFTWARE SIMULATION) ===
Trust chain:  proof  ->  signed certificate  ->  machine binding
Module:       .../showcase/edge_ai/mlp_infer.zs

[ .... ] Step 1/4: building module to produce a signed certificate...
[ PASS ] build succeeded; certificate emitted: mlp_infer.zcert
[ .... ] Step 2/4: verifying Ed25519 certificate signature...
[ PASS ] certificate signature valid (proof chain intact)
[ .... ] Step 3/4: checking SIMULATED machine binding...
        expected (--bind): deadbeef
        this machine:      8e525279af9264f8d017d4f552a7a6e5ab3e650377ff0f045c6a149d947df54e
[ FAIL ] ATTESTATION FAILED: binary is bound to a different machine
[ NOTE ] Refusing to run. (Simulated 'inert on other silicon'.)
[ NOTE ] This is a software check; a real PUF/TPM binding needs hardware.

$ echo $?
4
```

The program (`16`) is **never** printed: the wrapper refused before reaching the
execution step. This is the simulated "inert on other silicon" behavior.

---

## How the token is computed

```
token = sha256( "machine-id:" + contents_of(/etc/machine-id) )
```

or, if `/etc/machine-id` is missing/empty:

```
token = sha256( "hostname:" + hostname + ":salt:" + contents_of(attest/.attest_salt) )
```

`attest/.attest_salt` is created once (32 random bytes from `/dev/urandom` when
available) so the fallback token is stable across runs on the same host. **This
salt is a local file, not a hardware secret** — copying it copies the identity.

---

## Limitations

- **Not unclonable, not silicon-bound.** The token is plain software-derived
  data. Reading `/etc/machine-id` (or the salt file) and reusing it on another
  machine reproduces the token. This is the fundamental difference from a real
  PUF/TPM.
- **No hardware root of trust.** There is no TPM quote, no PCR measurement, no
  PUF challenge-response. Nothing here resists a hardware-capable attacker.
- **The gate is opt-in.** The binary produced by `zeus build` can be executed
  directly, bypassing this wrapper entirely. The wrapper enforces policy only
  for callers who go through it.
- **Token instability.** If `/etc/machine-id` changes (e.g. an image is
  re-provisioned) or the salt file is deleted, the token changes and previously
  valid `--bind` values stop matching. That is expected for an identifier, but
  it is not a cryptographic guarantee.
- **What a real version would need.** Replace step (c) with an actual hardware
  attestation: e.g. a TPM 2.0 `quote` over a known PCR set verified against the
  device's endorsement key, or an SRAM-PUF challenge whose response is checked
  against an enrolled helper-data record. Only then is "inert on other silicon"
  a real property rather than a demonstration of the policy hook.
