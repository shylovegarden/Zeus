# Implementation Plan: Absolute Black-Box Execution Model

To achieve a state where an attacker physically cannot reverse-engineer, decompile, or observe the logic of a Zeus-compiled binary, we must introduce the ultimate defense: a cryptographic "Black-Box" compilation target. We will build the foundations for **Indistinguishability Obfuscation (iO)**, **Oblivious RAM (ORAM)**, and **Hardware Enclave Binding**.

## User Review Required

> [!CAUTION]
> Implementing mathematically perfect Indistinguishability Obfuscation (iO) incurs extreme performance overhead at runtime. 
> To mitigate this, Zeus will only obfuscate designated `target { enclave }` blocks or secrets, rather than the entire execution space. 
> **Question:** Do you want to start by# The Zeus Terminal Vectors: Anti-Side-Channel, Auto-Fuzz, and IOMMU

We will complete the final architectural vectors of the Zeus Compiler in order: Concurrency Hardening, The AI Compiler Engine, and the Physical DMA Firewall.

## Proposed Changes

### Vector 1: The Anti-Side-Channel Engine (Concurrency Hardening)
- **Goal**: Neutralize Meltdown/Spectre (speculative execution) and Thermal Resonance Power Virus attacks.
- **Files to Modify**:
  #### [MODIFY] [codegen.rs](file:///Users/shy/Developer/ZEUS/zeus_compiler/src/codegen.rs)
  - **Hardware Speculation Flushes**: Inject `#define zeus_speculation_flush() _mm_lfence()` into the C header and emit it inside the `main()` M:N Fiber Scheduler loop to purge transient CPU cache states on context switch.
  - **Stochastic Core Hopping**: Add a fast PRNG (using `__rdtsc()`) inside the fiber scheduler that randomly calculates CPU core affinity and enforces it via `sched_setaffinity()` (mocked/simulated for macOS compat, or injected natively for Linux targets), forcing fibers to bounce between physical cores to disrupt thermal tracking.

---

### Vector 2: The Auto-Fuzz AI Synthesis Engine
- **Goal**: Implement the `zeus build --tune` flag to train a neural network at compile-time and bake the INT4-quantized weights directly into `.rodata` for the `@adaptive` keyword.
- **Files to Modify**:
  #### [MODIFY] [main.rs](file:///Users/shy/Developer/ZEUS/zeus_compiler/src/main.rs)
  - Add argument parsing for the `--tune` flag.
  - Create a mock compiler fuzzing loop that simulates 1,000 inputs and "trains" an AST model.
  #### [MODIFY] [codegen.rs](file:///Users/shy/Developer/ZEUS/zeus_compiler/src/codegen.rs)
  - Allow the CGenerator to accept dynamic weight arrays from the `--tune` pass.
  - Replace the currently static `__zeus_micro_ai_weights` array with a dynamically generated, flat, cache-aligned C-array based on the fuzzing results.

---

### Vector 3: Hardware IOMMU Segmentation (Physical DMA Firewall)
- **Goal**: Neutralize DMA Reflection attacks where malicious PCIe devices bypass the CPU to read our static memory arenas directly.
- **Files to Modify**:
  #### [MODIFY] [codegen.rs](file:///Users/shy/Developer/ZEUS/zeus_compiler/src/codegen.rs)
  - Inject a new initialization phase: `__zeus_iommu_secure_segment()`.
  - This function will emit mock C code simulating `ioctl(VFIO_IOMMU_MAP_DMA)` to strictly bind the `__zeus_arena` physical memory pages to a specific, trusted PCIe bus/slot, locking out unauthorized DMA.

## Verification Plan
1. **Automated Tests**:
   - `benchmarks/side_channel_test.zs`: Verify that `_mm_lfence()` and core affinity PRNG logic are compiled correctly in the scheduler.
   - `benchmarks/fuzz_test.zs`: Run `zeus build --tune` to verify the `.rodata` array dynamically updates with newly "trained" weights instead of static defaults.
   - `benchmarks/iommu_test.zs`: Verify the generated C code initializes the `VFIO_IOMMU_MAP_DMA` structures correctly.
