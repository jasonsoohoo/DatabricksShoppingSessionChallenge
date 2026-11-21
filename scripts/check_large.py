"""Check cross-table invariants and optionally compare both large outputs as multisets."""
import argparse
from pathlib import Path

import duckdb


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("actual", type=Path)
    parser.add_argument("--against", type=Path)
    args = parser.parse_args()
    con = duckdb.connect(config={"threads": "4", "memory_limit": "8GB", "temp_directory": "/scratch"})
    for table in ("user_daily", "campaign_daily"):
        def read(folder):
            return con.read_parquet(str(folder / table / "*.parquet"))
        read(args.actual).create_view(table)
        if args.against:
            read(args.against).create_view("reference_" + table)
            for left, right in ((table, "reference_" + table), ("reference_" + table, table)):
                count, = con.execute(f"select count(*) from (from {left} except all from {right})").fetchone()
                if count:
                    raise ValueError(f"{left} has {count} unmatched rows versus {right}")
    user = con.execute("select sum(sessions), sum(revenue_cents) from user_daily").fetchone()
    campaign = con.execute("select sum(sessions), sum(revenue_cents) from campaign_daily").fetchone()
    if user != campaign:
        raise ValueError(f"daily totals disagree: {user} versus {campaign}")
    errors, = con.execute("""
        select count(*) from campaign_daily where
          (campaign_id='organic' and assisted_sessions!=0)
          or (campaign_id!='organic' and assisted_sessions<sessions)
          or purchasing_sessions>sessions
    """).fetchone()
    if errors:
        raise ValueError(f"{errors} invalid campaign totals")
    print(f"PASS sessions={user[0]} revenue_cents={user[1]} exact_comparison={bool(args.against)}")


if __name__ == "__main__":
    main()
