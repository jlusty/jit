use std::{
    error::{self, Error},
    fmt,
    fs::File,
    io::Read,
};

use sha1::{Digest, Sha1};

use super::super::lockfile::Lockfile;

pub struct Checksum {
    file: File,
    digest: Sha1,
}

impl Checksum {
    const CHECKSUM_SIZE: usize = 20;

    pub fn new(file: File) -> Checksum {
        Checksum {
            file,
            digest: Sha1::new(),
        }
    }

    pub fn read(&mut self, size: usize) -> Result<Vec<u8>, Box<dyn error::Error>> {
        let mut buffer = vec![0; size];
        let bytesize = self.file.read(&mut buffer)?;

        if bytesize != size {
            Err(EndOfFile.into())
        } else {
            let data = buffer;
            self.digest.update(&data);
            Ok(data)
        }
    }

    pub fn verify_checksum(&mut self) -> Result<(), Box<dyn error::Error>> {
        let mut buffer = [0; Checksum::CHECKSUM_SIZE];
        self.file.read_exact(&mut buffer)?;

        let data = buffer;
        if data == self.digest.finalize_reset().as_slice() {
            Ok(())
        } else {
            Err(Invalid.into())
        }
    }
}

pub struct ChecksumWriter<'a> {
    file: &'a mut Lockfile,
    digest: Sha1,
}

impl ChecksumWriter<'_> {
    pub fn new(file: &mut Lockfile) -> ChecksumWriter {
        ChecksumWriter {
            file,
            digest: Sha1::new(),
        }
    }

    pub fn write(&mut self, bytes: &[u8]) {
        self.file.write(bytes).unwrap();
        self.digest.update(bytes);
    }

    pub fn write_checksum(&mut self) {
        let digest = self.digest.clone().finalize();
        self.file.write(digest.as_slice()).unwrap();
    }
}

#[derive(Debug)]
struct EndOfFile;
impl Error for EndOfFile {}
impl fmt::Display for EndOfFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Oh no, something bad went down")
    }
}

// TODO: Maybe combine with invalid in index.rs
#[derive(Debug)]
struct Invalid;
impl Error for Invalid {}
impl fmt::Display for Invalid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Oh no, something bad went down")
    }
}
