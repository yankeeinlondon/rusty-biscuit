use tracing::{debug, trace};

use crate::file_reference::error::FileReferenceError;
use crate::file_reference::{ParsedReference, PathTemplate, ReferenceKind, TemplateSegment};

/// Parse a raw reference string into a `ParsedReference`.
///
/// Parsing is purely syntactic -- no filesystem access occurs.
pub(crate) fn parse(raw: &str) -> Result<ParsedReference, FileReferenceError> {
    trace!(raw, "parsing file reference");

    if raw.is_empty() {
        return Err(FileReferenceError::InvalidSyntax(
            "empty reference string".to_string(),
        ));
    }

    let (recursive, remainder) = strip_recursive(raw);
    if recursive && remainder.is_empty() {
        return Err(FileReferenceError::InvalidSyntax(
            "`%` prefix requires a path".to_string(),
        ));
    }
    let (kind, path_str) = detect_kind(remainder)?;
    let template = parse_template(path_str)?;

    let parsed = ParsedReference {
        recursive,
        kind: match kind {
            DetectedKind::Relative => ReferenceKind::Relative(template),
            DetectedKind::ImplicitRelative => ReferenceKind::ImplicitRelative(template),
            DetectedKind::Absolute => ReferenceKind::Absolute(template),
            DetectedKind::Magic => ReferenceKind::Magic(template),
            DetectedKind::Package => ReferenceKind::Package(template),
            DetectedKind::Home => ReferenceKind::Home(template),
            DetectedKind::Vault => ReferenceKind::Vault(template),
            DetectedKind::Url => ReferenceKind::Url(template),
        },
    };

    debug!(kind = ?parsed.kind, recursive = parsed.recursive, "parsed reference");
    Ok(parsed)
}

/// Strip a leading `%` and return whether the reference is recursive.
fn strip_recursive(raw: &str) -> (bool, &str) {
    if let Some(rest) = raw.strip_prefix('%') {
        (true, rest)
    } else {
        (false, raw)
    }
}

enum DetectedKind {
    Relative,
    ImplicitRelative,
    Absolute,
    Magic,
    Package,
    Home,
    Vault,
    Url,
}

/// Detect the reference kind from its prefix and return the remaining path string.
///
/// Classification is purely lexical and host-independent: Windows drive/UNC
/// forms classify as absolute even when parsed on a POSIX host, so the grammar
/// is portable and testable everywhere (resolution of a foreign-platform path
/// is a separate, platform-gated concern).
///
/// ## Errors
///
/// Returns [`FileReferenceError::UnsupportedUserHome`] for a `~user` form,
/// which is not portable and must never fall through to magic or implicit.
fn detect_kind(s: &str) -> Result<(DetectedKind, &str), FileReferenceError> {
    // URL scheme is matched ASCII case-insensitively and before any drive/path
    // classifier so `HTTP://host` is never mistaken for a Windows path.
    if starts_with_ignore_ascii_case(s, "http://") || starts_with_ignore_ascii_case(s, "https://") {
        return Ok((DetectedKind::Url, s));
    }
    // vault:: (double colon) before vault: (single colon)
    if let Some(rest) = s.strip_prefix("vault::") {
        return Ok((DetectedKind::Vault, rest));
    }
    if let Some(rest) = s.strip_prefix("vault:") {
        return Ok((DetectedKind::Vault, rest));
    }
    if let Some(rest) = s.strip_prefix('@') {
        return Ok((DetectedKind::Magic, rest));
    }
    if let Some(rest) = s.strip_prefix('!') {
        return Ok((DetectedKind::Package, rest));
    }
    if let Some(rest) = s.strip_prefix('~') {
        return detect_home(s, rest);
    }
    if is_absolute_reference(s) {
        return Ok((DetectedKind::Absolute, s));
    }
    if is_explicit_relative(s) {
        return Ok((DetectedKind::Relative, s));
    }
    Ok((DetectedKind::ImplicitRelative, s))
}

/// Classify the tail of a `~`-prefixed reference.
///
/// `~`, `~/...`, and `~\...` are home-pinned; `~user` is rejected. The stored
/// path string is the portion after the home sigil and its separator, so
/// resolution can simply join it onto the home directory.
fn detect_home<'a>(
    raw: &str,
    rest: &'a str,
) -> Result<(DetectedKind, &'a str), FileReferenceError> {
    if rest.is_empty() {
        return Ok((DetectedKind::Home, ""));
    }
    if let Some(tail) = rest.strip_prefix('/').or_else(|| rest.strip_prefix('\\')) {
        return Ok((DetectedKind::Home, tail));
    }
    Err(FileReferenceError::UnsupportedUserHome(raw.to_string()))
}

