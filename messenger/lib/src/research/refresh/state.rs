//! The gitignored per-worktree state area and the run record.
//!
//! Layout under `messenger/.research-state/` (architecture record, "Local
//! state area"): `runs/<platform>/<run_id>/` holds `run.json`, Claudine's
//! `budget.json` ledger, the prepared `inputs/`, pass `outputs/`, the
//! `candidate/`, `source-checks.json`, and the check results; `renewals/`
//! holds routine unchanged-renewal records. Nothing here is committed, and
//! every record is written by atomic replacement with LF line endings.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::config::RunLimits;
use crate::research::canonical::xxh64_digest;
use crate::research::model::{Date, PlatformId};
use crate::research::paths::Workspace;
use crate::research::publish::RetryPolicy;
use crate::research::publish::fsutil::{read_optional, replace_file};

/// The run-record format tag.
pub const RUN_FORMAT: &str = "messenger-research-run/1";

/// `{UTC date}-{8 hex}`, unique per prepared run.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RunId(String);

impl RunId {
    /// A fresh ID dated `today`.
    pub fn generate(today: &Date, platform: PlatformId) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let seed = format!("{platform}:{nanos}:{}", std::process::id());
        let digest = xxh64_digest(seed.as_bytes());
        let hex = digest.trim_start_matches("xxh64:");
        Self(format!("{today}-{}", &hex[..8]))
    }

    /// Parses `YYYY-MM-DD-hhhhhhhh`.
    pub fn parse(text: &str) -> Option<Self> {
        let (date, hex) = (text.get(..10)?, text.get(10..)?);
        let hex = hex.strip_prefix('-')?;
        (Date::parse(date).is_some()
            && hex.len() == 8
            && hex.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
        .then(|| Self(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for RunId {
    type Error = String;
    fn try_from(text: String) -> Result<Self, String> {
        Self::parse(&text).ok_or_else(|| format!("invalid run ID `{text}`"))
    }
}

impl From<RunId> for String {
    fn from(id: RunId) -> String {
        id.0
    }
}

crate::research::model::common::string_enum! {
    /// Where a run rests. `active`, `awaiting_review`, and `interrupted` runs
    /// are protected from cleanup.
    pub enum RunStatus {
        Active => "active",
        AwaitingReview => "awaiting_review",
        Exhausted => "exhausted",
        Interrupted => "interrupted",
        Failed => "failed",
        Rejected => "rejected",
        Accepted => "accepted",
    }
}

impl RunStatus {
    /// Still open: a new run for the same platform is refused.
    pub fn is_open(self) -> bool {
        matches!(self, Self::Active | Self::AwaitingReview | Self::Interrupted | Self::Exhausted)
    }

    /// Decided; nothing may change the run any more.
    pub fn is_final(self) -> bool {
        matches!(self, Self::Accepted | Self::Rejected)
    }
}

crate::research::model::common::string_enum! {
    /// The run's stages, in order. The first three are agent passes; the
    /// review is the independent evidence reviewer.
    pub enum Stage {
        Discovery => "discovery",
        Reconcile => "reconcile",
        Sources => "sources",
        Validation => "validation",
        Review => "review",
    }
}

crate::research::model::common::string_enum! {
    pub enum StageStatus {
        Pending => "pending",
        Complete => "complete",
        Failed => "failed",
    }
}

/// The last judgment of one stage's outputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageResult {
    pub stage: Stage,
    pub status: StageStatus,
    /// Why the stage is pending or failed; empty when complete.
    pub findings: Vec<String>,
}

/// The fingerprints of the shared contract a document was researched under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchedUnder {
    /// xxh64 of `_fleet.md`.
    pub fleet_prompt: String,
    /// `_schema.yaml` + `_types.yaml` fingerprint.
    pub schema: String,
}

/// The accepted document a run started from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineRef {
    /// xxh64 of the whole published document.
    pub xxh64: String,
}

/// A recorded decision on a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    /// `human`, `renewal`, or `rejected`.
    pub kind: String,
    pub by: Option<String>,
    pub on: Date,
    pub reason: Option<String>,
}

/// `run.json`: identity, configuration, stage results, and outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunRecord {
    pub format: String,
    pub run_id: RunId,
    pub platform_id: PlatformId,
    pub created: Date,
    pub status: RunStatus,
    /// Why the run was selected (selection reason codes).
    pub selected_because: Vec<String>,
    pub limits: RunLimits,
    /// The published document at preparation; `None` for initial research.
    pub baseline: Option<BaselineRef>,
    pub researched_under: ResearchedUnder,
    /// Resumptions after a failed, interrupted, or exhausted attempt (max 2).
    pub recovery_attempts: u32,
    pub stages: Vec<StageResult>,
    pub stop_reason: Option<String>,
    pub decision: Option<Decision>,
}

