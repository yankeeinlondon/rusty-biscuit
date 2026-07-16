//! Opaque reference-graph provenance primitives.
//!
//! These compact, private types record how a [`ReferenceGraph`] was built so
//! that prebuilt-graph validation can reject a graph whose root document,
//! source, mode, options, or visited-descendant state no longer matches the
//! validation request. They are introduced here before being wired into
//! `ReferenceGraph` (opacity cutover); nothing in this module retains a full
//! [`Markdown`], `ComposeOptions`, context, cache, callback, preflight graph,
//! or remote-fetch runtime.
//!
//! Identities are practical in-process correctness guards computed with
//! `biscuit-hash` xxHash. They are **not** cryptographically unforgeable.
//!
//! [`ReferenceGraph`]: super::types::ReferenceGraph

use crate::markdown::Markdown;
use crate::markdown::compose::{ComposeSource, ReferenceGraphOptionsIdentity};
use biscuit_hash::xx_hash_bytes;

/// Domain separator for the whole-represented-state document fingerprint.
const DOCUMENT_IDENTITY_DOMAIN: &str = "dm.reference.document-identity.v1";

/// Which builder produced a graph, and therefore what reference extraction it
/// performed.
///
/// This is the sole input controlling reference extraction: `Full` extracts
/// links/images/imports at every node; `TransclusionOnly` records only the
/// transclusion tree. Full reference validation requires `Full` even when every
/// other identity component matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReferenceGraphMode {
    /// Transclusions plus every reference type at each node.
    Full,
    /// Transclusion tree only (no link/image extraction at leaf nodes).
    TransclusionOnly,
}

/// Compact identity of one document's complete represented state.
///
/// Combines Darkmatter's Markdown-aware frontmatter and body hashes (both
/// `strict`, preserving map order and body bytes for fail-closed reuse) with a
/// mandatory whole-represented-state fingerprint. The whole-state fingerprint
/// folds a versioned domain, a representation discriminant, the retained
/// authored [`Frontmatter::raw_source`] bytes when present, an order-preserving
/// canonical encoding of the **current** frontmatter map, and the current body
/// bytes. Both the raw source and the current map participate because
/// [`Frontmatter::as_map_mut`] can mutate the in-memory map without
/// invalidating the retained raw snapshot.
///
/// This is an in-process correctness guard, **not** a cryptographically
/// unforgeable token.
///
/// [`Frontmatter::raw_source`]: crate::markdown::Frontmatter::raw_source
/// [`Frontmatter::as_map_mut`]: crate::markdown::Frontmatter::as_map_mut
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReferenceDocumentIdentity {
    /// Markdown-aware frontmatter-map hash (`strict = true`).
    frontmatter_hash: u64,
    /// Markdown-aware body hash (`strict = true`).
    body_hash: u64,
    /// Whole-represented-state fingerprint (see type docs).
    whole_state: u64,
}

impl ReferenceDocumentIdentity {
    /// Captures the identity of `md`'s current in-memory represented state.
    pub(crate) fn capture(md: &Markdown) -> Self {
        Self {
            frontmatter_hash: md.hash_frontmatter(true),
            body_hash: md.hash_body(true),
            whole_state: whole_state_fingerprint(md),
        }
    }
}

/// Computes the whole-represented-state fingerprint for `md`.
fn whole_state_fingerprint(md: &Markdown) -> u64 {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(DOCUMENT_IDENTITY_DOMAIN.as_bytes());
    buf.push(0);

    // Representation discriminant + retained authored frontmatter source.
    match md.frontmatter().raw_source() {
        Some(raw) => {
            buf.push(1);
            buf.extend_from_slice(raw.as_bytes());
        }
        None => buf.push(0),
    }
    buf.push(0);

    // Order-preserving canonical encoding of the current frontmatter map
    // (`FrontmatterMap` is an `IndexMap`, so iteration preserves order).
    for (key, value) in md.frontmatter().as_map() {
        buf.extend_from_slice(key.as_bytes());
        buf.push(b'=');
        buf.extend_from_slice(serde_json::to_string(value).unwrap_or_default().as_bytes());
        buf.push(0);
    }
    buf.push(0);

    // Current body bytes.
    buf.extend_from_slice(md.content().as_bytes());

    xx_hash_bytes(&buf)
}

