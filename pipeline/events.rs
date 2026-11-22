mod clicks;
mod csv;
mod event_csv;
mod sessions;
mod people;

use std::ffi::{CStr, c_char};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::thread;
use std::sync::atomic::{AtomicUsize, Ordering};

const PARTITIONS: usize = 256;
const CHUNK: u64 = 16 << 20;

#[derive(Copy, Clone)]
struct Span {
    start: u32,
    len: u32,
}

impl Span {
    // All fields are slices of the same input chunk, checked to fit in u32.
    fn new(field: &[u8], bytes: &[u8]) -> Self {
        Self {
            start: if field.is_empty() { 0 } else { (field.as_ptr() as usize - bytes.as_ptr() as usize) as u32 },
            len: field.len() as u32,
        }
    }

    fn get(self, bytes: &[u8]) -> &[u8] {
        &bytes[self.start as usize..self.start as usize + self.len as usize]
    }
}

struct Event {
    hash: u64,
    time: i64,
    cents: i32,
    id: Span,
    user: Span,
    product: Span,
    ingest: u32,
}

struct Chunk {
    bytes: Vec<u8>,
    partitions: Vec<Vec<Event>>,
}

fn read_chunk(job: &(PathBuf, u64, u64), mut visit: impl FnMut(&[u8]) -> io::Result<()>) -> io::Result<()> {
    let (path, begin, limit) = job;
    let mut reader = BufReader::with_capacity(1 << 18, File::open(path)?);
    let mut pos = begin.saturating_sub(1);
    reader.seek(SeekFrom::Start(pos))?;
    let mut line = Vec::new();
    // Skip the header, or the partial record preceding this chunk.
    pos += reader.read_until(b'\n', &mut line)? as u64;
    while pos < *limit {
        line.clear();
        let size = reader.read_until(b'\n', &mut line)?;
        if size == 0 {
            break;
        }
        pos += size as u64;
        visit(line.strip_suffix(b"\n").unwrap_or(&line))?;
    }
    Ok(())
}

fn parse_chunk(job: &(PathBuf, u64, u64), start: i64, end: i64) -> io::Result<Chunk> {
    let (bytes, first) = event_csv::read(job)?;
    u32::try_from(bytes.len()).map_err(|_| io::Error::other("event input chunk exceeds 4 GiB"))?;
    let mut partitions = (0..PARTITIONS).map(|_| Vec::new()).collect::<Vec<_>>();
    let mut timestamps = csv::UtcDays::default();
    event_csv::visit(&bytes[first..], |[id, user, ts, ingest, kind, product, price]| {
        if id.is_empty() { return Ok(()); }
        // The specification guarantees fixed-width UTC (F1) ingestion times.
        // An F1 event on the same UTC day cannot be more than 24 hours late.
        if ingest.len() != 20 { return Err(io::Error::other("invalid ingest time")); }
        let time = timestamps.millis(ts).filter(|&t| t >= start && t < end);
        let timely = match time {
            None => false,
            Some(_) if ts.len() == 20 && ts[19] == b'Z' && ts[..10] == ingest[..10] => true,
            Some(t) => timestamps.millis(ingest).is_some_and(|i| i <= t + 86_400_000),
        };
        let cents = match kind {
            b"cart" | b"view" => Some(0),
            b"purchase" => csv::decimal(price).filter(|p| (1..=10_000_000).contains(p)),
            _ => None,
        };
        let hash = id.iter().fold(14695981039346656037u64, |n, b| {
            (n ^ u64::from(*b)).wrapping_mul(1099511628211)
        });
        let valid = timely && !user.is_empty() && !product.is_empty();
        partitions[hash as usize % PARTITIONS].push(Event {
            hash,
            time: time.unwrap_or(0),
            cents: if valid { cents.unwrap_or(-1) as i32 } else { -1 },
            id: Span::new(id, &bytes),
            user: Span::new(user, &bytes),
            product: Span::new(product, &bytes),
            ingest: Span::new(ingest, &bytes).start,
        });
        Ok(())
    })?;
    Ok(Chunk { bytes, partitions })
}

#[derive(Default)]
struct Strings {
    offsets: Vec<i64>,
    data: Vec<u8>,
}

impl Strings {
    fn push(&mut self, value: &[u8]) {
        self.offsets.push(self.data.len() as i64);
        self.data.extend_from_slice(value);
    }

    fn finish(&mut self) {
        self.offsets.push(self.data.len() as i64);
    }
}

#[derive(Default)]
struct Batch {
    users: Strings,
    products: Strings,
    times: Vec<i64>,
    cents: Vec<i32>,
}

