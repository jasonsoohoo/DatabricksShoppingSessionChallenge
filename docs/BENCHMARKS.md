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
