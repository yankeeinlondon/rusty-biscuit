//! Color deserializer for the style frontmatter.
//!
//! Lowers Tailwind names (`red-500`, `red-500/50`), hex (`#rrggbb`,
//! `#rrggbbaa`), and CSS web names (`orange`) into
//! [`renderable::color::Color`]. Opacity is preserved separately because
//! the underlying enum does not carry it.

use renderable::color::{Color, Tailwind, WEB_COLOR_LOOKUP};
use serde::de::{self, Deserializer};
use serde::Deserialize;

/// A frontmatter color value.
///
/// Wraps `renderable::color::Color` (which does not carry opacity) with an
/// optional Tailwind-style opacity (`/50` → `Some(50)`), in `0..=100`.
/// Opacity is documented as HTML-only by `docs/rendering/style.md`; terminal
/// targets drop it.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleColor {
    pub color: Color,
    pub opacity: Option<u8>,
}

/// Parse a color string.
pub fn parse(raw: &str) -> Result<StyleColor, &'static str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("empty color");
    }

    // Hex (handled in Task 8 — stub for now).
    if trimmed.starts_with('#') {
        return parse_hex(trimmed);
    }

    // Tailwind (`family-level` or `family-level/opacity`).
    if let Some(tw) = parse_tailwind(trimmed)? {
        return Ok(tw);
    }

    // Web named color (Task 8).
    if let Some(web) = parse_web_named(trimmed) {
        return Ok(web);
    }

    Err("unrecognized color; expected Tailwind name, hex, or web color name")
}

/// Parse a Tailwind palette string like `red-500` or `red-500/50`.
///
/// Returns `Ok(None)` if the input doesn't look like a Tailwind reference,
/// `Ok(Some(_))` if it parses, `Err` if it looks like a Tailwind reference
/// but is malformed.
fn parse_tailwind(raw: &str) -> Result<Option<StyleColor>, &'static str> {
    // Split off opacity.
    let (color_part, opacity) = match raw.split_once('/') {
        Some((c, op)) => {
            let n: u8 = op
                .parse()
                .map_err(|_| "malformed opacity (must be integer 0..=100)")?;
            if n > 100 {
                return Err("opacity must be in 0..=100");
            }
            (c, Some(n))
        }
        None => (raw, None),
    };

    // Specials with no level.
    if let Some(special) = parse_tailwind_special(color_part) {
        return Ok(Some(StyleColor { color: Color::Tailwind(special), opacity }));
    }

    // Split family from level (last hyphen).
    let Some((family, level)) = color_part.rsplit_once('-') else {
        return Ok(None);
    };

    // Level must be a canonical step. Anything that looks numeric but
    // isn't canonical (e.g. "501") is a real error.
    if !is_canonical_level(level) {
        if !level.is_empty() && level.chars().all(|c| c.is_ascii_digit()) {
            return Err(
                "Tailwind level must be one of: 50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950",
            );
        }
        return Ok(None);
    }

    let Some(variant) = lookup_tailwind(family, level) else {
        return Ok(None);
    };

    Ok(Some(StyleColor { color: Color::Tailwind(variant), opacity }))
}

fn is_canonical_level(level: &str) -> bool {
    matches!(
        level,
        "50" | "100" | "200" | "300" | "400" | "500"
            | "600" | "700" | "800" | "900" | "950"
    )
}

fn parse_tailwind_special(name: &str) -> Option<Tailwind> {
    match name {
        "transparent" => Some(Tailwind::Transparent),
        "current" => Some(Tailwind::Current),
        "inherit" => Some(Tailwind::Inherit),
        "black" => Some(Tailwind::Black),
        "white" => Some(Tailwind::White),
        _ => None,
    }
}

