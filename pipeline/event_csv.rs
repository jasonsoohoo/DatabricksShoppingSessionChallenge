// Parse whole event records from bounded file ranges. The next range skips the
// partial line before its start, so a row crossing a boundary is visited once.
use super::{File, PathBuf};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};

type Fields<'a> = [&'a [u8]; 7];

pub fn read(job: &(PathBuf, u64, u64)) -> io::Result<(Vec<u8>, usize)> {
    let (path, begin, limit) = job;
    let mut input = BufReader::new(File::open(path)?);
    let start = begin.saturating_sub(1);
    input.seek(SeekFrom::Start(start))?;
    let mut data = Vec::with_capacity((limit - start) as usize + 256);
    (&mut input).take(limit - start).read_to_end(&mut data)?;
    if data.last() != Some(&b'\n') {
        input.read_until(b'\n', &mut data)?;
    }
    let first = data.iter().position(|&c| c == b'\n').map_or(data.len(), |i| i + 1);
    if data.last() != Some(&b'\n') { data.push(b'\n'); }
    Ok((data, first))
}

pub fn visit(rows: &[u8], consume: impl FnMut(Fields) -> io::Result<()>) -> io::Result<()> {
    #[cfg(target_arch = "x86_64")]
    if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("popcnt") {
        // Both required CPU features were checked above.
        return unsafe { scan(rows, consume) };
    }
    scalar(rows, consume)
}

fn scalar(rows: &[u8], mut consume: impl FnMut(Fields) -> io::Result<()>) -> io::Result<()> {
    for row in rows.split(|&c| c == b'\n') {
        if let Some(fields) = super::csv::fields(row) { consume(fields)?; }
    }
    Ok(())
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,popcnt")]
unsafe fn scan(rows: &[u8], mut consume: impl FnMut(Fields) -> io::Result<()>) -> io::Result<()> {
    use std::arch::x86_64::*;
    let mut fields: Fields = [&[]; 7];
    let mut n = 0;
    let mut start = 0;
    let mut row_start = 0;
    let mut quotes = 0;
    for (i, block) in rows.chunks_exact(32).enumerate() {
        // The dispatch checked AVX2, and the block contains exactly 32 bytes.
        let (mut delimiters, mut quoted) = unsafe {
            let bytes = _mm256_loadu_si256(block.as_ptr().cast());
            let comma = _mm256_cmpeq_epi8(bytes, _mm256_set1_epi8(b',' as i8));
            let lf = _mm256_cmpeq_epi8(bytes, _mm256_set1_epi8(b'\n' as i8));
            let quote = _mm256_cmpeq_epi8(bytes, _mm256_set1_epi8(b'"' as i8));
            (_mm256_movemask_epi8(_mm256_or_si256(comma, lf)) as u32,
             _mm256_movemask_epi8(quote) as u32)
        };
        while delimiters != 0 {
            let bit = delimiters.trailing_zeros();
            delimiters &= delimiters - 1;
            let end = i * 32 + bit as usize;
            let before = (1u32 << bit) - 1;
            quotes += (quoted & before).count_ones() as usize;
            quoted &= !before;
            if n < fields.len() {
                let raw = &rows[start..end];
                if quotes == 0 {
                    fields[n] = raw;
                    n += 1;
                } else if quotes == 2 && raw.first() == Some(&b'"') && raw.last() == Some(&b'"') {
                    fields[n] = &raw[1..raw.len()-1];
                    n += 1;
                } else {
                    n = fields.len() + 1;
                }
            } else {
                n = fields.len() + 1;
            }
            start = end + 1;
            quotes = 0;
            if rows[end] == b'\n' {
                if n == fields.len() { consume(fields)?; }
                n = 0;
                row_start = start;
            }
        }
        quotes += quoted.count_ones() as usize;
    }
    // Reparse the last partial record and the short scalar tail together.
    scalar(&rows[row_start..], consume)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn fields_match_scalar_across_vector_and_file_boundaries() -> io::Result<()> {
        let mut data = b"a,b,c,d,e,f,g\n".to_vec();
        for width in 0..65 {
            for tail in [",b,c,d,e,f,g", ",\"b\",c,d,e,f,\"\"", ",b,c,d,e,f,\"",
                         ",b,c,d,e,f,g,h", ",b,c,d,e,f", ",b,c,d,e,f,x\"\"z",
                         ",b,c,d,e,f,\"\r\"", ",b,c,d,e,f,\"x\"\"y\""] {
                data.extend(std::iter::repeat_n(b'a', width));
                data.extend_from_slice(tail.as_bytes());
                data.push(b'\n');
            }
        }
        data.extend_from_slice(b"last,b,c,d,e,f,g");
        let owned = |fields: Fields| fields.map(<[u8]>::to_vec);
        let mut expected = Vec::new();
        scalar(&data[14..], |fields| { expected.push(owned(fields)); Ok(()) })?;
        let path = std::env::temp_dir().join(format!("event-csv-test-{}", std::process::id()));
        fs::write(&path, &data)?;
        for size in [31, 64, 513, data.len()] {
            let mut actual = Vec::new();
            for begin in (0..data.len()).step_by(size) {
                let (bytes, first) = read(&(path.clone(), begin as u64, data.len().min(begin + size) as u64))?;
                visit(&bytes[first..],
                      |fields| { actual.push(owned(fields)); Ok(()) })?;
            }
            assert_eq!(actual, expected);
        }
        fs::remove_file(path)
    }
}
