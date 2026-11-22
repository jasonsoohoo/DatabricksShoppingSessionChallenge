# Submission register

## Best available submission

**Selected local SHA:** `9776571dedb150b32836d8e0c1a10d547181edb6`. The clean build passed 36/36 fixtures, both official dev comparisons, a person-boundary regression, and both large table comparisons in every paired run. Median practice time was 8.793 s and median final-scale time was 27.912 s. Official score pending. The best official score remains 17.14 s for `c19ddd40924f3180f7ef5bb34f61a7bc9dec7f50`.

Run `git rev-parse best-verified` for the selected local submission SHA. BENCHMARKS.md records the implementation, local verification and measurements. Each tested implementation remains in the history of main. The development checkout may contain subsequent ledger or benchmark updates.

## Official results

Attempt 1, reported by the owner on 2026-11-21: **24.98 s** on official practice for `eb3762be83c1b0ee58826a8d674070f3a87fcf30`. This was our first official result. The owner reported a leaderboard lead of 0.27 s at the time.

Attempt 2, reported by the owner on 2026-11-21, scored **17.14 s** on official practice for `c19ddd40924f3180f7ef5bb34f61a7bc9dec7f50`. This is our best official result, 7.84 s (31.4%) below attempt 1.

## Manual hand-in

Before the cutoff, the owner publishes the repository to public GitHub, including the selected commit and all previously submitted commits. On the Shopping Sessions track, enter the public repository URL and exact SHA, then submit. Check that the selected final revision is fetchable. Keep the public history through publication of results. Report the SHA, scorer version, score or failure, and timestamp here after each official attempt. Update the best official SHA only when the scorer confirms a better result.

The starter team metadata is blank. Supply actual team details if used for registration.

## Recoverable local candidates

All revisions below passed the official local fixtures and dev comparison. BENCHMARKS.md records their runs and resource measurements. The native event partition release holds our best official score of 17.14 s.

| SHA | Implementation and status |
|---|---|
| f9c918fb040bd64bcd04fc3d2377a510fef82b1a | Python/DuckDB baseline |
| 3112f1cb9fdca6d1557ace281f19f6b9af8ea4ec | Parallel native reader. Final-scale dedup ran out of memory. |
| 220b77b7338f6a4b866ba56f37f9620d96f980ab | Compact native event validation. Tested on small and dev. |
| 51ba46ccf2e58e463f3909b7f16dd929154fbc1b | Partitioned dedup, SQL attribution. Large differential reference. |
| 9a8ec0b70cf4ff3088a19eb9bfb7fc1a41f0d6b7 | SQL priority aggregation. Slower on final scale. |
| 108514a89e2489f8d6469623a7c4f46a6a24517c | Sliding-window attribution. Pre-release benchmark. |
| eb3762be83c1b0ee58826a8d674070f3a87fcf30 | First official release. Official practice 24.98 s. |
| c19ddd40924f3180f7ef5bb34f61a7bc9dec7f50 | Native event partitions and Arrow buffers. Best official practice score, 17.14 s. |
| 9776571dedb150b32836d8e0c1a10d547181edb6 | Best verified local release. Parallel attribution sweep. Awaiting official score. |
