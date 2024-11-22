use clap::{
    builder::styling::{AnsiColor, Styles},
    Parser, Subcommand,
};
use pathdiff::diff_paths;
use std::path::{Path, PathBuf};
use std::process;
use std::time::{Duration, Instant, SystemTime};
use std::{env, fs};
use walkdir::{DirEntry, WalkDir};

const ROOT_DIR_NAME: &str = ".nous";
const DEFAULT_EXT: &str = "md";
const SUPPORTED_EXTS: [&str; 5] = ["md", "markdown", "org", "txt", "text"];
const FALLBACK_EDITOR: &str = if cfg!(windows) { "Notepad" } else { "vi" };

const CLI_STYLE: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default())
    .invalid(AnsiColor::Yellow.on_default().bold())
    .error(AnsiColor::Red.on_default().bold());

#[derive(Parser)]
#[command(name = "nous")]
#[command(author, version, about, long_about = None)]
#[command(styles = CLI_STYLE)]
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

    /// Edit a node using the default editor
    Edit {
        /// Node to edit
        node: String,
        /// Editor to use
        #[arg(short, long)]
        editor: Option<String>,
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
    Ls {
        /// Print the path
        #[arg(short, long)]
        path: bool,
        /// Print the absolute path
        #[arg(short, long)]
        absolute: bool,
    },
}

macro_rules! error {
    ($($arg:tt)*) => (_error(format!($($arg)*)));
}

fn _error(message: String) -> ! {
    let style = CLI_STYLE.get_error();
    eprintln!("{style}error:{style:#} {message}");
    process::exit(1);
}

macro_rules! warn {
    ($($arg:tt)*) => (_warn(format!($($arg)*)));
}

fn _warn(message: String) {
    let style = CLI_STYLE.get_invalid();
    eprintln!("{style}warning:{style:#} {message}");
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
            Commands::Edit { node, editor } => edit_node(&root, &node, editor.into()),
            Commands::Touch { node } => touch_node(&root, &node),
            Commands::Path { node, absolute } => path_to_node(&root, &node, *absolute),
            Commands::Ls { path, absolute } => list_nodes(&root, *path, *absolute),
            Commands::Root { absolute } => println_path(&root, *absolute),
            Commands::Init { root: _ } => unreachable!(),
        }
    } else {
        error!("not within a νοῦς realm; you could use 'init' to create one")
    }
}

fn current_dir() -> PathBuf {
    env::current_dir().unwrap_or_else(|_| error!("failed to retrieve working directory"))
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

fn println_path(path: &Path, absolute: bool) {
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

fn list_nodes(root: &Path, path: bool, absolute: bool) {
    for p in realm_walker(root) {
        if path || absolute {
            println_path(&p, absolute)
        } else {
            if let Some(osf) = p.file_stem() {
                if let Some(f) = osf.to_str() {
                    println!("{f}");
                } else {
                    warn!("failed to interpret name of a node as Unicode")
                }
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

fn find_node_once(root: &Path, node: &String, strict: bool) -> Option<PathBuf> {
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

fn path_to_node(root: &Path, node: &String, absolute: bool) {
    match find_node_once(root, node, false) {
        Some(path) => println_path(&path, absolute),
        None => warn!("node not found"),
    }
}

fn default_file_name(root: &Path, node: &String) -> PathBuf {
    root.join(format!("{node}.{DEFAULT_EXT}"))
}

fn touch_node(root: &Path, node: &String) {
    let path = find_node_once(root, node, false).unwrap_or(default_file_name(root, node));
    if path.file_name().is_some() && path.parent().map_or(true, |p| p.is_dir()) {
        let file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&path)
            .unwrap_or_else(|_| {
                error!(
                    "failed to touch file '{}'",
                    try_relative_path(&path).display()
                )
            });
        let _ = file.set_modified(SystemTime::now()); // not a problem if this fails
    } else {
        warn!("node name results in an invalid file, skipping")
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
        Ok(code) => {
            if !code.success() {
                error!("editor did not exit successfully, consider using the --editor flag")
            }
        }
        Err(_) => {
            error!("failed to launch '{editor}', consider using the --editor flag")
        }
    }

    if start_time.elapsed() <= Duration::from_millis(100) {
        warn!("editor exited under 100ms, this might indicate failure; consider using the --editor flag")
    }
}

fn remove_node(root: &Path, node: &String) {
    if let Some(path) = find_node_once(root, node, true) {
        fs::remove_file(&path).unwrap_or_else(|_| {
            error!(
                "failed to remove node at '{}'",
                try_relative_path(&path).display()
            )
        });
    } else {
        warn!("node does not exist, skipping removal");
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
