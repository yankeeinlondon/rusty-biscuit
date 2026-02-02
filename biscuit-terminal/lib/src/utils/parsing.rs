//! Style token parsing for Prose components.
//!
//! This module provides parsing utilities for replacing style tokens with
//! ANSI escape codes. Two types of tokens are supported:
//!
//! ## Atomic Tokens
//!
//! Atomic tokens use the `{{token}}` syntax and are replaced with a single
//! escape code. They do not automatically reset and require explicit `{{reset}}`
//! to return to default state.
//!
//! Examples: `{{bold}}`, `{{italic}}`, `{{red}}`, `{{bg-blue}}`, `{{reset}}`
//!
//! ## Block Tokens
//!
//! Block tokens use HTML-like syntax with opening and closing tags. The content
//! between tags is wrapped with appropriate escape codes that automatically reset.
//!
//! Examples: `<b>bold</b>`, `<i>italic</i>`, `<red>colored</red>`

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use crate::discovery::clipboard::get_clipboard;
use crate::utils::color::{BasicColor, WebColor, WEB_COLOR_LOOKUP};

// =============================================================================
// Escape Code Constants
// =============================================================================

const ESC: &str = "\x1b[";
const RESET: &str = "\x1b[0m";

// Style codes
const BOLD: &str = "1";
const DIM: &str = "2";
const ITALIC: &str = "3";
const UNDERLINE: &str = "4";
const DOUBLE_UNDERLINE: &str = "4:2";
const STRIKETHROUGH: &str = "9";
const INVERSE: &str = "7";

// Reset codes
const RESET_BOLD_DIM: &str = "22";
const RESET_ITALIC: &str = "23";
const RESET_UNDERLINE: &str = "24";
const RESET_STRIKETHROUGH: &str = "29";
const RESET_INVERSE: &str = "27";
const RESET_FG: &str = "39";
const RESET_BG: &str = "49";

// =============================================================================
// Atomic Token Lookup
// =============================================================================

/// Mapping of atomic token names to their escape codes.
static ATOMIC_TOKENS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    // Style tokens
    m.insert("bold", "\x1b[1m");
    m.insert("dim", "\x1b[2m");
    m.insert("italic", "\x1b[3m");
    m.insert("underline", "\x1b[4m");
    m.insert("double-underline", "\x1b[4:2m");
    m.insert("curly-underline", "\x1b[4:3m");
    m.insert("dotted-underline", "\x1b[4:4m");
    m.insert("dashed-underline", "\x1b[4:5m");
    m.insert("strikethrough", "\x1b[9m");
    m.insert("inverse", "\x1b[7m");

    // Reset tokens
    m.insert("reset", "\x1b[0m");
    m.insert("reset-fg", "\x1b[39m");
    m.insert("reset-bg", "\x1b[49m");

    // Style-specific reset tokens
    m.insert("normal-font-weight", "\x1b[22m"); // Resets bold and dim
    m.insert("not-italic", "\x1b[23m");
    m.insert("not-underline", "\x1b[24m");
    m.insert("not-strikethrough", "\x1b[29m");
    m.insert("not-inverse", "\x1b[27m");

    // Basic foreground colors
    m.insert("black", "\x1b[30m");
    m.insert("red", "\x1b[31m");
    m.insert("green", "\x1b[32m");
    m.insert("yellow", "\x1b[33m");
    m.insert("blue", "\x1b[34m");
    m.insert("magenta", "\x1b[35m");
    m.insert("cyan", "\x1b[36m");
    m.insert("white", "\x1b[37m");

    // Bright foreground colors
    m.insert("bright-black", "\x1b[90m");
    m.insert("bright-red", "\x1b[91m");
    m.insert("bright-green", "\x1b[92m");
    m.insert("bright-yellow", "\x1b[93m");
    m.insert("bright-blue", "\x1b[94m");
    m.insert("bright-magenta", "\x1b[95m");
    m.insert("bright-cyan", "\x1b[96m");
    m.insert("bright-white", "\x1b[97m");

    // Basic background colors
    m.insert("bg-black", "\x1b[40m");
    m.insert("bg-red", "\x1b[41m");
    m.insert("bg-green", "\x1b[42m");
    m.insert("bg-yellow", "\x1b[43m");
    m.insert("bg-blue", "\x1b[44m");
    m.insert("bg-magenta", "\x1b[45m");
    m.insert("bg-cyan", "\x1b[46m");
    m.insert("bg-white", "\x1b[47m");

    // Bright background colors
    m.insert("bg-bright-black", "\x1b[100m");
    m.insert("bg-bright-red", "\x1b[101m");
    m.insert("bg-bright-green", "\x1b[102m");
    m.insert("bg-bright-yellow", "\x1b[103m");
    m.insert("bg-bright-blue", "\x1b[104m");
    m.insert("bg-bright-magenta", "\x1b[105m");
    m.insert("bg-bright-cyan", "\x1b[106m");
    m.insert("bg-bright-white", "\x1b[107m");

    m
});

