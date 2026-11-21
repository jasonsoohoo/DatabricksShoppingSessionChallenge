use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

fn decimal(bytes: &[u8]) -> Option<i64> {
    if bytes.is_empty() {
        return None;
    }
    bytes.iter().try_fold(0i64, |n, &b| {
        if !b.is_ascii_digit() {
            return None;
        }
        n.checked_mul(10)?.checked_add(i64::from(b - b'0'))
    })
}

// Proleptic Gregorian day number relative to 1970-01-01.
fn civil_day(year: i64, month: i64, day: i64) -> Option<i64> {
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let limit = match month {
        2 => 28 + i64::from(leap),
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => return None,
    };
    if !(1..=limit).contains(&day) {
        return None;
    }
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = if month > 2 { month - 3 } else { month + 9 };
    Some(era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + (153 * m + 2) / 5 + day - 1 - 719468)
}

fn millis(s: &[u8]) -> Option<i64> {
    if s.get(4) != Some(&b'-') {
        return decimal(s);
    }
    if !matches!(s.len(), 20 | 25)
        || s[7] != b'-'
        || s[10] != b'T'
        || s[13] != b':'
        || s[16] != b':'
    {
        return None;
    }
    let day = civil_day(decimal(&s[..4])?, decimal(&s[5..7])?, decimal(&s[8..10])?)?;
    let h = decimal(&s[11..13])?;
    let m = decimal(&s[14..16])?;
    let sec = decimal(&s[17..19])?;
    if h >= 24 || m >= 60 || sec >= 60 {
        return None;
    }
    let offset = match (s.len(), s[19]) {
        (20, b'Z') => 0,
        (25, sign @ (b'+' | b'-')) if s[22] == b':' => {
            let oh = decimal(&s[20..22])?;
            let om = decimal(&s[23..25])?;
            if oh >= 24 || om >= 60 {
                return None;
            }
            (if sign == b'+' { 1 } else { -1 }) * (oh * 60 + om)
        }
        _ => return None,
    };
    Some(((day * 86400 + (h * 60 + m - offset) * 60 + sec) * 1000) as i64)
}

fn string_field(out: &mut impl Write, value: &[u8]) -> io::Result<()> {
    out.write_all(b"\"")?;
    out.write_all(value)?;
    out.write_all(b"\"")
}

fn event(out: &mut impl Write, v: &[&[u8]; 7], start: i64, end: i64) -> io::Result<()> {
    if v[0].is_empty() {
        return Ok(());
    }
    // Keep the dedup key and ingestion order even for invalid copies.
    string_field(out, v[0])?;
    out.write_all(b",")?;
    out.write_all(v[3])?;
    let t = millis(v[2]).filter(|&t| t >= start && t < end);
    let price = match v[4] {
        b"view" | b"cart" => Some(0),
        b"purchase" => decimal(v[6]).filter(|n| (1..=10_000_000).contains(n)),
        _ => None,
    };
    if let (Some(t), Some(price)) = (t, price) {
        if !v[1].is_empty() && !v[5].is_empty() && millis(v[3]).is_some_and(|i| i <= t + 86_400_000)
        {
            out.write_all(b",")?;
            string_field(out, v[1])?;
            write!(out, ",{t},{price},")?;
            if price > 0 {
                string_field(out, v[5])?;
            }
            return out.write_all(b"\n");
        }
    }
    out.write_all(b",,,,\n")
}

