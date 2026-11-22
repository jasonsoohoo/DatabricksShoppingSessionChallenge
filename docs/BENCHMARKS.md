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

## Release verification environment

Host CPU is an AMD Ryzen 7 7800X3D, eight cores and 16 logical processors. Windows reports build 26200. Docker Desktop Linux exposes 16 logical CPUs and 33,168,183,296 bytes of RAM to the engine. The benchmark containers receive the lower competition limits described above. All three starter ZIPs contain 290 identical fixture files (byte-verified), so each suite run covers the supplied fixture set across engines.

Release eb3762be83c1b0ee58826a8d674070f3a87fcf30 used a fresh Git archive and a no-cache Linux/AMD64 build. The archive held 25 files (29,362 bytes zipped); Docker reports 292,878,672 bytes for the image. The clean build passed 36/36 fixtures. On dev, the checker found 117,038 user/day rows and 27,400 campaign/day rows, with zero missing, extra, or duplicate keys in both tables. Dev took 1.991 s. Practice seed 23 took 17.608 s. At roughly two-second intervals on a 60M run, the highest of 22 combined scratch/output samples was 5,513,408,456 bytes. The sampled maximum does not capture transient activity between samples.

| Clean final repeat | Docker wall | Pipeline wall | CPU seconds | Peak cgroup bytes |
|---:|---:|---:|---:|---:|
| 1 | 49.387 s | 48.991 s | 233.441 | 13,021,577,216 |
| 2 | 47.052 s | 46.655 s | 221.399 | 13,108,256,768 |
| 3 | 44.921 s | 44.484 s | 210.550 | 13,035,716,608 |

The median local final-scale time was **47.052 s**. The median run used an average of 4.75 cores. Outputs were compared as multisets in both directions with EXCEPT ALL. Both tables matched the prior SQL attribution implementation on every final repeat (10,067,851 sessions, 20,418,435,163 revenue cents) and on practice (3,368,309 sessions, 6,155,929,657 cents). The small sweep run matched the earlier small result (19,939 sessions, 63,630,203 cents). Both daily tables reconciled and campaign assist checks passed. Large inputs have no published answer files.

## First official practice result

On 2026-11-21 the owner reported 24.98 s for eb3762be83c1b0ee58826a8d674070f3a87fcf30. That is 7.372 s (41.9%) longer than the clean local seed-23 practice run. Input seed and hardware differ, and the hidden run supplies no stage breakdown. One paired observation cannot isolate the cause or establish a final-scale conversion factor.

## Native event-ID partitions, November 21

Control A is the published 24.98 s submission, eb3762be83c1b0ee58826a8d674070f3a87fcf30 (image 7d61157ddff2). Candidate B is c19ddd40924f3180f7ef5bb34f61a7bc9dec7f50 (image dc542f1644c9). B was built without cache from a clean Git archive on linux/amd64. Image size was 487,486,953 bytes. The native library retains all event partitions in memory during deduplication. PyArrow exposes the winner buffers to DuckDB. Click preparation and the attribution timeline still use their existing CSV paths.

The clean B image passed all 36 fixtures. The published seed-1 dev comparison matched all 117,038 user/day and 27,400 campaign/day rows, both schemas, and zero duplicate keys. Dev wall was 1.524 s.

Both images ran sequentially with the same generator volumes, host, CPU quota, memory limit and fresh scratch volumes. Practice used seed 23 (20M events and 5M clicks); A/B order was AB, BA, AB. Final used seed 42 (60M events and 15M clicks); the first A run was the same-day control before the candidate build, followed later by B, A, B, A, B. Times include Docker CLI launch and exit. Use the third positional argument of scripts/local.ps1 to specify the control or candidate image. The image ID identifies the runtime even when the checkout points at a different revision.

| 20M practice run | A wall | B wall |
|---|---:|---:|
| 1 | 15.329 s | 10.121 s |
| 2 | 15.643 s | 9.896 s |
| 3 | 16.036 s | 10.115 s |
| Median | 15.643 s | 10.115 s |

Practice elapsed time decreased 35.3%. Median event deduplication/validation fell from 6.000 s to 1.762 s. CSV/identity preparation also fell from 3.481 s to 2.519 s because that stage no longer writes event shards. The owner subsequently reported 17.14 s for B on official practice, down from 24.98 s for A. The official reduction of 31.4% was about 4 percentage points smaller than the paired local improvement. The scorer provides no stage breakdown to explain the difference.

| 60M final-scale run | A wall | B wall | A peak process RSS | B peak process RSS |
|---|---:|---:|---:|---:|
| 1 | 47.525 s | 33.090 s | 7,885,156 KiB | 11,284,184 KiB |
| 2 | 56.770 s | 37.333 s | 7,686,360 KiB | 11,236,236 KiB |
| 3 | 48.255 s | 33.265 s | 7,837,148 KiB | 11,283,424 KiB |
| Median | 48.255 s | 33.265 s | 7,837,148 KiB | 11,283,424 KiB |

