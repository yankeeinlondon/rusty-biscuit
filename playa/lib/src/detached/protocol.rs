use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{PlaybackOptions, PlaybackReport};

pub(crate) const SCHEMA_VERSION: u16 = 1;
pub(crate) const CAPABILITY_VERSION: u16 = 1;
pub(crate) const DELEGATED_PLAY_VERSION: u16 = 1;
pub(crate) const PREPARATION_VERSION: u16 = 1;
pub(crate) const JOURNAL_VERSION: u16 = 1;
pub(crate) const PREPARATION_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// Durable identity assigned to one ordered spool slot.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(String);

impl JobId {
    /// Identity returned when detached playback is intentionally disabled.
    pub fn dry_run() -> Self {
        Self("dry-run".to_string())
    }

    /// Return the stable textual identity.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn allocated(sequence: u64, entropy: u64) -> Self {
        Self(format!("{sequence:020}-{entropy:016x}"))
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Lossless native OS-string representation used by the persisted protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsValue {
    /// Raw Unix path or argument bytes.
    UnixBytes(Vec<u8>),
    /// UTF-16 code units used by Windows paths and arguments.
    WindowsWide(Vec<u16>),
}

impl OsValue {
    /// Decode the value on the platform whose native encoding it represents.
    pub fn to_os_string(&self) -> Result<OsString, crate::PlaybackError> {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt as _;
            match self {
                Self::UnixBytes(bytes) => Ok(OsString::from_vec(bytes.clone())),
                Self::WindowsWide(_) => Err(protocol_error(
                    "Windows-wide OS string cannot execute on a Unix worker",
                )),
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt as _;
            match self {
                Self::WindowsWide(units) => Ok(OsString::from_wide(units)),
                Self::UnixBytes(_) => Err(protocol_error(
                    "Unix-byte OS string cannot execute on a Windows worker",
                )),
            }
        }
    }

    pub(crate) fn to_path_buf(&self) -> Result<PathBuf, crate::PlaybackError> {
        self.to_os_string().map(PathBuf::from)
    }
}

impl From<OsString> for OsValue {
    fn from(value: OsString) -> Self {
        Self::from(value.as_os_str())
    }
}

impl From<&OsStr> for OsValue {
    fn from(value: &OsStr) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt as _;
            Self::UnixBytes(value.as_bytes().to_vec())
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt as _;
            Self::WindowsWide(value.encode_wide().collect())
        }
    }
}

/// Whether the delegate may use native playback or must remain host-only.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackRouting {
    /// Use Playa's native-first route.
    #[default]
    #[serde(rename = "automatic", alias = "auto")]
    Auto,
    /// Bypass native playback.
    ForceHost,
}

/// Serializable projection of audio-ducking configuration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DetachedDucking {
    /// Fade duration in milliseconds.
    pub ramp_ms: u32,
    /// Target system-audio volume while playback runs.
    pub floor_scalar: f32,
}

/// Complete playback policy carried to the exact enqueuer executable.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DetachedPlayback {
    /// Speed, volume, and output-channel options.
    pub options: PlaybackOptions,
    /// Native-first or explicitly host-only routing.
    pub routing: PlaybackRouting,
    /// Optional system-audio ducking configuration.
    pub ducking: Option<DetachedDucking>,
}

#[derive(Serialize, Deserialize)]
struct DetachedPlaybackWire {
    #[serde(default)]
    routing: PlaybackRouting,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    speed: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    volume: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    channel: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ducking: Option<DetachedDucking>,
}

impl Serialize for DetachedPlayback {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        DetachedPlaybackWire {
            routing: self.routing,
            speed: self.options.speed,
            volume: self.options.volume,
            channel: self.options.channel.clone(),
            ducking: self.ducking,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DetachedPlayback {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = DetachedPlaybackWire::deserialize(deserializer)?;
        Ok(Self {
            options: PlaybackOptions {
                speed: wire.speed,
                volume: wire.volume,
                channel: wire.channel,
            },
            routing: wire.routing,
            ducking: wire.ducking,
        })
    }
}

/// A ready-to-run detached audio operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SpoolJob {
    /// Play an absolute local audio file.
    PlayFile {
        /// Losslessly encoded path.
        path: OsValue,
        /// Complete playback policy.
        #[serde(default)]
        playback: DetachedPlayback,
        /// Remove the source after playback only when it belongs to the spool.
        #[serde(default)]
        delete_after: bool,
    },
    /// Run a direct speech command without shell interpretation.
    Command {
        /// Absolute executable path.
        program: OsValue,
        /// Losslessly encoded argument vector.
        #[serde(default)]
        args: Vec<OsValue>,
    },
}

/// Private payload retained while a producer prepares a reserved slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreparingPayload {
    /// Preparation protocol spoken by the producer helper.
    pub preparation_version: u16,
    /// Absolute deadline in Unix milliseconds.
    pub deadline: chrono::DateTime<chrono::Utc>,
    /// Producer-owned private data; never projected into the journal.
    #[serde(flatten)]
    pub private_data: serde_json::Map<String, serde_json::Value>,
}

