//! Deterministic generation of the accepted research snapshot.
//!
//! [`load_fleet`] reads the roster, one document per active roster platform,
//! and the optional overrides and mappings; [`Fleet::validate`] applies every
//! schema and semantic rule in `Accepted` scope. Only a clean fleet becomes a
//! [`Snapshot`]: the accepted documents (byte for byte), the catalog, and the
//! summary with its generated region regenerated and its authored prose kept.
//!
//! [`generate`] publishes that snapshot through [`publish`](super::publish);
//! [`check`] rebuilds it read-only from the verified snapshot and reports
//! drift. Both refuse to act while a publication awaits recovery, and a
//! refused or interrupted generation leaves the previous snapshot selected.
//!
//! Accepted change history travels with the snapshot: every published review
//! record under `docs/research/reviews/` is carried forward byte for byte, new
//! ones arrive through [`generate_with`] (promotion), and `CHANGELOG.md` is
//! rendered from them, so history is selected atomically with the documents.
//!
//! Partial refresh: `updates` supplies new accepted document text for some
//! platforms; every other platform carries its previously published document
//! unchanged (so its own freshness dates remain), provided it still satisfies
//! the current schema and coverage. Without a published snapshot (initial
//! publication) the documents at the fixed paths are the baseline.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::Serialize;

use super::assess::{self, AssessmentOutcome};
use super::canonical::{schema_fingerprint, text_fingerprint};
use super::diagnostics::{Diagnostic, sort_diagnostics};
use super::error::ResearchError;
use super::load::{Loaded, Loader, SUPPORTED_SCHEMA_VERSION};
use super::model::{Date, Mappings, Overrides, PlatformId, Roster};
use super::paths::{CATALOG, FLEET_PROMPT, MANIFEST, RepoPath, SUMMARY, Workspace, document_path};
use super::project::{AcceptedDocument, Catalog, CatalogInputs, project};
use super::publish::{
    self, Artifact, ArtifactScope, InputEntry, Options, PublishError, PublishReport, REGION_BEGIN, REGION_END,
    SchemaEntry, Snapshot, Verified, covered_bytes, read_verified, splice_regions,
};
use super::refresh::review::{CHANGELOG, REVIEWS_DIR, ReviewRecord, is_review_path, render_changelog, review_path};
use super::report::{self, CatalogView, Enforceability, Filter};
use super::validate::{Context, DocumentValidation, Scope, validate_document, validate_fleet, validate_mappings, validate_overrides, validate_roster};

/// One active platform's document as loaded for generation.
#[derive(Debug, Clone)]
pub struct FleetDocument {
    pub platform: PlatformId,
    pub text: String,
    pub validation: DocumentValidation,
}

/// Every generation input, loaded but not yet judged as a whole.
#[derive(Debug, Clone)]
pub struct Fleet {
    pub roster: Loaded<Roster>,
    pub documents: Vec<FleetDocument>,
    /// Active roster platforms with neither an update nor a baseline document.
    pub missing: Vec<PlatformId>,
    pub overrides: Option<Loaded<Overrides>>,
    pub mappings: Option<Loaded<Mappings>>,
    pub schema: SchemaEntry,
    /// Every input except the documents, fingerprinted.
    inputs: Vec<InputEntry>,
    /// The current summary file, if any; its authored prose is kept.
    summary: Option<String>,
    /// Review records by repository path: the published ones plus new ones.
    reviews: BTreeMap<String, Vec<u8>>,
    workspace: Workspace,
}

/// Where each platform's document comes from.
#[derive(Debug, Clone, Copy)]
pub enum Baseline<'a> {
    /// No published snapshot: the fixed paths are the initial baseline.
    FixedPaths,
    /// Documents come only from this verified snapshot (or `updates`).
    Published(&'a Verified),
}

