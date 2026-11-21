// Consume the ordered timeline exported by DuckDB. Clicks at the same instant
// arrive in descending ID order, then the session. The last click wins.
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};

#[derive(Default)]
struct Daily {
    sessions: i64,
    purchases: i64,
    revenue: i64,
    assists: i64,
}

fn number(s: &[u8]) -> i64 {
    std::str::from_utf8(s).unwrap().parse().unwrap()
}

fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let mut reader = BufReader::with_capacity(1 << 20, File::open(&args[0])?);
    let mut output = BufWriter::new(File::create(&args[1])?);
    let mut daily = HashMap::<(i64, i64), Daily>::new();
    let mut person = Vec::new();
    let mut window = VecDeque::<(i64, i64)>::new();
    let mut active = HashMap::<i64, usize>::new();
    let mut line = Vec::new();
    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line)? == 0 { break; }
        let mut fields = line.strip_suffix(b"\n").unwrap_or(&line).split(|&b| b == b',');
        let uid = fields.next().unwrap();
        if uid != person {
            person.clear();
            person.extend_from_slice(uid);
            window.clear();
            active.clear();
        }
        let t = number(fields.next().unwrap());
        let code = number(fields.next().unwrap());
        if code != 0 {
            window.push_back((t, code));
            *active.entry(code).or_default() += 1;
            continue;
        }
        while window.front().is_some_and(|&(click, _)| t - click > 604_800_000) {
            let (_, expired) = window.pop_front().unwrap();
            let count = active.get_mut(&expired).unwrap();
            *count -= 1;
            if *count == 0 { active.remove(&expired); }
        }
        let revenue = number(fields.next().unwrap());
        let day = t.div_euclid(86_400_000);
        let campaign = window.back().map_or(0, |&(_, code)| code);
        let credited = daily.entry((campaign, day)).or_default();
        credited.sessions += 1;
        credited.purchases += i64::from(revenue > 0);
        credited.revenue += revenue;
        for &campaign in active.keys() {
            daily.entry((campaign, day)).or_default().assists += 1;
        }
    }
    for ((code, day), row) in daily {
        writeln!(output, "{code},{day},{},{},{},{}", row.sessions, row.purchases, row.revenue, row.assists)?;
    }
    output.flush()
}
