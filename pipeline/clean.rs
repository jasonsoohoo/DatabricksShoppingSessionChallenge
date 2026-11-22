use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

mod csv;
use csv::{fields, millis};

fn string_field(out: &mut impl Write, value: &[u8]) -> io::Result<()> {
    out.write_all(b"\"")?;
    out.write_all(value)?;
    out.write_all(b"\"")
}

fn clean_file(source: &Path, dest: &Path, width: usize, start: i64, end: i64) -> io::Result<()> {
    let mut input = BufReader::with_capacity(1 << 20, File::open(source)?);
    let mut output = BufWriter::new(File::create(dest)?);
    let mut row = Vec::new();
    input.read_until(b'\n', &mut row)?;
    loop {
        row.clear();
        if input.read_until(b'\n', &mut row)? == 0 {
            break;
        }
        let line = row.strip_suffix(b"\n").unwrap_or(&row);
        if width == 4 {
            let Some(values) = fields::<4>(line) else {
                continue;
            };
            let Some(t) = millis(values[3]) else { continue };
            if t < start - 604_800_000 || t >= end || values[..3].iter().any(|f| f.is_empty()) {
                continue;
            }
            for value in &values[..3] {
                string_field(&mut output, value)?;
                output.write_all(b",")?;
            }
            writeln!(output, "{t}")?;
            continue;
        }
        let Some(values) = fields::<2>(line) else {
            continue;
        };
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
    output.flush()
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args_os().skip(1).map(PathBuf::from).collect();
    let start = env::var("WINDOW_START").unwrap().parse().unwrap();
    let end = env::var("WINDOW_END").unwrap().parse().unwrap();
    let mut jobs = Vec::new();
    for (name, width) in [("clicks", 4), ("products", 2), ("campaigns", 2)] {
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
        File::create(dest.join("empty.csv"))?;
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