/// Explicit `(family, level)` → `Tailwind` table. 22 families × 11 levels = 242 arms.
fn lookup_tailwind(family: &str, level: &str) -> Option<Tailwind> {
    Some(match (family, level) {
        // ── Red ─────────────────────────────────────────────────────────
        ("red", "50") => Tailwind::Red50,
        ("red", "100") => Tailwind::Red100,
        ("red", "200") => Tailwind::Red200,
        ("red", "300") => Tailwind::Red300,
        ("red", "400") => Tailwind::Red400,
        ("red", "500") => Tailwind::Red500,
        ("red", "600") => Tailwind::Red600,
        ("red", "700") => Tailwind::Red700,
        ("red", "800") => Tailwind::Red800,
        ("red", "900") => Tailwind::Red900,
        ("red", "950") => Tailwind::Red950,
        // ── Orange ──────────────────────────────────────────────────────
        ("orange", "50") => Tailwind::Orange50,
        ("orange", "100") => Tailwind::Orange100,
        ("orange", "200") => Tailwind::Orange200,
        ("orange", "300") => Tailwind::Orange300,
        ("orange", "400") => Tailwind::Orange400,
        ("orange", "500") => Tailwind::Orange500,
        ("orange", "600") => Tailwind::Orange600,
        ("orange", "700") => Tailwind::Orange700,
        ("orange", "800") => Tailwind::Orange800,
        ("orange", "900") => Tailwind::Orange900,
        ("orange", "950") => Tailwind::Orange950,
        // ── Amber ───────────────────────────────────────────────────────
        ("amber", "50") => Tailwind::Amber50,
        ("amber", "100") => Tailwind::Amber100,
        ("amber", "200") => Tailwind::Amber200,
        ("amber", "300") => Tailwind::Amber300,
        ("amber", "400") => Tailwind::Amber400,
        ("amber", "500") => Tailwind::Amber500,
        ("amber", "600") => Tailwind::Amber600,
        ("amber", "700") => Tailwind::Amber700,
        ("amber", "800") => Tailwind::Amber800,
        ("amber", "900") => Tailwind::Amber900,
        ("amber", "950") => Tailwind::Amber950,
        // ── Yellow ──────────────────────────────────────────────────────
        ("yellow", "50") => Tailwind::Yellow50,
        ("yellow", "100") => Tailwind::Yellow100,
        ("yellow", "200") => Tailwind::Yellow200,
        ("yellow", "300") => Tailwind::Yellow300,
        ("yellow", "400") => Tailwind::Yellow400,
        ("yellow", "500") => Tailwind::Yellow500,
        ("yellow", "600") => Tailwind::Yellow600,
        ("yellow", "700") => Tailwind::Yellow700,
        ("yellow", "800") => Tailwind::Yellow800,
        ("yellow", "900") => Tailwind::Yellow900,
        ("yellow", "950") => Tailwind::Yellow950,
        // ── Lime ────────────────────────────────────────────────────────
        ("lime", "50") => Tailwind::Lime50,
        ("lime", "100") => Tailwind::Lime100,
        ("lime", "200") => Tailwind::Lime200,
        ("lime", "300") => Tailwind::Lime300,
        ("lime", "400") => Tailwind::Lime400,
        ("lime", "500") => Tailwind::Lime500,
        ("lime", "600") => Tailwind::Lime600,
        ("lime", "700") => Tailwind::Lime700,
        ("lime", "800") => Tailwind::Lime800,
        ("lime", "900") => Tailwind::Lime900,
        ("lime", "950") => Tailwind::Lime950,
        // ── Green ───────────────────────────────────────────────────────
        ("green", "50") => Tailwind::Green50,
        ("green", "100") => Tailwind::Green100,
        ("green", "200") => Tailwind::Green200,
        ("green", "300") => Tailwind::Green300,
        ("green", "400") => Tailwind::Green400,
        ("green", "500") => Tailwind::Green500,
        ("green", "600") => Tailwind::Green600,
        ("green", "700") => Tailwind::Green700,
        ("green", "800") => Tailwind::Green800,
        ("green", "900") => Tailwind::Green900,
        ("green", "950") => Tailwind::Green950,
        // ── Emerald ─────────────────────────────────────────────────────
        ("emerald", "50") => Tailwind::Emerald50,
        ("emerald", "100") => Tailwind::Emerald100,
        ("emerald", "200") => Tailwind::Emerald200,
        ("emerald", "300") => Tailwind::Emerald300,
        ("emerald", "400") => Tailwind::Emerald400,
        ("emerald", "500") => Tailwind::Emerald500,
        ("emerald", "600") => Tailwind::Emerald600,
        ("emerald", "700") => Tailwind::Emerald700,
        ("emerald", "800") => Tailwind::Emerald800,
        ("emerald", "900") => Tailwind::Emerald900,
        ("emerald", "950") => Tailwind::Emerald950,
        // ── Teal ────────────────────────────────────────────────────────
        ("teal", "50") => Tailwind::Teal50,
        ("teal", "100") => Tailwind::Teal100,
        ("teal", "200") => Tailwind::Teal200,
        ("teal", "300") => Tailwind::Teal300,
        ("teal", "400") => Tailwind::Teal400,
        ("teal", "500") => Tailwind::Teal500,
        ("teal", "600") => Tailwind::Teal600,
        ("teal", "700") => Tailwind::Teal700,
        ("teal", "800") => Tailwind::Teal800,
        ("teal", "900") => Tailwind::Teal900,
        ("teal", "950") => Tailwind::Teal950,
        // ── Cyan ────────────────────────────────────────────────────────
        ("cyan", "50") => Tailwind::Cyan50,
        ("cyan", "100") => Tailwind::Cyan100,
        ("cyan", "200") => Tailwind::Cyan200,
        ("cyan", "300") => Tailwind::Cyan300,
        ("cyan", "400") => Tailwind::Cyan400,
        ("cyan", "500") => Tailwind::Cyan500,
        ("cyan", "600") => Tailwind::Cyan600,
        ("cyan", "700") => Tailwind::Cyan700,
        ("cyan", "800") => Tailwind::Cyan800,
        ("cyan", "900") => Tailwind::Cyan900,
        ("cyan", "950") => Tailwind::Cyan950,
        // ── Sky ─────────────────────────────────────────────────────────
        ("sky", "50") => Tailwind::Sky50,
        ("sky", "100") => Tailwind::Sky100,
        ("sky", "200") => Tailwind::Sky200,
        ("sky", "300") => Tailwind::Sky300,
        ("sky", "400") => Tailwind::Sky400,
        ("sky", "500") => Tailwind::Sky500,
        ("sky", "600") => Tailwind::Sky600,
        ("sky", "700") => Tailwind::Sky700,
        ("sky", "800") => Tailwind::Sky800,
        ("sky", "900") => Tailwind::Sky900,
        ("sky", "950") => Tailwind::Sky950,
        // ── Blue ────────────────────────────────────────────────────────
        ("blue", "50") => Tailwind::Blue50,
        ("blue", "100") => Tailwind::Blue100,
        ("blue", "200") => Tailwind::Blue200,
        ("blue", "300") => Tailwind::Blue300,
        ("blue", "400") => Tailwind::Blue400,
        ("blue", "500") => Tailwind::Blue500,
        ("blue", "600") => Tailwind::Blue600,
        ("blue", "700") => Tailwind::Blue700,
        ("blue", "800") => Tailwind::Blue800,
        ("blue", "900") => Tailwind::Blue900,
        ("blue", "950") => Tailwind::Blue950,
        // ── Indigo ──────────────────────────────────────────────────────
        ("indigo", "50") => Tailwind::Indigo50,
        ("indigo", "100") => Tailwind::Indigo100,
        ("indigo", "200") => Tailwind::Indigo200,
        ("indigo", "300") => Tailwind::Indigo300,
        ("indigo", "400") => Tailwind::Indigo400,
        ("indigo", "500") => Tailwind::Indigo500,
        ("indigo", "600") => Tailwind::Indigo600,
        ("indigo", "700") => Tailwind::Indigo700,
        ("indigo", "800") => Tailwind::Indigo800,
        ("indigo", "900") => Tailwind::Indigo900,
        ("indigo", "950") => Tailwind::Indigo950,
        // ── Violet ──────────────────────────────────────────────────────
        ("violet", "50") => Tailwind::Violet50,
        ("violet", "100") => Tailwind::Violet100,
        ("violet", "200") => Tailwind::Violet200,
        ("violet", "300") => Tailwind::Violet300,
        ("violet", "400") => Tailwind::Violet400,
        ("violet", "500") => Tailwind::Violet500,
        ("violet", "600") => Tailwind::Violet600,
        ("violet", "700") => Tailwind::Violet700,
        ("violet", "800") => Tailwind::Violet800,
        ("violet", "900") => Tailwind::Violet900,
        ("violet", "950") => Tailwind::Violet950,
        // ── Purple ──────────────────────────────────────────────────────
        ("purple", "50") => Tailwind::Purple50,
        ("purple", "100") => Tailwind::Purple100,
        ("purple", "200") => Tailwind::Purple200,
        ("purple", "300") => Tailwind::Purple300,
        ("purple", "400") => Tailwind::Purple400,
        ("purple", "500") => Tailwind::Purple500,
        ("purple", "600") => Tailwind::Purple600,
        ("purple", "700") => Tailwind::Purple700,
        ("purple", "800") => Tailwind::Purple800,
        ("purple", "900") => Tailwind::Purple900,
        ("purple", "950") => Tailwind::Purple950,
        // ── Fuchsia ─────────────────────────────────────────────────────
        ("fuchsia", "50") => Tailwind::Fuchsia50,
        ("fuchsia", "100") => Tailwind::Fuchsia100,
        ("fuchsia", "200") => Tailwind::Fuchsia200,
        ("fuchsia", "300") => Tailwind::Fuchsia300,
        ("fuchsia", "400") => Tailwind::Fuchsia400,
        ("fuchsia", "500") => Tailwind::Fuchsia500,
        ("fuchsia", "600") => Tailwind::Fuchsia600,
        ("fuchsia", "700") => Tailwind::Fuchsia700,
        ("fuchsia", "800") => Tailwind::Fuchsia800,
        ("fuchsia", "900") => Tailwind::Fuchsia900,
        ("fuchsia", "950") => Tailwind::Fuchsia950,
        // ── Pink ────────────────────────────────────────────────────────
        ("pink", "50") => Tailwind::Pink50,
        ("pink", "100") => Tailwind::Pink100,
        ("pink", "200") => Tailwind::Pink200,
        ("pink", "300") => Tailwind::Pink300,
        ("pink", "400") => Tailwind::Pink400,
        ("pink", "500") => Tailwind::Pink500,
        ("pink", "600") => Tailwind::Pink600,
        ("pink", "700") => Tailwind::Pink700,
        ("pink", "800") => Tailwind::Pink800,
        ("pink", "900") => Tailwind::Pink900,
        ("pink", "950") => Tailwind::Pink950,
        // ── Rose ────────────────────────────────────────────────────────
        ("rose", "50") => Tailwind::Rose50,
        ("rose", "100") => Tailwind::Rose100,
        ("rose", "200") => Tailwind::Rose200,
        ("rose", "300") => Tailwind::Rose300,
        ("rose", "400") => Tailwind::Rose400,
        ("rose", "500") => Tailwind::Rose500,
        ("rose", "600") => Tailwind::Rose600,
        ("rose", "700") => Tailwind::Rose700,
        ("rose", "800") => Tailwind::Rose800,
        ("rose", "900") => Tailwind::Rose900,
        ("rose", "950") => Tailwind::Rose950,
        // ── Slate ───────────────────────────────────────────────────────
        ("slate", "50") => Tailwind::Slate50,
        ("slate", "100") => Tailwind::Slate100,
        ("slate", "200") => Tailwind::Slate200,
        ("slate", "300") => Tailwind::Slate300,
        ("slate", "400") => Tailwind::Slate400,
        ("slate", "500") => Tailwind::Slate500,
        ("slate", "600") => Tailwind::Slate600,
        ("slate", "700") => Tailwind::Slate700,
        ("slate", "800") => Tailwind::Slate800,
        ("slate", "900") => Tailwind::Slate900,
        ("slate", "950") => Tailwind::Slate950,
        // ── Gray ────────────────────────────────────────────────────────
        ("gray", "50") => Tailwind::Gray50,
        ("gray", "100") => Tailwind::Gray100,
        ("gray", "200") => Tailwind::Gray200,
        ("gray", "300") => Tailwind::Gray300,
        ("gray", "400") => Tailwind::Gray400,
        ("gray", "500") => Tailwind::Gray500,
        ("gray", "600") => Tailwind::Gray600,
        ("gray", "700") => Tailwind::Gray700,
        ("gray", "800") => Tailwind::Gray800,
        ("gray", "900") => Tailwind::Gray900,
        ("gray", "950") => Tailwind::Gray950,
        // ── Zinc ────────────────────────────────────────────────────────
        ("zinc", "50") => Tailwind::Zinc50,
        ("zinc", "100") => Tailwind::Zinc100,
        ("zinc", "200") => Tailwind::Zinc200,
        ("zinc", "300") => Tailwind::Zinc300,
        ("zinc", "400") => Tailwind::Zinc400,
        ("zinc", "500") => Tailwind::Zinc500,
        ("zinc", "600") => Tailwind::Zinc600,
        ("zinc", "700") => Tailwind::Zinc700,
        ("zinc", "800") => Tailwind::Zinc800,
        ("zinc", "900") => Tailwind::Zinc900,
        ("zinc", "950") => Tailwind::Zinc950,
        // ── Neutral ─────────────────────────────────────────────────────
        ("neutral", "50") => Tailwind::Neutral50,
        ("neutral", "100") => Tailwind::Neutral100,
        ("neutral", "200") => Tailwind::Neutral200,
        ("neutral", "300") => Tailwind::Neutral300,
        ("neutral", "400") => Tailwind::Neutral400,
        ("neutral", "500") => Tailwind::Neutral500,
        ("neutral", "600") => Tailwind::Neutral600,
        ("neutral", "700") => Tailwind::Neutral700,
        ("neutral", "800") => Tailwind::Neutral800,
        ("neutral", "900") => Tailwind::Neutral900,
        ("neutral", "950") => Tailwind::Neutral950,
        // ── Stone ───────────────────────────────────────────────────────
        ("stone", "50") => Tailwind::Stone50,
        ("stone", "100") => Tailwind::Stone100,
        ("stone", "200") => Tailwind::Stone200,
        ("stone", "300") => Tailwind::Stone300,
        ("stone", "400") => Tailwind::Stone400,
        ("stone", "500") => Tailwind::Stone500,
        ("stone", "600") => Tailwind::Stone600,
        ("stone", "700") => Tailwind::Stone700,
        ("stone", "800") => Tailwind::Stone800,
        ("stone", "900") => Tailwind::Stone900,
        ("stone", "950") => Tailwind::Stone950,
        _ => return None,
    })
}

