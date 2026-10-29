//! Schema documents compiled into the binary, addressed by repository-relative
//! virtual paths.
//!
//! Darkmatter's global catalog imports its partials, and its `fm` partial
//! imports Content Policy's editor schema, with ordinary relative references.
//! Resolving them from a table of embedded texts lets an installed binary
//! prepare the catalog and its baseline without a checkout. This is
//! a source for shipped schemas only: a user `$schema` reaches it solely
//! through the reserved namespace (`SHIPPED_NAMESPACE`,
//! `Name@darkmatter:<stem>`), and an embedded import never falls back to the
//! disk or the network.
//!
//! A host can also supply an [`EmbeddedSchemaBundle`] of trigger schemas and
//! their libraries. Its documents use the same lexical resolution, keyed by
//! `{bundle}/{file name}` and confined to that bundle; the reserved namespace
//! is the one way out of it.

use crate::markdown::schemas::errors::SchemaError;

/// A host-supplied set of schema documents compiled into the host binary,
/// offered to trigger discovery as a last-resort trigger location.
///
/// `name` identifies the bundle in origins, traces, and errors; `files` pairs
/// each document's plain file name (`claudine.yaml`) with its text. Within a
/// bundle, documents address each other with relative references
/// (`stack@./lifecycle.yaml`) that resolve lexically among the bundle's own
/// files, never on the disk, and name Darkmatter's shipped partials with the
/// reserved namespace (`say-short@darkmatter:lifecycle`). Darkmatter ships no
/// bundle of its own; a host
/// registers one with
/// [`DarkmatterSchemas::with_embedded_trigger_bundle`](super::DarkmatterSchemas::with_embedded_trigger_bundle).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct EmbeddedSchemaBundle {
    name: &'static str,
    files: &'static [(&'static str, &'static str)],
}

impl EmbeddedSchemaBundle {
    /// A bundle named `name` holding `files` as `(file name, text)` pairs.
    ///
    /// The shape is checked when the bundle is registered: `name` must be one
    /// non-empty path segment, and every file name a distinct (also after
    /// case-folding) plain `.yaml`/`.yml` file name.
    #[must_use]
    pub const fn new(name: &'static str, files: &'static [(&'static str, &'static str)]) -> Self {
        Self { name, files }
    }

    #[allow(missing_docs)]
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    #[allow(missing_docs)]
    #[must_use]
    pub const fn files(&self) -> &'static [(&'static str, &'static str)] {
        self.files
    }

    /// The bundle's files in UTF-8 file-name byte order, the order its
    /// triggers layer in (the same order a disk schema root uses).
    pub(crate) fn sorted_files(&self) -> Vec<(&'static str, &'static str)> {
        let mut files = self.files.to_vec();
        files.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        files
    }

    /// The virtual path that identifies `name` in this bundle:
    /// `{bundle}/{name}`. It names no host file.
    pub(crate) fn virtual_path(&self, name: &str) -> String {
        format!("{}/{name}", self.name)
    }

    /// Calls `f` with the bundle's documents keyed by virtual path.
    pub(crate) fn with_documents<R>(&self, f: impl FnOnce(&[EmbeddedDocument<'_>]) -> R) -> R {
        let paths: Vec<String> = self.files.iter().map(|(name, _)| self.virtual_path(name)).collect();
        let documents: Vec<EmbeddedDocument<'_>> = paths
            .iter()
            .zip(self.files)
            .map(|(path, (_, text))| EmbeddedDocument { path, text })
            .collect();
        f(&documents)
    }

    /// Checks the bundle's name and file names.
    ///
    /// ## Errors
    ///
    /// [`SchemaError::EmbeddedBundle`] naming the bundle and the defect.
    pub(crate) fn check(&self) -> Result<(), SchemaError> {
        let invalid = |message: String| SchemaError::EmbeddedBundle {
            bundle: self.name.to_string(),
            message,
        };
        if !is_plain_segment(self.name) {
            return Err(invalid(format!(
                "bundle name `{}` must be one non-empty path segment",
                self.name
            )));
        }
        let mut seen: Vec<(String, &str)> = Vec::with_capacity(self.files.len());
        for (name, _) in self.files {
            let is_yaml = name
                .rsplit_once('.')
                .is_some_and(|(stem, ext)| !stem.is_empty() && (ext == "yaml" || ext == "yml"));
            if !is_plain_segment(name) || !is_yaml {
                return Err(invalid(format!(
                    "file name `{name}` must be a plain `.yaml`/`.yml` file name"
                )));
            }
            let folded = name.to_lowercase();
            if let Some((_, earlier)) = seen.iter().find(|(other, _)| *other == folded) {
                return Err(invalid(format!(
                    "file names `{earlier}` and `{name}` collide (after case-folding)"
                )));
            }
            seen.push((folded, name));
        }
        Ok(())
    }
}

impl std::fmt::Debug for EmbeddedSchemaBundle {
    /// Names the bundle and its files, never their text.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddedSchemaBundle")
            .field("name", &self.name)
            .field("files", &self.files.iter().map(|(name, _)| *name).collect::<Vec<_>>())
            .finish()
    }
}