Final-scale elapsed time decreased 31.1%. The native partitions use more process memory. The highest B cgroup reading was 12,313,018,368 bytes (11.47 GiB) under the 24 GiB container limit.

For each A/B pair, scripts/check_large.py compared both Parquet tables with bidirectional EXCEPT ALL and checked aggregate invariants. All six comparisons passed. Each practice output counted 3,368,309 sessions and 6,155,929,657 cents; each final output counted 10,067,851 sessions and 20,418,435,163 cents.

## Parallel attribution sweep

The control was c19ddd40924f3180f7ef5bb34f61a7bc9dec7f50 (17.14 s official practice, image dc542f1644c9). Candidate 9776571dedb150b32836d8e0c1a10d547181edb6 changed only the Rust attribution executable source. Image 026dbd050fe8 was built without cache from the candidate Git archive for linux/amd64 (487,562,984 bytes).

For the stage profile, the scored pipeline used a temporary table for the joined clicks and sessions, then materialized the ordered timeline with the production keys (person, timestamp, descending click ID with NULL last). The next timed statement copied its four output columns to CSV. A separate timer surrounded the complete Rust process. This separates wall times that the production COPY query can overlap. Sort timings include materializing the ordered table. The end-to-end comparisons use the original fused COPY path and no profiling wrapper.

| Scored control, seed 23 | Run 1 | Run 2 | Run 3 | Median |
|---|---:|---:|---:|---:|
| Join and prepare | 0.627 s | 0.560 s | 0.533 s | 0.560 s |
| Sort and materialize | 0.443 s | 0.443 s | 0.437 s | 0.443 s |
| CSV export | 0.203 s | 0.199 s | 0.188 s | 0.199 s |
| Rust sweep | 1.438 s | 1.489 s | 1.476 s | 1.476 s |

The single-threaded sweep took longest. The candidate splits the sorted timeline at complete person boundaries, schedules those ranges on up to NCPU workers, and merges worker totals by campaign and day. Each worker uses the original seven-day eligibility, winner and assist logic. The clean build passed every official fixture and matched the published dev schemas and all 117,038 user/day and 27,400 campaign/day rows without duplicate keys. The range regression in scripts/test_attribution_ranges.py passed at one and eight workers, including a single person spanning several target boundaries and clicks expiring at seven days plus one millisecond.

| Candidate, same profile and input | Run 1 | Run 2 | Run 3 | Median |
|---|---:|---:|---:|---:|
| Join and prepare | 0.682 s | 0.674 s | 0.547 s | 0.674 s |
| Sort and materialize | 0.506 s | 0.538 s | 0.447 s | 0.506 s |
| CSV export | 0.265 s | 0.265 s | 0.204 s | 0.265 s |
| Rust sweep | 0.291 s | 0.289 s | 0.227 s | 0.289 s |

The sweep took 80.4% less wall time (5.1× speedup). End-to-end tests ran sequentially under the standard 8 CPU, 24 GiB Linux container limits. Both images used the same input volume at each scale and fresh scratch space; pair order was AB, BA, AB on both scales. A is the scored control and B is the candidate.

| Local practice, 20M events and 5M clicks | A wall | B wall |
|---|---:|---:|
| Pair 1 | 9.943 s | 9.342 s |
| Pair 2 | 10.270 s | 8.793 s |
| Pair 3 | 9.761 s | 8.663 s |
| Median | 9.943 s | 8.793 s |

Practice median elapsed time decreased 11.6%.

| Local final scale, 60M events and 15M clicks | A wall | B wall |
|---|---:|---:|
| Pair 1 | 33.593 s | 28.891 s |
| Pair 2 | 33.354 s | 27.912 s |
| Pair 3 | 31.711 s | 27.581 s |
| Median | 33.354 s | 27.912 s |

Final-scale median elapsed time decreased 16.3%. Peak candidate cgroup memory across the final runs was 12,309,655,552 bytes under the 24 GiB limit. Bidirectional EXCEPT ALL comparisons of both Parquet tables and the cross-table accounting checks passed in every pair. Practice totals were 3,368,309 sessions and 6,155,929,657 cents. Final totals were 10,067,851 sessions and 20,418,435,163 cents. The owner reported 15.6507 s on official practice, down from 17.14 s. The 8.69% official reduction was about 2.9 percentage points smaller than the 11.6% local practice reduction. The hidden run provides no stage timings to isolate that difference.

## Native identity resolution