/// Parse a CSS hex color: `#rgb`, `#rrggbb`, or `#rrggbbaa`.
fn parse_hex(raw: &str) -> Result<StyleColor, &'static str> {
    use renderable::color::RgbColor;

    let hex = raw.trim_start_matches('#');
    if hex.is_empty() || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("non-hex digit");
    }

    let (r, g, b, a) = match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).map_err(|_| "non-hex digit")?;
            let g = u8::from_str_radix(&hex[1..2], 16).map_err(|_| "non-hex digit")?;
            let b = u8::from_str_radix(&hex[2..3], 16).map_err(|_| "non-hex digit")?;
            (r * 17, g * 17, b * 17, None) // expand nibble (0xF → 0xFF)
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "non-hex digit")?;
            let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "non-hex digit")?;
            let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "non-hex digit")?;
            (r, g, b, None)
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "non-hex digit")?;
            let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "non-hex digit")?;
            let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "non-hex digit")?;
            let a_byte = u8::from_str_radix(&hex[6..8], 16).map_err(|_| "non-hex digit")?;
            // Alpha 0..=255 → opacity 0..=100.
            let opacity = ((a_byte as u32) * 100 / 255) as u8;
            (r, g, b, Some(opacity))
        }
        _ => {
            return Err("hex color must have 3, 6, or 8 digits after `#`");
        }
    };

    let fallback = dominant_basic_color(r, g, b);
    Ok(StyleColor {
        color: Color::Rgb(RgbColor::new(r, g, b, fallback)),
        opacity: a,
    })
}

