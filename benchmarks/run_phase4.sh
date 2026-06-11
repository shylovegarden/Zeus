#!/bin/bash
# run_phase4.sh — Phase 4 Dissemination (papers and open-source release)
# Usage: ./run_phase4.sh

set -euo pipefail

echo "=== Phase 4: Dissemination ==="
echo "Preparing papers and open-source release..."
echo

# Ensure previous phases are complete
if [ ! -f comparative_results.json ]; then
    echo "Error: comparative_results.json missing. Run Phase 3 first."
    exit 1
fi

echo "Draft abstracts ready in ../papers/"
echo "Open-source release plan documented in ../papers/README.md"
echo

echo "Next steps:"
echo "  - Submit abstracts to PLDI, ASPLOS, SOSP, USENIX Security, OSDI"
echo "  - Prepare Docker images for reproducibility"
echo "  - Set up GitHub repository with Apache 2.0 license"
echo "  - Engage community via LLVM Dev Meeting and RustConf"
