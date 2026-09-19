//! Deterministic semantic validation of the research contract.
//!
//! SimplifiedSchema proves structure; the rules here are the Rust-owned
//! `SR-*` rules in `docs/research/platforms/_rules.md`. Each submodule owns
//! one rule family. Validation never mutates input and never reaches the
//! network, a process, or the clock (the caller supplies `today`).
//!
//! Type layering: an authored [`PlatformDocument`] becomes a
//! [`ValidatedDocument`] only when it has no findings, and only a validated
//! document can yield executable constraints
//! ([`ValidatedDocument::eligibility`]).

mod bindings;
mod constraints;
mod coverage;
mod errors;
mod identity;
mod interaction;
pub mod replay;

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::diagnostics::{Diagnostic, Findings, Rule};
use super::load::Loaded;
use super::model::{Date, Knowledge, Mappings, Overrides, PlatformDocument, PlatformId, Roster, SourceKind};
use super::paths::RepoPath;

pub use constraints::{
    ConstraintEligibility, ExecutableCondition, ExecutableConstraint, IneligibleReason,
};
pub use coverage::{CategoryCoverage, CoverageState, CoverageSummary, InterfaceCoverageSummary};
pub(crate) use identity::refresh_due_overflow;

/// How complete a document must be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Fixtures, pilots, and candidates under construction: every present
    /// record must be valid, but the document need not cover every roster
    /// interface, and coverage may rest on open gaps.
    Fragment,
    /// Accepted research and initial baselines: additionally every active
    /// roster interface appears, and coverage rests only on investigated
    /// gaps (spec "Initial research completion").
    Accepted,
}

/// Inputs shared by document validation.
#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    /// The roster the document is checked against (SR-ROSTER).
    pub roster: Option<&'a Roster>,
    pub scope: Scope,
}

/// A document that passed every load-stage and semantic rule.
///
/// Only [`validate_document`] constructs one.
#[derive(Debug, Clone)]
pub struct ValidatedDocument {
    path: RepoPath,
    document: PlatformDocument,
    frontmatter: Value,
}

impl ValidatedDocument {
    pub fn path(&self) -> &RepoPath {
        &self.path
    }

    pub fn document(&self) -> &PlatformDocument {
        &self.document
    }

    pub fn platform_id(&self) -> PlatformId {
        self.document.platform_id
    }

    /// Every constraint with its executable projection or the reasons it
    /// has none, in document order. Ineligible facts stay listed.
    pub fn eligibility(&self) -> Vec<ConstraintEligibility> {
        constraints::eligibility(&self.document)
    }

    /// Per-interface, per-category completeness.
    pub fn coverage(&self) -> CoverageSummary {
        coverage::summary(&self.document)
    }

    /// The authored JSON of the record with `id` in any category array.
    pub fn record(&self, id: &str) -> Option<&Value> {
        record_json(&self.frontmatter, id)
    }

    /// The authored frontmatter (without `$schema`).
    pub fn frontmatter(&self) -> &Value {
        &self.frontmatter
    }

    /// Every fact-bearing record with its authored JSON, in document order.
    pub fn fact_records(&self) -> Vec<FactRecord<'_>> {
        fact_ids(&self.document)
            .into_iter()
            .filter_map(|(pointer, id)| {
                let record = self.frontmatter.pointer(&pointer)?;
                let array = pointer
                    .split('/')
                    .filter(|segment| !segment.is_empty() && !segment.bytes().all(|b| b.is_ascii_digit()))
                    .collect::<Vec<_>>()
                    .join(".");
                Some(FactRecord { array, id, record })
            })
            .collect()
    }
}

/// One fact-bearing record of a validated document.
#[derive(Debug, Clone)]
pub struct FactRecord<'a> {
    /// The frontmatter array holding it, e.g. `constraints`, or
    /// `format_profiles.fixtures` for a nested format fixture.
    pub array: String,
    pub id: &'a str,
    pub record: &'a Value,
}

