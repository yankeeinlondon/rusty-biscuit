//! The gitignored per-worktree state area and the run record.
//!
//! Layout under `messenger/.research-state/` (architecture record, "Local
//! state area"): `runs/<platform>/<run_id>/` holds `run.json`, the prepared
//! `inputs/`, pass `outputs/`, the `candidate/`, `source-checks.json`, and
//! the check results; `runs/fleet.lock` is the runner's whole-execution
//! lock; `runs/prepare.lock` serializes preparation; `renewals/` holds
//! routine unchanged-renewal records. Nothing here is committed, and every
//! record is written by atomic replacement with LF line endings.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::config::RunLimits;
use super::input::{DecisionReason, Maintainer};
use crate::research::canonical::xxh64_digest;
use crate::research::model::{Date, PlatformId};
use crate::research::paths::Workspace;
use crate::research::publish::RetryPolicy;
use crate::research::publish::fsutil::replace_file;

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
    /// are protected from cleanup. `exhausted` survives only as a readable,
    /// resumable resting state of ledger-era records (the budget ledger is
    /// removed); it is never promotable and never shown as successful.
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
    pub by: Option<Maintainer>,
    pub on: Date,
    pub reason: Option<DecisionReason>,
}

crate::research::model::common::string_enum! {
    /// What the Messenger-owned runner observed when the attempt's sequence
    /// command settled. `finished` leaves the status `check-run` decided;
    /// `interrupted` and `stopped` are recorded with a stop reason.
    pub enum RunnerOutcome {
        Finished => "finished",
        Interrupted => "interrupted",
        Stopped => "stopped",
    }
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
    /// The ledger-era budget `runs` count this attempt was prepared under.
    /// The budget ledger is removed; the field stays deserializable so
    /// ledger-era records load, and is never written.
    #[serde(skip_serializing, default)]
    pub ledger_runs: u64,
    /// Agent admissions across all attempts. Ledger-era records omit this
    /// field and begin at zero; malformed present values are rejected.
    #[serde(default)]
    pub invocations: u64,
    /// Whether a runner started this attempt's sequence. A launched `active`
    /// record with no live runner is stale and must be recovered explicitly.
    #[serde(default)]
    pub launched: bool,
    /// What the runner observed when this attempt's sequence settled;
    /// `None` until a runner runs it.
    #[serde(default)]
    pub runner: Option<RunnerOutcome>,
    pub stages: Vec<StageResult>,
    pub stop_reason: Option<String>,
    pub decision: Option<Decision>,
}

/// The maximum resumptions per run.
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
pub(crate) const PREPARE_LOCK: &str = "prepare.lock";

/// The repository-relative fleet-lock path, for diagnostics.
pub fn fleet_lock_path() -> String {
    format!("{RUNS}/fleet.lock")
}

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

    /// The fleet lock a research runner holds for its whole execution, so one
    /// research run executes at a time. An OS lock, not the file's existence:
    /// the OS releases it when the holder dies, so the file left behind never
    /// blocks a later run.
    pub fn fleet_lock(&self) -> PathBuf {
        self.runs_dir().join("fleet.lock")
    }

    /// Whether a research runner holds the fleet lock right now. Probing
    /// creates the lock file; the file itself is inert.
    pub fn fleet_lock_held(&self) -> bool {
        crate::research::publish::fsutil::PublicationLock::try_acquire(&self.fleet_lock()).is_err()
    }

    /// The lock `prepare` (and `prepare --resume`) holds from selection until
    /// its runs exist, so two preparations cannot both find a platform
    /// without an open run.
    pub fn prepare_lock(&self) -> PathBuf {
        self.runs_dir().join(PREPARE_LOCK)
    }

    pub fn run_dir(&self, platform: PlatformId, run_id: &RunId) -> PathBuf {
        self.runs_dir().join(platform.as_str()).join(run_id.as_str())
    }

    /// The repository-relative spelling of a run file.
    pub fn run_path(&self, platform: PlatformId, run_id: &RunId, file: &str) -> String {
        format!("{RUNS}/{platform}/{run_id}/{file}")
    }

    /// Whether a ledger-era `budget.json` sits beside the record. The ledger
    /// is removed; the artifact is neither read nor deleted, and callers only
    /// mention it.
    pub fn has_legacy_ledger(&self, record: &RunRecord) -> bool {
        self.run_dir(record.platform_id, &record.run_id).join("budget.json").exists()
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

    /// Every run directory with the platform whose directory holds it and its
    /// repository-relative path, sorted by platform then run ID. Unreadable
    /// records are returned as errors, never skipped; so is a record whose
    /// platform or run ID disagrees with the directory holding it, because
    /// callers key the platform on the directory, not on the record.
    pub fn list(&self) -> Vec<(PlatformId, String, Result<RunRecord, StateError>)> {
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
                let display = format!("{relative}/run.json");
                let path = self.runs_dir().join(platform.as_str()).join(&name).join("run.json");
                let record = read_run(&path, &display).and_then(|record| {
                    if record.platform_id == *platform && record.run_id.as_str() == name {
                        Ok(record)
                    } else {
                        Err(StateError::Corrupt {
                            path: display,
                            message: format!("it records run {} of {}, not its directory", record.run_id, record.platform_id),
                        })
                    }
                });
                out.push((*platform, relative, record));
            }
        }
        out
    }
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
    fn a_ledger_era_record_with_runs_and_ledger_still_loads() {
        // A record the ledger era wrote: it carries `ledger_runs` and no
        // `launched`/`runner` fields. It must load unchanged, and a fresh
        // save never re-writes the removed field.
        let record: RunRecord = serde_json::from_value(serde_json::json!({
            "format": RUN_FORMAT, "run_id": "2026-09-17-0123abcd", "platform_id": "discord",
            "created": "2026-09-17", "status": "exhausted", "selected_because": ["forced"],
            "limits": {"max_seconds": 60, "max_invocations": 4}, "baseline": null,
            "researched_under": {"fleet_prompt": "a", "schema": "b"}, "recovery_attempts": 0,
            "ledger_runs": 3, "stages": [], "stop_reason": "execution budget exhausted", "decision": null
        }))
        .unwrap();
        assert_eq!(record.status, RunStatus::Exhausted);
        assert_eq!(record.ledger_runs, 3);
        assert!(!record.launched, "a ledger-era record was never marked launched by the new runner");
        assert!(record.runner.is_none());
        assert!(!serde_json::to_value(&record).unwrap().to_string().contains("ledger_runs"), "the removed field is never written");
    }
}