/// Loads every generation input.
///
/// ## Errors
///
/// Returns [`ResearchError`] when a present file cannot be read or its schema
/// cannot be resolved. Contract problems become diagnostics on the fleet.
pub fn load_fleet(loader: &Loader, baseline: Baseline<'_>, updates: &BTreeMap<PlatformId, String>) -> Result<Fleet, ResearchError> {
    let workspace = loader.workspace().clone();
    let read = |path: &str| -> Result<Option<String>, ResearchError> {
        match std::fs::read_to_string(workspace.resolve(&RepoPath::from_portable(path))) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(ResearchError::Io { path: RepoPath::from_portable(path), source }),
        }
    };
    let repo = |path: &std::path::Path| workspace.repo_path(path).map(|path| path.to_string());

    let roster = loader.load_roster(&workspace.roster())?;
    let schema_text = read(&repo(&workspace.document_schema())?)?.unwrap_or_default();
    let types_text = read(&repo(&workspace.types_schema())?)?.unwrap_or_default();
    let schema = SchemaEntry {
        version: SUPPORTED_SCHEMA_VERSION,
        xxh64: schema_fingerprint(&schema_text, &types_text),
    };

    let mut inputs = Vec::new();
    let optional_inputs = [
        workspace.roster(),
        workspace.roster_schema(),
        workspace.document_schema(),
        workspace.types_schema(),
        workspace.overrides_schema(),
        workspace.overrides(),
        workspace.mappings_schema(),
        workspace.mappings(),
        workspace.fleet_prompt(),
    ];
    for path in optional_inputs {
        let path = repo(&path)?;
        if let Some(text) = read(&path)? {
            let (frontmatter, body) = if path == FLEET_PROMPT { markdown_hashes(&text) } else { (None, None) };
            inputs.push(InputEntry { body, frontmatter, path, xxh64: text_fingerprint(&text) });
        }
    }

    let overrides = if workspace.overrides().exists() { Some(loader.load_overrides(&workspace.overrides())?) } else { None };
    let mappings = if workspace.mappings().exists() { Some(loader.load_mappings(&workspace.mappings())?) } else { None };

    let mut documents = Vec::new();
    let mut missing = Vec::new();
    if let Some(record) = &roster.record {
        let context = Context { roster: Some(record), scope: Scope::Accepted };
        for platform in record.active_platforms() {
            let id = platform.platform_id;
            let path = document_path(id);
            let text = match (updates.get(&id), baseline) {
                (Some(update), _) => Some(update.clone()),
                (None, Baseline::FixedPaths) => read(&path)?,
                (None, Baseline::Published(verified)) => verified
                    .files
                    .get(&path)
                    .map(|bytes| String::from_utf8_lossy(bytes).into_owned()),
            };
            let Some(text) = text else {
                missing.push(id);
                continue;
            };
            let loaded = loader.load_document_text(&workspace.document(id), text.clone())?;
            let validation = validate_document(&loaded, &context);
            documents.push(FleetDocument { platform: id, text, validation });
        }
    }

    let summary = match baseline {
        Baseline::Published(verified) => verified.files.get(SUMMARY).map(|bytes| String::from_utf8_lossy(bytes).into_owned()),
        Baseline::FixedPaths => read(SUMMARY)?,
    };
    let reviews = match baseline {
        Baseline::Published(verified) => verified
            .files
            .iter()
            .filter(|(path, _)| is_review_path(path))
            .map(|(path, bytes)| (path.clone(), bytes.clone()))
            .collect(),
        Baseline::FixedPaths => read_reviews(&workspace)?,
    };

    Ok(Fleet { roster, documents, missing, overrides, mappings, schema, inputs, summary, reviews, workspace })
}

/// Review records at the fixed directory (initial publication only).
fn read_reviews(workspace: &Workspace) -> Result<BTreeMap<String, Vec<u8>>, ResearchError> {
    let dir = workspace.resolve(&RepoPath::from_portable(REVIEWS_DIR));
    let mut reviews = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(&dir) else { return Ok(reviews) };
    for entry in entries.flatten() {
        let path = format!("{REVIEWS_DIR}/{}", entry.file_name().to_string_lossy());
        if entry.path().is_file() && is_review_path(&path) {
            let bytes = std::fs::read(entry.path()).map_err(|source| ResearchError::Io { path: RepoPath::from_portable(&path), source })?;
            reviews.insert(path, bytes);
        }
    }
    Ok(reviews)
}

