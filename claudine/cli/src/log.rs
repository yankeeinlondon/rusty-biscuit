use std::io::Write;

/// Write a message to stderr (always visible, no verbose flag required).
pub fn message(msg: &str) {
    let _ = writeln!(std::io::stderr(), "{msg}");
}

/// Write an info message to stderr (only when verbose).
pub fn info(msg: &str, verbose: bool) {
    if verbose {
        let _ = writeln!(std::io::stderr(), "{msg}");
    }
}

/// Write data to stdout (for piping/machine consumption).
pub fn data(msg: &str) {
    let _ = writeln!(std::io::stdout(), "{msg}");
}

/// Write a warning to stderr in yellow.
pub fn warn(msg: &str) {
    let _ = writeln!(std::io::stderr(), "\x1b[33mwarning:\x1b[0m {msg}");
}

/// Write an error to stderr in red.
pub fn error(msg: &str) {
    let _ = writeln!(std::io::stderr(), "\x1b[31merror:\x1b[0m {msg}");
}
