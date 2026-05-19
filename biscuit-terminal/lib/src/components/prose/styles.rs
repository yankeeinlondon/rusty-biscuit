//! Color name lookups, href resolution, and per-layer SGR state.
//!
//! Resolves bracketed-tag color names to [`renderable::color`] values and
//! tracks the active SGR escape for each independent style layer so the
//! terminal emitter can restore a *parent* span's value when a child span
//! closes (instead of issuing a nuclear `\x1b[0m`).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{
    terminal::Terminal,
    utils::color::{Tailwind, WebColor},
};

/// Resolves the opening SGR escape for a `<double-underline>` request
/// against the terminal's actual underline-support profile.
///
/// ## Returns
///
/// - `Some("\x1b[4:2m")` when no terminal context is available (legacy
///   optimistic behavior) **or** when the terminal advertises
///   [`UnderlineSupport::double`].
/// - `Some("\x1b[4m")` when only [`UnderlineSupport::straight`] is
///   advertised — the canonical Apple Terminal path.
/// - `None` when neither variant is supported, signalling the caller to
///   suppress the underline entirely (no SGR at all).
///
/// ## Notes
///
/// The closing SGR for any non-`None` return is always `"\x1b[24m"`.
///
/// [`UnderlineSupport::double`]: crate::discovery::detection::UnderlineSupport::double
/// [`UnderlineSupport::straight`]: crate::discovery::detection::UnderlineSupport::straight
pub(super) fn degraded_double_underline_open(term: Option<&Terminal>) -> Option<&'static str> {
    match term {
        None => Some("\x1b[4:2m"),
        Some(t) if t.underline_support.double => Some("\x1b[4:2m"),
        Some(t) if t.underline_support.straight => Some("\x1b[4m"),
        Some(_) => None,
    }
}

/// Parse an RGB string in multiple formats into (r, g, b).
///
/// Supported formats:
/// - Comma-separated: "125,67,45"
/// - Space-separated: "125 67 45"
/// - Hex with #: "#8B0000"
/// - Hex without #: "8B0000"
pub(super) fn parse_rgb(s: &str) -> Option<(u8, u8, u8)> {
    let s = s.trim();

    // Try hex format first (#RRGGBB or RRGGBB)
    if let Some(hex_str) = s.strip_prefix('#') {
        return parse_hex_rgb(hex_str);
    }

    // Check if it looks like a hex value (6 hex digits, no separators)
    if s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        return parse_hex_rgb(s);
    }

    // Try comma-separated: "125,67,45"
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() == 3 {
        let r = parts[0].trim().parse::<u8>().ok()?;
        let g = parts[1].trim().parse::<u8>().ok()?;
        let b = parts[2].trim().parse::<u8>().ok()?;
        return Some((r, g, b));
    }

    // Try space-separated: "125 67 45"
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() == 3 {
        let r = parts[0].parse::<u8>().ok()?;
        let g = parts[1].parse::<u8>().ok()?;
        let b = parts[2].parse::<u8>().ok()?;
        return Some((r, g, b));
    }

    None
}

/// Parse a hex RGB string (6 hex digits) into (r, g, b).
fn parse_hex_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some((r, g, b))
}

/// Resolve an href value, handling relative file paths.
///
/// Path resolution rules:
/// - URLs (http://, https://, etc.) are returned as-is
/// - Absolute paths (/path/to/file) are returned as file:// URLs
/// - Paths starting with ./ are resolved relative to CWD
/// - Other paths (no ./ prefix) are resolved relative to:
///   1. If in a git repo monorepo: package root containing CWD
///   2. If in a git repo: repo root
///   3. Otherwise: CWD (fallback)
pub(super) fn resolve_href(href: &str) -> String {
    // Empty href
    if href.is_empty() {
        return String::new();
    }

    // URLs pass through unchanged
    if href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("file://")
        || href.starts_with("mailto:")
    {
        return href.to_string();
    }

    // Absolute paths become file:// URLs
    if href.starts_with('/') {
        return format!("file://{}", href);
    }

    // Relative paths starting with ./ are relative to CWD
    if let Some(relative_path) = href.strip_prefix("./") {
        if let Ok(cwd) = std::env::current_dir() {
            let resolved = cwd.join(relative_path);
            if let Ok(canonical) = resolved.canonicalize() {
                return format!("file://{}", canonical.display());
            }
            // Fall back to non-canonical path
            return format!("file://{}", resolved.display());
        }
        // CWD unavailable, return as-is
        return href.to_string();
    }

    // Other relative paths: resolve from git root (or package root in monorepo)
    if let Some(base) = find_git_relative_base() {
        let resolved = base.join(href);
        if let Ok(canonical) = resolved.canonicalize() {
            return format!("file://{}", canonical.display());
        }
        // Fall back to non-canonical path if file doesn't exist yet
        return format!("file://{}", resolved.display());
    }

    // No git root found, fall back to CWD
    if let Ok(cwd) = std::env::current_dir() {
        let resolved = cwd.join(href);
        return format!("file://{}", resolved.display());
    }

    // Last resort: return as-is
    href.to_string()
}