fn markdown_hashes(text: &str) -> (Option<String>, Option<String>) {
    match darkmatter::markdown::Markdown::try_from_content(text.to_string()) {
        Ok(markdown) => (
            Some(format!("{:016x}", markdown.hash_frontmatter(false))),
            Some(format!("{:016x}", markdown.hash_body(false))),
        ),
        Err(_) => (None, None),
    }
}

/// The judgment of a whole fleet.
#[derive(Debug, Clone)]
pub struct FleetValidation {
    /// Every finding across every input, in stable order.
    pub diagnostics: Vec<Diagnostic>,
    /// Active platforms without a document to publish.
    pub missing: Vec<PlatformId>,
    pub accepted: Vec<AcceptedDocument>,
    pub assessments: Vec<AssessmentOutcome>,
}

impl FleetValidation {
    /// Nothing prevents publication.
    pub fn is_clean(&self) -> bool {
        self.diagnostics.is_empty() && self.missing.is_empty()
    }
}

impl Fleet {
    /// Applies every rule. `today` decides override expiry only.
    pub fn validate(&self, today: &Date) -> FleetValidation {
        let mut diagnostics = validate_roster(&self.roster, Scope::Accepted);
        let mut accepted = Vec::new();
        for document in &self.documents {
            diagnostics.extend(document.validation.diagnostics.iter().cloned());
            if let Some(validated) = &document.validation.validated {
                match AcceptedDocument::new(validated.clone(), &document.text) {
                    Ok(document) => accepted.push(document),
                    Err(message) => diagnostics.push(Diagnostic {
                        path: validated.path().clone(),
                        pointer: String::new(),
                        rule: super::diagnostics::Rule::Schema,
                        subject: None,
                        message,
                    }),
                }
            }
        }
        if let Some(roster) = &self.roster.record {
            let present: Vec<PlatformId> = self.documents.iter().map(|document| document.platform).collect();
            if self.missing.is_empty() {
                diagnostics.extend(validate_fleet(&self.roster.path, roster, &present));
            }
        }
        if let Some(roster) = &self.roster.record {
            diagnostics.extend(self.review_diagnostics(roster));
        }
        let validated: Vec<&super::validate::ValidatedDocument> = accepted.iter().map(|a| &a.validated).collect();
        if let Some(overrides) = &self.overrides {
            diagnostics.extend(validate_overrides(overrides, &validated, &self.schema.xxh64, today));
        }
        let mut assessments = Vec::new();
        if let Some(mappings) = &self.mappings {
            diagnostics.extend(validate_mappings(mappings, &validated));
            if let Some(record) = &mappings.record {
                assessments = assess::evaluate(record, &validated, &self.workspace);
            }
        }
        sort_diagnostics(&mut diagnostics);
        FleetValidation { diagnostics, missing: self.missing.clone(), accepted, assessments }
    }

    /// Adds review records (a promotion's new record) to the fleet.
    pub fn add_reviews(&mut self, reviews: &BTreeMap<String, Vec<u8>>) {
        self.reviews.extend(reviews.iter().map(|(path, bytes)| (path.clone(), bytes.clone())));
    }

    /// Parsed review records; malformed ones are reported by `validate`.
    fn parsed_reviews(&self) -> BTreeMap<String, ReviewRecord> {
        self.reviews
            .iter()
            .filter_map(|(path, bytes)| ReviewRecord::parse(bytes).ok().map(|record| (path.clone(), record)))
            .collect()
    }

