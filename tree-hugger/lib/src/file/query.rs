//! Selective symbol queries: which kinds a caller wants and which analysis
//! work it needs, so a listing runs no more than it shows, plus the
//! predicates and naming inputs applied to the result after analysis and
//! cache lookup.

use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::file::extract::is_test_file;
use crate::file::filter::{FUNCTION_LIKE_KINDS, RecordFilter};
use crate::file::naming::{Naming, NamingContext};
use crate::file::pattern::NamePattern;
use crate::shared::{
    AnalysisPass, FileSymbolIndex, SymbolId, SymbolKindV2, SymbolRecord, SymbolScope,
    VisibilityLevel,
};

/// A unit of analysis a query can require.
///
/// Requiring a capability also requires its prerequisites:
/// [`References`](Self::References) implies
/// [`Containment`](Self::Containment), [`Occurrences`](Self::Occurrences)
/// implies [`References`](Self::References), [`Semantics`](Self::Semantics)
/// implies [`Occurrences`](Self::Occurrences),
/// [`Diagnostics`](Self::Diagnostics) implies [`Reexports`](Self::Reexports)
/// (the `unused-import` exemption reads proven forwarding), and every
/// capability implies [`Declarations`](Self::Declarations).
///
/// [`Flags`](Self::Flags) and [`Metrics`](Self::Metrics) fill record fields
/// that otherwise hold their defaults: an unrequested `false` flag or absent
/// `metrics` facet means "not computed", never "computed and not set".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Declaration records with identity, spans, container name and the
    /// cheap metadata (keyword, signature text, declaration text, signature,
    /// type metadata, generic parameters, written modifiers, attributes, raw
    /// doc comment), plus `relations.extends`/`relations.implements` with
    /// same-file target IDs, resolved against every declaration of the file
    /// whether or not its kind is selected. The file's import and export
    /// records come with it.
    Declarations,
    /// `relations.parent`, `relations.container` and `relations.members`,
    /// including Rust impl members linked to their impl's resolved self type.
    Containment,
    /// The bind pass: containment plus reference, dependency and
    /// referenced-by edges and resolved export targets. The edges are
    /// derived from the file's reference occurrences, which are resolved
    /// either way and kept in the result only under
    /// [`Occurrences`](Self::Occurrences).
    References,
    /// The file's reference occurrences and their coverage
    /// (`FileSymbolIndex::occurrences` and `reference_coverage`), over the
    /// whole file whatever kinds are selected.
    Occurrences,
    /// The semantic pass (`semantics` facet of functions, methods and
    /// constructors), read from the occurrences.
    Semantics,
    /// The docs pass (`docs.parsed`).
    StructuredDocs,
    /// The diagnostics pass (lint and syntax diagnostics).
    Diagnostics,
    /// `visibility.is_test` and `visibility.is_deprecated`. Attributes, doc
    /// comments and Rust `cfg(test)` are read while the declarations are
    /// extracted (and cached); the file-wide test rules (file name, Rust
    /// package `tests/`) are applied to every result after analysis or cache
    /// lookup.
    Flags,
    /// The `metrics` facet (`parameters`, `has_docs`, `lines` and
    /// `cyclomatic`), read while the declarations are extracted.
    Metrics,
    /// The file's re-export report (`FileSymbolIndex::reexports`) and the
    /// `reexported` marker on its import records. Read from the definition
    /// and import captures and a same-file binding scan, over the whole file
    /// whatever kinds are selected; it needs no reference captures.
    Reexports,
}

impl Capability {
    /// Every capability, in canonical order.
    pub const ALL: [Capability; 10] = [
        Capability::Declarations,
        Capability::Containment,
        Capability::References,
        Capability::Occurrences,
        Capability::Semantics,
        Capability::StructuredDocs,
        Capability::Diagnostics,
        Capability::Flags,
        Capability::Metrics,
        Capability::Reexports,
    ];

