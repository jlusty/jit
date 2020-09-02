use super::Storable;

pub struct Blob {
    oid: Option<String>,
    data: String,
}

impl Blob {
    pub fn new(data: String) -> Blob {
        Blob { oid: None, data }
    }
}

impl Storable for Blob {
    fn oid(&self) -> Option<&str> {
        self.oid.as_deref()
    }

    fn set_oid(&mut self, oid: String) {
        self.oid = Some(oid)
    }

    fn type_(&self) -> &str {
        "blob"
    }

    fn to_string(&self) -> Vec<u8> {
        self.data.as_bytes().to_vec()
    }
}
