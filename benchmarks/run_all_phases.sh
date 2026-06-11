#!/bin/bash
# run_all_phases.sh — Execute Phases 1–3 sequentially
# Usage: ./run_all_phases.sh

set -euo pipefail

echo "=== Zeus Research Benchmark Suite: All Phases ==="
echo

# Phase 1: Foundational validation
echo "--- Phase 1: Foundational Validation ---"
./run_research_suite.sh
echo

# Phase 2: Performance characterization
echo "--- Phase 2: Performance Characterization ---"
./run_microbenchmarks.sh
echo

# Phase 3: Comparative analysis
echo "--- Phase 3: Comparative Analysis ---"
./run_phase3.sh
echo

# Phase 4: Dissemination
echo "--- Phase 4: Dissemination ---"
./run_phase4.sh
echo

echo "=== All phases completed ==="
echo "Results:"
echo "  - research_results.json (Phase 1)"
echo "  - microbenchmarks.csv (Phase 2)"
echo "  - comparative_results.json (Phase 3)"
echo "  - Draft abstracts in ../papers/ (Phase 4)"
echo
echo "Dissemination ready: submit papers and prepare open-source release."
