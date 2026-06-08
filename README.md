# ZEUS Architecture
**The Zero-Bloat, Bare-Metal AI Systems Engine**

Zeus is an uncompromising compiler and systems programming language designed to run AI models and concurrent software at the absolute physical limits of hardware. It is built to completely bypass the legacy bottlenecks of modern Operating Systems (such as the Linux kernel's Virtual File System, `pthread` context switches, and garbage collection pauses).

## Core Pillars
1. **The Zero-Heap Enforcer**: Zeus physically bans dynamic memory allocation (`malloc`, `free`). All memory is statically bound to arenas, guaranteeing absolute determinism and zero fragmentation.
2. **Lock-Free Cooperative Fibers**: Concurrency is achieved without OS-level `pthread` mutexes. Zeus implements an M:N fiber scheduler with Lock-Free Chase-Lev Work-Stealing Deques, yielding 100% core utilization.
3. **The Phoenix Firewall**: For mission-critical environments, a dedicated out-of-band Sentinel Core (`fork()` + `mmap MAP_SHARED`) snoops execution cycles via hardware `__rdtsc()`. If a fiber is trapped in a DDoS payload or infinite loop, the firewall permanently marks it dead and vaporizes its memory space.
4. **Native Tensor Calculus**: Tensors and linear algebra are natively integrated via SIMD instructions.
5. **The Anti-Bloat Enforcer**: The Zeus compiler natively sweeps its own AST and generated C-bridges for legacy software abstractions. If a banned paradigm is detected, it terminates the build.
6. **Universal Hardware Blueprints**: Targets can be defined entirely abstractly via `.zeus_arch` blueprints (e.g. Photonic ASICs, Quantum chips), enabling Zeus to bypass hardcoded targets.

## Usage
Currently Zeus is run via its Rust-based compiler engine.
```bash
# Example: Building the AI inference benchmark on a Photonic Tensor Node
cargo run -- build ../benchmarks/ai_inference.zs --arch=../benchmarks/photonic.zeus_arch
```

### The Codex
For deep architectural insights, the laws of the system, and the reasoning behind our rebellion against the OS kernel, consult `THE_ZEUS_CODEX.md` (stored locally in the artifacts directory).

## Security and Secrets
All hardware matrix blueprints (`*.zeus_arch`) are strictly ignored via `.gitignore` to prevent leaking proprietary topological architectures to public repositories.
