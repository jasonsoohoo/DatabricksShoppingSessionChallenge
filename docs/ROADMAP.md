# Roadmap

| Milestone | Status | Evidence |
|---|---|---|
| M0 materials and tools | Complete | Empty workspace inventoried. All six supplied files accounted for. Three engine starters, 36 cases, published dev answers, Rust generator and official Linux limits inspected. Commands in README.md. |
| M1 correctness | Complete | Both output schemas and every expected row pass the official local fixture and seed-1 dev checkers. |
| M2 benchmarking | Complete | Four reproducible input scales. Docker entry point reports stage timings, CPU and memory. Local runner records wall time, revision and image size. |
| M3 optimization | Complete for this candidate | Native CSV and timestamp parsing, bounded event dedup and sliding-window attribution. Recoverable revisions and observed performance in BENCHMARKS.md. |
| M4 hand-in | Complete locally | Clean AMD64 release passed both official local checks. Three final-scale outputs exactly matched the prior implementation. Median 47.052 s. Submission SHA in SUBMISSIONS.md. |

The owner handles publication and scorer submission. Record each reported official score against its exact SHA, update the best official result, and use the hidden-run timing to prioritize the next experiment. The official dataset, scorer hardware and hidden results are unavailable locally.