/// Find the base directory for resolving relative paths without ./ prefix.
fn find_git_relative_base() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let git_root = find_git_root(&cwd)?;

    if let Some(package_root) = find_package_root(&cwd, &git_root)
        && package_root != git_root
    {
        return Some(package_root);
    }

    Some(git_root)
}

/// Find the git repository root starting from the given path.
fn find_git_root(start: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(start)
        .output()
        .ok()?;

    if output.status.success() {
        let path_str = String::from_utf8(output.stdout).ok()?;
        return Some(PathBuf::from(path_str.trim()));
    }

    let mut current = start.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Find the nearest package root (directory with Cargo.toml) between start and repo root.
fn find_package_root(start: &Path, git_root: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();

    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists()
            && let Ok(contents) = std::fs::read_to_string(&cargo_toml)
            && contents.contains("[package]")
        {
            return Some(current);
        }

        if current == git_root || !current.starts_with(git_root) {
            break;
        }

        if !current.pop() {
            break;
        }
    }

    None
}

/// Render a [`Color`](renderable::color::Color) as a CSS color string.
///
/// Shared by the Browser and MarkdownPlus emitters. Colors that have no RGB
/// representation degrade to `inherit`.
pub(super) fn css_color(color: &renderable::color::Color) -> String {
    match color.to_rgb() {
        Some((r, g, b)) => format!("rgb({}, {}, {})", r, g, b),
        None => "inherit".to_string(),
    }
}

/// Compare two strings case-insensitively, skipping hyphens in `input`.
fn eq_ignore_case_and_hyphens(input: &str, target: &str) -> bool {
    let mut input_chars = input.chars().filter(|c| *c != '-');
    let mut target_chars = target.chars();

    loop {
        match (input_chars.next(), target_chars.next()) {
            (Some(a), Some(b)) => {
                if !a.eq_ignore_ascii_case(&b) {
                    return false;
                }
            }
            (None, None) => return true,
            _ => return false,
        }
    }
}

