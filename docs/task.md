# Task List: The Zeus Terminal Vectors

## Vector 1: The Anti-Side-Channel Engine (Concurrency Hardening)
- [x] Inject `#define zeus_speculation_flush() _mm_lfence()` in `codegen.rs`.
- [x] Add `zeus_speculation_flush()` inside the fiber scheduler context switch loop.
- [x] Implement Stochastic Core Hopping using `__rdtsc()` and `sched_setaffinity()` in the fiber loop.
- [x] Create `benchmarks/side_channel_test.zs` and verify C output.

## Vector 2: The Auto-Fuzz AI Synthesis Engine
- [x] Update `main.rs` to parse the `--tune` flag.
- [x] Add the Auto-Fuzz Synthesis Loop to `main.rs` right before `CCodegen` invocation.
- [x] Modify `CCodegen` to accept an array of `tuned_weights`.
- [x] Emit the quantized AI weights directly into the `.rodata` section in the generated C code.
- [x] Create `benchmarks/fuzz_test.zs` and verify execution with `--tune`. output.

## Vector 3: Hardware IOMMU Segmentation
- [x] Inject `__zeus_iommu_secure_segment()` stub in `codegen.rs`.
- [x] Configure the stub to simulate opening `/dev/vfio/vfio` to bind the static memory regions and prevent DMA.
- [x] Ensure `__zeus_iommu_secure_segment()` is called upon startup (`main()`).
- [x] Create `benchmarks/iommu_test.zs` and verify C output.