/// Pick a `BasicColor` fallback for hex inputs. Used only when ANSI palette
/// is too narrow for true-color output. Simple dominant-channel heuristic.
fn dominant_basic_color(r: u8, g: u8, b: u8) -> renderable::color::BasicColor {
    use renderable::color::BasicColor::*;
    match (r, g, b) {
        (r, g, b) if r >= 200 && g >= 200 && b >= 200 => White,
        (r, g, b) if r < 50 && g < 50 && b < 50 => Black,
        (r, g, b) if r > 150 && g > 150 && b < 100 => Yellow,
        (r, g, b) if r > 150 && b > 150 && g < 100 => Magenta,
        (r, g, b) if g > 150 && b > 150 && r < 100 => Cyan,
        (r, g, b) if r > g && r > b => Red,
        (r, g, b) if g > r && g > b => Green,
        (r, g, b) if b > r && b > g => Blue,
        _ => White,
    }
}

/// Parse a CSS web-named color (`"orange"`, `"rebeccapurple"`).
///
/// Performs a case-insensitive match against the 148 CSS named colors by
/// serializing each `WebColor` variant (PascalCase) and comparing it
/// case-insensitively to the input. CSS color names are case-insensitive
/// and multi-word names are written as one word (`midnightblue`).
fn parse_web_named(raw: &str) -> Option<StyleColor> {
    let lower = raw.to_ascii_lowercase();
    // Serde serializes WebColor variants as PascalCase (default behavior).
    // Lower-casing both sides gives us a case-insensitive match that works
    // for single-word names ("orange" → "Orange") and multi-word names
    // ("midnightblue" → "MidnightBlue" → lowercased "midnightblue").
    let web = WEB_COLOR_LOOKUP
        .keys()
        .find(|&&variant| {
            serde_json::to_string(&variant)
                .ok()
                .map(|s| s.trim_matches('"').to_ascii_lowercase() == lower)
                .unwrap_or(false)
        })
        .copied()?;
    Some(StyleColor {
        color: Color::Web(web),
        opacity: None,
    })
}