/// Static lookup table mapping kebab-case web color names to [`WebColor`].
static WEB_COLOR_TABLE: &[(&str, WebColor)] = &[
    ("aliceblue", WebColor::AliceBlue),
    ("antiquewhite", WebColor::AntiqueWhite),
    ("aqua", WebColor::Aqua),
    ("aquamarine", WebColor::Aquamarine),
    ("azure", WebColor::Azure),
    ("beige", WebColor::Beige),
    ("bisque", WebColor::Bisque),
    ("blanchedalmond", WebColor::BlanchedAlmond),
    ("blueviolet", WebColor::BlueViolet),
    ("brown", WebColor::Brown),
    ("burlywood", WebColor::BurlyWood),
    ("cadetblue", WebColor::CadetBlue),
    ("chartreuse", WebColor::Chartreuse),
    ("chocolate", WebColor::Chocolate),
    ("coral", WebColor::Coral),
    ("cornflowerblue", WebColor::CornflowerBlue),
    ("cornsilk", WebColor::Cornsilk),
    ("crimson", WebColor::Crimson),
    ("darkblue", WebColor::DarkBlue),
    ("darkcyan", WebColor::DarkCyan),
    ("darkgoldenrod", WebColor::DarkGoldenrod),
    ("darkgray", WebColor::DarkGray),
    ("darkgrey", WebColor::DarkGray),
    ("darkgreen", WebColor::DarkGreen),
    ("darkkhaki", WebColor::DarkKhaki),
    ("darkmagenta", WebColor::DarkMagenta),
    ("darkolivegreen", WebColor::DarkOliveGreen),
    ("darkorange", WebColor::DarkOrange),
    ("darkorchid", WebColor::DarkOrchid),
    ("darkred", WebColor::DarkRed),
    ("darksalmon", WebColor::DarkSalmon),
    ("darkseagreen", WebColor::DarkSeaGreen),
    ("darkslateblue", WebColor::DarkSlateBlue),
    ("darkslategray", WebColor::DarkSlateGray),
    ("darkslategrey", WebColor::DarkSlateGray),
    ("darkturquoise", WebColor::DarkTurquoise),
    ("darkviolet", WebColor::DarkViolet),
    ("deeppink", WebColor::DeepPink),
    ("deepskyblue", WebColor::DeepSkyBlue),
    ("dimgray", WebColor::DimGray),
    ("dimgrey", WebColor::DimGray),
    ("dodgerblue", WebColor::DodgerBlue),
    ("firebrick", WebColor::FireBrick),
    ("floralwhite", WebColor::FloralWhite),
    ("forestgreen", WebColor::ForestGreen),
    ("fuchsia", WebColor::Fuchsia),
    ("gainsboro", WebColor::Gainsboro),
    ("ghostwhite", WebColor::GhostWhite),
    ("gold", WebColor::Gold),
    ("goldenrod", WebColor::Goldenrod),
    ("gray", WebColor::Gray),
    ("grey", WebColor::Gray),
    ("greenyellow", WebColor::GreenYellow),
    ("honeydew", WebColor::HoneyDew),
    ("hotpink", WebColor::HotPink),
    ("indianred", WebColor::IndianRed),
    ("indigo", WebColor::Indigo),
    ("ivory", WebColor::Ivory),
    ("khaki", WebColor::Khaki),
    ("lavender", WebColor::Lavender),
    ("lavenderblush", WebColor::LavenderBlush),
    ("lawngreen", WebColor::LawnGreen),
    ("lemonchiffon", WebColor::LemonChiffon),
    ("lightblue", WebColor::LightBlue),
    ("lightcoral", WebColor::LightCoral),
    ("lightcyan", WebColor::LightCyan),
    ("lightgoldenrodyellow", WebColor::LightGoldenrodYellow),
    ("lightgray", WebColor::LightGray),
    ("lightgrey", WebColor::LightGray),
    ("lightgreen", WebColor::LightGreen),
    ("lightpink", WebColor::LightPink),
    ("lightsalmon", WebColor::LightSalmon),
    ("lightseagreen", WebColor::LightSeaGreen),
    ("lightskyblue", WebColor::LightSkyBlue),
    ("lightslategray", WebColor::LightSlateGray),
    ("lightslategrey", WebColor::LightSlateGray),
    ("lightsteelblue", WebColor::LightSteelBlue),
    ("lightyellow", WebColor::LightYellow),
    ("lime", WebColor::Lime),
    ("limegreen", WebColor::LimeGreen),
    ("linen", WebColor::Linen),
    ("maroon", WebColor::Maroon),
    ("mediumaquamarine", WebColor::MediumAquamarine),
    ("mediumblue", WebColor::MediumBlue),
    ("mediumorchid", WebColor::MediumOrchid),
    ("mediumpurple", WebColor::MediumPurple),
    ("mediumseagreen", WebColor::MediumSeaGreen),
    ("mediumslateblue", WebColor::MediumSlateBlue),
    ("mediumspringgreen", WebColor::MediumSpringGreen),
    ("mediumturquoise", WebColor::MediumTurquoise),
    ("mediumvioletred", WebColor::MediumVioletRed),
    ("midnightblue", WebColor::MidnightBlue),
    ("mintcream", WebColor::MintCream),
    ("mistyrose", WebColor::MistyRose),
    ("moccasin", WebColor::Moccasin),
    ("navajowhite", WebColor::NavajoWhite),
    ("navy", WebColor::Navy),
    ("oldlace", WebColor::OldLace),
    ("olive", WebColor::Olive),
    ("olivedrab", WebColor::OliveDrab),
    ("orange", WebColor::Orange),
    ("orangered", WebColor::OrangeRed),
    ("orchid", WebColor::Orchid),
    ("palegoldenrod", WebColor::PaleGoldenrod),
    ("palegreen", WebColor::PaleGreen),
    ("paleturquoise", WebColor::PaleTurquoise),
    ("palevioletred", WebColor::PaleVioletRed),
    ("papayawhip", WebColor::PapayaWhip),
    ("peachpuff", WebColor::PeachPuff),
    ("peru", WebColor::Peru),
    ("pink", WebColor::Pink),
    ("plum", WebColor::Plum),
    ("powderblue", WebColor::PowderBlue),
    ("purple", WebColor::Purple),
    ("rebeccapurple", WebColor::RebeccaPurple),
    ("rosybrown", WebColor::RosyBrown),
    ("royalblue", WebColor::RoyalBlue),
    ("saddlebrown", WebColor::SaddleBrown),
    ("salmon", WebColor::Salmon),
    ("sandybrown", WebColor::SandyBrown),
    ("seagreen", WebColor::SeaGreen),
    ("seashell", WebColor::SeaShell),
    ("sienna", WebColor::Sienna),
    ("silver", WebColor::Silver),
    ("skyblue", WebColor::SkyBlue),
    ("slateblue", WebColor::SlateBlue),
    ("slategray", WebColor::SlateGray),
    ("slategrey", WebColor::SlateGray),
    ("snow", WebColor::Snow),
    ("springgreen", WebColor::SpringGreen),
    ("steelblue", WebColor::SteelBlue),
    ("tan", WebColor::Tan),
    ("teal", WebColor::Teal),
    ("thistle", WebColor::Thistle),
    ("tomato", WebColor::Tomato),
    ("turquoise", WebColor::Turquoise),
    ("violet", WebColor::Violet),
    ("wheat", WebColor::Wheat),
    ("whitesmoke", WebColor::WhiteSmoke),
    ("yellowgreen", WebColor::YellowGreen),
];

