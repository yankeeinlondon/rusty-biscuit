//! Refresh selection: which roster platforms are due, and why.
//!
//! A platform is refreshed when it has no accepted document, its research
//! expired, the operator forced it, its accepted document no longer
//! validates, the shared prompt or schema changed since it was researched,
//! no review or renewal record says what it was researched under, or an
//! observed provider/SDK/bridge version differs from the accepted findings.
//! Otherwise it is skipped with an auditable reason. A platform with an open
//! run is never selected again until that run is decided or resumed, and a
//! platform with an unreadable run record is never selected until the record
//! is repaired or removed.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::RefreshError;
use super::review::{RENEWAL_FORMAT, RenewalRecord, ReviewRecord, is_review_path};
use super::state::{ResearchedUnder, RunId, RunStatus, StateArea};
use crate::research::canonical::{schema_fingerprint, text_fingerprint};
use crate::research::load::Loader;
use crate::research::model::{Date, PlatformDocument, PlatformId, Roster};
use crate::research::paths::{Workspace, document_path};
use crate::research::publish::{PublishError, Verified, read_verified};
use crate::research::validate::{Context, Scope, validate_document};

/// Why a platform is due.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum Reason {
    /// No accepted document is published.
    Missing,
    Expired { refresh_due: Date },
    Forced,
    /// The accepted document fails current validation.
    SchemaInvalid { findings: usize },
    /// `_fleet.md` changed since the document was researched.
    PromptChanged,
    /// `_schema.yaml`/`_types.yaml` changed since the document was researched.
    SchemaChanged,
    /// Neither a review nor a renewal record says what it was researched under.
    NoReviewRecord,
    /// An observed version is not among the accepted findings for the interface.
    VersionChanged { interface: String, observed: String },
}

impl Reason {
    /// The stable code stored in a run record.
    pub fn code(&self) -> &'static str {
        match self {
            Reason::Missing => "missing",
            Reason::Expired { .. } => "expired",
            Reason::Forced => "forced",
            Reason::SchemaInvalid { .. } => "schema_invalid",
            Reason::PromptChanged => "prompt_changed",
            Reason::SchemaChanged => "schema_changed",
            Reason::NoReviewRecord => "no_review_record",
            Reason::VersionChanged { .. } => "version_changed",
        }
    }
}

