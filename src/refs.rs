use std::{
    error::{self, Error},
    fmt, fs,
    path::PathBuf,
};

use super::lockfile::Lockfile;

pub struct Refs {
    pathname: PathBuf,
}

impl Refs {
    pub fn new(pathname: PathBuf) -> Refs {
        Refs { pathname }
    }

    pub fn update_head(&self, oid: &str) -> Result<(), Box<dyn error::Error>> {
        let mut lockfile = Lockfile::new(self.head_path());
        if lockfile.hold_for_update().is_ok() {
            lockfile.write(oid.to_string().as_bytes())?;
            lockfile.write(String::from("\n").as_bytes())?;
            lockfile.commit()?;
            Ok(())
        } else {
            Err(LockDenied.into())
        }
    }

    fn head_path(&self) -> PathBuf {
        self.pathname.join("HEAD")
    }

    pub fn read_head(&self) -> Option<String> {
        if self.head_path().exists() {
            let head = fs::read_to_string(self.head_path()).expect("Failed to read HEAD file");
            Some(head.trim().to_string())
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct LockDenied;
impl Error for LockDenied {}
impl fmt::Display for LockDenied {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Oh no, something bad went down")
    }
}