Control A was the 15.6507 s official submission, 9776571dedb150b32836d8e0c1a10d547181edb6 (image 026dbd050fe8). The control profile split the existing wall timer immediately after creation of the DuckDB people table. Identity includes reading links, computing components, writing the lookup CSV and loading that table. CSV preparation includes the existing click/product/campaign cleaner and creation of its three views. Event CSV parsing runs later in the event stage.

| Control profile, practice seed 23 | Run 1 | Run 2 | Run 3 | Median |
|---|---:|---:|---:|---:|
| Identity resolution and lookup load | 3.610 s | 2.740 s | 2.824 s | 2.824 s |
| Click and dimension CSV preparation | 0.254 s | 0.213 s | 0.239 s | 0.239 s |

Candidate B (ae9ec3178f32c82c78e930a3b50944da3dcfc5a8) replaces the Python identity graph with a Rust executable. Its map interns exact ID slices from the link files into integer indices. Parent traversal and path compression use an array; each union selects the byte-wise smaller root. The executable releases its buffers after writing the canonical mapping. Identity memory scales with link-file bytes and the number of distinct linked IDs. The other CSV preparation code is unchanged.

For the end-to-end comparison, each image ran in a fresh no-network Linux AMD64 container, with eight CPUs, 24 GiB RAM, no swap, and fresh scratch. The control used its scored image. The candidate was built without cache from its exact Git archive. Both used the same generated volume for each scale. Run order within each of the three pairs was AB, BA, AB.

The public checks used the clean B image (e2b62e299797; 492,587,177 bytes). All 36 fixtures passed. Both dev tables matched the official schemas and all 117,038 user/day and 27,400 campaign/day rows, with zero duplicate keys. The separate lookup comparison uses the scored revision's identities and canonical_file functions on the same link inputs, loads both emitted mappings as two VARCHAR columns, and checks EXCEPT ALL in both directions. This includes linked IDs that never appear in the output tables.

| Practice, seed 23 (20M events, 5M clicks) | A wall | B wall | B identity | B CSV preparation |
|---|---:|---:|---:|---:|
| Pair 1 | 14.254 s | 11.242 s | 0.715 s | 0.311 s |
| Pair 2 | 16.020 s | 11.741 s | 0.751 s | 0.323 s |
| Pair 3 | 15.008 s | 12.408 s | 0.853 s | 0.332 s |
| Median | 15.008 s | 11.741 s | 0.751 s | 0.323 s |

The practice end-to-end median fell 21.8%. The separately profiled control identity median was 2.824 s, versus 0.751 s for B. Absolute times for unrelated stages varied across the profiling and paired runs. Use the fresh paired control for the end-to-end comparison.

| Final scale, seed 42 (60M events, 15M clicks) | A wall | B wall |
|---|---:|---:|
| Pair 1 | 44.210 s | 34.804 s |
| Pair 2 | 44.073 s | 39.157 s |
| Pair 3 | 44.067 s | 35.892 s |
| Median | 44.073 s | 35.892 s |

Final-scale elapsed time decreased 18.6%. Peak B container memory across these runs was 12,327,649,280 bytes, within the 24 GiB limit. Both identity maps matched exactly: 1,195,420 IDs in 383,629 components on practice; 3,594,075 IDs in 1,149,399 components on final scale. Both Parquet tables matched in both directions with EXCEPT ALL in all six pairs, and the cross-table accounting checks passed. Practice reconciled to 3,368,309 sessions and 6,155,929,657 cents; final scale reconciled to 10,067,851 sessions and 20,418,435,163 cents. The owner reported 13.3325 s for B on official practice, down from 15.6507 s for A. The official reduction was 14.81%, compared with 21.8% in the paired local practice benchmark. Hidden stage timings are unavailable, so the cause of the difference is unresolved.

## Native session labeling

Control A was ae9ec3178f32c82c78e930a3b50944da3dcfc5a8, scored at 13.3325 s on official practice (image e2b62e299797). To profile it, the SQL statements from the combined session timer were executed separately in the same order. One timer covered CREATE TABLE labeled, including lag, cumulative max and materialization. Another covered CREATE TABLE sessions, the per-person/per-start count and revenue sum. Dropping valid, defining the lazy user_daily view and its later export were outside both timers.

| Control, 20M practice seed 23 | Run 1 | Run 2 | Run 3 | Median |
|---|---:|---:|---:|---:|
| Session labeling | 2.639 s | 2.264 s | 2.091 s | 2.264 s |
| Per-session aggregation | 0.177 s | 0.161 s | 0.162 s | 0.162 s |