/// Mapping of named colors to BasicColor for block token parsing.
static NAMED_COLORS: LazyLock<HashMap<&'static str, BasicColor>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("black", BasicColor::Black);
    m.insert("red", BasicColor::Red);
    m.insert("green", BasicColor::Green);
    m.insert("yellow", BasicColor::Yellow);
    m.insert("blue", BasicColor::Blue);
    m.insert("magenta", BasicColor::Magenta);
    m.insert("cyan", BasicColor::Cyan);
    m.insert("white", BasicColor::White);
    m.insert("bright-black", BasicColor::BrightBlack);
    m.insert("bright-red", BasicColor::BrightRed);
    m.insert("bright-green", BasicColor::BrightGreen);
    m.insert("bright-yellow", BasicColor::BrightYellow);
    m.insert("bright-blue", BasicColor::BrightBlue);
    m.insert("bright-magenta", BasicColor::BrightMagenta);
    m.insert("bright-cyan", BasicColor::BrightCyan);
    m.insert("bright-white", BasicColor::BrightWhite);
    m
});

// =============================================================================
// Compiled Regex Patterns
// =============================================================================

/// Regex for atomic tokens: `{{token}}`
static ATOMIC_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{([a-zA-Z0-9_-]+)\}\}").unwrap());

/// Regex for simple block tokens: `<tag>content</tag>`
static SIMPLE_BLOCK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<([a-zA-Z0-9~-]+)>(.*?)</\1>").unwrap());

/// Regex for anchor tags: `<a href="url">content</a>`
static ANCHOR_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<a\s+href="([^"]+)">(.*?)</a>"#).unwrap());

/// Regex for RGB color tags: `<rgb R,G,B>content</rgb>`
static RGB_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<rgb\s+(\d{1,3}),\s*(\d{1,3}),\s*(\d{1,3})>(.*?)</rgb>").unwrap()
});

/// Regex for clipboard tags: `<clipboard>fallback</clipboard>`
static CLIPBOARD_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<clipboard>(.*?)</clipboard>").unwrap());

// =============================================================================
// Public API
// =============================================================================

