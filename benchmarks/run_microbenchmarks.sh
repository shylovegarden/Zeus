#!/bin/bash
# run_microbenchmarks.sh — Phase 2 Performance Characterization
# Usage: ./run_microbenchmarks.sh

set -euo pipefail

echo "=== Zeus Microbenchmarks: Performance Characterization ==="
echo "Measuring per-vector latency and throughput..."
echo

# Build microbenchmark binary
cd "$(dirname "$0")/.." || exit 1
cargo build --release --bin zeus_compiler || exit 1

cd benchmarks || exit 1
cargo run --release --bin microbenchmarks > microbenchmarks.csv 2>microbench_errors.log || {
    echo "Microbenchmark suite failed. See microbench_errors.log"
    exit 1
}

echo "Results saved to benchmarks/microbenchmarks.csv"
echo "Errors (if any) saved to benchmarks/microbench_errors.log"
echo

# Summarize key metrics
echo "=== Summary (cycles per operation) ==="
column -t -s, microbenchmarks.csv | tail -n +2 | awk '
BEGIN { print "Vector        Test            Cycles/op" }
{
    vector = $1
    test = $2
    cycles = $3
    if (!(vector in min) || cycles < min[vector]) {
        min[vector] = cycles
        best[vector] = test
    }
}
END {
    for (v in min) printf "%-14s %-15s %d\n", v, best[v], min[v]
}
'

echo
echo "Next steps:"
echo "  - Compare against academic baselines (branchless, cache-oblivious, schedulers)"
echo "  - Run macrobenchmarks on real workloads (ML inference, ECU control, blockchain)"
echo "  - Prepare Phase 3 comparative analysis"