/// Per-interface, per-category completeness of an authored document, valid
/// or not. An interface without a matrix reports every category as
/// [`CoverageState::Missing`], never as unrestricted.
pub fn coverage_summary(document: &PlatformDocument) -> CoverageSummary {
    coverage::summary(document)
}

/// The outcome of validating one document.
#[derive(Debug, Clone)]
pub struct DocumentValidation {
    pub path: RepoPath,
    /// Load-stage and semantic findings, in stable order.
    pub diagnostics: Vec<Diagnostic>,
    pub validated: Option<ValidatedDocument>,
}

/// Validates a loaded platform document.
///
/// Semantic rules run only when the document loaded into the typed model;
/// load-stage findings are carried through either way.
pub fn validate_document(loaded: &Loaded<PlatformDocument>, context: &Context<'_>) -> DocumentValidation {
    let mut findings = Findings::new(loaded.path.clone());
    findings.extend(loaded.diagnostics.iter().cloned());
    if let Some(document) = &loaded.record {
        let index = Index::new(document);
        identity::check(document, &index, context, &mut findings);
        constraints::check(document, &mut findings);
        bindings::check(document, &index, &mut findings);
        interaction::check(document, &index, &mut findings);
        errors::check(document, &index, &mut findings);
        coverage::check(document, &index, context, &mut findings);
    }
    let diagnostics = findings.finish();
    let validated = match (&loaded.record, diagnostics.is_empty()) {
        (Some(document), true) => Some(ValidatedDocument {
            path: loaded.path.clone(),
            document: document.clone(),
            frontmatter: loaded.frontmatter.clone(),
        }),
        _ => None,
    };
    DocumentValidation {
        path: loaded.path.clone(),
        diagnostics,
        validated,
    }
}

/// Validates a roster (SR-ROSTER, SR-CURATED). `Accepted` scope also
/// requires the full five-platform, seven-adapter fleet.
pub fn validate_roster(loaded: &Loaded<Roster>, scope: Scope) -> Vec<Diagnostic> {
    let mut findings = Findings::new(loaded.path.clone());
    findings.extend(loaded.diagnostics.iter().cloned());
    if let Some(roster) = &loaded.record {
        identity::check_roster(roster, scope, &mut findings);
    }
    findings.finish()
}

/// Validates the fleet as a whole: exactly one accepted document per active
/// roster platform (SR-ROSTER). `documents` are the platforms present.
pub fn validate_fleet(roster_path: &RepoPath, roster: &Roster, documents: &[PlatformId]) -> Vec<Diagnostic> {
    let mut findings = Findings::new(roster_path.clone());
    identity::check_fleet(roster, documents, &mut findings);
    findings.finish()
}

/// Validates overrides against the documents they target and the current
/// schema fingerprint (SR-OVERRIDE). `today` decides expiry.
pub fn validate_overrides(
    loaded: &Loaded<Overrides>,
    documents: &[&ValidatedDocument],
    schema_fingerprint: &str,
    today: &Date,
) -> Vec<Diagnostic> {
    let mut findings = Findings::new(loaded.path.clone());
    findings.extend(loaded.diagnostics.iter().cloned());
    if let Some(overrides) = &loaded.record {
        coverage::check_overrides(overrides, documents, schema_fingerprint, today, &mut findings);
    }
    findings.finish()
}

/// Validates implementation mappings against the documents they cite
/// (SR-MAPPING). Fingerprint reuse is decided separately by
/// [`crate::research::assess`].
pub fn validate_mappings(loaded: &Loaded<Mappings>, documents: &[&ValidatedDocument]) -> Vec<Diagnostic> {
    let mut findings = Findings::new(loaded.path.clone());
    findings.extend(loaded.diagnostics.iter().cloned());
    if let Some(mappings) = &loaded.record {
        super::assess::check_mappings(mappings, documents, &mut findings);
    }
    findings.finish()
}

