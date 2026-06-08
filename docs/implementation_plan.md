# Zeus Phase 5: The Bare-Metal OS-Bypass & Sentinel Firewall

This phase will elevate Zeus from a standalone compiler into a full user-space Operating System replacement, implementing DMA storage, Sentinel runtime defenses, and cache-perfect hardware mapping.

## User Review Required

> [!WARNING]  
> Bypassing the Kernel with NVMe DMA and Sentinel Process Forking is highly aggressive. Please review the architecture below before I begin execution.

## Proposed Changes

We will tackle the three pillars simultaneously by updating the `zeus_compiler`.

---

### 1. The Bare-Metal NVMe DMA Pipeline
We will expose the `NvmeDmaMap` compiler backend to the user via the standard library.

#### [NEW] [std/zeus/dma.zs](file:///Users/shy/Developer/ZEUS/std/zeus/dma.zs)
- Create the DMA standard library module containing `pub fn map_drive(path: String, size: f64)`.

#### [MODIFY] [codegen.rs](file:///Users/shy/Developer/ZEUS/zeus_compiler/src/codegen.rs)
- Update `NvmeDmaMap` generation to conditionally switch between `mmap(O_DIRECT)` bare-metal access and standard `fread` depending on the presence of a `--target nvme` compiler flag.

---

### 2. The Phoenix Firewall (M:N Fiber Sentinel)
We will rewrite the cooperative Fiber dispatcher inside `codegen.rs`'s `ParallelBlock` handler.

#### [MODIFY] [codegen.rs](file:///Users/shy/Developer/ZEUS/zeus_compiler/src/codegen.rs)
- **Sentinel `fork()`:** At the start of a `parallel` block, Zeus will `mmap(MAP_SHARED)` a global state array and `fork()` a hidden **Sentinel Core** process.
- **Cycle Heartbeats:** The primary child process will dispatch the M:N workers. Each worker will ping its exact `__rdtsc()` hardware cycle count into the shared map at the top of its execution loop.
- **The Executioner:** The Sentinel Core will infinitely poll the shared map. If `(__rdtsc() - worker_heartbeat) > 50,000,000` cycles, the Sentinel assumes the Fiber has been hit with a malicious infinite loop (DDoS payload). 
- **The Assassination:** The Sentinel will directly overwrite the worker's queue state in shared memory, mark it as `KILLED`, and reset the `__zeus_arena_offset`, successfully vaporizing the bad payload without crashing the main Zeus process.

---

### 3. `.zeus_arch` Hardware Blueprints
We will allow dynamic topological memory packing.

#### [MODIFY] [main.rs](file:///Users/shy/Developer/ZEUS/zeus_compiler/src/main.rs)
- Parse `--tune-arch=FILE` CLI argument.
- Read a `.zeus_arch` JSON definition (e.g. `{"l1_cache_size": 65536, "num_cores": 12}`).

#### [MODIFY] [codegen.rs](file:///Users/shy/Developer/ZEUS/zeus_compiler/src/codegen.rs)
- Pass the hardware blueprint into the `CTranspilerBackend`.
- Currently, `ParallelBlock` chunks work identically regardless of hardware (`__zeus_chunk_size = (__zeus_iters + 255) / 256`). We will dynamically emit chunking logic that aligns perfectly with the target machine's `l1_cache_size`, guaranteeing zero cache-misses for tensor calculations.

## Verification Plan

### Automated Tests
- We will build `benchmarks/phoenix_test.zs` that launches a `parallel` block. One of the fibers will intentionally trigger a malicious infinite loop (`while (1.0 == 1.0) {}`).
- We will compile with the Sentinel enabled and verify that the Sentinel process successfully detects the deadlock, outputs `[ZEUS SENTINEL] Assassinated Deadlocked Fiber`, and allows the program to safely exit instead of hanging forever.
