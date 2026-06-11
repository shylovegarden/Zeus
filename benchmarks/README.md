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

# Phase 3: Comparative analysis
./run_comparative_analysis.sh

# Phase 4: Dissemination (papers, open-source release)
./run_phase4.sh

# All phases (1–4)
./run_all_phases.sh
```

## Expected Outputs
- `research_results.json` — Vector telemetry from audit (Phase 1)
- `microbenchmarks.csv` — Cycle counts and latencies (Phase 2)
- `comparative_results.json` — Speedups vs baselines (Phase 3)
- Draft abstracts in `../papers/` (Phase 4)
- Summary tables printed to console

## Integration with CI
Add to `.github/workflows/ci.yml` (when OAuth scope allows):
```yaml
- name: Research Benchmarks
  run: |
    cd benchmarks
    ./run_all_phases.sh
```

## Dissemination
- Draft abstracts for PLDI, ASPLOS, SOSP, USENIX Security, OSDI
- Open-source release plan (Apache 2.0, Docker, reproducibility)