/// Why a platform is not refreshed now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum Skip {
    /// Accepted, valid, unexpired, and researched under the current contract.
    Current { last_updated: Date, refresh_due: Date },
    /// A run is still open; resume, promote, or reject it first.
    OpenRun { run_id: RunId, status: RunStatus },
    /// A run directory's record cannot be read, so whether the platform has
    /// an open run is unknown. Repair or remove the run directory at `path`
    /// (repository-relative) before the platform is selected again.
    UnreadableRun { path: String, error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Selection {
    pub platform_id: PlatformId,
    /// Empty when skipped.
    pub due: Vec<Reason>,
    pub skip: Option<Skip>,
}

impl Selection {
    pub fn is_due(&self) -> bool {
        self.skip.is_none()
    }
}

/// Operator inputs to selection.
#[derive(Debug, Clone, Default)]
pub struct Request {
    pub forced: BTreeSet<PlatformId>,
    /// Force every active platform.
    pub force_all: bool,
    /// Observed versions keyed by interface ID.
    pub observed_versions: BTreeMap<String, String>,
}

/// The shared contract as it is now.
///
/// ## Errors
///
/// [`RefreshError::Research`] when a contract file cannot be read.
pub fn current_contract(workspace: &Workspace) -> Result<ResearchedUnder, RefreshError> {
    let read = |path: std::path::PathBuf| {
        std::fs::read_to_string(&path).map_err(|source| crate::research::ResearchError::Io {
            path: workspace.repo_path(&path).unwrap_or_else(|_| crate::research::RepoPath::from_portable("?")),
            source,
        })
    };
    Ok(ResearchedUnder {
        fleet_prompt: text_fingerprint(&read(workspace.fleet_prompt())?),
        schema: schema_fingerprint(&read(workspace.document_schema())?, &read(workspace.types_schema())?),
    })
}

/// The published snapshot, if one exists.
///
/// ## Errors
///
/// A verification or I/O error; a missing snapshot is `None`.
pub fn published(workspace: &Workspace) -> Result<Option<Verified>, PublishError> {
    match read_verified(workspace) {
        Ok(verified) => Ok(Some(verified)),
        Err(PublishError::NoSnapshot) => Ok(None),
        Err(error) => Err(error),
    }
}

/// What each platform's accepted document was last researched under: the
/// newest review record (published) or renewal record (local).
pub fn researched_under(state: &StateArea, verified: Option<&Verified>) -> BTreeMap<PlatformId, (Date, ResearchedUnder)> {
    let mut latest: BTreeMap<PlatformId, (Date, String, ResearchedUnder)> = BTreeMap::new();
    let mut offer = |platform: PlatformId, on: Date, key: String, under: ResearchedUnder| {
        let newer = latest.get(&platform).is_none_or(|(date, k, _)| (&on, &key) > (date, k));
        if newer {
            latest.insert(platform, (on, key, under));
        }
    };
    if let Some(verified) = verified {
        for (path, bytes) in &verified.files {
            if is_review_path(path)
                && let Ok(record) = ReviewRecord::parse(bytes)
            {
                offer(record.platform_id, record.approval.on.clone(), record.run_id.to_string(), record.researched_under);
            }
        }
    }
    for record in renewals(state) {
        offer(record.platform_id, record.renewed_on.clone(), record.run_id.to_string(), record.researched_under);
    }
    latest.into_iter().map(|(platform, (on, _, under))| (platform, (on, under))).collect()
}

/// Readable local renewal records; unreadable ones are ignored here and
/// reported by cleanup.
pub fn renewals(state: &StateArea) -> Vec<RenewalRecord> {
    let Ok(entries) = std::fs::read_dir(state.renewals_dir()) else { return Vec::new() };
    let mut out: Vec<RenewalRecord> = entries
        .flatten()
        .filter_map(|entry| std::fs::read(entry.path()).ok())
        .filter_map(|bytes| serde_json::from_slice::<RenewalRecord>(&bytes).ok())
        .filter(|record| record.format == RENEWAL_FORMAT)
        .collect();
    out.sort_by(|a, b| (&a.renewed_on, &a.run_id).cmp(&(&b.renewed_on, &b.run_id)));
    out
}

/// What keeps each platform from getting another open run, with the
/// repository-relative run directory responsible: an open run, or an
/// unreadable run record (which might be the open run, so it blocks too and
/// takes precedence). `except` leaves one run out, for resuming it.
pub fn blocking_runs(state: &StateArea, except: Option<&RunId>) -> BTreeMap<PlatformId, (String, Skip)> {
    let mut blocked: BTreeMap<PlatformId, (String, Skip)> = BTreeMap::new();
    for (platform, path, record) in state.list() {
        let mut record = match record {
            Ok(record) if except == Some(&record.run_id) => continue,
            Ok(record) => record,
            Err(error) => {
                let error = error.to_string();
                blocked.insert(platform, (path.clone(), Skip::UnreadableRun { path, error }));
                continue;
            }
        };
        // Judge the run's resting state, not its last saved status. An
        // unreadable ledger leaves the saved status, and an `active` run
        // stays open.
        let ledger = state.ledger(&record).ok().flatten();
        super::state::apply_ledger(&mut record, ledger.as_ref());
        if record.status.is_open() && !matches!(blocked.get(&platform), Some((_, Skip::UnreadableRun { .. }))) {
            blocked.insert(platform, (path, Skip::OpenRun { run_id: record.run_id, status: record.status }));
        }
    }
    blocked
}

/// Decides every active roster platform in roster order.
///
/// ## Errors
///
/// [`RefreshError::InvalidRoster`], a snapshot verification error, or a read
/// error. Nothing is written.
pub fn select(loader: &Loader, today: &Date, request: &Request) -> Result<Vec<Selection>, RefreshError> {
    let workspace = loader.workspace();
    let state = StateArea::new(workspace);
    let roster_loaded = loader.load_roster(&workspace.roster())?;
    let roster: &Roster = roster_loaded.record.as_ref().ok_or(RefreshError::InvalidRoster)?;
    let verified = published(workspace)?;
    let contract = current_contract(workspace)?;
    let under = researched_under(&state, verified.as_ref());
    let mut blocked = blocking_runs(&state, None);
    for platform in &request.forced {
        if roster.active_platforms().all(|p| p.platform_id != *platform) {
            return Err(RefreshError::NotInRoster { platform: platform.to_string() });
        }
    }

    let mut selections = Vec::new();
    for platform in roster.active_platforms() {
        let id = platform.platform_id;
        if let Some((_, skip)) = blocked.remove(&id) {
            selections.push(Selection { platform_id: id, due: Vec::new(), skip: Some(skip) });
            continue;
        }
        let mut due = Vec::new();
        if request.force_all || request.forced.contains(&id) {
            due.push(Reason::Forced);
        }
        let text = verified
            .as_ref()
            .and_then(|verified| verified.files.get(&document_path(id)))
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned());
        let Some(text) = text else {
            due.push(Reason::Missing);
            selections.push(Selection { platform_id: id, due, skip: None });
            continue;
        };
        let loaded = loader.load_document_text(&workspace.document(id), text)?;
        let validation = validate_document(&loaded, &Context { roster: Some(roster), scope: Scope::Accepted });
        if !validation.diagnostics.is_empty() {
            due.push(Reason::SchemaInvalid { findings: validation.diagnostics.len() });
        }
        let interval = platform.refresh_interval_days.unwrap_or(roster.refresh_interval_days);
        let document: Option<&PlatformDocument> = loaded.record.as_ref();
        let mut current = None;
        if let Some(document) = document {
            // An unrepresentable refresh date is an SR-ROSTER finding, so the
            // platform is already due as `SchemaInvalid` and never `Current`.
            if let Some(refresh_due) = document.last_updated.checked_plus_days(interval) {
                if today >= &refresh_due {
                    due.push(Reason::Expired { refresh_due: refresh_due.clone() });
                }
                current = Some((document.last_updated.clone(), refresh_due));
            }
            for (interface, observed) in &request.observed_versions {
                let belongs = platform.interface(interface).is_some();
                let known = document
                    .api_versions
                    .iter()
                    .any(|finding| &finding.interface == interface && finding.latest_stable.as_deref() == Some(observed.as_str()));
                if belongs && !known {
                    due.push(Reason::VersionChanged { interface: interface.clone(), observed: observed.clone() });
                }
            }
        }
        match under.get(&id) {
            None => due.push(Reason::NoReviewRecord),
            Some((_, researched)) => {
                if researched.fleet_prompt != contract.fleet_prompt {
                    due.push(Reason::PromptChanged);
                }
                if researched.schema != contract.schema {
                    due.push(Reason::SchemaChanged);
                }
            }
        }
        let skip = match (due.is_empty(), current) {
            (true, Some((last_updated, refresh_due))) => Some(Skip::Current { last_updated, refresh_due }),
            _ => None,
        };
        selections.push(Selection { platform_id: id, due, skip });
    }
    Ok(selections)
}
