# Zeus Research Benchmarks

This directory contains the benchmark suites for validating and characterizing the 8 unthought algorithmic vectors (V11–V18).

## Structure
- `research_suite.rs` — Foundational validation (Phase 1)
  - Generates representative .zs workloads for each vector
  - Runs `zeus audit --json` to collect vector telemetry
  - Emits a single JSON with all vector metrics
- `microbenchmarks.rs` — Performance characterization (Phase 2)
  - Low-level cycle-accurate benchmarks for each vector
  - Emits CSV for downstream analysis
  - Simulates baseline vs vector-optimized implementations
- `run_research_suite.sh` — Executes Phase 1
- `run_microbenchmarks.sh` — Executes Phase 2

## Usage
```bash
# Phase 1: Foundational validation
./run_research_suite.sh

# Phase 2: Performance characterization
./run_microbenchmarks.sh
```

## Expected Outputs
- `research_results.json` — Vector telemetry from audit
- `microbenchmarks.csv` — Cycle counts and latencies
- Summary tables printed to console

## Integration with CI
Add to `.github/workflows/ci.yml` (when OAuth scope allows):
```yaml
- name: Research Benchmarks
  run: |
    cd benchmarks
    ./run_research_suite.sh
    ./run_microbenchmarks.sh
```

## Next Phases
- Phase 3: Comparative analysis vs academic baselines
- Phase 4: Dissemination (papers, open-source release)