A preliminary single global sort with Arrow batch scans measured 2.154 s for labeling in one practice run. Candidate B, edd24d4f9f43317c69c33f02416a14231ab53f26, instead resolves surviving events to canonical people and routes them to hash partitions. Each worker groups using exact person bytes, sorts each person's timestamps and walks that list once. DuckDB consumes the labeled Arrow columns directly. Product category lookup occurs for purchases during daily aggregation. B's labeling timer includes native identity routing and Arrow registration; in A, the identity and product joins ran during event validation. The matched end-to-end measurements include all of this work in both releases.

B was clean-built without cache from its Git archive (image 1c4255bb37ac, 492,681,487 bytes). All 36 fixtures passed. The official dev checker matched both output schemas and all 117,038 user/day and 27,400 campaign/day rows, with no duplicate keys. Benchmarks used the unmodified scored image and the clean B image under the existing Linux/amd64 8-CPU, 24-GiB limits. Each pair used the same input volume and fresh scratch; execution order was AB, BA, AB at each scale.

| Practice, seed 23 (20M events, 5M clicks) | A wall | B wall | B labeling | B per-session aggregation |
|---|---:|---:|---:|---:|
| Pair 1 | 7.291 s | 5.363 s | 1.091 s | 0.182 s |
| Pair 2 | 6.844 s | 5.922 s | 1.330 s | 0.198 s |
| Pair 3 | 6.953 s | 5.175 s | 1.021 s | 0.175 s |
| Median | 6.953 s | 5.363 s | 1.091 s | 0.182 s |

Practice median elapsed time decreased 22.9%. These are same-day local container timings. The scored control's official hidden practice result was 13.3325 s.

| Final scale, seed 42 (60M events, 15M clicks) | A wall | B wall |
|---|---:|---:|
| Pair 1 | 24.016 s | 18.630 s |
| Pair 2 | 23.207 s | 20.016 s |
| Pair 3 | 23.263 s | 19.316 s |
| Median | 23.263 s | 19.316 s |

Final-scale median elapsed time decreased 17.0%. Peak B cgroup usage across those runs was 12,150,829,056 bytes under the 24 GiB limit. Both daily Parquet tables matched in both directions with EXCEPT ALL in all six pairs. Session and revenue reconciliation and campaign assist invariants passed. Separate untimed seed-23 runs exported all intermediate rows. The multiset of (person, session start, cents, category) matched for 19,535,163 labeled events, and the complete per-session table matched for 3,368,309 sessions. The owner reported 8.8148 s on official practice, down from 13.3325 s. The official reduction of 33.88% exceeded the paired local practice reduction of 22.9% by about 11 percentage points. The hidden run supplies no stage timings to identify the cause.

## Native click timeline joins

The control was edd24d4f9f43317c69c33f02416a14231ab53f26 (8.8148 s on official practice). Candidate a44693d5344aa3b83303b8e6929c168075c7e828 was built from its Git archive, image 01b36cc06e1a. It resolves campaign codes and canonical people during the native click scan and supplies typed Arrow batches to the existing timeline sort. This removes the cleaned click CSV round-trip and the DuckDB identity lookup table.

| Control operation | Practice seed 23 | Final seed 42 |
|---|---:|---:|
| Click preparation alone | 0.186 s | 0.648 s |
| Timeline joins, session union and materialization | 0.777 s | 4.062 s |
| Timeline sort and materialization | 0.615 s | 2.002 s |
| Ordered CSV export | 0.269 s | 0.745 s |
| Parallel Rust sweep | 0.300 s | 0.838 s |

Click preparation is the median of three standalone runs of the control cleaner on the original clicks and header-only dimensions. The remaining rows are one instrumented pass per scale. Materialize the original click joins and session union in one temporary table, then the ordered projection in a second table. Time the ordered COPY and the attribute subprocess separately. These extra writes isolate the operators and affect their absolute times. The following paired end-to-end runs use the uninstrumented images.

| Local container wall time | Practice control | Practice candidate | Final control | Final candidate |
|---|---:|---:|---:|---:|
| Pair 1 | 5.991 s | 5.396 s | 18.869 s | 14.677 s |
| Pair 2 | 5.923 s | 5.408 s | 16.953 s | 14.494 s |
| Pair 3 | 5.500 s | 5.172 s | 18.962 s | 16.744 s |
| Median | 5.923 s | 5.396 s | 18.869 s | 14.677 s |

Run scripts/local.ps1 for practice and final with the control image and candidate image. Use a unique output label per run, the same volume for both images, and order AB, BA, AB per scale. Each container received 8 CPUs, 24 GiB and fresh scratch. Candidate median time fell 8.9% on practice and 22.2% at final scale. Its combined native click preparation and join timer had medians of 0.540 s and 1.548 s. Maximum final-scale candidate memory was 9,897,250,816 bytes.

