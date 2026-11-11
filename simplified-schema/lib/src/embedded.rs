//! Schema documents supplied as text, addressed by repository-relative
//! virtual paths.
//!
//! A host that compiles schema documents into its binary hands them to the
//! resolver as [`EmbeddedDocument`]s. Their imports resolve lexically among
//! the supplied documents ([`resolve_path`], [`resolve_bundle_path`]) and
//! never fall back to the disk or the network.
//!
//! A host may also reserve a namespace for its shipped documents
//! ([`ShippedSources`]): a named-type reference `Name@<namespace><stem>`
//! names the type `Name` in the shipped partial `<partials>/<stem>.yaml`,
//! wherever it is written. The resolver knows no namespace of its own; with
//! [`ShippedSources::NONE`] such a reference is an ordinary file reference.
//! A host whose shipped artifact holds those partials' parsed named types
//! attaches it with [`ShippedSources::with_prepared`].

use crate::shipped::PreparedSourcesDescriptor;

/// One embedded schema document.
#[derive(Debug, Clone, Copy)]
pub struct EmbeddedDocument<'a> {
    /// Repository-relative path with `/` separators; the document's identity.
    pub path: &'a str,
    pub text: &'a str,
}

/// A host's reserved namespace and the shipped documents it names.
///
/// Darkmatter supplies `darkmatter:` over its compiled-in catalog, so
/// `stack@darkmatter:lifecycle` names `stack` in
/// `darkmatter/schemas/partials/lifecycle.yaml`, and an import written inside
/// a shipped document resolves among [`Self::documents`].
#[derive(Debug, Clone, Copy)]
pub struct ShippedSources {
    namespace: &'static str,
    partials: &'static str,
    documents: &'static [EmbeddedDocument<'static>],
    prepared: Option<&'static PreparedSourcesDescriptor>,
}

impl ShippedSources {
    /// No reserved namespace and no shipped documents.
    pub const NONE: Self = Self { namespace: "", partials: "", documents: &[], prepared: None };

    /// The reserved `namespace` prefix (`darkmatter:`), the virtual
    /// directory its stems name partials in (`darkmatter/schemas/partials`),
    /// and the shipped `documents`.
    #[must_use]
    pub const fn new(
        namespace: &'static str,
        partials: &'static str,
        documents: &'static [EmbeddedDocument<'static>],
    ) -> Self {
        Self { namespace, partials, documents, prepared: None }
    }

    /// These sources with the host's prepared named-type tables attached.
    /// The resolver asks `descriptor` for a partial's table before parsing
    /// its text, and uses it only when [`Self::documents`] are exactly the
    /// documents the tables were prepared from; [`Self::new`] alone parses.
    #[must_use]
    pub const fn with_prepared(self, descriptor: &'static PreparedSourcesDescriptor) -> Self {
        Self { prepared: Some(descriptor), ..self }
    }

    /// The attached prepared-table descriptor, if any.
    #[must_use]
    pub const fn prepared(&self) -> Option<&'static PreparedSourcesDescriptor> {
        self.prepared
    }

    /// The shipped documents.
    #[must_use]
    pub const fn documents(&self) -> &'static [EmbeddedDocument<'static>] {
        self.documents
    }

    /// The `<stem>` of a reference written in the reserved form
    /// `<namespace><stem>`, or `None` for any other reference (always `None`
    /// for [`Self::NONE`]).
    #[must_use]
    pub fn reserved_stem<'r>(&self, reference: &'r str) -> Option<&'r str> {
        if self.namespace.is_empty() {
            return None;
        }
        reference.trim().strip_prefix(self.namespace)
    }

    /// The shipped partial `<namespace><stem>` names, or `None` when no
    /// shipped document is that partial.
    #[must_use]
    pub fn partial(&self, stem: &str) -> Option<EmbeddedDocument<'static>> {
        if !is_plain_segment(stem) || stem.contains('.') {
            return None;
        }
        find(self.documents, &format!("{}/{stem}.yaml", self.partials))
    }
}

/// `true` for a non-empty single path segment that is not `.`/`..` and holds
/// no separator, drive colon, or `biscuit-file` prefix character.
pub fn is_plain_segment(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value.trim() == value
        && !value.contains(['/', '\\', ':', '*', '?', '[', ']', '{', '}'])
        && !value.starts_with(['~', '&', '^', '$', '%', '@'])
}

/// Looks up the document registered at `path`.
pub fn find<'a>(documents: &[EmbeddedDocument<'a>], path: &str) -> Option<EmbeddedDocument<'a>> {
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
pub fn resolve_path(importer: &str, reference: &str) -> Result<String, String> {
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

/// Resolves `reference`, written in the bundle document `importer`, to a
/// virtual path inside the bundle rooted at `root`.
///
/// The same lexical rules as [`resolve_path`]; the target must stay under
/// `{root}/`.
///
/// ## Errors
///
/// A reason string for every [`resolve_path`] rejection, and when the target
/// leaves the bundle.
pub fn resolve_bundle_path(root: &str, importer: &str, reference: &str) -> Result<String, String> {
    let leaves = || format!("an embedded import cannot leave bundle `{root}`");
    let target = resolve_path(importer, reference).map_err(|reason| {
        if reason.contains("climb above") { leaves() } else { reason }
    })?;
    match target.strip_prefix(root).and_then(|rest| rest.strip_prefix('/')) {
        Some(rest) if !rest.is_empty() => Ok(target),
        _ => Err(leaves()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn bundle_paths_resolve_inside_the_bundle_only() {
        let importer = "host/trigger.yaml";
        for (reference, expected) in [
            ("lib.yaml", Ok("host/lib.yaml")),
            ("./lib.yaml", Ok("host/lib.yaml")),
            ("./nested/../lib.yaml", Ok("host/lib.yaml")),
            ("../host/lib.yaml", Ok("host/lib.yaml")),
            ("../other/lib.yaml", Err("leave bundle `host`")),
            ("../lib.yaml", Err("leave bundle `host`")),
            ("../../lib.yaml", Err("leave bundle `host`")),
            ("..", Err("leave bundle `host`")),
            ("~/lib.yaml", Err("prefix")),
            ("/lib.yaml", Err("relative path")),
        ] {
            match (resolve_bundle_path("host", importer, reference), expected) {
                (Ok(target), Ok(expected)) => assert_eq!(target, expected, "{reference}"),
                (Err(reason), Err(expected)) => assert!(reason.contains(expected), "{reference}: {reason}"),
                (result, expected) => panic!("{reference}: {result:?}, expected {expected:?}"),
            }
        }
    }

    #[test]
    fn without_a_reserved_namespace_no_reference_is_reserved() {
        assert_eq!(ShippedSources::NONE.reserved_stem("darkmatter:lifecycle"), None);
        assert!(ShippedSources::NONE.partial("lifecycle").is_none());
    }
}
