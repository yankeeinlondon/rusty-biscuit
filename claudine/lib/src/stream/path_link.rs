//! Format an absolute file path for rendering inside `Prose` markup.
//!
//! Long absolute paths inside tool call / result slots dominate the line
//! and hide the interesting tail. When the path lives inside the current
//! workspace (or the user's `$HOME`), this helper returns an OSC8
//! hyperlink whose *visible* text is the short form (repo-relative or
//! `~/`-relative) and whose `href` is the full absolute path so
//! click-to-open still works.
//!
//! Keep the helper fast: it is called for every rendered tool event.
//! Resolution is purely lexical — no `fs::canonicalize`, no I/O.

use std::path::{Path, PathBuf};

/// Maximum length of the *visible* text inside the OSC8 link. Paths
/// longer than this are truncated with a leading ellipsis so the stderr
/// column budget is preserved even for deeply nested files.
const MAX_VISIBLE_LEN: usize = 80;

/// Render a file reference as styled Prose markup.
///
/// ## Returns
///
/// - When `raw` is an absolute path under `cwd`: a blue OSC8 link whose
///   visible text is the path relative to `cwd`.
/// - When `raw` is an absolute path under `home`: a blue OSC8 link whose
///   visible text is `~/<rest>`.
/// - Otherwise: the escaped literal (`escape_prose`-style) so stray
///   markup characters in the path cannot break Prose parsing.
pub fn format_file_link(raw: &str, cwd: &Path, home: Option<&Path>) -> String {
    if raw.is_empty() {
        return String::new();
    }
    let path = Path::new(raw);
    if !path.is_absolute() {
        return escape_prose(raw);
    }
    if let Some(rel) = strip_prefix(path, cwd)
        && !rel.as_os_str().is_empty()
    {
        return osc8_link(raw, &rel.to_string_lossy());
    }
    if let Some(home) = home
        && let Some(rel) = strip_prefix(path, home)
        && !rel.as_os_str().is_empty()
    {
        let visible = format!("~/{}", rel.to_string_lossy());
        return osc8_link(raw, &visible);
    }
    escape_prose(raw)
}

fn strip_prefix(path: &Path, base: &Path) -> Option<PathBuf> {
    path.strip_prefix(base).ok().map(Path::to_path_buf)
}

fn osc8_link(href_raw: &str, visible_raw: &str) -> String {
    let visible = truncate_visible(visible_raw);
    let href = escape_href(href_raw);
    let visible = escape_prose(&visible);
    format!("<blue><a href=\"{href}\">{visible}</a></blue>")
}

fn truncate_visible(text: &str) -> String {
    let char_count = text.chars().count();
    if char_count <= MAX_VISIBLE_LEN {
        return text.to_string();
    }
    // Keep the tail; paths are most informative near the leaf.
    let skip = char_count - (MAX_VISIBLE_LEN - 1);
    let tail: String = text.chars().skip(skip).collect();
    format!("\u{2026}{tail}")
}

/// Escape characters that would break Prose markup inside the `href`
/// attribute. Prose escapes `<`, `>`, `{`, `\` and `"` via backslash; the
/// same characters matter here because the parser reads attributes as
/// Prose-escaped strings.
fn escape_href(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '<' | '>' | '{' | '"' => {
                out.push('\\');
                out.push(ch);
            }
            other => out.push(other),
        }
    }
    out
}

/// Escape visible text so stray `<`, `>`, `{`, `\` cannot be parsed as
/// Prose markup. Mirrors the `escape_prose` helper in the CLI's
/// `live_semantic_sink`, duplicated here so the library does not depend
/// on the CLI crate.
fn escape_prose(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '<' | '>' | '{' => {
                out.push('\\');
                out.push(ch);
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn path_inside_cwd_renders_relative_osc8_link() {
        let cwd = PathBuf::from("/repo");
        let raw = "/repo/src/main.rs";
        let rendered = format_file_link(raw, &cwd, None);
        assert_eq!(
            rendered,
            r#"<blue><a href="/repo/src/main.rs">src/main.rs</a></blue>"#
        );
    }

    #[test]
    fn path_inside_home_renders_tilde_prefix() {
        let cwd = PathBuf::from("/repo");
        let home = PathBuf::from("/home/ken");
        let raw = "/home/ken/notes/today.md";
        let rendered = format_file_link(raw, &cwd, Some(&home));
        assert_eq!(
            rendered,
            r#"<blue><a href="/home/ken/notes/today.md">~/notes/today.md</a></blue>"#
        );
    }

    #[test]
    fn path_outside_cwd_and_home_falls_back_to_escaped_literal() {
        let cwd = PathBuf::from("/repo");
        let home = PathBuf::from("/home/ken");
        let raw = "/etc/hosts";
        let rendered = format_file_link(raw, &cwd, Some(&home));
        assert_eq!(rendered, "/etc/hosts");
    }

    #[test]
    fn relative_path_passes_through_escaped() {
        let cwd = PathBuf::from("/repo");
        let raw = "src/main.rs";
        let rendered = format_file_link(raw, &cwd, None);
        assert_eq!(rendered, "src/main.rs");
    }

    #[test]
    fn prose_metacharacters_in_path_are_escaped() {
        let cwd = PathBuf::from("/repo");
        let raw = "/repo/weird<name>.rs";
        let rendered = format_file_link(raw, &cwd, None);
        // Both href and visible must have angle brackets escaped.
        assert!(rendered.contains(r#"href="/repo/weird\<name\>.rs""#));
        assert!(rendered.contains(">weird\\<name\\>.rs<"));
    }

    #[test]
    fn empty_input_returns_empty_string() {
        assert_eq!(format_file_link("", Path::new("/repo"), None), "");
    }

    #[test]
    fn cwd_itself_is_not_linked() {
        // strip_prefix yields an empty relative path — we fall back to the
        // escaped literal rather than rendering a zero-width link.
        let cwd = PathBuf::from("/repo");
        assert_eq!(format_file_link("/repo", &cwd, None), "/repo");
    }

    #[test]
    fn home_preferred_when_cwd_is_not_prefix() {
        let cwd = PathBuf::from("/repo");
        let home = PathBuf::from("/home/ken");
        let raw = "/home/ken/.config/foo.toml";
        let rendered = format_file_link(raw, &cwd, Some(&home));
        assert!(
            rendered
                .starts_with(r#"<blue><a href="/home/ken/.config/foo.toml">~/.config/foo.toml"#)
        );
    }

    #[test]
    fn cwd_preferred_over_home_when_both_could_match() {
        let cwd = PathBuf::from("/home/ken/repo");
        let home = PathBuf::from("/home/ken");
        let raw = "/home/ken/repo/src/main.rs";
        let rendered = format_file_link(raw, &cwd, Some(&home));
        assert!(rendered.contains(">src/main.rs<"));
    }

    #[test]
    fn long_path_truncates_visible_text_but_keeps_full_href() {
        let cwd = PathBuf::from("/repo");
        let long_tail: String = "a/".repeat(60);
        let raw = format!("/repo/{long_tail}file.rs");
        let rendered = format_file_link(&raw, &cwd, None);
        assert!(rendered.contains(&format!(r#"href="{raw}""#)));
        let visible_start = rendered.find(r#"">"#).unwrap() + 2;
        let visible_end = rendered.rfind("</a>").unwrap();
        let visible = &rendered[visible_start..visible_end];
        assert!(
            visible.chars().count() <= MAX_VISIBLE_LEN,
            "visible text {} chars",
            visible.chars().count()
        );
        assert!(visible.starts_with('\u{2026}'));
        assert!(visible.ends_with("file.rs"));
    }
}
