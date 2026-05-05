//! Color tables, SGR escape resolution, and per-tag style action policy.
//!
//! Bridges raw tag/token names to ANSI escape sequences while honoring
//! capability-aware degradation (e.g. `<double-underline>` on terminals
//! that only advertise straight underline).

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{
    terminal::Terminal,
    utils::color::{Tailwind, WEB_COLOR_LOOKUP, WebColor},
};

/// Action returned by [`block_tag_to_escape`] describing how a block tag
/// should be emitted into the rendered prose stream.
///
/// Replaces the older `(open, close)` tuple where empty strings doubled
/// as a "suppress this tag" sentinel. The named variant lets the parser
/// branch on intent rather than re-checking string emptiness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum BlockTagAction {
    /// Wrap the inner content with `open` before and `close` after.
    Wrap {
        open: Cow<'static, str>,
        close: Cow<'static, str>,
    },
    /// Emit only the inner content with no surrounding escapes.
    ///
    /// Used when the requested style is unsupported on the current
    /// terminal (e.g. `<double-underline>` with no underline support)
    /// or when the tag carries no useful payload (e.g. `<a href="">`).
    Suppress,
}

/// Static lookup table for atomic tokens.
///
/// Using a static array with `eq_ignore_ascii_case` avoids per-token allocations
/// from `to_lowercase()`.
static ATOMIC_TOKEN_TABLE: &[(&str, &str)] = &[
    // Text styles
    ("bold", "\x1b[1m"),
    ("dim", "\x1b[2m"),
    ("italic", "\x1b[3m"),
    ("underline", "\x1b[4m"),
    ("double-underline", "\x1b[4:2m"),
    ("curly-underline", "\x1b[4:3m"),
    ("dotted-underline", "\x1b[4:4m"),
    ("dashed-underline", "\x1b[4:5m"),
    ("blink", "\x1b[5m"),
    ("reverse", "\x1b[7m"),
    ("hidden", "\x1b[8m"),
    ("strikethrough", "\x1b[9m"),
    // Reset codes
    ("reset", "\x1b[0m"),
    ("reset-fg", "\x1b[39m"),
    ("reset-bg", "\x1b[49m"),
    // Resets foreground color and all text decorations but preserves background.
    // Equivalent to: normal-font-weight + not-italic + not-underline + not-blink
    //                + not-inverse + not-hidden + not-strikethrough + reset-fg
    ("reset-style", "\x1b[22;23;24;25;27;28;29;39m"),
    // Style-specific reset tokens (kebab-case standard)
    ("normal-font-weight", "\x1b[22m"), // Resets bold and dim
    ("not-italic", "\x1b[23m"),
    ("not-underline", "\x1b[24m"),
    ("not-blink", "\x1b[25m"),
    ("not-inverse", "\x1b[27m"),
    ("not-hidden", "\x1b[28m"),
    ("not-strikethrough", "\x1b[29m"),
    // Basic foreground colors
    ("black", "\x1b[30m"),
    ("red", "\x1b[31m"),
    ("green", "\x1b[32m"),
    ("yellow", "\x1b[33m"),
    ("blue", "\x1b[34m"),
    ("magenta", "\x1b[35m"),
    ("cyan", "\x1b[36m"),
    ("white", "\x1b[37m"),
    // Bright foreground colors
    ("bright-black", "\x1b[90m"),
    ("bright-red", "\x1b[91m"),
    ("bright-green", "\x1b[92m"),
    ("bright-yellow", "\x1b[93m"),
    ("bright-blue", "\x1b[94m"),
    ("bright-magenta", "\x1b[95m"),
    ("bright-cyan", "\x1b[96m"),
    ("bright-white", "\x1b[97m"),
    // Basic background colors
    ("bg-black", "\x1b[40m"),
    ("bg-red", "\x1b[41m"),
    ("bg-green", "\x1b[42m"),
    ("bg-yellow", "\x1b[43m"),
    ("bg-blue", "\x1b[44m"),
    ("bg-magenta", "\x1b[45m"),
    ("bg-cyan", "\x1b[46m"),
    ("bg-white", "\x1b[47m"),
    // Bright background colors
    ("bg-bright-black", "\x1b[100m"),
    ("bg-bright-red", "\x1b[101m"),
    ("bg-bright-green", "\x1b[102m"),
    ("bg-bright-yellow", "\x1b[103m"),
    ("bg-bright-blue", "\x1b[104m"),
    ("bg-bright-magenta", "\x1b[105m"),
    ("bg-bright-cyan", "\x1b[106m"),
    ("bg-bright-white", "\x1b[107m"),
];

