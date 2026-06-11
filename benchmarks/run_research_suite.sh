#!/bin/bash
# run_research_suite.sh — Execute the foundational validation benchmark suite
# Usage: ./run_research_suite.sh

set -euo pipefail

echo "=== Zeus Research Suite: Foundational Validation ==="
echo "Collecting telemetry for vectors V11–V18..."
echo

# Build the benchmark harness
cd "$(dirname "$0")/.." || exit 1
cargo build --release --bin zeus_compiler || exit 1

# Run the research suite
cd benchmarks || exit 1
cargo run --release --bin research_suite > research_results.json 2>research_errors.log || {
    echo "Benchmark suite failed. See research_errors.log"
    exit 1
}

echo "Results saved to benchmarks/research_results.json"
echo "Errors (if any) saved to benchmarks/research_errors.log"
echo

# Summarize key metrics
echo "=== Summary ==="
jq -r '
  .benchmarks | to_entries[] |
  "\(.key): " +
  (if .value.hif then "HIF branches eliminated=\(.value.hif.total_branches_eliminated) " else "" end) +
  (if .value.lph then "LPH vars woven=\(.value.lph.total_vars_woven) " else "" end) +
  (if .value.pts then "PTS fibers=\(.value.pts.fiber_count) " else "" end) +
  (if .value.metamorph then "Metamorph hot loops=\(.value.metamorph.hot_loops) " else "" end) +
  (if .value.live_zk then "Live ZK steps=\(.value.live_zk.total_steps) " else "" end) +
  (if .value.silicon_aware then "Silicon Aware variants=\(.value.silicon_aware.total_variants_generated) " else "" end) +
  (if .value.enclave then "Enclave arenas=\(.value.enclave.total_arenas) " else "" end) +
  (if .value.swarm then "Swarm nodes=\(.value.swarm.total_nodes) " else "" end)
' research_results.json || echo "jq not available; see JSON directly."

echo
echo "Next steps:"
echo "  - Validate correctness proofs in Phase 1"
echo "  - Run microbenchmarks for performance characterization"
echo "  - Compare against academic baselines in Phase 2"
