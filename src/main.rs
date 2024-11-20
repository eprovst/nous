use clap::{error::ErrorKind, CommandFactory, Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

const ROOT_DIR_NAME: &str = ".nous";
const DEFAULT_EXT: &str = "md";
const SUPPORTED_EXTS: [&str; 5] = ["md", "markdown", "org", "txt", "text"];

#[derive(Parser)]
#[command(name = "nous")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new νοῦς realm
    Init {
        /// Directory to initialize as a realm
        #[arg(default_value = ".")]
        root: String,
    },

    /// List forwardlinks of a node
    Bl {
        /// Node to show forwardlinks of
        node: String,
    },

    /// List backlinks of a node
    Fl {
        /// Node to collect backlinks of
        node: String,
    },

    /// Rename a node, correcting backlinks
    Mv {
        /// Old node name
        from: String,
        /// New node name
        to: String,
    },

    /// Remove a node
    Rm {
        /// Node to remove
        node: String,
    },

    /// Create a file for a new node
    New {
        /// Name of node to create
        node: String,
    },

    /// Show file path of a node
    Path {
        /// Node to show path of
        node: String,
    },

    /// List nodes in realm
    Ls,
}

fn main() {
    let cli = Cli::parse();

    if let Commands::Init { root } = &cli.command {
        init_realm(Path::new(root));
    } else if let Some(root) = find_root(Path::new(".")) {
        match &cli.command {
            Commands::Bl { node: _ } => todo!(),
            Commands::Fl { node: _ } => todo!(),
            Commands::Mv { from: _, to: _ } => todo!(),
            Commands::Rm { node } => remove_node(&root, &node),
            Commands::New { node } => new_node(&root, &node),
            Commands::Path { node } => path_to_node(&root, &node),
            Commands::Ls => list_nodes(&root),
            Commands::Init { root: _ } => unreachable!(),
        }
    } else {
        let mut cmd = Cli::command();
        cmd.find_subcommand_mut("init")
            // TODO: Deal with errors
            .unwrap()
            .error(
                ErrorKind::Io,
                "not within a νοῦς realm; you could use 'init' to create one.",
            )
            .exit();
    }
}

fn find_root(start_dir: &Path) -> Option<PathBuf> {
    let abs_start = start_dir.canonicalize().ok()?;
    for anc in abs_start.ancestors() {
        if anc.join(ROOT_DIR_NAME).is_dir() {
            return Some(anc.to_path_buf());
        }
    }
    None
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

fn realm_walker(root: &Path) -> impl Iterator<Item = PathBuf> {
    WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_path_buf())
        .filter(|p| {
            p.extension().map_or(false, |e| {
                SUPPORTED_EXTS.iter().any(|s| e.eq_ignore_ascii_case(s))
            })
        })
}

fn list_nodes(root: &Path) {
    for path in realm_walker(root) {
        if let Some(osstem) = path.file_stem() {
            if let Some(stem) = osstem.to_str() {
                println!("{}", stem);
            } else {
                eprintln!("warning: failed to interpret name of a node as Unicode.")
            }
        }
    }
}

fn find_node(root: &Path, node: &String) -> impl Iterator<Item = PathBuf> {
    let node = node.clone();
    realm_walker(root).filter(move |p| {
        p.file_stem()
            .map_or(false, |s| s.eq_ignore_ascii_case(&node))
            && p.is_file()
    })
}

fn node_exists(root: &Path, node: &String) -> bool {
    find_node(root, node).next().is_some()
}

fn find_node_once(root: &Path, node: &String) -> Option<PathBuf> {
    let mut matcher = find_node(root, node);
    let result = matcher.next();
    if let Some(_) = matcher.next() {
        eprintln!(
            "warning: multiple paths found for '{}', only using first.",
            node
        );
    }
    result
}

fn path_to_node(root: &Path, node: &String) {
    match find_node_once(root, node) {
        Some(path) => println!("{}", path.display()),
        None => eprintln!("warning: node not found."),
    }
}

fn default_file_name(root: &Path, node: &String) -> PathBuf {
    root.join(format!("{}.{}", node, DEFAULT_EXT))
}

fn new_node(root: &Path, node: &String) {
    if node_exists(root, node) {
        eprintln!("warning: node already exists, skipping creation.");
    } else {
        fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&default_file_name(root, node))
            // TODO: Deal with errors
            .unwrap();
    }
}

fn remove_node(root: &Path, node: &String) {
    if let Some(path) = find_node_once(root, node) {
        // TODO: Deal with errors
        fs::remove_file(path).unwrap();
    } else {
        eprintln!("warning: node does not exist, skipping removal.");
    }
}

fn init_realm(target: &Path) {
    match find_root(Path::new(target)) {
        Some(root) => {
            let mut cmd = Cli::command();
            cmd.error(
                ErrorKind::Io,
                format!(
                    "target directory already within the νοῦς realm '{}'.",
                    root.display()
                ),
            )
            .exit();
        }
        // TODO: Deal with errors
        None => fs::create_dir_all(Path::new(target).join(ROOT_DIR_NAME)).unwrap(),
    }
}
