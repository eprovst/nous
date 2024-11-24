use crate::config::{
    CLI_STYLE, DEFAULT_EXT, FALLBACK_EDITOR, FALLBACK_PAGER, ROOT_DIR_NAME, SUPPORTED_EXTS,
};
use crate::wikilinks::read_wikilinks;

use clap::{Parser, Subcommand};
use pathdiff;
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use std::{env, fs, process};
use walkdir;

mod config;
mod error_macros;
mod wikilinks;

#[derive(Parser)]
#[command(name = "nous")]
#[command(author, version, about, long_about = None)]
#[command(styles = CLI_STYLE)]
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

    /// Open a node using the default pager
    #[command(visible_alias = "o")]
    Open {
        /// Node to page
        node: String,
        /// Alternative pager to use
        #[arg(short, long)]
        pager: Option<String>,
    },

    /// Edit a node using the default editor
    #[command(visible_alias = "ed")]
    Edit {
        /// Node to edit
        node: String,
        /// Alternative editor to use
        #[arg(short, long)]
        editor: Option<String>,
    },

    /// List nodes which this node links to
    #[command(visible_alias = "fl")]
    Forwardlinks {
        /// Node to show links of
        node: String,
        /// Print the path
        #[arg(short, long)]
        path: bool,
        /// Print the absolute path
        #[arg(short, long)]
        absolute: bool,
    },

    /// List nodes which link to this node
    #[command(visible_alias = "bl")]
    Backlinks {
        /// Node to collect backlinks of
        node: String,
        /// Print the path
        #[arg(short, long)]
        path: bool,
        /// Print the absolute path
        #[arg(short, long)]
        absolute: bool,
    },

    /// List nodes which link to or are linked from this node
    #[command(visible_alias = "ln")]
    Links {
        /// Node to show links of
        node: String,
        /// Print the path
        #[arg(short, long)]
        path: bool,
        /// Print the absolute path
        #[arg(short, long)]
        absolute: bool,
    },

    /// Rename a node, correcting backlinks
    #[command(visible_alias = "mv")]
    Move {
        /// Old node name
        from: String,
        /// New node name
        to: String,
    },

    /// Remove a node
    #[command(visible_alias = "rm")]
    Remove {
        /// Node to remove
        node: String,
    },

    /// Touch the file of a (new) node
    Touch {
        /// Name of (new) node to touch
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
    #[command(visible_alias = "ls")]
    List {
        /// Print the path
        #[arg(short, long)]
        path: bool,
        /// Print the absolute path
        #[arg(short, long)]
        absolute: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    if let Commands::Init { root } = &cli.command {
        init_realm(Path::new(root));
        return;
    }

    let Some(root) = find_root(&current_dir()) else {
        error!("not within a νοῦς realm; you could use 'init' to create one")
    };

    match &cli.command {
        Commands::Backlinks {
            node,
            path,
            absolute,
        } => list_backlinks(&root, &node, *path, *absolute),
        Commands::Forwardlinks {
            node,
            path,
            absolute,
        } => list_forwardlinks(&root, &node, *path, *absolute),
        Commands::Links {
            node,
            path,
            absolute,
        } => list_links(&root, &node, *path, *absolute),
        Commands::Move { from: _, to: _ } => todo!(),
        Commands::Remove { node } => remove_node(&root, &node),
        Commands::Edit { node, editor } => edit_node(&root, &node, editor.into()),
        Commands::Open { node, pager } => open_node(&root, &node, pager.into()),
        Commands::Touch { node } => touch_node(&root, &node),
        Commands::Path { node, absolute } => path_to_node(&root, &node, *absolute),
        Commands::List { path, absolute } => list_nodes(&root, *path, *absolute),
        Commands::Root { absolute } => println_path(&root, *absolute),
        Commands::Init { root: _ } => unreachable!(),
    }
}

fn init_realm(target: &Path) {
    match find_root(Path::new(target)) {
        Some(root) => {
            error!(
                "target directory already within the νοῦς realm '{}'",
                try_relative_path(&root).display()
            )
        }
        None => {
            let rootdir = target.join(ROOT_DIR_NAME);
            fs::create_dir_all(&rootdir).unwrap_or_else(|_| {
                error!(
                    "failed to create realm root marker '{}'",
                    try_relative_path(&rootdir).display()
                )
            })
        }
    }
}

fn list_forwardlinks(root: &Path, node: &String, path: bool, absolute: bool) {
    if let Some(node_path) = find_node_once(root, node, false) {
        let Ok(mut f) = fs::File::open(node_path) else {
            error!("failed to open file of '{node}'")
        };
        for link in read_wikilinks(&mut f)
            .map(|(_, l)| l)
            .collect::<BTreeSet<_>>()
        {
            if absolute || path {
                match find_node_once(root, &link, false) {
                    Some(path) => println_path(&path, absolute),
                    None => warn!("no file found for '{link}'"),
                }
            } else {
                println!("{link}")
            }
        }
    }
}

fn list_backlinks(root: &Path, node: &String, path: bool, absolute: bool) {
    realm_walker(root)
        .par_bridge()
        .filter(|p| {
            fs::File::open(p).is_ok_and(|mut f| {
                read_wikilinks(&mut f).any(|(_, l)| node.eq_ignore_ascii_case(&l))
            })
        })
        .for_each(|bl| {
            if absolute || path {
                println_path(&bl, absolute)
            } else {
                println_node(&bl)
            }
        });
}

fn list_links(root: &Path, node: &String, path: bool, absolute: bool) {
    let style = CLI_STYLE.get_header();
    println!("{style}Backlinks:{style:#}");
    list_backlinks(root, node, path, absolute);

    println!("\n{style}Forward links:{style:#}");
    list_forwardlinks(root, node, path, absolute);
}

fn remove_node(root: &Path, node: &String) {
    match find_node_once(root, node, true) {
        Some(path) => fs::remove_file(&path).unwrap_or_else(|_| {
            error!(
                "failed to remove node at '{}'",
                try_relative_path(&path).display()
            )
        }),
        None => warn!("node does not exist, skipping removal"),
    }
}

fn edit_node(root: &Path, node: &String, editor: Option<&String>) {
    let path = find_node_once(root, node, false).unwrap_or(default_file_name(root, node));

    let editor = editor
        .cloned()
        .or_else(|| env::var("VISUAL").ok())
        .or_else(|| env::var("EDITOR").ok())
        .unwrap_or(FALLBACK_EDITOR.into());
    let mut editor_args = editor.split_whitespace();

    let start_time = Instant::now();
    let result = process::Command::new(editor_args.next().unwrap_or(FALLBACK_EDITOR))
        .args(editor_args)
        .arg(path)
        .spawn()
        .and_then(|mut c| c.wait());

    match result {
        Ok(code) if code.success() => {}
        Ok(_code) => {
            error!("editor did not exit successfully, consider using the --editor flag")
        }
        Err(_) => {
            error!("failed to launch '{editor}', consider using the --editor flag")
        }
    }

    if start_time.elapsed() <= Duration::from_millis(100) {
        warn!("editor exited under 100ms, this might indicate failure; consider using the --editor flag")
    }
}

fn open_node(root: &Path, node: &String, pager: Option<&String>) {
    let Some(path) = find_node_once(root, node, false) else {
        error!("node does not exist")
    };

    let pager = pager
        .cloned()
        .or_else(|| env::var("PAGER").ok())
        .unwrap_or(FALLBACK_PAGER.into());
    let mut pager_args = pager.split_whitespace();

    let start_time = Instant::now();
    let result = process::Command::new(pager_args.next().unwrap_or(FALLBACK_PAGER))
        .args(pager_args)
        .arg(path)
        .spawn()
        .and_then(|mut c| c.wait());

    match result {
        Ok(code) if code.success() => {}
        Ok(_code) => {
            error!("pager did not exit successfully, consider using the --editor flag")
        }
        Err(_) => {
            error!("failed to launch '{pager}', consider using the --pager flag")
        }
    }

    if start_time.elapsed() <= Duration::from_millis(100) {
        warn!("pager exited under 100ms, this might indicate failure; consider using the --pager flag")
    }
}

fn touch_node(root: &Path, node: &String) {
    let path = find_node_once(root, node, false).unwrap_or(default_file_name(root, node));
    if path.file_name().is_some() && path.parent().map_or(true, |p| p.is_dir()) {
        let Ok(file) = fs::OpenOptions::new().create(true).write(true).open(&path) else {
            error!(
                "failed to touch file '{}'",
                try_relative_path(&path).display()
            )
        };
        let _ = file.set_modified(SystemTime::now()); // not a problem if this fails
    } else {
        warn!("node name results in an invalid file, skipping")
    }
}

fn path_to_node(root: &Path, node: &String, absolute: bool) {
    match find_node_once(root, node, false) {
        Some(path) => println_path(&path, absolute),
        None => warn!("node not found"),
    }
}

fn list_nodes(root: &Path, path: bool, absolute: bool) {
    for p in realm_walker(root) {
        if path || absolute {
            println_path(&p, absolute)
        } else {
            println_node(&p)
        }
    }
}

fn println_path(path: &Path, absolute: bool) {
    if absolute {
        println!("{}", try_absolute_path(path).display())
    } else {
        println!("{}", try_relative_path(path).display())
    }
}

pub fn current_dir() -> PathBuf {
    env::current_dir().unwrap_or_else(|_| error!("failed to retrieve working directory"))
}

pub fn find_root(start_dir: &Path) -> Option<PathBuf> {
    let abs_start = start_dir.canonicalize().ok()?;
    for anc in abs_start.ancestors() {
        if anc.join(ROOT_DIR_NAME).is_dir() {
            return Some(anc.to_path_buf());
        }
    }
    None
}

pub fn try_absolute_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or(path.to_path_buf())
}

