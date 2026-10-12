//! Lexical rules for a `FileChanged` path, enforced with or without a file
//! provider.
//!
//! A path must name the same file on every host, so absolute and
//! machine-dependent forms are rejected here rather than resolved and found
//! missing. Only boundary containment needs the file system; the bundled
//! adapter checks that.

use crate::diagnostic::DiagnosticCode;
use crate::grammar::RuleError;

fn invalid(path: &str, why: &str) -> RuleError {
    RuleError {
        code: DiagnosticCode::InvalidPath,
        message: format!("`FileChanged` path `{path}` {why}"),
    }
}

/// Checks the authored path against the spec's path table: implicit and
/// explicit relative paths and the `&` and `^` sigils are accepted; every
/// other form is rejected with the reason.
pub(crate) fn validate_path(path: &str) -> Result<(), RuleError> {
    if path.is_empty() {
        return Err(invalid(path, "is empty; name a file such as `src/config.rs`"));
    }
    if path.trim() != path {
        return Err(invalid(
            path,
            "has leading or trailing whitespace; write the path with none, such as \
             `FileChanged(src/config.rs, @config_fingerprint)`",
        ));
    }
    if path.contains(',') || path.contains(')') {
        return Err(invalid(
            path,
            "contains `,` or `)`, which delimit the rule's arguments; rename the file or watch \
             another one",
        ));
    }
    if path.contains('\\') {
        return Err(invalid(path, "uses `\\`; write `/` as the separator on every OS"));
    }
    if path.contains("{{") {
        return Err(invalid(
            path,
            "contains a `{{ }}` variable, which names a different file on each host",
        ));
    }
    if path.contains("://") {
        return Err(invalid(path, "is a URL; `FileChanged` watches a local file"));
    }
    let (sigil, rest) = match path.as_bytes()[0] {
        b'&' => (Some('&'), &path[1..]),
        b'^' => (Some('^'), &path[1..]),
        _ => (None, path),
    };
    // The sigils accept an optional `/` after them, as Biscuit File does.
    let payload = match sigil {
        Some(_) => rest.strip_prefix('/').unwrap_or(rest),
        None => rest,
    };
    if payload.is_empty() {
        return Err(invalid(path, "names no file after its sigil, such as `&Cargo.toml`"));
    }
    if payload.starts_with('/') || sigil.is_none() && path.starts_with('/') {
        return Err(invalid(
            path,
            "is absolute; write it relative to the document, such as `../src/config.rs`, or \
             from the repository root with `&`",
        ));
    }
    let first = payload.split('/').next().unwrap_or(payload);
    if first.len() >= 2 && first.as_bytes()[0].is_ascii_alphabetic() && first.as_bytes()[1] == b':' {
        return Err(invalid(path, "starts with a drive letter; write a relative path"));
    }
    if first.contains(':') {
        return Err(invalid(
            path,
            "starts with a scheme such as `vault:`; write a path relative to the document",
        ));
    }
    let leading = payload.as_bytes()[0];
    let why = match leading {
        b'~' => Some("starts with `~`, which names a different home directory on each host"),
        b'@' => Some("starts with `@`, a search across configured roots; name one file"),
        b'%' => Some("starts with `%`, a recursive search; name one file"),
        b'&' | b'^' => Some("repeats a sigil; write `&` or `^` once"),
        _ => None,
    };
    match why {
        Some(why) => Err(invalid(path, why)),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_forms() {
        for path in [
            "src/config.rs",
            "config.rs",
            "./config.rs",
            "../src/config.rs",
            "../../a/b.md",
            "&Cargo.toml",
            "&/Cargo.toml",
            "^README.md",
            "notes/a #1.md",
            "logs/run: 2.txt",
            "a(b.md",
            "dir/",
            ".hidden",
        ] {
            validate_path(path).unwrap_or_else(|error| panic!("{path}: {}", error.message));
        }
    }

    #[test]
    fn every_rejected_lexical_form_names_its_reason() {
        for (path, reason) in [
            ("", "is empty"),
            (" src/config.rs", "whitespace"),
            ("src/config.rs ", "whitespace"),
            ("src/config.rs\t", "whitespace"),
            ("a,b.md", "`,` or `)`"),
            ("a)b.md", "`,` or `)`"),
            ("src\\config.rs", "`\\`"),
            ("C:\\x", "`\\`"),
            ("{{HOME}}/x", "variable"),
            ("docs/{{name}}.md", "variable"),
            ("https://example.com/x", "URL"),
            ("/etc/hosts", "absolute"),
            ("&//x", "absolute"),
            ("C:/x", "drive letter"),
            ("C:x", "drive letter"),
            ("vault:x", "scheme"),
            ("ab:y/z", "scheme"),
            ("x:y/z", "drive letter"),
            ("~/x", "`~`"),
            ("~", "`~`"),
            ("@x", "`@`"),
            ("%x", "`%`"),
            ("%@x", "`%`"),
            ("&", "names no file"),
            ("^/", "names no file"),
            ("&~/x", "`~`"),
            ("^@x", "`@`"),
            ("&^x", "repeats a sigil"),
        ] {
            let error = validate_path(path).expect_err(path);
            assert_eq!(error.code, DiagnosticCode::InvalidPath, "{path}");
            assert!(error.message.contains(reason), "{path}: {}", error.message);
        }
    }
}
