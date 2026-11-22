use super::{Batch, PARTITIONS, Strings, parallel, people::People};
use std::collections::HashMap;
use std::ffi::c_void;

#[derive(Clone, Copy)]
struct Event<'a> {
    time: i64,
    cents: i32,
    product: &'a [u8],
}

type Routed<'a> = Vec<Vec<(&'a [u8], Event<'a>)>>;

fn value(strings: &Strings, i: usize) -> &[u8] {
    &strings.data[strings.offsets[i] as usize..strings.offsets[i + 1] as usize]
}

fn route<'a>(batch: &'a Batch, people: &'a People) -> Routed<'a> {
    let mut buckets = vec![Vec::new(); PARTITIONS];
    for (i, &time) in batch.times.iter().enumerate() {
        let uid = value(&batch.users, i);
        let (person, hash) = people.canonical(uid);
        buckets[hash as usize % PARTITIONS].push((
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

fn push_label(result: &mut Batch, person: &[u8], start: i64, cents: i32, product: &[u8]) {
    result.users.push(person);
    result.times.push(start);
    result.cents.push(cents);
    result.products.push(product);
}

fn compact_session(result: &mut Batch, person: &[u8], session: &[Event]) {
    let first = result.times.len();
    let start = session[0].time;
    let mut revenue = 0i64;
    for event in session.iter().filter(|event| event.cents > 0) {
        push_label(result, person, start, event.cents, event.product);
        revenue += i64::from(event.cents);
    }
    if result.times.len() == first {
        push_label(result, person, start, 0, b"");
    }
    // Only the first row owns the totals, even in sessions with multiple purchases.
    result.event_counts.resize(result.times.len(), 0);
    result.revenues.resize(result.times.len(), 0);
    result.event_counts[first] = session.len() as i64;
    result.revenues[first] = revenue;
}

fn label_partition(routed: &[Routed], partition: usize, compact: bool) -> Batch {
    let mut groups = HashMap::<&[u8], Vec<Event>>::new();
    for buckets in routed {
        for &(person, event) in &buckets[partition] {
            groups.entry(person).or_default().push(event);
        }
    }
    let mut result = Batch::default();
    for (person, mut events) in groups {
        events.sort_unstable_by_key(|event| event.time);
        for session in events.chunk_by(|a, b| b.time - a.time <= 1_800_000) {
            if compact {
                compact_session(&mut result, person, session);
            } else {
                for event in session {
                    push_label(&mut result, person, session[0].time, event.cents, event.product);
                }
            }
        }
    }
    result.users.finish();
    result.products.finish();
    result
}

// Labeling copies person and product bytes into the output before returning.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn events_label(handle: *mut c_void, lookup: *const People, workers: usize, compact: bool) {
    let batches = unsafe { &mut *handle.cast::<Vec<Batch>>() };
    let people = unsafe { &*lookup };
    let workers = workers.max(1);
    let routed = parallel(batches.len(), workers, |i| route(&batches[i], people));
    let labeled = parallel(PARTITIONS, workers, |i| label_partition(&routed, i, compact));
    *batches = labeled;
}
