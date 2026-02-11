/// produces a vector where each element in in the vector
/// represents a line in the content, and the value represents
/// the length of the line after all escape codes have been
/// removed.
pub fn content_length(content: &str) -> Vec<u32> {
    content
        .lines()
        .map(|line| {
            // Strip ANSI escape codes (e.g., \x1b[31m, \x1b[0m)
            // This pattern matches CSI sequences: ESC followed by [ and any characters until a letter
            let stripped = regex::Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]")
                .unwrap()
                .replace_all(line, "");
            // Also strip OSC sequences (e.g., \x1b]0;title\x07)
            let stripped = regex::Regex::new(r"\x1b\].*?\x07")
                .unwrap()
                .replace_all(&stripped, "");
            stripped.len() as u32
        })
        .collect()
}
