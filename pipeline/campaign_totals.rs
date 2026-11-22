use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

#[derive(Default)]
pub struct Daily {
    pub sessions: i64,
    pub purchases: i64,
    pub revenue: i64,
    pub assists: i64,
}

pub type Totals = HashMap<(i64, i64), Daily>;

pub fn merge(daily: &mut Totals, part: Totals) {
    for (key, part) in part {
        let total = daily.entry(key).or_default();
        total.sessions += part.sessions;
        total.purchases += part.purchases;
        total.revenue += part.revenue;
        total.assists += part.assists;
    }
}

pub fn write(path: &Path, daily: Totals) -> io::Result<()> {
    let mut output = BufWriter::new(File::create(path)?);
    for ((code, day), row) in daily {
        writeln!(
            output,
            "{code},{day},{},{},{},{}",
            row.sessions, row.purchases, row.revenue, row.assists
        )?;
    }
    output.flush()
}
