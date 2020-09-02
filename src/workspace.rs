use std::{
    ffi::OsString,
    fs::{self, Metadata},
    path::{Path, PathBuf},
};

pub struct Workspace {
    pathname: PathBuf,
}

impl Workspace {
    pub fn new(pathname: PathBuf) -> Workspace {
        Workspace { pathname }
    }

    pub fn list_workspace_files(&self, pathname: &Path) -> Vec<PathBuf> {
        Workspace::list_files(&self.pathname, pathname)
    }

    pub fn list_files(root_path: &Path, pathname: &Path) -> Vec<PathBuf> {
        // TODO: Make ignore paths better
        let ignore_paths: [OsString; 2] = [OsString::from(".git"), OsString::from("target")];

        // TODO: Maybe simplify checking directory here?
        if pathname.is_dir() {
            let dir_entries = fs::read_dir(&pathname).unwrap();

            dir_entries
                .flat_map(|entry| {
                    let entry = entry.unwrap();
                    let path = entry.path();
                    if ignore_paths.contains(&entry.file_name()) {
                        Vec::new()
                    } else if path.is_dir() {
                        Workspace::list_files(&root_path, &path)
                    } else {
                        let relative_path = entry.path();
                        vec![PathBuf::from(
                            relative_path
                                .strip_prefix(&root_path)
                                .expect("Failed to get file path relative to directory root"),
                        )]
                    }
                })
                .collect()
        } else if ignore_paths.iter().any(|e| e == pathname.as_os_str()) {
            Vec::new()
        } else {
            vec![PathBuf::from(pathname.strip_prefix(&root_path).expect(
                "Failed to get file path relative to directory root",
            ))]
        }
    }

    pub fn read_file(&self, path: &Path) -> String {
        fs::read_to_string(path).expect("Unable to read file")
    }

    // TODO: Return "Result"s everywhere?
    pub fn stat_file(&self, path: &Path) -> Metadata {
        self.pathname
            .join(path)
            .metadata()
            .expect("Failed to get metadata for path")
    }
}
