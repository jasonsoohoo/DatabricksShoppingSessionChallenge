use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

#[allow(dead_code)]
mod csv;
#[allow(dead_code)]
mod people;

fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let people = people::People::load(Path::new(&args[0]))?;
    let mut out = BufWriter::new(File::create(&args[1])?);
    for (uid, person) in people.mappings() {
        out.write_all(b"\"")?;
        out.write_all(uid)?;
        out.write_all(b"\",\"")?;
        out.write_all(person)?;
        out.write_all(b"\"\n")?;
    }
    out.flush()
}
