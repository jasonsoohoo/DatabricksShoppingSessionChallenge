"""Shopping Sessions entry point. Paths may also be supplied by the fixture runner."""

import json
import os
import re
import resource
import subprocess
import sys
import tempfile
import time
from contextlib import contextmanager
from datetime import datetime
from pathlib import Path

import duckdb


@contextmanager
def stage(name):
    start = time.monotonic()
    yield
    print(f"{name} {time.monotonic() - start:.3f}s", file=sys.stderr)


def literal(value):
    return "'" + str(value).replace("'", "''") + "'"


def records(files, width):
    field = rb'(?:"[^",\n]*"|[^",\n]*)'
    pattern = re.compile(field + (b',' + field) * (width - 1))
    for path in files:
        with path.open("rb") as source:
            next(source, None)
            for line in source:
                row = line.removesuffix(b"\n")
                if pattern.fullmatch(row):
                    yield row


def identities(data):
    parent = {}
    def root(key):
        while parent[key] != key:
            parent[key] = parent[parent[key]]
            key = parent[key]
        return key

    for row in records(data.glob("links/*.csv"), 2):
        a, b = row.replace(b'"', b'').split(b",")
        if a and b:
            parent.setdefault(a, a)
            parent.setdefault(b, b)
            left, right = sorted((root(a), root(b)))
            parent[right] = left
    return ((key, root(key)) for key in parent)


def canonical_file(path, rows):
    with path.open("wb") as target:
        for row in rows:
            # Quote every field containing CR so the SQL reader preserves it.
            if b"\r" in row:
                row = b",".join(b'"' + f.strip(b'"') + b'"' for f in row.split(b","))
            target.write(row + b"\n")


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
    size = sum(p.stat().st_size for p in data.glob("events/*.csv"))
    shards = max(1, (size + (512 << 20) - 1) // (512 << 20))
    execute = con.execute
    with stage("csv and identity"):
        path = work / "people.csv"
        canonical_file(path, (a + b"," + b for a, b in identities(data)))
        execute("create table people as select * from " + relation(path, "uid person"))
        subprocess.run(["/app/clean", data, work], check=True,
                       env=dict(os.environ, EVENT_SHARDS=str(shards), WINDOW_START=str(start), WINDOW_END=str(end)))
        for table, fields in {
            "clicks": "cid uid campaign ts",
            "products": "product category",
            "campaigns": "campaign channel",
        }.items():
            path = work / table / "*.csv"
            execute(f"create view raw_{table} as select * from " + relation(path, fields))

    with stage("event dedup and validation"):
        execute("create table valid(person varchar, t bigint, cents integer, category varchar)")
        for shard in range(shards):
            src = relation(work / "events" / f"*.{shard}.csv", "eid ingest uid ts price product")
            execute(f"""
            insert into valid
            with kept as (
              select distinct on(eid) * from {src} order by eid, ingest
            )
            select coalesce(person, uid), ts::bigint, price::int,
                   case when price!='0' then coalesce(category,'unknown') end
            from kept left join people using(uid) left join raw_products using(product)
            where uid is not null
            """)
    with stage("sessions and user daily"):
        execute("""
        create table labeled as
        with gaps as (
          select *, lag(t) over(partition by person order by t) as prev from valid
        )
        select person, cents, category, max(case when prev is null or t-prev>1800000 then t end)
          over(partition by person order by t range unbounded preceding) as begin_ms from gaps;
        drop table valid;
        create table sessions as select person, begin_ms, epoch_ms(begin_ms)::date as day,
          count(*) as events, sum(cents)::bigint as revenue
          from labeled group by person,begin_ms;
        create view user_daily as
        with categories as (
          select person, epoch_ms(begin_ms)::date as day, category, sum(cents) as total
          from labeled where cents>0 group by person, day, category
          qualify row_number() over(partition by person,day order by total desc, category)=1
        )
        select person as user_id, day, count(*) as sessions, sum(events)::bigint as events,
               sum(revenue)::bigint as revenue_cents, max(category) as top_category
        from sessions left join categories using(person, day) group by person,day;
        """)
    with stage("clicks and attribution"):
        execute("""
        create table clicks as
          select coalesce(person,uid) as person, campaign, ts::bigint as t, min(cid) as cid
          from raw_clicks join raw_campaigns using(campaign) left join people using(uid)
          group by all;
        create table matches as
          select s.person, s.begin_ms, c.campaign, min(struct_pack(neg_t := -t, id := cid)) as priority
          from sessions s join clicks c on s.person=c.person and c.t between s.begin_ms-604800000 and s.begin_ms
          group by s.person, s.begin_ms, c.campaign;
        create view campaign_daily as
        with winners as (
          select person, begin_ms, arg_min(campaign, priority) as campaign from matches group by person,begin_ms
        ), credited as (
          select coalesce(campaign,'organic') as campaign, day, count(*) as sessions,
            count(*) filter(where revenue>0) as purchasing_sessions, sum(revenue)::bigint as revenue_cents
          from sessions left join winners using(person,begin_ms) group by all
        ), assists as (
          select campaign, epoch_ms(begin_ms)::date as day, count(*) as assisted_sessions from matches group by all
        )
        select campaign as campaign_id, coalesce(channel,'organic') as channel, day,
          coalesce(sessions,0) as sessions, coalesce(purchasing_sessions,0) as purchasing_sessions,
          coalesce(revenue_cents,0) as revenue_cents, coalesce(assisted_sessions,0) as assisted_sessions
        from credited full join assists using(campaign,day) left join raw_campaigns using(campaign)
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
