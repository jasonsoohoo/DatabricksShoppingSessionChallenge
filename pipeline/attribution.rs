use super::{Batch, PARTITIONS, Strings, campaign_totals::{self, Totals}, parallel, people::People};
use std::collections::HashMap;
use std::ffi::{CStr, OsStr, c_char, c_void};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[derive(Clone, Copy)]
struct Click<'a> {
    time: i64,
    id: &'a [u8],
    code: i32,
}

fn value(strings: &Strings, index: usize) -> &[u8] {
    &strings.data[strings.offsets[index] as usize..strings.offsets[index + 1] as usize]
}

type Routed<'a> = Vec<Vec<(&'a [u8], Click<'a>)>>;

fn route<'a>(batch: &'a Batch, people: &People) -> Routed<'a> {
    let mut buckets = vec![Vec::new(); PARTITIONS];
    for (i, &time) in batch.times.iter().enumerate() {
        let person = value(&batch.users, i);
        let shard = people.hash_name(person) as usize % PARTITIONS;
        buckets[shard].push((person, Click { time, id: value(&batch.products, i), code: batch.cents[i] }));
    }
    buckets
}

// Campaign codes are dense positive integers assigned by the campaign lookup.
// Track each active campaign once, with constant-time insertion and expiry.
#[derive(Default)]
struct Active {
    entries: Vec<(usize, usize)>, // count, index in codes
    codes: Vec<i32>,
}

impl Active {
    fn add(&mut self, code: i32) {
        let i = code as usize;
        if i >= self.entries.len() { self.entries.resize(i + 1, (0, 0)); }
        let (count, index) = &mut self.entries[i];
        if *count == 0 {
            *index = self.codes.len();
            self.codes.push(code);
        }
        *count += 1;
    }

    fn remove(&mut self, code: i32) {
        let (count, index) = &mut self.entries[code as usize];
        *count -= 1;
        if *count == 0 {
            let slot = *index;
            self.codes.swap_remove(slot);
            if let Some(&moved) = self.codes.get(slot) {
                self.entries[moved as usize].1 = slot;
            }
        }
    }

    fn clear(&mut self) {
        for code in self.codes.drain(..) { self.entries[code as usize].0 = 0; }
    }
}

fn partition(labels: &Batch, people: &People, chunks: &[Routed]) -> Totals {
    let mut totals = Totals::new();
    if labels.times.is_empty() { return totals; }
    // The labeler emits one batch per canonical-person hash partition, with
    // contiguous people and nondecreasing session starts within each person.
    let shard = people.hash_name(value(&labels.users, 0)) as usize % PARTITIONS;
    let mut groups = HashMap::<&[u8], Vec<Click>>::new();
    for chunk in chunks {
        for &(person, click) in &chunk[shard] {
            groups.entry(person).or_default().push(click);
        }
    }
    let mut active = Active::default();
    let mut i = 0;
    while i < labels.times.len() {
        let person = value(&labels.users, i);
        let mut clicks = groups.remove(person).unwrap_or_default();
        clicks.sort_unstable_by(|a, b| a.time.cmp(&b.time).then_with(|| b.id.cmp(a.id)));
        let (mut left, mut right) = (0, 0);
        loop {
            let t = labels.times[i];
            let mut revenue = 0i64;
            loop {
                revenue += i64::from(labels.cents[i]);
                i += 1;
                if i == labels.times.len() || labels.times[i] != t || value(&labels.users, i) != person {
                    break;
                }
            }
            while right < clicks.len() && clicks[right].time <= t {
                active.add(clicks[right].code);
                right += 1;
            }
            while left < right && t - clicks[left].time > 604_800_000 {
                active.remove(clicks[left].code);
                left += 1;
            }
            let code = if right == left { 0 } else { clicks[right - 1].code };
            let day = t.div_euclid(86_400_000);
            let credited = totals.entry((i64::from(code), day)).or_default();
            credited.sessions += 1;
            credited.revenue += revenue;
            credited.purchases += i64::from(revenue > 0);
            for &assist in &active.codes {
                totals.entry((i64::from(assist), day)).or_default().assists += 1;
            }
            if i == labels.times.len() || value(&labels.users, i) != person { break; }
        }
        active.clear();
    }
    totals
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn attribute_batches(
    labels: *const c_void, clicks: *const c_void, people: *const People,
    workers: usize, output: *const c_char,
) -> bool {
    let labels = unsafe { &*labels.cast::<Vec<Batch>>() };
    let clicks = unsafe { &*clicks.cast::<Vec<Batch>>() };
    let people = unsafe { &*people };
    let workers = workers.max(1);
    let chunks = parallel(clicks.len(), workers, |i| route(&clicks[i], people));
    let parts = parallel(labels.len(), workers, |i| partition(&labels[i], people, &chunks));
    let mut totals = Totals::new();
    for part in parts { campaign_totals::merge(&mut totals, part); }
    let path = Path::new(OsStr::from_bytes(unsafe { CStr::from_ptr(output) }.to_bytes()));
    match campaign_totals::write(path, totals) {
        Ok(()) => true,
        Err(err) => { eprintln!("attribution: {err}"); false }
    }
}
