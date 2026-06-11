#!/bin/bash
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
