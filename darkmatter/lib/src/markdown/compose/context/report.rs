//! Compose execution report (`ComposeReport`), source mapping (`SourceRange`),
//! and non-fatal `ComposeWarning`.

use super::super::cache::CacheStats;
use super::super::perf::ComposePerfReport;
use crate::markdown::normalize::NormalizationReport;
use crate::markdown::schemas::SchemaAdvisory;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Report of changes made during compose execution.
///
/// Contains counts of changes made by each stage and any warnings
/// generated during processing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ComposeReport {
    /// Number of frontmatter interpolation expressions resolved.
    pub frontmatter_interpolations_applied: usize,

    /// Number of frontmatter shell expansions applied.
    pub frontmatter_shell_expansions_applied: usize,

    /// Number of text replacements applied.
    pub replacements_applied: usize,

    /// Number of interpolations resolved.
    pub interpolations_applied: usize,

    /// Number of toc-linking directives expanded.
    pub toc_links_generated: usize,

    /// Number of shell expansions applied.
    pub shell_expansions_applied: usize,

    /// Number of shell blocks applied.
    pub shell_blocks_applied: usize,

    /// Number of shell approvals used.
    pub shell_approvals_used: usize,

    /// Whether the cleanup stage modified the content.
    pub cleanup_changed: bool,

    /// Normalization report if normalization was performed.
    pub normalization_report: Option<NormalizationReport>,

    /// Number of page blocks that evaluated to true and were rendered.
    pub page_blocks_rendered: usize,

    /// Number of page blocks that evaluated to false and were skipped.
    pub page_blocks_skipped: usize,

    /// Number of transclusions applied.
    pub transclusions_applied: usize,

    /// Number of transclusions skipped (conditions/invalid ignored).
    pub transclusions_skipped: usize,

    /// Number of local links resolved to absolute paths.
    pub link_resolves_applied: usize,

    /// Number of absolute paths normalized to portable forms.
    pub link_normalizations_applied: usize,

    /// Maximum recursive transclusion depth observed.
    pub max_transclusion_depth: usize,

    /// Warnings generated during compose (non-fatal issues).
    ///
    /// Add through [`Self::add_warning`] so a repeated issue collapses.
    pub warnings: Vec<ComposeWarning>,

    /// Where each coded issue in `warnings` first appears, so `add_warning`
    /// checks membership in O(1) instead of scanning `warnings`.
    warning_index: WarningIndex,

    /// Cache statistics from this compose run.
    pub cache_stats: Option<CacheStats>,

    /// Performance timings when `ComposeOptions::perf_enabled` is `true`.
    pub perf: Option<ComposePerfReport>,

    /// Source map tracking which byte ranges came from transcluded files.
    pub source_map: Vec<SourceRange>,

    /// Remote-fetch statistics from this compose run.
    pub remote_fetch_stats: Option<super::super::remote_fetch::RemoteFetchStats>,

    /// Top-level frontmatter keys intentionally deferred from compose-time
    /// resolution (via `ComposeOptions::with_exclude_keys`). Their values
    /// survive raw in `effective_frontmatter` for caller-owned event-time
    /// interpolation. Lets callers distinguish "raw because deferred" from
    /// "raw because composition failed". Empty when no keys were deferred.
    pub deferred_frontmatter_keys: std::collections::HashSet<String>,

    /// Reads of roots unknown when they were evaluated (spec Requirement 4).
    /// Frontmatter pass 1 runs before the final state and schema exist, so
    /// these are candidates, not warnings: each document's pipeline
    /// reconciles its own before its report is merged into a parent, so a
    /// merged child report carries none.
    pub(crate) unknown_root_candidates: UnknownRootCandidates,
}

/// First-read unknown-root candidates in read order, one per root.
///
/// The root set is private to this type so it cannot drift from the list.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct UnknownRootCandidates {
    list: Vec<UnknownRootCandidate>,
    roots: HashSet<String>,
}

impl UnknownRootCandidates {
    pub(crate) fn len(&self) -> usize {
        self.list.len()
    }

    /// Removes every candidate, in first-read order.
    pub(crate) fn take(&mut self) -> Vec<UnknownRootCandidate> {
        self.roots.clear();
        std::mem::take(&mut self.list)
    }

    /// Rewrites the locus of each candidate recorded at or after `start`.
    /// Only the locus is exposed, so a root can never change under the set.
    pub(crate) fn relocate_from(&mut self, start: usize, relocate: impl Fn(&CandidateLocus) -> CandidateLocus) {
        for candidate in &mut self.list[start..] {
            candidate.locus = relocate(&candidate.locus);
        }
    }

