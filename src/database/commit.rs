use super::{author::Author, Storable};

pub struct Commit {
    oid: Option<String>,
    parent: Option<String>,
    tree: String,
    author: Author,
    message: String,
}

impl Commit {
    pub fn new(parent: Option<String>, tree: String, author: Author, message: &str) -> Commit {
        Commit {
            oid: None,
            parent,
            tree,
            author,
            message: message.to_string(),
        }
    }
}

impl Storable for Commit {
    fn oid(&self) -> Option<&str> {
        self.oid.as_deref()
    }

    fn set_oid(&mut self, oid: String) {
        self.oid = Some(oid)
    }

    fn type_(&self) -> &str {
        "commit"
    }

    fn to_string(&self) -> Vec<u8> {
        let mut lines = Vec::new();

        lines.push(format!("tree {}", self.tree));
        match &self.parent {
            Some(parent) => lines.push(format!("parent {}", parent)),
            None => {}
        }
        lines.push(format!("author {}", self.author));
        lines.push(format!("committer {}", self.author));
        lines.push(String::from(""));
        lines.push(self.message.clone());

        lines.join("\n").into_bytes()
    }
}
