use super::csv;
use std::collections::hash_map::RandomState;
use std::fs;
use std::hash::BuildHasher;
use std::io;
use std::path::Path;

struct Node {
    begin: usize,
    end: usize,
    hash: u64,
    root: usize,
}

#[derive(Default)]
pub struct People {
    bytes: Vec<u8>,
    nodes: Vec<Node>,
    slots: Vec<usize>,
    hasher: RandomState,
}

const EMPTY: usize = usize::MAX;

impl People {
    fn name(&self, i: usize) -> &[u8] {
        let node = &self.nodes[i];
        &self.bytes[node.begin..node.end]
    }

    // A hash selects the probe sequence. Only an exact byte match identifies a user.
    fn slot(&self, name: &[u8], hash: u64) -> usize {
        let mask = self.slots.len() - 1;
        let mut slot = hash as usize & mask;
        loop {
            let i = self.slots[slot];
            if i == EMPTY || (self.nodes[i].hash == hash && self.name(i) == name) {
                return slot;
            }
            slot = (slot + 1) & mask;
        }
    }

    fn grow(&mut self) {
        let size = (self.slots.len() * 2).max(16);
        self.slots = vec![EMPTY; size];
        for (i, node) in self.nodes.iter().enumerate() {
            let slot = self.slot(self.name(i), node.hash);
            self.slots[slot] = i;
        }
    }

    fn intern(&mut self, name: &[u8]) -> usize {
        if self.nodes.len() * 4 >= self.slots.len() * 3 {
            self.grow();
        }
        let hash = self.hasher.hash_one(name);
        let slot = self.slot(name, hash);
        if self.slots[slot] != EMPTY {
            return self.slots[slot];
        }
        let index = self.nodes.len();
        let begin = self.bytes.len();
        self.bytes.extend_from_slice(name);
        self.nodes.push(Node { begin, end: self.bytes.len(), hash, root: index });
        self.slots[slot] = index;
        index
    }

    fn root(&mut self, mut i: usize) -> usize {
        while self.nodes[i].root != i {
            let next = self.nodes[self.nodes[i].root].root;
            self.nodes[i].root = next;
            i = next;
        }
        i
    }

    fn link(&mut self, a: &[u8], b: &[u8]) {
        let a = self.intern(a);
        let b = self.intern(b);
        let left = self.root(a);
        let right = self.root(b);
        if self.name(left) <= self.name(right) {
            self.nodes[right].root = left;
        } else {
            self.nodes[left].root = right;
        }
    }

    pub fn load(dir: &Path) -> io::Result<Self> {
        let mut people = Self::default();
        people.grow();
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(people),
            Err(err) => return Err(err),
        };
        for entry in entries {
            let path = entry?.path();
            if path.extension().is_none_or(|ext| ext != "csv") { continue; }
            let part = fs::read(path)?;
            for row in part.split(|&b| b == b'\n').skip(1) {
                if let Some([a, b]) = csv::fields(row) {
                    if !a.is_empty() && !b.is_empty() { people.link(a, b); }
                }
            }
        }
        for i in 0..people.nodes.len() {
            let root = people.root(i);
            people.nodes[i].root = root;
        }
        Ok(people)
    }

    pub fn hash_name(&self, name: &[u8]) -> u64 {
        self.hasher.hash_one(name)
    }

    pub fn canonical<'a>(&'a self, uid: &'a [u8]) -> (&'a [u8], u64) {
        let hash = self.hash_name(uid);
        let i = self.slots[self.slot(uid, hash)];
        if i == EMPTY { return (uid, hash); }
        let root = self.nodes[i].root;
        (self.name(root), self.nodes[root].hash)
    }

    pub fn mappings(&self) -> impl Iterator<Item = (&[u8], &[u8])> {
        self.nodes.iter().enumerate().map(|(i, node)| (self.name(i), self.name(node.root)))
    }
}
