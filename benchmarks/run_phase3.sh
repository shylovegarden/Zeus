#!/bin/bash
# run_phase3.sh — Execute Phase 3 comparative analysis
# Usage: ./run_phase3.sh

set -euo pipefail

echo "=== Phase 3: Comparative Analysis ==="
echo "Comparing vectors against academic baselines..."
echo

# Ensure previous phases are complete
if [ ! -f research_results.json ]; then
    echo "Error: research_results.json missing. Run Phase 1 first."
    exit 1
fi
if [ ! -f microbenchmarks.csv ]; then
    echo "Error: microbenchmarks.csv missing. Run Phase 2 first."
    exit 1
fi

# Run comparative analysis
./run_comparative_analysis.sh

echo
echo "Phase 3 completed. Proceed to Phase 4: Dissemination."