pub fn try_relative_path(path: &Path) -> PathBuf {
    if let Ok(apath) = path.canonicalize() {
        match pathdiff::diff_paths(&apath, &current_dir()) {
            Some(diff) if diff == Path::new("") => Path::new(".").to_path_buf(),
            Some(diff) => diff,
            None => apath,
        }
    } else {
        path.to_path_buf()
    }
}

// Returns filename of a path, if a filename is present
// Invalid Unicode characters are replaced by U+FFFD
pub fn node_from_path(path: &Path) -> Option<String> {
    Some(path.file_stem()?.to_string_lossy().to_string())
}

pub fn println_node(path: &Path) {
    match node_from_path(path) {
        Some(node) => println!("{node}"),
        None => warn!("empty node name, skipping"),
    }
}

pub fn realm_walker(root: &Path) -> impl Iterator<Item = PathBuf> {
    fn is_hidden(entry: &walkdir::DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .map(|s| s.starts_with("."))
            .unwrap_or(false)
    }

    walkdir::WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| {
            p.extension().map_or(false, |e| {
                SUPPORTED_EXTS.iter().any(|s| e.eq_ignore_ascii_case(s))
            })
        })
}

fn find_node(root: &Path, node: &String) -> impl Iterator<Item = PathBuf> {
    let node = node.clone();
    realm_walker(root).filter(move |p| {
        p.file_stem()
            .map_or(false, |s| s.eq_ignore_ascii_case(&node))
            && p.is_file()
    })
}

pub fn find_node_once(root: &Path, node: &String, strict: bool) -> Option<PathBuf> {
    let mut matcher = find_node(root, node);
    let result = matcher.next();
    if let Some(_) = matcher.next() {
        if strict {
            error!("multiple paths found for '{node}'")
        } else {
            warn!("multiple paths found for '{node}', only using first");
        }
    }
    result
}

pub fn default_file_name(root: &Path, node: &String) -> PathBuf {
    root.join(format!("{node}.{DEFAULT_EXT}"))
}
