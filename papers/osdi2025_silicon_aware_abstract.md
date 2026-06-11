# Autonomous Silicon-Aware Lowering: Formal MLIR Dialect Selection with Runtime Bounds Preservation

**Abstract**

Modern compilers must target diverse hardware (CPU, NPU, CGRA, WASM), but existing auto-vectorizers lack formal guarantees that the generated code respects high-level safety properties. We propose Autonomous Silicon-Aware Lowering, a compiler middle-end that detects silicon capabilities via CPUID, selects an MLIR dialect (cpu/nvptx/tosa/cgra/wasm), and generates a dispatch table guarded by an embedded Z3-lite prover. The prover ensures that each variant respects the original zero-heap, constant-time, and WCET bounds before dispatch. We implement Silicon-Aware in the Zeus compiler and evaluate it on tensor kernels across x86_64, ARM64, and simulated NPU backends. Silicon-Aware achieves up to 3× speedup over the CPU baseline on NPU workloads while preserving all formal bounds. In a case study on an edge AI inference pipeline, Silicon-Aware automatically selects the optimal backend and reduces latency by 2.1× without violating safety constraints. Our work provides a principled path to portable, formally verified code generation for heterogeneous systems.

**Keywords**: code generation, MLIR, formal verification, heterogeneous computing, silicon-aware compilation
