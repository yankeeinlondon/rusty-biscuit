use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{
    components::renderable::Renderable,
    terminal::Terminal,
    utils::color::{Tailwind, WEB_COLOR_LOOKUP, WebColor},
    utils::layout::{Layout, Margin, WordWrap},
};

/// Prose content allows plain text to be passed in and that content will be parsed
/// for two kinds of tokens:
///
/// ## Atomic Tokens
///
/// Atomic tokens will be of the form `{{token}}` and the prose
/// parser does a simple lookup table on the atomic token and
/// replaces it with an escape code.
///
/// Examples include:
///
/// - `{{bold}}`, `{{dim}}`
/// - `{{italic}}`, `{{underline}}`, `{{strikethrough}}`
/// - `{{double-underline}}`, `{{curly-underline}}`, `{{dotted-underline}}`, `{{dashed-underline}}`
/// - `{{red}}`, `{{blue}}`, `{{bright-red}}`, etc.
/// - `{{bg-red}}`, `{{bg-blue}}`, etc.
/// - `{{reset}}`, `{{reset-fg}}`, `{{reset-bg}}`
/// - `{{normal-font-weight}}`, `{{not-italic}}`, `{{not-underline}}`, `{{not-strikethrough}}`
///
/// The key characteristic of these atomic tokens is that they don't clean up
/// after themselves and require the caller to use the `{{reset}}` token whenever
/// they want to return to a known/default state.
///
/// **Note:** a `{{reset}}` is _always_ added to the end of a prose section which
/// has used at least one atomic token. This is just to be sure that styles do not
/// bleed out.
///
/// ## Block Tokens
///
/// Block tokens use an _HTML-like_ syntax but are really just a tiny subset of HTML's
/// vast catalog of tags. A block tag, in contrast to an atomic token, has a clear
/// start and stop token and like HTML we use the nomenclature of `<tag>content</tag>`.
///
/// Supported block tokens are:
///
/// - `<bold>` or `<b>` for bold text
/// - `<dim>` for dim text
/// - `<italic>` or `<i>` for italic text
/// - `<underline>` or `<u>` for underlined text
/// - `<double-underline>` or `<uu>` for double-underlined text
/// - `<curly-underline>` for curly-underlined text
/// - `<dotted-underline>` for dotted-underlined text
/// - `<dashed-underline>` for dashed-underlined text
/// - `<blink>` for blinking text
/// - `<inverse>` or `<reverse>` for inverse video
/// - `<hidden>` for hidden text
/// - `<strikethrough>` or `<~>` for strikethrough content
/// - `<a href="...">content</a>` for an OSC8 link to a file or URL
/// - `<rgb 125,67,45>content</rgb>` for RGB colored foreground text (comma-separated)
/// - `<rgb 125 67 45>content</rgb>` for RGB colored foreground text (space-separated)
/// - `<rgb #8B0000>content</rgb>` for RGB colored foreground text (hex with #)
/// - `<rgb 8B0000>content</rgb>` for RGB colored foreground text (hex without #)
/// - `<red>content</red>` for named color foreground text
/// - `<bg-rgb 125,67,45>content</bg-rgb>` for RGB colored background text (comma-separated)
/// - `<bg-rgb 125 67 45>content</bg-rgb>` for RGB colored background text (space-separated)
/// - `<bg-rgb #8B0000>content</bg-rgb>` for RGB colored background text (hex with #)
/// - `<bg-coral>content</bg-coral>` for named color (web) background text
/// - `<bg-red-800>content</bg-red-800>` for Tailwind color background text
/// - `<clipboard>fallback</clipboard>` injects clipboard content or fallback
///
#[derive(Debug, Clone)]
pub struct Prose {
    /// the raw content as received
    content: String,
    /// Layout configuration for margins, alignment, word wrap, etc.
    layout: Layout,
}

impl Prose {
    /// Create a new Prose instance with the given content.
    pub fn new<T: Into<String>>(content: T) -> Self {
        Prose {
            content: content.into(),
            layout: Layout::default(),
        }
    }

    /// Returns the raw content as received.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Set the word wrap strategy.
    pub fn with_word_wrap(mut self, wrap: WordWrap) -> Self {
        self.layout.word_wrap = wrap;
        self
    }

    /// Set the left margin.
    pub fn with_left_margin(mut self, margin: Margin) -> Self {
        self.layout.left_margin = margin;
        self
    }

    /// Set the right margin.
    pub fn with_right_margin(mut self, margin: Margin) -> Self {
        self.layout.right_margin = margin;
        self
    }