All 36 fixtures passed. The official dev checker matched 117,038 user/day rows and 27,400 campaign/day rows with zero duplicate keys. Both daily tables matched in both directions with EXCEPT ALL in every large pair. Session, revenue and assist checks passed on both scales. An additional untimed seed-23 comparison matched all 5,131,997 joined click rows, including multiplicities, by click ID, canonical person, timestamp and campaign ID. The owner reported 8.7559 s on official practice, down from 8.8148 s (0.67%). The local practice median fell 8.9%. The official run provided no stage timings to explain the difference.

## Shared identity lookup for session routing

Control a44693d5344aa3b83303b8e6929c168075c7e828 scored 8.7559 s on official practice. In a diagnostic build of its Rust library, time loading and indexing the resolver CSV plus routing all winner batches. Then group and sort every partition before a separate pass assigns start labels and writes the Arrow column buffers. Time the Python Arrow wrappers separately and include them with buffer construction. These barriers extend intermediate lifetimes. Use the uninstrumented images for end-to-end comparisons.

| Control session labeling stage | Practice, seed 23 (median of 3) | Final, seed 42 (one run) |
|---|---:|---:|
| Identity lookup and routing | 0.609 s | 2.325 s |
| Per-person grouping and sorting | 0.374 s | 1.282 s |
| Label scan, Arrow buffer construction and wrappers | 0.155 s | 0.467 s |

Candidate 0cdcdc9cacfe0ad20489568b7645917acabd7f3a builds one native identity index from the link files. Indexed names occupy a contiguous byte buffer. Union-find chooses the bytewise smallest name in each component, and a final compression pass fixes each ID's canonical root. Event routing uses the canonical name's stored hash to select a partition. Click preparation queries the same immutable index. This removes the identity CSV export and both subsequent index builds. The diagnostic identity exporter uses the same resolver.

The production images were the scored control (01b36cc06e1a) and the candidate built from its Git archive (20e685a6c5c8). Run scripts/local.ps1 practice and final for each image with a fresh output label and scratch space, 8 CPUs, 24 GiB, and the same input volume. Execution order by pair was AB, BA, AB at each scale.

| Container wall time | Practice control | Practice candidate | Final control | Final candidate |
|---|---:|---:|---:|---:|
| Pair 1 | 5.283 s | 4.485 s | 14.741 s | 13.467 s |
| Pair 2 | 4.982 s | 4.546 s | 14.372 s | 14.128 s |
| Pair 3 | 4.882 s | 4.656 s | 14.546 s | 13.458 s |
| Median | 4.982 s | 4.546 s | 14.546 s | 13.467 s |

Median end-to-end wall time fell 8.8% on practice and 7.4% at final scale. Median session labeling time fell from 1.076 s to 0.856 s on practice and from 3.532 s to 3.270 s at final scale. Maximum final-scale container memory rose from 9,920,815,104 to 10,361,298,944 bytes, within the 24 GiB limit.

All 36 fixtures passed. The official dev checker matched both schemas and all 117,038 user/day and 27,400 campaign/day rows, with zero duplicate keys. Both Parquet tables matched as multisets in both directions for all six pairs, and all session, revenue and assist checks passed.
A candidate timing probe measured shared lookup and routing at 0.430 s on practice (median of three) and 1.863 s at final scale (one run). A separate bidirectional multiset comparison matched the complete identity maps for 1,195,420 IDs in 383,629 components on practice and 3,594,075 IDs in 1,149,399 components at final scale. The owner reported 7.6800 s on official practice, a 12.29% reduction from 8.7559 s. The corresponding paired local practice median fell 8.8%.

## Event parsing and partitioning

Control 0cdcdc9cacfe0ad20489568b7645917acabd7f3a scored 7.6800 s on official practice. An instrumented copy of its native library first read and partitioned all input ranges, then selected the earliest-ingested copy of every exact event ID, then constructed the winner output columns. Each phase joined its workers before starting the next timer. This extends the lifetime of winner references in the diagnostic build. The production benchmark uses the original fused selection and output loop.

| Initial control profile | Practice, seed 23 (median of 3) | Final, seed 42 (one run) |
|---|---:|---:|
| Event parsing and partitioning | 1.959 s | 7.760 s |
| Duplicate winner selection | 0.314 s | 1.629 s |
| Output batch construction | 0.404 s | 1.202 s |

The first candidate, a450e7a86d4d75ca4013e32650ae0cddbd7b73f1, scans delimiters and quote counts in 32-byte blocks on CPUs with AVX2 and POPCNT, with a scalar fallback. It also caches validated UTC dates. It passed the public checks and all six large differential comparisons. Its local practice median was 11.090 s versus 10.969 s for the control. Final-scale medians were 34.881 s and 35.942 s respectively. The practice result did not qualify this candidate for promotion.

