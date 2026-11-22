use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

mod csv;

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args_os().skip(1).map(PathBuf::from).collect();
    for name in ["campaigns", "products"] {
        let dir = args[1].join(name);
        fs::create_dir(&dir)?;
        let mut output = BufWriter::new(File::create(dir.join("clean.csv"))?);
        let mut input = BufReader::new(File::open(args[0].join(format!("{name}.csv")))?);
        let mut row = Vec::new();
        input.read_until(b'\n', &mut row)?;
        loop {
            row.clear();
            if input.read_until(b'\n', &mut row)? == 0 { break; }
            let line = row.strip_suffix(b"\n").unwrap_or(&row);
            let Some([id, value]) = csv::fields(line) else { continue };
            if line.contains(&b'\r') {
                output.write_all(b"\"")?;
                output.write_all(id)?;
                output.write_all(b"\",\"")?;
                output.write_all(value)?;
                output.write_all(b"\"")?;
            } else {
                output.write_all(line)?;
            }
            output.write_all(b"\n")?;
        }
        output.flush()?;
    }
    Ok(())
}