/// Parses content for style tokens and replaces them with ANSI escape codes.
///
/// This function processes both atomic tokens (`{{token}}`) and block tokens
/// (`<tag>content</tag>`), replacing them with appropriate ANSI escape sequences.
///
/// ## Atomic Tokens
///
/// Atomic tokens are simple replacements that don't automatically reset:
/// - `{{bold}}`, `{{dim}}`, `{{italic}}`, `{{underline}}`, `{{strikethrough}}`
/// - `{{red}}`, `{{blue}}`, `{{bright-red}}`, etc. (foreground colors)
/// - `{{bg-red}}`, `{{bg-blue}}`, etc. (background colors)
/// - `{{reset}}`, `{{reset-fg}}`, `{{reset-bg}}`
///
/// ## Block Tokens
///
/// Block tokens wrap content with escape codes that automatically reset:
/// - `<i>content</i>` - italic
/// - `<b>content</b>` - bold
/// - `<u>content</u>` - underline
/// - `<uu>content</uu>` - double underline
/// - `<~>content</~>` - strikethrough
/// - `<a href="url">content</a>` - OSC8 hyperlink
/// - `<rgb R,G,B>content</rgb>` - RGB foreground color
/// - `<red>content</red>` - named color foreground
/// - `<clipboard>fallback</clipboard>` - clipboard content or fallback
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::utils::parsing::parse_and_replace_style_tokens;
///
/// // Atomic tokens
/// let result = parse_and_replace_style_tokens("{{bold}}Hello{{reset}}");
/// assert!(result.contains("\x1b[1m"));
/// assert!(result.contains("\x1b[0m"));
///
/// // Block tokens
/// let result = parse_and_replace_style_tokens("<b>bold text</b>");
/// assert!(result.contains("\x1b[1m"));
/// assert!(result.contains("bold text"));
/// ```
///
/// ## Returns
///
/// A new string with all recognized tokens replaced by escape codes.
/// If at least one atomic token was used, a reset sequence is appended
/// to prevent style bleeding.
pub fn parse_and_replace_style_tokens<T: Into<String>>(content: T) -> String {
    let content = content.into();

    // Track if any atomic tokens were found (to append reset at end)
    let mut used_atomic = false;

    // Process atomic tokens
    let result = ATOMIC_PATTERN.replace_all(&content, |caps: &regex::Captures| {
        let token = caps.get(1).map_or("", |m| m.as_str()).to_lowercase();
        if let Some(escape) = ATOMIC_TOKENS.get(token.as_str()) {
            used_atomic = true;
            escape.to_string()
        } else {
            // Unknown token - leave as-is
            caps.get(0).map_or("", |m| m.as_str()).to_string()
        }
    });

    let result = result.into_owned();

    // Process block tokens
    let result = process_block_tokens(&result);

    // Append reset if atomic tokens were used (to prevent style bleeding)
    if used_atomic && !result.ends_with(RESET) {
        format!("{}{}", result, RESET)
    } else {
        result
    }
}

/// Result of parsing, including metadata about what was processed.
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// The processed content with escape codes.
    pub content: String,
    /// Whether any atomic tokens were found and replaced.
    pub used_atomic_tokens: bool,
    /// Whether any block tokens were found and replaced.
    pub used_block_tokens: bool,
}

/// Parses content and returns detailed result with metadata.
///
/// This is similar to [`parse_and_replace_style_tokens`] but returns
/// additional metadata about what was processed.
pub fn parse_with_metadata<T: Into<String>>(content: T) -> ParseResult {
    let content = content.into();

    let mut used_atomic = false;
    let mut used_block = false;

    // Process atomic tokens
    let result = ATOMIC_PATTERN.replace_all(&content, |caps: &regex::Captures| {
        let token = caps.get(1).map_or("", |m| m.as_str()).to_lowercase();
        if let Some(escape) = ATOMIC_TOKENS.get(token.as_str()) {
            used_atomic = true;
            escape.to_string()
        } else {
            caps.get(0).map_or("", |m| m.as_str()).to_string()
        }
    });

    let result = result.into_owned();

    // Process block tokens (check if any exist first)
    let has_blocks = SIMPLE_BLOCK_PATTERN.is_match(&result)
        || ANCHOR_PATTERN.is_match(&result)
        || RGB_PATTERN.is_match(&result)
        || CLIPBOARD_PATTERN.is_match(&result);

    let result = if has_blocks {
        used_block = true;
        process_block_tokens(&result)
    } else {
        result
    };

    // Append reset if atomic tokens were used
    let content = if used_atomic && !result.ends_with(RESET) {
        format!("{}{}", result, RESET)
    } else {
        result
    };

    ParseResult {
        content,
        used_atomic_tokens: used_atomic,
        used_block_tokens: used_block,
    }
}

// =============================================================================
// Block Token Processing
// =============================================================================

/// Process all block token types in the content.
fn process_block_tokens(content: &str) -> String {
    let mut result = content.to_string();

    // Process clipboard tags first (they may contain other tags)
    result = process_clipboard_tags(&result);

    // Process anchor tags (OSC8 links)
    result = process_anchor_tags(&result);

    // Process RGB color tags
    result = process_rgb_tags(&result);

    // Process simple block tags (includes named colors)
    result = process_simple_block_tags(&result);

    result
}

