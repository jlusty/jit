pub mod author;
pub mod blob;
pub mod commit;
pub mod tree;

use std::{
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::PathBuf,
};

use flate2::{write::ZlibEncoder, Compression};
use rand::{seq::SliceRandom, thread_rng};
use sha1::{Digest, Sha1};

pub trait Storable {
    fn oid(&self) -> Option<&str>;
    fn set_oid(&mut self, oid: String);
    fn type_(&self) -> &str;
    // TODO: Maybe rename to to_bytes since this isn't a string?
    fn to_string(&self) -> Vec<u8>;
}

pub struct Database {
    pathname: PathBuf,
}

impl Database {
    pub fn new(pathname: PathBuf) -> Database {
        Database { pathname }
    }

    fn generate_temp_name() -> String {
        const TEMP_CHARS: [char; 62] = [
            'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q',
            'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H',
            'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y',
            'Z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
        ];
        let mut rng = thread_rng();

        format!(
            "tmp_obj_{}",
            (0..6)
                .map(|_i| TEMP_CHARS.choose(&mut rng).unwrap())
                .collect::<String>()
        )
    }

    pub fn store<T: Storable>(&self, object: &mut T) {
        let mut bytes = object.to_string();
        let mut content = format!("{} {}\0", object.type_(), bytes.len()).into_bytes();
        content.append(&mut bytes);

        object.set_oid(hash_bytes(&content));
        self.write_object(object.oid().expect("oid not set on object"), &content);
    }

    pub fn write_object(&self, oid: &str, content: &[u8]) {
        let object_path = self.pathname.join(&oid[..2]).join(&oid[2..]);
        if object_path.exists() {
            return;
        }

        let dirname = object_path.parent().unwrap();
        let temp_path = dirname.join(Database::generate_temp_name());

        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .unwrap_or_else(|error| {
                if error.kind() == ErrorKind::NotFound {
                    fs::create_dir(dirname).expect("Error when creating directory");
                    OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&temp_path)
                        .expect("Error when creating file")
                } else {
                    panic!("Error when creating file: {:?}", error);
                }
            });

        let mut e = ZlibEncoder::new(file, Compression::fast());
        e.write_all(content).expect("Error writing content to file");

        fs::rename(temp_path, object_path).expect("Error renaming temporary file");
    }
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    hex::encode(result)
}