    fn contains(&self, root: &str) -> bool {
        note_identity_work();
        self.roots.contains(root)
    }

    /// Appends `candidate` unless its root is already recorded.
    fn push_first(&mut self, candidate: UnknownRootCandidate) {
        if !self.contains(&candidate.root) {
            self.roots.insert(candidate.root.clone());
            self.list.push(candidate);
        }
    }
}

/// `(source, code, identity)`: the key two warnings must share to report
/// the same issue.
type IssueKey = (Option<String>, Option<String>, WarningIdentity);

/// Maps each coded issue to the position of its first warning.
///
/// A cache over [`ComposeReport::warnings`], which is public and also grows
/// through direct `push`/`extend`. Entries past `covered` are indexed on the
/// next lookup, a shrunken vector or a position that no longer holds its
/// issue triggers a rebuild, and report equality ignores the index. An
/// element replaced in place outside `add_warning` is not seen until one of
/// those rebuilds.
#[derive(Debug, Clone, Default)]
struct WarningIndex {
    first: HashMap<IssueKey, usize>,
    covered: usize,
}

impl PartialEq for WarningIndex {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl WarningIndex {
    /// Whether `warnings` already reports `key`; if not, records it at the
    /// position the caller is about to push to.
    fn admit(&mut self, warnings: &[ComposeWarning], key: IssueKey) -> bool {
        if self.covered > warnings.len() {
            self.rebuild(warnings);
        }
        self.index_tail(warnings);
        note_identity_work();
        if let Some(&position) = self.first.get(&key) {
            if warnings.get(position).is_some_and(|warning| warning.has_issue_key(&key)) {
                return false;
            }
            self.rebuild(warnings);
            note_identity_work();
            if self.first.contains_key(&key) {
                return false;
            }
        }
        self.first.insert(key, warnings.len());
        self.covered = warnings.len() + 1;
        true
    }

    fn rebuild(&mut self, warnings: &[ComposeWarning]) {
        self.first.clear();
        self.covered = 0;
        self.index_tail(warnings);
    }

    fn index_tail(&mut self, warnings: &[ComposeWarning]) {
        for (position, warning) in warnings.iter().enumerate().skip(self.covered) {
            if let Some(key) = warning.issue_key() {
                note_identity_work();
                self.first.entry(key).or_insert(position);
            }
        }
        self.covered = warnings.len();
    }
}

#[cfg(test)]
thread_local! {
    /// Identity probes (hash lookups, inserts, and verifications) made on
    /// this thread; the complexity regressions read it.
    static IDENTITY_WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[inline]
fn note_identity_work() {
    #[cfg(test)]
    IDENTITY_WORK.with(|work| work.set(work.get() + 1));
}

/// A read of a root that nothing known when it was evaluated defined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnknownRootCandidate {
    pub(crate) root: String,
    pub(crate) stage: &'static str,
    pub(crate) locus: CandidateLocus,
}

/// Where an unknown-root candidate was read, as precisely as the reading
/// surface can prove.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CandidateLocus {
    /// A one-based line of the document file.
    Line(usize),
    /// A one-based line of the body the recording stage parsed. The stage
    /// converts it with `unknown_identifiers::directive_locus` before it
    /// rewrites that body; reconciliation treats a leftover as `Document`.
    BodyLine(usize),
    /// A top-level frontmatter key's value.
    FrontmatterKey(String),
    /// Somewhere in the document; no authored line is provable.
    Document,
}

/// Maps a byte range in composed output to its originating source file.
///
/// Populated by `BlockTransclusion` when file content replaces a
/// `::file` directive. Byte positions refer to the final composed content.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceRange {
    /// Start byte offset in the composed output (inclusive).
    pub byte_start: usize,
    /// End byte offset in the composed output (exclusive).
    pub byte_end: usize,
    /// The source file whose content occupies this range.
    pub source_file: PathBuf,
    /// The starting line number in the source file (1-based).
    pub source_start_line: usize,
}