/// Convert an atomic token name to its ANSI escape code.
///
/// Uses `eq_ignore_ascii_case` for case-insensitive matching without allocation.
pub(super) fn atomic_token_to_escape(token: &str) -> Option<&'static str> {
    ATOMIC_TOKEN_TABLE
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(token))
        .map(|(_, escape)| *escape)
}

// TODO(apple-terminal-followup): generalize to `degraded_underline_open(term, UnderlineKind)` when the curly/dotted/dashed work at the `block_tag_to_escape` TODO (see `prose.rs` ~L375-381) lands.
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
///   suppress the underline entirely (no SGR at all, including no
///   `\x1b[0m` reset — `state.used_styles` must remain unchanged).
///
/// ## Notes
///
/// The closing SGR for any non-`None` return is always `"\x1b[24m"` and
/// is intentionally not returned by this helper.
///
/// [`UnderlineSupport::double`]: crate::discovery::detection::UnderlineSupport::double
/// [`UnderlineSupport::straight`]: crate::discovery::detection::UnderlineSupport::straight
fn degraded_double_underline_open(term: Option<&Terminal>) -> Option<&'static str> {
    match term {
        None => Some("\x1b[4:2m"),
        Some(t) if t.underline_support.double => Some("\x1b[4:2m"),
        Some(t) if t.underline_support.straight => Some("\x1b[4m"),
        Some(_) => None,
    }
}

/// Capability-aware variant of [`atomic_token_to_escape`].
///
/// Returns the same escape as `atomic_token_to_escape` for every token
/// **except** `double-underline`, which is routed through the same
/// degradation policy as the `<double-underline>` block tag:
///
/// - No terminal context (`term == None`) — optimistic `\x1b[4:2m`.
/// - Terminal advertises `underline_support.double` — `\x1b[4:2m`.
/// - Terminal advertises only `underline_support.straight` —
///   degrade to `\x1b[4m`.
/// - Neither supported — `None` (the parser drops the token entirely).
///
/// Wraps `ATOMIC_TOKEN_TABLE` so the static lookup stays the source of
/// truth for the non-degrading tokens.
pub(super) fn atomic_token_to_escape_with_term(
    token: &str,
    term: Option<&Terminal>,
) -> Option<Cow<'static, str>> {
    if token.eq_ignore_ascii_case("double-underline") {
        return degraded_double_underline_open(term).map(Cow::Borrowed);
    }

    atomic_token_to_escape(token).map(Cow::Borrowed)
}

