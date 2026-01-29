/// Strips **all** escape codes out of the passed in string.
pub fn strip_escape_codes<T: Into<String>>(content: T) -> String {
    let content = content.into();
    // Strip CSI sequences (Control Sequence Introducer): \x1b[...[a-zA-Z]
    let csi_pattern = regex::Regex::new(r"\x1b\[.*?[a-zA-Z]").unwrap();
    let result = csi_pattern.replace_all(&content, "");
    // Strip OSC sequences (Operating System Command): \x1b]...\x07 or \x1b]...\x1b\\
    let osc_pattern = regex::Regex::new(r"\x1b\].*?(\x07|\x1b\\)").unwrap();
    let result = osc_pattern.replace_all(&result, "");
    // Strip other single-character escape sequences (e.g., \x1b7, \x1b8)
    let other_pattern = regex::Regex::new(r"\x1b[0-9]").unwrap();
    other_pattern.replace_all(&result, "").to_string()
}

/// Strips all OSC8 Links out of the passed in text while retaining
/// other escape codes.
///
/// OSC8 links have the format: \x1b]8;;<uri>\x07<link text>\x1b]8;;\x07
pub fn strip_osc8_links<T: Into<String>>(content: T) -> String {
    let content = content.into();
    // Match OSC8 link start: \x1b]8;;<uri>\x07
    let link_start_pattern = regex::Regex::new(r"\x1b\]8;;[^\x07]*\x07").unwrap();
    let result = link_start_pattern.replace_all(&content, "");
    // Match OSC8 link end: \x1b]8;;\x07
    let link_end_pattern = regex::Regex::new(r"\x1b\]8;;\x07").unwrap();
    link_end_pattern.replace_all(&result, "").to_string()
}

/// Strip escape codes used for cursor movement while retaining other escape codes
///
/// Cursor movement CSI sequences:
/// - \x1b[<n>A - Cursor up
/// - \x1b[<n>B - Cursor down
/// - \x1b[<n>C - Cursor forward
/// - \x1b[<n>D - Cursor backward
/// - \x1b[<n>E - Cursor next line
/// - \x1b[<n>F - Cursor previous line
/// - \x1b[<n>G - Cursor horizontal absolute
/// - \x1b[<row>;<col>H - Cursor position
/// - \x1b[<row>;<col>f - Horizontal and vertical position
/// - \x1b[s - Save cursor position
/// - \x1b[u - Restore cursor position
pub fn strip_cursor_movement_codes<T: Into<String>>(content: T) -> String {
    let content = content.into();
    // Match cursor movement CSI sequences ending with A, B, C, D, E, F, G, H, f, s, u
    let cursor_pattern = regex::Regex::new(r"\x1b\[([0-9;]*[ABCDEFGHfsu])").unwrap();
    cursor_pattern.replace_all(&content, "").to_string()
}

/// Strip terminal query codes from a string while retaining other escape codes.
///
/// Query codes include:
/// - \x1b[c - Device Attributes (DA1)
/// - \x1b[0c - Primary Device Attributes
/// - \x1b[>c - Secondary Device Attributes
/// - \x1b[n - Device Status Report (DSR)
/// - \x1b[5n - Device Status Report
/// - \x1b[6n - Report Cursor Position
/// - \x1b[c - Terminal Type query
pub fn strip_query_codes<T: Into<String>>(content: T) -> String {
    let content = content.into();
    // Match query CSI sequences ending with c or n
    let query_pattern = regex::Regex::new(r"\x1b\[([0-9;>]*[cn])").unwrap();
    query_pattern.replace_all(&content, "").to_string()
}

/// Strip color codes from a string while retaining other escape codes
///
/// Color codes are CSI SGR (Select Graphic Rendition) sequences:
/// - \x1b[<n>m - SGR parameters (e.g., \x1b[31m for red, \x1b[0m for reset)
pub fn strip_color_codes<T: Into<String>>(content: T) -> String {
    let content = content.into();
    // Match SGR sequences ending with 'm'
    let color_pattern = regex::Regex::new(r"\x1b\[([0-9;]*m)").unwrap();
    color_pattern.replace_all(&content, "").to_string()
}
