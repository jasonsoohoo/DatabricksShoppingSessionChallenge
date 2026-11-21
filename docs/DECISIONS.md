# Decisions

- Start with Python and DuckDB for exact SQL aggregates, typed Parquet and disk spill. Use the supplied Polars checker as an independent implementation of the comparison.
- Introduce a Rust reader after profiling attributed 5.056 s of the 7.847 s dev baseline to CSV and identity preparation. Keep Rust out of the final runtime image.
- Preserve invalid dedup winners with a key and ingestion time. Early validation is safe only when it cannot rescue a later copy.
- Partition by event ID before selecting winners. The unpartitioned query reached DuckDB’s 16 GiB limit on the 60M-event input. Hashes choose a partition only. The database still compares complete IDs.
- Use actual generated final-scale runs to select candidates. Short dev runs are dominated by startup and filesystem variation. Preserve larger outputs for differential checks where official answers are unavailable.
- Replace the SQL session/click range join with a time-ordered sweep. The 20M-event run dropped from 22.107 to 15.072 seconds. This also avoids building a table proportional to the number of qualifying session/click pairs.
