use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::diagnostics::DiagnosticSnapshot;
use crate::events::AgenticEvent;

use crate::provider::Provider;
/// Common provider/repository filters shared by reporting queries.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportingFilters {
    /// Limit results to one provider.
    pub provider: Option<Provider>,
    /// Limit results to one repository name, or `org/name`.
    pub repo: Option<String>,
    /// Limit results to one monorepo package area.
    pub package_area: Option<String>,
    /// Limit results to one monorepo package.
    pub package: Option<String>,
}

/// Inclusive date range used by reporting queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateRange {
    /// Inclusive start date.
    pub from: NaiveDate,
    /// Inclusive end date.
    pub to: NaiveDate,
}

impl DateRange {
    /// Create a single-day range.
    pub fn single(date: NaiveDate) -> Self {
        Self {
            from: date,
            to: date,
        }
    }
}

/// Scope of a sync run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncRequest {
    /// Sync all known JSONL files.
    #[default]
    All,
    /// Sync one local calendar date.
    Date(NaiveDate),
    /// Sync an inclusive date range.
    Range(DateRange),
}

/// One per-file sync failure.
///
/// A persisted report record, so the failure crosses this boundary as a
/// [`DiagnosticSnapshot`] rather than a Rust error value (spec §D9). `message`
/// stays beside it because serialization may *add* a human line — it may not
/// replace the facets or the structured detail.
///
/// Not `Eq`: the snapshot's `detail` is an opaque `serde_json::Value`, which is
/// only `PartialEq`. Nothing keys or sets on a `SyncFailure`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncFailure {
    /// JSONL file path that failed.
    pub source_file: String,
    /// One-based line number that failed to parse, or `0` for a whole-file
    /// failure.
    pub line_number: usize,
    /// Human-readable failure message.
    pub message: String,
    /// The machine projection of the same failure.
    ///
    /// `None` only for a record persisted before this field existed. `Option`
    /// is what makes an older record readable — serde treats a missing
    /// `Option` field as `None` — and it is also the honest representation:
    /// such a record has no snapshot, rather than a synthesized one.
    #[serde(default)]
    pub diagnostic: Option<DiagnosticSnapshot>,
}

/// Result of a sync run.
///
/// Not `Eq` for the same reason [`SyncFailure`] is not.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SyncSummary {
    pub files_scanned: u64,
    pub files_rebuilt: u64,
    pub events_inserted: u64,
    pub events_skipped: u64,
    pub parse_failures: u64,
    pub anonymous_session_fallbacks: u64,
    pub failures: Vec<SyncFailure>,
}

/// Count bucket for simple label/value lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabeledCount {
    pub label: String,
    pub count: u64,
}

/// Provider split for a result set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderSplit {
    pub provider: Provider,
    pub count: u64,
    /// Number of turn_complete events for this provider.
    pub turns: u64,
    /// Number of tool_error + turn_error events for this provider.
    pub error_count: u64,
}

/// Aggregated token and cost usage.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UsageTotals {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub total_cost_usd: f64,
}

/// High-level derived metrics for reports.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DerivedMetrics {
    pub autonomy_ratio: Option<f64>,
    pub research_vs_action_ratio: Option<f64>,
    pub error_recovery_rate: Option<f64>,
    pub delegation_ratio: Option<f64>,
    pub session_efficiency: Option<f64>,
    pub context_pressure_index: Option<f64>,
}

/// Coarse tool classification for reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolActionClass {
    Research,
    Action,
    Delegation,
    Other,
}

/// Aggregated tool statistics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyToolStat {
    pub tool_name: String,
    pub call_count: u64,
    pub error_count: u64,
    pub classification: ToolActionClass,
}

/// Summary for one local day.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DailySummary {
    pub date: NaiveDate,
    pub total_events: u64,
    pub session_count: u64,
    pub total_turns: u64,
    pub total_tool_calls: u64,
    pub total_tool_errors: u64,
    pub total_turn_errors: u64,
    pub total_subagents: u64,
    pub total_compactions: u64,
    pub total_permission_requests: u64,
    pub total_human_in_loop: u64,
    pub provider_count: u64,
    pub repo_count: u64,
    pub providers: Vec<ProviderSplit>,
    pub top_tools: Vec<DailyToolStat>,
    pub permission_modes: Vec<LabeledCount>,
    pub models: Vec<LabeledCount>,
    pub usage: UsageTotals,
    pub metrics: DerivedMetrics,
}

