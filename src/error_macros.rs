use crate::config::CLI_STYLE;
use std::process;

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ($crate::error_macros::_error(format!($($arg)*)));
}

pub fn _error(message: String) -> ! {
    let style = CLI_STYLE.get_error();
    eprintln!("{style}error:{style:#} {message}");
    process::exit(1);
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ($crate::error_macros::_warn(format!($($arg)*)));
}

pub fn _warn(message: String) {
    let style = CLI_STYLE.get_invalid();
    eprintln!("{style}warning:{style:#} {message}");
}
