use super::{Batch, PARTITIONS, Strings, csv, parallel};
use std::collections::HashMap;
use std::ffi::{CStr, OsStr, c_char, c_void};
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[derive(Clone, Copy)]
struct Event<'a> {
    time: i64,
    cents: i32,
    product: &'a [u8],
}

type Routed<'a> = Vec<Vec<(&'a [u8], Event<'a>)>>;

fn shard(person: &[u8]) -> usize {
    let hash = person.iter().fold(14695981039346656037u64, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(1099511628211)
    });
    hash as usize % PARTITIONS
}

fn value(strings: &Strings, i: usize) -> &[u8] {
    &strings.data[strings.offsets[i] as usize..strings.offsets[i + 1] as usize]
}

fn route<'a>(batch: &'a Batch, people: &HashMap<&'a [u8], (&'a [u8], usize)>) -> Routed<'a> {
    let mut buckets = vec![Vec::new(); PARTITIONS];
    for (i, &time) in batch.times.iter().enumerate() {
        let uid = value(&batch.users, i);
        let (person, partition) = people
            .get(uid)
            .copied()
            .unwrap_or_else(|| (uid, shard(uid)));
        buckets[partition].push((
            person,
            Event {
                time,
                cents: batch.cents[i],
                product: value(&batch.products, i),
            },
        ));
    }
    buckets
}

fn label_partition(routed: &[Routed], partition: usize) -> Batch {
    let mut groups = HashMap::<&[u8], Vec<Event>>::new();
    for buckets in routed {
        for &(person, event) in &buckets[partition] {
            groups.entry(person).or_default().push(event);
        }
    }
    let mut result = Batch::default();
    for (person, mut events) in groups {
        events.sort_unstable_by_key(|event| event.time);
        let mut previous = events[0].time;
        let mut start = previous;
        for event in events {
            if event.time - previous > 1_800_000 {
                start = event.time;
            }
            result.users.push(person);
            result.times.push(start);
            result.cents.push(event.cents);
            result.products.push(event.product);
            previous = event.time;
        }
    }
    result.users.finish();
    result.products.finish();
    result
}

fn label(batches: &[Batch], lookup: &Path, workers: usize) -> io::Result<Vec<Batch>> {
    let bytes = fs::read(lookup)?;
    let mut people = HashMap::new();
    for row in bytes.split(|&b| b == b'\n').filter(|row| !row.is_empty()) {
        let [uid, person] =
            csv::fields(row).ok_or_else(|| io::Error::other("invalid identity lookup row"))?;
        people.insert(uid, (person, shard(person)));
    }
    let routed = parallel(batches.len(), workers, |i| route(&batches[i], &people));
    Ok(parallel(PARTITIONS, workers, |i| {
        label_partition(&routed, i)
    }))
}

// Called once on the owning batch handle, before any buffers are exported.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn events_label(
    handle: *mut c_void,
    lookup: *const c_char,
    workers: usize,
) -> bool {
    let batches = unsafe { &mut *handle.cast::<Vec<Batch>>() };
    let path = Path::new(OsStr::from_bytes(
        unsafe { CStr::from_ptr(lookup) }.to_bytes(),
    ));
    match label(batches, path, workers.max(1)) {
        Ok(labeled) => {
            *batches = labeled;
            true
        }
        Err(err) => {
            eprintln!("session labeler: {err}");
            false
        }
    }
}