/// One visited file-backed descendant document and its recorded identity.
#[derive(Debug, Clone)]
pub(crate) struct ReferenceDocumentDependency {
    /// The exact resolved source used to construct the node.
    pub(crate) source: ComposeSource,
    /// The descendant's represented-state identity at construction time.
    pub(crate) document: ReferenceDocumentIdentity,
}

/// Private build output: one compact identity per unique visited local child.
///
/// Only production graph construction assembles a non-empty manifest; the
/// `Default` empty manifest supports synthetic-shape tests that never validate.
#[derive(Debug, Clone, Default)]
pub(crate) struct ReferenceDependencyManifest {
    documents: Vec<ReferenceDocumentDependency>,
}

impl ReferenceDependencyManifest {
    /// Records one dependency for `source`, deduped by exact resolved source.
    ///
    /// Inserts at most one entry per unique `ComposeSource` already used to
    /// construct the node — no second or ad-hoc canonicalization. Entries keep
    /// their insertion order, which production construction drives in a
    /// deterministic traversal order so clones and diagnostics are stable.
    pub(crate) fn record(&mut self, source: ComposeSource, document: ReferenceDocumentIdentity) {
        if self.documents.iter().any(|entry| entry.source == source) {
            return;
        }
        self.documents
            .push(ReferenceDocumentDependency { source, document });
    }

    /// Returns the recorded dependency entries in deterministic order.
    pub(crate) fn documents(&self) -> &[ReferenceDocumentDependency] {
        &self.documents
    }

    /// Number of recorded dependency entries.
    ///
    /// Descendant verification iterates [`documents`](Self::documents); `len`
    /// and `is_empty` exist for assertions in the reference-graph tests.
    #[allow(dead_code)]
    pub(crate) fn len(&self) -> usize {
        self.documents.len()
    }

    /// Returns `true` when no dependencies were recorded.
    #[allow(dead_code)]
    pub(crate) fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
}

/// Why a prebuilt graph is incompatible with a validation request.
///
/// A dependency mismatch names the changed, missing, or unreadable child source
/// **without** exposing any content fingerprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReferenceGraphMismatch {
    /// The root document's represented state differs.
    Document,
    /// The document source differs (file, URL, or unknown).
    Source,
    /// The graph mode differs (e.g. transclusion-only where full is required).
    Mode {
        /// The mode the validation request requires.
        expected: ReferenceGraphMode,
        /// The mode recorded on the prebuilt graph.
        found: ReferenceGraphMode,
    },
    /// The graph-affecting options differ.
    Options,
    /// A visited child dependency changed, is missing, or is unreadable.
    Dependency {
        /// The child source, without its content fingerprint.
        source: ComposeSource,
        /// What went wrong with the descendant.
        kind: DependencyMismatchKind,
    },
}

/// The way a visited descendant failed re-verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DependencyMismatchKind {
    /// The descendant still exists and is readable but its content changed.
    Changed,
    /// The descendant no longer exists.
    Missing,
    /// The descendant exists but could not be read.
    Unreadable,
}

/// Compact, mandatory build provenance carried by every `ReferenceGraph`.
///
/// Records the root document identity, source, mode, options identity, and the
/// visited-descendant manifest. It never changes graph presentation or
/// serialization and never retains a stateful compose resource.
#[derive(Debug, Clone)]
pub(crate) struct ReferenceGraphProvenance {
    document: ReferenceDocumentIdentity,
    source: Option<ComposeSource>,
    mode: ReferenceGraphMode,
    options: ReferenceGraphOptionsIdentity,
    dependencies: ReferenceDependencyManifest,
}

impl ReferenceGraphProvenance {
    /// Assembles provenance from already-computed identities and the dependency
    /// manifest produced by the graph-build runtime.
    pub(crate) fn new(
        document: ReferenceDocumentIdentity,
        source: Option<ComposeSource>,
        mode: ReferenceGraphMode,
        options: ReferenceGraphOptionsIdentity,
        dependencies: ReferenceDependencyManifest,
    ) -> Self {
        Self {
            document,
            source,
            mode,
            options,
            dependencies,
        }
    }

    /// The recorded graph mode.
    ///
    /// `check` compares the mode internally; this accessor exists for
    /// reference-graph test assertions.
    #[allow(dead_code)]
    pub(crate) fn mode(&self) -> ReferenceGraphMode {
        self.mode
    }