    fn review_diagnostics(&self, roster: &Roster) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for (path, bytes) in &self.reviews {
            let mut push = |message: String| {
                diagnostics.push(Diagnostic {
                    path: RepoPath::from_portable(path),
                    pointer: String::new(),
                    rule: super::diagnostics::Rule::Review,
                    subject: None,
                    message,
                });
            };
            match ReviewRecord::parse(bytes) {
                Err(message) => push(format!("not a review record: {message}")),
                Ok(record) => {
                    if review_path(&record.approval.on, record.platform_id, &record.run_id) != *path {
                        push("the file name does not match the record's approval date, platform, and run".to_string());
                    }
                    if roster.active_platforms().all(|platform| platform.platform_id != record.platform_id) {
                        push(format!("{} is not an active roster platform", record.platform_id));
                    }
                }
            }
        }
        diagnostics
    }

    /// The snapshot a clean fleet publishes.
    ///
    /// ## Errors
    ///
    /// [`GenerateError::Refused`] for an unclean fleet, and
    /// [`GenerateError::SummaryRegions`] when the authored summary does not
    /// hold exactly one well-formed generated region.
    pub fn snapshot(&self, validation: &FleetValidation) -> Result<(Snapshot, Catalog), GenerateError> {
        if !validation.is_clean() {
            return Err(GenerateError::Refused {
                diagnostics: validation.diagnostics.clone(),
                missing: validation.missing.clone(),
            });
        }
        let roster = self.roster.record.as_ref().expect("a clean fleet has a typed roster");
        let mut inputs = self.inputs.clone();
        for (document, accepted) in self.documents.iter().zip(&validation.accepted) {
            inputs.push(InputEntry {
                body: Some(accepted.body_hash.clone()),
                frontmatter: Some(accepted.frontmatter_hash.clone()),
                path: accepted.validated.path().to_string(),
                xxh64: text_fingerprint(&document.text),
            });
        }
        inputs.sort();
        let catalog = project(CatalogInputs {
            roster,
            documents: &validation.accepted,
            overrides: self.overrides.as_ref().and_then(|o| o.record.as_ref()),
            assessments: &validation.assessments,
            schema: &self.schema,
            inputs: &inputs,
        });
        let catalog_bytes = catalog.to_bytes();
        let view = CatalogView::parse(&catalog_bytes).expect("a projected catalog parses as a view");
        let region = summary_region(&view);
        let summary = match &self.summary {
            Some(authored) => splice_regions(authored, &[region]).ok_or(GenerateError::SummaryRegions)?,
            None => default_summary(&region),
        };

        let mut artifacts = BTreeMap::new();
        for document in &self.documents {
            artifacts.insert(
                document_path(document.platform),
                Artifact { bytes: document.text.clone().into_bytes(), scope: ArtifactScope::File },
            );
        }
        artifacts.insert(CATALOG.to_string(), Artifact { bytes: catalog_bytes, scope: ArtifactScope::File });
        artifacts.insert(SUMMARY.to_string(), Artifact { bytes: summary.into_bytes(), scope: ArtifactScope::GeneratedRegions });
        for (path, bytes) in &self.reviews {
            artifacts.insert(path.clone(), Artifact { bytes: bytes.clone(), scope: ArtifactScope::File });
        }
        artifacts.insert(
            CHANGELOG.to_string(),
            Artifact { bytes: render_changelog(&self.parsed_reviews()).into_bytes(), scope: ArtifactScope::File },
        );
        Ok((Snapshot { schema: self.schema.clone(), inputs, artifacts }, catalog))
    }
}

