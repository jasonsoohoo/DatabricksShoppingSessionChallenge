"""Exercise timeline range cuts that the small official fixtures cannot reach."""
import os
import subprocess
import tempfile
from pathlib import Path


def check(binary="/app/attribute"):
    with tempfile.TemporaryDirectory(dir=os.environ.get("SCRATCH_DIR")) as work:
        timeline = Path(work) / "timeline.csv"
        with timeline.open("wb") as out:
            # One person spans many target cuts. Each campaign assists once.
            out.write(b"a,0,1,0\na,0,2,0\n" * 250_000)
            out.write(b"a,0,0,123\na,604800000,0,0\na,604800001,0,0\n")
            for index in range(100_000):
                uid = f"b{index:06d}"
                out.write(f"{uid},0,1,0\n{uid},0,0,1\n".encode())
        expected = sorted([
            "0,7,1,0,0,0",
            "1,0,100000,100000,100000,100001",
            "1,7,0,0,0,1",
            "2,0,1,1,123,1",
            "2,7,1,0,0,1",
        ])
        for workers in (1, 8):
            totals = Path(work) / f"totals-{workers}.csv"
            subprocess.run([binary, timeline, totals], check=True,
                           env=dict(os.environ, NCPU=str(workers)))
            rows = sorted(totals.read_text().splitlines())
            if rows != expected:
                raise AssertionError(f"{workers} workers: {rows} != {expected}")
    print("PASS: single-worker and parallel totals match expected rows")


if __name__ == "__main__":
    check()
