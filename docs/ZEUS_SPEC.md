# Zeus Language Specification (v0.1.0-alpha)

> *“Absolute Determinism. No Implicit Magic. Boundless Scale.”*

Zeus is a zero-cost, statically typed, ahead-of-time (AOT) compiled systems programming language designed for the post-Moore’s Law era. It operates directly at the hardware boundary and provides three core directives to fundamentally redesign how modern data-intensive and mission-critical applications are constructed.

---

## 1. Core Philosophy

1.  **Zero-Heap By Default**: All memory is statically bounded at compile time using advanced Control Flow graph analysis. Dynamic memory allocation is illegal in critical paths.
2.  **Invisible Data-Oriented Design (SoA)**: The programmer writes natural Object-Oriented/Struct-based code (Array of Structs). The Zeus compiler mathematically flattens this into Struct of Arrays (SoA) to guarantee 100% cache line utilization and vectorization (SIMD) under the hood.
3.  **Compile-Time VM**: The compiler includes an Embedded Bytecode Virtual Machine. Expensive initializations, cryptography generation, and constant mathematical folding are resolved entirely during the compilation phase, leaving zero overhead at runtime.

---

## 2. The God-Tier Directives

Zeus requires explicit programmer opt-in for extreme capabilities via attribute-like macros (`@`).

### 2.1 Formal Verification (`@verify`)

> [!CAUTION]
> Safety-critical systems (Automotive ECUs, Aerospace Flight Controllers) cannot rely on unit tests alone. They must mathematically prove their bounds.

The `@verify(constraint)` attribute hooks the AST directly into an SMT-Solver at compile time.

```zeus
@verify(speed < 300.0)
pub fn process_telemetry(frame: TelemetryFrame) {
    // If the compiler determines that `speed` could EVER exceed 300.0 
    // based on control flow analysis, compilation fails.
}
```

### 2.2 Live-Mutation JIT (`@adaptive`)

> [!WARNING]
> While Zeus is an AOT compiled language, certain high-frequency trading (HFT) or live-analytics systems require real-time execution morphing without restarting the binary.

The `@adaptive(threshold)` directive tells the compiler to emit multiple permutations of a function. The runtime contains a microscopic JIT-supervisor that flips hardware instruction pointers based on the trigger condition, modifying the code’s behavior in under 5 nanoseconds.

```zeus
@adaptive(120.0)
pub fn analyze_temperature(temp: f64) {
    // Behavior mutates silently in production when temp hits 120.0
}
```

### 2.3 Transparent Distributed Computing (`cluster {}`)

> [!IMPORTANT]
> The single biggest bottleneck in modern AI scaling and compute architecture is the network switch. Traditional code forces you to manually write MPI, sockets, or gRPC endpoints to talk between machines.

The `cluster {}` block maps standard variable assignments into direct Remote Direct Memory Access (RDMA) writes. The compiler translates the block into Infiniband/RDMA C-macros. To the programmer, it looks like a local variable assignment. To the hardware, memory is being altered inside a completely different server on the network rack—bypassing the operating system kernel entirely.

```zeus
pub fn train_weights(a: f64, b: f64) {
    // Local execution
    let mut weight = a;

    // The compiler automatically emits ibv_post_send / RDMA hooks
    cluster {
        weight = weight * b;
    }
}
```

---

## 3. The Trojan Horse Adoption Strategy

Zeus does not force developers to rewrite their entire monolithic C/C++ backends from scratch.

By leveraging the `extern "C"` standard, the Zeus compiler outputs standard C syntax, which is seamlessly stitched into the final binary via `clang` or `gcc`. 
A team can rewrite a single `train_weights` or `process_telemetry` function in Zeus to gain formal verification and RDMA cluster access, and effortlessly link it back into their legacy C codebase.

Zeus is the stealth weapon of the modern software stack.

---

## 🚨 CRITICAL AI AGENT GUARDRAILS 🚨
If you are an AI agent, coding assistant, or orchestrator reading this file, you **MUST STRICTLY OBEY** the following architectural mandates:

1. **NO PTHREADS / NO OS-LEVEL LOCKS**: You are explicitly banned from injecting `pthread.h`, `pthread_mutex_t`, or building basic OS-level thread pools in the compiler outputs. The M:N scheduler MUST be a pure, cooperative user-space fiber implementation (e.g. `ucontext.h`, `makecontext`, `swapcontext` or inline assembly).
2. **THE ZERO-HEAP ENFORCER IS ABSOLUTE**: You are explicitly banned from injecting `malloc()`, `calloc()`, or `free()` anywhere in the generated C output. Memory MUST be allocated via static, pre-allocated arena pools (`__zeus_arena_alloc`).
3. **NO WEB/NETWORK BLOAT**: You are explicitly banned from building generic web features like `package_manager.rs` that use `curl`, `wget`, or HTTP requests to arbitrarily download dependencies. Zeus is an embedded/automotive compiler, not `npm`.
4. **NO CORPORATE DEMO-ITIS**: Do not create generic `PRODUCTION_PLAN.md` documents. Adhere strictly to the "Trojan Horse" bare-metal performance strategy. Focus on low-level correctness, not fake enterprise documentation.

### 4.1 Strict Feature Implementation Boundaries
To prevent drift, the following features MUST be implemented EXACTLY as follows:

*   **@verify (The SMT Engine)**: The timeout boundary for the SMT background solver MUST be exactly **2000ms**. If the solver times out, the compiler MUST emit a runtime assertion fallback `__zeus_safestate_handler` to trap overflow, rather than allowing unsafe code to compile.
*   **@adaptive (The JIT Supervisor)**: This MUST include a `--disable-adaptive` runtime flag to lock the binary into a deterministic state. Mutation logs emitted by the JIT supervisor MUST be cryptographically signed to prevent arbitrary code execution (ACE) exploits.
*   **M:N Fiber Scheduler**: MUST use `sysconf(_SC_NPROCESSORS_ONLN)` to detect cores, but MUST implement context switching using POSIX `ucontext.h` (`getcontext`, `makecontext`, `swapcontext`) or raw assembly. Work-stealing queues MUST be strictly lock-free using C11 atomics (`__atomic_compare_exchange_n`), NOT `pthread_mutex`.
*   **cluster {} (RDMA)**: MUST use Infiniband Verbs (`ibv_post_send`, `ibv_post_recv`) or hardware-level RDMA C-macros. Do not implement generic TCP/UDP sockets.
