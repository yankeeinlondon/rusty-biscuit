//! Native discovery of Claude Code sessions from its session registry.
//!
//! Each running Claude Code process writes `<config>/sessions/<pid>.json`
//! (`<config>` is `$CLAUDE_CONFIG_DIR`, or `~/.claude`). Observed with
//! Claude Code 2.1.280–2.1.284, a record carries `pid`, `sessionId`,
//! `startedAt` (epoch milliseconds), `cwd`, `kind`, `entrypoint`, `status`
//! (`busy`, `idle`, `shell`), `version`, `name`, and the messaging socket
//! path. Discovery reads those files and nothing else: it never connects to
//! a socket, reads a credential, or asks the process anything.
//!
//! A record is only a claim. It names a live session when its process is
//! still running *and* that process started no later than the record's
//! `startedAt`: a record left behind by an exited session whose PID was
//! reused names a process that started afterwards, which is rejected. The
//! process start time comes from the caller (the CLI's `sysinfo` lookup), in
//! the same form managed registrations use, so the two identities compare.
//!
//! Only an interactive terminal session (`kind: interactive`,
//! `entrypoint: cli`) maps to a researched launch profile
//! (`ordinary-interactive`). The registry does not say whether an SDK or
//! print-mode session (`entrypoint: sdk-*`) is one-shot or retained, so those
//! are listed with no profile rather than guessed.
//!
//! Research: `docs/research/steering/claude.md` (discovery
//! `registry-<os>-native-*`).

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::provider_id::Provider;
use crate::steering::discovery::{DiscoveryFuture, NativeDiscoverer, NativeObservation};
use crate::steering::identity::ProcessStartIdentity;
use crate::steering::vocabulary::{DiscoveryMethod, ExecutionState, HostOs, LaunchMode, LaunchOrigin};

#[cfg(test)]
mod tests;

/// Research launch profile of an ordinary interactive Claude Code session.
pub const INTERACTIVE_PROFILE: &str = "ordinary-interactive";

/// Slack for `startedAt` (milliseconds) against a process start time that
/// has whole-second resolution.
const START_TOLERANCE_SECS: u64 = 2;

/// The load-bearing fields of one registry record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryRecord {
    pub pid: u32,
    pub session_id: String,
    /// Epoch milliseconds.
    pub started_at_ms: u64,
    pub kind: String,
    pub entrypoint: String,
    pub status: Option<String>,
    pub cwd: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
}

/// Why a registry record could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RecordError {
    #[error("the record is not a JSON object")]
    NotAnObject,
    #[error("`{0}` is missing")]
    Missing(&'static str),
    #[error("`{0}` is null")]
    Null(&'static str),
    #[error("`{field}` is not {expected}")]
    WrongType { field: &'static str, expected: &'static str },
}

/// Reads one registry file's text. `pid`, `sessionId`, `startedAt`, `kind`,
/// and `entrypoint` are required; `status`, `cwd`, `name`, and `version` may
/// be absent, but a present one must be a string.
pub fn read_record(text: &str) -> Result<RegistryRecord, RecordError> {
    let Ok(Value::Object(record)) = serde_json::from_str::<Value>(text) else {
        return Err(RecordError::NotAnObject);
    };
    let pid = field(&record, "pid")?
        .as_u64()
        .and_then(|pid| u32::try_from(pid).ok())
        .filter(|pid| *pid != 0)
        .ok_or(RecordError::WrongType { field: "pid", expected: "a process id" })?;
    let session_id = string(&record, "sessionId")?;
    if session_id.is_empty() {
        return Err(RecordError::WrongType { field: "sessionId", expected: "a non-empty string" });
    }
    Ok(RegistryRecord {
        pid,
        session_id,
        started_at_ms: field(&record, "startedAt")?
            .as_u64()
            .ok_or(RecordError::WrongType { field: "startedAt", expected: "epoch milliseconds" })?,
        kind: string(&record, "kind")?,
        entrypoint: string(&record, "entrypoint")?,
        status: optional_string(&record, "status")?,
        cwd: optional_string(&record, "cwd")?,
        name: optional_string(&record, "name")?,
        version: optional_string(&record, "version")?,
    })
}

/// What discovery reports for `record`, given its process's start time
/// (epoch seconds; `None` when no such process is running). `None` when the
/// record does not name a live session.
pub fn observation(record: &RegistryRecord, process_start_secs: Option<u64>) -> Option<NativeObservation> {
    let started = process_start_secs?;
    if started > record.started_at_ms / 1000 + START_TOLERANCE_SECS {
        return None;
    }
    let interactive = record.kind == "interactive" && record.entrypoint == "cli";
    let state = match record.status.as_deref() {
        Some("busy") => ExecutionState::Working,
        Some("idle") => ExecutionState::Idle,
        _ => ExecutionState::Unknown,
    };
    Some(NativeObservation {
        process: ProcessStartIdentity::new(record.pid, started.to_string()).ok()?,
        conversation: record.session_id.clone(),
        name: record.name.clone().filter(|name| !name.is_empty()),
        cwd: record.cwd.clone(),
        state,
        launch_profile: interactive.then(|| INTERACTIVE_PROFILE.to_string()),
        // An SDK or print-mode session is driven by a program, not a person.
        launch_mode: if record.kind == "interactive" && !record.entrypoint.starts_with("sdk") {
            LaunchMode::Interactive
        } else {
            LaunchMode::NonInteractive
        },
        provider_version: record.version.clone(),
    })
}

/// The registry directory: `$CLAUDE_CONFIG_DIR/sessions` when that variable
/// names an absolute directory, else `<home>/.claude/sessions`.
pub fn registry_dir(config_dir: Option<&str>, home: Option<&Path>) -> Option<PathBuf> {
    match config_dir.map(Path::new).filter(|dir| dir.is_absolute()) {
        Some(dir) => Some(dir.join("sessions")),
        None => home.map(|home| home.join(".claude").join("sessions")),
    }
}

/// Reads every `<pid>.json` record in `dir` and keeps the live sessions.
/// Other files are not records. A record that cannot be read is skipped with
/// a trace: it names no session this build can identify, and one damaged
/// file must not hide the others. A missing directory means no sessions.
pub fn scan(dir: &Path, process_start: impl Fn(u32) -> Option<u64>) -> std::io::Result<Vec<NativeObservation>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut found = Vec::new();
    for entry in entries {
        let path = entry?.path();
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else { continue };
        let named_for_pid = path.extension().is_some_and(|ext| ext == "json") && !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_digit());
        if !named_for_pid {
            continue;
        }
        let record = match std::fs::read_to_string(&path).map_err(RecordReadError::Io).and_then(|text| read_record(&text).map_err(RecordReadError::Record)) {
            Ok(record) => record,
            Err(error) => {
                tracing::debug!(target: "claudine::steering", error = &error as &dyn std::error::Error, "unreadable Claude session record");
                continue;
            }
        };
        if record.pid.to_string() != stem {
            tracing::debug!(target: "claudine::steering", "Claude session record names another process than its file");
            continue;
        }
        if let Some(observation) = observation(&record, process_start(record.pid)) {
            found.push(observation);
        }
    }
    Ok(found)
}

