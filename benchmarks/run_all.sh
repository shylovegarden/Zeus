#!/bin/bash
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
