use clap::builder::styling::{AnsiColor, Styles};

pub const ROOT_DIR_NAME: &str = ".nous";
pub const DEFAULT_EXT: &str = "md";
pub const SUPPORTED_EXTS: [&str; 5] = ["md", "markdown", "org", "txt", "text"];
pub const FALLBACK_EDITOR: &str = if cfg!(windows) { "Notepad" } else { "vi" };
pub const FALLBACK_PAGER: &str = "more";

pub const CLI_STYLE: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default())
    .invalid(AnsiColor::Yellow.on_default().bold())
    .error(AnsiColor::Red.on_default().bold());
