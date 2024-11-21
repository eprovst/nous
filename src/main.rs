use clap::{error::ErrorKind, CommandFactory, Parser, Subcommand};
use pathdiff::diff_paths;
use std::path::{Path, PathBuf};
use std::{env, fs};
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

    /// Print root of this νοῦς realm
    Root {
        /// Print the absolute path
        #[arg(short, long)]
        absolute: bool,
    },

    /// List nodes which this node links to
    Fl {
        /// Node to show links of
        node: String,
    },

    /// List nodes which link to this node
    Bl {
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
        /// Print the absolute path
        #[arg(short, long)]
        absolute: bool,
    },

    /// List nodes in realm
    Ls,
}

fn main() {
    let cli = Cli::parse();

    if let Commands::Init { root } = &cli.command {
        init_realm(Path::new(root));
    } else if let Some(root) = find_root(&current_dir()) {
        match &cli.command {
            Commands::Bl { node: _ } => todo!(),
            Commands::Fl { node: _ } => todo!(),
            Commands::Mv { from: _, to: _ } => todo!(),
            Commands::Rm { node } => remove_node(&root, &node),
            Commands::New { node } => new_node(&root, &node),
            Commands::Path { node, absolute } => path_to_node(&root, &node, *absolute),
            Commands::Ls => list_nodes(&root),
            Commands::Root { absolute } => print_path(&root, *absolute),
            Commands::Init { root: _ } => unreachable!(),
        }
    } else {
        let mut cmd = Cli::command();
        cmd.find_subcommand_mut("init")
            .unwrap() // we know init exists
            .error(
                ErrorKind::Io,
                "not within a νοῦς realm; you could use 'init' to create one.",
            )
            .exit();
    }
}

fn current_dir() -> PathBuf {
    env::current_dir().expect("error: failed to retrieve working directory.")
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

fn try_absolute_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or(path.to_path_buf())
}

fn try_relative_path(path: &Path) -> PathBuf {
    if let Ok(apath) = path.canonicalize() {
        match diff_paths(&apath, &current_dir()) {
            Some(diff) if diff == Path::new("") => Path::new(".").to_path_buf(),
            Some(diff) => diff,
            None => apath,
        }
    } else {
        path.to_path_buf()
    }
}

fn print_path(path: &Path, absolute: bool) {
    if absolute {
        println!("{}", try_absolute_path(path).display())
    } else {
        println!("{}", try_relative_path(path).display())
    }
}

fn realm_walker(root: &Path) -> impl Iterator<Item = PathBuf> {
    fn is_hidden(entry: &DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .map(|s| s.starts_with("."))
            .unwrap_or(false)
    }

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

fn path_to_node(root: &Path, node: &String, absolute: bool) {
    match find_node_once(root, node) {
        Some(path) => print_path(&path, absolute),
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
        let file_name = default_file_name(root, node);
        fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&file_name)
            .expect(&format!(
                "error: failed to create node at '{}'",
                try_relative_path(&file_name).display()
            ));
    }
}

fn remove_node(root: &Path, node: &String) {
    if let Some(path) = find_node_once(root, node) {
        fs::remove_file(&path).expect(&format!(
            "error: failed to remove node at '{}'",
            try_relative_path(&path).display()
        ));
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
                    try_relative_path(&root).display()
                ),
            )
            .exit();
        }
        None => {
            let rootdir = target.join(ROOT_DIR_NAME);
            fs::create_dir_all(&rootdir).expect(&format!(
                "error: failed to create realm root marker '{}'",
                try_relative_path(&rootdir).display()
            ))
        }
    }
}
