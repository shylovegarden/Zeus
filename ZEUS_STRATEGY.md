# Zeus: The Trust Gate for AI-Generated Code

## Thesis

The 2026 inflection point in software is not that AI writes code — it is that AI writes code faster than humans can read it. Every cloud platform has solved *containment*: how to run untrusted, machine-written code without it escaping and burning down the host. None of them have solved *trust*: how to know, mechanically and before execution, that a given function is actually safe along the dimensions that matter for security-critical and resource-critical software. Zeus closes that gap. It proves per-function non-functional properties — constant-time, bounded worst-case time and stack, determinism, zero-heap — and emits a signed, machine-checkable certificate that a deployment gate can refuse to run without. It works on Zeus's own language and, crucially, on *foreign* LLVM-IR emitted by clang from AI-generated C, C++, or Rust. Zeus does not compete with the sandboxes; it sits on top of them and sells the one thing they structurally cannot offer.

## What Zeus actually does today

Zeus is a real, working toolchain — not a research prototype or a deck. It compiles a small source language (`.zs`) to readable C and then to a native binary at the same speed as hand-written C, because it *is* C underneath. Its more interesting half is the **Lens**: a sound, multi-block taint analysis that audits foreign LLVM-IR (`.ll`) — the kind you get from `clang -emit-llvm` on AI-generated C/C++/Rust.

For each function, Zeus proves a defined envelope of non-functional properties:

- **Constant-time** — no secret-tainted value reaches a branch condition, a memory index, or a division. This is the property that defeats timing side-channels.
- **Bounded worst-case** — `@wcet` (execution time) and `@stack` bounds that actually hold, not estimates.
- **Determinism / reproducibility** — no nondeterministic source (entropy, time, uninitialized reads) influences output.
- **Zero-heap** — no dynamic allocation on the certified path.

The discipline that makes this credible: when Zeus cannot prove a property, it returns **UNDECIDABLE**, never a false "safe." Soundness over coverage. A green result means proven, full stop.

On success Zeus emits an **Ed25519-signed `.zcert` certificate** plus **SLSA v1.0 in-toto provenance**, and provides a gate — `zeus run --require ...` backed by a `zeus.policy` file — that *refuses to execute* any binary whose certificate does not prove the required properties. Audit findings also emit **SARIF 2.1.0**, so results drop straight into existing code-scanning dashboards (GitHub, GitLab, etc.) with no new UI to adopt.

## The 2026 competitor map

The honest way to read the landscape is by what each category proves and what it structurally cannot.

**Agent sandboxes — E2B/Firecracker, Modal/gVisor, Daytona, Wassette/Wasmtime.** This is the hottest category. Microsoft's Wassette (released August 2025, actively developed through 2026) gives each AI tool a deny-by-default Wasm sandbox — no filesystem, network, or env access unless granted ([Microsoft Open Source Blog](https://opensource.microsoft.com/blog/2025/08/06/introducing-wassette-webassembly-based-tools-for-ai-agents/)). E2B and Modal isolate code execution at the microVM/gVisor level. All of them *contain* untrusted code so it cannot hurt the host. **None of them can tell you the contained code is good.** A sandbox happily runs a constant-time violation, an unbounded loop, or a heap-spraying function — it just runs it in a box. Containment and correctness are orthogonal, and the entire sandbox industry lives on the containment side of that line.

**Confidential computing — Intel TDX, AMD SEV-SNP, Arm CCA.** These encrypt memory and attest the *environment* a workload runs in. They prove "this VM booted the image you expect on genuine silicon." They prove **nothing about the properties of the code inside** the enclave — a TDX-attested VM can still run timing-leaky crypto. This category is owned by Intel and AMD; competing here is suicide. (See DO NOT BUILD.)