/// Session listing row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_key: String,
    pub session_id: Option<String>,
    pub provider: Provider,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub duration_seconds: i64,
    pub cwd: Option<String>,
    pub repo_name: Option<String>,
    pub repo_org: Option<String>,
    pub branch: Option<String>,
    pub package_area: Option<String>,
    pub package: Option<String>,
    pub model: Option<String>,
    pub permission_mode: Option<String>,
    pub hostname: Option<String>,
    pub primary_language: Option<String>,
    pub event_count: u64,
    pub turn_count: u64,
    pub tool_call_count: u64,
    pub tool_error_count: u64,
    pub turn_error_count: u64,
    pub subagent_count: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub total_cost_usd: f64,
    /// Claudine's own process ID, captured once at wrapper startup.
    ///
    /// Report and query output: a stable nullable field. `null` means no
    /// Claudine PID was recorded for this session's rows (e.g. pre-feature
    /// history). The key is always emitted, never skipped.
    #[serde(default)]
    pub claudine_pid: Option<u32>,
    /// Immediate child PID returned by the wrapper spawn operation.
    ///
    /// Report and query output: a stable nullable field. `null` means no
    /// provider child PID was available for this session's rows. The key is
    /// always emitted, never skipped.
    #[serde(default)]
    pub agent_pid: Option<u32>,
}

/// Session list wrapper for JSON output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionsReport {
    pub range: DateRange,
    pub sessions: Vec<SessionInfo>,
    pub metrics: DerivedMetrics,
}

/// Error/event record with full payloads for machine-oriented output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorRecord {
    pub timestamp: DateTime<Utc>,
    pub provider: Provider,
    pub event: AgenticEvent,
    pub session_key: String,
    pub session_id: Option<String>,
    pub repo_name: Option<String>,
    pub tool_name: Option<String>,
    pub model: Option<String>,
    pub error: String,
    pub prompt: Option<String>,
    pub tool_input: Option<JsonValue>,
    pub notification_message: Option<String>,
    /// Raw extra JSON from the event, used as fallback context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<JsonValue>,
    /// Claudine's own process ID for the row's session, or `null`.
    ///
    /// Stable nullable query output: always emitted.
    #[serde(default)]
    pub claudine_pid: Option<u32>,
    /// Spawned provider child PID for the row's session, or `null`.
    ///
    /// Stable nullable query output: always emitted.
    #[serde(default)]
    pub agent_pid: Option<u32>,
}

/// Error list wrapper for JSON output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorsReport {
    pub range: DateRange,
    pub errors: Vec<ErrorRecord>,
}

/// Per-repository activity summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoActivity {
    pub repo_name: String,
    pub repo_org: Option<String>,
    pub event_count: u64,
    pub session_count: u64,
    pub head_sha_count: u64,
    pub dirty_transitions: u64,
    pub branches: Vec<LabeledCount>,
}

/// Repository list wrapper for JSON output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReposReport {
    pub range: DateRange,
    pub repos: Vec<RepoActivity>,
}

/// Tool report wrapper for JSON output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolsReport {
    pub range: DateRange,
    pub tools: Vec<DailyToolStat>,
    pub metrics: DerivedMetrics,
}

/// One trend point for a single day.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrendPoint {
    pub date: NaiveDate,
    pub events: u64,
    pub wrapped: u64,
    pub unwrapped: u64,
    pub non_interactive: u64,
    pub yolo_percent: f64,
    pub turns: u64,
    pub tool_calls: u64,
    pub tool_errors: u64,
    pub turn_errors: u64,
    pub total_tokens: u64,
    pub cost_usd: f64,
    pub repos: Vec<String>,
    pub providers: Vec<ProviderSplit>,
}

/// Trend report wrapper for JSON output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrendsReport {
    pub range: DateRange,
    pub points: Vec<TrendPoint>,
    pub provider_split: Vec<ProviderSplit>,
    pub top_tools: Vec<DailyToolStat>,
}

