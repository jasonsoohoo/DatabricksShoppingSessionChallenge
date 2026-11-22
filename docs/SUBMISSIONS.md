# Submission register

## Best available submission

**Selected local SHA:** `a44693d5344aa3b83303b8e6929c168075c7e828`. Native click joins passed 36/36 fixtures, both official dev comparisons and six paired large output comparisons. Local median wall times were 5.396 s on practice and 14.677 s at final scale. Official score pending. The best official practice score remains 8.8148 s for `edd24d4f9f43317c69c33f02416a14231ab53f26`.

Run `git rev-parse best-verified` for the selected local submission SHA. BENCHMARKS.md records the implementation, local verification and measurements. Each tested implementation remains in the history of main. The development checkout may contain subsequent ledger or benchmark updates.

## Official results

Attempt 1, reported by the owner on 2026-11-21: **24.98 s** on official practice for `eb3762be83c1b0ee58826a8d674070f3a87fcf30`. This was our first official result. The owner reported a leaderboard lead of 0.27 s at the time.

Attempt 2, reported by the owner on 2026-11-21, scored **17.14 s** on official practice for `c19ddd40924f3180f7ef5bb34f61a7bc9dec7f50`. This reduced runtime by 7.84 s (31.4%) from attempt 1.

Attempt 3, reported by the owner on 2026-11-21, scored **15.6507 s** on official practice for `9776571dedb150b32836d8e0c1a10d547181edb6`. This reduced runtime by 1.4893 s (8.69%) from attempt 2.

Attempt 4, reported by the owner on 2026-11-21, scored **13.3325 s** on official practice for `ae9ec3178f32c82c78e930a3b50944da3dcfc5a8`. It improved on attempt 3 by 2.3182 s (14.81%).

Attempt 5, reported by the owner on 2026-11-21, scored **8.8148 s** on official practice for `edd24d4f9f43317c69c33f02416a14231ab53f26`. This is our best official result, 4.5177 s (33.88%) below attempt 4.

## Manual hand-in

Before the cutoff, the owner publishes the repository to public GitHub, including the selected commit and all previously submitted commits. On the Shopping Sessions track, enter the public repository URL and exact SHA, then submit. Check that the selected final revision is fetchable. Keep the public history through publication of results. Report the SHA, scorer version, score or failure, and timestamp here after each official attempt. Update the best official SHA only when the scorer confirms a better result.

The starter team metadata is blank. Supply actual team details if used for registration.

## Recoverable local candidates

All revisions below passed the official local fixtures and dev comparison. BENCHMARKS.md records their runs and resource measurements. The native session labeling release holds our best official score of 8.8148 s.

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
| edd24d4f9f43317c69c33f02416a14231ab53f26 | Native session labeling. Best official practice score, 8.8148 s. |
| a44693d5344aa3b83303b8e6929c168075c7e828 | Best verified local release. Native click timeline joins. Official practice score pending. |
