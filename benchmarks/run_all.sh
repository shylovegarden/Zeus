#!/bin/bash
<<<<<<< HEAD
# run_all.sh — Execute Phase 1 and Phase 2 sequentially
# Usage: ./run_all.sh

set -euo pipefail

echo "=== Zeus Research Benchmark Suite (All Phases) ==="
echo

# Phase 1: Foundational validation
echo "--- Phase 1: Foundational Validation ---"
./run_research_suite.sh
echo

# Phase 2: Performance characterization
echo "--- Phase 2: Performance Characterization ---"
./run_microbenchmarks.sh
echo

echo "=== All phases completed ==="
echo "Results:"
echo "  - research_results.json (Phase 1)"
echo "  - microbenchmarks.csv (Phase 2)"
echo
echo "Proceed to Phase 3: Comparative Analysis"
=======
# Parallel benchmark runner for Zeus

ZEUS_BIN="${ZEUS_BIN:-./zeus_compiler/target/release/zeus_compiler}"
BENCHMARKS=(
    "crypto/aes_bench"
    "medical/ecg_processing"
    "aerospace/attitude_control"
)

run_benchmark() {
    local bench=$1
    echo "=== Running $bench ==="
    cd "$(dirname "$0")" || exit
    if $ZEUS_BIN build "${bench}.zs" 2>/dev/null; then
        time "./$(basename $bench)"
        echo "✅ $bench PASSED"
    else
        echo "❌ $bench FAILED"
    fi
}

export -f run_benchmark
export ZEUS_BIN

# Run benchmarks in parallel
echo "Running benchmarks in parallel..."
printf '%s\n' "${BENCHMARKS[@]}" | xargs -P 4 -I {} bash -c 'run_benchmark "$@"' _ {}

echo ""
echo "All benchmarks complete!"
>>>>>>> 15c776e (Add parallel expansion components - multiple features)
