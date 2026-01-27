/// **line_widths**`(content)`
///
/// returns the length of each line in the content
/// where the length is determined _after_ stripping out all zero-length
/// escape codes.
pub fn line_widths<T: Into<String>>(content: T) -> Vec<u16> {
    todo!()
}

pub fn has_escape_codes<T: Into<String>>(content: T) -> bool {
    todo!()
}

pub fn has_osc8_link<T: Into<String>>(content: T) -> bool {
    todo!()
}
