mod checksum;
pub mod entry;

use std::{
    collections::{hash_map::Entry as EntryApi, BTreeMap, HashMap, HashSet},
    error::{self, Error},
    ffi::{OsStr, OsString},
    fmt,
    fs::{File, Metadata},
    io::ErrorKind,
    path::PathBuf,
};

use super::database::tree::TreeEntry;
use super::lockfile::Lockfile;
use checksum::{Checksum, ChecksumWriter};
use entry::Entry;

pub struct Index {
    // We do care about order, so this needs to be a BTreeMap
    // Could use HashMap and entry ordering?
    entries: BTreeMap<OsString, Entry>,
    parents: HashMap<OsString, HashSet<OsString>>,
    lockfile: Lockfile,
    changed: bool,
}

impl Index {
    pub fn new(pathname: PathBuf) -> Index {
        Index {
            // Note entries already sorted so no need to add SortedSet etc
            entries: BTreeMap::new(),
            parents: HashMap::new(),
            lockfile: Lockfile::new(pathname),
            changed: false,
        }
    }

    pub fn add(&mut self, pathname: PathBuf, oid: String, stat: Metadata) {
        let entry = Entry::new(pathname, &oid, stat).expect("Failed to create entry");
        self.discard_conflicts(&entry);
        self.store_entry(entry);
        self.changed = true;
    }

    pub fn write_updates(&mut self) {
        if !self.changed {
            self.lockfile
                .rollback()
                .expect("Failed to rollback lockfile");
            return;
        }

        let mut writer = ChecksumWriter::new(&mut self.lockfile);

        let mut header = String::from("DIRC").into_bytes();
        header.extend(&2u32.to_be_bytes());
        header.extend(&(self.entries.len() as u32).to_be_bytes());
        writer.write(&header);
        // TODO: Tidy this up
        let mut value_strings = Vec::new();
        for value in self.entries.values() {
            value_strings.push(value.to_string());
        }
        for value in value_strings {
            writer.write(&value);
        }

        writer.write_checksum();
        self.lockfile.commit().unwrap();

        self.changed = false;
    }

    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.values()
    }

    pub fn load_for_update(&mut self) -> bool {
        if self.lockfile.hold_for_update().is_ok() {
            self.load();
            true
        } else {
            false
        }
    }

    pub fn load(&mut self) {
        self.clear();
        let file = self.open_index_file();

        if let Some(file) = file {
            let mut reader = Checksum::new(file);
            let count = self
                .read_header(&mut reader)
                .expect("Failed to read index header");
            self.read_entries(&mut reader, count);
            reader
                .verify_checksum()
                .expect("Failed to verify index checksum");
        }
        // TODO: Confirm no need to close file
    }

    fn clear(&mut self) {
        // TODO: Determine if I need to do anything with lockfile here - I think is OK like this
        self.entries = BTreeMap::new();
        self.parents = HashMap::new();
        self.changed = false;
    }

    // TODO: Convert to returning result?
    fn open_index_file(&self) -> Option<File> {
        match File::open(self.lockfile.file_path()) {
            Ok(file) => Some(file),
            Err(error) => match error.kind() {
                ErrorKind::NotFound => None,
                _ => {
                    panic!("Error when opening file: {:?}", error);
                }
            },
        }
    }

    const HEADER_SIZE: usize = 12;
    const SIGNATURE: &'static str = "DIRC";
    const VERSION: u32 = 2;

    fn read_header(&self, reader: &mut Checksum) -> Result<u32, Box<dyn error::Error>> {
        let data = reader
            .read(Index::HEADER_SIZE)
            .expect("Failed to read header data from index");
        let signature = String::from_utf8(data[0..4].to_owned()).unwrap();
        let version = u32::from_be_bytes(clone_into_array(&data[4..8]));
        let count = u32::from_be_bytes(clone_into_array(&data[8..12]));

        if signature != Index::SIGNATURE || version != Index::VERSION {
            return Err(Invalid.into());
        }
        Ok(count)
    }

    const ENTRY_BLOCK: usize = 8;
    const ENTRY_MIN_SIZE: usize = 64;

    fn read_entries(&mut self, reader: &mut Checksum, count: u32) {
        for _i in 0..count {
            let mut entry = reader
                .read(Index::ENTRY_MIN_SIZE)
                .expect("Failed to read entry data from index");

            while entry[entry.len() - 1..] != [0u8] {
                entry.append(
                    &mut reader
                        .read(Index::ENTRY_BLOCK)
                        .expect("Failed to read entry data from index"),
                );
            }

            self.store_entry(Entry::from(entry).expect("Failed to parse entry in index"));
        }
    }

    fn store_entry(&mut self, entry: Entry) {
        // TODO: Confirm to_owned here, also confirm clone
        self.entries.insert(entry.path().to_owned(), entry.clone());

        for parent in entry.name().ancestors() {
            let child_set = self
                .parents
                .entry(parent.as_os_str().to_owned())
                .or_insert(HashSet::new());

            child_set.insert(entry.name().as_os_str().to_owned());
        }
    }

    fn discard_conflicts(&mut self, entry: &Entry) {
        for parent in entry.name().ancestors() {
            self.entries.remove(parent.as_os_str());
        }
        self.remove_children(entry.name().as_os_str());
    }

    fn remove_children(&mut self, path: &OsStr) {
        let children = self.parents.get(path);
        match children {
            Some(children) => {
                for child in children {
                    self.remove_entry(child)
                }
            }
            None => return,
        }
    }

    fn remove_entry(&mut self, entry_path: &OsStr) {
        self.entries.remove(entry_path);

        for parent in entry.name().ancestors() {
            let child_set = self.parents.entry(parent.as_os_str().to_owned());

            match child_set {
                EntryApi::Occupied(child_set) => {
                    child_set.remove(entry.name().as_os_str().to_owned())
                }
                EntryApi::Vacant() => {}
            }
        }
    }
}

