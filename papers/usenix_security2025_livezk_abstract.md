# Live ZK-SNARK Execution Exhaust: Continuous Cryptographic Attestation of Control Flow

**Abstract**

Static attestation proves that a binary was compiled safely, but runtime attacks (e.g., control-flow hijacking, fault injection) can compromise execution integrity. We introduce Live ZK-SNARK Execution Exhaust (Live ZK), a runtime system that continuously generates cryptographic proofs of correct control flow. Live ZK injects lightweight telemetry hooks at control-flow points; each hook emits a SHA-256 hash of the program counter and a per-process secret into a rolling Merkle tree. The resulting exhaust stream can be verified by a supervisor without pausing execution or reading memory. We implement Live ZK in the Zeus compiler and evaluate it on cryptographic kernels and control-flow-intensive benchmarks. Live ZK adds 11.6 cycles per step and can generate proofs at 10 kHz on modern CPUs. In a fault-injection experiment, Live ZK detects 100% of control-flow deviations within 1 µs. Our approach provides a practical foundation for runtime integrity monitoring in aerospace, medical devices, and blockchain infrastructure where static guarantees are insufficient.

**Keywords**: runtime attestation, control-flow integrity, zero-knowledge proofs, cryptographic monitoring, system security