/// `true` for a non-empty single path segment that is not `.`/`..` and holds
/// no separator, drive colon, or `biscuit-file` prefix character.
fn is_plain_segment(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value.trim() == value
        && !value.contains(['/', '\\', ':', '*', '?', '[', ']', '{', '}'])
        && !value.starts_with(['~', '&', '^', '$', '%', '@'])
}

/// One embedded schema document.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EmbeddedDocument<'a> {
    /// Repository-relative path with `/` separators; the document's identity.
    pub path: &'a str,
    pub text: &'a str,
}

/// Virtual path of Content Policy's editor schema.
pub(crate) const CONTENT_POLICY_PATH: &str = "content-policy/schemas/content-policy.yaml";

/// The shipped documents: Darkmatter's global catalog, its partials
/// (including the neutral lifecycle action definitions, the `md` profile's
/// stacks, and the expression-function catalog, whose `recent-commits` helper
/// type the context partial imports), and the Content Policy editor schema the
/// `fm` partial imports.
/// The runtime baseline is the catalog's resolved `fm` definition.
pub(crate) const SHIPPED: &[EmbeddedDocument<'static>] = &[
    EmbeddedDocument {
        path: super::catalog::DARKMATTER_CATALOG_PATH,
        text: include_str!("../../../../schemas/darkmatter.yaml"),
    },
    EmbeddedDocument {
        path: "darkmatter/schemas/partials/fm.yaml",
        text: include_str!("../../../../schemas/partials/fm.yaml"),
    },
    EmbeddedDocument {
        path: "darkmatter/schemas/partials/style.yaml",
        text: include_str!("../../../../schemas/partials/style.yaml"),
    },
    EmbeddedDocument {
        path: "darkmatter/schemas/partials/context-variables.yaml",
        text: include_str!("../../../../schemas/partials/context-variables.yaml"),
    },
    EmbeddedDocument {
        path: "darkmatter/schemas/partials/error.yaml",
        text: include_str!("../../../../schemas/partials/error.yaml"),
    },
    EmbeddedDocument {
        path: "darkmatter/schemas/partials/timing.yaml",
        text: include_str!("../../../../schemas/partials/timing.yaml"),
    },
    EmbeddedDocument {
        path: "darkmatter/schemas/partials/lifecycle.yaml",
        text: include_str!("../../../../schemas/partials/lifecycle.yaml"),
    },
    EmbeddedDocument {
        path: "darkmatter/schemas/partials/md-lifecycle.yaml",
        text: include_str!("../../../../schemas/partials/md-lifecycle.yaml"),
    },
    EmbeddedDocument {
        path: "darkmatter/schemas/partials/functions.yaml",
        text: include_str!("../../../../schemas/partials/functions.yaml"),
    },
    EmbeddedDocument {
        path: CONTENT_POLICY_PATH,
        text: content_policy::EDITOR_SCHEMA,
    },
];

/// The reserved namespace of a named-type reference to one of Darkmatter's
/// shipped partials: `Name@darkmatter:<stem>` names the type `Name` in the
/// [`SHIPPED`] document `darkmatter/schemas/partials/<stem>.yaml`, whoever
/// writes it and wherever it is written. It names no host file, so the disk
/// is never consulted, and references inside the partial resolve among the
/// shipped documents.
pub(crate) const SHIPPED_NAMESPACE: &str = "darkmatter:";