fn clean_file(source: &Path, dest: &Path, width: usize, start: i64, end: i64) -> io::Result<()> {
    let mut input = BufReader::with_capacity(1 << 20, File::open(source)?);
    let shards = if width == 7 {
        env::var("EVENT_SHARDS").unwrap().parse().unwrap()
    } else {
        1
    };
    let mut outputs = Vec::new();
    for shard in 0..shards {
        let path = if width == 7 {
            dest.with_extension(format!("{shard}.csv"))
        } else {
            dest.to_owned()
        };
        outputs.push(BufWriter::with_capacity(1 << 17, File::create(path)?));
    }
    let mut row = Vec::new();
    input.read_until(b'\n', &mut row)?;
    loop {
        row.clear();
        if input.read_until(b'\n', &mut row)? == 0 {
            break;
        }
        let line = row.strip_suffix(b"\n").unwrap_or(&row);
        let mut fields = line.split(|b| *b == b',');
        let mut values: [&[u8]; 7] = [&[]; 7];
        let mut valid = true;
        for value in &mut values[..width] {
            let Some(raw) = fields.next() else {
                valid = false;
                break;
            };
            let field = raw
                .strip_prefix(b"\"")
                .and_then(|s| s.strip_suffix(b"\""))
                .unwrap_or(raw);
            if field.contains(&b'"') {
                valid = false;
                break;
            }
            *value = field;
        }
        if !valid || fields.next().is_some() {
            continue;
        }
        if width == 7 {
            let hash = values[0]
                .iter()
                .fold(0u64, |n, b| n.wrapping_mul(16777619) ^ u64::from(*b));
            event(&mut outputs[hash as usize % shards], &values, start, end)?;
            continue;
        }
        let output = &mut outputs[0];
        if width == 4 {
            let Some(t) = millis(values[3]) else { continue };
            if t < start - 604_800_000 || t >= end || values[..3].iter().any(|f| f.is_empty()) {
                continue;
            }
            for value in &values[..3] {
                string_field(output, value)?;
                output.write_all(b",")?;
            }
            writeln!(output, "{t}")?;
            continue;
        }
        if line.contains(&b'\r') {
            for (i, value) in values[..width].iter().enumerate() {
                if i != 0 {
                    output.write_all(b",")?;
                }
                output.write_all(b"\"")?;
                output.write_all(value)?;
                output.write_all(b"\"")?;
            }
        } else {
            output.write_all(line)?;
        }
        output.write_all(b"\n")?;
    }
    for output in &mut outputs {
        output.flush()?;
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args_os().skip(1).map(PathBuf::from).collect();
    let start = env::var("WINDOW_START").unwrap().parse().unwrap();
    let end = env::var("WINDOW_END").unwrap().parse().unwrap();
    let mut jobs = Vec::new();
    for (name, width) in [
        ("events", 7),
        ("clicks", 4),
        ("products", 2),
        ("campaigns", 2),
    ] {
        let mut paths = Vec::new();
        if name == "products" || name == "campaigns" {
            paths.push(args[0].join(format!("{name}.csv")));
        } else {
            for entry in fs::read_dir(args[0].join(name))? {
                let path = entry?.path();
                if path.extension().is_some_and(|e| e == "csv") {
                    paths.push(path);
                }
            }
        }
        let dest = args[1].join(name);
        fs::create_dir(&dest)?;
        // A typed empty input also works when a directory contains no parts.
        if name == "events" && paths.is_empty() {
            File::create(dest.join("empty.0.csv"))?;
        } else {
            File::create(dest.join("empty.csv"))?;
        }
        for (index, path) in paths.into_iter().enumerate() {
            let size = fs::metadata(&path)?.len();
            jobs.push((size, path, dest.join(format!("{index}.csv")), width));
        }
    }
    jobs.sort_unstable_by_key(|job| std::cmp::Reverse(job.0));
    let next = AtomicUsize::new(0);
    let workers = env::var("NCPU")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);
    thread::scope(|scope| {
        let mut handles = Vec::new();
        for _ in 0..workers {
            handles.push(scope.spawn(|| -> io::Result<()> {
                while let Some((_, source, dest, width)) =
                    jobs.get(next.fetch_add(1, Ordering::Relaxed))
                {
                    clean_file(source, dest, *width, start, end)?;
                }
                Ok(())
            }));
        }
        for handle in handles {
            handle.join().unwrap()?;
        }
        Ok(())
    })
}
