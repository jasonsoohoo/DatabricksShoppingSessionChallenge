use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

type Fields<'a> = [&'a [u8]; 7];

pub fn read(job: &(PathBuf, u64, u64), mut consume: impl FnMut(Fields) -> io::Result<()>) -> io::Result<()> {
    let (path, begin, limit) = job;
    let mut pos = begin.saturating_sub(1);
    let mut input = BufReader::with_capacity(1 << 17, File::open(path)?);
    input.seek(SeekFrom::Start(pos))?;
    pos += input.skip_until(b'\n')? as u64;
    let mut index = Index::default();
    let mut crossing = Vec::new();
    while pos < *limit {
        let buf = input.fill_buf()?;
        if buf.is_empty() { break; }
        let allowed = buf.len().min((limit - pos) as usize);
        if let Some(last) = buf[..allowed].iter().rposition(|&b| b == b'\n') {
            let count = last + 1;
            visit(&buf[..count], &mut index, &mut consume)?;
            input.consume(count);
            pos += count as u64;
        } else {
            // Only the row straddling an I/O buffer or job boundary needs copying.
            crossing.clear();
            pos += input.read_until(b'\n', &mut crossing)? as u64;
            if crossing.last() != Some(&b'\n') { crossing.push(b'\n'); }
            visit(&crossing, &mut index, &mut consume)?;
        }
    }
    Ok(())
}

fn visit(mut data: &[u8], index: &mut Index, mut consume: impl FnMut(Fields) -> io::Result<()>) -> io::Result<()> {
    #[cfg(target_arch = "x86_64")]
    if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("popcnt") {

        while !data.is_empty() {
            let limit = data.len().min(8192);
            let Some(last) = data[..limit].iter().rposition(|&b| b == b'\n') else {
                // A single long record does not need a delimiter index.
                let end = limit + data[limit..].iter().position(|&b| b == b'\n').unwrap();
                if let Some(fields) = super::csv::fields(&data[..end]) { consume(fields)?; }
                data = &data[end + 1..];
                continue;
            };
            let limit = last + 1;
            let (block, rest) = data.split_at(limit);
            // Dispatch above checks both instruction sets.
            unsafe { index.scan(block); }
            index.decode(block, &mut consume)?;
            data = rest;
        }
        return Ok(());
    }
    for line in data.split(|&b| b == b'\n') {
        if let Some(fields) = super::csv::fields(line) { consume(fields)?; }
    }
    Ok(())
}

#[derive(Default)]
struct Index {
    ends: Vec<usize>,
    // Cumulative separator and quote counts at each newline.
    lines: Vec<(usize, usize)>,
}

impl Index {
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2,popcnt")]
    unsafe fn scan(&mut self, data: &[u8]) {
        use std::arch::x86_64::*;
        self.ends.clear();
        self.lines.clear();
        let mut quote_count = 0;
        let mut chunks = data.chunks_exact(32);
        for (i, bytes) in chunks.by_ref().enumerate() {
            // The slice has exactly 32 readable bytes.
            let (mut separators, mut newlines, quotes) = unsafe {
                let v = _mm256_loadu_si256(bytes.as_ptr().cast());
                let lf = _mm256_cmpeq_epi8(v, _mm256_set1_epi8(b'\n' as i8));
                let comma = _mm256_cmpeq_epi8(v, _mm256_set1_epi8(b',' as i8));
                let quote = _mm256_cmpeq_epi8(v, _mm256_set1_epi8(b'"' as i8));
                (_mm256_movemask_epi8(_mm256_or_si256(lf, comma)) as u32,
                 _mm256_movemask_epi8(lf) as u32, _mm256_movemask_epi8(quote) as u32)
            };
            while newlines != 0 {
                let before = (newlines & newlines.wrapping_neg()) - 1;
                self.lines.push((self.ends.len() + (separators & before).count_ones() as usize + 1,
                                 quote_count + (quotes & before).count_ones() as usize));
                newlines &= newlines - 1;
            }
            quote_count += quotes.count_ones() as usize;
            while separators != 0 {
                self.ends.push(i * 32 + separators.trailing_zeros() as usize);
                separators &= separators - 1;
            }
        }
        let tail = chunks.remainder();
        let begin = data.len() - tail.len();
        for (i, &b) in tail.iter().enumerate() {
            match b {
                b'"' => quote_count += 1,
                b',' => self.ends.push(begin + i),
                b'\n' => {
                    self.ends.push(begin + i);
                    self.lines.push((self.ends.len(), quote_count));
                }
                _ => {}
            }
        }
    }

    fn decode(&self, data: &[u8], consume: &mut impl FnMut(Fields) -> io::Result<()>) -> io::Result<()> {
        let (mut first, mut start, mut previous_quotes) = (0, 0, 0);
        for &(end, total_quotes) in &self.lines {
            let quotes = total_quotes - previous_quotes;
            if let Ok(offsets) = <&[usize; 7]>::try_from(&self.ends[first..end]) {
                let mut fields = offsets.map(|offset| {
                    let field = &data[start..offset];
                    start = offset + 1;
                    field
                });
                if quotes == 0 {
                    consume(fields)?;
                } else {
                    let mut wrappers = 0;
                    for field in &mut fields {
                        if field.len() >= 2 && field[0] == b'"' && field[field.len() - 1] == b'"' {
                            *field = &field[1..field.len() - 1];
                            wrappers += 2;
                        }
                    }
                    // Every quote must belong to a complete pair around a field.
                    if wrappers == quotes { consume(fields)?; }
                }
            }
            first = end;
            start = self.ends[end - 1] + 1;
            previous_quotes = total_quotes;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::iter::repeat_n;

    #[test]
    fn indexed_rows_match_scalar_at_record_and_file_boundaries() -> io::Result<()> {
        let header = b"a,b,c,d,e,f,g\n";
        let mut data = header.to_vec();
        for width in 0..65 {
            for suffix in [",b,c,d,e,f,g", ",\"b\",c,d,e,f,\"\"", ",b,c,d,e,f,\"",
                           ",b,c,d,e,f,g,h", ",b,c,d,e,f", ",b,c,d,e,f,x\"\"z",
                           ",b,c,d,e,f,\"\r\"", ",b,c,d,e,f,\"x\"\"y\""] {
                data.extend(repeat_n(b'a', width));
                data.extend_from_slice(suffix.as_bytes());
                data.push(b'\n');
            }
        }
        data.extend(repeat_n(b'x', (1 << 17) + 17));
        data.extend_from_slice(b",b,c,d,e,f,g\nlast,b,c,d,e,f,g");
        let owned = |fields: Fields| fields.map(<[u8]>::to_vec);
        let expected: Vec<_> = data[header.len()..].split(|&b| b == b'\n')
            .filter_map(super::super::csv::fields).map(owned).collect();
        let path = std::env::temp_dir().join(format!("event-blocks-{}", std::process::id()));
        fs::write(&path, &data)?;
        for size in [509, 8192, 65536, data.len()] {
            let mut actual = Vec::new();
            for begin in (0..data.len()).step_by(size) {
                read(&(path.clone(), begin as u64, data.len().min(begin + size) as u64), |row| {
                    actual.push(owned(row));
                    Ok(())
                })?;
            }
            assert_eq!(actual, expected);
        }
        fs::remove_file(path)
    }
}
