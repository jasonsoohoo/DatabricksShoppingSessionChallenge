use super::{Batch, csv, csv_jobs, parallel, read_chunk};
use std::collections::HashMap;
use std::ffi::{CStr, OsStr, c_char, c_void};
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

fn prepare(dir: &Path, lookup: &Path, codes: &Path, start: i64, end: i64, workers: usize)
    -> io::Result<Vec<Batch>> {
    let people_data = fs::read(lookup)?;
    let campaign_data = fs::read(codes)?;
    let mut people = HashMap::new();
    let mut campaigns = HashMap::new();
    for row in people_data.split(|&b| b == b'\n').filter(|r| !r.is_empty()) {
        let [uid, person] = csv::fields(row).ok_or_else(|| io::Error::other("invalid identity mapping"))?;
        people.insert(uid, person);
    }
    for row in campaign_data.split(|&b| b == b'\n').filter(|r| !r.is_empty()) {
        let [id, code] = csv::fields(row).ok_or_else(|| io::Error::other("invalid campaign mapping"))?;
        let code = csv::decimal(code).and_then(|n| i32::try_from(n).ok())
            .ok_or_else(|| io::Error::other("invalid campaign code"))?;
        campaigns.insert(id, code);
    }
    let jobs = csv_jobs(dir)?;
    let mut batches = parallel(jobs.len(), workers, |i| {
        let mut batch = Batch::default();
        read_chunk(&jobs[i], |line| {
            let Some([id, user, campaign, ts]) = csv::fields(line) else { return Ok(()) };
            if id.is_empty() || user.is_empty() || campaign.is_empty() { return Ok(()); }
            let Some(&code) = campaigns.get(campaign) else { return Ok(()) };
            let Some(t) = csv::millis(ts) else { return Ok(()) };
            if t >= start - 604_800_000 && t < end {
                batch.users.push(people.get(user).copied().unwrap_or(user));
                batch.products.push(id);
                batch.times.push(t);
                batch.cents.push(code);
            }
            Ok(())
        })?;
        Ok(batch)
    }).into_iter().collect::<io::Result<Vec<_>>>()?;
    if batches.is_empty() { batches.push(Batch::default()); }
    for batch in &mut batches {
        batch.users.finish();
        batch.products.finish();
    }
    Ok(batches)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn clicks_open(
    dir: *const c_char, people: *const c_char, codes: *const c_char, start: i64, end: i64,
    workers: usize, count: *mut usize,
) -> *mut c_void {
    let path = |ptr| Path::new(OsStr::from_bytes(unsafe { CStr::from_ptr(ptr) }.to_bytes()));
    match prepare(path(dir), path(people), path(codes), start, end, workers.max(1)) {
        Ok(batches) => {
            unsafe { *count = batches.len(); }
            Box::into_raw(Box::new(batches)).cast()
        }
        Err(err) => {
            eprintln!("click preparation: {err}");
            std::ptr::null_mut()
        }
    }
}