/// Process `<clipboard>fallback</clipboard>` tags.
fn process_clipboard_tags(content: &str) -> String {
    CLIPBOARD_PATTERN
        .replace_all(content, |caps: &regex::Captures| {
            let fallback = caps.get(1).map_or("", |m| m.as_str());

            // Try to get clipboard content, fall back to provided text
            match get_clipboard() {
                Some(clip) => clip,
                None => fallback.to_string(),
            }
        })
        .into_owned()
}

/// Process `<a href="url">content</a>` tags into OSC8 hyperlinks.
fn process_anchor_tags(content: &str) -> String {
    ANCHOR_PATTERN
        .replace_all(content, |caps: &regex::Captures| {
            let url = caps.get(1).map_or("", |m| m.as_str());
            let text = caps.get(2).map_or("", |m| m.as_str());

            // OSC8 hyperlink format: ESC]8;;URL BEL text ESC]8;; BEL
            format!("\x1b]8;;{}\x07{}\x1b]8;;\x07", url, text)
        })
        .into_owned()
}

/// Process `<rgb R,G,B>content</rgb>` tags.
fn process_rgb_tags(content: &str) -> String {
    RGB_PATTERN
        .replace_all(content, |caps: &regex::Captures| {
            let r: u8 = caps
                .get(1)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let g: u8 = caps
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let b: u8 = caps
                .get(3)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let text = caps.get(4).map_or("", |m| m.as_str());

            // 24-bit color: ESC[38;2;R;G;Bm
            format!("{}38;2;{};{};{}m{}{}", ESC, r, g, b, text, RESET)
        })
        .into_owned()
}

