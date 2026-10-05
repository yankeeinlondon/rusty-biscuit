//! Selective symbol queries: which kinds a caller wants and which analysis
//! work it needs, so a listing runs no more than it shows.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::shared::{AnalysisPass, FileSymbolIndex, SymbolKindV2};

/// A unit of analysis a query can require.
///
/// Requiring a capability also requires its prerequisites:
/// [`References`](Self::References) implies
/// [`Containment`](Self::Containment), [`Semantics`](Self::Semantics) implies
/// [`References`](Self::References), and every capability implies
/// [`Declarations`](Self::Declarations).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Declaration records with identity, spans, container name and the
    /// cheap metadata (keyword, signature text, declaration text, signature,
    /// type metadata, raw doc comment). The file's import and export records
    /// come with it.
    Declarations,
    /// `relations.parent`, `relations.container` and `relations.members`.
    Containment,
    /// The bind pass: containment plus reference, dependency and
    /// referenced-by edges and resolved export targets.
    References,
    /// The semantic pass (`semantics` facet of functions and methods).
    Semantics,
    /// The docs pass (`docs.parsed` and attached comments).
    StructuredDocs,
    /// The diagnostics pass (lint and syntax diagnostics).
    Diagnostics,
}

impl Capability {
    /// Every capability, in canonical order.
    pub const ALL: [Capability; 6] = [
        Capability::Declarations,
        Capability::Containment,
        Capability::References,
        Capability::Semantics,
        Capability::StructuredDocs,
        Capability::Diagnostics,
    ];

    fn prerequisites(self) -> &'static [Capability] {
        match self {
            Capability::Declarations => &[],
            Capability::Containment => &[Capability::Declarations],
            Capability::References => &[Capability::Containment],
            Capability::Semantics => &[Capability::References],
            Capability::StructuredDocs | Capability::Diagnostics => &[Capability::Declarations],
        }
    }
}

/// Which symbols a caller wants from a file and what must be computed for
/// them.
///
/// ## Examples
///
/// ```no_run
/// use tree_hugger::{Capability, SourceFile, SymbolKindV2, SymbolQuery};
///
/// let query = SymbolQuery::new()
///     .kinds([SymbolKindV2::Function, SymbolKindV2::Method])
///     .capabilities([Capability::Declarations, Capability::Containment]);
/// let result = SourceFile::read("src/lib.rs")?.query(&query)?;
/// # Ok::<(), tree_hugger::TreeHuggerError>(())
/// ```
///
/// ## Notes
///
/// Selection never removes prerequisite data: declarations of unselected
/// kinds are still extracted (identity, spans, container) so they can own or
/// shadow selected ones, and a selected record's relations may name them.
/// Only their metadata is skipped, and they are left out of the result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolQuery {
    /// `None` selects every kind.
    kinds: Option<BTreeSet<SymbolKindV2>>,
    /// Closed under prerequisites; always holds `Declarations`.
    capabilities: BTreeSet<Capability>,
}

impl Default for SymbolQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolQuery {
    /// Every kind, declarations only.
    pub fn new() -> Self {
        Self {
            kinds: None,
            capabilities: BTreeSet::from([Capability::Declarations]),
        }
    }

    /// Every kind and every capability: the complete index
    /// [`TreeFile::symbol_index_v2`](crate::TreeFile::symbol_index_v2) returns.
    pub fn complete_index() -> Self {
        Self::new().capabilities(Capability::ALL)
    }

    /// Every kind, with the capabilities that produce `passes`.
    pub fn for_passes(passes: &[AnalysisPass]) -> Self {
        Self::new().capabilities(passes.iter().map(|pass| match pass {
            AnalysisPass::Parse => Capability::Declarations,
            AnalysisPass::Bind => Capability::References,
            AnalysisPass::Semantic => Capability::Semantics,
            AnalysisPass::Docs => Capability::StructuredDocs,
            AnalysisPass::Diagnostics => Capability::Diagnostics,
        }))
    }

    /// Selects only `kinds` (replacing any earlier selection); an empty set
    /// selects nothing.
    pub fn kinds(mut self, kinds: impl IntoIterator<Item = SymbolKindV2>) -> Self {
        self.kinds = Some(kinds.into_iter().collect());
        self
    }