impl PreparingPayload {
    /// Construct a v1 preparation payload with the required ten-minute deadline.
    pub fn new(private_data: serde_json::Value) -> Self {
        let private_data = match private_data {
            serde_json::Value::Object(map) => map,
            value => serde_json::Map::from_iter([("data".to_string(), value)]),
        };
        Self {
            preparation_version: PREPARATION_VERSION,
            deadline: chrono::Utc::now()
                + chrono::Duration::from_std(PREPARATION_TIMEOUT)
                    .unwrap_or_else(|_| chrono::Duration::minutes(10)),
            private_data,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum JobState {
    Preparing {
        #[serde(flatten)]
        preparation: PreparingPayload,
    },
    Ready {
        #[serde(flatten)]
        job: SpoolJob,
    },
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct JobEnvelope {
    pub schema_version: u16,
    pub job_id: JobId,
    pub sequence: u64,
    pub enqueued_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "enqueuer")]
    pub enqueuer_executable: OsValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enqueuer_fingerprint: Option<u64>,
    pub capability_version: u16,
    #[serde(rename = "payload")]
    pub state: JobState,
}

/// Redacted category retained by the bounded journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalSourceKind {
    /// Local audio file; its path is not retained.
    #[serde(alias = "speech_file")]
    File,
    /// Direct executable; its path and arguments are not retained.
    Command,
    /// Private producer preparation record.
    Preparation,
    /// Record whose full payload was not trusted.
    Unknown,
}

/// Durable state transition represented by one journal record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalTransition {
    /// A delegated operation finished and produced an outcome.
    Completed,
    /// A validated operation failed without being replayed.
    Failed,
    /// An unsafe or unsupported record was quarantined.
    Quarantined,
    /// A stale pending record was discarded before playback.
    Discarded,
}

/// Typed, redacted terminal outcome for a detached job.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JournalOutcome {
    /// Playa playback report from the delegated executable.
    Playback { report: PlaybackReport },
    /// Exit result from a direct command.
    CommandExit { code: Option<i32>, success: bool },
    /// Failure safe to display without exposing private payload data.
    Failed { reason: String },
}

impl<'de> Deserialize<'de> for JournalOutcome {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;

        let mut value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| D::Error::custom("journal outcome must be an object"))?;
        let kind = object
            .remove("kind")
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .ok_or_else(|| D::Error::custom("journal outcome is missing kind"))?;
        match kind.as_str() {
            "playback" => {
                let report = object
                    .remove("report")
                    .unwrap_or_else(|| serde_json::Value::Object(object.clone()));
                serde_json::from_value(report)
                    .map(|report| Self::Playback { report })
                    .map_err(D::Error::custom)
            }
            "command_exit" => {
                #[derive(Deserialize)]
                struct CommandExit {
                    code: Option<i32>,
                    success: bool,
                }
                serde_json::from_value::<CommandExit>(serde_json::Value::Object(object.clone()))
                    .map(|exit| Self::CommandExit {
                        code: exit.code,
                        success: exit.success,
                    })
                    .map_err(D::Error::custom)
            }
            "failed" => object
                .remove("reason")
                .and_then(|value| value.as_str().map(ToOwned::to_owned))
                .map(|reason| Self::Failed { reason })
                .ok_or_else(|| D::Error::custom("failed outcome is missing reason")),
            _ => Err(D::Error::custom("unsupported journal outcome kind")),
        }
    }
}

/// One durable, redacted journal projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalEntry {
    /// Journal schema, versioned independently of job envelopes.
    #[serde(rename = "schema_version", alias = "journal_version")]
    pub journal_version: u16,
    /// Job identity.
    pub job_id: JobId,
    /// Durable queue order.
    pub sequence: u64,
    /// Time the slot was durably allocated.
    pub enqueued_at: chrono::DateTime<chrono::Utc>,
    /// Time this terminal transition completed.
    pub finished_at: chrono::DateTime<chrono::Utc>,
    /// Redacted input category.
    pub source_kind: JournalSourceKind,
    /// Terminal state transition.
    pub transition: JournalTransition,
    /// Typed outcome without paths, arguments, or speech text.
    pub outcome: JournalOutcome,
}

/// Redacted pending-job row for status rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingJob {
    /// Job identity.
    pub job_id: JobId,
    /// Durable queue order.
    pub sequence: u64,
    /// Current state label.
    pub state: &'static str,
    /// Redacted source category.
    pub source_kind: JournalSourceKind,
}

/// Current persisted queue and journal state.
#[derive(Debug, Clone, Default)]
pub struct SpoolSnapshot {
    /// Private spool root, intended only for portable/redacted display.
    pub root: PathBuf,
    /// Pending entries in durable order.
    pub pending: Vec<PendingJob>,
    /// Journal entries in append order, including the one prior file.
    pub journal: Vec<JournalEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DelegatedRequest {
    pub schema_version: u16,
    pub delegated_play_version: u16,
    pub job_id: JobId,
    pub sequence: u64,
    pub delegate_executable: OsValue,
    pub delegate_fingerprint: u64,
    pub capability_version: u16,
    pub job: SpoolJob,
    pub report_path: OsValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DelegatedReport {
    pub schema_version: u16,
    pub job_id: JobId,
    pub sequence: u64,
    pub delegate: OsValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delegate_fingerprint: Option<u64>,
    pub outcome: JournalOutcome,
}

pub(crate) fn protocol_error(detail: impl Into<String>) -> crate::PlaybackError {
    crate::PlaybackError::DetachedProtocol {
        detail: detail.into(),
    }
}
