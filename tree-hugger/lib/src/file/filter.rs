//! The one record filter behind [`SymbolQuery`](crate::SymbolQuery)'s
//! predicates, applied after analysis and cache lookup.
//!
//! Every predicate is an AND term; a list (scopes, visibility levels, names,
//! containers) matches when any of its values does, and a name exclusion
//! always wins.

use std::collections::BTreeSet;

use crate::file::pattern::{NamePattern, glob_match, qualified_name_separators};
use crate::file::query::Capability;
use crate::shared::{
    Modifier, ProgrammingLanguage, SymbolKindData, SymbolKindV2, SymbolRecord, SymbolScope,
    VisibilityLevel,
};

/// The kinds signature-shape predicates (`async`, return and parameter type,
/// parameter count) select.
pub(crate) const FUNCTION_LIKE_KINDS: [SymbolKindV2; 3] = [
    SymbolKindV2::Function,
    SymbolKindV2::Method,
    SymbolKindV2::Constructor,
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RecordFilter {
    /// `None` keeps every scope.
    pub(crate) scopes: Option<BTreeSet<SymbolScope>>,
    /// Empty keeps every level.
    pub(crate) visibility: Vec<VisibilityLevel>,
    pub(crate) exported: bool,
    /// `Some(wanted)` keeps records whose `is_test` is `wanted`.
    pub(crate) is_test: Option<bool>,
    pub(crate) undocumented: bool,
    pub(crate) deprecated: bool,
    pub(crate) is_async: bool,
    pub(crate) returns: Option<String>,
    pub(crate) param_type: Option<String>,
    pub(crate) min_params: Option<u32>,
    pub(crate) containers: Vec<NamePattern>,
    pub(crate) names: Vec<NamePattern>,
    pub(crate) excluded_names: Vec<NamePattern>,
}

impl RecordFilter {
    pub(crate) fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Whether a signature-shape predicate is set, which limits the query to
    /// [`FUNCTION_LIKE_KINDS`].
    pub(crate) fn needs_function_like(&self) -> bool {
        self.is_async || self.returns.is_some() || self.param_type.is_some() || self.min_params.is_some()
    }

    /// The capabilities the predicates read.
    pub(crate) fn capabilities(&self) -> Vec<Capability> {
        let mut needed = Vec::new();
        if self.is_test.is_some() || self.deprecated {
            needed.push(Capability::Flags);
        }
        if self.undocumented || self.min_params.is_some() {
            needed.push(Capability::Metrics);
        }
        needed
    }

    pub(crate) fn matches(&self, record: &SymbolRecord) -> bool {
        let function_like = FUNCTION_LIKE_KINDS.contains(&record.kind);
        let metrics = record.metrics.as_ref();
        self.scopes.as_ref().is_none_or(|scopes| scopes.contains(&record.scope))
            && (self.visibility.is_empty() || self.visibility.contains(&record.visibility.level))
            && (!self.exported || record.visibility.is_exported)
            && self.is_test.is_none_or(|wanted| record.visibility.is_test == wanted)
            && (!self.deprecated || record.visibility.is_deprecated)
            // An unsupported `has_docs` (`None`) is never undocumented.
            && (!self.undocumented || metrics.and_then(|metrics| metrics.has_docs) == Some(false))
            && (!self.is_async
                || function_like && record.modifiers.iter().any(|modifier| matches!(modifier, Modifier::Async)))
            && self.min_params.is_none_or(|minimum| {
                function_like && metrics.and_then(|metrics| metrics.parameters).is_some_and(|count| count >= minimum)
            })
            && self.returns.as_deref().is_none_or(|pattern| {
                function_like && return_type(record).is_some_and(|text| glob_match(pattern, text))
            })
            && self.param_type.as_deref().is_none_or(|pattern| {
                function_like && parameter_types(record).any(|text| glob_match(pattern, text))
            })
            && (self.containers.is_empty()
                || self.containers.iter().any(|pattern| container_matches(pattern, record)))
            && (self.names.is_empty() || self.names.iter().any(|pattern| name_matches(pattern, record)))
            && !self.excluded_names.iter().any(|pattern| name_matches(pattern, record))
    }
}

fn return_type(record: &SymbolRecord) -> Option<&str> {
    match &record.kind_data {
        SymbolKindData::Function(data) => data.return_info.type_text.as_deref(),
        _ => None,
    }
}

/// The declared type text of each parameter other than a receiver (Rust's
/// `self`, which the parameter list carries; Go's receiver is not in it).
fn parameter_types(record: &SymbolRecord) -> impl Iterator<Item = &str> {
    let parameters = match &record.kind_data {
        SymbolKindData::Function(data) => data.parameters.as_slice(),
        _ => &[],
    };
    let is_rust = record.language == ProgrammingLanguage::Rust;
    parameters
        .iter()
        .filter(move |parameter| !(is_rust && parameter.name == "self"))
        .filter_map(|parameter| parameter.type_text.as_deref())
}

/// A pattern naming a qualified name matches the displayed `qualified_name`;
/// any other matches the bare name.
pub(crate) fn name_matches(pattern: &NamePattern, record: &SymbolRecord) -> bool {
    if pattern.is_qualified_for(record.language) {
        pattern.matches(&record.identity.qualified_name)
    } else {
        pattern.matches(&record.identity.name)
    }
}

/// A record without a container never matches. A qualified pattern matches
/// the qualified name of the enclosing named declaration: the record's
/// `qualified_name` without its own name and the separator before it.
fn container_matches(pattern: &NamePattern, record: &SymbolRecord) -> bool {
    let Some(container) = record.identity.container_name.as_deref() else {
        return false;
    };
    if !pattern.is_qualified_for(record.language) {
        return pattern.matches(container);
    }
    enclosing_qualified_name(record).is_some_and(|enclosing| pattern.matches(enclosing))
}

fn enclosing_qualified_name(record: &SymbolRecord) -> Option<&str> {
    let qualified = &record.identity.qualified_name;
    let without_name = qualified.strip_suffix(record.identity.name.as_str())?;
    std::iter::once("#")
        .chain(qualified_name_separators(record.language).iter().copied())
        .find_map(|separator| without_name.strip_suffix(separator))
        .filter(|enclosing| !enclosing.is_empty())
}