    /// Requires `capabilities` and their prerequisites (replacing any earlier
    /// requirement); `Declarations` is always required.
    pub fn capabilities(mut self, capabilities: impl IntoIterator<Item = Capability>) -> Self {
        let mut closed = BTreeSet::from([Capability::Declarations]);
        let mut pending: Vec<Capability> = capabilities.into_iter().collect();
        while let Some(capability) = pending.pop() {
            if closed.insert(capability) {
                pending.extend_from_slice(capability.prerequisites());
            }
        }
        self.capabilities = closed;
        self
    }

    /// The selected kinds in canonical order; `None` when every kind is
    /// selected.
    pub fn selected_kinds(&self) -> Option<Vec<SymbolKindV2>> {
        self.kinds.as_ref().map(|kinds| kinds.iter().copied().collect())
    }

    /// Whether records of `kind` are part of the result.
    pub fn selects(&self, kind: SymbolKindV2) -> bool {
        self.kinds.as_ref().is_none_or(|kinds| kinds.contains(&kind))
    }

    /// The required capabilities, prerequisites included, in canonical order.
    pub fn required_capabilities(&self) -> Vec<Capability> {
        self.capabilities.iter().copied().collect()
    }

    /// Whether `capability` will be computed.
    pub fn requires(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }

    pub(crate) fn kind_set(&self) -> Option<&BTreeSet<SymbolKindV2>> {
        self.kinds.as_ref()
    }

    pub(crate) fn capability_set(&self) -> &BTreeSet<Capability> {
        &self.capabilities
    }

    /// Whether every kind `other` selects is selected here.
    pub(crate) fn selects_all_of(&self, other: &SymbolQuery) -> bool {
        match (&self.kinds, &other.kinds) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(mine), Some(theirs)) => theirs.is_subset(mine),
        }
    }

    /// The capabilities `other` requires that are not required here.
    pub(crate) fn capabilities_missing_for(&self, other: &SymbolQuery) -> BTreeSet<Capability> {
        other
            .capabilities
            .difference(&self.capabilities)
            .copied()
            .collect()
    }

    /// The kinds and capabilities of both queries.
    pub(crate) fn union(&self, other: &SymbolQuery) -> SymbolQuery {
        let kinds = match (&self.kinds, &other.kinds) {
            (Some(mine), Some(theirs)) => Some(mine.union(theirs).copied().collect()),
            _ => None,
        };
        SymbolQuery {
            kinds,
            capabilities: self.capabilities.union(&other.capabilities).copied().collect(),
        }
    }

    /// This query without `capability` (whose dependents must not be
    /// required).
    pub(crate) fn without(mut self, capability: Capability) -> SymbolQuery {
        self.capabilities.remove(&capability);
        self
    }
}

/// The outcome of a [`SymbolQuery`] over one file.
#[derive(Debug, Clone)]
pub struct SymbolQueryResult {
    /// The file's identity, language, hash, completed passes, import and
    /// export records, diagnostics (when required) and the selected records
    /// only.
    pub index: FileSymbolIndex,
    /// What was computed, in canonical order: an empty facet of a capability
    /// missing here was not computed, not found empty.
    pub completed_capabilities: Vec<Capability>,
    /// The kinds the query selected, in canonical order; `None` when it
    /// selected every kind.
    pub selected_kinds: Option<Vec<SymbolKindV2>>,
}

impl SymbolQueryResult {
    /// This result reduced to exactly what `query` computes over the same
    /// source, or `None` when `query` selects a kind or requires a capability
    /// this result lacks.
    ///
    /// Lets a caller that analyzed a file once for a wide query answer a
    /// narrower one without parsing it again.
    pub fn narrowed(self, query: &SymbolQuery) -> Option<SymbolQueryResult> {
        let covers_kinds = match (&self.selected_kinds, query.kind_set()) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(mine), Some(theirs)) => theirs.iter().all(|kind| mine.contains(kind)),
        };
        let covers_capabilities = query
            .capability_set()
            .iter()
            .all(|capability| self.completed_capabilities.contains(capability));
        (covers_kinds && covers_capabilities)
            .then(|| crate::analysis::coverage::project(self.index, query))
    }
}