    /// The recorded visited-descendant manifest.
    pub(crate) fn dependencies(&self) -> &ReferenceDependencyManifest {
        &self.dependencies
    }

    /// Verifies the four base dimensions in order document → source → mode →
    /// options against a validation request.
    ///
    /// Descendant verification is performed separately by the validator, which
    /// reloads the manifest entries from the authoritative filesystem.
    ///
    /// ## Errors
    ///
    /// Returns the first differing dimension as a [`ReferenceGraphMismatch`].
    pub(crate) fn check(
        &self,
        request_document: &ReferenceDocumentIdentity,
        request_source: &Option<ComposeSource>,
        request_mode: ReferenceGraphMode,
        request_options: &ReferenceGraphOptionsIdentity,
    ) -> Result<(), ReferenceGraphMismatch> {
        if self.document != *request_document {
            return Err(ReferenceGraphMismatch::Document);
        }
        if self.source != *request_source {
            return Err(ReferenceGraphMismatch::Source);
        }
        if self.mode != request_mode {
            return Err(ReferenceGraphMismatch::Mode {
                expected: request_mode,
                found: self.mode,
            });
        }
        if self.options != *request_options {
            return Err(ReferenceGraphMismatch::Options);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::ComposeOptions;
    use std::path::PathBuf;

    fn url(raw: &str) -> ComposeSource {
        ComposeSource::Url(raw.parse().unwrap())
    }

    fn identity_of(content: &str) -> ReferenceDocumentIdentity {
        let md: Markdown = content.into();
        ReferenceDocumentIdentity::capture(&md)
    }

    #[test]
    fn document_identity_same_bytes_equal() {
        let a = identity_of("---\ntitle: A\n---\nBody text\n");
        let b = identity_of("---\ntitle: A\n---\nBody text\n");
        assert_eq!(a, b);
    }

    #[test]
    fn document_identity_different_body_unequal() {
        let a = identity_of("---\ntitle: A\n---\nBody one\n");
        let b = identity_of("---\ntitle: A\n---\nBody two\n");
        assert_ne!(a, b);
    }

    #[test]
    fn document_identity_different_frontmatter_same_reference_shape_unequal() {
        // Same body (no references either way), different represented
        // frontmatter — must not be interchangeable.
        let a = identity_of("---\ntitle: A\n---\nSame body\n");
        let b = identity_of("---\ntitle: B\n---\nSame body\n");
        assert_ne!(a, b);
    }

    #[test]
    fn document_identity_mutated_map_unequal_even_with_raw_source_present() {
        let mut md: Markdown = "---\ntitle: A\n---\nBody\n".into();
        assert!(
            md.frontmatter().raw_source().is_some(),
            "parsed frontmatter retains raw source"
        );
        let before = ReferenceDocumentIdentity::capture(&md);

        md.frontmatter_mut()
            .as_map_mut()
            .insert("title".to_string(), serde_json::json!("B"));

        // Raw source is unchanged, but the current map differs, so identity
        // must change (the whole-state fingerprint folds the current map).
        assert!(md.frontmatter().raw_source().is_some());
        let after = ReferenceDocumentIdentity::capture(&md);
        assert_ne!(before, after);
    }

    #[test]
    fn dependency_manifest_dedupes_same_source() {
        let mut manifest = ReferenceDependencyManifest::default();
        let src = url("https://example.com/child.md");
        manifest.record(src.clone(), identity_of("first"));
        manifest.record(src.clone(), identity_of("second"));
        assert_eq!(manifest.len(), 1, "same source records exactly one entry");
    }

    #[test]
    fn dependency_manifest_preserves_insertion_order() {
        let mut manifest = ReferenceDependencyManifest::default();
        let a = url("https://example.com/a.md");
        let b = url("https://example.com/b.md");
        let c = url("https://example.com/c.md");
        manifest.record(a.clone(), identity_of("a"));
        manifest.record(b.clone(), identity_of("b"));
        manifest.record(c.clone(), identity_of("c"));
        let sources: Vec<&ComposeSource> =
            manifest.documents().iter().map(|d| &d.source).collect();
        assert_eq!(sources, vec![&a, &b, &c]);
    }

    fn provenance(
        document: ReferenceDocumentIdentity,
        source: Option<ComposeSource>,
        mode: ReferenceGraphMode,
        options: &ComposeOptions,
    ) -> ReferenceGraphProvenance {
        ReferenceGraphProvenance::new(
            document,
            source,
            mode,
            ReferenceGraphOptionsIdentity::capture(options),
            ReferenceDependencyManifest::default(),
        )
    }

    #[test]
    fn check_accepts_matching_dimensions() {
        let opts = ComposeOptions::new();
        let doc = identity_of("---\ntitle: A\n---\nBody\n");
        let src = Some(url("https://example.com/root.md"));
        let prov = provenance(doc.clone(), src.clone(), ReferenceGraphMode::Full, &opts);

        let request_opts = ReferenceGraphOptionsIdentity::capture(&opts);
        assert!(
            prov.check(&doc, &src, ReferenceGraphMode::Full, &request_opts)
                .is_ok()
        );
    }

    #[test]
    fn check_reports_document_first() {
        let opts = ComposeOptions::new();
        let doc = identity_of("---\ntitle: A\n---\nBody\n");
        let other = identity_of("---\ntitle: A\n---\nDifferent\n");
        let src = Some(url("https://example.com/root.md"));
        let prov = provenance(doc, src.clone(), ReferenceGraphMode::Full, &opts);

        let request_opts = ReferenceGraphOptionsIdentity::capture(&opts);
        // Document, source, and mode all differ; document is checked first.
        let err = prov
            .check(
                &other,
                &Some(url("https://example.com/moved.md")),
                ReferenceGraphMode::TransclusionOnly,
                &request_opts,
            )
            .unwrap_err();
        assert_eq!(err, ReferenceGraphMismatch::Document);
    }

    #[test]
    fn check_reports_source_before_mode() {
        let opts = ComposeOptions::new();
        let doc = identity_of("---\ntitle: A\n---\nBody\n");
        let prov = provenance(
            doc.clone(),
            Some(url("https://example.com/root.md")),
            ReferenceGraphMode::Full,
            &opts,
        );

        let request_opts = ReferenceGraphOptionsIdentity::capture(&opts);
        let err = prov
            .check(
                &doc,
                &Some(url("https://example.com/moved.md")),
                ReferenceGraphMode::TransclusionOnly,
                &request_opts,
            )
            .unwrap_err();
        assert_eq!(err, ReferenceGraphMismatch::Source);
    }

    #[test]
    fn check_reports_mode_before_options() {
        let opts = ComposeOptions::new();
        let doc = identity_of("---\ntitle: A\n---\nBody\n");
        let src = Some(url("https://example.com/root.md"));
        let prov = provenance(doc.clone(), src.clone(), ReferenceGraphMode::TransclusionOnly, &opts);

        // Different options AND different mode; mode is checked first.
        let other_opts = ComposeOptions::new().with_max_transclusion_depth(3);
        let request_opts = ReferenceGraphOptionsIdentity::capture(&other_opts);
        let err = prov
            .check(&doc, &src, ReferenceGraphMode::Full, &request_opts)
            .unwrap_err();
        assert_eq!(
            err,
            ReferenceGraphMismatch::Mode {
                expected: ReferenceGraphMode::Full,
                found: ReferenceGraphMode::TransclusionOnly,
            }
        );
    }

    #[test]
    fn check_reports_options_mismatch() {
        let opts = ComposeOptions::new();
        let doc = identity_of("---\ntitle: A\n---\nBody\n");
        let src = Some(url("https://example.com/root.md"));
        let prov = provenance(doc.clone(), src.clone(), ReferenceGraphMode::Full, &opts);

        let other_opts = ComposeOptions::new().with_max_transclusion_depth(3);
        let request_opts = ReferenceGraphOptionsIdentity::capture(&other_opts);
        let err = prov
            .check(&doc, &src, ReferenceGraphMode::Full, &request_opts)
            .unwrap_err();
        assert_eq!(err, ReferenceGraphMismatch::Options);
    }

    #[test]
    fn provenance_accessors_expose_mode_and_dependencies() {
        let opts = ComposeOptions::new();
        let doc = identity_of("body");
        let prov = provenance(doc, None, ReferenceGraphMode::Full, &opts);
        assert_eq!(prov.mode(), ReferenceGraphMode::Full);
        assert!(prov.dependencies().is_empty());
    }

    #[allow(dead_code)]
    fn file_source(path: &str) -> ComposeSource {
        ComposeSource::File(PathBuf::from(path))
    }
}
