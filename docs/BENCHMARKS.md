# Benchmark history

All timings are local Docker Desktop Linux/AMD64 measurements on an AMD Windows host. Each invocation uses a fresh container, 8 CPU quota, 24 GiB memory, equal memory+swap limit, uid 1000, a read-only root, and no network. The processor, filesystem and CPU pinning differ from the official Xeon environment. Wall time below includes Docker CLI launch and exit. A single run per cell is reported unless indicated.

| Input | Seed | Nominal events | Nominal clicks | Input size | Storage |
|---|---:|---:|---:|---:|---|
| Small | 7 | 100,000 | 25,000 | 10,358,457 bytes | Windows bind mount |
| Official dev | 1 | 1,000,000 | 250,000 | 96,210,428 bytes | Windows bind mount |
| Practice scale | 23 | 20,000,000 | 5,000,000 | 1.92 GB | Linux Docker volume |
| Final scale | 42 | 60,000,000 | 15,000,000 | 5.74 GB | Linux Docker volume |

| Commit | Implementation | Small | Dev | Practice | Final |
|---|---|---:|---:|---:|---:|
| f9c918f | Python validation and DuckDB | 1.614 s | 7.847 s | — | — |
| 3112f1c | Parallel Rust CSV reader | 1.073 s | 2.444 s | 25.314 s | Out of memory during dedup |
| 220b77b | Native event validation | 1.055 s | 2.479 s | — | — |
| 51ba46c | Partitioned event IDs | — | 2.307 s | 22.265 s | 68.340 s |
| 9a8ec0b | SQL campaign priority aggregation | 1.010 s | 2.261 s | 22.107 s | 76.999 s |

All five revisions passed the 36 official fixtures and exact dev comparisons. The baseline dev run used 10.461 CPU-seconds and 1,311,780 KiB peak process RSS. The partitioned final run used 405.767 CPU-seconds and 18,072,448 KiB peak process RSS. Its 67.900 s pipeline included 12.348 s CSV/identity, 18.624 s dedup/validation, 6.697 s session construction, 20.179 s click attribution and 8.930 s output/aggregations. The next SQL experiment reached the 24 GiB cgroup limit on seed 42 and ran slower. Earlier dev timing for 220b77b overlapped a large scale test and should be treated as indicative only.

Stage durations use monotonic time. DuckDB evaluates views on export, so the output stage includes their final aggregates. CPU totals and duration are printed by the entry point. Divide CPU seconds by pipeline wall time for average active cores. The runner reports Git revision and Docker image digest and size. Official scores are recorded separately in SUBMISSIONS.md.

The sliding-window revision 108514a89e2489f8d6469623a7c4f46a6a24517c passed all 36 fixtures and the exact dev comparison. Local times were 0.974 s small, 1.818 s dev, 15.072 s practice and 51.371 s final. The final run used 250.500 CPU-seconds, 7,885,816 KiB peak process RSS and 17,920,823,296 bytes peak container memory including filesystem cache. Build pins the same base image digests used in these measurements.
