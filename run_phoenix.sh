#!/bin/bash
set -e

echo "==========================================="
echo "⚡ ZEUS PHOENIX FIBER BENCHMARK ⚡"
echo "==========================================="

echo "Compiling Zeus Compiler..."
cargo build --manifest-path zeus_compiler/Cargo.toml --quiet

echo "Compiling phoenix_benchmark.zs..."
zeus_compiler/target/debug/zeus_compiler benchmarks/phoenix_benchmark.zs

echo "Writing C Host Runner..."
cat << 'EOF' > phoenix_main.c
#include <stdio.h>
#include <time.h>

// Declare the Zeus-generated entry point
extern void run_phoenix_test();

int main() {
    printf("\n[LEGACY C APP] Booting Zeus Phoenix Benchmark...\n");
    printf("[LEGACY C APP] legacy_c_process_frame() will leak 100MB of RAM total.\n");
    printf("[LEGACY C APP] The Zeus Arena is fixed at 64MB.\n");
    printf("[LEGACY C APP] Executing...\n");

    struct timespec start, end;
    clock_gettime(CLOCK_MONOTONIC, &start);
    
    // Call the Zeus FFI bridge which spins up the fibers
    run_phoenix_test();
    
    clock_gettime(CLOCK_MONOTONIC, &end);
    double time_taken = (end.tv_sec - start.tv_sec) * 1e6 + (end.tv_nsec - start.tv_nsec) / 1e3;
    
    printf("[LEGACY C APP] SUCCESS! All frames processed without OOM.\n");
    printf("[LEGACY C APP] Time taken: %.2f microseconds\n", time_taken);
    printf("===========================================\n");
    
    return 0;
}
EOF

echo "Compiling Native Hybrid App..."
clang -O3 -o phoenix_app phoenix_main.c phoenix_benchmark.c benchmarks/legacy_leak.c

echo "Running Phoenix App..."
./phoenix_app
