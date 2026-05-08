//! Shared filepath formatting helpers for filesystem output.

/// Format a filepath with dim directory and bold filename,
/// wrapped in an OSC8 hyperlink.
pub(super) fn format_styled_filepath(relative: &str, absolute: &str) -> String {
    match relative.rsplit_once('/') {
        Some((dir, file)) => {
            format!("<a href=\"{absolute}\"><blue><dim>{dir}/</dim><b>{file}</b></blue></a>")
        }
        None => {
            format!("<a href=\"{absolute}\"><blue><b>{relative}</b></blue></a>")
        }
    }
}

/// Format a filepath showing only the basename, with an OSC8 hyperlink.
pub(super) fn format_basename_filepath(relative: &str, absolute: &str) -> String {
    let basename = relative.rsplit_once('/').map_or(relative, |(_, f)| f);
    format!("<a href=\"{absolute}\"><blue>{basename}</blue></a>")
}

/// Format a git-status filepath preserving the existing visible text while
/// making the path clickable through Prose OSC8 support.
pub(super) fn format_git_status_filepath(relative: &str, absolute: &str) -> String {
    match relative.rsplit_once('/') {
        Some((dir, file)) => format!("<a href=\"{absolute}\">{dir}/<b>{file}</b></a>"),
        None => format!("<a href=\"{absolute}\"><b>{relative}</b></a>"),
    }
}
