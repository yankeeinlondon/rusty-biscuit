use std::io::Write;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::terminal::Terminal;

/// Write a message to stderr (always visible, no verbose flag required).
pub fn message(msg: &str) {
    let _ = writeln!(std::io::stderr(), "{msg}");
}

/// Write an info message to stderr (only when INFO tracing level is enabled).
#[allow(dead_code)]
pub fn info(msg: &str) {
    if tracing::enabled!(tracing::Level::INFO) {
        let _ = writeln!(std::io::stderr(), "{msg}");
    }
}

/// Write data to stdout (for piping/machine consumption).
pub fn data(msg: &str) {
    let _ = writeln!(std::io::stdout(), "{msg}");
}

/// Write output to stdout without a trailing newline.
pub fn output(msg: &str) {
    let _ = write!(std::io::stdout(), "{msg}");
}

/// Write a warning to stderr in yellow, rendered through Prose.
pub fn warn(msg: &str) {
    let term = Terminal::new();
    let rendered =
        Prose::new(format!("<orange><bold>warning:</bold></orange> {msg}")).fallback_render(&term);
    let _ = writeln!(std::io::stderr(), "{rendered}");
}

/// Write an error to stderr in red, rendered through Prose.
///
/// Error messages may contain Prose tags (e.g. `<blue>--flag</blue>`) for
/// styled rendering of CLI switches and other highlights.
pub fn error(msg: &str) {
    let term = Terminal::new();
    let rendered =
        Prose::new(format!("<red><bold>Error:</bold></red> {msg}")).fallback_render(&term);
    let _ = writeln!(std::io::stderr(), "\n{rendered}");
}