    /// Parse and render the content, replacing tokens with ANSI escape codes.
    ///
    /// Creates a fresh [`StyleState`] and delegates to [`parse_tokens_inner`].
    /// Only this outermost call emits the final `\x1b[0m` reset.
    fn parse_tokens(&self, term: Option<&Terminal>) -> String {
        let mut state = StyleState::default();
        let mut result = parse_tokens_inner(&self.content, term, &mut state);
        if state.used_styles {
            result.push_str("\x1b[0m");
        }
        result
    }
}

impl Default for Prose {
    fn default() -> Prose {
        Prose {
            content: "".to_string(),
            layout: Layout::default(),
        }
    }
}

impl Renderable for Prose {
    fn render(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let parsed = self.parse_tokens(None);
        self.layout.apply_layout(&parsed, width)
    }

    fn fallback_render(&self, term: &Terminal) -> String {
        let width = term.width();
        let parsed = self.parse_tokens(Some(term));
        self.layout.apply_layout(&parsed, width)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }
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
fn atomic_token_to_escape(token: &str) -> Option<&'static str> {
    ATOMIC_TOKEN_TABLE
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(token))
        .map(|(_, escape)| *escape)
}

/// Parse an opening tag into its name and attributes.
fn parse_opening_tag(tag_content: &str) -> Option<(String, Vec<(String, String)>)> {
    let tag_content = tag_content.trim();
    if tag_content.is_empty() {
        return None;
    }

    let parts: Vec<&str> = tag_content.splitn(2, |c: char| c.is_whitespace()).collect();
    let tag_name = parts[0].to_lowercase();

    let mut attrs = Vec::new();
    if parts.len() > 1 {
        // Parse attributes
        let attr_str = parts[1];

        // Check if there's any '=' in the attribute string
        // If not, treat the entire remainder as a positional value (empty key)
        if !attr_str.contains('=') {
            let value = attr_str.trim();
            if !value.is_empty() {
                attrs.push((value.to_string(), String::new()));
            }
        } else {
            // Original parsing for key=value attributes
            let mut current_attr = String::new();
            let mut current_value = String::new();
            let mut in_value = false;
            let mut quote_char: Option<char> = None;

            for c in attr_str.chars() {
                if in_value {
                    if let Some(qc) = quote_char {
                        if c == qc {
                            attrs.push((current_attr.clone(), current_value.clone()));
                            current_attr.clear();
                            current_value.clear();
                            in_value = false;
                            quote_char = None;
                        } else {
                            current_value.push(c);
                        }
                    } else if c == '"' || c == '\'' {
                        quote_char = Some(c);
                    } else if c.is_whitespace() {
                        if !current_value.is_empty() {
                            attrs.push((current_attr.clone(), current_value.clone()));
                            current_attr.clear();
                            current_value.clear();
                        }
                        in_value = false;
                    } else {
                        current_value.push(c);
                    }
                } else if c == '=' {
                    in_value = true;
                } else if !c.is_whitespace() {
                    current_attr.push(c);
                }
            }

            // Handle last attribute without closing quote
            if !current_attr.is_empty() || !current_value.is_empty() {
                attrs.push((current_attr, current_value));
            }
        }
    }

    Some((tag_name, attrs))
}