fn dedup(chunks: &[Chunk], partition: usize) -> Batch {
    let count = chunks
        .iter()
        .map(|c| c.partitions[partition].len())
        .sum::<usize>();
    let capacity = (count + count / 2 + 1).next_power_of_two();
    let mut slots = vec![usize::MAX; capacity];
    let mut winners: Vec<(&Event, &[u8])> = Vec::with_capacity(count);
    for chunk in chunks {
        let bytes = chunk.bytes.as_slice();
        for row in &chunk.partitions[partition] {
            let mut slot = (row.hash as usize / PARTITIONS) & (capacity - 1);
            loop {
                let index = slots[slot];
                if index == usize::MAX {
                    slots[slot] = winners.len();
                    winners.push((row, bytes));
                    break;
                }
                let (prev, previous_bytes) = winners[index];
                if prev.hash == row.hash
                    && prev.id.get(previous_bytes) == row.id.get(bytes)
                {
                    // Fixed-width F1 timestamps sort chronologically by bytes.
                    let old = prev.ingest as usize;
                    let new = row.ingest as usize;
                    if bytes[new..new + 20] < previous_bytes[old..old + 20] {
                        winners[index] = (row, bytes);
                    }
                    break;
                }
                slot = (slot + 1) & (capacity - 1);
            }
        }
    }
    let mut batch = Batch::default();
    for (row, bytes) in winners {
        if row.cents < 0 {
            continue;
        }
        batch.users.push(row.user.get(bytes));
        batch.products.push(if row.cents > 0 { row.product.get(bytes) } else { &[] });
        batch.times.push(row.time);
        batch.cents.push(row.cents);
    }
    batch.users.finish();
    batch.products.finish();
    batch
}

fn parallel<T: Send>(count: usize, workers: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    thread::scope(|scope| {
        let f = &f;
        let handles: Vec<_> = (0..workers.min(count))
            .map(|i| scope.spawn(move || (i..count).step_by(workers).map(f).collect::<Vec<_>>()))
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    })
}

fn csv_jobs(dir: &Path) -> io::Result<Vec<(PathBuf, u64, u64)>> {
    let mut jobs = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().is_none_or(|ext| ext != "csv") {
            continue;
        }
        let len = fs::metadata(&path)?.len();
        for begin in (0..len).step_by(CHUNK as usize) {
            jobs.push((path.clone(), begin, len.min(begin + CHUNK)));
        }
    }
    Ok(jobs)
}

fn build(dir: &Path, start: i64, end: i64, workers: usize) -> io::Result<Vec<Batch>> {
    let mut jobs = csv_jobs(dir)?;
    jobs.sort_unstable_by_key(|(_, begin, end)| std::cmp::Reverse(end - begin));
    // File sizes vary. Assign the next largest range to the first free parser.
    let next = AtomicUsize::new(0);
    let chunks = thread::scope(|scope| -> io::Result<Vec<Chunk>> {
        let mut threads = Vec::new();
        for _ in 0..workers.min(jobs.len()) {
            threads.push(scope.spawn(|| -> io::Result<Vec<Chunk>> {
                let mut result = Vec::new();
                while let Some(job) = jobs.get(next.fetch_add(1, Ordering::Relaxed)) {
                    result.push(parse_chunk(job, start, end)?);
                }
                Ok(result)
            }));
        }
        let mut chunks = Vec::with_capacity(jobs.len());
        for handle in threads {
            chunks.extend(handle.join().unwrap()?);
        }
        Ok(chunks)
    })?;
    Ok(parallel(PARTITIONS, workers, |i| dedup(&chunks, i)))
}

// Buffer addresses stay valid until Python closes the owning batch collection.
#[repr(C)]
pub struct Buffers {
    rows: usize,
    users: *const u8,
    user_offsets: *const i64,
    user_bytes: usize,
    products: *const u8,
    product_offsets: *const i64,
    product_bytes: usize,
    times: *const i64,
    cents: *const i32,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn events_open(
    dir: *const c_char,
    start: i64,
    end: i64,
    workers: usize,
    count: *mut usize,
) -> *mut std::ffi::c_void {
    let path = Path::new(std::ffi::OsStr::from_bytes(
        unsafe { CStr::from_ptr(dir) }.to_bytes(),
    ));
    match build(path, start, end, workers.max(1)) {
        Ok(batches) => {
            unsafe {
                *count = batches.len();
            }
            Box::into_raw(Box::new(batches)).cast()
        }
        Err(err) => {
            eprintln!("event reader: {err}");
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn events_batch(
    handle: *const std::ffi::c_void,
    index: usize,
    out: *mut Buffers,
) {
    let batches = unsafe { &*handle.cast::<Vec<Batch>>() };
    let b = &batches[index];
    unsafe {
        out.write(Buffers {
            rows: b.times.len(),
            users: b.users.data.as_ptr(),
            user_offsets: b.users.offsets.as_ptr(),
            user_bytes: b.users.data.len(),
            products: b.products.data.as_ptr(),
            product_offsets: b.products.offsets.as_ptr(),
            product_bytes: b.products.data.len(),
            times: b.times.as_ptr(),
            cents: b.cents.as_ptr(),
        });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn events_close(handle: *mut std::ffi::c_void) {
    drop(unsafe { Box::from_raw(handle.cast::<Vec<Batch>>()) });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn people_open(dir: *const c_char) -> *mut std::ffi::c_void {
    let path = Path::new(std::ffi::OsStr::from_bytes(unsafe { CStr::from_ptr(dir) }.to_bytes()));
    match people::People::load(path) {
        Ok(people) => Box::into_raw(Box::new(people)).cast(),
        Err(err) => {
            eprintln!("identity resolver: {err}");
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn people_close(handle: *mut std::ffi::c_void) {
    drop(unsafe { Box::from_raw(handle.cast::<people::People>()) });
}
