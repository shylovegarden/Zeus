#!/bin/bash
# run_comparative_analysis.sh — Phase 3 Comparative Analysis
# Usage: ./run_comparative_analysis.sh

set -euo pipefail

echo "=== Zeus Comparative Analysis: Phase 3 ==="
echo "Comparing vectors against academic baselines..."
echo

# Build comparative analysis binary
cd "$(dirname "$0")/.." || exit 1
cargo build --release --bin zeus_compiler || exit 1

cd benchmarks || exit 1
cargo run --release --bin comparative_analysis > comparative_results.json 2>comparative_errors.log || {
    echo "Comparative analysis failed. See comparative_errors.log"
    exit 1
}

echo "Results saved to benchmarks/comparative_results.json"
echo "Errors (if any) saved to benchmarks/comparative_errors.log"
echo

# Summarize speedups
echo "=== Speedup Summary ==="
if command -v jq >/dev/null 2>&1; then
    jq -r '.[] | "\(.vector): \(.speedup)x"' comparative_results.json | sort -k2 -nr
else
    echo "jq not available; see JSON directly."
fi

echo
echo "Next steps:"
echo "  - Prepare Phase 4: Dissemination (papers, open-source release)"
echo "  - Target conferences: PLDI, ASPLOS, SOSP, USENIX Security, OSDI"