    fn prerequisites(self) -> &'static [Capability] {
        match self {
            Capability::Declarations => &[],
            Capability::Containment => &[Capability::Declarations],
            Capability::References => &[Capability::Containment],
            Capability::Occurrences => &[Capability::References],
            Capability::Semantics => &[Capability::Occurrences],
            Capability::Diagnostics => &[Capability::Reexports],
            Capability::StructuredDocs
            | Capability::Flags
            | Capability::Metrics
            | Capability::Reexports => &[Capability::Declarations],
        }
    }
}

/// Which symbols a caller wants from a file and what must be computed for
/// them.
///
/// ## Examples
///
/// ```no_run
/// use tree_hugger::{
///     Capability, SourceFile, SymbolKindV2, SymbolQuery, SymbolScope, VisibilityLevel,
/// };
///
/// let query = SymbolQuery::new()
///     .kinds([SymbolKindV2::Function, SymbolKindV2::Method])
///     .scopes([SymbolScope::Item, SymbolScope::Member])
///     .visibility(VisibilityLevel::Public)
///     .exclude_tests(true)
///     .scan_root(".");
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
///
/// The predicates (scopes, visibility, flags, names, signature shape) and
/// the naming inputs ([`scan_root`](Self::scan_root),
/// [`naming_context`](Self::naming_context)) are applied to the result after
/// analysis and cache lookup, so they never change a cache entry or a symbol
/// ID. A predicate pushes down only what it reads: a signature-shape
/// predicate narrows the kinds to functions, methods and constructors, the
/// test and deprecation predicates require [`Capability::Flags`], and the
/// documentation and parameter-count predicates require
/// [`Capability::Metrics`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolQuery {
    /// As given to [`kinds`](Self::kinds); `None` selects every kind.
    requested_kinds: Option<BTreeSet<SymbolKindV2>>,
    /// As given to [`capabilities`](Self::capabilities), closed under
    /// prerequisites.
    requested_capabilities: BTreeSet<Capability>,
    /// The requested kinds narrowed by the predicates; `None` selects every
    /// kind.
    kinds: Option<BTreeSet<SymbolKindV2>>,
    /// The requested capabilities plus those the predicates read, closed
    /// under prerequisites; always holds `Declarations`.
    capabilities: BTreeSet<Capability>,
    filter: RecordFilter,
    naming: Naming,
}