The second candidate, e901b8c968b58689388ae02879581efdec222307, retained raw ranges and stored field offsets. It assigned the largest remaining range to each free parser and overlapped the event read with identity resolution. Fixed-width F1 ingestion times permitted bytewise duplicate election and a same-UTC-day lateness shortcut. Both daily tables matched in all six pairs. The two-way intermediate multiset comparison matched 19,535,163 practice winner rows. It passed the public checkers and two parser regression tests, but end-to-end wall time increased at both scales.

| Second candidate | Practice control | Practice candidate | Final control | Final candidate |
|---|---:|---:|---:|---:|
| Pair 1 | 7.176 s | 8.225 s | 23.328 s | 25.919 s |
| Pair 2 | 7.368 s | 8.448 s | 22.241 s | 26.249 s |
| Pair 3 | 7.301 s | 8.428 s | 23.456 s | 25.069 s |
| Median | 7.301 s | 8.428 s | 23.328 s | 25.919 s |

Each series used fresh containers and scratch space, identical named input volumes, 8 CPUs, 24 GiB and no swap. Run scripts/local.ps1 practice and final on each release image, with distinct output labels and order AB, BA, AB. Check both Parquet tables in both directions with scripts/check_large.py. Compare within each series because the control's elapsed time varied between series.

A third practice-only pilot used compact event storage and routed the elected rows directly to session labeling. The three control times were 7.338, 8.131 and 8.147 s. The pilot took 8.486, 8.295 and 8.674 s in the respective pairs. It lost all three comparisons and did not advance to release verification. The 7.6800 s official control remains the selected revision.

A final stage-only check loaded the instrumented control and block scanner libraries into one container per input scale. Each call opened and released the full event batches with eight workers. Order was AB, BA, AB. Control median parsing, winner selection and batch construction were 1.170, 0.155 and 0.217 s on practice, and 3.507, 0.823 and 0.600 s at final scale. The block scanner reduced median parsing to 1.042 s and 2.950 s respectively. Its full-pipeline practice median remained slower in the separate uninstrumented release comparison above.

## Delimiter scanning, timestamp decoding and partition writes

Control 0cdcdc9cacfe0ad20489568b7645917acabd7f3a scored 7.6800 s on official practice. In a diagnostic copy of its native event reader, sample every 127th row within each input range. Time the buffered LF read and row copy, the seven-field split and quote validation, both timestamp decodes with the window/lateness check, and the compact partition byte and metadata appends. Subtract one empty clock measurement per timed interval. Event-type/price validation and event-ID hashing are outside these intervals. Samples are worker-local elapsed times, including any stalls. They are diagnostic estimates rather than additive pipeline wall times. The field/timestamp/write timers exclude malformed rows that return before their end. Three fresh event reads used each original input volume with eight workers.

| Median sampled nanoseconds per row | Practice (169,294 timed rows/run) | Final (507,615 timed rows/run) |
|---|---:|---:|
| Delimiter handling, including LF read and copy | 174.8 | 196.6 |
| LF read and copy alone | 53.3 | 64.8 |
| Field delimiters and quote validation alone | 121.5 | 131.8 |
| Timestamp decoding and time checks | 37.6 | 45.7 |
| Partition buffer writes | 147.1 | 134.3 |

Candidate 6832e74c86b7c7e611bebeb41f2ca33e544464f1 indexes comma/LF positions and per-record quote counts in blocks of complete rows from a buffered reader. Only records crossing an I/O buffer or file-range boundary require a separate row copy. A scalar path handles long records and CPUs without AVX2 and POPCNT. All quote bytes must be accounted for by complete field wrappers. Partition buffers reserve space in proportion to the range size and store a 32-bit probe hash. Winner selection still compares entire event IDs. Workers take the largest remaining range when free.

The first event-only pilot indexed a whole input range and did not improve median wall time. The streaming pilot reduced event-stage medians from 1.852 to 1.426 s on practice and 7.786 to 5.995 s at final scale. These diagnostic comparisons alternated AB, BA, AB within a single container per scale. The end-to-end release benchmark below uses separate containers and includes startup, both Parquet outputs and exit.

The scored control image was 20e685a6c5c8 and the candidate Git-archive image was 0bae5bf8f3fc. Use scripts/local.ps1 with the practice seed-23 and final seed-42 volumes, eight CPUs, 24 GiB and fresh scratch. Use a unique output label per run. Alternate AB and BA for six pairs per scale. Wall time includes container launch, execution and exit.

| Local container wall time | Practice control | Practice candidate | Final control | Final candidate |
|---|---:|---:|---:|---:|
| Pair 1 | 12.846 s | 11.031 s | 35.451 s | 28.872 s |
| Pair 2 | 16.239 s | 12.008 s | 32.614 s | 30.779 s |
| Pair 3 | 10.610 s | 10.964 s | 32.445 s | 31.445 s |
| Pair 4 | 12.351 s | 10.370 s | 31.299 s | 34.055 s |
| Pair 5 | 11.765 s | 10.363 s | 31.248 s | 28.344 s |
| Pair 6 | 13.533 s | 10.833 s | 30.989 s | 28.096 s |
| Median | 12.5985 s | 10.8985 s | 31.872 s | 29.8255 s |