/// Process simple block tags like `<b>`, `<i>`, `<u>`, `<uu>`, `<~>`, and named colors.
fn process_simple_block_tags(content: &str) -> String {
    SIMPLE_BLOCK_PATTERN
        .replace_all(content, |caps: &regex::Captures| {
            let tag = caps.get(1).map_or("", |m| m.as_str()).to_lowercase();
            let text = caps.get(2).map_or("", |m| m.as_str());

            match tag.as_str() {
                // Style tags
                "b" => format!("{}{}m{}{}{}m", ESC, BOLD, text, ESC, RESET_BOLD_DIM),
                "i" => format!("{}{}m{}{}{}m", ESC, ITALIC, text, ESC, RESET_ITALIC),
                "u" => format!("{}{}m{}{}{}m", ESC, UNDERLINE, text, ESC, RESET_UNDERLINE),
                "uu" => format!(
                    "{}{}m{}{}{}m",
                    ESC, DOUBLE_UNDERLINE, text, ESC, RESET_UNDERLINE
                ),
                "~" => format!(
                    "{}{}m{}{}{}m",
                    ESC, STRIKETHROUGH, text, ESC, RESET_STRIKETHROUGH
                ),
                "dim" => format!("{}{}m{}{}{}m", ESC, DIM, text, ESC, RESET_BOLD_DIM),
                "inverse" | "inv" => {
                    format!("{}{}m{}{}{}m", ESC, INVERSE, text, ESC, RESET_INVERSE)
                }

                // Check for basic named colors
                _ => {
                    if let Some(color) = NAMED_COLORS.get(tag.as_str()) {
                        let code = basic_color_fg_code(*color);
                        format!("{}{}m{}{}{}m", ESC, code, text, ESC, RESET_FG)
                    } else if let Some(rgb) = lookup_web_color(&tag) {
                        // Try web colors
                        format!(
                            "{}38;2;{};{};{}m{}{}{}m",
                            ESC, rgb.0, rgb.1, rgb.2, text, ESC, RESET_FG
                        )
                    } else {
                        // Unknown tag - leave as-is
                        caps.get(0).map_or("", |m| m.as_str()).to_string()
                    }
                }
            }
        })
        .into_owned()
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Get the ANSI foreground color code for a BasicColor.
fn basic_color_fg_code(color: BasicColor) -> u8 {
    match color {
        BasicColor::Black => 30,
        BasicColor::Red => 31,
        BasicColor::Green => 32,
        BasicColor::Yellow => 33,
        BasicColor::Blue => 34,
        BasicColor::Magenta => 35,
        BasicColor::Cyan => 36,
        BasicColor::White => 37,
        BasicColor::BrightBlack => 90,
        BasicColor::BrightRed => 91,
        BasicColor::BrightGreen => 92,
        BasicColor::BrightYellow => 93,
        BasicColor::BrightBlue => 94,
        BasicColor::BrightMagenta => 95,
        BasicColor::BrightCyan => 96,
        BasicColor::BrightWhite => 97,
    }
}

/// Try to look up a web color by name and return its RGB values.
fn lookup_web_color(name: &str) -> Option<(u8, u8, u8)> {
    // Convert name to WebColor enum variant
    let web_color = match name.to_lowercase().as_str() {
        "aliceblue" => Some(WebColor::AliceBlue),
        "antiquewhite" => Some(WebColor::AntiqueWhite),
        "aqua" => Some(WebColor::Aqua),
        "aquamarine" => Some(WebColor::Aquamarine),
        "azure" => Some(WebColor::Azure),
        "beige" => Some(WebColor::Beige),
        "bisque" => Some(WebColor::Bisque),
        "blanchedalmond" => Some(WebColor::BlanchedAlmond),
        "blueviolet" => Some(WebColor::BlueViolet),
        "brown" => Some(WebColor::Brown),
        "burlywood" => Some(WebColor::BurlyWood),
        "cadetblue" => Some(WebColor::CadetBlue),
        "chartreuse" => Some(WebColor::Chartreuse),
        "chocolate" => Some(WebColor::Chocolate),
        "coral" => Some(WebColor::Coral),
        "cornflowerblue" => Some(WebColor::CornflowerBlue),
        "cornsilk" => Some(WebColor::Cornsilk),
        "crimson" => Some(WebColor::Crimson),
        "darkblue" => Some(WebColor::DarkBlue),
        "darkcyan" => Some(WebColor::DarkCyan),
        "darkgoldenrod" => Some(WebColor::DarkGoldenrod),
        "darkgray" | "darkgrey" => Some(WebColor::DarkGray),
        "darkgreen" => Some(WebColor::DarkGreen),
        "darkkhaki" => Some(WebColor::DarkKhaki),
        "darkmagenta" => Some(WebColor::DarkMagenta),
        "darkolivegreen" => Some(WebColor::DarkOliveGreen),
        "darkorange" => Some(WebColor::DarkOrange),
        "darkorchid" => Some(WebColor::DarkOrchid),
        "darkred" => Some(WebColor::DarkRed),
        "darksalmon" => Some(WebColor::DarkSalmon),
        "darkseagreen" => Some(WebColor::DarkSeaGreen),
        "darkslateblue" => Some(WebColor::DarkSlateBlue),
        "darkslategray" | "darkslategrey" => Some(WebColor::DarkSlateGray),
        "darkturquoise" => Some(WebColor::DarkTurquoise),
        "darkviolet" => Some(WebColor::DarkViolet),
        "deeppink" => Some(WebColor::DeepPink),
        "deepskyblue" => Some(WebColor::DeepSkyBlue),
        "dimgray" | "dimgrey" => Some(WebColor::DimGray),
        "dodgerblue" => Some(WebColor::DodgerBlue),
        "firebrick" => Some(WebColor::FireBrick),
        "floralwhite" => Some(WebColor::FloralWhite),
        "forestgreen" => Some(WebColor::ForestGreen),
        "fuchsia" => Some(WebColor::Fuchsia),
        "gainsboro" => Some(WebColor::Gainsboro),
        "ghostwhite" => Some(WebColor::GhostWhite),
        "gold" => Some(WebColor::Gold),
        "goldenrod" => Some(WebColor::Goldenrod),
        "gray" | "grey" => Some(WebColor::Gray),
        "greenyellow" => Some(WebColor::GreenYellow),
        "honeydew" => Some(WebColor::HoneyDew),
        "hotpink" => Some(WebColor::HotPink),
        "indianred" => Some(WebColor::IndianRed),
        "indigo" => Some(WebColor::Indigo),
        "ivory" => Some(WebColor::Ivory),
        "khaki" => Some(WebColor::Khaki),
        "lavender" => Some(WebColor::Lavender),
        "lavenderblush" => Some(WebColor::LavenderBlush),
        "lawngreen" => Some(WebColor::LawnGreen),
        "lemonchiffon" => Some(WebColor::LemonChiffon),
        "lightblue" => Some(WebColor::LightBlue),
        "lightcoral" => Some(WebColor::LightCoral),
        "lightcyan" => Some(WebColor::LightCyan),
        "lightgoldenrodyellow" => Some(WebColor::LightGoldenrodYellow),
        "lightgray" | "lightgrey" => Some(WebColor::LightGray),
        "lightgreen" => Some(WebColor::LightGreen),
        "lightpink" => Some(WebColor::LightPink),
        "lightsalmon" => Some(WebColor::LightSalmon),
        "lightseagreen" => Some(WebColor::LightSeaGreen),
        "lightskyblue" => Some(WebColor::LightSkyBlue),
        "lightslategray" | "lightslategrey" => Some(WebColor::LightSlateGray),
        "lightsteelblue" => Some(WebColor::LightSteelBlue),
        "lightyellow" => Some(WebColor::LightYellow),
        "lime" => Some(WebColor::Lime),
        "limegreen" => Some(WebColor::LimeGreen),
        "linen" => Some(WebColor::Linen),
        "maroon" => Some(WebColor::Maroon),
        "mediumaquamarine" => Some(WebColor::MediumAquamarine),
        "mediumblue" => Some(WebColor::MediumBlue),
        "mediumorchid" => Some(WebColor::MediumOrchid),
        "mediumpurple" => Some(WebColor::MediumPurple),
        "mediumseagreen" => Some(WebColor::MediumSeaGreen),
        "mediumslateblue" => Some(WebColor::MediumSlateBlue),
        "mediumspringgreen" => Some(WebColor::MediumSpringGreen),
        "mediumturquoise" => Some(WebColor::MediumTurquoise),
        "mediumvioletred" => Some(WebColor::MediumVioletRed),
        "midnightblue" => Some(WebColor::MidnightBlue),
        "mintcream" => Some(WebColor::MintCream),
        "mistyrose" => Some(WebColor::MistyRose),
        "moccasin" => Some(WebColor::Moccasin),
        "navajowhite" => Some(WebColor::NavajoWhite),
        "navy" => Some(WebColor::Navy),
        "oldlace" => Some(WebColor::OldLace),
        "olive" => Some(WebColor::Olive),
        "olivedrab" => Some(WebColor::OliveDrab),
        "orange" => Some(WebColor::Orange),
        "orangered" => Some(WebColor::OrangeRed),
        "orchid" => Some(WebColor::Orchid),
        "palegoldenrod" => Some(WebColor::PaleGoldenrod),
        "palegreen" => Some(WebColor::PaleGreen),
        "paleturquoise" => Some(WebColor::PaleTurquoise),
        "palevioletred" => Some(WebColor::PaleVioletRed),
        "papayawhip" => Some(WebColor::PapayaWhip),
        "peachpuff" => Some(WebColor::PeachPuff),
        "peru" => Some(WebColor::Peru),
        "pink" => Some(WebColor::Pink),
        "plum" => Some(WebColor::Plum),
        "powderblue" => Some(WebColor::PowderBlue),
        "purple" => Some(WebColor::Purple),
        "rebeccapurple" => Some(WebColor::RebeccaPurple),
        "rosybrown" => Some(WebColor::RosyBrown),
        "royalblue" => Some(WebColor::RoyalBlue),
        "saddlebrown" => Some(WebColor::SaddleBrown),
        "salmon" => Some(WebColor::Salmon),
        "sandybrown" => Some(WebColor::SandyBrown),
        "seagreen" => Some(WebColor::SeaGreen),
        "seashell" => Some(WebColor::SeaShell),
        "sienna" => Some(WebColor::Sienna),
        "silver" => Some(WebColor::Silver),
        "skyblue" => Some(WebColor::SkyBlue),
        "slateblue" => Some(WebColor::SlateBlue),
        "slategray" | "slategrey" => Some(WebColor::SlateGray),
        "snow" => Some(WebColor::Snow),
        "springgreen" => Some(WebColor::SpringGreen),
        "steelblue" => Some(WebColor::SteelBlue),
        "tan" => Some(WebColor::Tan),
        "teal" => Some(WebColor::Teal),
        "thistle" => Some(WebColor::Thistle),
        "tomato" => Some(WebColor::Tomato),
        "turquoise" => Some(WebColor::Turquoise),
        "violet" => Some(WebColor::Violet),
        "wheat" => Some(WebColor::Wheat),
        "whitesmoke" => Some(WebColor::WhiteSmoke),
        "yellowgreen" => Some(WebColor::YellowGreen),
        _ => None,
    };

    web_color.and_then(|wc| {
        WEB_COLOR_LOOKUP
            .get(&wc)
            .map(|rgb| (rgb.red(), rgb.green(), rgb.blue()))
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Atomic Token Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_atomic_bold() {
        let result = parse_and_replace_style_tokens("{{bold}}Hello");
        assert!(result.starts_with("\x1b[1m"));
        assert!(result.contains("Hello"));
        // Should end with reset since atomic token was used
        assert!(result.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_atomic_italic() {
        let result = parse_and_replace_style_tokens("{{italic}}text{{reset}}");
        assert!(result.contains("\x1b[3m"));
        assert!(result.contains("text"));
        assert!(result.contains("\x1b[0m"));
    }

    #[test]
    fn test_atomic_colors() {
        let result = parse_and_replace_style_tokens("{{red}}red text{{reset}}");
        assert!(result.contains("\x1b[31m"));

        let result = parse_and_replace_style_tokens("{{bright-blue}}bright{{reset}}");
        assert!(result.contains("\x1b[94m"));

        let result = parse_and_replace_style_tokens("{{bg-green}}background{{reset}}");
        assert!(result.contains("\x1b[42m"));
    }

    #[test]
    fn test_atomic_reset_variants() {
        let result = parse_and_replace_style_tokens("{{reset-fg}}");
        assert!(result.contains("\x1b[39m"));

        let result = parse_and_replace_style_tokens("{{reset_bg}}");
        assert!(result.contains("\x1b[49m"));
    }

    #[test]
    fn test_unknown_atomic_token_preserved() {
        let result = parse_and_replace_style_tokens("{{unknown_token}}");
        assert_eq!(result, "{{unknown_token}}");
    }

    #[test]
    fn test_atomic_case_insensitive() {
        let result = parse_and_replace_style_tokens("{{BOLD}}{{Bold}}{{bold}}");
        // All three should become the same escape code
        assert_eq!(result.matches("\x1b[1m").count(), 3);
    }

    // -------------------------------------------------------------------------
    // Block Token Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_block_bold() {
        let result = parse_and_replace_style_tokens("<b>bold text</b>");
        assert!(result.contains("\x1b[1m"));
        assert!(result.contains("bold text"));
        assert!(result.contains("\x1b[22m")); // Reset bold/dim
    }

    #[test]
    fn test_block_italic() {
        let result = parse_and_replace_style_tokens("<i>italic text</i>");
        assert!(result.contains("\x1b[3m"));
        assert!(result.contains("italic text"));
        assert!(result.contains("\x1b[23m")); // Reset italic
    }

    #[test]
    fn test_block_underline() {
        let result = parse_and_replace_style_tokens("<u>underlined</u>");
        assert!(result.contains("\x1b[4m"));
        assert!(result.contains("\x1b[24m")); // Reset underline
    }

    #[test]
    fn test_block_double_underline() {
        let result = parse_and_replace_style_tokens("<uu>double</uu>");
        assert!(result.contains("\x1b[4:2m"));
        assert!(result.contains("\x1b[24m"));
    }

    #[test]
    fn test_block_strikethrough() {
        let result = parse_and_replace_style_tokens("<~>strikethrough</~>");
        assert!(result.contains("\x1b[9m"));
        assert!(result.contains("\x1b[29m"));
    }

    #[test]
    fn test_block_named_color() {
        let result = parse_and_replace_style_tokens("<red>red text</red>");
        assert!(result.contains("\x1b[31m"));
        assert!(result.contains("red text"));
        assert!(result.contains("\x1b[39m")); // Reset foreground
    }

    #[test]
    fn test_block_bright_color() {
        let result = parse_and_replace_style_tokens("<bright-cyan>bright</bright-cyan>");
        assert!(result.contains("\x1b[96m"));
        assert!(result.contains("\x1b[39m"));
    }

    #[test]
    fn test_block_rgb_color() {
        let result = parse_and_replace_style_tokens("<rgb 255,128,64>orange</rgb>");
        assert!(result.contains("\x1b[38;2;255;128;64m"));
        assert!(result.contains("orange"));
        assert!(result.contains("\x1b[0m"));
    }

    #[test]
    fn test_block_rgb_with_spaces() {
        let result = parse_and_replace_style_tokens("<rgb 100, 150, 200>spaced</rgb>");
        assert!(result.contains("\x1b[38;2;100;150;200m"));
    }

    #[test]
    fn test_anchor_tag() {
        let result = parse_and_replace_style_tokens(r#"<a href="https://example.com">link</a>"#);
        // OSC8 format: ESC]8;;URL BEL text ESC]8;; BEL
        assert!(result.contains("\x1b]8;;https://example.com\x07"));
        assert!(result.contains("link"));
        assert!(result.contains("\x1b]8;;\x07"));
    }

    #[test]
    fn test_clipboard_tag_fallback() {
        // In test environment, clipboard is likely unavailable, so fallback is used
        let result = parse_and_replace_style_tokens("<clipboard>fallback text</clipboard>");
        assert!(result.contains("fallback text"));
    }

    #[test]
    fn test_web_color_block() {
        let result = parse_and_replace_style_tokens("<coral>coral colored</coral>");
        // Coral is RGB(255, 127, 80)
        assert!(result.contains("\x1b[38;2;255;127;80m"));
        assert!(result.contains("coral colored"));
    }

    #[test]
    fn test_unknown_block_tag_preserved() {
        let result = parse_and_replace_style_tokens("<unknown>content</unknown>");
        assert_eq!(result, "<unknown>content</unknown>");
    }

    // -------------------------------------------------------------------------
    // Combined Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_nested_styles() {
        let result = parse_and_replace_style_tokens("<b>bold with <i>italic</i> inside</b>");
        assert!(result.contains("\x1b[1m")); // Bold
        assert!(result.contains("\x1b[3m")); // Italic
    }

    #[test]
    fn test_mixed_atomic_and_block() {
        let result = parse_and_replace_style_tokens("{{bold}}<red>colored</red>{{reset}}");
        assert!(result.contains("\x1b[1m")); // Bold atomic
        assert!(result.contains("\x1b[31m")); // Red block
        assert!(result.contains("\x1b[0m")); // Reset
    }

    #[test]
    fn test_empty_input() {
        let result = parse_and_replace_style_tokens("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_no_tokens() {
        let result = parse_and_replace_style_tokens("plain text");
        assert_eq!(result, "plain text");
    }

    #[test]
    fn test_malformed_tokens_preserved() {
        let result = parse_and_replace_style_tokens("{bold} <b>unclosed");
        assert!(result.contains("{bold}"));
        assert!(result.contains("<b>unclosed"));
    }

    // -------------------------------------------------------------------------
    // ParseResult Metadata Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_with_metadata_atomic() {
        let result = parse_with_metadata("{{bold}}test");
        assert!(result.used_atomic_tokens);
        assert!(!result.used_block_tokens);
    }

    #[test]
    fn test_parse_with_metadata_block() {
        let result = parse_with_metadata("<b>test</b>");
        assert!(!result.used_atomic_tokens);
        assert!(result.used_block_tokens);
    }

    #[test]
    fn test_parse_with_metadata_both() {
        let result = parse_with_metadata("{{italic}}<b>test</b>");
        assert!(result.used_atomic_tokens);
        assert!(result.used_block_tokens);
    }

    #[test]
    fn test_parse_with_metadata_none() {
        let result = parse_with_metadata("plain text");
        assert!(!result.used_atomic_tokens);
        assert!(!result.used_block_tokens);
        assert_eq!(result.content, "plain text");
    }
}