/// Convert a block tag to its opening and closing ANSI escape codes.
///
/// TODO: `Prose` tag styling and `utils::styling::Style`/`Stylist` should
/// eventually converge so capability-aware degradation lives in one place
/// instead of being duplicated between the prose parser and the styling
/// helpers.
// TODO(apple-terminal-followup): make `curly-underline`, `dotted-underline`,
// and `dashed-underline` capability-aware in the same way `double-underline`
// is. `UnderlineSupport` already exposes `curly`, `dotted`, and `dashed`
// booleans; the atomic and block tag handlers should consult them and fall
// back to single underline (or plain text) when unsupported. Scoped out of
// the 2026-05-02 Apple Terminal feature — see
// features/2026-05-02-apple-terminal/spec.md.
pub(super) fn block_tag_to_escape(
    tag_name: &str,
    attrs: &[(String, String)],
    term: Option<&Terminal>,
) -> Option<BlockTagAction> {
    /// Helper: build a `Wrap` action with two static-string escapes.
    fn wrap_static(open: &'static str, close: &'static str) -> BlockTagAction {
        BlockTagAction::Wrap {
            open: Cow::Borrowed(open),
            close: Cow::Borrowed(close),
        }
    }

    match tag_name {
        // Text styles (full names + short aliases)
        "bold" | "b" => Some(wrap_static("\x1b[1m", "\x1b[22m")),
        "dim" => Some(wrap_static("\x1b[2m", "\x1b[22m")),
        "italic" | "i" => Some(wrap_static("\x1b[3m", "\x1b[23m")),
        "underline" | "u" => Some(wrap_static("\x1b[4m", "\x1b[24m")),
        "double-underline" | "uu" => {
            // Capability-aware degradation:
            //   - No terminal context: optimistic `\x1b[4:2m` (legacy behavior).
            //   - Double underline supported: `\x1b[4:2m`.
            //   - Only straight underline supported: degrade to `\x1b[4m`.
            //   - Neither supported: suppress the underline entirely.
            match degraded_double_underline_open(term) {
                Some(open) => Some(wrap_static(open, "\x1b[24m")),
                None => Some(BlockTagAction::Suppress),
            }
        }
        "curly-underline" => Some(wrap_static("\x1b[4:3m", "\x1b[24m")),
        "dotted-underline" => Some(wrap_static("\x1b[4:4m", "\x1b[24m")),
        "dashed-underline" => Some(wrap_static("\x1b[4:5m", "\x1b[24m")),
        "blink" => Some(wrap_static("\x1b[5m", "\x1b[25m")),
        "inverse" | "reverse" => Some(wrap_static("\x1b[7m", "\x1b[27m")),
        "hidden" => Some(wrap_static("\x1b[8m", "\x1b[28m")),
        "strikethrough" | "~" => Some(wrap_static("\x1b[9m", "\x1b[29m")),

        // OSC8 hyperlinks
        "a" => {
            let href = attrs
                .iter()
                .find(|(k, _)| k == "href")
                .map(|(_, v)| v.as_str())
                .unwrap_or("");

            // Resolve the href (handles relative paths)
            let resolved_href = resolve_href(href);

            // Check if terminal supports OSC8 (default true when no context).
            let supports_osc8 = term.map(|t| t.osc_link_support).unwrap_or(true);
            if resolved_href.is_empty() {
                // No href: just show the content with no link wrapping.
                Some(BlockTagAction::Suppress)
            } else if supports_osc8 {
                Some(BlockTagAction::Wrap {
                    open: Cow::Owned(format!("\x1b]8;;{}\x1b\\", resolved_href)),
                    close: Cow::Borrowed("\x1b]8;;\x1b\\"),
                })
            } else {
                // Markdown fallback: `[description](resolved_href)`.
                //
                // The opening bracket is a static `Cow::Borrowed` to
                // avoid a per-tag `String` allocation; only the close
                // string folds the resolved href into `](url)` because
                // the structural emit pattern is `open + inner + close`.
                Some(BlockTagAction::Wrap {
                    open: Cow::Borrowed("["),
                    close: Cow::Owned(format!("]({})", resolved_href)),
                })
            }
        }

        // RGB colors
        "rgb" => {
            // Parse RGB from attrs like "rgb 125,67,45"
            // The RGB value appears as an attr with the value in the key and empty value,
            // e.g., attrs = [("125,67,45", "")]
            let rgb_str = attrs
                .iter()
                .find(|(_, v)| v.is_empty())
                .map(|(k, _)| k.as_str())
                .unwrap_or("");

            parse_rgb(rgb_str).map(|(r, g, b)| BlockTagAction::Wrap {
                open: Cow::Owned(format!("\x1b[38;2;{};{};{}m", r, g, b)),
                close: Cow::Borrowed("\x1b[39m"),
            })
        }

        // Background RGB colors
        "bg-rgb" => {
            let rgb_str = attrs
                .iter()
                .find(|(_, v)| v.is_empty())
                .map(|(k, _)| k.as_str())
                .unwrap_or("");

            parse_rgb(rgb_str).map(|(r, g, b)| BlockTagAction::Wrap {
                open: Cow::Owned(format!("\x1b[48;2;{};{};{}m", r, g, b)),
                close: Cow::Borrowed("\x1b[49m"),
            })
        }

        // Basic foreground colors
        "black" => Some(wrap_static("\x1b[30m", "\x1b[39m")),
        "red" => Some(wrap_static("\x1b[31m", "\x1b[39m")),
        "green" => Some(wrap_static("\x1b[32m", "\x1b[39m")),
        "yellow" => Some(wrap_static("\x1b[33m", "\x1b[39m")),
        "blue" => Some(wrap_static("\x1b[34m", "\x1b[39m")),
        "magenta" => Some(wrap_static("\x1b[35m", "\x1b[39m")),
        "cyan" => Some(wrap_static("\x1b[36m", "\x1b[39m")),
        "white" => Some(wrap_static("\x1b[37m", "\x1b[39m")),

        // Bright foreground colors
        "bright-black" => Some(wrap_static("\x1b[90m", "\x1b[39m")),
        "bright-red" => Some(wrap_static("\x1b[91m", "\x1b[39m")),
        "bright-green" => Some(wrap_static("\x1b[92m", "\x1b[39m")),
        "bright-yellow" => Some(wrap_static("\x1b[93m", "\x1b[39m")),
        "bright-blue" => Some(wrap_static("\x1b[94m", "\x1b[39m")),
        "bright-magenta" => Some(wrap_static("\x1b[95m", "\x1b[39m")),
        "bright-cyan" => Some(wrap_static("\x1b[96m", "\x1b[39m")),
        "bright-white" => Some(wrap_static("\x1b[97m", "\x1b[39m")),

        // Clipboard - actual clipboard handling would be done externally;
        // emit only the inner content with no surrounding escapes.
        "clipboard" => Some(BlockTagAction::Suppress),

        // Try web colors, then Tailwind colors (foreground and background)
        _ => {
            // Check for bg- prefix for background colors
            if let Some(color_name) = tag_name.strip_prefix("bg-") {
                // Try web color lookup for background
                if let Some(rgb) = lookup_web_color(color_name) {
                    return Some(BlockTagAction::Wrap {
                        open: Cow::Owned(format!(
                            "\x1b[48;2;{};{};{}m",
                            rgb.red(),
                            rgb.green(),
                            rgb.blue()
                        )),
                        close: Cow::Borrowed("\x1b[49m"),
                    });
                }

                // Try Tailwind color lookup for background
                if let Some(hdr) = lookup_tailwind_color(color_name) {
                    return Some(BlockTagAction::Wrap {
                        open: Cow::Owned(format!("\x1b[48;2;{};{};{}m", hdr.0, hdr.1, hdr.2)),
                        close: Cow::Borrowed("\x1b[49m"),
                    });
                }
            }

            // Try web color lookup (kebab-case like "alice-blue")
            if let Some(rgb) = lookup_web_color(tag_name) {
                return Some(BlockTagAction::Wrap {
                    open: Cow::Owned(format!(
                        "\x1b[38;2;{};{};{}m",
                        rgb.red(),
                        rgb.green(),
                        rgb.blue()
                    )),
                    close: Cow::Borrowed("\x1b[39m"),
                });
            }

            // Try Tailwind color lookup (kebab-case like "purple-500")
            if let Some(hdr) = lookup_tailwind_color(tag_name) {
                return Some(BlockTagAction::Wrap {
                    open: Cow::Owned(format!("\x1b[38;2;{};{};{}m", hdr.0, hdr.1, hdr.2)),
                    close: Cow::Borrowed("\x1b[39m"),
                });
            }

            None
        }
    }
}

