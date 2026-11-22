# Submission register

## Best available submission

**Selected local SHA:** `ba1c45ec10eb2051ff7913d88c1c84f932978f63`. In-memory per-person attribution passed the public fixtures, both versions of the official dev checks, and all sixteen paired large output comparisons. Local medians on the original volumes were 5.2575 s on practice and 13.8910 s at final scale. Current-generator practice median was 5.392 s. Official score pending. The best reported official practice score is 7.4790 s for `6832e74c86b7c7e611bebeb41f2ca33e544464f1`.

Run `git rev-parse best-verified` for the selected local submission SHA. BENCHMARKS.md records the implementation, local verification and measurements. Each tested implementation remains in the history of main. The development checkout may contain subsequent ledger or benchmark updates.

## Official results

Attempt 1, reported by the owner on 2026-11-21: **24.98 s** on official practice for `eb3762be83c1b0ee58826a8d674070f3a87fcf30`. This was our first official result. The owner reported a leaderboard lead of 0.27 s at the time.

Attempt 2, reported by the owner on 2026-11-21, scored **17.14 s** on official practice for `c19ddd40924f3180f7ef5bb34f61a7bc9dec7f50`. This reduced runtime by 7.84 s (31.4%) from attempt 1.

Attempt 3, reported by the owner on 2026-11-21, scored **15.6507 s** on official practice for `9776571dedb150b32836d8e0c1a10d547181edb6`. This reduced runtime by 1.4893 s (8.69%) from attempt 2.

Attempt 4, reported by the owner on 2026-11-21, scored **13.3325 s** on official practice for `ae9ec3178f32c82c78e930a3b50944da3dcfc5a8`. It improved on attempt 3 by 2.3182 s (14.81%).

Attempt 5, reported by the owner on 2026-11-21, scored **8.8148 s** on official practice for `edd24d4f9f43317c69c33f02416a14231ab53f26`. This was 4.5177 s (33.88%) below attempt 4.

Attempt 6, reported by the owner on 2026-11-21, scored **8.7559 s** on official practice for `a44693d5344aa3b83303b8e6929c168075c7e828`. This is 0.0589 s (0.67%) below attempt 5 and 64.95% below the first run of 24.98 s.

Attempt 7, reported by the owner on 2026-11-22, scored **7.6800 s** on official practice for `0cdcdc9cacfe0ad20489568b7645917acabd7f3a`. Runtime fell by 1.0759 s (12.29%) from attempt 6 and by 17.3000 s (69.26%) from the first run.

Attempt 8, reported by the owner on 2026-11-22, scored **7.4790 s** on official practice for `6832e74c86b7c7e611bebeb41f2ca33e544464f1`. Runtime fell by 0.2010 s (2.62%) from attempt 7 and by 17.5010 s from the first run of 24.98 s.

## Manual hand-in

Before the cutoff, the owner publishes the repository to public GitHub, including the selected commit and all previously submitted commits. On the Shopping Sessions track, enter the public repository URL and exact SHA, then submit. Check that the selected final revision is fetchable. Keep the public history through publication of results. Report the SHA, scorer version, score or failure, and timestamp here after each official attempt. Update the best official SHA only when the scorer confirms a better result.

The starter team metadata is blank. Supply actual team details if used for registration.

## Recoverable local candidates

All revisions below passed the official local fixtures and dev comparison. BENCHMARKS.md records their runs and resource measurements. The buffered delimiter index release holds our best official score of 7.4790 s.

| SHA | Implementation and status |
|---|---|
| f9c918fb040bd64bcd04fc3d2377a510fef82b1a | Python/DuckDB baseline |
| 3112f1cb9fdca6d1557ace281f19f6b9af8ea4ec | Parallel native reader. Final-scale dedup ran out of memory. |
| 220b77b7338f6a4b866ba56f37f9620d96f980ab | Compact native event validation. Tested on small and dev. |
| 51ba46ccf2e58e463f3909b7f16dd929154fbc1b | Partitioned dedup, SQL attribution. Large differential reference. |
| 9a8ec0b70cf4ff3088a19eb9bfb7fc1a41f0d6b7 | SQL priority aggregation. Slower on final scale. |
| 108514a89e2489f8d6469623a7c4f46a6a24517c | Sliding-window attribution. Pre-release benchmark. |
| eb3762be83c1b0ee58826a8d674070f3a87fcf30 | First official release. Official practice 24.98 s. |
| c19ddd40924f3180f7ef5bb34f61a7bc9dec7f50 | Native event partitions and Arrow buffers. Official practice score, 17.14 s. |
| 9776571dedb150b32836d8e0c1a10d547181edb6 | Parallel attribution sweep. Official practice score, 15.6507 s. |
| ae9ec3178f32c82c78e930a3b50944da3dcfc5a8 | Native identity resolution. Official practice score, 13.3325 s. |
| edd24d4f9f43317c69c33f02416a14231ab53f26 | Native session labeling. Official practice score, 8.8148 s. |
| a44693d5344aa3b83303b8e6929c168075c7e828 | Native click timeline joins. Official practice score, 8.7559 s. |
| 0cdcdc9cacfe0ad20489568b7645917acabd7f3a | Shared identity index. Official practice score, 7.6800 s. |
| a450e7a86d4d75ca4013e32650ae0cddbd7b73f1 | Block event scanner. Passed public and large checks. Slower on practice. Local experiment. |
| e901b8c968b58689388ae02879581efdec222307 | Raw-range event partitions. Passed public and large checks. Slower on both scales. Local experiment. |
| 6832e74c86b7c7e611bebeb41f2ca33e544464f1 | Buffered delimiter index. Best official practice score, 7.4790 s. |
| ba1c45ec10eb2051ff7913d88c1c84f932978f63 | Best verified local release. In-memory per-person attribution. Official practice score pending. |
