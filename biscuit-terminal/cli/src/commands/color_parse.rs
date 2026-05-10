/// Extracts a hex color from the end of a string.
///
/// Looks for patterns like:
/// - `color: #3178c6` or `color:#3178c6`
/// - `#3178c6` (standalone at end)
///
/// Returns (remaining_string, Some(color)) if found, or (original, None) if not.
pub fn extract_color(s: &str) -> (&str, Option<String>) {
    let s = s.trim();

    // Try "color: #hex" or "color:#hex" pattern first
    if let Some(color_idx) = s.to_lowercase().rfind("color:") {
        let before = s[..color_idx].trim();
        let color_part = s[color_idx + 6..].trim(); // Skip "color:"

        if let Some(color) = parse_hex_color(color_part) {
            return (before, Some(color));
        }
    }

    // Try standalone #hex at the end
    // Find the last whitespace and check if what follows is a hex color
    if let Some(last_space) = s.rfind(char::is_whitespace) {
        let potential_color = s[last_space + 1..].trim();
        if let Some(color) = parse_hex_color(potential_color) {
            return (s[..last_space].trim(), Some(color));
        }
    }

    (s, None)
}

/// Parses a hex color string, returning it normalized if valid.
///
/// Accepts: `#rgb`, `#rrggbb`, `#rrggbbaa`
pub fn parse_hex_color(s: &str) -> Option<String> {
    let s = s.trim();
    if !s.starts_with('#') {
        return None;
    }

    let hex_part = &s[1..];
    // Valid lengths: 3 (#rgb), 6 (#rrggbb), or 8 (#rrggbbaa)
    if !matches!(hex_part.len(), 3 | 6 | 8) {
        return None;
    }

    // Check all characters are valid hex
    if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    Some(s.to_string())
}
