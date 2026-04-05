use crate::discovery::eval::strip_ansi_codes;

/// Produces a vector where each element represents a line's length
/// after all escape codes have been stripped.
pub fn content_length(content: &str) -> Vec<u32> {
    content
        .lines()
        .map(|line| strip_ansi_codes(line).len() as u32)
        .collect()
}
