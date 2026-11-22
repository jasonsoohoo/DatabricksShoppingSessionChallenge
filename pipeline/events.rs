mod clicks;
mod csv;
mod event_blocks;
mod sessions;
mod people;

use std::ffi::{CStr, c_char};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::thread;

const PARTITIONS: usize = 256;
const CHUNK: u64 = 16 << 20;

struct Event {
    hash: u32, // A probe hint; collisions are resolved using the full ID.
    ingest: i64,
    time: i64,
    cents: i32, // -1 retains an invalid copy in the earliest-ingest election
    begin: u32,
    key_end: u32,
    user_end: u32,
    end: u32,
}

struct Bucket {
    bytes: Vec<u8>,
    rows: Vec<Event>,
}

fn offset(bytes: &[u8]) -> io::Result<u32> {
    u32::try_from(bytes.len()).map_err(|_| io::Error::other("event input chunk exceeds 4 GiB"))
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

fn parse_chunk(job: &(PathBuf, u64, u64), start: i64, end: i64) -> io::Result<Vec<Bucket>> {
    // Estimate capacity from this range only. Longer fields and skewed keys grow normally.
    let per_partition = (job.2 - job.1) as usize / PARTITIONS;
    let mut buckets = (0..PARTITIONS).map(|_| Bucket {
        rows: Vec::with_capacity(per_partition / 96),
        bytes: Vec::with_capacity(per_partition / 4),
    }).collect::<Vec<_>>();
    event_blocks::read(job, |[id, user, ts, ingest, kind, product, price]| {
        if id.is_empty() {
            return Ok(());
        }
        let ingest = csv::millis(ingest).ok_or_else(|| io::Error::other("invalid ingest time"))?;
        let time = csv::millis(ts).filter(|&t| t >= start && t < end && ingest <= t + 86_400_000);
        let cents = match kind {
            b"cart" | b"view" => Some(0),
            b"purchase" => csv::decimal(price).filter(|p| (1..=10_000_000).contains(p)),
            _ => None,
        };
        let valid = time.is_some() && cents.is_some() && !user.is_empty() && !product.is_empty();
        let hash = id.iter().fold(14695981039346656037u64, |n, b| {
            (n ^ u64::from(*b)).wrapping_mul(1099511628211)
        });
        let hash = hash as u32;
        let bucket = &mut buckets[hash as usize % PARTITIONS];
        let bytes = &mut bucket.bytes;
        let begin = offset(bytes)?;
        bytes.extend_from_slice(id);
        let key_end = offset(bytes)?;
        if valid {
            bytes.extend_from_slice(user);
        }
        let user_end = offset(bytes)?;
        if valid && cents != Some(0) {
            bytes.extend_from_slice(product);
        }
        bucket.rows.push(Event {
            hash,
            ingest,
            time: time.unwrap_or(0),
            cents: if valid { cents.unwrap() as i32 } else { -1 },
            begin,
            key_end,
            user_end,
            end: offset(bytes)?,
        });
        Ok(())
    })?;
    Ok(buckets)
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

fn dedup(chunks: &[Vec<Bucket>], partition: usize) -> Batch {
    let count = chunks
        .iter()
        .map(|c| c[partition].rows.len())
        .sum::<usize>();
    let capacity = (count + count / 2 + 1).next_power_of_two();
    let mut slots = vec![usize::MAX; capacity];
    let mut winners: Vec<(&Event, &[u8])> = Vec::with_capacity(count);
    for chunk in chunks {
        let bucket = &chunk[partition];
        for row in &bucket.rows {
            let mut slot = (row.hash as usize / PARTITIONS) & (capacity - 1);
            loop {
                let index = slots[slot];
                if index == usize::MAX {
                    slots[slot] = winners.len();
                    winners.push((row, &bucket.bytes));
                    break;
                }
                let (prev, bytes) = winners[index];
                if prev.hash == row.hash
                    && bytes[prev.begin as usize..prev.key_end as usize]
                        == bucket.bytes[row.begin as usize..row.key_end as usize]
                {
                    if row.ingest < prev.ingest {
                        winners[index] = (row, &bucket.bytes);
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
        batch
            .users
            .push(&bytes[row.key_end as usize..row.user_end as usize]);
        batch
            .products
            .push(&bytes[row.user_end as usize..row.end as usize]);
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
    let next = std::sync::atomic::AtomicUsize::new(0);
    let chunks = thread::scope(|scope| -> io::Result<Vec<Vec<Bucket>>> {
        let mut threads = Vec::new();
        for _ in 0..workers.min(jobs.len()) {
            threads.push(scope.spawn(|| -> io::Result<Vec<_>> {
                let mut result = Vec::new();
                while let Some(job) = jobs.get(next.fetch_add(1, std::sync::atomic::Ordering::Relaxed)) {
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