/// Convert a block tag to its opening and closing ANSI escape codes.
fn block_tag_to_escape(
    tag_name: &str,
    attrs: &[(String, String)],
    term: Option<&Terminal>,
) -> Option<(String, String)> {
    match tag_name {
        // Text styles (full names + short aliases)
        "bold" | "b" => Some(("\x1b[1m".to_string(), "\x1b[22m".to_string())),
        "dim" => Some(("\x1b[2m".to_string(), "\x1b[22m".to_string())),
        "italic" | "i" => Some(("\x1b[3m".to_string(), "\x1b[23m".to_string())),
        "underline" | "u" => Some(("\x1b[4m".to_string(), "\x1b[24m".to_string())),
        "double-underline" | "uu" => Some(("\x1b[4:2m".to_string(), "\x1b[24m".to_string())),
        "curly-underline" => Some(("\x1b[4:3m".to_string(), "\x1b[24m".to_string())),
        "dotted-underline" => Some(("\x1b[4:4m".to_string(), "\x1b[24m".to_string())),
        "dashed-underline" => Some(("\x1b[4:5m".to_string(), "\x1b[24m".to_string())),
        "blink" => Some(("\x1b[5m".to_string(), "\x1b[25m".to_string())),
        "inverse" | "reverse" => Some(("\x1b[7m".to_string(), "\x1b[27m".to_string())),
        "hidden" => Some(("\x1b[8m".to_string(), "\x1b[28m".to_string())),
        "strikethrough" | "~" => Some(("\x1b[9m".to_string(), "\x1b[29m".to_string())),

        // OSC8 hyperlinks
        "a" => {
            let href = attrs
                .iter()
                .find(|(k, _)| k == "href")
                .map(|(_, v)| v.as_str())
                .unwrap_or("");

            // Resolve the href (handles relative paths)
            let resolved_href = resolve_href(href);

            // Check if terminal supports OSC8
            let supports_osc8 = term.map(|t| t.osc_link_support).unwrap_or(true);
            if supports_osc8 && !resolved_href.is_empty() {
                Some((
                    format!("\x1b]8;;{}\x1b\\", resolved_href),
                    "\x1b]8;;\x1b\\".to_string(),
                ))
            } else {
                // Fallback: just show the content (no link)
                Some((String::new(), String::new()))
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

            if let Some((r, g, b)) = parse_rgb(rgb_str) {
                Some((
                    format!("\x1b[38;2;{};{};{}m", r, g, b),
                    "\x1b[39m".to_string(),
                ))
            } else {
                None
            }
        }

        // Background RGB colors
        "bg-rgb" => {
            let rgb_str = attrs
                .iter()
                .find(|(_, v)| v.is_empty())
                .map(|(k, _)| k.as_str())
                .unwrap_or("");

            if let Some((r, g, b)) = parse_rgb(rgb_str) {
                Some((
                    format!("\x1b[48;2;{};{};{}m", r, g, b),
                    "\x1b[49m".to_string(),
                ))
            } else {
                None
            }
        }

        // Basic foreground colors
        "black" => Some(("\x1b[30m".to_string(), "\x1b[39m".to_string())),
        "red" => Some(("\x1b[31m".to_string(), "\x1b[39m".to_string())),
        "green" => Some(("\x1b[32m".to_string(), "\x1b[39m".to_string())),
        "yellow" => Some(("\x1b[33m".to_string(), "\x1b[39m".to_string())),
        "blue" => Some(("\x1b[34m".to_string(), "\x1b[39m".to_string())),
        "magenta" => Some(("\x1b[35m".to_string(), "\x1b[39m".to_string())),
        "cyan" => Some(("\x1b[36m".to_string(), "\x1b[39m".to_string())),
        "white" => Some(("\x1b[37m".to_string(), "\x1b[39m".to_string())),

        // Bright foreground colors
        "bright-black" => Some(("\x1b[90m".to_string(), "\x1b[39m".to_string())),
        "bright-red" => Some(("\x1b[91m".to_string(), "\x1b[39m".to_string())),
        "bright-green" => Some(("\x1b[92m".to_string(), "\x1b[39m".to_string())),
        "bright-yellow" => Some(("\x1b[93m".to_string(), "\x1b[39m".to_string())),
        "bright-blue" => Some(("\x1b[94m".to_string(), "\x1b[39m".to_string())),
        "bright-magenta" => Some(("\x1b[95m".to_string(), "\x1b[39m".to_string())),
        "bright-cyan" => Some(("\x1b[96m".to_string(), "\x1b[39m".to_string())),
        "bright-white" => Some(("\x1b[97m".to_string(), "\x1b[39m".to_string())),

        // Clipboard - returns empty escapes, actual clipboard handling would be done externally
        "clipboard" => Some((String::new(), String::new())),

        // Try web colors, then Tailwind colors (foreground and background)
        _ => {
            // Check for bg- prefix for background colors
            if let Some(color_name) = tag_name.strip_prefix("bg-") {
                // Try web color lookup for background
                if let Some(rgb) = lookup_web_color(color_name) {
                    return Some((
                        format!("\x1b[48;2;{};{};{}m", rgb.red(), rgb.green(), rgb.blue()),
                        "\x1b[49m".to_string(),
                    ));
                }

                // Try Tailwind color lookup for background
                if let Some(hdr) = lookup_tailwind_color(color_name) {
                    return Some((
                        format!("\x1b[48;2;{};{};{}m", hdr.0, hdr.1, hdr.2),
                        "\x1b[49m".to_string(),
                    ));
                }
            }

            // Try web color lookup (kebab-case like "alice-blue")
            if let Some(rgb) = lookup_web_color(tag_name) {
                return Some((
                    format!("\x1b[38;2;{};{};{}m", rgb.red(), rgb.green(), rgb.blue()),
                    "\x1b[39m".to_string(),
                ));
            }

            // Try Tailwind color lookup (kebab-case like "purple-500")
            if let Some(hdr) = lookup_tailwind_color(tag_name) {
                return Some((
                    format!("\x1b[38;2;{};{};{}m", hdr.0, hdr.1, hdr.2),
                    "\x1b[39m".to_string(),
                ));
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
fn resolve_href(href: &str) -> String {
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
enum StyleLayer {
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
struct StyleState {
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
    used_styles: bool,
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
    fn set(&mut self, layer: StyleLayer, code: &str) -> Option<String> {
        self.used_styles = true;
        let slot = self.slot_mut(layer);
        let prev = slot.take();
        *slot = Some(code.to_string());
        prev
    }

    /// Restore a layer to a previous value (typically from [`set`](Self::set)).
    fn restore(&mut self, layer: StyleLayer, prev: Option<String>) {
        *self.slot_mut(layer) = prev;
    }

    /// The escape code to emit when closing a block tag on `layer`.
    ///
    /// Returns the parent's code if one exists, otherwise the layer's
    /// default reset.
    fn close_code(&self, layer: StyleLayer) -> &str {
        self.get(layer).unwrap_or(layer.default_reset())
    }

    /// Clear all layers (used by `{{reset}}`).
    fn clear_all(&mut self) {
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
    fn clear_all_except_background(&mut self) {
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
fn block_tag_layer(tag_name: &str) -> Option<StyleLayer> {
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
fn atomic_token_layer(token: &str) -> Option<(StyleLayer, bool)> {
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

/// Inner token parser that shares a [`StyleState`] across recursion levels.
///
/// Block tags with a style layer push/pop via [`StyleState::set`] and
/// [`StyleState::restore`] so that closing a tag emits the *parent's*
/// escape code (or the layer's default reset) instead of `\x1b[0m`.
fn parse_tokens_inner(content: &str, term: Option<&Terminal>, state: &mut StyleState) -> String {
    let mut result = String::new();
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        // ── Atomic tokens: {{token}} ──────────────────────────────────
        if ch == '{' && chars.peek() == Some(&'{') {
            chars.next(); // consume second '{'
            let mut token = String::new();
            let mut found_close = false;

            while let Some(c) = chars.next() {
                if c == '}' && chars.peek() == Some(&'}') {
                    chars.next(); // consume second '}'
                    found_close = true;
                    break;
                }
                token.push(c);
            }

            if found_close {
                if let Some(escape) = atomic_token_to_escape(&token) {
                    result.push_str(escape);
                    state.used_styles = true;

                    // Update layer tracking
                    let token_lower = token.to_ascii_lowercase();
                    if token_lower == "reset" {
                        state.clear_all();
                    } else if token_lower == "reset-style" {
                        state.clear_all_except_background();
                    } else if let Some((layer, is_set)) = atomic_token_layer(&token_lower) {
                        if is_set {
                            state.set(layer, escape);
                        } else {
                            state.restore(layer, None);
                        }
                    }
                } else {
                    // Unknown token, output as-is
                    result.push_str("{{");
                    result.push_str(&token);
                    result.push_str("}}");
                }
            } else {
                // Unclosed token, output as-is
                result.push_str("{{");
                result.push_str(&token);
            }
            continue;
        }

        // ── Block tokens: <tag>content</tag> ──────────────────────────
        if ch == '<' {
            let mut tag_content = String::new();
            let mut found_close = false;

            // Collect until '>'
            for c in chars.by_ref() {
                if c == '>' {
                    found_close = true;
                    break;
                }
                tag_content.push(c);
            }

            if found_close && !tag_content.starts_with('/') {
                // Parse the opening tag
                if let Some((tag_name, attrs)) = parse_opening_tag(&tag_content) {
                    // Pre-compute tag patterns once
                    let closing_tag = format!("</{}>", tag_name);
                    let opening_tag_full = format!("<{}>", tag_name);
                    let opening_tag_with_space = format!("<{} ", tag_name);

                    let mut inner_content = String::new();
                    let mut depth = 1;

                    while depth > 0 {
                        if let Some(c) = chars.next() {
                            inner_content.push(c);
                            let len = inner_content.len();

                            if c == '>' {
                                if len >= opening_tag_full.len() {
                                    let start = len - opening_tag_full.len();
                                    if inner_content.get(start..).is_some_and(|slice| {
                                        slice.eq_ignore_ascii_case(&opening_tag_full)
                                    }) {
                                        depth += 1;
                                    }
                                }
                                if len >= closing_tag.len() {
                                    let start = len - closing_tag.len();
                                    if inner_content.get(start..).is_some_and(|slice| {
                                        slice.eq_ignore_ascii_case(&closing_tag)
                                    }) {
                                        depth -= 1;
                                        if depth == 0 {
                                            inner_content.truncate(start);
                                        }
                                    }
                                }
                            } else if c == ' ' && len >= opening_tag_with_space.len() {
                                let start = len - opening_tag_with_space.len();
                                if inner_content.get(start..).is_some_and(|slice| {
                                    slice.eq_ignore_ascii_case(&opening_tag_with_space)
                                }) {
                                    depth += 1;
                                }
                            }
                        } else {
                            break;
                        }
                    }

                    // Apply the block style
                    if let Some((open, close)) = block_tag_to_escape(&tag_name, &attrs, term) {
                        let layer = block_tag_layer(&tag_name);
                        state.used_styles = true;

                        if let Some(layer) = layer {
                            // Styled tag: layer-aware push/pop
                            let prev = state.set(layer, &open);
                            result.push_str(&open);
                            result.push_str(&parse_tokens_inner(&inner_content, term, state));
                            state.restore(layer, prev);
                            result.push_str(state.close_code(layer));
                        } else {
                            // Structural tag (a, clipboard): emit open/close as-is
                            result.push_str(&open);
                            result.push_str(&parse_tokens_inner(&inner_content, term, state));
                            result.push_str(&close);
                        }
                    } else {
                        // Unknown tag, output as-is
                        result.push('<');
                        result.push_str(&tag_content);
                        result.push('>');
                        result.push_str(&parse_tokens_inner(&inner_content, term, state));
                        result.push_str(&closing_tag);
                    }
                    continue;
                }
            }

            // Not a valid block tag, output as-is
            result.push('<');
            result.push_str(&tag_content);
            if found_close {
                result.push('>');
            }
            continue;
        }

        result.push(ch);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_bold_token() {
        let prose = Prose::new("Hello {{bold}}world{{reset}}!");
        let result = prose.render(None);
        assert_eq!(result, "Hello \x1b[1mworld\x1b[0m!\x1b[0m");
    }

    #[test]
    fn test_atomic_color_token() {
        let prose = Prose::new("{{red}}Error{{reset}}");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[31mError\x1b[0m\x1b[0m");
    }

    #[test]
    fn test_block_bold_tag() {
        let prose = Prose::new("<b>bold text</b>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[1mbold text\x1b[22m\x1b[0m");
    }

    #[test]
    fn test_block_italic_tag() {
        let prose = Prose::new("<i>italic text</i>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[3mitalic text\x1b[23m\x1b[0m");
    }

    #[test]
    fn test_block_underline_tag() {
        let prose = Prose::new("<u>underlined</u>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[4munderlined\x1b[24m\x1b[0m");
    }

    #[test]
    fn test_underline_variants_atomic() {
        // Double underline
        let prose = Prose::new("{{double-underline}}text{{reset}}");
        let result = prose.render(None);
        assert!(result.contains("\x1b[4:2m"));

        // Curly underline
        let prose = Prose::new("{{curly-underline}}text{{reset}}");
        let result = prose.render(None);
        assert!(result.contains("\x1b[4:3m"));

        // Dotted underline
        let prose = Prose::new("{{dotted-underline}}text{{reset}}");
        let result = prose.render(None);
        assert!(result.contains("\x1b[4:4m"));

        // Dashed underline
        let prose = Prose::new("{{dashed-underline}}text{{reset}}");
        let result = prose.render(None);
        assert!(result.contains("\x1b[4:5m"));
    }

    #[test]
    fn test_underline_variants_block() {
        // Double underline (full name)
        let prose = Prose::new("<double-underline>double</double-underline>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[4:2mdouble\x1b[24m\x1b[0m");

        // Double underline (alias)
        let prose = Prose::new("<uu>double</uu>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[4:2mdouble\x1b[24m\x1b[0m");

        // Curly underline
        let prose = Prose::new("<curly-underline>curly</curly-underline>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[4:3mcurly\x1b[24m\x1b[0m");

        // Dotted underline
        let prose = Prose::new("<dotted-underline>dotted</dotted-underline>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[4:4mdotted\x1b[24m\x1b[0m");

        // Dashed underline
        let prose = Prose::new("<dashed-underline>dashed</dashed-underline>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[4:5mdashed\x1b[24m\x1b[0m");
    }

    #[test]
    fn test_block_aliases() {
        // Test that aliases produce the same output as full names
        assert_eq!(
            Prose::new("<bold>x</bold>").render(None),
            Prose::new("<b>x</b>").render(None)
        );
        assert_eq!(
            Prose::new("<italic>x</italic>").render(None),
            Prose::new("<i>x</i>").render(None)
        );
        assert_eq!(
            Prose::new("<underline>x</underline>").render(None),
            Prose::new("<u>x</u>").render(None)
        );
        assert_eq!(
            Prose::new("<strikethrough>x</strikethrough>").render(None),
            Prose::new("<~>x</~>").render(None)
        );
    }

    #[test]
    fn test_nested_block_tags() {
        let prose = Prose::new("<b><i>bold italic</i></b>");
        let result = prose.render(None);
        // One final \x1b[0m, not two — inner recursion no longer adds its own reset
        assert_eq!(result, "\x1b[1m\x1b[3mbold italic\x1b[23m\x1b[22m\x1b[0m");
    }

    #[test]
    fn test_nested_tags_with_unicode_before_rgb_attr_tag() {
        let prose = Prose::new("<b>Stage 0 — <rgb 64,128,255>planning</rgb></b>");
        let result = prose.render(None);
        assert!(result.contains("Stage 0 —"));
        assert!(result.contains("planning"));
        assert!(result.contains("\x1b[38;2;64;128;255m"));
    }

    #[test]
    fn test_osc8_link() {
        let prose = Prose::new("<a href=\"https://example.com\">link</a>");
        let result = prose.render(None);
        assert_eq!(
            result,
            "\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\\x1b[0m"
        );
    }

    #[test]
    fn test_plain_text_no_reset() {
        let prose = Prose::new("Plain text with no styles");
        let result = prose.render(None);
        assert_eq!(result, "Plain text with no styles");
    }

    #[test]
    fn test_background_color() {
        let prose = Prose::new("{{bg-red}}highlight{{reset}}");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[41mhighlight\x1b[0m\x1b[0m");
    }

    #[test]
    fn test_strikethrough_block() {
        let prose = Prose::new("<~>deleted</~>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[9mdeleted\x1b[29m\x1b[0m");
    }

    #[test]
    fn test_named_color_block() {
        let prose = Prose::new("<red>error message</red>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[31merror message\x1b[39m\x1b[0m");
    }

    #[test]
    fn test_bright_color_block() {
        let prose = Prose::new("<bright-red>bright error</bright-red>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[91mbright error\x1b[39m\x1b[0m");

        let prose = Prose::new("<bright-cyan>info</bright-cyan>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[96minfo\x1b[39m\x1b[0m");
    }

    #[test]
    fn test_web_color_block() {
        // coral is RGB(255, 127, 80)
        let prose = Prose::new("<coral>coral text</coral>");
        let result = prose.render(None);
        assert!(result.contains("\x1b[38;2;255;127;80m"));
        assert!(result.contains("coral text"));
        assert!(result.contains("\x1b[39m"));

        // alice-blue (with hyphen) - RGB(240, 248, 255)
        let prose = Prose::new("<alice-blue>light blue</alice-blue>");
        let result = prose.render(None);
        assert!(result.contains("\x1b[38;2;240;248;255m"));
        assert!(result.contains("light blue"));
    }

    #[test]
    fn test_tailwind_color_block() {
        // purple-500 should resolve to a purple color
        let prose = Prose::new("<purple-500>tailwind purple</purple-500>");
        let result = prose.render(None);
        // Should have 24-bit color escape
        assert!(result.contains("\x1b[38;2;"));
        assert!(result.contains("tailwind purple"));
        assert!(result.contains("\x1b[39m"));

        // slate-500 should resolve to a gray-ish color
        let prose = Prose::new("<slate-500>muted</slate-500>");
        let result = prose.render(None);
        assert!(result.contains("\x1b[38;2;"));
        assert!(result.contains("muted"));
    }

    #[test]
    fn test_unknown_tag_preserved() {
        let prose = Prose::new("<unknown-tag>content</unknown-tag>");
        let result = prose.render(None);
        assert!(result.contains("<unknown-tag>content</unknown-tag>"));
    }

    #[test]
    fn test_rgb_tag() {
        // Test RGB color tag parsing
        let prose = Prose::new("<rgb 255,0,0>red text</rgb>");
        let result = prose.render(None);
        assert!(
            result.contains("\x1b[38;2;255;0;0m"),
            "Expected RGB escape code, got: {:?}",
            result
        );
        assert!(result.contains("red text"));
        assert!(result.contains("\x1b[39m"));

        // Test with different RGB values
        let prose = Prose::new("<rgb 125,67,45>brown text</rgb>");
        let result = prose.render(None);
        assert!(result.contains("\x1b[38;2;125;67;45m"));
        assert!(result.contains("brown text"));
    }

    #[test]
    fn test_resolve_href_urls_unchanged() {
        assert_eq!(resolve_href("https://example.com"), "https://example.com");
        assert_eq!(resolve_href("http://example.com"), "http://example.com");
        assert_eq!(
            resolve_href("mailto:test@example.com"),
            "mailto:test@example.com"
        );
        assert_eq!(resolve_href("file:///path/to/file"), "file:///path/to/file");
    }

    #[test]
    fn test_resolve_href_absolute_path() {
        let result = resolve_href("/usr/local/bin/test");
        assert_eq!(result, "file:///usr/local/bin/test");
    }

    #[test]
    fn test_resolve_href_empty() {
        assert_eq!(resolve_href(""), "");
    }

    #[test]
    fn test_resolve_href_relative_with_dot_slash() {
        // ./something should resolve relative to CWD
        let result = resolve_href("./test.txt");
        assert!(result.starts_with("file://"));
        assert!(result.contains("test.txt"));
    }

    #[test]
    fn test_resolve_href_relative_without_prefix() {
        // Something without ./ should try git-relative resolution
        let result = resolve_href("src/main.rs");
        assert!(result.starts_with("file://"));
        assert!(result.contains("src/main.rs"));
    }

    #[test]
    fn test_bg_rgb_tag() {
        let prose = Prose::new("<bg-rgb 255,0,0>red bg</bg-rgb>");
        let result = prose.render(None);
        assert!(
            result.contains("\x1b[48;2;255;0;0m"),
            "Expected bg RGB escape code, got: {:?}",
            result
        );
        assert!(result.contains("red bg"));
        assert!(result.contains("\x1b[49m"));

        let prose = Prose::new("<bg-rgb 125,67,45>brown bg</bg-rgb>");
        let result = prose.render(None);
        assert!(result.contains("\x1b[48;2;125;67;45m"));
        assert!(result.contains("brown bg"));
    }

    #[test]
    fn test_rgb_hex_format() {
        // Hex with # prefix
        let prose = Prose::new("<rgb #FF0000>red</rgb>");
        let result = prose.render(None);
        assert!(result.contains("\x1b[38;2;255;0;0m"));

        // Hex without # prefix
        let prose = Prose::new("<rgb FF0000>red</rgb>");
        let result = prose.render(None);
        assert!(result.contains("\x1b[38;2;255;0;0m"));

        // Dark red #8B0000
        let prose = Prose::new("<rgb #8B0000>dark red</rgb>");
        let result = prose.render(None);
        assert!(result.contains("\x1b[38;2;139;0;0m"));
    }

    #[test]
    fn test_rgb_space_separated() {
        // Space-separated values
        let prose = Prose::new("<rgb 255 0 0>red</rgb>");
        let result = prose.render(None);
        assert!(result.contains("\x1b[38;2;255;0;0m"));

        // With extra spaces
        let prose = Prose::new("<rgb  125  67  45 >brown</rgb>");
        let result = prose.render(None);
        assert!(result.contains("\x1b[38;2;125;67;45m"));
    }

    #[test]
    fn test_bg_rgb_hex_format() {
        // Hex with # prefix
        let prose = Prose::new("<bg-rgb #00FF00>green bg</bg-rgb>");
        let result = prose.render(None);
        assert!(result.contains("\x1b[48;2;0;255;0m"));

        // Hex without # prefix
        let prose = Prose::new("<bg-rgb 0000FF>blue bg</bg-rgb>");
        let result = prose.render(None);
        assert!(result.contains("\x1b[48;2;0;0;255m"));
    }

    #[test]
    fn test_bg_rgb_space_separated() {
        // Space-separated values
        let prose = Prose::new("<bg-rgb 255 128 0>orange bg</bg-rgb>");
        let result = prose.render(None);
        assert!(result.contains("\x1b[48;2;255;128;0m"));
    }

    #[test]
    fn test_bg_web_color_block() {
        // coral is RGB(255, 127, 80)
        let prose = Prose::new("<bg-coral>coral bg</bg-coral>");
        let result = prose.render(None);
        assert!(result.contains("\x1b[48;2;255;127;80m"));
        assert!(result.contains("coral bg"));
        assert!(result.contains("\x1b[49m"));

        // alice-blue (with hyphen) - RGB(240, 248, 255)
        let prose = Prose::new("<bg-alice-blue>light blue bg</bg-alice-blue>");
        let result = prose.render(None);
        assert!(result.contains("\x1b[48;2;240;248;255m"));
        assert!(result.contains("light blue bg"));
    }

    #[test]
    fn test_bg_tailwind_color_block() {
        let prose = Prose::new("<bg-red-800>danger bg</bg-red-800>");
        let result = prose.render(None);
        assert!(result.contains("\x1b[48;2;"));
        assert!(result.contains("danger bg"));
        assert!(result.contains("\x1b[49m"));

        let prose = Prose::new("<bg-slate-500>muted bg</bg-slate-500>");
        let result = prose.render(None);
        assert!(result.contains("\x1b[48;2;"));
        assert!(result.contains("muted bg"));
    }

    #[test]
    fn test_osc8_link_with_absolute_path() {
        let prose = Prose::new("<a href=\"/usr/local/bin/test\">link</a>");
        let result = prose.render(None);
        assert!(result.contains("file:///usr/local/bin/test"));
    }

    // ── Style-layer tracking tests ───────────────────────────────────

    #[test]
    fn test_same_layer_fg_nesting_restores_parent() {
        // </red> should restore blue, not reset to default
        let prose = Prose::new("<blue>before <red>red</red> after</blue>");
        let result = prose.render(None);
        assert_eq!(
            result,
            "\x1b[34mbefore \x1b[31mred\x1b[34m after\x1b[39m\x1b[0m"
        );
    }

    #[test]
    fn test_atomic_then_block_restores_atomic() {
        // {{blue}} sets foreground, then <red> opens a new scope.
        // </red> should restore to \x1b[34m (the atomic blue).
        let prose = Prose::new("{{blue}}<red>red</red>blue");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[34m\x1b[31mred\x1b[34mblue\x1b[0m");
    }

    #[test]
    fn test_no_mid_content_resets() {
        // Only one \x1b[0m should appear, at the very end
        let prose = Prose::new("<b><i>text</i> more</b>");
        let result = prose.render(None);
        let reset_count = result.matches("\x1b[0m").count();
        assert_eq!(
            reset_count, 1,
            "Expected exactly one \\x1b[0m, got: {:?}",
            result
        );
        assert!(result.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_deep_nesting_three_layers() {
        let prose = Prose::new("<b><red><i>deep</i></red></b>");
        let result = prose.render(None);
        // bold → red → italic → close italic (→ \x1b[23m]) → close red (→ \x1b[39m]) → close bold (→ \x1b[22m]) → final reset
        assert_eq!(
            result,
            "\x1b[1m\x1b[31m\x1b[3mdeep\x1b[23m\x1b[39m\x1b[22m\x1b[0m"
        );
    }

    #[test]
    fn test_reset_style_preserves_background() {
        // {{reset-style}} should clear everything except background
        let prose = Prose::new("{{bg-red}}{{bold}}text{{reset-style}}still bg");
        let result = prose.render(None);
        // After reset-style: background layer should still be Some
        // The escape sequence \x1b[22;23;24;25;27;28;29;39m is emitted,
        // but background \x1b[41m stays active in the state.
        assert!(result.contains("\x1b[41m")); // bg-red open
        assert!(result.contains("\x1b[1m")); // bold
        assert!(result.contains("\x1b[22;23;24;25;27;28;29;39m")); // reset-style
        assert!(result.contains("still bg"));
    }

    #[test]
    fn test_single_block_no_extra_reset() {
        // Single block tag should produce exactly one \x1b[0m
        let prose = Prose::new("<red>hello</red>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[31mhello\x1b[39m\x1b[0m");
    }

    #[test]
    fn test_block_tag_layer_classification() {
        assert_eq!(block_tag_layer("b"), Some(StyleLayer::FontWeight));
        assert_eq!(block_tag_layer("bold"), Some(StyleLayer::FontWeight));
        assert_eq!(block_tag_layer("dim"), Some(StyleLayer::FontWeight));
        assert_eq!(block_tag_layer("i"), Some(StyleLayer::Italic));
        assert_eq!(block_tag_layer("italic"), Some(StyleLayer::Italic));
        assert_eq!(block_tag_layer("u"), Some(StyleLayer::Underline));
        assert_eq!(block_tag_layer("red"), Some(StyleLayer::Foreground));
        assert_eq!(block_tag_layer("bright-cyan"), Some(StyleLayer::Foreground));
        assert_eq!(block_tag_layer("rgb"), Some(StyleLayer::Foreground));
        assert_eq!(block_tag_layer("bg-rgb"), Some(StyleLayer::Background));
        assert_eq!(block_tag_layer("bg-red-800"), Some(StyleLayer::Background));
        assert_eq!(block_tag_layer("a"), None);
        assert_eq!(block_tag_layer("clipboard"), None);
        // Web/tailwind colors fall through to Foreground
        assert_eq!(block_tag_layer("coral"), Some(StyleLayer::Foreground));
        assert_eq!(block_tag_layer("bg-coral"), Some(StyleLayer::Background));
    }
}
