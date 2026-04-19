use crate::discovery::eval::ANSI_ESCAPE_RE;
use std::sync::LazyLock;

use regex::Regex;

/// Strips **all** escape codes from the passed-in string.
///
/// Uses the canonical `ANSI_ESCAPE_RE` from `discovery::eval` which handles
/// CSI, OSC (with both BEL and ST terminators), and Fe escape sequences.
pub fn strip_escape_codes<T: Into<String>>(content: T) -> String {
    let content = content.into();
    ANSI_ESCAPE_RE.replace_all(&content, "").into_owned()
}

/// Strips all OSC8 hyperlinks from the passed-in text while retaining
/// other escape codes.
///
/// OSC8 links have the format: `\x1b]8;;<uri>\x07<link text>\x1b]8;;\x07`
/// Also handles ST terminator variant: `\x1b]8;;<uri>\x1b\\`
pub fn strip_osc8_links<T: Into<String>>(content: T) -> String {
    static OSC8_LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\x1b\]8;;[^\x07\x1b]*(?:\x07|\x1b\\)").expect("Invalid OSC8 link regex")
    });

    let content = content.into();
    OSC8_LINK_RE.replace_all(&content, "").into_owned()
}

/// Strip escape codes used for cursor movement while retaining other escape codes.
///
/// Cursor movement CSI sequences:
/// - `\x1b[<n>A` through `\x1b[<n>G` — directional movement
/// - `\x1b[<row>;<col>H` / `\x1b[<row>;<col>f` — absolute positioning
/// - `\x1b[s` / `\x1b[u` — save/restore cursor position
pub fn strip_cursor_movement_codes<T: Into<String>>(content: T) -> String {
    static CURSOR_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\x1b\[[0-9;]*[ABCDEFGHfsu]").expect("Invalid cursor movement regex")
    });

    let content = content.into();
    CURSOR_RE.replace_all(&content, "").into_owned()
}

/// Strip terminal query codes from a string while retaining other escape codes.
///
/// Query codes include Device Attributes (`c`) and Device Status Report (`n`).
pub fn strip_query_codes<T: Into<String>>(content: T) -> String {
    static QUERY_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\x1b\[[0-9;>]*[cn]").expect("Invalid query code regex"));

    let content = content.into();
    QUERY_RE.replace_all(&content, "").into_owned()
}

/// Strip color/SGR codes from a string while retaining other escape codes.
///
/// SGR sequences end with `m` (e.g., `\x1b[31m` for red, `\x1b[0m` for reset).
pub fn strip_color_codes<T: Into<String>>(content: T) -> String {
    static SGR_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\x1b\[[0-9;]*m").expect("Invalid SGR regex"));

    let content = content.into();
    SGR_RE.replace_all(&content, "").into_owned()
}
