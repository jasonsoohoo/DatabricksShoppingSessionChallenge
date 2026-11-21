# Roadmap

| Milestone | Status | Evidence |
|---|---|---|
| M0 materials and tools | Complete | Empty workspace inventoried. All six supplied files accounted for. Three engine starters, 36 cases, published dev answers, Rust generator and official Linux limits inspected. Commands in README.md. |
| M1 correctness | Complete | Both output schemas and every expected row pass the official local fixture and seed-1 dev checkers. |
| M2 benchmarking | Complete | Four reproducible input scales. Docker entry point reports stage timings, CPU and memory. Local runner records wall time, revision and image size. |
| M3 optimization | Complete for this candidate | Native CSV and timestamp parsing, bounded event dedup and sliding-window attribution. Recoverable revisions and observed performance in BENCHMARKS.md. |
| M4 hand-in | First official practice run scored 24.98 s | Clean AMD64 release passed both official local checks. Three final-scale outputs exactly matched the prior implementation. Median 47.052 s. Submission SHA in SUBMISSIONS.md. |

The owner handles publication and scorer submission. Record each reported official score against its exact SHA, update the best official result, and use the hidden-run timing to prioritize the next experiment. The official dataset, scorer hardware and hidden results are unavailable locally.

## Next performance experiment

Keep the scored SHA as the control. On its seed-23 practice run, CSV and identity preparation took 3.946 s and event deduplication and validation took 6.874 s out of 17.608 s end to end. Prioritize a native event-ID partition experiment that eliminates the intermediate CSV write and SQL reparse. Compare identical inputs and limits, check both tables against the control, and require all fixtures and the official dev comparison before promoting a candidate. The reported leader is about 92.5 times faster by elapsed time; even eliminating both measured stages would leave roughly 6.8 s locally.
