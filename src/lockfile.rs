use std::{
    error::{self, Error},
    fmt,
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

pub struct Lockfile {
    file_path: PathBuf,
    lock_path: PathBuf,
    lock: Option<File>,
}

impl Lockfile {
    pub fn new(path: PathBuf) -> Lockfile {
        let mut lock_path = path.clone();
        lock_path.set_extension("lock");
        Lockfile {
            file_path: path,
            lock_path,
            lock: None,
        }
    }

    pub fn hold_for_update(&mut self) -> Result<bool, Box<dyn error::Error>> {
        if self.lock.is_none() {
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&self.lock_path);
            match file {
                Ok(file) => {
                    self.lock = Some(file);
                    Ok(true)
                }
                Err(error) => match error.kind() {
                    ErrorKind::AlreadyExists => Ok(false),
                    ErrorKind::NotFound => Err(MissingParent.into()),
                    ErrorKind::PermissionDenied => Err(NoPermission.into()),
                    _ => {
                        panic!("Error when creating file: {:?}", error);
                    }
                },
            }
        } else {
            Ok(false)
        }
    }

    pub fn write(&mut self, bytes: &[u8]) -> Result<(), StaleLock> {
        match self.lock.as_ref() {
            Some(mut lock) => {
                lock.write_all(bytes).expect("Error writing data to file");
                Ok(())
            }
            None => Err(StaleLock),
        }
    }

    pub fn commit(&mut self) -> Result<(), StaleLock> {
        match self.lock.as_ref() {
            Some(_) => {
                fs::rename(&self.lock_path, &self.file_path).expect("Error renaming locked file");
                self.lock = None;
                Ok(())
            }
            None => Err(StaleLock),
        }
    }

    pub fn rollback(&mut self) -> Result<(), StaleLock> {
        match self.lock.as_ref() {
            Some(_) => {
                fs::remove_file(&self.lock_path).expect("Error removing unused lockfile");
                self.lock = None;
                Ok(())
            }
            None => Err(StaleLock),
        }
    }

    pub fn file_path(&self) -> &Path {
        &self.file_path
    }
}

#[derive(Debug)]
struct MissingParent;
impl Error for MissingParent {}
impl fmt::Display for MissingParent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Oh no, something bad went down")
    }
}

#[derive(Debug)]
struct NoPermission;
impl Error for NoPermission {}
impl fmt::Display for NoPermission {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Oh no, something bad went down")
    }
}

#[derive(Debug)]
pub struct StaleLock;
impl Error for StaleLock {}
impl fmt::Display for StaleLock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Oh no, something bad went down")
    }
}
