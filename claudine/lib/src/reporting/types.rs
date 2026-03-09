use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::events::{AgenticEvent, Provider};

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncFailure {
    /// JSONL file path that failed.
    pub source_file: String,
    /// One-based line number that failed to parse.
    pub line_number: usize,
    /// Human-readable failure message.
    pub message: String,
}

/// Result of a sync run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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
}

/// Session list wrapper for JSON output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionsReport {
    pub range: DateRange,
    pub sessions: Vec<SessionInfo>,
    pub metrics: DerivedMetrics,
}

/// Error/event record with short context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorRecord {
    pub timestamp: DateTime<Utc>,
    pub provider: Provider,
    pub event: AgenticEvent,
    pub session_key: String,
    pub repo_name: Option<String>,
    pub tool_name: Option<String>,
    pub error: String,
    pub context: Option<String>,
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
    pub sessions: u64,
    pub turns: u64,
    pub tool_calls: u64,
    pub errors: u64,
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
