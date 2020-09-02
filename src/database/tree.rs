use std::{
    ffi::OsString,
    os::unix::ffi::OsStrExt,
    path::{Components, Path},
};

use indexmap::IndexMap;

use super::{super::index::entry::Entry, Storable};

pub trait TreeEntry {
    fn name(&self) -> &Path;
    fn parent_directories(&self) -> Components;
    fn basename(&self) -> OsString;
}

pub struct Tree {
    oid: Option<String>,
    entries: IndexMap<OsString, TreeNode>,
}

pub trait OidAndMode {
    fn oid(&self) -> &str;
    fn mode(&self) -> u32;
}

enum TreeNode {
    Tree(Tree),
    Entry(Entry),
}

impl OidAndMode for TreeNode {
    fn oid(&self) -> &str {
        match self {
            TreeNode::Tree(tree) => <Tree as OidAndMode>::oid(tree),
            TreeNode::Entry(entry) => entry.oid(),
        }
    }

    fn mode(&self) -> u32 {
        match self {
            TreeNode::Tree(tree) => tree.mode(),
            TreeNode::Entry(entry) => entry.mode(),
        }
    }
}

impl Tree {
    pub fn new() -> Tree {
        Tree {
            oid: None,
            entries: IndexMap::new(),
        }
    }

    // Assume index entries passed in are sorted
    pub fn build<'a, I>(sorted_entries: I) -> Tree
    where
        I: IntoIterator<Item = &'a Entry>,
    {
        let mut root = Tree::new();

        sorted_entries.into_iter().for_each(|entry| {
            root.add_entry(entry.parent_directories(), entry);
        });

        root
    }

    // TODO: Make this work with generic entry <T: TreeEntry>
    fn add_entry(&mut self, mut parents: Components, entry: &Entry) {
        // TODO: Do this better
        let entry = entry.to_owned();

        let base_component = match parents.next() {
            None => {
                self.entries
                    .insert(entry.basename(), TreeNode::Entry(entry));
                return;
            }
            Some(component) => component.as_os_str().to_owned(),
        };

        let tree_node = self
            .entries
            .entry(base_component)
            .or_insert(TreeNode::Tree(Tree::new()));
        let tree = match tree_node {
            TreeNode::Tree(tree) => tree,
            TreeNode::Entry(_) => panic!("Found entry with children"),
        };
        tree.add_entry(parents, &entry)
    }

    pub fn traverse<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Tree) + Clone,
    {
        self.entries.values_mut().for_each(|entry| {
            if let TreeNode::Tree(tree) = entry {
                tree.traverse(f.clone())
            }
        });
        f(self);
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

impl OidAndMode for Tree {
    fn oid(&self) -> &str {
        self.oid
            .as_ref()
            .expect("Tried to get oid on Tree without oid")
    }

    fn mode(&self) -> u32 {
        Entry::TREE_MODE
    }
}

impl Storable for Tree {
    fn oid(&self) -> Option<&str> {
        self.oid.as_deref()
    }

    fn set_oid(&mut self, oid: String) {
        self.oid = Some(oid)
    }

    fn type_(&self) -> &str {
        "tree"
    }

    fn to_string(&self) -> Vec<u8> {
        // Note entries sorted on creation of tree
        let entries_vec = self
            .entries
            .keys()
            .map(|name| {
                let entry = self.entries.get(name).unwrap();
                let mut oid_bytes =
                    hex::decode(&entry.oid()).expect("Failed to convert oid to bytes");
                let mut s = format!("{:o} ", entry.mode()).into_bytes();
                s.extend(name.as_bytes());
                s.extend(vec![0]);
                s.append(&mut oid_bytes);
                s
            })
            .collect::<Vec<Vec<u8>>>();
        entries_vec.concat()
    }
}
