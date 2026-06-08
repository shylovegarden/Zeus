# The Zeus Rulebook: Constraints & Trade-Offs

This is the living document tracking every architectural rule and the trade-offs we accept to maintain the Zeus Manifesto. Every major architectural decision must be logged here so we always know exactly what we are sacrificing to achieve our goals.

## Fundamental Laws
1. **No Implicit Magic:** Absolute determinism is the highest law.
2. **Zero-Heap Enforcer:** No `malloc`/`free`. All memory must use static arena pools.
3. **Lock-Free Concurrency:** No OS-level locks (`pthreads`). M:N cooperative fibers only.
4. **Hermetic Compilation:** Byte-for-byte reproducible builds without network bloat.

## Feature Trade-Offs & The Balanced Ledger

To maintain our bare-metal dominance without sacrificing usability, we systematically neutralized our original brutal trade-offs using advanced hardware/compiler mechanics:

| The Core Feature | Original Brutal Trade-Off (What We Lost) | The Zeus Engineering Fix (How We Reclaimed It) | Final Operational Impact |
| :--- | :--- | :--- | :--- |
| **M:N Fiber Scheduler** | Lost OS-level preemption. Threads could permanently stall. | **Implicit Yields & SMT Hyper-Thread Smuggling** | 100% core availability. The hardware sibling thread snoops without stealing primary execution cycles. |
| **Invisible SoA & FFI** | Broke C-ABI compatibility and isolated us from the Open Source ecosystem. | **The Alchemy AST Mutator Pass** | Legacy C/C++ dependencies are hot-swapped at compile time. `malloc` and `pthreads` are automatically rewritten into Arena and Fiber primitives. |
| **The Zero-Heap Enforcer** | Lost dynamic memory scaling. Arena capacity ceilings forced packet drops. | **Elastic Arena Ballooning (Virtual Over-Provisioning)** | Flat total RAM usage, but local arenas can dynamically expand by stealing physical pages from adjacent arenas via atomic bit-shifts. |
| **Holographic Replay Engine** | Lost standard GDB usability and required dedicating physical cores. | **Out-of-Band State Emulation Bridge** | Developers use standard GDB breakpoints on a virtualized live timeline, while the physical hardware runs uninterrupted. |

*Note: The AI Agent must consult and update this rulebook whenever proposing a new architectural upgrade. The user must always be informed of the Trade-Off before implementation begins.*