/// The maximum resumptions per run; a resumption never adds budget.
pub const MAX_RECOVERY_ATTEMPTS: u32 = 2;

impl RunRecord {
    /// The first stage that is not complete.
    pub fn next_stage(&self) -> Option<Stage> {
        Stage::ALL.iter().copied().find(|stage| {
            self.stages
                .iter()
                .find(|result| result.stage == *stage)
                .is_none_or(|result| result.status != StageStatus::Complete)
        })
    }
}

/// A read-only view of Claudine's ledger (`budget.json`); Claudine is its
/// only writer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerView {
    pub state: String,
    #[serde(default)]
    pub stage: Option<String>,
    #[serde(default)]
    pub stop_reason: Option<String>,
    pub limits: Allowance,
    pub used: Allowance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Allowance {
    pub invocations: u64,
    pub active_ms: u64,
}

/// Local-state failures. Paths are repository-relative.
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: io::Error,
    },
    #[error("{path} is not a valid record: {message}")]
    Corrupt { path: String, message: String },
    #[error("no research run `{0}` in the local state area")]
    UnknownRun(String),
}

/// `messenger/.research-state/`.
#[derive(Debug, Clone)]
pub struct StateArea {
    workspace: Workspace,
}

pub(crate) const RUNS: &str = "messenger/.research-state/runs";
pub(crate) const RENEWALS: &str = "messenger/.research-state/renewals";

impl StateArea {
    pub fn new(workspace: &Workspace) -> Self {
        Self { workspace: workspace.clone() }
    }

    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    pub fn root(&self) -> PathBuf {
        self.workspace.state_dir()
    }

    pub fn runs_dir(&self) -> PathBuf {
        self.root().join("runs")
    }

    pub fn renewals_dir(&self) -> PathBuf {
        self.root().join("renewals")
    }

    /// The lock file every ledger shares, so one platform runs at a time.
    pub fn fleet_lock(&self) -> PathBuf {
        self.runs_dir().join("fleet.lock")
    }

    pub fn run_dir(&self, platform: PlatformId, run_id: &RunId) -> PathBuf {
        self.runs_dir().join(platform.as_str()).join(run_id.as_str())
    }

    /// The repository-relative spelling of a run file.
    pub fn run_path(&self, platform: PlatformId, run_id: &RunId, file: &str) -> String {
        format!("{RUNS}/{platform}/{run_id}/{file}")
    }

    /// Creates the run directory (refusing an existing one) and its record.
    ///
    /// ## Errors
    ///
    /// [`StateError::Io`] when the directory exists or cannot be written.
    pub fn create(&self, record: &RunRecord) -> Result<PathBuf, StateError> {
        let dir = self.run_dir(record.platform_id, &record.run_id);
        fs::create_dir_all(dir.parent().expect("platform directory")).map_err(io_err("create the runs directory"))?;
        fs::create_dir(&dir).map_err(io_err(format!("create run {}", record.run_id)))?;
        self.save(record)?;
        Ok(dir)
    }

    /// Replaces `run.json` atomically.
    ///
    /// ## Errors
    ///
    /// [`StateError::Io`].
    pub fn save(&self, record: &RunRecord) -> Result<(), StateError> {
        let path = self.run_dir(record.platform_id, &record.run_id).join("run.json");
        write_json(&path, record, record.run_id.as_str())
    }

    /// Loads a run by ID from any platform directory.
    ///
    /// ## Errors
    ///
    /// [`StateError::UnknownRun`], or [`StateError::Corrupt`] for an
    /// unreadable record.
    pub fn load(&self, run_id: &str) -> Result<RunRecord, StateError> {
        let id = RunId::parse(run_id).ok_or_else(|| StateError::UnknownRun(run_id.to_string()))?;
        for platform in PlatformId::ALL {
            let path = self.run_dir(*platform, &id).join("run.json");
            if path.exists() {
                return read_run(&path, &format!("{RUNS}/{platform}/{id}/run.json"));
            }
        }
        Err(StateError::UnknownRun(run_id.to_string()))
    }

    /// Every run directory, sorted by platform then run ID; unreadable
    /// records are returned as errors, never skipped.
    pub fn list(&self) -> Vec<(String, Result<RunRecord, StateError>)> {
        let mut out = Vec::new();
        for platform in PlatformId::ALL {
            let Ok(entries) = fs::read_dir(self.runs_dir().join(platform.as_str())) else { continue };
            let mut names: Vec<String> = entries
                .flatten()
                .filter(|entry| entry.path().is_dir())
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            for name in names {
                let relative = format!("{RUNS}/{platform}/{name}");
                let path = self.runs_dir().join(platform.as_str()).join(&name).join("run.json");
                out.push((relative.clone(), read_run(&path, &format!("{relative}/run.json"))));
            }
        }
        out
    }

