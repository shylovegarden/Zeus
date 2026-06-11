# Hyper-Dimensional Memory Weaving: Cache-Oblivious Data Placement via Locality-Preserving Hashing

**Abstract**

Modern memory hierarchies suffer from cache misses due to irregular data access patterns. We present Hyper-Dimensional Memory Weaving (LPH), a compiler pass that reorganizes data structures into cache-line-aligned clusters based on their co-access graph. LPH employs Locality-Preserving Hashing to map frequently accessed variables onto the same 64-byte cache line, reducing L1 miss rates without programmer annotations. Unlike traditional cache-oblivious algorithms that focus on algorithmic structure, LPH operates on the dataflow graph to predict temporal locality. We implement LPH in the Zeus compiler and evaluate it on pointer-chasing data structures, graph traversals, and matrix kernels. On microbenchmarks, LPH reduces L1 miss rates by up to 45% and improves throughput by 1.8× compared to random access baselines. On a real-world graph traversal, LPH achieves 1.3× speedup over hand-optimized SoA layouts. Our approach is fully automatic, requires no source changes, and is compatible with existing C interop. LPH provides a practical path to cache-efficient code for high-performance computing and embedded systems.

**Keywords**: cache optimization, data layout, locality-preserving hashing, compiler optimization, performance
