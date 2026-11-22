pub fn decimal(bytes: &[u8]) -> Option<i64> {
    if bytes.is_empty() {
        return None;
    }
    bytes.iter().try_fold(0i64, |n, &b| {
        if !b.is_ascii_digit() {
            return None;
        }
        n.checked_mul(10)?.checked_add(i64::from(b - b'0'))
    })
}

// Proleptic Gregorian day number relative to 1970-01-01.
fn civil_day(year: i64, month: i64, day: i64) -> Option<i64> {
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let limit = match month {
        2 => 28 + i64::from(leap),
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => return None,
    };
    if !(1..=limit).contains(&day) {
        return None;
    }
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = if month > 2 { month - 3 } else { month + 9 };
    Some(era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + (153 * m + 2) / 5 + day - 1 - 719468)
}

pub fn millis(s: &[u8]) -> Option<i64> {
    if s.get(4) != Some(&b'-') {
        return decimal(s);
    }
    if !matches!(s.len(), 20 | 25)
        || s[7] != b'-'
        || s[10] != b'T'
        || s[13] != b':'
        || s[16] != b':'
    {
        return None;
    }
    let day = civil_day(decimal(&s[..4])?, decimal(&s[5..7])?, decimal(&s[8..10])?)?;
    let h = decimal(&s[11..13])?;
    let m = decimal(&s[14..16])?;
    let sec = decimal(&s[17..19])?;
    if h >= 24 || m >= 60 || sec >= 60 {
        return None;
    }
    let offset = match (s.len(), s[19]) {
        (20, b'Z') => 0,
        (25, sign @ (b'+' | b'-')) if s[22] == b':' => {
            let oh = decimal(&s[20..22])?;
            let om = decimal(&s[23..25])?;
            if oh >= 24 || om >= 60 {
                return None;
            }
            (if sign == b'+' { 1 } else { -1 }) * (oh * 60 + om)
        }
        _ => return None,
    };
    Some(((day * 86400 + (h * 60 + m - offset) * 60 + sec) * 1000) as i64)
}

pub fn fields<const N: usize>(line: &[u8]) -> Option<[&[u8]; N]> {
    let mut values: [&[u8]; N] = [&[]; N];
    let mut fields = line.split(|&c| c == b',');
    for value in &mut values {
        let raw = fields.next()?;
        let field = raw
            .strip_prefix(b"\"")
            .and_then(|s| s.strip_suffix(b"\""))
            .unwrap_or(raw);
        if field.contains(&b'"') {
            return None;
        }
        *value = field;
    }
    fields.next().is_none().then_some(values)
}

// Event and ingestion timestamps often share a UTC date. Cache only validated
// dates and compare all ten date bytes before using their midnight value.
#[derive(Default)]
pub struct UtcDays {
    entries: [Option<([u8; 10], i64)>; 32],
}

impl UtcDays {
    pub fn millis(&mut self, text: &[u8]) -> Option<i64> {
        if text.len() != 20 || text[10] != b'T' || text[19] != b'Z' {
            return millis(text);
        }
        let slot = (text[8] as usize * 10 + text[9] as usize) % self.entries.len();
        if let Some((date, midnight)) = &self.entries[slot] {
            if date == &text[..10] {
                return Some(midnight + clock_ms(text)?);
            }
        }
        let time = millis(text)?;
        let date = text[..10].try_into().ok()?;
        self.entries[slot] = Some((date, time - clock_ms(text)?));
        Some(time)
    }
}

fn clock_ms(text: &[u8]) -> Option<i64> {
    fn pair(a: u8, b: u8) -> Option<i64> {
        let a = a.wrapping_sub(b'0');
        let b = b.wrapping_sub(b'0');
        (a <= 9 && b <= 9).then(|| i64::from(a * 10 + b))
    }
    if text[13] != b':' || text[16] != b':' { return None; }
    let h = pair(text[11], text[12])?;
    let m = pair(text[14], text[15])?;
    let s = pair(text[17], text[18])?;
    (h < 24 && m < 60 && s < 60).then_some((h * 3600 + m * 60 + s) * 1000)
}

#[cfg(test)]
mod timestamp_tests {
    use super::{millis, UtcDays};

    #[test]
    fn cached_and_uncached_timestamps_agree() {
        let mut days = UtcDays::default();
        for original in [b"2000-02-29T12:34:56Z", b"1900-02-29T12:34:56Z",
                         b"2024-03-29T00:00:00Z", b"1960-12-31T23:59:59Z"] {
            for pos in 0..20 {
                for ch in [0, b'0', b'1', b'5', b'9', b'Z', b'/', b':', 255] {
                    assert_eq!(days.millis(original), millis(original));
                    let mut text = *original;
                    text[pos] = ch;
                    assert_eq!(days.millis(&text), millis(&text));
                }
            }
        }
    }
}
