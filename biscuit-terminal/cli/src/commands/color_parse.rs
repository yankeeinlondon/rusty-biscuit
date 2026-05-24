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

/// Parses a CLI color flag into a render-tree [`Color`].
///
/// Accepts the eight named basic colors (`red`, `green`, `blue`, `cyan`,
/// `yellow`, `magenta`, `white`, `black`) and a `#rrggbb` hex triple. A hex
/// value lowers to [`Color::Rgb`] with a perceptually-near basic fallback so
/// it degrades on terminals without truecolor support.
///
/// ## Errors
///
/// Returns an error when `s` is neither a recognized name nor a valid
/// `#rrggbb` hex triple.
pub fn parse_color(s: &str) -> color_eyre::Result<renderable::color::Color> {
    use renderable::color::{BasicColor, Color, RgbColor};

    let trimmed = s.trim();
    let basic = match trimmed.to_ascii_lowercase().as_str() {
        "black" => Some(BasicColor::Black),
        "red" => Some(BasicColor::Red),
        "green" => Some(BasicColor::Green),
        "yellow" => Some(BasicColor::Yellow),
        "blue" => Some(BasicColor::Blue),
        "magenta" => Some(BasicColor::Magenta),
        "cyan" => Some(BasicColor::Cyan),
        "white" => Some(BasicColor::White),
        _ => None,
    };
    if let Some(basic) = basic {
        return Ok(Color::BasicColor(basic));
    }

    let hex = parse_hex_color(trimmed).ok_or_else(|| {
        color_eyre::eyre::eyre!(
            "invalid color {trimmed:?}: expected a named color \
             (red, green, blue, cyan, yellow, magenta, white, black) or a \
             #rrggbb hex value"
        )
    })?;
    let body = &hex[1..];
    if body.len() != 6 {
        return Err(color_eyre::eyre::eyre!(
            "invalid color {trimmed:?}: only #rrggbb hex values are accepted"
        ));
    }
    let r = u8::from_str_radix(&body[0..2], 16).expect("validated hex");
    let g = u8::from_str_radix(&body[2..4], 16).expect("validated hex");
    let b = u8::from_str_radix(&body[4..6], 16).expect("validated hex");
    Ok(Color::Rgb(RgbColor::new(r, g, b, nearest_basic(r, g, b))))
}

/// Picks the perceptually-nearest [`BasicColor`] for an RGB triple, used as
/// the 16-color fallback for a `#rrggbb` flag value.
fn nearest_basic(r: u8, g: u8, b: u8) -> renderable::color::BasicColor {
    use renderable::color::{BasicColor, basic_color_to_rgb};

    const PALETTE: [BasicColor; 8] = [
        BasicColor::Black,
        BasicColor::Red,
        BasicColor::Green,
        BasicColor::Yellow,
        BasicColor::Blue,
        BasicColor::Magenta,
        BasicColor::Cyan,
        BasicColor::White,
    ];
    PALETTE
        .into_iter()
        .min_by_key(|&basic| {
            let (pr, pg, pb) = basic_color_to_rgb(basic);
            let dr = i32::from(r) - i32::from(pr);
            let dg = i32::from(g) - i32::from(pg);
            let db = i32::from(b) - i32::from(pb);
            dr * dr + dg * dg + db * db
        })
        .unwrap_or(BasicColor::White)
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