impl ComposeReport {
    /// Creates a new empty report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if any changes were made by any stage.
    pub fn has_changes(&self) -> bool {
        self.frontmatter_interpolations_applied > 0
            || self.frontmatter_shell_expansions_applied > 0
            || self.replacements_applied > 0
            || self.interpolations_applied > 0
            || self.toc_links_generated > 0
            || self.shell_expansions_applied > 0
            || self.shell_blocks_applied > 0
            || self.link_resolves_applied > 0
            || self.link_normalizations_applied > 0
            || self.cleanup_changed
            || self.page_blocks_rendered > 0
            || self.transclusions_applied > 0
            || self
                .normalization_report
                .as_ref()
                .is_some_and(|r| r.has_changes())
    }

    /// Returns a summary of changes made.
    pub fn summary(&self) -> String {
        if !self.has_changes() {
            return "No changes made".to_string();
        }

        let mut parts = Vec::new();

        if self.frontmatter_interpolations_applied > 0 {
            parts.push(format!(
                "{} frontmatter interpolation(s)",
                self.frontmatter_interpolations_applied
            ));
        }

        if self.frontmatter_shell_expansions_applied > 0 {
            parts.push(format!(
                "{} frontmatter shell expansion(s)",
                self.frontmatter_shell_expansions_applied
            ));
        }

        if self.replacements_applied > 0 {
            parts.push(format!("{} replacement(s)", self.replacements_applied));
        }

        if self.interpolations_applied > 0 {
            parts.push(format!("{} interpolation(s)", self.interpolations_applied));
        }

        if self.toc_links_generated > 0 {
            parts.push(format!("{} toc-link(s)", self.toc_links_generated));
        }

        if self.shell_expansions_applied > 0 {
            parts.push(format!(
                "{} shell expansion(s)",
                self.shell_expansions_applied
            ));
        }

        if self.shell_blocks_applied > 0 {
            parts.push(format!("{} shell block(s)", self.shell_blocks_applied));
        }

        if self.shell_approvals_used > 0 {
            parts.push(format!("{} shell approval(s)", self.shell_approvals_used));
        }

        if self.link_resolves_applied > 0 {
            parts.push(format!("{} link resolve(s)", self.link_resolves_applied));
        }

        if self.link_normalizations_applied > 0 {
            parts.push(format!(
                "{} link normalization(s)",
                self.link_normalizations_applied
            ));
        }

        if self.cleanup_changed {
            parts.push("cleanup applied".to_string());
        }

        if self.page_blocks_rendered > 0 {
            parts.push(format!(
                "{} page block(s) rendered",
                self.page_blocks_rendered
            ));
        }

        if self.page_blocks_skipped > 0 {
            parts.push(format!(
                "{} page block(s) skipped",
                self.page_blocks_skipped
            ));
        }

        if self.transclusions_applied > 0 {
            parts.push(format!("{} transclusion(s)", self.transclusions_applied));
        }

        if self.transclusions_skipped > 0 {
            parts.push(format!(
                "{} transclusion(s) skipped",
                self.transclusions_skipped
            ));
        }

        if let Some(ref norm) = self.normalization_report
            && norm.has_changes()
        {
            parts.push(format!("normalization: {}", norm.summary()));
        }

        if let Some(ref stats) = self.cache_stats
            && stats.has_activity()
        {
            parts.push(format!(
                "cache: {} hit(s), {} miss(es)",
                stats.hits, stats.misses
            ));
        }

        if let Some(ref rf_stats) = self.remote_fetch_stats
            && (rf_stats.fetched > 0 || rf_stats.cache_hits > 0 || rf_stats.not_modified > 0)
        {
            parts.push(format!(
                "remote: {} fetched, {} cached, {} revalidated ({} not-modified, {} stale), \
                 {} denied, {} failed",
                rf_stats.fetched,
                rf_stats.cache_hits,
                rf_stats.revalidations,
                rf_stats.not_modified,
                rf_stats.stale_served,
                rf_stats.policy_denials,
                rf_stats.failures
            ));
        }

        parts.join(", ")
    }

    /// Adds a warning to the report, unless it reports an issue this report
    /// already carries.
    ///
    /// Two warnings report the same issue when both declare a
    /// [`WarningIdentity`] and their `(source, code, subject)` keys match; the
    /// first one added is kept, so its location is the one displayed. Warnings
    /// without an identity are always added.
    pub fn add_warning(&mut self, warning: ComposeWarning) {
        if let Some(key) = warning.issue_key()
            && !self.warning_index.admit(&self.warnings, key)
        {
            return;
        }
        self.warnings.push(warning);
    }

    /// [`Self::add_warning`] for each warning, in order.
    pub(crate) fn add_warnings(&mut self, warnings: impl IntoIterator<Item = ComposeWarning>) {
        for warning in warnings {
            self.add_warning(warning);
        }
    }