/// Look up a [`WebColor`] by kebab-case name (e.g. `"alice-blue"`).
///
/// Uses case-insensitive, hyphen-ignoring matching without allocation.
pub(super) fn web_color_by_name(name: &str) -> Option<WebColor> {
    WEB_COLOR_TABLE
        .iter()
        .find(|(pattern, _)| eq_ignore_case_and_hyphens(name, pattern))
        .map(|(_, wc)| *wc)
}

/// Static lookup table mapping kebab-case Tailwind names to [`Tailwind`].
static TAILWIND_COLOR_TABLE: &[(&str, Tailwind)] = &[
    ("black", Tailwind::Black),
    ("white", Tailwind::White),
    ("red-50", Tailwind::Red50),
    ("red-100", Tailwind::Red100),
    ("red-200", Tailwind::Red200),
    ("red-300", Tailwind::Red300),
    ("red-400", Tailwind::Red400),
    ("red-500", Tailwind::Red500),
    ("red-600", Tailwind::Red600),
    ("red-700", Tailwind::Red700),
    ("red-800", Tailwind::Red800),
    ("red-900", Tailwind::Red900),
    ("red-950", Tailwind::Red950),
    ("orange-50", Tailwind::Orange50),
    ("orange-100", Tailwind::Orange100),
    ("orange-200", Tailwind::Orange200),
    ("orange-300", Tailwind::Orange300),
    ("orange-400", Tailwind::Orange400),
    ("orange-500", Tailwind::Orange500),
    ("orange-600", Tailwind::Orange600),
    ("orange-700", Tailwind::Orange700),
    ("orange-800", Tailwind::Orange800),
    ("orange-900", Tailwind::Orange900),
    ("orange-950", Tailwind::Orange950),
    ("amber-50", Tailwind::Amber50),
    ("amber-100", Tailwind::Amber100),
    ("amber-200", Tailwind::Amber200),
    ("amber-300", Tailwind::Amber300),
    ("amber-400", Tailwind::Amber400),
    ("amber-500", Tailwind::Amber500),
    ("amber-600", Tailwind::Amber600),
    ("amber-700", Tailwind::Amber700),
    ("amber-800", Tailwind::Amber800),
    ("amber-900", Tailwind::Amber900),
    ("amber-950", Tailwind::Amber950),
    ("yellow-50", Tailwind::Yellow50),
    ("yellow-100", Tailwind::Yellow100),
    ("yellow-200", Tailwind::Yellow200),
    ("yellow-300", Tailwind::Yellow300),
    ("yellow-400", Tailwind::Yellow400),
    ("yellow-500", Tailwind::Yellow500),
    ("yellow-600", Tailwind::Yellow600),
    ("yellow-700", Tailwind::Yellow700),
    ("yellow-800", Tailwind::Yellow800),
    ("yellow-900", Tailwind::Yellow900),
    ("yellow-950", Tailwind::Yellow950),
    ("lime-50", Tailwind::Lime50),
    ("lime-100", Tailwind::Lime100),
    ("lime-200", Tailwind::Lime200),
    ("lime-300", Tailwind::Lime300),
    ("lime-400", Tailwind::Lime400),
    ("lime-500", Tailwind::Lime500),
    ("lime-600", Tailwind::Lime600),
    ("lime-700", Tailwind::Lime700),
    ("lime-800", Tailwind::Lime800),
    ("lime-900", Tailwind::Lime900),
    ("lime-950", Tailwind::Lime950),
    ("green-50", Tailwind::Green50),
    ("green-100", Tailwind::Green100),
    ("green-200", Tailwind::Green200),
    ("green-300", Tailwind::Green300),
    ("green-400", Tailwind::Green400),
    ("green-500", Tailwind::Green500),
    ("green-600", Tailwind::Green600),
    ("green-700", Tailwind::Green700),
    ("green-800", Tailwind::Green800),
    ("green-900", Tailwind::Green900),
    ("green-950", Tailwind::Green950),
    ("emerald-50", Tailwind::Emerald50),
    ("emerald-100", Tailwind::Emerald100),
    ("emerald-200", Tailwind::Emerald200),
    ("emerald-300", Tailwind::Emerald300),
    ("emerald-400", Tailwind::Emerald400),
    ("emerald-500", Tailwind::Emerald500),
    ("emerald-600", Tailwind::Emerald600),
    ("emerald-700", Tailwind::Emerald700),
    ("emerald-800", Tailwind::Emerald800),
    ("emerald-900", Tailwind::Emerald900),
    ("emerald-950", Tailwind::Emerald950),
    ("teal-50", Tailwind::Teal50),
    ("teal-100", Tailwind::Teal100),
    ("teal-200", Tailwind::Teal200),
    ("teal-300", Tailwind::Teal300),
    ("teal-400", Tailwind::Teal400),
    ("teal-500", Tailwind::Teal500),
    ("teal-600", Tailwind::Teal600),
    ("teal-700", Tailwind::Teal700),
    ("teal-800", Tailwind::Teal800),
    ("teal-900", Tailwind::Teal900),
    ("teal-950", Tailwind::Teal950),
    ("cyan-50", Tailwind::Cyan50),
    ("cyan-100", Tailwind::Cyan100),
    ("cyan-200", Tailwind::Cyan200),
    ("cyan-300", Tailwind::Cyan300),
    ("cyan-400", Tailwind::Cyan400),
    ("cyan-500", Tailwind::Cyan500),
    ("cyan-600", Tailwind::Cyan600),
    ("cyan-700", Tailwind::Cyan700),
    ("cyan-800", Tailwind::Cyan800),
    ("cyan-900", Tailwind::Cyan900),
    ("cyan-950", Tailwind::Cyan950),
    ("sky-50", Tailwind::Sky50),
    ("sky-100", Tailwind::Sky100),
    ("sky-200", Tailwind::Sky200),
    ("sky-300", Tailwind::Sky300),
    ("sky-400", Tailwind::Sky400),
    ("sky-500", Tailwind::Sky500),
    ("sky-600", Tailwind::Sky600),
    ("sky-700", Tailwind::Sky700),
    ("sky-800", Tailwind::Sky800),
    ("sky-900", Tailwind::Sky900),
    ("sky-950", Tailwind::Sky950),
    ("blue-50", Tailwind::Blue50),
    ("blue-100", Tailwind::Blue100),
    ("blue-200", Tailwind::Blue200),
    ("blue-300", Tailwind::Blue300),
    ("blue-400", Tailwind::Blue400),
    ("blue-500", Tailwind::Blue500),
    ("blue-600", Tailwind::Blue600),
    ("blue-700", Tailwind::Blue700),
    ("blue-800", Tailwind::Blue800),
    ("blue-900", Tailwind::Blue900),
    ("blue-950", Tailwind::Blue950),
    ("indigo-50", Tailwind::Indigo50),
    ("indigo-100", Tailwind::Indigo100),
    ("indigo-200", Tailwind::Indigo200),
    ("indigo-300", Tailwind::Indigo300),
    ("indigo-400", Tailwind::Indigo400),
    ("indigo-500", Tailwind::Indigo500),
    ("indigo-600", Tailwind::Indigo600),
    ("indigo-700", Tailwind::Indigo700),
    ("indigo-800", Tailwind::Indigo800),
    ("indigo-900", Tailwind::Indigo900),
    ("indigo-950", Tailwind::Indigo950),
    ("violet-50", Tailwind::Violet50),
    ("violet-100", Tailwind::Violet100),
    ("violet-200", Tailwind::Violet200),
    ("violet-300", Tailwind::Violet300),
    ("violet-400", Tailwind::Violet400),
    ("violet-500", Tailwind::Violet500),
    ("violet-600", Tailwind::Violet600),
    ("violet-700", Tailwind::Violet700),
    ("violet-800", Tailwind::Violet800),
    ("violet-900", Tailwind::Violet900),
    ("violet-950", Tailwind::Violet950),
    ("purple-50", Tailwind::Purple50),
    ("purple-100", Tailwind::Purple100),
    ("purple-200", Tailwind::Purple200),
    ("purple-300", Tailwind::Purple300),
    ("purple-400", Tailwind::Purple400),
    ("purple-500", Tailwind::Purple500),
    ("purple-600", Tailwind::Purple600),
    ("purple-700", Tailwind::Purple700),
    ("purple-800", Tailwind::Purple800),
    ("purple-900", Tailwind::Purple900),
    ("purple-950", Tailwind::Purple950),
    ("fuchsia-50", Tailwind::Fuchsia50),
    ("fuchsia-100", Tailwind::Fuchsia100),
    ("fuchsia-200", Tailwind::Fuchsia200),
    ("fuchsia-300", Tailwind::Fuchsia300),
    ("fuchsia-400", Tailwind::Fuchsia400),
    ("fuchsia-500", Tailwind::Fuchsia500),
    ("fuchsia-600", Tailwind::Fuchsia600),
    ("fuchsia-700", Tailwind::Fuchsia700),
    ("fuchsia-800", Tailwind::Fuchsia800),
    ("fuchsia-900", Tailwind::Fuchsia900),
    ("fuchsia-950", Tailwind::Fuchsia950),
    ("pink-50", Tailwind::Pink50),
    ("pink-100", Tailwind::Pink100),
    ("pink-200", Tailwind::Pink200),
    ("pink-300", Tailwind::Pink300),
    ("pink-400", Tailwind::Pink400),
    ("pink-500", Tailwind::Pink500),
    ("pink-600", Tailwind::Pink600),
    ("pink-700", Tailwind::Pink700),
    ("pink-800", Tailwind::Pink800),
    ("pink-900", Tailwind::Pink900),
    ("pink-950", Tailwind::Pink950),
    ("rose-50", Tailwind::Rose50),
    ("rose-100", Tailwind::Rose100),
    ("rose-200", Tailwind::Rose200),
    ("rose-300", Tailwind::Rose300),
    ("rose-400", Tailwind::Rose400),
    ("rose-500", Tailwind::Rose500),
    ("rose-600", Tailwind::Rose600),
    ("rose-700", Tailwind::Rose700),
    ("rose-800", Tailwind::Rose800),
    ("rose-900", Tailwind::Rose900),
    ("rose-950", Tailwind::Rose950),
    ("slate-50", Tailwind::Slate50),
    ("slate-100", Tailwind::Slate100),
    ("slate-200", Tailwind::Slate200),
    ("slate-300", Tailwind::Slate300),
    ("slate-400", Tailwind::Slate400),
    ("slate-500", Tailwind::Slate500),
    ("slate-600", Tailwind::Slate600),
    ("slate-700", Tailwind::Slate700),
    ("slate-800", Tailwind::Slate800),
    ("slate-900", Tailwind::Slate900),
    ("slate-950", Tailwind::Slate950),
    ("gray-50", Tailwind::Gray50),
    ("gray-100", Tailwind::Gray100),
    ("gray-200", Tailwind::Gray200),
    ("gray-300", Tailwind::Gray300),
    ("gray-400", Tailwind::Gray400),
    ("gray-500", Tailwind::Gray500),
    ("gray-600", Tailwind::Gray600),
    ("gray-700", Tailwind::Gray700),
    ("gray-800", Tailwind::Gray800),
    ("gray-900", Tailwind::Gray900),
    ("gray-950", Tailwind::Gray950),
    ("zinc-50", Tailwind::Zinc50),
    ("zinc-100", Tailwind::Zinc100),
    ("zinc-200", Tailwind::Zinc200),
    ("zinc-300", Tailwind::Zinc300),
    ("zinc-400", Tailwind::Zinc400),
    ("zinc-500", Tailwind::Zinc500),
    ("zinc-600", Tailwind::Zinc600),
    ("zinc-700", Tailwind::Zinc700),
    ("zinc-800", Tailwind::Zinc800),
    ("zinc-900", Tailwind::Zinc900),
    ("zinc-950", Tailwind::Zinc950),
    ("neutral-50", Tailwind::Neutral50),
    ("neutral-100", Tailwind::Neutral100),
    ("neutral-200", Tailwind::Neutral200),
    ("neutral-300", Tailwind::Neutral300),
    ("neutral-400", Tailwind::Neutral400),
    ("neutral-500", Tailwind::Neutral500),
    ("neutral-600", Tailwind::Neutral600),
    ("neutral-700", Tailwind::Neutral700),
    ("neutral-800", Tailwind::Neutral800),
    ("neutral-900", Tailwind::Neutral900),
    ("neutral-950", Tailwind::Neutral950),
    ("stone-50", Tailwind::Stone50),
    ("stone-100", Tailwind::Stone100),
    ("stone-200", Tailwind::Stone200),
    ("stone-300", Tailwind::Stone300),
    ("stone-400", Tailwind::Stone400),
    ("stone-500", Tailwind::Stone500),
    ("stone-600", Tailwind::Stone600),
    ("stone-700", Tailwind::Stone700),
    ("stone-800", Tailwind::Stone800),
    ("stone-900", Tailwind::Stone900),
    ("stone-950", Tailwind::Stone950),
];

