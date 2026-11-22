// Consume the ordered timeline exported by DuckDB. Clicks at the same instant
// arrive in descending ID order, then the session. The last click wins.
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

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

type Totals = HashMap<(i64, i64), Daily>;

fn sweep(mut reader: impl BufRead, daily: &mut Totals) -> io::Result<()> {
    let mut person = Vec::new();
    let mut window = VecDeque::<(i64, i64)>::new();
    let mut active = HashMap::<i64, usize>::new();
    let mut line = Vec::new();
    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line)? == 0 {
            break;
        }
        let mut fields = line
            .strip_suffix(b"\n")
            .unwrap_or(&line)
            .split(|&b| b == b',');
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
        while window
            .front()
            .is_some_and(|&(click, _)| t - click > 604_800_000)
        {
            let (_, expired) = window.pop_front().unwrap();
            let count = active.get_mut(&expired).unwrap();
            *count -= 1;
            if *count == 0 {
                active.remove(&expired);
            }
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
    Ok(())
}

// Move each approximate byte boundary to the start of a complete person.
// A group can span several target boundaries; schedule it exactly once.
fn boundaries(path: &Path, workers: usize) -> io::Result<Vec<u64>> {
    let file = File::open(path)?;
    let size = file.metadata()?.len();
    let parts = (workers * 4).min((size >> 20) as usize).max(1);
    let mut reader = BufReader::new(file);
    let mut cuts = vec![0];
    let mut line = Vec::new();
    for i in 1..parts {
        let target = size / parts as u64 * i as u64;
        if target <= *cuts.last().unwrap() {
            continue;
        }
        reader.seek(SeekFrom::Start(target - 1))?;
        line.clear();
        let mut pos = target - 1 + reader.read_until(b'\n', &mut line)? as u64;
        line.clear();
        pos += reader.read_until(b'\n', &mut line)? as u64;
        let person = line.split(|&b| b == b',').next().unwrap().to_vec();
        while pos < size {
            line.clear();
            let n = reader.read_until(b'\n', &mut line)?;
            if line.split(|&b| b == b',').next() != Some(person.as_slice()) {
                cuts.push(pos);
                break;
            }
            pos += n as u64;
        }
        if pos >= size {
            break;
        }
    }
    cuts.push(size);
    Ok(cuts)
}

fn worker(path: &Path, cuts: &[u64], next: &AtomicUsize) -> io::Result<Totals> {
    let mut file = File::open(path)?;
    let mut totals = Totals::new();
    loop {
        let i = next.fetch_add(1, Ordering::Relaxed);
        if i >= cuts.len() - 1 {
            break;
        }
        file.seek(SeekFrom::Start(cuts[i]))?;
        let range = (&mut file).take(cuts[i + 1] - cuts[i]);
        sweep(BufReader::with_capacity(1 << 18, range), &mut totals)?;
    }
    Ok(totals)
}

fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let path = Path::new(&args[0]);
    let workers = std::env::var("NCPU")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(8)
        .max(1);
    let cuts = boundaries(path, workers)?;
    let next = AtomicUsize::new(0);
    let daily = thread::scope(|scope| -> io::Result<Totals> {
        let handles: Vec<_> = (0..workers.min(cuts.len() - 1))
            .map(|_| scope.spawn(|| worker(path, &cuts, &next)))
            .collect();
        let mut daily = Totals::new();
        for handle in handles {
            for (key, part) in handle.join().unwrap()? {
                let total = daily.entry(key).or_default();
                total.sessions += part.sessions;
                total.purchases += part.purchases;
                total.revenue += part.revenue;
                total.assists += part.assists;
            }
        }
        Ok(daily)
    })?;
    let mut output = BufWriter::new(File::create(&args[1])?);
    for ((code, day), row) in daily {
        writeln!(
            output,
            "{code},{day},{},{},{},{}",
            row.sessions, row.purchases, row.revenue, row.assists
        )?;
    }
    output.flush()
}
