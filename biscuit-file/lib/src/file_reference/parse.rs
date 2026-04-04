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
    let (kind, path_str) = detect_kind(remainder);
    let template = parse_template(path_str)?;

    let parsed = ParsedReference {
        recursive,
        kind: match kind {
            DetectedKind::Relative => ReferenceKind::Relative(template),
            DetectedKind::Absolute => ReferenceKind::Absolute(template),
            DetectedKind::Magic => ReferenceKind::Magic(template),
            DetectedKind::Package => ReferenceKind::Package(template),
            DetectedKind::Vault => ReferenceKind::Vault(template),
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
    Absolute,
    Magic,
    Package,
    Vault,
}

/// Detect the reference kind from its prefix and return the remaining path string.
fn detect_kind(s: &str) -> (DetectedKind, &str) {
    // vault:: (double colon) before vault: (single colon)
    if let Some(rest) = s.strip_prefix("vault::") {
        return (DetectedKind::Vault, rest);
    }
    if let Some(rest) = s.strip_prefix("vault:") {
        return (DetectedKind::Vault, rest);
    }
    if let Some(rest) = s.strip_prefix('@') {
        return (DetectedKind::Magic, rest);
    }
    if let Some(rest) = s.strip_prefix('!') {
        return (DetectedKind::Package, rest);
    }
    if s.starts_with('/') {
        return (DetectedKind::Absolute, s);
    }
    (DetectedKind::Relative, s)
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
        assert!(matches!(parsed.kind, ReferenceKind::Relative(_)));
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
    fn bare_filename() {
        let parsed = parse("foo.md").unwrap();
        assert!(!parsed.recursive);
        assert!(matches!(parsed.kind, ReferenceKind::Relative(_)));
    }
}
