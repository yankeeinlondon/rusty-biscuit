//! Schema documents compiled into the binary, addressed by repository-relative
//! virtual paths.
//!
//! The runtime baseline imports Content Policy's editor schema with an
//! ordinary relative reference. Resolving it from a table of embedded texts
//! lets an installed binary prepare the baseline without a checkout. This is
//! a source for shipped schemas only: user `$schema` references never consult
//! it, and an embedded import never falls back to the disk or the network.

/// One embedded schema document.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EmbeddedDocument<'a> {
    /// Repository-relative path with `/` separators; the document's identity.
    pub path: &'a str,
    pub text: &'a str,
}

/// Virtual path of Darkmatter's runtime baseline.
pub(crate) const BASELINE_PATH: &str = "darkmatter/docs/schemas/darkmatter.yaml";

/// Virtual path of Content Policy's editor schema.
pub(crate) const CONTENT_POLICY_PATH: &str = "content-policy/schemas/content-policy.yaml";

/// The shipped documents: the runtime baseline and the Content Policy editor
/// schema it imports.
pub(crate) const SHIPPED: &[EmbeddedDocument<'static>] = &[
    EmbeddedDocument {
        path: BASELINE_PATH,
        text: include_str!("../../../../docs/schemas/darkmatter.yaml"),
    },
    EmbeddedDocument {
        path: CONTENT_POLICY_PATH,
        text: content_policy::EDITOR_SCHEMA,
    },
];

/// Looks up the document registered at `path`.
pub(crate) fn find<'a>(documents: &[EmbeddedDocument<'a>], path: &str) -> Option<EmbeddedDocument<'a>> {
    documents.iter().copied().find(|document| document.path == path)
}

/// Resolves `reference`, written in the embedded document at `importer`, to a
/// virtual path.
///
/// Resolution is lexical: `reference` is joined to `importer`'s directory one
/// `/`-separated segment at a time, `.` is dropped, and `..` removes the
/// previous segment. The virtual root is the repository root, so `..` may
/// leave the importer's directory but never the root.
///
/// ## Errors
///
/// A reason string when `reference` is empty, a URL, an absolute, drive, or
/// UNC path, spelled with `\`, empty segments, glob characters, or a
/// `biscuit-file` prefix (`~`, `&`, `^`, `$`, `%`, `@`), or when it climbs
/// above the virtual root.
pub(crate) fn resolve_path(importer: &str, reference: &str) -> Result<String, String> {
    let reference = reference.trim();
    if reference.is_empty() {
        return Err("an embedded import names no document".into());
    }
    if reference.contains("://") {
        return Err("an embedded import cannot be a URL".into());
    }
    if reference.starts_with('/') || reference.contains('\\') || reference.contains(':') {
        return Err("an embedded import must be a relative path written with `/`".into());
    }
    if reference.starts_with(['~', '&', '^', '$', '%', '@']) {
        return Err("an embedded import cannot use a root, home, or variable prefix".into());
    }
    if reference.contains(['*', '?', '[', ']', '{', '}']) {
        return Err("an embedded import cannot be a glob".into());
    }

    let mut segments: Vec<&str> = importer.split('/').collect();
    segments.pop();
    for segment in reference.split('/') {
        match segment {
            "" => return Err("an embedded import cannot contain an empty path segment".into()),
            "." => {}
            ".." => {
                if segments.pop().is_none() {
                    return Err("an embedded import cannot climb above the repository root".into());
                }
            }
            name => segments.push(name),
        }
    }
    Ok(segments.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_baseline_import_climbs_three_directories_to_the_policy_schema() {
        assert_eq!(
            resolve_path(BASELINE_PATH, "../../../content-policy/schemas/content-policy.yaml"),
            Ok(CONTENT_POLICY_PATH.to_string())
        );
    }

    #[test]
    fn relative_forms_resolve_lexically() {
        let importer = "a/b/schema.yaml";
        for (reference, expected) in [
            ("types.yaml", "a/b/types.yaml"),
            ("./types.yaml", "a/b/types.yaml"),
            ("../types.yaml", "a/types.yaml"),
            ("../../types.yaml", "types.yaml"),
            ("./c/../d/./types.yaml", "a/b/d/types.yaml"),
            (" ./types.yaml ", "a/b/types.yaml"),
        ] {
            assert_eq!(resolve_path(importer, reference).as_deref(), Ok(expected), "{reference}");
        }
    }

    #[test]
    fn host_paths_urls_and_escapes_are_rejected() {
        let importer = "a/b/schema.yaml";
        for reference in [
            "",
            "   ",
            "../../../types.yaml",
            "../../../../types.yaml",
            "/etc/types.yaml",
            "C:/types.yaml",
            "C:\\types.yaml",
            "\\\\server\\share\\types.yaml",
            "..\\types.yaml",
            "https://example.com/types.yaml",
            "file:///types.yaml",
            "~/types.yaml",
            "&types.yaml",
            "^types.yaml",
            "$HOME/types.yaml",
            "%APPDATA%/types.yaml",
            "@types.yaml",
            "*.yaml",
            "a//types.yaml",
            "types.yaml/",
        ] {
            assert!(resolve_path(importer, reference).is_err(), "`{reference}` resolved");
        }
    }

    #[test]
    fn the_shipped_table_registers_exactly_the_baseline_and_policy_schema() {
        let paths: Vec<_> = SHIPPED.iter().map(|document| document.path).collect();
        assert_eq!(paths, [BASELINE_PATH, CONTENT_POLICY_PATH]);
        assert_eq!(find(SHIPPED, CONTENT_POLICY_PATH).map(|d| d.text), Some(content_policy::EDITOR_SCHEMA));
        assert!(find(SHIPPED, "darkmatter/schemas/darkmatter.yaml").is_none());
    }
}