    /// Adds a schema advisory unless this report already carries the same
    /// semantic code and referenced path.
    pub(crate) fn add_schema_advisory(
        &mut self,
        advisory: &SchemaAdvisory,
        consumer: impl Into<PathBuf>,
    ) {
        self.add_warning(ComposeWarning::from_schema_advisory(advisory, consumer));
    }

    /// Attributes every document-scoped warning identity that does not yet
    /// name a document to `document`, then drops warnings that the
    /// attribution made duplicates.
    ///
    /// Each document's pipeline calls this before its report is merged into a
    /// parent, so a child's issues stay distinct from the parent's and from a
    /// sibling's, while repeats inside one document collapse.
    pub(crate) fn attribute_to_document(&mut self, document: &std::path::Path) {
        let mut attributed = Vec::with_capacity(self.warnings.len());
        for mut warning in std::mem::take(&mut self.warnings) {
            if let Some(identity) = warning.identity.as_mut() {
                identity.subject.attribute_to(document);
            }
            attributed.push(warning);
        }
        self.add_warnings(attributed);
    }

    /// Merges another report into this one.
    pub fn merge(&mut self, mut other: ComposeReport) {
        self.frontmatter_interpolations_applied += other.frontmatter_interpolations_applied;
        self.frontmatter_shell_expansions_applied += other.frontmatter_shell_expansions_applied;
        self.replacements_applied += other.replacements_applied;
        self.interpolations_applied += other.interpolations_applied;
        self.toc_links_generated += other.toc_links_generated;
        self.shell_expansions_applied += other.shell_expansions_applied;
        self.shell_blocks_applied += other.shell_blocks_applied;
        self.shell_approvals_used += other.shell_approvals_used;
        self.link_resolves_applied += other.link_resolves_applied;
        self.link_normalizations_applied += other.link_normalizations_applied;
        self.cleanup_changed |= other.cleanup_changed;
        self.page_blocks_rendered += other.page_blocks_rendered;
        self.page_blocks_skipped += other.page_blocks_skipped;
        self.transclusions_applied += other.transclusions_applied;
        self.transclusions_skipped += other.transclusions_skipped;
        self.max_transclusion_depth = self
            .max_transclusion_depth
            .max(other.max_transclusion_depth);

        if self.normalization_report.is_none() {
            self.normalization_report = other.normalization_report.take();
        }

        self.add_warnings(other.warnings.drain(..));

        // Merge cache stats
        match (&mut self.cache_stats, other.cache_stats) {
            (Some(self_stats), Some(ref other_stats)) => self_stats.merge(other_stats),
            (None, Some(other_stats)) => self.cache_stats = Some(other_stats),
            _ => {}
        }

        // Merge perf reports
        match (&mut self.perf, other.perf) {
            (Some(self_perf), Some(ref other_perf)) => self_perf.merge(other_perf),
            (None, Some(other_perf)) => self.perf = Some(other_perf),
            _ => {}
        }

        // Deferred keys accumulate across recursive child pipelines.
        self.deferred_frontmatter_keys
            .extend(other.deferred_frontmatter_keys);
        for candidate in other.unknown_root_candidates.take() {
            self.unknown_root_candidates.push_first(candidate);
        }
    }

    /// Records the unknown-root `roots` one stage read, each at `locus(root)`.
    ///
    /// Only a root's first read is kept (and located): the document warns
    /// once per root, at its first authored location.
    pub(crate) fn add_unknown_root_candidates(
        &mut self,
        stage: &'static str,
        roots: Vec<crate::markdown::compose::expression::absence::MissingRoot>,
        locus: impl Fn(&crate::markdown::compose::expression::absence::MissingRoot) -> CandidateLocus,
    ) {
        for root in roots {
            if self.unknown_root_candidates.contains(&root.root) {
                continue;
            }
            let locus = locus(&root);
            self.unknown_root_candidates.push_first(UnknownRootCandidate {
                root: root.root,
                stage,
                locus,
            });
        }
    }
}

