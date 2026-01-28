/// Strips **all** escape codes out of the passed in string.
pub fn strip_escape_codes<T: Into<String>>(_content: T) -> String {
    todo!()
}

/// Strips all OSC8 Links out of the passed in text while retaining
/// other escape codes.
pub fn strip_osc8_links<T: Into<String>>(_content: T) -> String {
    todo!()
}

/// Strip escape codes used for cursor movement while retaining other escape codes
pub fn strip_cursor_movement_codes<T: Into<String>>(_content: T) -> String {
    todo!()
}

/// Strip terminal query codes from a string while retaining other escape codes.
/// Query codes include:
///
/// -
pub fn strip_query_codes<T: Into<String>>(_content: T) -> String {
    todo!()
}

/// Strip terminal query codes from a string while retaining other escape codes
pub fn strip_color_codes<T: Into<String>>(_content: T) -> String {
    todo!()
}
