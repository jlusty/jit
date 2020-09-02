// *** File probably not needed, leaving for future reference just in case ***

use std::{
    ffi::OsString,
    path::{Components, Path, PathBuf},
};

use super::database::tree::{OidAndMode, TreeEntry};

pub struct Entry {
    name: PathBuf,
    oid: String,
    stat_mode: u32,
}

impl Entry {
    pub const REGULAR_MODE: u32 = 0o100644;
    pub const EXECUTABLE_MODE: u32 = 0o100755;
    pub const _DIRECTORY_MODE: u32 = 0o40000;

    pub fn _new(name: PathBuf, oid: &str, stat_mode: u32) -> Entry {
        Entry {
            name,
            oid: oid.to_string(),
            stat_mode,
        }
    }
}

impl TreeEntry for Entry {
    fn name(&self) -> &Path {
        &self.name
    }

    fn parent_directories(&self) -> Components {
        match self.name.parent() {
            Some(parent) => parent.components(),
            None => Path::new("").components(),
        }
    }

    // TODO: Return a result
    fn basename(&self) -> OsString {
        self.name
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
        match self.stat_mode {
            Entry::EXECUTABLE_MODE => Entry::EXECUTABLE_MODE,
            _ => Entry::REGULAR_MODE,
        }
    }
}