/// Per-provider aggregate of one model-catalog signal kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriftSignalSummary {
    pub provider: Provider,
    /// Signal kind slug (`model_catalog_drift`, `model_resolved`,
    /// `model_fallback`).
    pub kind: String,
    /// Distinct sessions that observed the signal in the range.
    pub session_count: u64,
    /// Total folded emissions across those sessions.
    pub occurrences: u64,
    pub last_seen: DateTime<Utc>,
    /// From the most recent `model_catalog_drift` payload: ids the
    /// provider reported that the baseline lacks. Empty for other kinds.
    pub unexpected: Vec<String>,
    /// From the most recent drift payload: baseline ids the provider's
    /// listing no longer returns. Empty for other kinds.
    pub missing: Vec<String>,
    /// `listing` or `resolved_model` for drift rows; `None` otherwise.
    pub observed_via: Option<String>,
}

/// One session's `family_latest` alias stamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AliasResolution {
    pub session_key: String,
    pub session_id: Option<String>,
    pub provider: Provider,
    /// Session end time (the stamp is written on the SessionEnd row).
    pub ended_at: DateTime<Utc>,
    /// The rolling alias the run requested (e.g. `opus`).
    pub alias: String,
    /// The family-latest release identity key the alias meant.
    pub identity_key: String,
    pub family_key: Option<String>,
    /// Whether the vendored artifact was past its max age at stamp time.
    pub stale: bool,
    pub age_days: Option<u64>,
}

/// Model-catalog drift report wrapper for JSON output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriftReport {
    pub range: DateRange,
    pub signals: Vec<DriftSignalSummary>,
    pub aliases: Vec<AliasResolution>,
}