#[derive(Debug)]
struct Invalid;
impl Error for Invalid {}
impl fmt::Display for Invalid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Oh no, something bad went down")
    }
}

// TODO: Tidy, make common
use std::convert::AsMut;

fn clone_into_array<A, T>(slice: &[T]) -> A
where
    A: Sized + Default + AsMut<[T]>,
    T: Clone,
{
    let mut a = Default::default();
    <A as AsMut<[T]>>::as_mut(&mut a).clone_from_slice(slice);
    a
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, path::PathBuf};

    use super::*;

    use rand::Rng;

    fn index() -> Index {
        let tmp_path = PathBuf::from("../tmp");
        let index_path = tmp_path.join("index");
        Index::new(index_path)
    }

    fn stat() -> Metadata {
        PathBuf::from(file!())
            .metadata()
            .expect("Failed to get metadata for path")
    }

    fn random_hex(length: usize) -> String {
        const CHARSET: &[u8] = b"0123456789abcdef";
        let mut rng = rand::thread_rng();

        let oid: String = (0..length)
            .map(|_| {
                let idx = rng.gen_range(0, CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
        oid
    }

    #[test]
    fn it_adds_a_single_file() {
        let mut index: Index = index();
        let stat = stat();
        let oid = random_hex(20);

        index.add(PathBuf::from("alice.txt"), oid, stat);

        assert_eq!(
            vec!["alice.txt"],
            index.entries().map(|e| e.path()).collect::<Vec<&OsStr>>()
        );
    }

    #[test]
    fn it_replaces_a_file_with_a_directory() {
        let mut index: Index = index();

        index.add(PathBuf::from("alice.txt"), random_hex(20), stat());
        index.add(PathBuf::from("bob.txt"), random_hex(20), stat());

        index.add(
            PathBuf::from("alice.txt/nested.txt"),
            random_hex(20),
            stat(),
        );

        assert_eq!(
            vec!["alice.txt/nested.txt", "bob.txt"],
            index.entries().map(|e| e.path()).collect::<Vec<&OsStr>>()
        );
    }

    #[test]
    fn it_replaces_a_directory_with_a_file() {
        let mut index: Index = index();

        index.add(PathBuf::from("alice.txt"), random_hex(20), stat());
        index.add(PathBuf::from("nested/bob.txt"), random_hex(20), stat());

        index.add(PathBuf::from("nested"), random_hex(20), stat());

        assert_eq!(
            vec!["alice.txt", "nested"],
            index.entries().map(|e| e.path()).collect::<Vec<&OsStr>>()
        );
    }

    #[test]
    fn it_recursively_replaces_a_directory_with_a_file() {
        let mut index: Index = index();

        index.add(PathBuf::from("alice.txt"), random_hex(20), stat());
        index.add(PathBuf::from("nested/bob.txt"), random_hex(20), stat());
        index.add(
            PathBuf::from("nested/inner/claire.txt"),
            random_hex(20),
            stat(),
        );

        index.add(PathBuf::from("nested"), random_hex(20), stat());

        assert_eq!(
            vec!["alice.txt", "nested"],
            index.entries().map(|e| e.path()).collect::<Vec<&OsStr>>()
        );
    }
}