impl Default for SymbolQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolQuery {
    /// Every kind and scope, declarations only, no predicate.
    pub fn new() -> Self {
        Self {
            requested_kinds: None,
            requested_capabilities: BTreeSet::from([Capability::Declarations]),
            kinds: None,
            capabilities: BTreeSet::from([Capability::Declarations]),
            filter: RecordFilter::default(),
            naming: Naming::default(),
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
    /// selects nothing. A signature-shape predicate still narrows the
    /// selection to function-like kinds.
    pub fn kinds(mut self, kinds: impl IntoIterator<Item = SymbolKindV2>) -> Self {
        self.requested_kinds = Some(kinds.into_iter().collect());
        self.settle()
    }

    /// Requires `capabilities` and their prerequisites (replacing any earlier
    /// requirement); `Declarations` is always required, as is what the
    /// predicates read.
    pub fn capabilities(mut self, capabilities: impl IntoIterator<Item = Capability>) -> Self {
        self.requested_capabilities = closed(capabilities);
        self.settle()
    }

    /// Keeps only records in `scopes` (replacing any earlier selection).
    pub fn scopes(mut self, scopes: impl IntoIterator<Item = SymbolScope>) -> Self {
        self.filter.scopes = Some(scopes.into_iter().collect());
        self
    }

    /// Also keeps records whose visibility level is exactly `level`; with no
    /// call, every level is kept. [`VisibilityLevel::Unknown`] matches only
    /// `Unknown`.
    pub fn visibility(mut self, level: VisibilityLevel) -> Self {
        if !self.filter.visibility.contains(&level) {
            self.filter.visibility.push(level);
        }
        self
    }

    /// Keeps only exported records (`visibility.is_exported`, re-exports
    /// included) when `exported`.
    pub fn exported(mut self, exported: bool) -> Self {
        self.filter.exported = exported;
        self
    }

    /// Drops test-only records when `exclude`; replaces
    /// [`tests_only`](Self::tests_only).
    pub fn exclude_tests(mut self, exclude: bool) -> Self {
        self.filter.is_test = exclude.then_some(false);
        self.settle()
    }

    /// Keeps only test-only records when `only`; replaces
    /// [`exclude_tests`](Self::exclude_tests).
    pub fn tests_only(mut self, only: bool) -> Self {
        self.filter.is_test = only.then_some(true);
        self.settle()
    }

    /// Keeps only records whose `has_docs` is `Some(false)` when
    /// `undocumented`; a language without doc-comment detection never
    /// matches.
    pub fn undocumented(mut self, undocumented: bool) -> Self {
        self.filter.undocumented = undocumented;
        self.settle()
    }

    /// Keeps only deprecated records when `deprecated`.
    pub fn deprecated(mut self, deprecated: bool) -> Self {
        self.filter.deprecated = deprecated;
        self.settle()
    }

    /// Keeps only `async` functions, methods and constructors when
    /// `is_async`.
    pub fn is_async(mut self, is_async: bool) -> Self {
        self.filter.is_async = is_async;
        self.settle()
    }

    /// Keeps only functions, methods and constructors whose declared return
    /// type text matches `pattern`, a whole-string, case-sensitive
    /// [`glob_match`](crate::glob_match) pattern. Missing type text never
    /// matches.
    pub fn returns(mut self, pattern: impl Into<String>) -> Self {
        self.filter.returns = Some(pattern.into());
        self.settle()
    }

    /// Keeps only functions, methods and constructors with a non-receiver
    /// parameter whose declared type text matches `pattern` (as in
    /// [`returns`](Self::returns)).
    pub fn param_type(mut self, pattern: impl Into<String>) -> Self {
        self.filter.param_type = Some(pattern.into());
        self.settle()
    }

    /// Keeps only functions, methods and constructors with at least
    /// `minimum` declared parameters; an unsupported count never matches.
    pub fn min_params(mut self, minimum: u32) -> Self {
        self.filter.min_params = Some(minimum);
        self.settle()
    }

    /// Also keeps members of containers matching `pattern`: the immediate
    /// `container_name`, or, for a qualified pattern, the qualified name of
    /// the enclosing declaration. A record without a container never matches.
    pub fn in_container(mut self, pattern: NamePattern) -> Self {
        self.filter.containers.push(pattern);
        self
    }

    /// Also keeps records whose name matches `pattern`; a qualified pattern
    /// (see [`NamePattern::is_qualified_for`]) matches the displayed
    /// `qualified_name`.
    pub fn name(mut self, pattern: NamePattern) -> Self {
        self.filter.names.push(pattern);
        self
    }

    /// Drops records whose name matches `pattern`, as in
    /// [`name`](Self::name); an exclusion wins over every inclusion.
    pub fn exclude_name(mut self, pattern: NamePattern) -> Self {
        self.filter.excluded_names.push(pattern);
        self
    }

    /// The directory the file prefix of a qualified name is relative to;
    /// without one, each file's own directory.
    pub fn scan_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.naming.scan_root = Some(root.into());
        self
    }

    /// Names files below the context's root by module path instead of by
    /// file path.
    pub fn naming_context(mut self, context: NamingContext) -> Self {
        self.naming.context = Some(context);
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

    /// Whether `record` passes every predicate (kinds aside). `record` must
    /// carry the capabilities the predicates read, as a result of this query
    /// does.
    pub fn matches(&self, record: &SymbolRecord) -> bool {
        self.filter.matches(record)
    }

    /// Recomputes the effective kinds and capabilities from the requested
    /// ones and the predicates.
    fn settle(mut self) -> Self {
        self.kinds = match (&self.requested_kinds, self.filter.needs_function_like()) {
            (requested, false) => requested.clone(),
            (None, true) => Some(FUNCTION_LIKE_KINDS.into_iter().collect()),
            (Some(requested), true) => Some(
                requested
                    .iter()
                    .copied()
                    .filter(|kind| FUNCTION_LIKE_KINDS.contains(kind))
                    .collect(),
            ),
        };
        self.capabilities = closed(
            self.requested_capabilities
                .iter()
                .copied()
                .chain(self.filter.capabilities()),
        );
        self
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

    /// The kinds and capabilities of both queries, without predicates or
    /// naming inputs.
    pub(crate) fn union(&self, other: &SymbolQuery) -> SymbolQuery {
        let query = SymbolQuery::new()
            .capabilities(self.capabilities.union(&other.capabilities).copied());
        match (&self.kinds, &other.kinds) {
            (Some(mine), Some(theirs)) => query.kinds(mine.union(theirs).copied()),
            _ => query,
        }
    }

    /// This query without `capability` (whose dependents must not be
    /// required).
    pub(crate) fn without(mut self, capability: Capability) -> SymbolQuery {
        self.requested_capabilities.remove(&capability);
        self.capabilities.remove(&capability);
        self
    }
}

/// `capabilities` plus their prerequisites and `Declarations`.
fn closed(capabilities: impl IntoIterator<Item = Capability>) -> BTreeSet<Capability> {
    let mut closed = BTreeSet::from([Capability::Declarations]);
    let mut pending: Vec<Capability> = capabilities.into_iter().collect();
    while let Some(capability) = pending.pop() {
        if closed.insert(capability) {
            pending.extend_from_slice(capability.prerequisites());
        }
    }
    closed
}

/// The outcome of a [`SymbolQuery`] over one file.
#[derive(Debug, Clone)]
pub struct SymbolQueryResult {
    /// The file's identity, language, hash, completed passes, import and
    /// export records, diagnostics and the reference occurrences with their
    /// coverage (each when required), and the selected records that pass the
    /// query's predicates. Each record's
    /// `identity.qualified_name` carries the [`name_prefix`](Self::name_prefix)
    /// unless it is namespace-rooted; relation and export references keep the
    /// in-file name.
    pub index: FileSymbolIndex,
    /// What was computed, in canonical order: an empty facet of a capability
    /// missing here was not computed, not found empty.
    pub completed_capabilities: Vec<Capability>,
    /// The kinds the query selected, in canonical order; `None` when it
    /// selected every kind.
    pub selected_kinds: Option<Vec<SymbolKindV2>>,
    /// `None` until [`finish`](Self::finish) applied a prefix.
    name_prefix: Option<String>,
    /// The predicates applied, when any were.
    filter: Option<RecordFilter>,
    /// `Some` when `finish` found the file a test file by a file-wide rule,
    /// holding the records that rule (not a source attribute) made tests, so
    /// the decision can be undone exactly when the package layout changes.
    file_wide_tests: Option<HashSet<SymbolId>>,
}

impl SymbolQueryResult {
    /// A result straight from analysis or the cache: in-file qualified names,
    /// no predicate applied.
    pub(crate) fn unfinished(
        index: FileSymbolIndex,
        completed_capabilities: Vec<Capability>,
        selected_kinds: Option<Vec<SymbolKindV2>>,
    ) -> Self {
        Self {
            index,
            completed_capabilities,
            selected_kinds,
            name_prefix: None,
            filter: None,
            file_wide_tests: None,
        }
    }

    /// The prefix this result's qualified names carry (not namespace-rooted
    /// ones): `src/symbol.rs#`, a naming context's `crate::symbol::`, or
    /// empty. Stripping it gives the in-file name the cache stores.
    pub fn name_prefix(&self) -> &str {
        self.name_prefix.as_deref().unwrap_or_default()
    }

    /// Applies the file-wide test classification, `query`'s naming inputs
    /// (once) and predicates.
    ///
    /// The file-wide test rules ([`is_test_file`]) read the file's path and,
    /// for Rust, the nearest `Cargo.toml` above it. Neither analysis nor the
    /// cache stores their outcome (a manifest can appear, move or vanish
    /// while the source and its entry stay unchanged), so they are decided
    /// here, on every call, before the predicates read `is_test`: a record is
    /// a test when its source marks it (attribute, Rust `cfg(test)`) or the
    /// file is a test file now. An earlier call's file-wide decision is
    /// undone first, so a source-marked record stays a test and a record
    /// only that decision marked does not.
    pub(crate) fn finish(mut self, query: &SymbolQuery) -> Self {
        self.undo_file_wide_tests();
        if self.completed_capabilities.contains(&Capability::Flags)
            && is_test_file(&self.index.file, self.index.language)
        {
            let mut marked = HashSet::new();
            for record in &mut self.index.symbols {
                if !record.visibility.is_test {
                    record.visibility.is_test = true;
                    marked.insert(record.id.clone());
                }
            }
            self.file_wide_tests = Some(marked);
        }
        if self.name_prefix.is_none() {
            let prefix = query.naming.prefix_for(&self.index.file, self.index.language);
            for record in &mut self.index.symbols {
                if !record.identity.is_namespace_rooted {
                    record.identity.qualified_name.insert_str(0, &prefix);
                }
            }
            self.name_prefix = Some(prefix);
        }
        if !query.filter.is_empty() {
            self.index.symbols.retain(|record| query.filter.matches(record));
            self.filter = Some(query.filter.clone());
        }
        self
    }

    /// Restores each record's source-derived `is_test`.
    fn undo_file_wide_tests(&mut self) {
        if let Some(marked) = self.file_wide_tests.take() {
            for record in &mut self.index.symbols {
                if marked.contains(&record.id) {
                    record.visibility.is_test = false;
                }
            }
        }
    }

    /// This result's index as analysis or the cache produced it: in-file
    /// qualified names and source-derived test flags (the predicates' drops
    /// stay dropped).
    fn into_unfinished_index(mut self) -> FileSymbolIndex {
        self.undo_file_wide_tests();
        if let Some(prefix) = self.name_prefix.take() {
            for record in &mut self.index.symbols {
                if !record.identity.is_namespace_rooted
                    && let Some(in_file) = record.identity.qualified_name.strip_prefix(&prefix)
                {
                    record.identity.qualified_name = in_file.to_string();
                }
            }
        }
        self.index
    }

    /// This result reduced to exactly what `query` computes over the same
    /// source, or `None` when `query` selects a kind or requires a capability
    /// this result lacks, or this result was filtered by other predicates,
    /// under another prefix, or by a test selector under a file-wide test
    /// decision that no longer holds. Qualified names are re-prefixed for
    /// `query`'s naming inputs.
    ///
    /// Lets a caller that analyzed a file once for a wide query answer a
    /// narrower one without parsing it again. The reuse contract: the
    /// narrowed result equals a fresh `query` over the same source *now*.
    /// Facts read from the source are reused; the file-wide test rules and
    /// the name prefix, which read the filesystem, are decided again, so a
    /// `Cargo.toml` added or removed since this result was made is honored.
    /// A `None` means "analyze again", never "nothing matches".
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
        // Filtered records are gone, and a qualified-name predicate matched
        // them under this result's prefix: only the same predicates under the
        // same prefix can be answered.
        // A test selector dropped records under this result's file-wide
        // test decision; if that decision changed, a dropped record may now
        // match.
        let covers_records = self.filter.as_ref().is_none_or(|filter| {
            *filter == query.filter
                && self.name_prefix.as_deref()
                    == Some(query.naming.prefix_for(&self.index.file, self.index.language).as_str())
                && (filter.is_test.is_none()
                    || self.file_wide_tests.is_some()
                        == is_test_file(&self.index.file, self.index.language))
        });
        (covers_kinds && covers_capabilities && covers_records).then(|| {
            crate::analysis::coverage::project(self.into_unfinished_index(), query).finish(query)
        })
    }
}
