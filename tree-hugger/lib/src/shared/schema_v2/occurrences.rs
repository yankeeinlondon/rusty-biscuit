//! Per-occurrence reference evidence: where each reference is written, which
//! code owns it, what it binds to and how sure the analysis is.
//!
//! User contract: `tree-hugger/docs/references.md`.

use serde::{Deserialize, Serialize};

use super::{SymbolRef, TextSpan};

/// One reference written in a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Occurrence {
    /// The nearest enclosing callable declaration record; absent for
    /// file-level code, an anonymous callable or an initializer with no
    /// established owner. Never a synthetic ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<SymbolRef>,
    /// The owning region: the callable's span, an initializer's span, or the
    /// whole file for file-level code. Two anonymous callables never share
    /// one.
    pub owner_span: TextSpan,
    /// The name as written. `qualified_name` is the written path or receiver
    /// expression including the name (`Self::new`, `other.run`), never the
    /// declaration's qualified name; `id` is set only for a `Scoped` binding
    /// to one in-file declaration record.
    pub target: SymbolRef,
    /// The name span of the particular import this occurrence binds to.
    /// Absent for in-file declarations, unreported local bindings and
    /// uncertain references; never inferred from a matching import name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_span: Option<TextSpan>,
    /// Same-named in-file declarations for a `NameOnly` occurrence, ordered
    /// by declaration position then ID. Their IDs are suggestions; empty for
    /// `Scoped` and `Unresolved`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<SymbolRef>,
    /// The referenced name token.
    pub span: TextSpan,
    pub kind: OccurrenceKind,
    pub confidence: Confidence,
}

/// Whether an occurrence invokes its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum OccurrenceKind {
    /// The target of a syntactic invocation, an explicit constructor
    /// invocation included.
    Call,
    /// Any other reference: a type, a value, a function used as a value, a
    /// macro invocation.
    Usage,
}

/// How firmly an occurrence's target is established.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Confidence {
    /// Syntax and the language's supported visibility rules establish one
    /// source-level binding (not which implementation runs).
    Scoped,
    /// Same-named declarations exist, but nothing establishes one of them.
    NameOnly,
    /// No supported binding and no in-file candidate.
    Unresolved,
}

/// The reference coverage of one owning region of a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceCoverage {
    /// As [`Occurrence::owner`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<SymbolRef>,
    /// As [`Occurrence::owner_span`]; regions are keyed by it.
    pub span: TextSpan,
    pub state: CoverageState,
    /// Why the region is `Limited`, ascending; empty exactly when it is
    /// `Complete`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<LimitReason>,
}

/// Whether every reference form in a region was extracted and assessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoverageState {
    /// Every relevant reference form in the region was extracted, and the
    /// supported rules suffice to assess its calls. Never inferred from the
    /// pass running or from finding nothing.
    Complete,
    Limited,
}

/// A machine-readable reason a region's coverage is limited. New reasons
/// may be added; existing ones keep their meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitReason {
    /// The language has no resolver rules.
    UnsupportedLanguage,
    /// The region uses a binding form the language rules do not support.
    UnsupportedBindingForm,
    /// The region has invocation syntax the reference query does not
    /// capture.
    MissingCallCapture,
    /// A call goes through a receiver, a variable or dynamic access.
    IndirectDispatch,
    /// A macro invocation whose expansion could hide calls.
    UnexpandedMacro,
    /// Code whose evaluating owner cannot be established.
    OwnershipUnknown,
    /// The region contains a syntax error.
    ParseError,
    /// The language has no reference query.
    EmptyReferenceQuery,
}

/// A file's occurrences and the coverage of each owning region, both over the
/// whole file and in their canonical order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceEvidence {
    pub occurrences: Vec<Occurrence>,
    pub reference_coverage: Vec<ReferenceCoverage>,
}