/// Look up a [`Tailwind`] color by kebab-case name (e.g. `"purple-500"`).
///
/// Uses case-insensitive matching without allocation.
pub(super) fn tailwind_by_name(name: &str) -> Option<Tailwind> {
    if name.eq_ignore_ascii_case("inherit")
        || name.eq_ignore_ascii_case("current")
        || name.eq_ignore_ascii_case("transparent")
    {
        return None;
    }

    TAILWIND_COLOR_TABLE
        .iter()
        .find(|(pattern, _)| pattern.eq_ignore_ascii_case(name))
        .map(|(_, tw)| *tw)
}

/// Independent style layers tracked by [`StyleState`].
///
/// Each variant maps to a single SGR attribute group. Nested spans push/pop
/// per-layer so that closing a span restores the *parent's* value instead of
/// issuing a nuclear `\x1b[0m` reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StyleLayer {
    FontWeight,
    Foreground,
    Background,
    Italic,
    Underline,
    Strikethrough,
    Blink,
    Inverse,
    Hidden,
}

impl StyleLayer {
    /// Map a shared [`EmphasisLayer`] onto its terminal [`StyleLayer`].
    pub(super) fn from_emphasis(layer: renderable::style::EmphasisLayer) -> Self {
        use renderable::style::EmphasisLayer;
        match layer {
            EmphasisLayer::Weight => Self::FontWeight,
            EmphasisLayer::Italic => Self::Italic,
            EmphasisLayer::Underline => Self::Underline,
            EmphasisLayer::Strikethrough => Self::Strikethrough,
            EmphasisLayer::Blink => Self::Blink,
        }
    }