/// Parse an RGB string in multiple formats into (r, g, b).
///
/// Supported formats:
/// - Comma-separated: "125,67,45"
/// - Space-separated: "125 67 45"
/// - Hex with #: "#8B0000"
/// - Hex without #: "8B0000"
fn parse_rgb(s: &str) -> Option<(u8, u8, u8)> {
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
///
/// Returns the first valid base from:
/// 1. Package root (for monorepos: directory containing Cargo.toml closest to CWD)
/// 2. Git repository root
fn find_git_relative_base() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;

    // First, try to find git repo root
    let git_root = find_git_root(&cwd)?;

    // Check if this is a monorepo by looking for Cargo.toml between CWD and git root
    if let Some(package_root) = find_package_root(&cwd, &git_root) {
        // Verify this isn't the repo root itself (which would be the workspace Cargo.toml)
        if package_root != git_root {
            return Some(package_root);
        }
    }

    // Not a monorepo or no package found, use git root
    Some(git_root)
}

/// Find the git repository root starting from the given path.
fn find_git_root(start: &Path) -> Option<PathBuf> {
    // Try using git command for accuracy
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(start)
        .output()
        .ok()?;

    if output.status.success() {
        let path_str = String::from_utf8(output.stdout).ok()?;
        return Some(PathBuf::from(path_str.trim()));
    }

    // Fallback: walk up looking for .git directory
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
        if cargo_toml.exists() {
            // Check if this is a package Cargo.toml (not just a workspace)
            // A simple heuristic: if it contains [package], it's a package
            if let Ok(contents) = std::fs::read_to_string(&cargo_toml)
                && contents.contains("[package]")
            {
                return Some(current);
            }
        }

        // Stop if we've reached or passed the git root
        if current == git_root || !current.starts_with(git_root) {
            break;
        }

        if !current.pop() {
            break;
        }
    }

    None
}

