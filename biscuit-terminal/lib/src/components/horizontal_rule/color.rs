use crate::utils::color::{BasicColor, RgbColor};

/// Parses a CSS basic-16 color name (case-insensitive) into a [`BasicColor`].
/// Also accepts `gray` / `grey` as aliases for `BrightBlack`.
///
/// ## Notes
///
/// Matches without allocating — `eq_ignore_ascii_case` compares in place, so a
/// rule rendered on every paragraph break in a large document no longer pays
/// a per-call `String` allocation for the comparison.
pub(super) fn parse_basic_color(raw: &str) -> Option<BasicColor> {
    // Static table keeps the allocation amortized across matches and makes
    // extending the list a one-line change. Hyphenated and concatenated
    // bright variants are kept alongside the canonical names for
    // compatibility with existing callers.
    const PAIRS: &[(&str, BasicColor)] = &[
        ("black", BasicColor::Black),
        ("red", BasicColor::Red),
        ("green", BasicColor::Green),
        ("yellow", BasicColor::Yellow),
        ("blue", BasicColor::Blue),
        ("magenta", BasicColor::Magenta),
        ("cyan", BasicColor::Cyan),
        ("white", BasicColor::White),
        ("gray", BasicColor::BrightBlack),
        ("grey", BasicColor::BrightBlack),
        ("bright-black", BasicColor::BrightBlack),
        ("brightblack", BasicColor::BrightBlack),
        ("bright-red", BasicColor::BrightRed),
        ("brightred", BasicColor::BrightRed),
        ("bright-green", BasicColor::BrightGreen),
        ("brightgreen", BasicColor::BrightGreen),
        ("bright-yellow", BasicColor::BrightYellow),
        ("brightyellow", BasicColor::BrightYellow),
        ("bright-blue", BasicColor::BrightBlue),
        ("brightblue", BasicColor::BrightBlue),
        ("bright-magenta", BasicColor::BrightMagenta),
        ("brightmagenta", BasicColor::BrightMagenta),
        ("bright-cyan", BasicColor::BrightCyan),
        ("brightcyan", BasicColor::BrightCyan),
        ("bright-white", BasicColor::BrightWhite),
        ("brightwhite", BasicColor::BrightWhite),
    ];
    PAIRS
        .iter()
        .find(|(name, _)| raw.eq_ignore_ascii_case(name))
        .map(|(_, color)| *color)
}

/// Parses a `#rrggbb` hex color into an [`RgbColor`] with a nearest-primary
/// [`BasicColor`] fallback for non-truecolor terminals. Returns `None` if the
/// string does not match `#` + 6 hex digits.
pub(super) fn parse_hex_color(raw: &str) -> Option<RgbColor> {
    let hex = raw.strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    let fallback = nearest_basic_color(r, g, b);
    Some(RgbColor::new(r, g, b, fallback))
}

/// Maps an RGB triple to its nearest basic ANSI foreground color. This is a
/// coarse nearest-primary heuristic — sufficient for horizontal rule
/// fallbacks where fidelity is secondary to "picks a reasonable color".
pub(super) fn nearest_basic_color(r: u8, g: u8, b: u8) -> BasicColor {
    let max = r.max(g).max(b);
    if max < 64 {
        return BasicColor::Black;
    }
    let bright = max >= 192;
    let r_on = r >= max.saturating_sub(64);
    let g_on = g >= max.saturating_sub(64);
    let b_on = b >= max.saturating_sub(64);
    match (r_on, g_on, b_on, bright) {
        (true, true, true, true) => BasicColor::BrightWhite,
        (true, true, true, false) => BasicColor::White,
        (true, false, false, true) => BasicColor::BrightRed,
        (true, false, false, false) => BasicColor::Red,
        (false, true, false, true) => BasicColor::BrightGreen,
        (false, true, false, false) => BasicColor::Green,
        (false, false, true, true) => BasicColor::BrightBlue,
        (false, false, true, false) => BasicColor::Blue,
        (true, true, false, true) => BasicColor::BrightYellow,
        (true, true, false, false) => BasicColor::Yellow,
        (true, false, true, true) => BasicColor::BrightMagenta,
        (true, false, true, false) => BasicColor::Magenta,
        (false, true, true, true) => BasicColor::BrightCyan,
        (false, true, true, false) => BasicColor::Cyan,
        (false, false, false, _) => BasicColor::Black,
    }
}