**Supply-chain trust — SLSA, Sigstore, in-toto.** SLSA v1.0 plus Sigstore/Rekor and in-toto attestations are becoming standard in developer platforms; GitHub now ships signed provenance and SBOMs in Actions, and Red Hat Konflux issues in-toto attestations tied to policy ([InfoQ, 2025](https://www.infoq.com/news/2025/08/provenance/); [SLSA spec](https://slsa.dev/spec/v1.0/distributing-provenance)). This proves **where an artifact came from and that the build wasn't tampered with** — *provenance*, not *properties*. SLSA tells you the build pipeline was clean; it says nothing about whether the resulting function leaks secrets through a branch. Zeus is complementary here, not competitive: it *emits* SLSA provenance and carries a properties certificate as additional attested evidence.

**Verified-code research tools — FaCT, Jasmin, HACL\*, SPARK/Frama-C/Dafny, CodeQL/Semgrep.** This is the only category that touches code properties, and it splits two ways. The formal-crypto tools (FaCT, Jasmin, HACL\*) *can* prove constant-time and more — but they are expert-only, require you to write in their bespoke languages, and target a handful of cryptographic primitives. They are world-class and nearly unusable by a normal engineer. The verification tools (SPARK, Frama-C, Dafny) demand heavy annotation and PhD-adjacent effort. The scanners (CodeQL, Semgrep) are ergonomic and ubiquitous but **pattern-match for known-bad shapes** — they find *evidence of* bugs, they never *prove absence* of a property violation, and they emit no signed, gateable certificate. So: the rigorous tools aren't usable, the usable tools aren't rigorous, and **nobody proves the non-functional safety envelope at a general-purpose, ergonomic level and binds it to a signed artifact.**

## The moat, stated precisely

Zeus's defensible position is a four-part methodology, and the value is in the combination, not any single piece:

1. **Prove**, soundly, a non-functional safety envelope (constant-time, bounded WCET/stack, determinism, zero-heap) — degrading to UNDECIDABLE rather than ever lying.
2. **Sign** the result as a machine-checkable Ed25519 certificate with SLSA provenance.
3. **Gate** on it: refuse to *run* code whose certificate doesn't prove the required properties.
4. **Audit foreign LLVM-IR**, so this applies to AI-generated C/C++/Rust you didn't write — exactly the code nobody trusts.

The sandbox industry contains untrusted code; the supply-chain industry proves its origin; the confidential-computing industry attests its environment. The gap none of them fill is *proving the code itself is good, at a level a working engineer can use, and refusing to run it otherwise*. That gap is the moat. Zeus is the trust gate that sits on top of the sandbox.

## Go-to-market

**Beachhead 1 — FIPS-140-3 constant-time crypto tooling.** This uses exactly what's already built and needs no new capability. Constant-time is a hard regulatory and security requirement for crypto implementations, and today the only tools that can prove it (Jasmin, HACL\*, FaCT) are expert-only and primitive-specific. Zeus offers proof-of-constant-time as a usable product with a signed certificate auditors can verify — turning a multi-week formal-methods engagement into a CI check. Narrow, deep, well-funded, and underserved.

**Beachhead 2 — AI-code CI gating.** Ride the 2026 provenance wave. As AI writes more production code, "we contained it" stops being a sufficient answer to security review. Zeus emits SARIF into the dashboards teams already use and a gate that blocks merges or deploys of code that can't be proven safe. The pitch is *proved-absent, not pattern-found*: unlike CodeQL/Semgrep, a Zeus green is a proof, not a heuristic. This is the volume market and the wedge into every CI pipeline running AI-generated code.

**Later markets, reached by being the certifier.** Embedded and automotive (bounded WCET/stack and zero-heap are existing certification requirements) and defense (provable determinism and side-channel freedom) are natural expansions. We reach them not by rebuilding their stacks but by *becoming the tool that issues the certificate* their compliance regimes already demand.

## DO NOT BUILD — redundant with giants

These are tempting adjacencies that are already owned and would only dilute the moat:

- **A new OS or unikernel** — the unikernel thesis stalled years ago; commodity OSes win.
- **Bare-metal runtime / custom drivers** — enormous effort, zero differentiation, and orthogonal to proof.
- **Kernel-bypass networking (DPDK / AF_XDP)** — fully commoditized; nothing to add.
- **IOMMU / memory-isolation hardware tricks** — solved in silicon.
- **Confidential computing (SEV-SNP / TDX / CCA)** — owned by Intel and AMD; they attest the environment, we attest the code. Integrate, don't compete.
- **A latency/performance story** — Zeus emits C and native binaries; it is *not* faster than C and must never claim to be. Speed is not the product. Proof is.

## The honest one-liner

**Zeus is the trust gate for AI-generated code: it proves a function is constant-time, bounded, deterministic, and heap-free — or honestly says it can't — and signs a certificate your deployment gate can require. The sandboxes contain the code; Zeus is the only thing that tells you it's good.**
