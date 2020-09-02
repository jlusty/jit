mod database;
mod entry;
mod index;
mod lockfile;
mod refs;
mod workspace;

use std::{
    env, fs,
    io::{self, Read},
    path::PathBuf,
    process,
};

use database::{author::Author, blob::Blob, commit::Commit, tree::Tree, Database, Storable};
use index::Index;
use lockfile::Lockfile;
use refs::Refs;
use workspace::Workspace;

use chrono::Local;
use structopt::StructOpt;

/// Like git but worse
#[derive(StructOpt)]
enum Cli {
    /// Initialise a repository
    Init {
        /// The path where to initialise the repo [default: current working directory]
        #[structopt(parse(from_os_str))]
        path: Option<std::path::PathBuf>,
    },
    /// Save changes to this repository
    Commit {},
    /// Add files to the index
    Add {
        /// The path of the file to add to the index
        #[structopt(parse(from_os_str))]
        paths: Vec<std::path::PathBuf>,
    },
}

fn main() {
    match Cli::from_args() {
        Cli::Init { path } => {
            let root_path = env::current_dir()
                .expect("Failed to get current working directory")
                .join(match path {
                    Some(path) => path,
                    None => PathBuf::from("."),
                });
            let git_path = root_path.join(".git");

            let dirs = vec!["objects", "refs"];
            for dir in dirs {
                fs::create_dir_all(&git_path.join(dir)).unwrap_or_else(|err| {
                    eprintln!("Error when creating directory {}: {}", dir, err);
                    process::exit(1);
                })
            }

            println!(
                "Initialized empty Jit repository in {}",
                &git_path.canonicalize().unwrap().to_str().unwrap()
            );
        }
        Cli::Commit {} => {
            let root_path = env::current_dir().expect("Failed to get current working directory");
            let git_path = root_path.join(".git");
            let db_path = git_path.join("objects");

            let database = Database::new(db_path);
            let mut index = Index::new(git_path.join("index"));
            let refs = Refs::new(git_path.clone());

            index.load();

            let mut root = Tree::build(index.entries());
            root.traverse(|tree| database.store(tree));

            let parent = refs.read_head();
            let name = env::var("GIT_AUTHOR_NAME").expect("Author name env var not found");
            let email = env::var("GIT_AUTHOR_EMAIL").expect("Author email env var not found");
            let author = Author::new(name, email, Local::now());
            let mut message = String::new();
            io::stdin()
                .read_to_string(&mut message)
                .expect("Error reading from stdin");

            let mut commit = Commit::new(
                parent.clone(),
                root.oid().as_ref().unwrap().to_string(),
                author,
                &message,
            );
            database.store(&mut commit);
            refs.update_head(&commit.oid().as_ref().unwrap().to_string())
                .expect("Failed to write commit to HEAD");

            let is_root = match parent {
                Some(_) => "",
                None => "(root-commit) ",
            };
            println!(
                "[{}{}] {}",
                is_root,
                commit.oid().as_ref().unwrap().to_string(),
                message.lines().next().expect("Error: no commit message")
            );

            Lockfile::new(git_path.join("HEAD"));
        }
        Cli::Add { paths } => {
            let root_path = env::current_dir().expect("Failed to get current working directory");
            let git_path = root_path.join(".git");

            let workspace = Workspace::new(root_path.clone());
            let database = Database::new(git_path.join("objects"));
            let mut index = Index::new(git_path.join("index"));

            index.load_for_update();

            for path in paths {
                let path = root_path.join(path);
                for pathname in workspace.list_workspace_files(&path) {
                    let data = workspace.read_file(&pathname);
                    let stat = workspace.stat_file(&pathname);

                    let mut blob = Blob::new(data);
                    database.store(&mut blob);
                    index.add(pathname, blob.oid().as_ref().unwrap().to_string(), stat);
                }
            }

            index.write_updates();
        }
    }
}
