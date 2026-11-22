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

// Labeling copies person and product bytes into the output before returning.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn events_label(handle: *mut c_void, lookup: *const People, workers: usize) {
    let batches = unsafe { &mut *handle.cast::<Vec<Batch>>() };
    let people = unsafe { &*lookup };
    let workers = workers.max(1);
    let routed = parallel(batches.len(), workers, |i| route(&batches[i], people));
    let labeled = parallel(PARTITIONS, workers, |i| label_partition(&routed, i));
    *batches = labeled;
}
