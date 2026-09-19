//! Explicit, dry-run-first cleanup of old local records.
//!
//! Only run directories and renewal records in the state area are
//! considered, and only when at least the threshold (initially 30 days) old.
//! Active, awaiting-review, and interrupted runs are protected, as is any run
//! whose ledger lock is held or whose record cannot be read. Accepted research
//! and its evidence live under `docs/research/` and are never considered.
//! Nothing is deleted unless the caller applies a plan, and nothing touches
//! Git.

use std::fs::{self, OpenOptions};

use serde::Serialize;

use super::RefreshError;
use super::review::{RENEWAL_FORMAT, RenewalRecord};
use super::state::{RENEWALS, RunStatus, StateArea, io_err};
use crate::research::model::Date;

/// The initial retention threshold.
pub const DEFAULT_THRESHOLD_DAYS: u32 = 30;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Removal {
    /// Repository-relative path of the directory or file.
    pub path: String,
    /// `run` or `renewal`.
    pub kind: &'static str,
    pub status: Option<RunStatus>,
    pub age_days: i64,
    /// Removing it ends the ability to resume the run.
    pub loses_resumability: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Protected {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CleanupPlan {
    pub threshold_days: u32,
    pub today: Date,
    /// Repository-relative state-area location.
    pub state_dir: String,
    pub removals: Vec<Removal>,
    pub protected: Vec<Protected>,
    /// Records younger than the threshold, kept.
    pub younger: usize,
}

/// Whether Claudine's ledger lock for a run is held right now.
fn ledger_lock_held(dir: &std::path::Path) -> bool {
    let path = dir.join("budget.json.lock");
    if !path.exists() {
        return false;
    }
    match OpenOptions::new().read(true).write(true).open(&path) {
        Ok(file) => file.try_lock().is_err(),
        Err(_) => true,
    }
}

/// Computes exactly what cleanup would remove. Reads only.
pub fn plan(state: &StateArea, today: &Date, threshold_days: u32) -> CleanupPlan {
    let mut plan = CleanupPlan {
        threshold_days,
        today: today.clone(),
        state_dir: crate::research::paths::STATE_DIR.to_string(),
        removals: Vec::new(),
        protected: Vec::new(),
        younger: 0,
    };
    for (_, path, record) in state.list() {
        let record = match record {
            Ok(record) => record,
            Err(error) => {
                plan.protected.push(Protected { path, reason: format!("unreadable run record ({error}); inspect it by hand") });
                continue;
            }
        };
        let dir = state.run_dir(record.platform_id, &record.run_id);
        let protected = match record.status {
            RunStatus::Active => Some("the run is active"),
            RunStatus::AwaitingReview => Some("the candidate awaits review"),
            RunStatus::Interrupted => Some("the run awaits operator resumption"),
            _ if ledger_lock_held(&dir) => Some("a Claudine runner holds the ledger lock"),
            _ => None,
        };
        if let Some(reason) = protected {
            plan.protected.push(Protected { path, reason: reason.to_string() });
            continue;
        }
        let age_days = record.created.days_until(today);
        if age_days < i64::from(threshold_days) {
            plan.younger += 1;
            continue;
        }
        plan.removals.push(Removal {
            path,
            kind: "run",
            status: Some(record.status),
            age_days,
            loses_resumability: matches!(record.status, RunStatus::Failed | RunStatus::Exhausted),
        });
    }
    let mut renewals: Vec<(String, Option<RenewalRecord>)> = fs::read_dir(state.renewals_dir())
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().is_file())
                .map(|entry| {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    let record = fs::read(entry.path())
                        .ok()
                        .and_then(|bytes| serde_json::from_slice::<RenewalRecord>(&bytes).ok())
                        .filter(|record| record.format == RENEWAL_FORMAT);
                    (format!("{RENEWALS}/{name}"), record)
                })
                .collect()
        })
        .unwrap_or_default();
    renewals.sort_by(|a, b| a.0.cmp(&b.0));
    for (path, record) in renewals {
        let Some(record) = record else {
            plan.protected.push(Protected { path, reason: "unreadable renewal record; inspect it by hand".to_string() });
            continue;
        };
        let age_days = record.renewed_on.days_until(today);
        if age_days < i64::from(threshold_days) {
            plan.younger += 1;
            continue;
        }
        plan.removals.push(Removal { path, kind: "renewal", status: None, age_days, loses_resumability: false });
    }
    plan
}

/// Removes exactly the plan's entries. Each is re-checked against a fresh
/// plan, so an entry that became protected in between is kept.
///
/// ## Errors
///
/// [`RefreshError::State`] when a removal fails; earlier removals stay done.
pub fn apply(state: &StateArea, planned: &CleanupPlan) -> Result<Vec<String>, RefreshError> {
    let fresh = plan(state, &planned.today, planned.threshold_days);
    let mut removed = Vec::new();
    for removal in &planned.removals {
        if !fresh.removals.contains(removal) {
            continue;
        }
        let path = state.workspace().resolve(&crate::research::RepoPath::from_portable(&removal.path));
        let result = if removal.kind == "run" { fs::remove_dir_all(&path) } else { fs::remove_file(&path) };
        result.map_err(io_err(format!("remove {}", removal.path)))?;
        removed.push(removal.path.clone());
    }
    Ok(removed)
}
