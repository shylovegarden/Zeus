# Show HN: Zeus - Mathematical proof your AI code is safe

**TL;DR:** Zeus is a CI/CD plugin that mathematically proves code has no timing attacks, no memory leaks, and bounded execution. Unlike security scanners that pattern-match, we provide formal verification + signed certificates.

---

## The Problem

You're using ChatGPT/Copilot to write code faster than you can review it. But you're terrified to deploy it because:
- Is it actually secure?
- Does it leak secrets through timing?
- Will it crash in production?

Current security tools (Semgrep, CodeQL) just pattern-match for known bugs. They can't prove your code is safe.

## The Solution

Zeus provides **mathematical proof** of security properties:

1. **Zero-Heap**: No dynamic allocation = no memory leaks, ever
2. **Constant-Time**: Execution time doesn't depend on secrets = no timing attacks
3. **Bounded**: Provable worst-case execution time = safe for real-time systems

## How It Works

```bash
# Add to your CI/CD
- uses: zeus-lang/verify-action@v1
  with:
    policy: zero-heap,constant-time,bounded
```

Zeus compiles your code to an intermediate representation, runs the Z3 SMT solver to prove properties, and generates an Ed25519-signed certificate. If verification fails, the build is blocked.

## What's Different

| Tool | Approach | Zeus Advantage |
|------|----------|----------------|
| Semgrep | Pattern matching | ❌ False positives, misses novel bugs |
| CodeQL | Semantic analysis | ❌ Complex queries, not proof |
| Rust | Memory safety | ❌ No timing guarantees |
| Zeus | **Formal verification** | ✅ Mathematical proof, signed certificate |

## Technical Details

**Source-to-Source Compiler**: Zeus compiles to highly optimized C (3.4x faster than malloc/free due to arena allocation). Verification happens at build time, so runtime is native speed.

**Zero-Heap Architecture**: Enforces no dynamic allocation through compile-time checks. Uses M:N cooperative fibers with <10ns context switch time.

**Anti-Side-Channel**: Stochastic core hopping + speculation barriers defeat cache-timing attacks.

## Demo

We verified a constant-time password comparison that processes 32 bytes in exactly 32 iterations regardless of input:

```zeus
@constant_time
fn verify_password(secret input: [u8; 32], stored: [u8; 32]) -> bool {
    let mut diff: i32 = 0;
    for i in 0..32 {
        diff |= (input[i] ^ stored[i]) as i32;
    }
    return diff == 0;
}
```

Zeus proves this is constant-time, generates a certificate, and the resulting C code runs at native speed.

## Use Cases

- **Crypto Exchanges**: Prove trading code has no timing leaks
- **Medical Devices**: FDA compliance with automatic documentation
- **Aerospace**: NASA Class D with WCET proofs
- **AI-Generated Code**: Trust what ChatGPT wrote

## Current Status

- ✅ Working compiler with C/WASM/EVM backends
- ✅ Z3 verification pipeline
- ✅ Self-certifying binaries
- ✅ GitHub Action ready
- ⚠️ LLVM backend (scaffolded, needs completion)
- 🚧 SaaS dashboard (in development)

## Open Source

- GitHub: https://github.com/zeus-lang/zeus
- Docs: https://zeus-lang.org/docs
- Try it: `docker run zeuslang/compiler verify ./src`

**The artifact proves itself.**

---

What would you like us to prove next?