End-to-end median runtime fell 13.5% on practice and 6.4% at final scale. All 36 official fixtures passed. Both official dev schemas and all 117,038 user/day and 27,400 campaign/day rows matched with zero duplicate keys. Both daily outputs matched the control in both directions with EXCEPT ALL in all twelve large pairs, and session, revenue and assist checks passed. A separate intermediate comparison matched 19,535,163 validated practice winner rows including multiplicities. A targeted collision check confirmed distinct event IDs with the same 32-bit hash remain separate and that an invalid earliest copy suppresses only its own ID. The owner reported 7.4790 s on official practice. The 2.62% improvement over 7.6800 s was smaller than the 13.5% reduction in the paired local practice benchmark.
The candidate won five of six full-pipeline pairs at each scale. Its median event deduplication and validation stage fell from 2.9500 to 1.7945 s on practice and from 6.844 to 4.831 s at final scale. Peak container memory across the final-scale runs was 10,068,660,224 bytes for the candidate and 10,267,873,280 bytes for the control.

## Full pipeline profile and in-memory click attribution

The control is 6832e74c86b7c7e611bebeb41f2ca33e544464f1, scored at 7.4790 s on official practice. The stage probe wrapped the scored Python entry point with monotonic timers and measured process and child CPU seconds. CPU seconds divided by elapsed seconds estimates average logical CPUs used within each stage. Three sequential runs at each scale used 8 CPUs, 24 GiB, the original seed-23 practice and seed-42 final inputs, and a fresh container and scratch each time. Times and CPU utilization are separate medians. Click preparation, timeline export and the sweep are nested under attribution. The two Parquet writes are nested under daily outputs. The output timers include execution of their lazy views. Container startup and cleanup are included in the end-to-end benchmark.

| Control stage | Practice seconds | Practice CPUs | Final seconds | Final CPUs |
|---|---:|---:|---:|---:|
| identity resolution and lookup table | 0.361 | 0.98 | 1.67 | 1 |
| dimension CSV preparation | 0.007 | 0.71 | 0.006 | 0.83 |
| event dedup and validation | 1.024 | 7.69 | 2.895 | 7.75 |
| session labeling | 1.15 | 7.06 | 4.468 | 7.51 |
| per-session aggregation | 0.278 | 7.81 | 0.74 | 7.81 |
| clicks and attribution | 1.697 | 6.69 | 5.094 | 7.41 |
| native click preparation and timeline joins | 0.472 | 4.86 | 1.337 | 6.74 |
| sort and export timeline | 0.924 | 7.49 | 2.841 | 7.76 |
| timeline sweep | 0.295 | 7.21 | 0.913 | 7.63 |
| daily outputs | 0.614 | 3.67 | 1.811 | 4.34 |
| user_daily | 0.585 | 3.78 | 1.779 | 4.4 |
| campaign_daily | 0.027 | 1.61 | 0.029 | 1.64 |

Click attribution was the largest stage on both scales. The scored implementation globally sorted the combined session/click timeline and exported it to CSV. Candidate ba1c45ec10eb2051ff7913d88c1c84f932978f63 routes clicks to the same 256 canonical-person partitions already used for session labels. Each worker sorts clicks within each person and sweeps their sessions in time order. It accumulates campaign/day totals in memory, then the workers merge their totals. The seven-day window includes both endpoints. A bytewise click-ID tie break, per-campaign active counts, and organic fallback preserve credit and assist semantics.

The paired release benchmarks used image 0bae5bf8f3fc for the control and the archived Git revision built as image 740fb53a9809 for the candidate. Run scripts/local.ps1 for each image on the same named volume, with fresh scratch space and distinct output folders. Alternate control-first and candidate-first for six pairs at each scale. Container wall time includes startup, all computation, both Parquet writes, and exit.

| Supplied generator, container wall seconds | Practice control | Practice candidate | Final control | Final candidate |
|---|---:|---:|---:|---:|
| Pair 1 | 5.858 | 5.221 | 18.370 | 17.354 |
| Pair 2 | 6.079 | 5.700 | 15.733 | 15.300 |
| Pair 3 | 5.766 | 5.114 | 16.273 | 14.159 |
| Pair 4 | 5.624 | 5.294 | 16.012 | 12.852 |
| Pair 5 | 5.859 | 5.539 | 16.415 | 13.530 |
| Pair 6 | 5.794 | 5.201 | 15.336 | 13.623 |
| Median | 5.8260 | 5.2575 | 16.1425 | 13.8910 |