/// A warning generated during compose processing.
///
/// Warnings indicate non-fatal issues that did not prevent the
/// transform from completing. A failing expression in a full document is
/// never one: it fails composition whatever `fail_fast` says. The expression
/// failure codes are emitted only by best-effort callers (a lenient subtree,
/// preflight discovery).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeWarning {
    /// The stage that generated this warning.
    pub stage: String,

    /// Human-readable description of the issue.
    pub message: String,

    /// Line number where the issue occurred (1-indexed), if applicable.
    pub line_number: Option<usize>,

    /// Stable producer identity when the warning originates from a typed
    /// diagnostic source.
    pub source: Option<String>,

    /// Stable machine-readable code when available.
    pub code: Option<String>,

    /// Referenced file associated with the warning, when applicable.
    pub path: Option<PathBuf>,

    /// Root document that consumed the advisory.
    pub consumer: Option<PathBuf>,

    /// Which issue this warning reports, when its family declares one.
    /// Set only by a family's constructor; see [`WarningIdentity`].
    pub(crate) identity: Option<WarningIdentity>,
}

/// The issue a coded warning reports, so each issue is reported once.
///
/// A report keys a warning by `(source, code, subject)`. Each coded-warning
/// family chooses its subject in its constructor. The key never includes the
/// message: message is prose, not identity, and keying on it would merge
/// distinct advisories and split reworded ones.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct WarningIdentity {
    pub(crate) subject: WarningSubject,
}

/// What a coded warning is about.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum WarningSubject {
    /// A referenced file. Schema advisories use the referenced schema, which
    /// keeps their key exactly `(source, code, path)`, and so they collapse
    /// across every document that references that schema.
    Path(PathBuf),
    /// An unresolved identifier root, once per source document no matter how
    /// many times it is referenced there.
    Root {
        /// Source document; `None` until the document's pipeline attributes it.
        document: Option<PathBuf>,
        /// The normalized root name.
        name: String,
    },
    /// One expression, once per source document no matter how many times a
    /// rescan evaluates it.
    Expression {
        /// Source document; `None` until the document's pipeline attributes it.
        document: Option<PathBuf>,
        /// Which scanned text `origin` indexes, when a document is scanned
        /// as more than one text (a frontmatter key path); `None` for the body.
        scope: Option<String>,
        origin: ExpressionOrigin,
    },
}

/// Where an expression was first observed in a scanned text.
///
/// Byte offsets into the text as the caller supplied it, never into the
/// buffer a rescan rewrites.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum ExpressionOrigin {
    /// The expression is authored in the scanned text.
    Authored(std::ops::Range<usize>),
    /// A replacement generated the expression; `span` is where the rescan at
    /// `pass` first observed it. Kept distinct from `Authored` so a generated
    /// expression never aliases an authored one at the same offsets.
    Generated {
        pass: usize,
        span: std::ops::Range<usize>,
    },
}

impl WarningSubject {
    fn attribute_to(&mut self, source_document: &std::path::Path) {
        match self {
            Self::Path(_) => {}
            Self::Root { document, .. } | Self::Expression { document, .. } => {
                if document.is_none() {
                    *document = Some(source_document.to_path_buf());
                }
            }
        }
    }
}

impl ComposeWarning {
    /// Code of the `unknown context variable` warning.
    pub const UNKNOWN_CONTEXT_VARIABLE_CODE: &'static str = "dm.expression.unknown_context_variable";
    /// Code of the unknown-identifier warning, shared with DMLS.
    pub const UNKNOWN_IDENTIFIER_CODE: &'static str = "dm.expression.unknown_identifier";
    /// Producer of the unknown-identifier warning.
    pub const EXPRESSION_SOURCE: &'static str = "darkmatter.expression";
    /// Code of a lenient expression parse failure.
    pub const EXPRESSION_PARSE_FAILURE_CODE: &'static str = "dm.expression.parse_failure";
    /// Code of a lenient expression evaluation failure.
    pub const EXPRESSION_EVALUATION_FAILURE_CODE: &'static str =
        "dm.expression.evaluation_failure";