/// One event row within a session detail report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionEvent {
    pub timestamp: DateTime<Utc>,
    pub event: AgenticEvent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_org: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_dirty: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_response: Option<JsonValue>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub cache_read_tokens: u64,
    pub cost_usd: f64,
    /// Raw extra JSON payload from the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<JsonValue>,
    /// Environment context snapshot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<JsonValue>,
    /// Claudine's own process ID for this event row, or `null`.
    ///
    /// Stable nullable query output: always emitted.
    #[serde(default)]
    pub claudine_pid: Option<u32>,
    /// Spawned provider child PID for this event row, or `null`.
    ///
    /// Stable nullable query output: always emitted.
    #[serde(default)]
    pub agent_pid: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sample_session() -> SessionInfo {
        SessionInfo {
            session_key: "k".into(),
            session_id: None,
            provider: Provider::Claude,
            started_at: Utc.timestamp_opt(0, 0).unwrap(),
            ended_at: Utc.timestamp_opt(0, 0).unwrap(),
            duration_seconds: 0,
            cwd: None,
            repo_name: None,
            repo_org: None,
            branch: None,
            package_area: None,
            package: None,
            model: None,
            permission_mode: None,
            hostname: None,
            primary_language: None,
            event_count: 0,
            turn_count: 0,
            tool_call_count: 0,
            tool_error_count: 0,
            turn_error_count: 0,
            subagent_count: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_tokens: 0,
            total_cache_read_tokens: 0,
            total_cost_usd: 0.0,
            claudine_pid: None,
            agent_pid: None,
        }
    }

    /// Review-1 Finding 2 — report output exposes a stable nullable
    /// `agent_pid` (and `claudine_pid`): the key is present and `null`, not
    /// omitted, so machine consumers can tell "no PID" from "old schema".
    #[test]
    fn session_info_emits_null_pids_not_omitted() {
        let json = serde_json::to_value(sample_session()).unwrap();
        assert!(json.get("agent_pid").is_some(), "agent_pid key must exist");
        assert!(json["agent_pid"].is_null(), "agent_pid must be null when absent");
        assert!(json["claudine_pid"].is_null(), "claudine_pid must be null when absent");
    }

    /// D9 round-trip: every facet, the structured detail, and the one-level
    /// cause survive serialization. A record that keeps only `message` cannot
    /// pass this.
    #[test]
    fn sync_failure_round_trips_every_facet_detail_and_cause() {
        let original = SyncFailure {
            source_file: "/logs/a.jsonl".into(),
            line_number: 7,
            message: "boom".into(),
            diagnostic: Some(DiagnosticSnapshot {
                schema_version: 1,
                category: "io".into(),
                code: "io.read_failed".into(),
                disposition: "correctable".into(),
                origin: "environment".into(),
                severity: "error".into(),
                detail: serde_json::json!({ "path": "/logs/a.jsonl" }),
                message: "could not read".into(),
                cause: Some(crate::diagnostics::DiagnosticCause {
                    category: "config".into(),
                    code: "config.invalid".into(),
                    disposition: "correctable".into(),
                    origin: "caller".into(),
                    severity: "error".into(),
                    detail: serde_json::json!({ "field": null, "message": "bad line" }),
                    message: "bad line".into(),
                }),
            }),
        };

        let text = serde_json::to_string(&original).unwrap();
        let restored: SyncFailure = serde_json::from_str(&text).unwrap();
        assert_eq!(restored, original);
    }

    /// D9 backward compatibility: a record persisted before the snapshot field
    /// existed must still deserialize, reporting "no snapshot" rather than
    /// failing the whole read.
    #[test]
    fn a_pre_snapshot_record_still_deserializes() {
        let legacy = r#"{
            "source_file": "/logs/old.jsonl",
            "line_number": 3,
            "message": "legacy failure"
        }"#;

        let restored: SyncFailure = serde_json::from_str(legacy).unwrap();
        assert_eq!(restored.line_number, 3);
        assert_eq!(restored.message, "legacy failure");
        assert!(restored.diagnostic.is_none());

        // Non-vacuity: the legacy payload really does omit the field, so the
        // read above succeeds because `diagnostic` is optional — not because
        // the fixture quietly carries a snapshot.
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct RequiredSnapshot {
            diagnostic: DiagnosticSnapshot,
        }
        assert!(serde_json::from_str::<RequiredSnapshot>(legacy).is_err());
    }

    /// D9 forward compatibility: a newer producer's unknown code and unknown
    /// detail key survive an older consumer's read/write cycle untouched — the
    /// facets are owned strings and `detail` is opaque precisely so this holds.
    #[test]
    fn an_unknown_code_and_detail_key_survive_a_read_write_cycle() {
        let newer = r#"{
            "source_file": "/logs/new.jsonl",
            "line_number": 1,
            "message": "from the future",
            "diagnostic": {
                "schema_version": 1,
                "category": "io",
                "code": "io.future_condition",
                "disposition": "correctable",
                "origin": "environment",
                "severity": "error",
                "detail": { "path": "/logs/new.jsonl", "future_key": [1, 2] },
                "message": "from the future"
            }
        }"#;

        let restored: SyncFailure = serde_json::from_str(newer).unwrap();
        let snapshot = restored.diagnostic.as_ref().expect("a snapshot");
        assert_eq!(snapshot.code, "io.future_condition");
        assert_eq!(snapshot.detail["future_key"], serde_json::json!([1, 2]));

        let round_tripped: SyncFailure =
            serde_json::from_str(&serde_json::to_string(&restored).unwrap()).unwrap();
        assert_eq!(round_tripped, restored);
    }

    /// Review-1 Finding 3 — event-row query DTOs expose stable nullable PID
    /// columns sourced from the `events` table.
    #[test]
    fn error_record_emits_null_pids_not_omitted() {
        let record = ErrorRecord {
            timestamp: Utc.timestamp_opt(0, 0).unwrap(),
            provider: Provider::Claude,
            event: AgenticEvent::ToolError,
            session_key: "k".into(),
            session_id: None,
            repo_name: None,
            tool_name: None,
            model: None,
            error: "boom".into(),
            prompt: None,
            tool_input: None,
            notification_message: None,
            extra: None,
            claudine_pid: Some(42),
            agent_pid: None,
        };
        let json = serde_json::to_value(&record).unwrap();
        assert_eq!(json["claudine_pid"], 42);
        assert!(json.get("agent_pid").is_some(), "agent_pid key must exist");
        assert!(json["agent_pid"].is_null(), "agent_pid must be null when absent");
    }
}

/// Full detail for a single session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionDetailReport {
    pub session: SessionInfo,
    pub events: Vec<SessionEvent>,
    pub tools: Vec<DailyToolStat>,
    pub errors: Vec<ErrorRecord>,
}