All twelve pairs favored the candidate. Attribution medians fell from 1.7565 to 1.1565 seconds on practice and 4.935 to 2.537 seconds at final scale. End-to-end medians fell 9.8% and 13.9% respectively. Maximum final-scale container memory was 10,312,753,152 bytes for the candidate and 10,167,291,904 bytes for the control. Both daily tables matched by two-way EXCEPT ALL in all twelve pairs, and session, revenue and assist checks passed.

On November 22 the organizer listed scorer shopping-sessions-5 and a revised generator with variable-length opaque IDs. The original supplied fixtures, generator and dev references remain the comparison set above. The candidate passed all 36 fixtures and matched all 117,038 user/day and 27,400 campaign/day rows in the supplied dev references.

For the current official dev check, build the generator from the track page with Rust 1.85 and generate seed 1 with 1M events and 250k clicks. The official checker matched all 118,107 user/day and 27,449 campaign/day rows with zero duplicate keys. The downloaded generator ZIP has SHA-256 78f973f75d14b48e792628284f19c75de71fa597d20952e16a9ced93c78bee4a. The dev-answer ZIP has SHA-256 c2439e0e2e7456d721852814e9928e077648803d5c57a1760a40c7d3b0864d20. [Organizer track and downloads](https://www.joinbuilders.org/buildathon/tracks/shopping-sessions)

For current-generator practice, generate seed 23 with 20M events and 5M clicks, and run four pairs in AB, BA, AB, BA order under the same container limits. Control wall times were 7.020, 6.541, 6.245 and 5.601 s. Candidate times were 5.892, 5.442, 5.342 and 4.888 s. The median fell from 6.393 to 5.392 s (15.7%). Both daily outputs matched in all four pairs. The owner reported 6.2805 s on official practice, down from 7.4790 s. The 16.02% reduction was close to the 15.7% measured on current-generator practice.

A separate worker-count experiment ran the scored control on the current seed-23 practice volume. Both the CPU quota and NCPU were set to 1, 4, 8, 8, 4, 1 in sequence, with all other limits unchanged. Median start-to-exit times from the two runs per setting were 23.6125 s, 7.6365 s and 5.5570 s respectively. Eight workers gave a 4.25x end-to-end speedup over one. The organizer limits every submission to eight pinned vCPUs. The public board showed a leading time of 0.4293 s when checked, without implementation details that would identify the team's strategy. [Official track and board](https://www.joinbuilders.org/buildathon/tracks/shopping-sessions)

## Session totals during labeling

Candidate 505f3edac7f245782df1f48866e5ef19c30f2b89 was compared with ba1c45ec10eb2051ff7913d88c1c84f932978f63, which scored 6.2805 s on official practice. Three separate baseline profiles measured median session labeling and identity resolution at 1.099 s and 0.345 s on practice, and 4.074 s and 1.660 s at final scale.

Use the current generator identified above with seed 23 (20M events, 5M clicks) and seed 42 (60M events, 15M clicks). Mount each input read-only for both revisions, with eight CPUs, 24 GiB memory and eight workers. Time the full container invocation with six paired runs per scale, alternating control-first and candidate-first. The control and candidate images were built from the respective revisions. Times below are seconds.

| Scale | Control wall times | Candidate wall times | Control median | Candidate median | Paired wins |
|---|---|---|---:|---:|---:|
| practice | 4.865, 4.707, 4.522, 4.695, 5.091, 4.714 | 4.557, 4.366, 4.248, 4.352, 4.563, 4.519 | 4.7105 | 4.4425 | 6/6 |
| final | 13.815, 13.172, 13.601, 13.076, 13.493, 15.032 | 13.794, 13.127, 12.6, 12.22, 13.2, 12.773 | 13.547 | 12.95 | 6/6 |

On practice the labeling median went from 1.065 to 1.019 s. Maximum container memory went from 4501827584 to 3806507008 bytes. On final the labeling median went from 3.8965 to 3.7665 s. Maximum container memory went from 11260833792 to 10296082432 bytes.

The exact candidate passed all 36 fixtures. The supplied dev references matched 117,038 user/day and 27,400 campaign/day rows. The current references matched 118,107 and 27,449 rows respectively. Both tables had zero unmatched rows and duplicate keys on both dev datasets. All twelve large comparisons matched both daily tables in both directions with EXCEPT ALL and passed the session, revenue and assist checks.

On current-generator practice, the control emitted 19,535,729 event labels and the candidate emitted 3,491,237 rows. A separate two-way EXCEPT ALL comparison matched every one of the 3,396,494 per-session summaries (person, start, event count, revenue) and 557,914 positive purchases (person, start, cents, product), including multiplicity.
