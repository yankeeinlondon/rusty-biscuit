//! Why a document enters canonical preparation, and the stages that entry runs.
//!
//! The spec's stage matrix lives here as **data**, not as `if` branches spread
//! across the pipeline. A scattered matrix is how the motivating bug happened:
//! the proxy route and the direct route each grew their own answer to "does
//! this run `initialize`?" and drifted apart. One table with one row per entry
//! reason makes divergence a diff rather than an archaeology exercise.
//!
//! See `features/2026-07-13-proxy-with/spec.md` → R3.

#[cfg(test)]
mod tests;

/// Why a document is entering canonical preparation.
///
/// Every entry reason has exactly one row in [`STAGE_MATRIX`]; there is no
/// fall-through to another reason's policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocumentEntryReason {
    /// The document the caller named on the command line.
    Direct,
    /// A document adopted from a committed proxy handoff.
    ProxyTarget,
    /// The same document, re-entered with a fresh provider attempt.
    Retry,
    /// The same document, re-entered against a compatible live session.
    Resume,
    /// The same document, refreshed for the next document-loop iteration.
    LoopIteration,
}

/// What canonical preparation reads to build the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceBasis {
    /// The source the caller resolved and loaded, plus the caller's input
    /// layers. Nothing has run yet that could have mutated it.
    CallerResolved,
    /// A fresh read from disk, because a preceding run may have mutated the
    /// document (an `initialize` side effect, an inline closure rewrite, a
    /// loop mutation).
    FreshRead,
    /// The structural plan stamped when the loop's active document was first
    /// prepared. A loop iteration re-materializes against it and therefore
    /// cannot introduce command bytes the audit never saw.
    StampedStructuralPlan,
}

/// Who decides document-loop ownership for this entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopOwnership {
    /// Recognize `loop:` from this document's own stabilized frontmatter —
    /// this document is (or is not) a loop owner in its own right.
    Recognize,
    /// Keep the loop the active document already owns.
    InheritActive,
    /// Reuse the owning loop's stamped structural plan.
    ReuseStamped,
}

/// The stages one entry reason runs, as a row of the spec's stage matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparationStages {
    /// What the document is built from.
    pub source_basis: SourceBasis,
    /// Whether this entry emits the document's `initialize` signal.
    ///
    /// `initialize` runs once per *active document*, not once per attempt: a
    /// retry or resume re-enters a document that has already initialized.
    pub emits_initialize: bool,
    /// Whether this entry runs schema validation and the full shell audit.
    ///
    /// A loop iteration does not: it re-materializes against an already-audited
    /// structural plan, so there is nothing new to validate or approve.
    pub full_validation: bool,
    /// Who owns the document loop for this entry.
    pub loop_ownership: LoopOwnership,
}

/// The spec's stage matrix: one row per [`DocumentEntryReason`].
///
/// Direct and `ProxyTarget` are deliberately identical apart from the read
/// basis — that identity *is* R3's equivalence contract. A proxy target reads
/// fresh because the handoff commits to a document the source may never have
/// touched; a direct document is already loaded.
const STAGE_MATRIX: &[(DocumentEntryReason, PreparationStages)] = &[
    (
        DocumentEntryReason::Direct,
        PreparationStages {
            source_basis: SourceBasis::CallerResolved,
            emits_initialize: true,
            full_validation: true,
            loop_ownership: LoopOwnership::Recognize,
        },
    ),
    (
        DocumentEntryReason::ProxyTarget,
        PreparationStages {
            source_basis: SourceBasis::FreshRead,
            emits_initialize: true,
            full_validation: true,
            loop_ownership: LoopOwnership::Recognize,
        },
    ),
    (
        DocumentEntryReason::Retry,
        PreparationStages {
            source_basis: SourceBasis::FreshRead,
            emits_initialize: false,
            full_validation: true,
            loop_ownership: LoopOwnership::InheritActive,
        },
    ),
    (
        DocumentEntryReason::Resume,
        PreparationStages {
            source_basis: SourceBasis::FreshRead,
            emits_initialize: false,
            full_validation: true,
            loop_ownership: LoopOwnership::InheritActive,
        },
    ),
    (
        DocumentEntryReason::LoopIteration,
        PreparationStages {
            source_basis: SourceBasis::StampedStructuralPlan,
            emits_initialize: false,
            full_validation: false,
            loop_ownership: LoopOwnership::ReuseStamped,
        },
    ),
];

impl DocumentEntryReason {
    /// Every entry reason, for exhaustive matrix coverage.
    pub const ALL: &'static [DocumentEntryReason] = &[
        DocumentEntryReason::Direct,
        DocumentEntryReason::ProxyTarget,
        DocumentEntryReason::Retry,
        DocumentEntryReason::Resume,
        DocumentEntryReason::LoopIteration,
    ];

    /// This entry reason's row of the stage matrix.
    ///
    /// ## Panics
    ///
    /// Panics when a variant has no row. [`stage_matrix_covers_every_entry_reason`]
    /// makes that unreachable in a tested build.
    ///
    /// [`stage_matrix_covers_every_entry_reason`]: super::tests
    pub fn stages(self) -> PreparationStages {
        STAGE_MATRIX
            .iter()
            .find(|(reason, _)| *reason == self)
            .map(|(_, stages)| *stages)
            .expect("every DocumentEntryReason has a STAGE_MATRIX row")
    }

    /// A stable label for tracing and diagnostics.
    pub fn label(self) -> &'static str {
        match self {
            DocumentEntryReason::Direct => "direct",
            DocumentEntryReason::ProxyTarget => "proxy-target",
            DocumentEntryReason::Retry => "retry",
            DocumentEntryReason::Resume => "resume",
            DocumentEntryReason::LoopIteration => "loop-iteration",
        }
    }
}