    /// The SGR code that clears this layer back to the terminal default.
    fn default_reset(self) -> &'static str {
        match self {
            Self::FontWeight => "\x1b[22m",
            Self::Foreground => "\x1b[39m",
            Self::Background => "\x1b[49m",
            Self::Italic => "\x1b[23m",
            Self::Underline => "\x1b[24m",
            Self::Strikethrough => "\x1b[29m",
            Self::Blink => "\x1b[25m",
            Self::Inverse => "\x1b[27m",
            Self::Hidden => "\x1b[28m",
        }
    }
}

/// Tracks the current SGR escape code for each [`StyleLayer`].
///
/// Spans call [`set`](StyleState::set) on open and
/// [`restore`](StyleState::restore) on close, so nested spans of the same
/// layer correctly restore the parent's value.
#[derive(Debug, Default)]
pub(super) struct StyleState {
    font_weight: Option<String>,
    foreground: Option<String>,
    background: Option<String>,
    italic: Option<String>,
    underline: Option<String>,
    strikethrough: Option<String>,
    blink: Option<String>,
    inverse: Option<String>,
    hidden: Option<String>,
    /// True once any style escape has been emitted.
    pub(super) used_styles: bool,
}

impl StyleState {
    fn get(&self, layer: StyleLayer) -> Option<&str> {
        let slot = match layer {
            StyleLayer::FontWeight => &self.font_weight,
            StyleLayer::Foreground => &self.foreground,
            StyleLayer::Background => &self.background,
            StyleLayer::Italic => &self.italic,
            StyleLayer::Underline => &self.underline,
            StyleLayer::Strikethrough => &self.strikethrough,
            StyleLayer::Blink => &self.blink,
            StyleLayer::Inverse => &self.inverse,
            StyleLayer::Hidden => &self.hidden,
        };
        slot.as_deref()
    }