/// Compare two strings case-insensitively, skipping hyphens in `input`.
///
/// This allows matching "alice-blue" against "aliceblue" without allocation.
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

/// Look up a web color by kebab-case name (e.g., "alice-blue").
///
/// Returns the RgbColor if found.
/// Uses case-insensitive matching without allocation.
fn lookup_web_color(name: &str) -> Option<crate::utils::color::RgbColor> {
    // Static lookup table for web colors - avoids allocation from to_lowercase().replace()
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

    // Use case-insensitive hyphen-ignoring lookup
    WEB_COLOR_TABLE
        .iter()
        .find(|(pattern, _)| eq_ignore_case_and_hyphens(name, pattern))
        .and_then(|(_, wc)| WEB_COLOR_LOOKUP.get(wc).copied())
}

/// Static lookup table for Tailwind colors.
///
/// Using a static array with `eq_ignore_ascii_case` avoids per-lookup allocations.
static TAILWIND_COLOR_TABLE: &[(&str, Tailwind)] = &[
    // Black/White
    ("black", Tailwind::Black),
    ("white", Tailwind::White),
    // Red
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
    // Orange
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
    // Amber
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
    // Yellow
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
    // Lime
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
    // Green
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
    // Emerald
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
    // Teal
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
    // Cyan
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
    // Sky
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
    // Blue
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
    // Indigo
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
    // Violet
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
    // Purple
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
    // Fuchsia
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
    // Pink
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
    // Rose
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
    // Slate
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
    // Gray
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
    // Zinc
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
    // Neutral
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
    // Stone
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

/// Look up a Tailwind color by kebab-case name (e.g., "purple-500").
///
/// Returns the RGB tuple if found.
/// Uses case-insensitive matching without allocation.
fn lookup_tailwind_color(name: &str) -> Option<(u8, u8, u8)> {
    // Special values that should return None
    if name.eq_ignore_ascii_case("inherit")
        || name.eq_ignore_ascii_case("current")
        || name.eq_ignore_ascii_case("transparent")
    {
        return None;
    }

    TAILWIND_COLOR_TABLE
        .iter()
        .find(|(pattern, _)| pattern.eq_ignore_ascii_case(name))
        .and_then(|(_, tw)| tw.to_hdr_color())
        .map(|hdr| (hdr.red(), hdr.green(), hdr.blue()))
}

/// Independent style layers tracked by [`StyleState`].
///
/// Each variant maps to a single SGR attribute group. Block tags push/pop
/// per-layer so that closing a tag restores the *parent's* value instead of
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
/// Block tags call [`set`](StyleState::set) on open and
/// [`restore`](StyleState::restore) on close, so nested tags of the same
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

    /// The escape code to emit when closing a block tag on `layer`.
    ///
    /// Returns the parent's code if one exists, otherwise the layer's
    /// default reset.
    pub(super) fn close_code(&self, layer: StyleLayer) -> &str {
        self.get(layer).unwrap_or(layer.default_reset())
    }

    /// Clear all layers (used by `{{reset}}`).
    pub(super) fn clear_all(&mut self) {
        self.font_weight = None;
        self.foreground = None;
        self.background = None;
        self.italic = None;
        self.underline = None;
        self.strikethrough = None;
        self.blink = None;
        self.inverse = None;
        self.hidden = None;
    }

    /// Clear all layers except background (used by `{{reset-style}}`).
    pub(super) fn clear_all_except_background(&mut self) {
        self.font_weight = None;
        self.foreground = None;
        // background intentionally preserved
        self.italic = None;
        self.underline = None;
        self.strikethrough = None;
        self.blink = None;
        self.inverse = None;
        self.hidden = None;
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

/// Map a block tag name to its style layer.
///
/// Returns `None` for structural tags (`a`, `clipboard`) that don't
/// correspond to an SGR attribute.
pub(super) fn block_tag_layer(tag_name: &str) -> Option<StyleLayer> {
    match tag_name {
        "bold" | "b" | "dim" => Some(StyleLayer::FontWeight),
        "italic" | "i" => Some(StyleLayer::Italic),
        "underline" | "u" | "double-underline" | "uu" | "curly-underline" | "dotted-underline"
        | "dashed-underline" => Some(StyleLayer::Underline),
        "blink" => Some(StyleLayer::Blink),
        "inverse" | "reverse" => Some(StyleLayer::Inverse),
        "hidden" => Some(StyleLayer::Hidden),
        "strikethrough" | "~" => Some(StyleLayer::Strikethrough),
        // Structural tags — no SGR layer
        "a" | "clipboard" => None,
        // Named foreground colors + rgb
        "rgb" | "black" | "red" | "green" | "yellow" | "blue" | "magenta" | "cyan" | "white"
        | "bright-black" | "bright-red" | "bright-green" | "bright-yellow" | "bright-blue"
        | "bright-magenta" | "bright-cyan" | "bright-white" => Some(StyleLayer::Foreground),
        // Background rgb
        "bg-rgb" => Some(StyleLayer::Background),
        // Catch-all for web/tailwind colors
        _ => {
            if tag_name.starts_with("bg-") {
                Some(StyleLayer::Background)
            } else {
                // Safe default for web/tailwind foreground colors.
                // If block_tag_to_escape returns None the layer is never used.
                Some(StyleLayer::Foreground)
            }
        }
    }
}

/// Classify an atomic token for layer tracking.
///
/// Returns `Some((layer, true))` for set-tokens (e.g. `bold`, `red`),
/// `Some((layer, false))` for clear-tokens (e.g. `normal-font-weight`),
/// and `None` for `reset`/`reset-style` (handled by the caller) or
/// unknown tokens.
///
/// Expects a **lowercased** token string.
pub(super) fn atomic_token_layer(token: &str) -> Option<(StyleLayer, bool)> {
    match token {
        // --- setters ---
        "bold" | "dim" => Some((StyleLayer::FontWeight, true)),
        "italic" => Some((StyleLayer::Italic, true)),
        "underline" | "double-underline" | "curly-underline" | "dotted-underline"
        | "dashed-underline" => Some((StyleLayer::Underline, true)),
        "blink" => Some((StyleLayer::Blink, true)),
        "reverse" => Some((StyleLayer::Inverse, true)),
        "hidden" => Some((StyleLayer::Hidden, true)),
        "strikethrough" => Some((StyleLayer::Strikethrough, true)),
        // Foreground colors
        "black" | "red" | "green" | "yellow" | "blue" | "magenta" | "cyan" | "white"
        | "bright-black" | "bright-red" | "bright-green" | "bright-yellow" | "bright-blue"
        | "bright-magenta" | "bright-cyan" | "bright-white" => Some((StyleLayer::Foreground, true)),
        // Background colors
        "bg-black" | "bg-red" | "bg-green" | "bg-yellow" | "bg-blue" | "bg-magenta" | "bg-cyan"
        | "bg-white" | "bg-bright-black" | "bg-bright-red" | "bg-bright-green"
        | "bg-bright-yellow" | "bg-bright-blue" | "bg-bright-magenta" | "bg-bright-cyan"
        | "bg-bright-white" => Some((StyleLayer::Background, true)),
        // --- clearers ---
        "normal-font-weight" => Some((StyleLayer::FontWeight, false)),
        "not-italic" => Some((StyleLayer::Italic, false)),
        "not-underline" => Some((StyleLayer::Underline, false)),
        "not-blink" => Some((StyleLayer::Blink, false)),
        "not-inverse" => Some((StyleLayer::Inverse, false)),
        "not-hidden" => Some((StyleLayer::Hidden, false)),
        "not-strikethrough" => Some((StyleLayer::Strikethrough, false)),
        "reset-fg" => Some((StyleLayer::Foreground, false)),
        "reset-bg" => Some((StyleLayer::Background, false)),
        // reset / reset-style → caller handles via clear_all / clear_all_except_background
        _ => None,
    }
}
