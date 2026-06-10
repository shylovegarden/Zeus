# Task List: Zeus Execution Hardening

## Phase 5: The Bare-Metal OS-Bypass & Sentinel Firewall
- [x] Create `std/zeus/dma.zs` and add NVMe abstraction.
- [x] Update `zeus_compiler/src/main.rs` to accept `--target nvme` flag.
- [x] Update `zeus_compiler/src/codegen.rs` to emit `mmap(O_DIRECT)` for NVMe targets.
- [x] Modify `zeus_compiler/src/codegen.rs` to inject Phoenix Firewall `fork()` Sentinel into `ParallelBlock`.
- [x] Add `__rdtsc()` heartbeats and DDoS assassination logic to Fiber execution loop.
- [x] Update `zeus_compiler/src/main.rs` to parse `--tune-arch` and read `.zeus_arch` JSON files.
- [x] Pass `HardwareBlueprint` to `CTranspilerBackend` and emit optimal L1 Cache chunks for `ParallelBlock`.
- [x] Build and run `benchmarks/phoenix_test.zs` to test Sentinel assassination of an infinite loop.