    /// Claudine's ledger for a run, when one was initialized.
    ///
    /// ## Errors
    ///
    /// [`StateError::Corrupt`] for a ledger that does not parse.
    pub fn ledger(&self, record: &RunRecord) -> Result<Option<LedgerView>, StateError> {
        let path = self.run_dir(record.platform_id, &record.run_id).join("budget.json");
        let display = self.run_path(record.platform_id, &record.run_id, "budget.json");
        match read_optional(&path).map_err(io_err(format!("read {display}")))? {
            None => Ok(None),
            Some(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|error| StateError::Corrupt { path: display, message: error.to_string() }),
        }
    }
}

/// Applies the ledger's resting state to an active run: an exhausted ledger
/// makes the run `exhausted` (never finished, never an investigated unknown);
/// a crash-recovered ledger makes it `interrupted`. Returns whether the
/// record changed.
pub fn apply_ledger(record: &mut RunRecord, ledger: Option<&LedgerView>) -> bool {
    let Some(ledger) = ledger else { return false };
    if record.status != RunStatus::Active {
        return false;
    }
    let stage = ledger.stage.as_deref().unwrap_or("unknown");
    let (status, reason) = match ledger.state.as_str() {
        "exhausted" => (RunStatus::Exhausted, format!("execution budget exhausted before or during stage {stage}")),
        "interrupted" => (RunStatus::Interrupted, format!("the runner stopped unexpectedly during stage {stage}")),
        _ => return false,
    };
    record.status = status;
    record.stop_reason = Some(ledger.stop_reason.clone().unwrap_or(reason));
    true
}

fn read_run(path: &Path, display: &str) -> Result<RunRecord, StateError> {
    let bytes = fs::read(path).map_err(io_err(format!("read {display}")))?;
    let record: RunRecord = serde_json::from_slice(&bytes)
        .map_err(|error| StateError::Corrupt { path: display.to_string(), message: error.to_string() })?;
    if record.format != RUN_FORMAT {
        return Err(StateError::Corrupt { path: display.to_string(), message: format!("unknown format {}", record.format) });
    }
    Ok(record)
}

/// Pretty JSON with a trailing LF, replaced atomically.
pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T, tag: &str) -> Result<(), StateError> {
    let mut bytes = serde_json::to_vec_pretty(value).expect("state records always serialize");
    bytes.push(b'\n');
    replace_file(path, &bytes, tag, RetryPolicy::default()).map_err(io_err(format!("write {}", path.display())))
}

pub(crate) fn io_err(context: impl Into<String>) -> impl FnOnce(io::Error) -> StateError {
    let context = context.into();
    move |source| StateError::Io { context, source }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_ids_are_dated_hex_and_parse_strictly() {
        let today = Date::parse("2026-09-17").unwrap();
        let id = RunId::generate(&today, PlatformId::Discord);
        assert!(id.as_str().starts_with("2026-09-17-"), "{id}");
        assert_eq!(RunId::parse(id.as_str()), Some(id));
        for bad in ["2026-09-17", "2026-09-17-1234567", "2026-09-17-1234567G", "2026-13-17-12345678", "../x-12345678", "2026-09-17_12345678"] {
            assert!(RunId::parse(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn only_an_active_run_takes_the_ledger_resting_state() {
        let ledger = |state: &str| LedgerView {
            state: state.to_string(),
            stage: Some("reconcile".to_string()),
            stop_reason: None,
            limits: Allowance { invocations: 4, active_ms: 1000 },
            used: Allowance { invocations: 4, active_ms: 900 },
        };
        let mut record: RunRecord = serde_json::from_value(serde_json::json!({
            "format": RUN_FORMAT, "run_id": "2026-09-17-0123abcd", "platform_id": "discord",
            "created": "2026-09-17", "status": "active", "selected_because": ["forced"],
            "limits": {"max_seconds": 60, "max_invocations": 4}, "baseline": null,
            "researched_under": {"fleet_prompt": "a", "schema": "b"}, "recovery_attempts": 0,
            "stages": [], "stop_reason": null, "decision": null
        }))
        .unwrap();
        assert!(!apply_ledger(&mut record, Some(&ledger("suspended"))));
        assert!(apply_ledger(&mut record, Some(&ledger("exhausted"))));
        assert_eq!(record.status, RunStatus::Exhausted);
        assert!(record.stop_reason.as_deref().unwrap().contains("reconcile"));
        // A decided or already-exhausted run never changes again.
        assert!(!apply_ledger(&mut record, Some(&ledger("interrupted"))));
        assert_eq!(record.next_stage(), Some(Stage::Discovery));
    }
}