    /// Creates a new warning.
    pub fn new(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            message: message.into(),
            line_number: None,
            source: None,
            code: None,
            path: None,
            consumer: None,
            identity: None,
        }
    }

    /// Projects a typed schema advisory into the compose warning model.
    pub(crate) fn from_schema_advisory(
        advisory: &SchemaAdvisory,
        consumer: impl Into<PathBuf>,
    ) -> Self {
        let path = std::fs::canonicalize(advisory.path())
            .unwrap_or_else(|_| advisory.path().to_path_buf());
        Self {
            stage: "schema_validation".to_string(),
            message: advisory.message(),
            line_number: None,
            source: Some(advisory.source().to_string()),
            code: Some(advisory.code().to_string()),
            identity: Some(WarningIdentity {
                subject: WarningSubject::Path(path.clone()),
            }),
            path: Some(path),
            consumer: Some(consumer.into()),
        }
    }

    /// An `unknown context variable` warning for the `ctx.*` group `name`.
    ///
    /// One per source document per group: `ctx.toady` and `ctx.toady.x`
    /// referenced anywhere in one document are one issue.
    pub(crate) fn unknown_context_variable(
        stage: impl Into<String>,
        message: impl Into<String>,
        name: &str,
    ) -> Self {
        Self {
            code: Some(Self::UNKNOWN_CONTEXT_VARIABLE_CODE.to_string()),
            identity: Some(WarningIdentity {
                subject: WarningSubject::Root {
                    document: None,
                    name: format!("ctx.{name}"),
                },
            }),
            ..Self::new(stage, message)
        }
    }

    /// An unknown-identifier warning for the root `name`, read in `document`
    /// (at `line` when provable). One per source document per root: every
    /// read of `name` there is one issue.
    pub(crate) fn unknown_identifier(
        stage: impl Into<String>,
        message: impl Into<String>,
        name: &str,
        document: Option<PathBuf>,
        line: Option<usize>,
    ) -> Self {
        Self {
            source: Some(Self::EXPRESSION_SOURCE.to_string()),
            code: Some(Self::UNKNOWN_IDENTIFIER_CODE.to_string()),
            identity: Some(WarningIdentity {
                subject: WarningSubject::Root {
                    document: None,
                    name: name.to_string(),
                },
            }),
            path: document,
            line_number: line,
            ..Self::new(stage, message)
        }
    }

    /// A lenient expression parse or evaluation failure at `origin` in the
    /// scanned text.
    pub(crate) fn expression_failure(
        stage: impl Into<String>,
        message: impl Into<String>,
        code: &'static str,
        origin: ExpressionOrigin,
    ) -> Self {
        Self {
            code: Some(code.to_string()),
            identity: Some(WarningIdentity {
                subject: WarningSubject::Expression {
                    document: None,
                    scope: None,
                    origin,
                },
            }),
            ..Self::new(stage, message)
        }
    }

    /// Names the scanned text an expression-failure identity indexes, when
    /// one document is scanned as several texts. Other families are unchanged.
    #[must_use]
    pub(crate) fn in_scope(mut self, scope: &str) -> Self {
        if let Some(WarningIdentity {
            subject: WarningSubject::Expression { scope: slot, .. },
        }) = self.identity.as_mut()
            && slot.is_none()
        {
            *slot = Some(scope.to_string());
        }
        self
    }

    /// Adds a line number to this warning.
    #[must_use]
    pub fn at_line(mut self, line: usize) -> Self {
        self.line_number = Some(line);
        self
    }

    /// The `(source, code, subject)` key, for a warning whose family
    /// declares an identity.
    fn issue_key(&self) -> Option<IssueKey> {
        let identity = self.identity.clone()?;
        Some((self.source.clone(), self.code.clone(), identity))
    }

    fn has_issue_key(&self, (source, code, identity): &IssueKey) -> bool {
        self.identity.as_ref() == Some(identity) && &self.source == source && &self.code == code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root_warning(name: &str, message: &str) -> ComposeWarning {
        ComposeWarning::unknown_context_variable("interpolation", message, name)
    }

    fn authored_failure(span: std::ops::Range<usize>, message: &str) -> ComposeWarning {
        ComposeWarning::expression_failure(
            "interpolation",
            message,
            ComposeWarning::EXPRESSION_PARSE_FAILURE_CODE,
            ExpressionOrigin::Authored(span),
        )
    }

    fn identity_work() -> usize {
        IDENTITY_WORK.with(std::cell::Cell::get)
    }

    fn missing(root: &str) -> crate::markdown::compose::expression::absence::MissingRoot {
        crate::markdown::compose::expression::absence::MissingRoot {
            root: root.to_string(),
            span: None,
        }
    }

    fn report_of(document: &str, warnings: Vec<ComposeWarning>) -> ComposeReport {
        let mut report = ComposeReport::new();
        report.add_warnings(warnings);
        report.attribute_to_document(std::path::Path::new(document));
        report
    }

    #[test]
    fn the_add_path_keeps_the_first_of_two_same_issue_warnings() {
        let mut report = ComposeReport::new();
        report.add_warning(root_warning("toady", "first").at_line(3));
        report.add_warning(root_warning("toady", "reworded").at_line(9));

        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].message, "first");
        assert_eq!(report.warnings[0].line_number, Some(3));
    }

    /// Message is prose, not identity: identical prose for two different
    /// issues stays two warnings.
    #[test]
    fn identical_messages_with_different_identities_stay_distinct() {
        let mut report = ComposeReport::new();
        report.add_warning(root_warning("toady", "same prose"));
        report.add_warning(root_warning("fooo", "same prose"));
        report.add_warning(authored_failure(0..5, "same prose"));
        report.add_warning(authored_failure(6..11, "same prose"));

        assert_eq!(report.warnings.len(), 4, "{:?}", report.warnings);
    }

    #[test]
    fn warnings_without_an_identity_are_never_collapsed() {
        let mut report = ComposeReport::new();
        report.add_warning(ComposeWarning::new("interpolation", "plain"));
        report.add_warning(ComposeWarning::new("interpolation", "plain"));

        assert_eq!(report.warnings.len(), 2);
    }

    #[test]
    fn merge_collapses_one_documents_repeats_but_not_two_documents_issues() {
        let mut root = report_of("root.md", vec![root_warning("toady", "root")]);
        let first_child = report_of("a.md", vec![root_warning("toady", "a")]);
        let first_child_again = report_of("a.md", vec![root_warning("toady", "a again")]);
        let second_child = report_of("b.md", vec![root_warning("toady", "b")]);

        root.merge(first_child);
        root.merge(first_child_again);
        root.merge(second_child);

        let messages: Vec<&str> = root.warnings.iter().map(|w| w.message.as_str()).collect();
        assert_eq!(messages, ["root", "a", "b"]);
    }

    /// Attribution never overwrites a child's document, and repeats that only
    /// become identical once attributed are collapsed then.
    #[test]
    fn attribution_keeps_child_documents_and_collapses_newly_equal_repeats() {
        let child = report_of("child.md", vec![root_warning("toady", "child")]);
        let mut parent = ComposeReport::new();
        parent.add_warning(root_warning("toady", "parent"));
        parent.merge(child);
        parent.warnings.push(root_warning("toady", "parent repeat"));

        parent.attribute_to_document(std::path::Path::new("parent.md"));

        let messages: Vec<&str> = parent.warnings.iter().map(|w| w.message.as_str()).collect();
        assert_eq!(messages, ["parent", "child"]);
    }

    /// A frontmatter document is scanned once per value, so the same offsets
    /// in two values are two expressions.
    #[test]
    fn expression_failures_in_different_scopes_do_not_alias() {
        let mut report = ComposeReport::new();
        report.add_warning(authored_failure(4..10, "a").in_scope("title"));
        report.add_warning(authored_failure(4..10, "b").in_scope("summary"));
        report.add_warning(authored_failure(4..10, "c").in_scope("title"));

        let messages: Vec<&str> = report.warnings.iter().map(|w| w.message.as_str()).collect();
        assert_eq!(messages, ["a", "b"]);
    }

    #[test]
    fn a_generated_origin_never_aliases_an_authored_one_at_the_same_offsets() {
        let mut report = ComposeReport::new();
        report.add_warning(authored_failure(0..17, "authored"));
        report.add_warning(ComposeWarning::expression_failure(
            "interpolation",
            "generated",
            ComposeWarning::EXPRESSION_PARSE_FAILURE_CODE,
            ExpressionOrigin::Generated { pass: 1, span: 0..17 },
        ));

        assert_eq!(report.warnings.len(), 2);
    }

    /// `in_scope` only scopes expression failures; a root warning's identity
    /// is per document, never per scanned text.
    #[test]
    fn scoping_leaves_root_identities_per_document() {
        let mut report = ComposeReport::new();
        report.add_warning(root_warning("toady", "title").in_scope("title"));
        report.add_warning(root_warning("toady", "body"));

        assert_eq!(report.warnings.len(), 1);
    }

    /// The schema-advisory family's key is exactly `(source, code, path)`: the
    /// consumer and message are not part of it.
    #[test]
    fn schema_advisory_identity_is_source_code_and_path() {
        let advisory = |consumer: &str, message: &str| ComposeWarning {
            message: message.to_string(),
            ..ComposeWarning {
                stage: "schema_validation".to_string(),
                message: String::new(),
                line_number: None,
                source: Some(SchemaAdvisory::SOURCE.to_string()),
                code: Some("dm.schema.missing_simplified_envelope".to_string()),
                path: Some(PathBuf::from("schema.yaml")),
                consumer: Some(PathBuf::from(consumer)),
                identity: Some(WarningIdentity {
                    subject: WarningSubject::Path(PathBuf::from("schema.yaml")),
                }),
            }
        };
        let mut report = ComposeReport::new();
        report.add_warning(advisory("root.md", "first"));
        let mut child = ComposeReport::new();
        child.add_warning(advisory("child.md", "reworded"));
        report.merge(child);

        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].message, "first");
    }

    const N: usize = 1000;

    /// Complexity regression (spec Performance Posture): each root read costs
    /// O(1) identity work, so `N` distinct roots, each read three times across
    /// stages, stay linear. The prior vector scan did ~N²/2 comparisons.
    #[test]
    fn distinct_unknown_roots_do_linear_identity_work() {
        let mut report = ComposeReport::new();
        let before = identity_work();

        for stage in ["frontmatter", "interpolation", "condition"] {
            for index in 0..N {
                report.add_unknown_root_candidates(stage, vec![missing(&format!("root_{index}"))], |_| {
                    CandidateLocus::Document
                });
            }
        }

        let work = identity_work() - before;
        assert_eq!(report.unknown_root_candidates.len(), N);
        assert!(work <= 4 * N, "{work} identity probes for {N} distinct roots read 3 times");
    }

    /// `N` distinct coded warnings added one at a time, then `N` single-warning
    /// child reports merged in, each cost O(1) identity work.
    #[test]
    fn distinct_coded_warnings_do_linear_identity_work() {
        let mut report = ComposeReport::new();
        let children: Vec<ComposeReport> = (0..N)
            .map(|index| report_of("child.md", vec![root_warning(&format!("child_{index}"), "child")]))
            .collect();
        let before = identity_work();

        for index in 0..N {
            report.add_warning(root_warning(&format!("root_{index}"), "root"));
            report.add_warning(root_warning(&format!("root_{index}"), "repeat"));
        }
        for child in children {
            report.merge(child);
        }

        let work = identity_work() - before;
        assert_eq!(report.warnings.len(), 2 * N);
        assert!(work <= 6 * N, "{work} identity probes for {} warning insertions", 3 * N);
    }

    /// Hash-backed membership keeps the first read of each root, in first-read
    /// order and at its first locus, however the reads interleave.
    #[test]
    fn unknown_root_candidates_keep_first_read_order_and_locus() {
        let mut report = ComposeReport::new();
        for pass in 0..3 {
            let roots = (0..50).rev().map(|index| missing(&format!("r{}", (index * 7) % 50))).collect();
            report.add_unknown_root_candidates("interpolation", roots, |_| CandidateLocus::Line(pass + 1));
        }
        let mut child = ComposeReport::new();
        child.add_unknown_root_candidates("condition", vec![missing("r3"), missing("late")], |_| {
            CandidateLocus::Document
        });
        report.merge(child);

        let candidates = report.unknown_root_candidates.take();
        let expected: Vec<String> = (0..50)
            .rev()
            .map(|index| format!("r{}", (index * 7) % 50))
            .chain(["late".to_string()])
            .collect();
        let roots: Vec<&str> = candidates.iter().map(|candidate| candidate.root.as_str()).collect();
        assert_eq!(roots, expected);
        assert!(candidates[..50].iter().all(|candidate| candidate.locus == CandidateLocus::Line(1)));
        assert!(candidates[..50].iter().all(|candidate| candidate.stage == "interpolation"));
    }

    /// `warnings` is public, so it can change without `add_warning`: a direct
    /// push is indexed on the next add, and a shrunken vector is reindexed.
    #[test]
    fn the_warning_index_follows_direct_changes_to_warnings() {
        let mut report = ComposeReport::new();
        report.add_warning(root_warning("kept", "kept"));
        report.warnings.push(root_warning("pushed", "pushed"));
        report.add_warning(root_warning("pushed", "pushed again"));
        assert_eq!(report.warnings.len(), 2);

        report.warnings.clear();
        report.add_warning(root_warning("kept", "kept after clear"));
        assert_eq!(report.warnings.len(), 1);

        report.warnings = vec![root_warning("other", "other"), root_warning("kept", "moved")];
        report.add_warning(root_warning("kept", "moved again"));
        let messages: Vec<&str> = report.warnings.iter().map(|w| w.message.as_str()).collect();
        assert_eq!(messages, ["other", "moved"]);
    }
}