pub fn deserialize_optional_color<'de, D>(de: D) -> Result<Option<StyleColor>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: Option<String> = Option::deserialize(de)?;
    match raw {
        None => Ok(None),
        Some(s) => parse(&s).map(Some).map_err(de::Error::custom),
    }
}

// ---------------------------------------------------------------------------
// Target-specific lowering helpers (Phase 2)
// ---------------------------------------------------------------------------

use crate::markdown::output::terminal::ColorDepth;

/// Lower a [`StyleColor`] to a CSS color value string.
///
/// ## Returns
///
/// - RGB-capable colors produce `rgb(r, g, b)` or `rgba(r, g, b, a)` when
///   opacity is present.
/// - Tailwind special values map to CSS keywords:
///   - `transparent` → `"transparent"`
///   - `current` → `"currentColor"`
///   - `inherit` → `"inherit"`
/// - Non-RGB values (`DefaultForeground`, `DefaultBackground`, `Reset`)
///   return `None`.
pub fn lower_to_css(style_color: &StyleColor) -> Option<String> {
    match &style_color.color {
        Color::Tailwind(Tailwind::Transparent) => Some("transparent".to_string()),
        Color::Tailwind(Tailwind::Current) => Some("currentColor".to_string()),
        Color::Tailwind(Tailwind::Inherit) => Some("inherit".to_string()),
        Color::DefaultForeground | Color::DefaultBackground | Color::Reset => None,
        _ => {
            let (r, g, b) = style_color.color.to_rgb()?;
            match style_color.opacity {
                None => Some(format!("rgb({r}, {g}, {b})")),
                Some(op) => {
                    let alpha = f32::from(op) / 100.0;
                    Some(format!("rgba({r}, {g}, {b}, {alpha})"))
                }
            }
        }
    }
}

/// Lower a [`StyleColor`] to an SGR escape sequence.
///
/// ## Returns
///
/// - `ColorDepth::None` → `None`.
/// - RGB-capable colors emit `\x1b[38;2;r;g;b`m` for foreground or
///   `\x1b[48;2;r;g;b`m` for background.
/// - Non-RGB values (`DefaultForeground`, `DefaultBackground`, `Reset`,
///   Tailwind specials) return `None`.
pub fn lower_to_sgr(style_color: &StyleColor, color_depth: ColorDepth, is_background: bool) -> Option<String> {
    if color_depth == ColorDepth::None {
        return None;
    }
    let (r, g, b) = style_color.color.to_rgb()?;
    let lead = if is_background { 48 } else { 38 };
    Some(format!("\x1b[{lead};2;{r};{g};{b}m"))
}