    /// Set a layer's active escape code. Returns the previous value.
    pub(super) fn set(&mut self, layer: StyleLayer, code: &str) -> Option<String> {
        self.used_styles = true;
        let slot = self.slot_mut(layer);
        let prev = slot.take();
        *slot = Some(code.to_string());
        prev
    }

    /// Restore a layer to a previous value (typically from [`set`](Self::set)).
    pub(super) fn restore(&mut self, layer: StyleLayer, prev: Option<String>) {
        *self.slot_mut(layer) = prev;
    }

    /// The escape code to emit when closing a span on `layer`.
    ///
    /// Returns the parent's code if one exists, otherwise the layer's
    /// default reset.
    pub(super) fn close_code(&self, layer: StyleLayer) -> &str {
        self.get(layer).unwrap_or(layer.default_reset())
    }

    /// Re-emit the opening escape for every currently-active layer.
    ///
    /// Used after a hard `\x1b[0m` reset — such as the one closing a
    /// fenced code block — to restore an enclosing span's styling so
    /// following sibling text is not left unstyled.
    pub(super) fn reapply_active_layers(&self, out: &mut String) {
        for slot in [
            &self.font_weight,
            &self.foreground,
            &self.background,
            &self.italic,
            &self.underline,
            &self.strikethrough,
            &self.blink,
            &self.inverse,
            &self.hidden,
        ] {
            if let Some(code) = slot {
                out.push_str(code);
            }
        }
    }

    fn slot_mut(&mut self, layer: StyleLayer) -> &mut Option<String> {
        match layer {
            StyleLayer::FontWeight => &mut self.font_weight,
            StyleLayer::Foreground => &mut self.foreground,
            StyleLayer::Background => &mut self.background,
            StyleLayer::Italic => &mut self.italic,
            StyleLayer::Underline => &mut self.underline,
            StyleLayer::Strikethrough => &mut self.strikethrough,
            StyleLayer::Blink => &mut self.blink,
            StyleLayer::Inverse => &mut self.inverse,
            StyleLayer::Hidden => &mut self.hidden,
        }
    }
}