/// ASCII case-insensitive prefix check.
pub(crate) fn starts_with_ignore_ascii_case(s: &str, prefix: &str) -> bool {
    s.as_bytes()
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix.as_bytes()))
}

/// Host-independent absolute-path classification for the reference grammar.
///
/// Recognizes POSIX roots (`/`), Windows drive-absolute paths (`C:\` or
/// `C:/`), and UNC paths (`\\server\share`). A drive-relative path such as
/// `C:foo.md` is deliberately **not** absolute.
pub(crate) fn is_absolute_reference(s: &str) -> bool {
    if s.starts_with('/') || s.starts_with("\\\\") {
        return true;
    }
    let bytes = s.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

/// Explicit-relative classification, accepting Windows backslash spellings
/// alongside the portable forward-slash forms.
pub(crate) fn is_explicit_relative(s: &str) -> bool {
    s.starts_with("./")
        || s.starts_with("../")
        || s.starts_with(".\\")
        || s.starts_with("..\\")
        || s == "."
        || s == ".."
}

/// Parse a path string into a `PathTemplate` with literal and env-var segments.
fn parse_template(s: &str) -> Result<PathTemplate, FileReferenceError> {
    let mut segments = Vec::new();
    let mut remaining = s;

    while !remaining.is_empty() {
        if let Some(start) = remaining.find("{{") {
            // Push literal before the opening braces
            if start > 0 {
                segments.push(TemplateSegment::Literal(remaining[..start].to_string()));
            }

            let after_open = &remaining[start + 2..];
            let end = after_open.find("}}").ok_or_else(|| {
                FileReferenceError::InvalidSyntax("unclosed `{{` in interpolation".to_string())
            })?;

            let var_name = &after_open[..end];

            if var_name.is_empty() {
                return Err(FileReferenceError::InvalidSyntax(
                    "empty variable name in `{{}}`".to_string(),
                ));
            }

            // Variable names must match [A-Z0-9_]+
            if !var_name
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            {
                return Err(FileReferenceError::InvalidSyntax(format!(
                    "invalid variable name `{var_name}` -- must match [A-Z0-9_]+"
                )));
            }

            segments.push(TemplateSegment::EnvVar(var_name.to_string()));
            remaining = &after_open[end + 2..];
        } else {
            segments.push(TemplateSegment::Literal(remaining.to_string()));
            break;
        }
    }

    Ok(PathTemplate { segments })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_path() {
        let parsed = parse("./foo.md").unwrap();
        assert!(!parsed.recursive);
        assert!(matches!(parsed.kind, ReferenceKind::Relative(_)));
    }

    #[test]
    fn relative_recursive() {
        let parsed = parse("%./foo.md").unwrap();
        assert!(parsed.recursive);
        assert!(matches!(parsed.kind, ReferenceKind::Relative(_)));
    }

    #[test]
    fn magic_path() {
        let parsed = parse("@docs/spec.md").unwrap();
        assert!(!parsed.recursive);
        assert!(matches!(parsed.kind, ReferenceKind::Magic(_)));
        let template = parsed.kind.template();
        assert_eq!(template.segments.len(), 1);
        assert_eq!(
            template.segments[0],
            TemplateSegment::Literal("docs/spec.md".to_string())
        );
    }

    #[test]
    fn magic_recursive() {
        let parsed = parse("%@README.md").unwrap();
        assert!(parsed.recursive);
        assert!(matches!(parsed.kind, ReferenceKind::Magic(_)));
    }

    #[test]
    fn package_path() {
        let parsed = parse("!lib/src/lib.rs").unwrap();
        assert!(!parsed.recursive);
        assert!(matches!(parsed.kind, ReferenceKind::Package(_)));
    }

    #[test]
    fn absolute_path() {
        let parsed = parse("/Users/bob/file.txt").unwrap();
        assert!(!parsed.recursive);
        assert!(matches!(parsed.kind, ReferenceKind::Absolute(_)));
    }

    #[test]
    fn vault_single_colon() {
        let parsed = parse("vault:notes/today.md").unwrap();
        assert!(!parsed.recursive);
        assert!(matches!(parsed.kind, ReferenceKind::Vault(_)));
        let template = parsed.kind.template();
        assert_eq!(
            template.segments[0],
            TemplateSegment::Literal("notes/today.md".to_string())
        );
    }

    #[test]
    fn vault_double_colon() {
        let parsed = parse("vault::notes/today.md").unwrap();
        assert!(matches!(parsed.kind, ReferenceKind::Vault(_)));
        let template = parsed.kind.template();
        assert_eq!(
            template.segments[0],
            TemplateSegment::Literal("notes/today.md".to_string())
        );
    }

    #[test]
    fn interpolation_single_var() {
        let parsed = parse("{{DIR}}/foo.md").unwrap();
        assert!(matches!(parsed.kind, ReferenceKind::ImplicitRelative(_)));
        let template = parsed.kind.template();
        assert_eq!(template.segments.len(), 2);
        assert_eq!(
            template.segments[0],
            TemplateSegment::EnvVar("DIR".to_string())
        );
        assert_eq!(
            template.segments[1],
            TemplateSegment::Literal("/foo.md".to_string())
        );
    }

    #[test]
    fn interpolation_empty_var_rejected() {
        let err = parse("{{}}").unwrap_err();
        assert!(
            matches!(err, FileReferenceError::InvalidSyntax(_)),
            "expected InvalidSyntax, got: {err}"
        );
    }

    #[test]
    fn interpolation_invalid_name_rejected() {
        let err = parse("{{invalid-name}}").unwrap_err();
        assert!(matches!(err, FileReferenceError::InvalidSyntax(_)));
    }

    #[test]
    fn interpolation_multiple_vars() {
        let parsed = parse("foo{{A}}bar{{B}}baz").unwrap();
        let template = parsed.kind.template();
        assert_eq!(template.segments.len(), 5);
        assert_eq!(
            template.segments[0],
            TemplateSegment::Literal("foo".to_string())
        );
        assert_eq!(
            template.segments[1],
            TemplateSegment::EnvVar("A".to_string())
        );
        assert_eq!(
            template.segments[2],
            TemplateSegment::Literal("bar".to_string())
        );
        assert_eq!(
            template.segments[3],
            TemplateSegment::EnvVar("B".to_string())
        );
        assert_eq!(
            template.segments[4],
            TemplateSegment::Literal("baz".to_string())
        );
    }

    #[test]
    fn recursive_vault_with_interpolation() {
        let parsed = parse("%vault:{{V}}/note.md").unwrap();
        assert!(parsed.recursive);
        assert!(matches!(parsed.kind, ReferenceKind::Vault(_)));
        let template = parsed.kind.template();
        assert_eq!(template.segments.len(), 2);
        assert_eq!(
            template.segments[0],
            TemplateSegment::EnvVar("V".to_string())
        );
        assert_eq!(
            template.segments[1],
            TemplateSegment::Literal("/note.md".to_string())
        );
    }

    #[test]
    fn empty_string_rejected() {
        let err = parse("").unwrap_err();
        assert!(matches!(err, FileReferenceError::InvalidSyntax(_)));
    }

    #[test]
    fn bare_filename_is_implicit_relative() {
        let parsed = parse("foo.md").unwrap();
        assert!(!parsed.recursive);
        assert!(matches!(parsed.kind, ReferenceKind::ImplicitRelative(_)));
    }

    #[test]
    fn bare_subdir_path_is_implicit_relative() {
        let parsed = parse("docs/spec.md").unwrap();
        assert!(!parsed.recursive);
        assert!(matches!(parsed.kind, ReferenceKind::ImplicitRelative(_)));
        let template = parsed.kind.template();
        assert_eq!(
            template.segments[0],
            TemplateSegment::Literal("docs/spec.md".to_string())
        );
    }

    #[test]
    fn explicit_dot_slash_is_relative() {
        let parsed = parse("./foo.md").unwrap();
        assert!(matches!(parsed.kind, ReferenceKind::Relative(_)));
    }

    #[test]
    fn lone_recursive_prefix_rejected() {
        let err = parse("%").unwrap_err();
        assert!(
            matches!(err, FileReferenceError::InvalidSyntax(_)),
            "expected InvalidSyntax, got: {err}"
        );
    }

    #[test]
    fn explicit_dotdot_slash_is_relative() {
        let parsed = parse("../foo.md").unwrap();
        assert!(matches!(parsed.kind, ReferenceKind::Relative(_)));
    }

    #[test]
    fn http_url_detected_as_url_kind() {
        let parsed = parse("http://example.com/doc.md").unwrap();
        assert!(!parsed.recursive);
        assert!(matches!(parsed.kind, ReferenceKind::Url(_)));
        let template = parsed.kind.template();
        assert_eq!(
            template.segments[0],
            TemplateSegment::Literal("http://example.com/doc.md".to_string())
        );
    }

    #[test]
    fn https_url_detected_as_url_kind() {
        let parsed = parse("https://example.com/doc.md").unwrap();
        assert!(matches!(parsed.kind, ReferenceKind::Url(_)));
    }

    #[test]
    fn url_with_path_segments() {
        let parsed = parse("https://cdn.example.com/assets/v2/spec.md").unwrap();
        assert!(matches!(parsed.kind, ReferenceKind::Url(_)));
        let template = parsed.kind.template();
        assert_eq!(
            template.segments[0],
            TemplateSegment::Literal("https://cdn.example.com/assets/v2/spec.md".to_string())
        );
    }

    #[test]
    fn tilde_bare_is_home_with_empty_path() {
        let parsed = parse("~").unwrap();
        assert!(matches!(parsed.kind, ReferenceKind::Home(_)));
        // A bare `~` carries no path segments -- it pins to the home dir itself.
        assert!(parsed.kind.template().segments.is_empty());
    }

    #[test]
    fn tilde_slash_is_home_and_strips_sigil() {
        let parsed = parse("~/foo.md").unwrap();
        assert!(matches!(parsed.kind, ReferenceKind::Home(_)));
        let template = parsed.kind.template();
        // The stored path is the portion after `~/`, ready to join onto home.
        assert_eq!(
            template.segments[0],
            TemplateSegment::Literal("foo.md".to_string())
        );
    }

    #[test]
    fn tilde_backslash_is_home() {
        let parsed = parse(r"~\foo.md").unwrap();
        assert!(matches!(parsed.kind, ReferenceKind::Home(_)));
        let template = parsed.kind.template();
        assert_eq!(
            template.segments[0],
            TemplateSegment::Literal("foo.md".to_string())
        );
    }

    #[test]
    fn tilde_user_is_rejected_typed() {
        let err = parse("~alice/notes.md").unwrap_err();
        assert!(
            matches!(err, FileReferenceError::UnsupportedUserHome(ref s) if s == "~alice/notes.md"),
            "expected UnsupportedUserHome, got: {err}"
        );
    }

    #[test]
    fn tilde_user_bare_is_rejected_typed() {
        let err = parse("~alice").unwrap_err();
        assert!(matches!(err, FileReferenceError::UnsupportedUserHome(_)));
    }

    #[test]
    fn recursive_home_reference() {
        let parsed = parse("~/notes.md").unwrap();
        assert!(!parsed.recursive);
        let parsed = parse("%~/notes.md").unwrap();
        assert!(parsed.recursive);
        assert!(matches!(parsed.kind, ReferenceKind::Home(_)));
    }

    #[test]
    fn windows_drive_absolute_backslash() {
        let parsed = parse(r"C:\work\a.md").unwrap();
        assert!(matches!(parsed.kind, ReferenceKind::Absolute(_)));
    }

    #[test]
    fn windows_drive_absolute_forward_slash() {
        let parsed = parse("C:/work/a.md").unwrap();
        assert!(matches!(parsed.kind, ReferenceKind::Absolute(_)));
    }

    #[test]
    fn windows_unc_is_absolute() {
        let parsed = parse(r"\\server\share\a.md").unwrap();
        assert!(matches!(parsed.kind, ReferenceKind::Absolute(_)));
    }

    #[test]
    fn windows_drive_relative_is_not_absolute() {
        // `C:foo.md` is drive-relative, not absolute.
        let parsed = parse("C:foo.md").unwrap();
        assert!(
            matches!(parsed.kind, ReferenceKind::ImplicitRelative(_)),
            "drive-relative must not classify absolute; got {:?}",
            parsed.kind
        );
    }

    #[test]
    fn backslash_explicit_relative() {
        assert!(matches!(
            parse(r".\foo.md").unwrap().kind,
            ReferenceKind::Relative(_)
        ));
        assert!(matches!(
            parse(r"..\foo.md").unwrap().kind,
            ReferenceKind::Relative(_)
        ));
    }

    #[test]
    fn url_scheme_is_ascii_case_insensitive() {
        for raw in ["HTTP://example.com/a.md", "HttpS://example.com/a.md", "HTTPS://x"] {
            let parsed = parse(raw).unwrap();
            assert!(
                matches!(parsed.kind, ReferenceKind::Url(_)),
                "expected Url for `{raw}`, got {:?}",
                parsed.kind
            );
        }
    }
}
