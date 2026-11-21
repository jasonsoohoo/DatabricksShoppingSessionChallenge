# Shopping Sessions

The scorer builds the root Dockerfile and runs its default command. Development uses PowerShell 7 and Docker Desktop in Linux container mode. The production image contains DuckDB 1.5.6. The separate validator image contains Polars 2.1.0.

## Restore local materials

Run in the repository root with the supplied archives in Downloads. These commands create inputs for development on a fresh checkout.

```powershell
New-Item -Force -ItemType Directory .local
tar -xf "$HOME/Downloads/generator.zip" -C .local
tar -xf "$HOME/Downloads/dev-answer.zip"
tar -xf "$HOME/Downloads/starter-duckdb.zip" --strip-components 1 starter-duckdb/fixtures
$local = (Resolve-Path .local).Path
docker run --rm --cpus 4 --memory 4g -v "${local}:/work" -w /work/generator rust:1.85-bookworm sh -ec 'cargo build --release --offline && target/release/generate --threads 4 --seed 1 --events 1000000 --clicks 250000 --output /work/dev && target/release/generate --threads 4 --seed 7 --events 100000 --clicks 25000 --output /work/small'
```

## Build, test and benchmark

```powershell
./scripts/local.ps1 build
./scripts/local.ps1 fixtures
./scripts/local.ps1 dev
./scripts/local.ps1 compare
./scripts/local.ps1 small
```

The dev comparison checks both Parquet schemas, row membership and unique keys against the published CSVs. Fixtures invoke the production entry point in the checker image. Benchmarks invoke the production image directly. The optional second argument labels each output directory, e.g. use the same label for dev and compare. Use a fresh label for an independent run.

For the large benchmarks, generate input into dedicated Docker volumes once. The named volumes keep large input reads on the Linux filesystem.

```powershell
$bin = (Resolve-Path .local/generator/target/release).Path
docker run --rm --cpus 4 --memory 10g -v "${bin}:/bin-gen:ro" -v shopping-buildathon-practice:/practice -v shopping-buildathon-final:/final rust:1.85-bookworm sh -ec '/bin-gen/generate --threads 4 --seed 23 --events 20000000 --clicks 5000000 --output /practice && /bin-gen/generate --threads 4 --seed 42 --events 60000000 --clicks 15000000 --output /final'
./scripts/local.ps1 practice
./scripts/local.ps1 final
```

The official POSIX Makefile and Docker wrapper are also included. For current candidate hashes, measured results and manual hand-in, see [submissions](docs/SUBMISSIONS.md). Project progress is in [the roadmap](docs/ROADMAP.md).
