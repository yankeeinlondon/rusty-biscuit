//! Shared filepath formatting helpers for filesystem output.

/// Escape underscores so Prose's Markdown pre-processor does not treat them
/// as italics markers (`_text_`).
fn escape_prose_underscores(s: &str) -> String {
    s.replace('_', r"\_")
}

/// Format a filepath with dim directory and bold filename,
/// wrapped in an OSC8 hyperlink.
pub(super) fn format_styled_filepath(relative: &str, absolute: &str) -> String {
    match relative.rsplit_once('/') {
        Some((dir, file)) => {
            let file = escape_prose_underscores(file);
            format!("<a href=\"{absolute}\"><blue><dim>{dir}/</dim><b>{file}</b></blue></a>")
        }
        None => {
            let relative = escape_prose_underscores(relative);
            format!("<a href=\"{absolute}\"><blue><b>{relative}</b></blue></a>")
        }
    }
}

/// Format a filepath showing only the basename, with an OSC8 hyperlink.
pub(super) fn format_basename_filepath(relative: &str, absolute: &str) -> String {
    let basename = relative.rsplit_once('/').map_or(relative, |(_, f)| f);
    let basename = escape_prose_underscores(basename);
    format!("<a href=\"{absolute}\"><blue>{basename}</blue></a>")
}

/// Format a git-status filepath preserving the existing visible text while
/// making the path clickable through Prose OSC8 support.
pub(super) fn format_git_status_filepath(relative: &str, absolute: &str) -> String {
    match relative.rsplit_once('/') {
        Some((dir, file)) => {
            let file = escape_prose_underscores(file);
            format!("<a href=\"{absolute}\">{dir}/<b>{file}</b></a>")
        }
        None => {
            let relative = escape_prose_underscores(relative);
            format!("<a href=\"{absolute}\"><b>{relative}</b></a>")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styled_filepath_escapes_underscores_in_filename() {
        let out = format_styled_filepath(
            "darkmatter/lib/frontmatter_shell_expansion.rs",
            "/abs/darkmatter/lib/frontmatter_shell_expansion.rs",
        );
        assert!(out.contains(r"frontmatter\_shell\_expansion.rs"));
        assert!(out.contains("<b>"));
    }

    #[test]
    fn basename_filepath_escapes_underscores() {
        let out = format_basename_filepath(
            "darkmatter/lib/frontmatter_shell_expansion.rs",
            "/abs/darkmatter/lib/frontmatter_shell_expansion.rs",
        );
        assert!(out.contains(r"frontmatter\_shell\_expansion.rs"));
    }

    #[test]
    fn git_status_filepath_escapes_underscores_in_filename() {
        let out = format_git_status_filepath(
            "darkmatter/lib/frontmatter_shell_expansion.rs",
            "/abs/darkmatter/lib/frontmatter_shell_expansion.rs",
        );
        assert!(out.contains(r"frontmatter\_shell\_expansion.rs"));
        assert!(out.contains("<b>"));
    }

    #[test]
    fn filepath_without_underscores_unchanged() {
        let out = format_styled_filepath("foo/bar/baz.rs", "/abs/foo/bar/baz.rs");
        assert_eq!(out, "<a href=\"/abs/foo/bar/baz.rs\"><blue><dim>foo/bar/</dim><b>baz.rs</b></blue></a>");
    }
}