/// Finds the authored JSON of a record by ID across a document's arrays.
pub(crate) fn record_json<'a>(frontmatter: &'a Value, id: &str) -> Option<&'a Value> {
    frontmatter.as_object()?.values().find_map(|value| {
        value
            .as_array()?
            .iter()
            .find(|record| record.get("id").and_then(Value::as_str) == Some(id))
    })
}

/// Record lookups shared by the rule families.
pub(crate) struct Index<'a> {
    pub sources: BTreeMap<&'a str, &'a super::model::Source>,
    pub interfaces: BTreeMap<&'a str, &'a super::model::Interface>,
    pub constraints: BTreeMap<&'a str, &'a super::model::Constraint>,
    pub profiles: BTreeMap<&'a str, &'a super::model::FormatProfile>,
    pub text_bindings: BTreeMap<&'a str, &'a super::model::TextBinding>,
    pub image_bindings: BTreeMap<&'a str, &'a super::model::ImageBinding>,
    pub envelopes: BTreeMap<&'a str, &'a super::model::Envelope>,
    pub errors: BTreeMap<&'a str, &'a super::model::ErrorRecord>,
    pub error_fixtures: BTreeMap<&'a str, &'a super::model::ErrorFixture>,
    pub question_bindings: BTreeMap<&'a str, &'a super::model::QuestionBinding>,
    pub form_bindings: BTreeMap<&'a str, &'a super::model::FormBinding>,
    pub gaps: BTreeMap<&'a str, &'a super::model::Gap>,
    /// Every fact-bearing record ID (anything a gap or change may name).
    pub facts: BTreeSet<&'a str>,
}

impl<'a> Index<'a> {
    pub fn new(document: &'a PlatformDocument) -> Self {
        fn by_id<'a, T>(items: &'a [T], id: impl Fn(&'a T) -> &'a str) -> BTreeMap<&'a str, &'a T> {
            items.iter().map(|item| (id(item), item)).collect()
        }
        let d = document;
        let facts = fact_ids(d).into_iter().map(|(_, id)| id).collect();
        Self {
            sources: by_id(&d.sources, |s| s.id.as_str()),
            interfaces: by_id(&d.interfaces, |i| i.interface_id.as_str()),
            constraints: by_id(&d.constraints, |c| c.id.as_str()),
            profiles: by_id(&d.format_profiles, |p| p.id.as_str()),
            text_bindings: by_id(&d.text_bindings, |b| b.id.as_str()),
            image_bindings: by_id(&d.image_bindings, |b| b.id.as_str()),
            envelopes: by_id(&d.envelopes, |e| e.id.as_str()),
            errors: by_id(&d.errors, |e| e.id.as_str()),
            error_fixtures: by_id(&d.error_fixtures, |f| f.id.as_str()),
            question_bindings: by_id(&d.question_bindings, |b| b.id.as_str()),
            form_bindings: by_id(&d.form_bindings, |b| b.id.as_str()),
            gaps: by_id(&d.gaps, |g| g.id.as_str()),
            facts,
        }
    }

    /// Whether every evidence ID is a source whose kind is `secondary`.
    pub fn secondary_only(&self, evidence: &[String]) -> bool {
        !evidence.is_empty()
            && evidence.iter().all(|id| {
                self.sources
                    .get(id.as_str())
                    .is_some_and(|source| source.kind == SourceKind::Secondary)
            })
    }
}

