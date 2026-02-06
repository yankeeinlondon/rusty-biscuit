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
/// - `<rgb 125,67,45>content</rgb>` for RGB colored foreground text
/// - `<red>content</red>` for named color foreground text
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
    fn parse_tokens(&self, _term: Option<&Terminal>) -> String {
        let mut result = String::new();
        let mut used_styles = false;
        let mut chars = self.content.chars().peekable();

        while let Some(ch) = chars.next() {
            // Check for atomic tokens: {{token}}
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
                        used_styles = true;
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

            // Check for block tokens: <tag>content</tag>
            if ch == '<' {
                let mut tag_content = String::new();
                let mut found_close = false;

                // Collect until '>'
                while let Some(c) = chars.next() {
                    if c == '>' {
                        found_close = true;
                        break;
                    }
                    tag_content.push(c);
                }

                if found_close && !tag_content.starts_with('/') {
                    // Parse the opening tag
                    if let Some((tag_name, attrs)) = parse_opening_tag(&tag_content) {
                        // Pre-compute tag patterns once (avoid repeated format! allocations)
                        let closing_tag = format!("</{}>", tag_name);
                        let opening_tag_full = format!("<{}>", tag_name);
                        let opening_tag_with_space = format!("<{} ", tag_name);

                        let mut inner_content = String::new();
                        let mut depth = 1;

                        // Use index-based matching instead of repeated ends_with
                        // Track potential match positions for efficiency
                        while depth > 0 {
                            if let Some(c) = chars.next() {
                                inner_content.push(c);
                                let len = inner_content.len();

                                // Only check for tag matches when we have enough characters
                                // and the last character could complete a tag (i.e., '>' or ' ')
                                if c == '>' {
                                    // Check for nested opening tag: <tag_name>
                                    if len >= opening_tag_full.len() {
                                        let start = len - opening_tag_full.len();
                                        if inner_content[start..].eq_ignore_ascii_case(&opening_tag_full) {
                                            depth += 1;
                                        }
                                    }
                                    // Check for closing tag: </tag_name>
                                    if len >= closing_tag.len() {
                                        let start = len - closing_tag.len();
                                        if inner_content[start..].eq_ignore_ascii_case(&closing_tag) {
                                            depth -= 1;
                                            if depth == 0 {
                                                // Remove the closing tag from content
                                                inner_content.truncate(start);
                                            }
                                        }
                                    }
                                } else if c == ' ' {
                                    // Check for opening tag with attributes: <tag_name ...
                                    if len >= opening_tag_with_space.len() {
                                        let start = len - opening_tag_with_space.len();
                                        if inner_content[start..].eq_ignore_ascii_case(&opening_tag_with_space) {
                                            depth += 1;
                                        }
                                    }
                                }
                            } else {
                                break;
                            }
                        }

                        // Recursively parse inner content
                        let inner_prose = Prose::new(inner_content);
                        let parsed_inner = inner_prose.parse_tokens(_term);

                        // Apply the block style
                        if let Some((open, close)) = block_tag_to_escape(&tag_name, &attrs, _term) {
                            result.push_str(&open);
                            result.push_str(&parsed_inner);
                            result.push_str(&close);
                            used_styles = true;
                        } else {
                            // Unknown tag, output as-is
                            result.push('<');
                            result.push_str(&tag_content);
                            result.push('>');
                            result.push_str(&parsed_inner);
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

        // Add reset at the end if styles were used
        if used_styles {
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

        // Try web colors, then Tailwind colors
        _ => {
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

/// Parse an RGB string like "125,67,45" into (r, g, b).
fn parse_rgb(s: &str) -> Option<(u8, u8, u8)> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() == 3 {
        let r = parts[0].trim().parse::<u8>().ok()?;
        let g = parts[1].trim().parse::<u8>().ok()?;
        let b = parts[2].trim().parse::<u8>().ok()?;
        Some((r, g, b))
    } else {
        None
    }
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
    if href.starts_with("./") {
        let relative_path = &href[2..]; // Strip the ./
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
            if let Ok(contents) = std::fs::read_to_string(&cargo_toml) {
                if contents.contains("[package]") {
                    return Some(current);
                }
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
        assert_eq!(
            result,
            "\x1b[1m\x1b[3mbold italic\x1b[23m\x1b[0m\x1b[22m\x1b[0m"
        );
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
    fn test_osc8_link_with_absolute_path() {
        let prose = Prose::new("<a href=\"/usr/local/bin/test\">link</a>");
        let result = prose.render(None);
        assert!(result.contains("file:///usr/local/bin/test"));
    }
}