/// The `<stem>` of a reference written in the reserved form
/// `darkmatter:<stem>`, or `None` for any other reference.
pub(crate) fn reserved_stem(reference: &str) -> Option<&str> {
    reference.trim().strip_prefix(SHIPPED_NAMESPACE)
}

/// The shipped partial `darkmatter:<stem>` names, or `None` when [`SHIPPED`]
/// has no partial of that stem.
pub(crate) fn shipped_partial(stem: &str) -> Option<EmbeddedDocument<'static>> {
    if !is_plain_segment(stem) || stem.contains('.') {
        return None;
    }
    find(SHIPPED, &format!("darkmatter/schemas/partials/{stem}.yaml"))
}

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
pub(crate) fn resolve_bundle_path(root: &str, importer: &str, reference: &str) -> Result<String, String> {
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
    fn the_fm_partial_import_climbs_three_directories_to_the_policy_schema() {
        assert_eq!(
            resolve_path("darkmatter/schemas/partials/fm.yaml", "../../../content-policy/schemas/content-policy.yaml"),
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
    fn bundle_checks_reject_unusable_names() {
        const FILES: &[(&str, &str)] = &[("a.yaml", ""), ("b.yml", "")];
        assert!(EmbeddedSchemaBundle::new("host", FILES).check().is_ok());
        for name in ["", ".", "..", "a/b", "a\\b", "c:", " host", "~host", "*"] {
            assert!(EmbeddedSchemaBundle::new(name, FILES).check().is_err(), "bundle `{name}`");
        }
        for files in [
            &[("a/b.yaml", "")][..],
            &[("..", "")],
            &[(".yaml", "")],
            &[("a.txt", "")],
            &[("a", "")],
            &[("a.yaml", ""), ("a.yaml", "")],
            &[("a.yaml", ""), ("A.YAML", "")],
        ] {
            assert!(
                matches!(EmbeddedSchemaBundle::new("host", files).check(), Err(SchemaError::EmbeddedBundle { .. })),
                "{files:?}"
            );
        }
    }

    #[test]
    fn bundle_debug_names_files_but_not_their_text() {
        let bundle = EmbeddedSchemaBundle::new("host", &[("a.yaml", "secret: text")]);
        let shown = format!("{bundle:?}");
        assert!(shown.contains("a.yaml") && !shown.contains("secret"), "{shown}");
    }

    #[test]
    fn reserved_references_name_shipped_partials_only() {
        assert_eq!(reserved_stem(" darkmatter:lifecycle "), Some("lifecycle"));
        assert_eq!(reserved_stem("./lifecycle.yaml"), None);
        assert_eq!(reserved_stem("this"), None);
        for stem in ["lifecycle", "md-lifecycle", "fm", "timing"] {
            let document = shipped_partial(stem).unwrap_or_else(|| panic!("`{stem}` is shipped"));
            assert_eq!(document.path, format!("darkmatter/schemas/partials/{stem}.yaml"));
        }
        for stem in ["", "bogus", "lifecycle.yaml", "../darkmatter", "partials/fm", ".", "..", "darkmatter"] {
            assert!(shipped_partial(stem).is_none(), "`{stem}` names no shipped partial");
        }
    }

    #[test]
    fn the_shipped_table_registers_exactly_the_catalog_its_partials_and_policy_schema() {
        let paths: Vec<_> = SHIPPED.iter().map(|document| document.path).collect();
        assert_eq!(
            paths,
            [
                super::super::catalog::DARKMATTER_CATALOG_PATH,
                "darkmatter/schemas/partials/fm.yaml",
                "darkmatter/schemas/partials/style.yaml",
                "darkmatter/schemas/partials/context-variables.yaml",
                "darkmatter/schemas/partials/error.yaml",
                "darkmatter/schemas/partials/timing.yaml",
                "darkmatter/schemas/partials/lifecycle.yaml",
                "darkmatter/schemas/partials/md-lifecycle.yaml",
                "darkmatter/schemas/partials/functions.yaml",
                CONTENT_POLICY_PATH,
            ]
        );
        assert_eq!(find(SHIPPED, CONTENT_POLICY_PATH).map(|d| d.text), Some(content_policy::EDITOR_SCHEMA));
        assert!(find(SHIPPED, "darkmatter/docs/schemas/darkmatter.yaml").is_none());
    }
}