/// Wrap `content` with SGR sequences when foreground or background colors
/// are present, guaranteeing a reset (`\x1b[0m`) when any SGR is opened.
///
/// Returns `content` unchanged when both colors are `None` or when
/// `color_depth` is `ColorDepth::None`.
pub fn wrap_with_color(
    content: &str,
    fg: Option<&StyleColor>,
    bg: Option<&StyleColor>,
    color_depth: ColorDepth,
) -> String {
    let mut open = String::new();
    if let Some(fg) = fg
        && let Some(seq) = lower_to_sgr(fg, color_depth, false)
    {
        open.push_str(&seq);
    }
    if let Some(bg) = bg
        && let Some(seq) = lower_to_sgr(bg, color_depth, true)
    {
        open.push_str(&seq);
    }
    if open.is_empty() {
        content.to_string()
    } else {
        format!("{open}{content}\x1b[0m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tailwind_named_family_level() {
        let c = parse("red-500").unwrap();
        assert_eq!(c.color, Color::Tailwind(Tailwind::Red500));
        assert_eq!(c.opacity, None);
    }

    #[test]
    fn tailwind_with_opacity() {
        let c = parse("red-500/50").unwrap();
        assert_eq!(c.color, Color::Tailwind(Tailwind::Red500));
        assert_eq!(c.opacity, Some(50));
    }

    #[test]
    fn tailwind_opacity_bounds() {
        assert_eq!(parse("red-500/0").unwrap().opacity, Some(0));
        assert_eq!(parse("red-500/100").unwrap().opacity, Some(100));
        assert!(parse("red-500/101").is_err());
    }

    #[test]
    fn tailwind_specials() {
        assert_eq!(
            parse("transparent").unwrap().color,
            Color::Tailwind(Tailwind::Transparent)
        );
        assert_eq!(parse("black").unwrap().color, Color::Tailwind(Tailwind::Black));
        assert_eq!(parse("white").unwrap().color, Color::Tailwind(Tailwind::White));
    }

    #[test]
    fn tailwind_bad_level_errors() {
        // Level looks numeric but isn't a canonical step.
        assert!(parse("red-501").is_err());
    }

    #[test]
    fn empty_color_rejected() {
        assert!(parse("").is_err());
        assert!(parse("   ").is_err());
    }

    #[test]
    fn hex_short() {
        let c = parse("#fff").unwrap();
        match c.color {
            Color::Rgb(rgb) => {
                assert_eq!(rgb.red(), 255);
                assert_eq!(rgb.green(), 255);
                assert_eq!(rgb.blue(), 255);
            }
            _ => panic!("expected Color::Rgb"),
        }
        assert_eq!(c.opacity, None);
    }

    #[test]
    fn hex_long() {
        let c = parse("#ff8000").unwrap();
        match c.color {
            Color::Rgb(rgb) => {
                assert_eq!(rgb.red(), 255);
                assert_eq!(rgb.green(), 128);
                assert_eq!(rgb.blue(), 0);
            }
            _ => panic!("expected Color::Rgb"),
        }
    }

    #[test]
    fn hex_with_alpha() {
        let c = parse("#ff000080").unwrap();
        match c.color {
            Color::Rgb(rgb) => {
                assert_eq!(rgb.red(), 255);
                assert_eq!(rgb.green(), 0);
                assert_eq!(rgb.blue(), 0);
            }
            _ => panic!("expected Color::Rgb"),
        }
        // 0x80 / 255 * 100 ≈ 50
        assert_eq!(c.opacity, Some(50));
    }

    #[test]
    fn hex_invalid_digit_rejected() {
        let err = parse("#fg0").unwrap_err();
        assert_eq!(err, "non-hex digit");
    }

    #[test]
    fn hex_wrong_length_rejected() {
        assert!(parse("#ff").is_err());
        assert!(parse("#ffff").is_err());
        assert!(parse("#fffffff").is_err());
    }

    #[test]
    fn web_named_orange() {
        let c = parse("orange").unwrap();
        assert!(matches!(c.color, Color::Web(_)));
        assert_eq!(c.opacity, None);
    }

    /// Exhaustive table of every supported Tailwind `(family, level)` pair.
    /// Used by the matrix test below; kept as a separate constant so a
    /// missing or extra row is reviewable in isolation.
    ///
    /// Generated against the `Tailwind` variants the renderable crate
    /// publishes — 22 families × 11 levels = 242 entries.
    const TAILWIND_MATRIX: &[(&str, [Tailwind; 11])] = {
        use Tailwind::*;
        &[
            ("red",     [Red50, Red100, Red200, Red300, Red400, Red500, Red600, Red700, Red800, Red900, Red950]),
            ("orange",  [Orange50, Orange100, Orange200, Orange300, Orange400, Orange500, Orange600, Orange700, Orange800, Orange900, Orange950]),
            ("amber",   [Amber50, Amber100, Amber200, Amber300, Amber400, Amber500, Amber600, Amber700, Amber800, Amber900, Amber950]),
            ("yellow",  [Yellow50, Yellow100, Yellow200, Yellow300, Yellow400, Yellow500, Yellow600, Yellow700, Yellow800, Yellow900, Yellow950]),
            ("lime",    [Lime50, Lime100, Lime200, Lime300, Lime400, Lime500, Lime600, Lime700, Lime800, Lime900, Lime950]),
            ("green",   [Green50, Green100, Green200, Green300, Green400, Green500, Green600, Green700, Green800, Green900, Green950]),
            ("emerald", [Emerald50, Emerald100, Emerald200, Emerald300, Emerald400, Emerald500, Emerald600, Emerald700, Emerald800, Emerald900, Emerald950]),
            ("teal",    [Teal50, Teal100, Teal200, Teal300, Teal400, Teal500, Teal600, Teal700, Teal800, Teal900, Teal950]),
            ("cyan",    [Cyan50, Cyan100, Cyan200, Cyan300, Cyan400, Cyan500, Cyan600, Cyan700, Cyan800, Cyan900, Cyan950]),
            ("sky",     [Sky50, Sky100, Sky200, Sky300, Sky400, Sky500, Sky600, Sky700, Sky800, Sky900, Sky950]),
            ("blue",    [Blue50, Blue100, Blue200, Blue300, Blue400, Blue500, Blue600, Blue700, Blue800, Blue900, Blue950]),
            ("indigo",  [Indigo50, Indigo100, Indigo200, Indigo300, Indigo400, Indigo500, Indigo600, Indigo700, Indigo800, Indigo900, Indigo950]),
            ("violet",  [Violet50, Violet100, Violet200, Violet300, Violet400, Violet500, Violet600, Violet700, Violet800, Violet900, Violet950]),
            ("purple",  [Purple50, Purple100, Purple200, Purple300, Purple400, Purple500, Purple600, Purple700, Purple800, Purple900, Purple950]),
            ("fuchsia", [Fuchsia50, Fuchsia100, Fuchsia200, Fuchsia300, Fuchsia400, Fuchsia500, Fuchsia600, Fuchsia700, Fuchsia800, Fuchsia900, Fuchsia950]),
            ("pink",    [Pink50, Pink100, Pink200, Pink300, Pink400, Pink500, Pink600, Pink700, Pink800, Pink900, Pink950]),
            ("rose",    [Rose50, Rose100, Rose200, Rose300, Rose400, Rose500, Rose600, Rose700, Rose800, Rose900, Rose950]),
            ("slate",   [Slate50, Slate100, Slate200, Slate300, Slate400, Slate500, Slate600, Slate700, Slate800, Slate900, Slate950]),
            ("gray",    [Gray50, Gray100, Gray200, Gray300, Gray400, Gray500, Gray600, Gray700, Gray800, Gray900, Gray950]),
            ("zinc",    [Zinc50, Zinc100, Zinc200, Zinc300, Zinc400, Zinc500, Zinc600, Zinc700, Zinc800, Zinc900, Zinc950]),
            ("neutral", [Neutral50, Neutral100, Neutral200, Neutral300, Neutral400, Neutral500, Neutral600, Neutral700, Neutral800, Neutral900, Neutral950]),
            ("stone",   [Stone50, Stone100, Stone200, Stone300, Stone400, Stone500, Stone600, Stone700, Stone800, Stone900, Stone950]),
        ]
    };

    const TAILWIND_LEVELS: [&str; 11] =
        ["50", "100", "200", "300", "400", "500", "600", "700", "800", "900", "950"];

    /// Spec test #2: every supported Tailwind `(family, level)` combination
    /// parses to its matching `Tailwind` enum variant, including the four
    /// neutral families (`slate`, `zinc`, `neutral`, `stone`).
    #[test]
    fn tailwind_full_matrix() {
        assert_eq!(TAILWIND_MATRIX.len(), 22, "should cover all 22 families");
        let mut total = 0usize;
        for (family, variants) in TAILWIND_MATRIX {
            assert_eq!(
                variants.len(),
                TAILWIND_LEVELS.len(),
                "family {} should have 11 levels",
                family
            );
            for (level, expected) in TAILWIND_LEVELS.iter().zip(variants.iter()) {
                let input = format!("{}-{}", family, level);
                let parsed = parse(&input).unwrap_or_else(|e| {
                    panic!("`{}` failed to parse: {}", input, e)
                });
                assert_eq!(
                    parsed.color,
                    Color::Tailwind(*expected),
                    "`{}` produced unexpected variant",
                    input
                );
                assert_eq!(parsed.opacity, None, "`{}` should have no opacity", input);
                total += 1;
            }
        }
        assert_eq!(total, 242, "expected 22 * 11 = 242 cases");
    }

    /// Spot-check: opacity round-trips through the matrix for a sample of
    /// families/levels. The opacity-only edge cases (0, 50, 100, 101) are
    /// already covered by `tailwind_opacity_bounds`; this confirms the
    /// matrix entries don't lose opacity when present.
    #[test]
    fn tailwind_matrix_opacity_smoke() {
        for (family, variants) in TAILWIND_MATRIX {
            for (level, expected) in TAILWIND_LEVELS.iter().zip(variants.iter()) {
                let input = format!("{}-{}/50", family, level);
                let parsed = parse(&input).unwrap_or_else(|e| {
                    panic!("`{}` failed to parse: {}", input, e)
                });
                assert_eq!(parsed.color, Color::Tailwind(*expected), "`{}`", input);
                assert_eq!(parsed.opacity, Some(50), "`{}` opacity", input);
            }
        }
    }

    // ---------- Phase 2: lowering helper tests ----------

    use super::{lower_to_css, lower_to_sgr, wrap_with_color};
    use crate::markdown::output::terminal::ColorDepth;

    fn red_rgb() -> StyleColor {
        StyleColor {
            color: Color::Rgb(renderable::color::RgbColor::new(255, 0, 0, renderable::color::BasicColor::Red)),
            opacity: None,
        }
    }

    fn red_with_opacity() -> StyleColor {
        StyleColor {
            color: Color::Rgb(renderable::color::RgbColor::new(255, 0, 0, renderable::color::BasicColor::Red)),
            opacity: Some(50),
        }
    }

    #[test]
    fn lower_to_css_rgb() {
        assert_eq!(lower_to_css(&red_rgb()), Some("rgb(255, 0, 0)".to_string()));
    }

    #[test]
    fn lower_to_css_rgba_with_opacity() {
        assert_eq!(
            lower_to_css(&red_with_opacity()),
            Some("rgba(255, 0, 0, 0.5)".to_string())
        );
    }

    #[test]
    fn lower_to_css_tailwind_specials() {
        assert_eq!(
            lower_to_css(&StyleColor { color: Color::Tailwind(Tailwind::Transparent), opacity: None }),
            Some("transparent".to_string())
        );
        assert_eq!(
            lower_to_css(&StyleColor { color: Color::Tailwind(Tailwind::Current), opacity: None }),
            Some("currentColor".to_string())
        );
        assert_eq!(
            lower_to_css(&StyleColor { color: Color::Tailwind(Tailwind::Inherit), opacity: None }),
            Some("inherit".to_string())
        );
    }

    #[test]
    fn lower_to_css_unsupported_returns_none() {
        assert!(lower_to_css(&StyleColor { color: Color::DefaultForeground, opacity: None }).is_none());
        assert!(lower_to_css(&StyleColor { color: Color::DefaultBackground, opacity: None }).is_none());
        assert!(lower_to_css(&StyleColor { color: Color::Reset, opacity: None }).is_none());
    }

    #[test]
    fn lower_to_sgr_truecolor_fg() {
        assert_eq!(
            lower_to_sgr(&red_rgb(), ColorDepth::TrueColor, false),
            Some("\x1b[38;2;255;0;0m".to_string())
        );
    }

    #[test]
    fn lower_to_sgr_truecolor_bg() {
        assert_eq!(
            lower_to_sgr(&red_rgb(), ColorDepth::TrueColor, true),
            Some("\x1b[48;2;255;0;0m".to_string())
        );
    }

    #[test]
    fn lower_to_sgr_none_emits_nothing() {
        assert_eq!(lower_to_sgr(&red_rgb(), ColorDepth::None, false), None);
        assert_eq!(lower_to_sgr(&red_rgb(), ColorDepth::None, true), None);
    }

    #[test]
    fn lower_to_sgr_non_rgb_returns_none() {
        let transparent = StyleColor { color: Color::Tailwind(Tailwind::Transparent), opacity: None };
        assert_eq!(lower_to_sgr(&transparent, ColorDepth::TrueColor, false), None);

        let current = StyleColor { color: Color::Tailwind(Tailwind::Current), opacity: None };
        assert_eq!(lower_to_sgr(&current, ColorDepth::TrueColor, false), None);

        let inherit = StyleColor { color: Color::Tailwind(Tailwind::Inherit), opacity: None };
        assert_eq!(lower_to_sgr(&inherit, ColorDepth::TrueColor, false), None);
    }

    #[test]
    fn wrap_with_color_no_colors_returns_unchanged() {
        assert_eq!(wrap_with_color("hello", None, None, ColorDepth::TrueColor), "hello");
    }

    #[test]
    fn wrap_with_color_fg_only() {
        let out = wrap_with_color("hello", Some(&red_rgb()), None, ColorDepth::TrueColor);
        assert_eq!(out, "\x1b[38;2;255;0;0mhello\x1b[0m");
    }

    #[test]
    fn wrap_with_color_bg_only() {
        let out = wrap_with_color("hello", None, Some(&red_rgb()), ColorDepth::TrueColor);
        assert_eq!(out, "\x1b[48;2;255;0;0mhello\x1b[0m");
    }

    #[test]
    fn wrap_with_color_fg_and_bg() {
        let out = wrap_with_color("hello", Some(&red_rgb()), Some(&red_rgb()), ColorDepth::TrueColor);
        assert_eq!(out, "\x1b[38;2;255;0;0m\x1b[48;2;255;0;0mhello\x1b[0m");
    }

    #[test]
    fn wrap_with_color_none_depth_ignores_colors() {
        assert_eq!(
            wrap_with_color("hello", Some(&red_rgb()), Some(&red_rgb()), ColorDepth::None),
            "hello"
        );
    }

    #[test]
    fn wrap_with_color_reset_only_when_sgr_opened() {
        // A non-RGB color should not trigger reset.
        let transparent = StyleColor { color: Color::Tailwind(Tailwind::Transparent), opacity: None };
        assert_eq!(
            wrap_with_color("hello", Some(&transparent), None, ColorDepth::TrueColor),
            "hello"
        );
    }

    #[test]
    fn lower_to_css_web_color() {
        let orange = StyleColor {
            color: Color::Web(renderable::color::WebColor::Orange),
            opacity: None,
        };
        assert_eq!(lower_to_css(&orange), Some("rgb(255, 165, 0)".to_string()));
    }

    #[test]
    fn lower_to_sgr_basic_color() {
        let basic = StyleColor {
            color: Color::BasicColor(renderable::color::BasicColor::Red),
            opacity: None,
        };
        assert_eq!(
            lower_to_sgr(&basic, ColorDepth::TrueColor, false),
            Some("\x1b[38;2;128;0;0m".to_string())
        );
    }

    #[test]
    fn lower_to_sgr_tailwind_concrete() {
        let red = StyleColor {
            color: Color::Tailwind(Tailwind::Red500),
            opacity: None,
        };
        let sgr = lower_to_sgr(&red, ColorDepth::TrueColor, false);
        assert!(sgr.is_some());
        assert!(sgr.as_ref().unwrap().starts_with("\x1b[38;2;"));
    }

    #[test]
    fn lower_to_css_tailwind_concrete() {
        let red = StyleColor {
            color: Color::Tailwind(Tailwind::Red500),
            opacity: Some(75),
        };
        let css = lower_to_css(&red).unwrap();
        assert!(css.starts_with("rgba("));
        assert!(css.contains("0.75"));
    }
}