/// Generation failures. Every one leaves the previous snapshot selected.
#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    #[error(transparent)]
    Research(#[from] ResearchError),

    #[error(transparent)]
    Publish(#[from] PublishError),

    #[error("generation refused: {} finding(s), {} platform(s) without an accepted document", diagnostics.len(), missing.len())]
    Refused { diagnostics: Vec<Diagnostic>, missing: Vec<PlatformId> },

    #[error("{SUMMARY} must contain exactly one generated region delimited by the generated-region markers")]
    SummaryRegions,
}

/// The result of a successful generation.
#[derive(Debug, Clone, Serialize)]
pub struct Generated {
    pub snapshot_id: String,
    /// `true` when every artifact and the manifest were already current.
    pub unchanged: bool,
    pub replaced: usize,
    pub removed: usize,
    pub artifacts: Vec<String>,
}

/// Validates every input and publishes the snapshot they produce.
///
/// ## Errors
///
/// [`GenerateError::Publish`] with [`PublishError::RecoveryRequired`] while a
/// journal is pending, or with a verification error when the published
/// snapshot was edited by hand; [`GenerateError::Refused`] for invalid inputs
/// or a missing baseline. The previous snapshot stays selected.
pub fn generate(
    loader: &Loader,
    updates: &BTreeMap<PlatformId, String>,
    today: &Date,
    options: Options,
) -> Result<Generated, GenerateError> {
    generate_with(loader, updates, &BTreeMap::new(), today, options)
}

/// [`generate`] with new review records (repository path to bytes) added to
/// the published history; promotion's only way to write accepted research.
///
/// ## Errors
///
/// As [`generate`]; a malformed or misnamed review record refuses
/// generation with an `SR-REVIEW` finding.
pub fn generate_with(
    loader: &Loader,
    updates: &BTreeMap<PlatformId, String>,
    reviews: &BTreeMap<String, Vec<u8>>,
    today: &Date,
    options: Options,
) -> Result<Generated, GenerateError> {
    let workspace = loader.workspace();
    if publish::pending(workspace) {
        return Err(PublishError::RecoveryRequired.into());
    }
    let verified = match read_verified(workspace) {
        Ok(verified) => Some(verified),
        Err(PublishError::NoSnapshot) => None,
        Err(error) => return Err(error.into()),
    };
    let baseline = verified.as_ref().map_or(Baseline::FixedPaths, Baseline::Published);
    let mut fleet = load_fleet(loader, baseline, updates)?;
    fleet.add_reviews(reviews);
    let validation = fleet.validate(today);
    let (snapshot, _catalog) = fleet.snapshot(&validation)?;
    let manifest = snapshot.manifest();
    let report: PublishReport = publish::publish(workspace, &snapshot, options)?;
    Ok(Generated {
        snapshot_id: manifest.snapshot_id.clone(),
        unchanged: report.replaced == 0 && report.removed == 0 && verified.is_some_and(|v| v.manifest == manifest),
        replaced: report.replaced,
        removed: report.removed,
        artifacts: manifest.artifacts.iter().map(|entry| entry.path.clone()).collect(),
    })
}

/// Why `generate --check` failed.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Drift {
    /// The published artifacts differ from a fresh generation. `paths` lists
    /// every differing artifact (and the manifest), sorted.
    Changed { paths: Vec<String> },
    /// The inputs no longer validate, so no fresh generation exists.
    Invalid { diagnostics: Vec<Diagnostic>, missing: Vec<PlatformId> },
}

/// Rebuilds the snapshot read-only from the verified published documents and
/// the current inputs; `None` means no drift.
///
/// ## Errors
///
/// [`PublishError::NoSnapshot`], [`PublishError::RecoveryRequired`], a
/// verification error, or a read error; none writes anything.
pub fn check(loader: &Loader, today: &Date) -> Result<Option<Drift>, GenerateError> {
    let workspace = loader.workspace();
    if publish::pending(workspace) {
        return Err(PublishError::RecoveryRequired.into());
    }
    let verified = read_verified(workspace)?;
    let fleet = load_fleet(loader, Baseline::Published(&verified), &BTreeMap::new())?;
    let validation = fleet.validate(today);
    if !validation.is_clean() {
        return Ok(Some(Drift::Invalid { diagnostics: validation.diagnostics, missing: validation.missing }));
    }
    let (snapshot, _) = fleet.snapshot(&validation)?;
    let manifest = snapshot.manifest();
    let mut paths: Vec<String> = Vec::new();
    for (path, artifact) in &snapshot.artifacts {
        let published = verified.files.get(path).and_then(|bytes| covered_bytes(bytes, artifact.scope));
        if published != covered_bytes(&artifact.bytes, artifact.scope) {
            paths.push(path.clone());
        }
    }
    for entry in &verified.manifest.artifacts {
        if !snapshot.artifacts.contains_key(&entry.path) {
            paths.push(entry.path.clone());
        }
    }
    if manifest.to_bytes() != verified.manifest_bytes {
        paths.push(MANIFEST.to_string());
    }
    paths.sort();
    paths.dedup();
    Ok((!paths.is_empty()).then_some(Drift::Changed { paths }))
}

