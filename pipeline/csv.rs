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
