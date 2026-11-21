use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

fn clean_file(source: &Path, dest: &Path, width: usize) -> io::Result<()> {
    let mut input = BufReader::with_capacity(1 << 20, File::open(source)?);
    let mut output = BufWriter::with_capacity(1 << 20, File::create(dest)?);
    let mut row = Vec::new();
    input.read_until(b'\n', &mut row)?;
    loop {
        row.clear();
        if input.read_until(b'\n', &mut row)? == 0 { break; }
        let line = row.strip_suffix(b"\n").unwrap_or(&row);
        let mut fields = line.split(|b| *b == b',');
        let mut values: [&[u8]; 7] = [&[]; 7];
        let mut valid = true;
        for value in &mut values[..width] {
            let Some(raw) = fields.next() else { valid = false; break };
            let field = raw.strip_prefix(b"\"").and_then(|s| s.strip_suffix(b"\"")).unwrap_or(raw);
            if field.contains(&b'"') { valid = false; break; }
            *value = field;
        }
        if !valid || fields.next().is_some() { continue; }
        if line.contains(&b'\r') {
            for (i, value) in values[..width].iter().enumerate() {
                if i != 0 { output.write_all(b",")?; }
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
    let mut jobs = Vec::new();
    for (name, width) in [("events",7), ("clicks",4), ("products",2), ("campaigns",2)] {
        let mut paths = Vec::new();
        if name == "products" || name == "campaigns" {
            paths.push(args[0].join(format!("{name}.csv")));
        } else {
            for entry in fs::read_dir(args[0].join(name))? {
                let path = entry?.path();
                if path.extension().is_some_and(|e| e == "csv") { paths.push(path); }
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
    let workers = env::var("NCPU").ok().and_then(|s| s.parse().ok()).unwrap_or(8);
    thread::scope(|scope| {
        let mut handles = Vec::new();
        for _ in 0..workers {
            handles.push(scope.spawn(|| -> io::Result<()> {
                while let Some((_, source, dest, width)) = jobs.get(next.fetch_add(1, Ordering::Relaxed)) {
                    clean_file(source, dest, *width)?;
                }
                Ok(())
            }));
        }
        for handle in handles { handle.join().unwrap()?; }
        Ok(())
    })
}