#[derive(Debug, thiserror::Error)]
enum RecordReadError {
    #[error("the record could not be read")]
    Io(#[source] std::io::Error),
    #[error("the record is malformed")]
    Record(#[source] RecordError),
}

/// The [`NativeDiscoverer`] over a registry directory, for one host OS.
pub struct ClaudeRegistryDiscoverer {
    dir: PathBuf,
    discovery_id: &'static str,
    process_start: fn(u32) -> Option<u64>,
}

impl ClaudeRegistryDiscoverer {
    /// The discoverer for the researched registry method on `os`, when the
    /// research defines one.
    pub fn for_host(dir: PathBuf, os: HostOs, process_start: fn(u32) -> Option<u64>) -> Option<Self> {
        let discovery_id = crate::steering::facts(Provider::Claude)
            .discovery
            .iter()
            .find(|record| {
                record.os == os
                    && record.origin == LaunchOrigin::Native
                    && record.method == DiscoveryMethod::ProviderRegistry
                    && record.profile_id == INTERACTIVE_PROFILE
            })?
            .id;
        Some(Self { dir, discovery_id, process_start })
    }
}

impl NativeDiscoverer for ClaudeRegistryDiscoverer {
    fn provider(&self) -> Provider {
        Provider::Claude
    }

    fn discovery_id(&self) -> &'static str {
        self.discovery_id
    }

    fn discover(&self) -> DiscoveryFuture<Vec<NativeObservation>> {
        let (dir, process_start) = (self.dir.clone(), self.process_start);
        Box::pin(async move {
            tokio::task::spawn_blocking(move || scan(&dir, process_start))
                .await
                .map_err(|join| Box::new(join) as crate::steering::discovery::SourceError)?
                .map_err(|io| Box::new(io) as crate::steering::discovery::SourceError)
        })
    }
}

fn field<'a>(record: &'a Map<String, Value>, name: &'static str) -> Result<&'a Value, RecordError> {
    match record.get(name) {
        None => Err(RecordError::Missing(name)),
        Some(Value::Null) => Err(RecordError::Null(name)),
        Some(value) => Ok(value),
    }
}

fn string(record: &Map<String, Value>, name: &'static str) -> Result<String, RecordError> {
    field(record, name)?.as_str().map(str::to_string).ok_or(RecordError::WrongType { field: name, expected: "a string" })
}

fn optional_string(record: &Map<String, Value>, name: &'static str) -> Result<Option<String>, RecordError> {
    match record.get(name) {
        None => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(Value::Null) => Err(RecordError::Null(name)),
        Some(_) => Err(RecordError::WrongType { field: name, expected: "a string" }),
    }
}
