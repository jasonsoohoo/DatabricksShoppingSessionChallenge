# Pipeline

1. Union-find resolves links before joins. Path compression and byte-wise minimum roots preserve names contributed by link-only IDs. Unlinked IDs fall back to their original bytes.
2. A std-only Rust reader scans LF-delimited byte records on up to NCPU workers, largest files first. It checks the entire field layout before removing wrapping quotes and preserves CR in values. Timestamps use checked ASCII arithmetic and explicit Gregorian and offset limits.
3. Event workers retain the event ID and original F1 ingestion time even on invalid copies. Valid copies carry only the columns needed downstream. Exact event IDs hash to bounded scratch partitions. DuckDB picks the earliest copy within each partition before it removes invalid winners. Unknown purchased products map to the literal unknown category.
4. SQL orders events per canonical person. Lag identifies gaps over 30 minutes. A range window assigns the same start to equal timestamps. All aggregates use the UTC start date, including purchases after midnight.
5. Click processing discards invalid IDs/times and unknown campaigns. Identical copies have the same winner and assist semantics. DuckDB exports a timeline ordered by person, timestamp, and descending click ID, with the session after clicks at the same time. A Rust sweep expires clicks older than seven days, credits the last eligible click, and counts one assist per active campaign. Per-campaign reference counts preserve exact semantics for repeat clicks. Campaign/day totals include assist-only rows and organic sessions.
6. The runtime writes one typed Parquet file per table. A per-run scratch directory holds intermediates and DuckDB spills. The host runner caps CPU and RAM, disables swap and network, and uses an unprivileged UID.

The exact local baseline remains recoverable in Git. Implementation-specific performance and known scale failures are recorded in BENCHMARKS.md.