/// Every fact-bearing record: `(array pointer, id)` in document order.
/// Sources, interfaces, gaps, and changes are identities, not facts.
pub(crate) fn fact_ids(d: &PlatformDocument) -> Vec<(String, &str)> {
    let mut ids: Vec<(String, &str)> = Vec::new();
    macro_rules! collect {
        ($field:ident) => {
            for (index, record) in d.$field.iter().enumerate() {
                ids.push((format!("/{}/{index}", stringify!($field)), record.id.as_str()));
            }
        };
    }
    collect!(api_versions);
    collect!(chronology);
    collect!(constraints);
    collect!(format_profiles);
    collect!(text_bindings);
    collect!(image_bindings);
    collect!(attachment_bindings);
    collect!(addressing);
    collect!(receipts);
    collect!(attribution_bindings);
    collect!(location_bindings);
    collect!(author_geolocation);
    collect!(expression_bindings);
    collect!(delivery_controls);
    collect!(eligibility);
    collect!(rate_limits);
    collect!(envelopes);
    collect!(errors);
    collect!(error_fixtures);
    collect!(inbound_bindings);
    collect!(question_bindings);
    collect!(form_bindings);
    collect!(interaction_fixtures);
    for (index, profile) in d.format_profiles.iter().enumerate() {
        for (fixture_index, fixture) in profile.fixtures.iter().enumerate() {
            ids.push((
                format!("/format_profiles/{index}/fixtures/{fixture_index}"),
                fixture.id.as_str(),
            ));
        }
    }
    ids
}

/// Every record carrying a knowledge block: `(pointer, id, knowledge)`.
pub(crate) fn knowledge_blocks(d: &PlatformDocument) -> Vec<(String, &str, &Knowledge)> {
    let mut blocks: Vec<(String, &str, &Knowledge)> = Vec::new();
    macro_rules! collect {
        ($field:ident) => {
            for (index, record) in d.$field.iter().enumerate() {
                blocks.push((
                    format!("/{}/{index}/knowledge", stringify!($field)),
                    record.id.as_str(),
                    &record.knowledge,
                ));
            }
        };
    }
    collect!(api_versions);
    collect!(chronology);
    collect!(constraints);
    collect!(format_profiles);
    collect!(text_bindings);
    collect!(image_bindings);
    collect!(attachment_bindings);
    collect!(addressing);
    collect!(receipts);
    collect!(attribution_bindings);
    collect!(location_bindings);
    collect!(author_geolocation);
    collect!(expression_bindings);
    collect!(delivery_controls);
    collect!(eligibility);
    collect!(rate_limits);
    collect!(envelopes);
    collect!(errors);
    collect!(inbound_bindings);
    collect!(question_bindings);
    collect!(form_bindings);
    blocks
}

/// Every record carrying `applies_when`: `(pointer, id, conditions)`.
pub(crate) fn condition_lists(d: &PlatformDocument) -> Vec<(String, &str, &[super::model::Condition])> {
    let mut lists: Vec<(String, &str, &[super::model::Condition])> = Vec::new();
    macro_rules! collect {
        ($field:ident) => {
            for (index, record) in d.$field.iter().enumerate() {
                lists.push((
                    format!("/{}/{index}/applies_when", stringify!($field)),
                    record.id.as_str(),
                    &record.applies_when,
                ));
            }
        };
    }
    for (index, interface) in d.interfaces.iter().enumerate() {
        lists.push((
            format!("/interfaces/{index}/applies_when"),
            interface.interface_id.as_str(),
            &interface.applies_when,
        ));
    }
    collect!(constraints);
    collect!(image_bindings);
    collect!(attachment_bindings);
    collect!(addressing);
    collect!(attribution_bindings);
    collect!(location_bindings);
    collect!(expression_bindings);
    collect!(delivery_controls);
    collect!(eligibility);
    collect!(errors);
    collect!(inbound_bindings);
    collect!(question_bindings);
    collect!(form_bindings);
    lists
}

