"""Shopping Sessions entry point. Paths may also be supplied by the fixture runner."""

import json
import os
import resource
import subprocess
import sys
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager
from datetime import datetime
from pathlib import Path

import duckdb
from events import ClickBatches, EventBatches, People


@contextmanager
def stage(name):
    start = time.monotonic()
    yield
    print(f"{name} {time.monotonic() - start:.3f}s", file=sys.stderr)


def literal(value):
    return "'" + str(value).replace("'", "''") + "'"


def relation(path, names):
    columns = ", ".join(f"'{name}':'VARCHAR'" for name in names.split())
    return (f"read_csv({literal(path)}, columns={{{columns}}}, auto_detect=false, "
            "header=false, delim=',', quote='\x22', escape='\x22', new_line='\\n', parallel=true)")


def run(data, output, work):
    con = duckdb.connect(config={
        "threads": os.getenv("NCPU", "8"),
        "memory_limit": f"{max(1, int(os.getenv('MEM_GB', '24')) * 0.72)}GB",
        "temp_directory": str(work / "spill"),
        "preserve_insertion_order": "false",
    })
    manifest = json.loads((data / "manifest.json").read_text())
    start, end = [int(datetime.fromisoformat(manifest[k]).timestamp()) * 1000
                  for k in ("window_start", "window_end")]
    execute = con.execute
    workers = int(os.getenv("NCPU", "8"))
    # Only session labeling needs both inputs. Parse events while building the
    # identity index, reserving one CPU for the single-threaded resolver.
    with stage("identity resolution and event validation"):
        with ThreadPoolExecutor(max_workers=1) as pool:
            pending = pool.submit(People, data / "links")
            if workers == 1:
                pending.result()
            winners = EventBatches(data / "events", start, end, max(1, workers - 1))
            people = pending.result()
            del pending

    with stage("dimension CSV preparation"):
        subprocess.run(["/app/clean", data, work], check=True)

        for table, fields in {
            "products": "product category",
            "campaigns": "campaign channel",
        }.items():
            path = work / table / "*.csv"
            execute(f"create view raw_{table} as select * from " + relation(path, fields))

    with stage("session labeling"):
        con.register("labeled", winners.labeled_table(people, workers))
        del winners
    with stage("per-session aggregation"):
        execute("""
        create table sessions as select person, begin_ms, epoch_ms(begin_ms)::date as day,
          count(*) as events, sum(cents)::bigint as revenue
          from labeled group by person,begin_ms;
        """)
    execute("""
        create view user_daily as
        with categories as (
          select person, epoch_ms(begin_ms)::date as day, coalesce(category, 'unknown') as category, sum(cents) as total
          from labeled left join raw_products using(product) where cents>0
          group by person, day, coalesce(category, 'unknown')
        ), top_categories as (
          select person, day, category from categories
          qualify row_number() over(partition by person,day order by total desc, category)=1
        )
        select person as user_id, day, count(*) as sessions, sum(events)::bigint as events,
               sum(revenue)::bigint as revenue_cents, max(category) as top_category
        from sessions left join top_categories using(person, day) group by person,day;
        """)
    with stage("clicks and attribution"):
        timeline = work / "timeline.csv"
        totals = work / "totals.csv"
        execute("create table codes as select *, row_number() over() as code from raw_campaigns")
        code_path = work / "codes.csv"
        execute(f"copy (select campaign, code from codes) to {literal(code_path)} (format csv, header false)")
        with stage("native click preparation and timeline joins"):
            clicks = ClickBatches(data / "clicks", people, code_path, start, end, workers)
            con.register("prepared_clicks", clicks.table())
            del clicks, people
        execute("set preserve_insertion_order=true")
        execute(f"""
          copy (
            select person,t,code,revenue from (
              select person, t, code, 0 as revenue, cid
              from prepared_clicks
              union all
              select person, begin_ms, 0, revenue, null from sessions
            ) order by person, t, cid desc nulls last
          ) to {literal(timeline)} (format csv, header false)
        """)
        con.unregister("prepared_clicks")
        subprocess.run(["/app/attribute", timeline, totals], check=True)
        source = relation(totals, "code days sessions purchasing_sessions revenue_cents assisted_sessions")
        execute(f"""
          create view campaign_daily as
          select coalesce(campaign,'organic') as campaign_id, coalesce(channel,'organic') as channel,
            date '1970-01-01' + days::int as day, sessions::bigint as sessions,
            purchasing_sessions::bigint as purchasing_sessions, revenue_cents::bigint as revenue_cents,
            assisted_sessions::bigint as assisted_sessions from {source} left join codes using(code)
        """)
    with stage("daily outputs"):
        for name in ("user_daily", "campaign_daily"):
            folder = output / name
            folder.mkdir(parents=True, exist_ok=True)
            execute(f"copy {name} to {literal(folder / 'part.parquet')} (format parquet)")
    con.close()


if __name__ == "__main__":
    data, output = (map(Path, sys.argv[1:3]) if len(sys.argv) == 3 else
                    (Path(os.getenv("DATA_DIR", "/data")), Path(os.getenv("OUTPUT_DIR", "/output"))))
    began = time.monotonic()
    with tempfile.TemporaryDirectory(dir=os.getenv("SCRATCH_DIR")) as scratch:
        run(data, output, Path(scratch))
    usage = resource.getrusage(resource.RUSAGE_SELF)
    child = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = usage.ru_utime + usage.ru_stime + child.ru_utime + child.ru_stime
    print(f"wall={time.monotonic()-began:.3f}s cpu={cpu:.3f}s peak_rss={usage.ru_maxrss}KiB")
    peak = Path("/sys/fs/cgroup/memory.peak")
    if peak.is_file():
        print(f"container_peak_bytes={peak.read_text().strip()}")
