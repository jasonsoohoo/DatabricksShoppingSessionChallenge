use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::Path;

#[allow(dead_code)]
mod csv;

#[derive(Default)]
struct People<'a> {
    indices: HashMap<&'a [u8], usize>,
    names: Vec<&'a [u8]>,
    parents: Vec<usize>,
}

impl<'a> People<'a> {
    fn index(&mut self, name: &'a [u8]) -> usize {
        match self.indices.entry(name) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let index = self.names.len();
                entry.insert(index);
                self.names.push(name);
                self.parents.push(index);
                index
            }
        }
    }

    fn root(&mut self, mut i: usize) -> usize {
        while self.parents[i] != i {
            self.parents[i] = self.parents[self.parents[i]];
            i = self.parents[i];
        }
        i
    }

    fn link(&mut self, a: &'a [u8], b: &'a [u8]) {
        let left = self.index(a);
        let right = self.index(b);
        let left = self.root(left);
        let right = self.root(right);
        if self.names[left] <= self.names[right] {
            self.parents[right] = left;
        } else {
            self.parents[left] = right;
        }
    }

    fn write(mut self, output: &Path) -> io::Result<()> {
        let mut writer = BufWriter::with_capacity(1 << 20, File::create(output)?);
        for index in 0..self.names.len() {
            let root = self.root(index);
            // Input validation excludes embedded quotes. Quoting preserves CR.
            writer.write_all(b"\"")?;
            writer.write_all(self.names[index])?;
            writer.write_all(b"\",\"")?;
            writer.write_all(self.names[root])?;
            writer.write_all(b"\"\n")?;
        }
        writer.flush()
    }
}

fn resolve(dir: &Path, output: &Path) -> io::Result<()> {
    // The borrowed keys stay valid until the complete lookup has been written.
    let mut parts = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return People::default().write(output),
        Err(err) => return Err(err),
    };
    for entry in entries {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "csv") {
            parts.push(fs::read(path)?);
        }
    }
    let mut people = People::default();
    for part in &parts {
        for row in part.split(|&b| b == b'\n').skip(1) {
            if let Some([a, b]) = csv::fields(row) {
                if !a.is_empty() && !b.is_empty() {
                    people.link(a, b);
                }
            }
        }
    }
    people.write(output)
}

fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    resolve(Path::new(&args[0]), Path::new(&args[1]))
}
