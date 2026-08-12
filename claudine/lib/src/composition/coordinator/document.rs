//! The prepared-document layer: one active document's canonical preparation
//! output, plus the document-scoped overlay and provenance that produced it.
//!
//! Phase 5 of `features/2026-07-13-proxy-with` builds the canonical
//! preparation service that fills [`PreparedDocument`] and adds the stored
//! `ComposeContext`; Phase 9 adds the target-rebuilt launch plan. This layer
//! defines the ownership boundary those phases fill in — see
//! `notes/state-migration.md`.

use indexmap::IndexMap;

use crate::composition::types::PreparedComposition;

use super::handoff::{ProxyHandoff, ProxyProvenance};

/// The evaluated `proxy.with` mapping, stored at document scope.
///
/// Immutable and pre-schema by construction. Schema defaults, coercion, and
/// invalid-optional drops shape the prepared *effective* frontmatter; if they
/// could also rewrite this, a later refresh would reapply a different overlay
/// than the one the source authored, and repeated refreshes would drift.
#[derive(Clone, Default, PartialEq)]
pub struct DocumentOverlay {
    values: IndexMap<String, serde_json::Value>,
}

impl std::fmt::Debug for DocumentOverlay {
    /// Names the properties, never the values — see
    /// [`property_names`](Self::property_names). `PreparedDocument` derives
    /// `Debug` and holds one of these, so this impl is what keeps a
    /// `{:?}`-printed prepared document from spilling the overlay.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(
                self.values
                    .keys()
                    .map(|key| (key, super::handoff::REDACTED_VALUE)),
            )
            .finish()
    }
}

impl DocumentOverlay {
    /// The overlay a document invoked without a proxy receives, and the one a
    /// proxy that omitted `with:` installs. Forwarding is explicit: a target
    /// never inherits its source's overlay.
    pub fn empty() -> Self {
        Self::default()
    }

    #[allow(missing_docs)]
    pub fn values(&self) -> &IndexMap<String, serde_json::Value> {
        &self.values
    }

    #[allow(missing_docs)]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// The overlay's property names, for status and tracing.
    ///
    /// Values are deliberately not exposed here: they can carry secrets, and
    /// status may report that a handoff includes an overlay without printing
    /// what is in it.
    pub fn property_names(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(String::as_str)
    }
}

impl From<IndexMap<String, serde_json::Value>> for DocumentOverlay {
    fn from(values: IndexMap<String, serde_json::Value>) -> Self {
        Self { values }
    }
}

/// One active document, canonically prepared.
///
/// Direct and transitioned documents share this representation: that identity
/// is what makes "proxying to a document behaves like invoking it directly"
/// checkable rather than aspirational.
#[derive(Debug, Clone)]
pub struct PreparedDocument {
    composition: PreparedComposition,
    overlay: DocumentOverlay,
    provenance: Option<ProxyProvenance>,
}

impl PreparedDocument {
    /// A document the caller named directly: no overlay, no proxy provenance.
    pub fn direct(composition: PreparedComposition) -> Self {
        Self {
            composition,
            overlay: DocumentOverlay::empty(),
            provenance: None,
        }
    }

    /// A document adopted from `handoff`, carrying that handoff's overlay and
    /// provenance.
    pub fn from_handoff(handoff: ProxyHandoff, composition: PreparedComposition) -> Self {
        let provenance = handoff.provenance().clone();
        Self {
            composition,
            overlay: DocumentOverlay::from(handoff.into_overlay()),
            provenance: Some(provenance),
        }
    }

    /// The canonical composition: resolved source, repo root, prompt,
    /// effective frontmatter, selection hints, closure plan, lifecycle config,
    /// warnings, and dropped optionals.
    pub fn composition(&self) -> &PreparedComposition {
        &self.composition
    }

    /// The immutable overlay for this document.
    pub fn overlay(&self) -> &DocumentOverlay {
        &self.overlay
    }

    /// How this document was reached, when it was proxied to.
    pub fn provenance(&self) -> Option<&ProxyProvenance> {
        self.provenance.as_ref()
    }

    /// Replace the composition after a canonical refresh (retry, resume, or a
    /// loop iteration), retaining the overlay and provenance.
    ///
    /// The overlay outlives every refresh of the same document and is
    /// reapplied deterministically; only a handoff to another document
    /// discards it.
    pub fn refreshed(self, composition: PreparedComposition) -> Self {
        Self {
            composition,
            ..self
        }
    }
}
