# Roadmap

| Milestone | Status | Evidence |
|---|---|---|
| M0 materials and tools | Complete | Empty workspace inventoried. All six supplied files accounted for. Three engine starters, 36 cases, published dev answers, Rust generator and official Linux limits inspected. Commands in README.md. |
| M1 correctness | Complete | Both output schemas and every expected row pass the official local fixture and seed-1 dev checkers. |
| M2 benchmarking | Complete | Four reproducible input scales. Docker entry point reports stage timings, CPU and memory. Local runner records wall time, revision and image size. |
| M3 optimization | Complete for this candidate | Native CSV and timestamp parsing, bounded event dedup and sliding-window attribution. Recoverable revisions and observed performance in BENCHMARKS.md. |
| M4 hand-in | First official practice run scored 24.98 s | Clean AMD64 release passed both official local checks. Three final-scale outputs exactly matched the prior implementation. Median 47.052 s. Submission SHA in SUBMISSIONS.md. |

The owner handles publication and scorer submission. Record each reported official score against its exact SHA, update the best official result, and use the hidden-run timing to prioritize the next experiment. The official dataset, scorer hardware and hidden results are unavailable locally.

## Native event-ID partition experiment

The experiment replaces the event scratch CSV and DuckDB DISTINCT ON step with in-memory, full-key deduplication and typed Arrow columns. The scored release is the comparison control. The native clean build passed the 36 fixtures and matched both public dev tables. Three local seed-23 runs reduced median wall time from 15.643 s to 10.115 s. Three final-scale runs reduced the median from 48.255 s to 33.265 s. Both large tables matched the scored release in all six comparisons. BENCHMARKS.md records the head-to-head results, including final-scale memory and the large differential comparisons. The next useful external measurement is the official hidden practice score for the candidate.
