"""Shopping Sessions entry point. Paths may also be supplied by the fixture runner."""

import json
import os
import re
import resource
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
            "header=false, delim=',', quote='\x22', escape='\x22', new_line='\\n', parallel=false)")


def run(data, output, work):
    con = duckdb.connect(config={
        "threads": os.getenv("NCPU", "8"),
        "memory_limit": f"{max(1, int(os.getenv('MEM_GB', '24')) * 0.72)}GB",
        "temp_directory": str(work / "spill"),
        "preserve_insertion_order": "false",
    })
    execute = con.execute
    with stage("csv and identity"):
        path = work / "people.csv"
        canonical_file(path, (a + b"," + b for a, b in identities(data)))
        execute("create table people as select * from " + relation(path, "uid person"))
        for table, fields in {
            "events": "eid uid ts ingest kind product price",
            "clicks": "cid uid campaign ts",
            "products": "product category",
            "campaigns": "campaign channel",
        }.items():
            files = [data / (table + ".csv")] if table in ("products", "campaigns") else data.glob(table + "/*.csv")
            path = work / (table + ".csv")
            canonical_file(path, records(files, len(fields.split())))
            execute(f"create view raw_{table} as select * from " + relation(path, fields))

    manifest = json.loads((data / "manifest.json").read_text())
    start, end = [int(datetime.fromisoformat(manifest[k]).timestamp()) * 1000
                  for k in ("window_start", "window_end")]
    execute("""
        create macro millis(s) as case
          when regexp_full_match(s, '[0-9]+') then try_cast(coalesce(nullif(ltrim(s, '0'), ''), '0') as bigint)
          when regexp_full_match(s, '[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-5][0-9]:[0-5][0-9](Z|[+-]([01][0-9]|2[0-3]):[0-5][0-9])')
            and substring(s,12,2) < '24' then
              epoch_ms(try_strptime(left(s,19), '%Y-%m-%dT%H:%M:%S')) -
              case when length(s)=20 then 0 else
                (case when substring(s,20,1)='-' then -1 else 1 end) *
                (try_cast(substring(s,21,2) as int)*60 + try_cast(right(s,2) as int))*60000 end
        end
    """)
    with stage("event dedup and validation"):
        execute(f"""
        create table valid as
        with kept as (
          select distinct on(eid) * from raw_events where eid is not null order by eid, ingest
        ), parsed as (
          select *, millis(ts) as t, case when kind='purchase' and regexp_full_match(price,'[0-9]+')
                then try_cast(ltrim(price,'0') as bigint) when kind!='purchase' then 0 end as cents
          from kept
        )
        select coalesce(person, uid) as person, t, cents,
               case when cents>0 then coalesce(category,'unknown') end as category
        from parsed left join people using(uid) left join raw_products using(product)
        where uid is not null and product is not null and kind in ('purchase','cart','view')
          and (kind!='purchase' or cents between 1 and 10000000) and t >= {start} and t < {end}
          and t >= epoch_ms(strptime(ingest, '%Y-%m-%dT%H:%M:%SZ')) - 86400000
        """)
    with stage("sessions and user daily"):
        execute("""
        create table labeled as
        with gaps as (
          select *, lag(t) over(partition by person order by t) as prev from valid
        )
        select *, max(case when prev is null or t-prev>1800000 then t end)
          over(partition by person order by t range unbounded preceding) as begin_ms from gaps;
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
        execute(f"""
        create table clicks as select distinct cid, coalesce(person,uid) as person, campaign, millis(ts) as t
        from raw_clicks join raw_campaigns using(campaign) left join people using(uid)
        where cid is not null and uid is not null and millis(ts)>={start - 604800000} and millis(ts)<{end};
        create table matches as select s.person, s.begin_ms, c.campaign,
          first(cid order by t desc, cid) as cid, max(t) as t
        from sessions s join clicks c on s.person=c.person and c.t between s.begin_ms-604800000 and s.begin_ms
        group by s.person, s.begin_ms, c.campaign;
        create view campaign_daily as
        with ranked as (
          select *, row_number() over(partition by person,begin_ms order by t desc,cid) as rank from matches
        ), totals as (
          select coalesce(campaign,'organic') as campaign, day,
            count(*) filter(where rank=1 or rank is null) as sessions,
            count(*) filter(where (rank=1 or rank is null) and revenue>0) as purchasing_sessions,
            sum(case when rank=1 or rank is null then revenue else 0 end)::bigint as revenue_cents,
            count(rank) as assisted_sessions
          from sessions left join ranked using(person,begin_ms) group by all
        )
        select campaign as campaign_id, coalesce(channel,'organic') as channel, day, sessions,
          purchasing_sessions, revenue_cents, assisted_sessions from totals left join raw_campaigns using(campaign)
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
    print(f"wall={time.monotonic()-began:.3f}s cpu={usage.ru_utime+usage.ru_stime:.3f}s peak_rss={usage.ru_maxrss}KiB")

