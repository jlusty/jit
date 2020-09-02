use std::{
    cmp,
    convert::TryFrom,
    error::Error,
    ffi::{OsStr, OsString},
    fs::Metadata,
    os::unix::{ffi::OsStrExt, fs::MetadataExt},
    path::{Components, Path, PathBuf},
};

use super::super::database::tree::{OidAndMode, TreeEntry};

// TODO: Remove this Clone?
#[derive(Clone)]
pub struct Entry {
    ctime: u32,
    ctime_nsec: u32,
    mtime: u32,
    mtime_nsec: u32,
    dev: u32,
    ino: u32,
    mode: u32,
    uid: u32,
    gid: u32,
    size: u32,
    oid: String,
    flags: u16,
    path: PathBuf,
}

impl Entry {
    pub const TREE_MODE: u32 = 0o40000;

    pub const REGULAR_MODE: u32 = 0o100644;
    pub const EXECUTABLE_MODE: u32 = 0o100755;
    pub const MAX_PATH_SIZE: u16 = 0xfff;
    pub const ENTRY_BLOCK: usize = 8;

    pub fn new(pathname: PathBuf, oid: &str, stat: Metadata) -> Result<Entry, Box<dyn Error>> {
        let path = pathname;
        let mode = match stat.mode() {
            Entry::EXECUTABLE_MODE => Entry::EXECUTABLE_MODE,
            _ => Entry::REGULAR_MODE,
        };
        let flags = match u16::try_from(path.as_os_str().len()) {
            Ok(path_len) => cmp::min(path_len, Entry::MAX_PATH_SIZE),
            Err(_) => Entry::MAX_PATH_SIZE,
        };

        Ok(Entry {
            ctime: u32::try_from(stat.ctime())?,
            ctime_nsec: u32::try_from(stat.ctime_nsec())?,
            mtime: u32::try_from(stat.mtime())?,
            mtime_nsec: u32::try_from(stat.mtime_nsec())?,
            dev: u32::try_from(stat.dev())?,
            ino: u32::try_from(stat.ino())?,
            mode,
            uid: stat.uid(),
            gid: stat.gid(),
            size: u32::try_from(stat.size())?,
            oid: oid.to_string(),
            flags,
            path,
        })
    }

    pub fn from(entry: Vec<u8>) -> Result<Entry, Box<dyn Error>> {
        // TODO: Tidy up
        Ok(Entry {
            ctime: u32::from_be_bytes(clone_into_array(&entry[0..4])),
            ctime_nsec: u32::from_be_bytes(clone_into_array(&entry[4..8])),
            mtime: u32::from_be_bytes(clone_into_array(&entry[8..12])),
            mtime_nsec: u32::from_be_bytes(clone_into_array(&entry[12..16])),
            dev: u32::from_be_bytes(clone_into_array(&entry[16..20])),
            ino: u32::from_be_bytes(clone_into_array(&entry[20..24])),
            mode: u32::from_be_bytes(clone_into_array(&entry[24..28])),
            uid: u32::from_be_bytes(clone_into_array(&entry[28..32])),
            gid: u32::from_be_bytes(clone_into_array(&entry[32..36])),
            size: u32::from_be_bytes(clone_into_array(&entry[36..40])),
            oid: hex::encode(entry[40..60].to_owned()),
            flags: u16::from_be_bytes(clone_into_array(&entry[60..62])),
            path: PathBuf::from(
                String::from_utf8(entry[62..].to_owned())
                    .unwrap()
                    .trim_matches(char::from(0)),
            ),
        })
    }

    pub fn path(&self) -> &OsStr {
        self.path.as_os_str()
    }

    pub fn to_string(&self) -> Vec<u8> {
        let mut string = self.ctime.to_be_bytes().to_vec();
        string.extend(&self.ctime_nsec.to_be_bytes());
        string.extend(&self.mtime.to_be_bytes());
        string.extend(&self.mtime_nsec.to_be_bytes());
        string.extend(&self.dev.to_be_bytes());
        string.extend(&self.ino.to_be_bytes());
        string.extend(&self.mode.to_be_bytes());
        string.extend(&self.uid.to_be_bytes());
        string.extend(&self.gid.to_be_bytes());
        string.extend(&self.size.to_be_bytes());
        string.append(&mut hex::decode(&self.oid).expect("Failed to convert oid to bytes"));
        string.extend(&self.flags.to_be_bytes());
        string.extend(&self.path.as_path().as_os_str().as_bytes().to_vec());
        if string[string.len() - 1] != 0 {
            string.append(&mut String::from("\0").into_bytes());
        }
        while string.len() % Entry::ENTRY_BLOCK != 0 {
            string.append(&mut String::from("\0").into_bytes());
        }
        string
    }
}

impl TreeEntry for Entry {
    fn name(&self) -> &Path {
        &self.path
    }

    fn parent_directories(&self) -> Components {
        match self.path.parent() {
            Some(parent) => parent.components(),
            None => Path::new("").components(),
        }
    }

    // TODO: Return result
    fn basename(&self) -> OsString {
        self.path
            .file_name()
            .expect("Failed to find file name for entry")
            .to_os_string()
    }
}

impl OidAndMode for Entry {
    fn oid(&self) -> &str {
        &self.oid
    }

    fn mode(&self) -> u32 {
        match self.mode {
            Entry::EXECUTABLE_MODE => Entry::EXECUTABLE_MODE,
            _ => Entry::REGULAR_MODE,
        }
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