/// Reads the verified published catalog for reporting.
///
/// ## Errors
///
/// A verification error, or [`PublishError::Corrupt`] when the verified
/// catalog does not parse.
pub fn published_catalog(workspace: &Workspace) -> Result<CatalogView, PublishError> {
    let verified = read_verified(workspace)?;
    let bytes = verified
        .files
        .get(CATALOG)
        .ok_or_else(|| PublishError::Inconsistent { path: CATALOG.to_string(), reason: "not in the manifest".to_string() })?;
    CatalogView::parse(bytes).map_err(|error| PublishError::Corrupt(format!("{CATALOG}: {error}")))
}

fn default_summary(region: &str) -> String {
    format!(
        "# Provider research summary\n\n\
         Cross-provider comparison derived from accepted research. The region \
         below is regenerated by `messenger research generate`; prose outside it \
         is authored.\n\n{REGION_BEGIN}{region}{REGION_END}\n"
    )
}

/// The machine-derived tables: coverage per interface and the truncation
/// handoff per adapter. Every row names stable IDs.
fn summary_region(catalog: &CatalogView) -> String {
    // Coverage and handoffs do not depend on the date; only freshness does,
    // and the summary omits it so it never changes merely because time passed.
    let report = report::build(catalog, &Filter::default(), &Date::parse("1970-01-01").expect("date"));
    let mut out = String::from("\n\n## Coverage by interface\n\n");
    out.push_str("| Platform | Interface | Researched | Not applicable | Investigated gaps | Open gaps | Missing |\n");
    out.push_str("|---|---|---|---|---|---|---|\n");
    let mut counts: BTreeMap<(String, String), [usize; 5]> = BTreeMap::new();
    for row in &report.coverage {
        let index = match row.state {
            super::validate::CoverageState::Researched => 0,
            super::validate::CoverageState::NotApplicable => 1,
            super::validate::CoverageState::InvestigatedGap => 2,
            super::validate::CoverageState::OpenGap => 3,
            super::validate::CoverageState::Missing => 4,
        };
        counts.entry((row.platform_id.to_string(), row.interface.clone())).or_default()[index] += 1;
    }
    for ((platform, interface), cells) in &counts {
        let _ = writeln!(out, "| {platform} | `{interface}` | {} | {} | {} | {} | {} |", cells[0], cells[1], cells[2], cells[3], cells[4]);
    }
    out.push_str("\n## Truncation handoff by adapter\n\n");
    out.push_str("| Adapter | Interface | Surface | Enforceability | Constraints |\n");
    out.push_str("|---|---|---|---|---|\n");
    for handoff in &report.truncation {
        let interface = handoff.interface.as_deref().map_or("—".to_string(), |i| format!("`{i}`"));
        if handoff.surfaces.is_empty() {
            let _ = writeln!(out, "| {} | {interface} | — | unmapped | — |", handoff.adapter);
        }
        for surface in &handoff.surfaces {
            let status = match &surface.enforceability {
                Enforceability::Enforceable => "enforceable".to_string(),
                Enforceability::NotEnforceable { .. } => "not enforceable".to_string(),
                Enforceability::NoResearchedBound { gap, .. } => {
                    gap.as_deref().map_or("no researched bound".to_string(), |gap| format!("no researched bound (`{gap}`)"))
                }
            };
            let ids = surface.constraints.iter().map(|c| format!("`{}`", c.id)).collect::<Vec<_>>().join(", ");
            let ids = if ids.is_empty() { "—".to_string() } else { ids };
            let _ = writeln!(out, "| {} | {interface} | {} | {status} | {ids} |", handoff.adapter, surface.surface);
        }
    }
    out.push('\n');
    out
}
