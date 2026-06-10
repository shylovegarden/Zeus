# Security Policy

## What Zeus claims (and does not)
Zeus proves **per-function, non-functional properties on its modeled subset**:
- **constant-time** — no secret-dependent branch, memory index, division, modulo, or
  shift *at the source/ZIR level*. This is a LOGICAL property; it does **not** account
  for microarchitectural channels (cache, branch prediction, speculation, SMT,
  variable-latency instructions) or physical channels (power, EM, fault injection).
- **bounded WCET / stack**, **determinism**, and **zero (libc-heap) allocation**.

A signed certificate (`.zcert`) records exactly what was proven and now binds to both
the **source** (`source_sha256`) and the **compiled binary** (`binary_sha256`).
`zeus verify-cert` checks the Ed25519 signature and the binary hash.

**Zeus is a high-assurance tool, not a formally verified compiler.** The proofs are
only as trustworthy as the Zeus compiler (Rust) and Z3. Treat verdicts as strong
evidence, not as an unconditional guarantee. See `DRAWBACKS.md` for the full register.

## Reporting a vulnerability
Please report suspected unsoundness (e.g. a program that gets PROVED-SAFE but leaks),
crashes, or signing/verification flaws privately to the maintainers (add a contact
email/GitHub Security Advisory here before public release). Include a minimal `.zs` or
`.ll` reproducer and the `zeus` version. We treat **false PROVED-SAFE** as the highest
severity.

## Supported scope
Latest `main` only during the research-preview phase.