/// Every record scoped to an interface: `(pointer, id, interface)`.
pub(crate) fn interface_scoped(d: &PlatformDocument) -> Vec<(String, &str, &str)> {
    let mut scoped: Vec<(String, &str, &str)> = Vec::new();
    macro_rules! collect {
        ($field:ident) => {
            for (index, record) in d.$field.iter().enumerate() {
                scoped.push((
                    format!("/{}/{index}/interface", stringify!($field)),
                    record.id.as_str(),
                    record.interface.as_str(),
                ));
            }
        };
    }
    collect!(api_versions);
    collect!(chronology);
    collect!(constraints);
    collect!(text_bindings);
    collect!(image_bindings);
    collect!(attachment_bindings);
    collect!(addressing);
    collect!(receipts);
    collect!(attribution_bindings);
    collect!(location_bindings);
    collect!(author_geolocation);
    collect!(expression_bindings);
    collect!(delivery_controls);
    collect!(eligibility);
    collect!(rate_limits);
    collect!(envelopes);
    collect!(errors);
    collect!(inbound_bindings);
    collect!(question_bindings);
    collect!(form_bindings);
    scoped
}

/// Records of one category for one interface, used by SR-COVERAGE.
pub(crate) fn category_record_count(d: &PlatformDocument, interface: &str, category: super::model::Category) -> usize {
    use super::model::Category as C;
    fn count<T>(items: &[T], interface: &str, of: impl Fn(&T) -> &str) -> usize {
        items.iter().filter(|item| of(item) == interface).count()
    }
    match category {
        C::Versions => count(&d.api_versions, interface, |r| &r.interface) + count(&d.chronology, interface, |r| &r.interface),
        C::Constraints => count(&d.constraints, interface, |r| &r.interface),
        // Profiles are document-wide: text bindings attach them to surfaces.
        C::Formatting => d.format_profiles.len(),
        C::TextBindings => count(&d.text_bindings, interface, |r| &r.interface),
        C::Images => count(&d.image_bindings, interface, |r| &r.interface) + count(&d.role_coverage, interface, |r| &r.interface),
        C::Attachments => count(&d.attachment_bindings, interface, |r| &r.interface),
        C::Addressing => count(&d.addressing, interface, |r| &r.interface),
        C::Receipts => count(&d.receipts, interface, |r| &r.interface),
        C::Attribution => count(&d.attribution_bindings, interface, |r| &r.interface),
        C::Location => count(&d.location_bindings, interface, |r| &r.interface) + count(&d.author_geolocation, interface, |r| &r.interface),
        C::Expression => count(&d.expression_bindings, interface, |r| &r.interface),
        // Interactivity spans an interface and its companions (Gateway,
        // Events API): related records count in both directions.
        C::Interactivity => d
            .interfaces
            .iter()
            .filter(|candidate| {
                let related = |from: &super::model::Interface, to: &str| {
                    from.relationships.iter().any(|relationship| relationship.target == to)
                };
                candidate.interface_id == interface
                    || related(candidate, interface)
                    || d.interfaces
                        .iter()
                        .any(|this| this.interface_id == interface && related(this, &candidate.interface_id))
            })
            .map(|scope| {
                let scope = scope.interface_id.as_str();
                count(&d.inbound_bindings, scope, |r| &r.interface)
                    + count(&d.question_bindings, scope, |r| &r.interface)
                    + count(&d.form_bindings, scope, |r| &r.interface)
            })
            .sum::<usize>()
            + d.question_bindings.iter().filter(|q| q.companion_interface.as_deref() == Some(interface)).count()
            + d.form_bindings.iter().filter(|f| f.companion_interface.as_deref() == Some(interface)).count(),
        C::DeliveryControls => count(&d.delivery_controls, interface, |r| &r.interface),
        C::Eligibility => count(&d.eligibility, interface, |r| &r.interface),
        C::RateLimits => count(&d.rate_limits, interface, |r| &r.interface),
        C::Errors => count(&d.envelopes, interface, |r| &r.interface) + count(&d.errors, interface, |r| &r.interface),
    }
}

pub(crate) fn push(findings: &mut Findings, rule: Rule, pointer: impl Into<String>, subject: &str, message: impl Into<String>) {
    findings.push(rule, pointer, Some(subject), message);
}
