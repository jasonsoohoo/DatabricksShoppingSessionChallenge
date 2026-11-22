# Roadmap

| Milestone | Status | Evidence |
|---|---|---|
| M0 materials and tools | Complete | Empty workspace inventoried. All six supplied files accounted for. Three engine starters, 36 cases, published dev answers, Rust generator and official Linux limits inspected. Commands in README.md. |
| M1 correctness | Complete | Both output schemas and every expected row pass the official local fixture and seed-1 dev checkers. |
| M2 benchmarking | Complete | Four reproducible input scales. Docker entry point reports stage timings, CPU and memory. Local runner records wall time, revision and image size. |
| M3 optimization | Complete for this candidate | Native CSV and timestamp parsing, bounded event dedup and sliding-window attribution. Recoverable revisions and observed performance in BENCHMARKS.md. |
| M4 hand-in | Fifth official practice run scored 8.8148 s | Native session labeling passed the official local checks. Three final-scale outputs exactly matched the fourth submission. Local median 19.316 s. Submission SHA in SUBMISSIONS.md. |

The owner handles publication and scorer submission. Record each reported official score against its exact SHA, update the best official result, and use the hidden-run timing to prioritize the next experiment. The official dataset, scorer hardware and hidden results are unavailable locally.

## Native event-ID partition experiment

The experiment replaces the event scratch CSV and DuckDB DISTINCT ON step with in-memory, full-key deduplication and typed Arrow columns. The scored release is the comparison control. The native clean build passed the 36 fixtures and matched both public dev tables. Three local seed-23 runs reduced median wall time from 15.643 s to 10.115 s. Three final-scale runs reduced the median from 48.255 s to 33.265 s. Both large tables matched the scored release in all six comparisons. BENCHMARKS.md records the head-to-head results, including final-scale memory and the large differential comparisons. The official practice result improved from 24.98 s to 17.14 s. Use the 17.14 s release as the next control. Profile the attribution timeline sort, CSV export and Rust sweep separately before choosing which to optimize. Together the clicks and attribution stage took about 2.66 s on local practice.

## Parallel click attribution

Completed the stage profile against the 17.14 s submission. The Rust sweep took 1.476 s of practice wall time, compared with 0.443 s for sort/materialization and 0.199 s for CSV export. Splitting the sweep between people reduced its median to 0.289 s. The new release passed the public checkers and exact comparisons at practice and final scale. Official practice fell from 17.14 s to 15.6507 s. Use the parallel sweep release as the control. Next, time identity resolution and CSV preparation separately. Together they took about 2.5 s on local practice, compared with 0.289 s for the optimized sweep.

## Native identity resolution

The separate practice profile measured 2.824 s for identity resolution and lookup loading, and 0.239 s for click and dimension CSV preparation. The native resolver reduced the identity median to 0.751 s. End-to-end practice and final-scale medians fell 21.8% and 18.6% against the 15.6507 s release. The complete lookup and both daily tables passed exact comparisons. Official practice fell from 15.6507 s to 13.3325 s. Use the native identity release as the control. Next, profile session labeling and per-session aggregation separately. Their combined stage took a median 3.536 s on local practice, compared with 0.751 s for identity resolution.

## Native session labeling

The session labeler took 2.264 s on local practice versus 0.162 s for per-session aggregation. Native per-person sorting and scanning reduced end-to-end median runtime by 22.9% on practice and 17.0% on final scale. All public checks, the six paired output comparisons and a full practice comparison of the intermediate tables passed. The official practice score fell to 8.8148 s. Use this revision as the next control. Reprofile click preparation, the timeline join and sort, CSV export, and the parallel sweep on this revision before selecting the next change. The combined clicks and attribution stage took 6.391 s in the median local final-scale run.
