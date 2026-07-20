pub mod error;
pub mod ingest;
pub mod metrics;
pub mod paths;
pub mod queries;
pub mod schema;
pub mod types;

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::Result;

pub use error::IngestError;
pub use types::{
    AliasResolution, DailySummary, DailyToolStat, DateRange, DerivedMetrics, DriftReport,
    DriftSignalSummary, ErrorRecord, ErrorsReport, LabeledCount, ProviderSplit, RepoActivity,
    ReportingFilters, ReposReport, SessionDetailReport, SessionEvent, SessionInfo, SessionsReport,
    SyncFailure, SyncRequest, SyncSummary, ToolActionClass, ToolsReport, TrendPoint, TrendsReport,
    UsageTotals,
};

/// SQLite-backed reporting index built from Claudine JSONL event logs.
pub struct ReportingStore {
    connection: Connection,
    logs_dir: PathBuf,
    db_path: PathBuf,
}

impl ReportingStore {
    /// Open the default reporting database under `~/.claudine/logs/metrics.db`.
    pub fn open_default() -> Result<Self> {
        let logs_dir = paths::default_logs_dir()?;
        let db_path = paths::default_metrics_db_path()?;
        Self::open(&logs_dir, &db_path)
    }

    /// Open a reporting store using explicit paths.
    pub fn open(logs_dir: &Path, db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::create_dir_all(logs_dir)?;

        let connection = Connection::open(db_path)?;
        schema::initialize(&connection)?;

        Ok(Self {
            connection,
            logs_dir: logs_dir.to_path_buf(),
            db_path: db_path.to_path_buf(),
        })
    }

    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Incrementally ingest JSONL logs into the SQLite index.
    pub fn sync(&mut self, request: SyncRequest) -> Result<SyncSummary> {
        ingest::sync(&mut self.connection, &self.logs_dir, request)
    }

    /// Query one day's top-level summary.
    pub fn daily_summary(
        &self,
        date: chrono::NaiveDate,
        filters: &ReportingFilters,
    ) -> Result<DailySummary> {
        queries::daily_summary(&self.connection, date, filters)
    }

    /// Query sessions for an inclusive date range.
    pub fn sessions(&self, range: DateRange, filters: &ReportingFilters) -> Result<SessionsReport> {
        queries::sessions(&self.connection, range, filters)
    }

    /// Query tool usage for an inclusive date range.
    pub fn tools(
        &self,
        range: DateRange,
        filters: &ReportingFilters,
        top_n: usize,
    ) -> Result<ToolsReport> {
        queries::tools(&self.connection, range, filters, top_n)
    }

    /// Query error events for an inclusive date range.
    pub fn errors(
        &self,
        range: DateRange,
        filters: &ReportingFilters,
        top_n: usize,
    ) -> Result<ErrorsReport> {
        queries::errors(&self.connection, range, filters, top_n)
    }

    /// Query repository activity for an inclusive date range.
    pub fn repos(&self, range: DateRange, filters: &ReportingFilters) -> Result<ReposReport> {
        queries::repos(&self.connection, range, filters)
    }

    /// Query model-catalog signal activity (drift, resolutions,
    /// fallbacks) and `family_latest` alias stamps for a date range.
    pub fn drift(
        &self,
        range: DateRange,
        filters: &ReportingFilters,
        top_n: usize,
    ) -> Result<DriftReport> {
        queries::drift(&self.connection, range, filters, top_n)
    }

    /// Query full detail for a single session by key or ID.
    pub fn session_detail(&self, id: &str) -> Result<SessionDetailReport> {
        queries::session_detail(&self.connection, id)
    }

    /// Query daily trends for an inclusive date range.
    pub fn trends(
        &self,
        range: DateRange,
        filters: &ReportingFilters,
        top_n: usize,
    ) -> Result<TrendsReport> {
        queries::trends(&self.connection, range, filters, top_n)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::{TimeZone, Utc};
    use tempfile::tempdir;

    use crate::events::{AgenticEvent, EnvironmentContext, EventMeta};

    use super::*;
    use crate::provider::Provider;

    #[test]
    fn reporting_filters_support_package_area_and_package() {
        let workspace = tempdir().unwrap();
        let logs_dir = workspace.path().join("logs");
        let db_path = workspace.path().join("metrics.db");
        fs::create_dir_all(&logs_dir).unwrap();

        let date = chrono::NaiveDate::from_ymd_opt(2026, 3, 7).unwrap();
        let log_path = logs_dir.join("2026-03-07.jsonl");

        let claudine_session = EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::SessionStart,
            timestamp: Utc.with_ymd_and_hms(2026, 3, 7, 18, 0, 0).unwrap(),
            session_id: Some("claudine-session".to_string()),
            cwd: Some("/Volumes/coding/personal/rusty-biscuit".to_string()),
            tool_name: None,
            tool_input: None,
            tool_response: None,
            error: None,
            prompt: None,
            agent_type: None,
            notification_type: None,
            notification_message: None,
            agent_pid: Some(12_345),
            extra: Default::default(),
            env: EnvironmentContext {
                package_area: Some("claudine".to_string()),
                package: Some("claudine-cli".to_string()),
                claudine_pid: Some(54_321),
                ..EnvironmentContext::default()
            },
        };

        let sniff_session = EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::SessionStart,
            timestamp: Utc.with_ymd_and_hms(2026, 3, 7, 19, 0, 0).unwrap(),
            session_id: Some("sniff-session".to_string()),
            cwd: Some("/Volumes/coding/personal/rusty-biscuit".to_string()),
            tool_name: None,
            tool_input: None,
            tool_response: None,
            error: None,
            prompt: None,
            agent_type: None,
            notification_type: None,
            notification_message: None,
            agent_pid: None,
            extra: Default::default(),
            env: EnvironmentContext {
                package_area: Some("sniff".to_string()),
                package: Some("sniff-cli".to_string()),
                claudine_pid: None,
                ..EnvironmentContext::default()
            },
        };

        fs::write(
            &log_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&claudine_session).unwrap(),
                serde_json::to_string(&sniff_session).unwrap()
            ),
        )
        .unwrap();

        let mut store = ReportingStore::open(&logs_dir, &db_path).unwrap();
        store.sync(SyncRequest::All).unwrap();

        let report = store
            .sessions(
                DateRange::single(date),
                &ReportingFilters {
                    package_area: Some("claudine".to_string()),
                    package: Some("claudine-cli".to_string()),
                    ..ReportingFilters::default()
                },
            )
            .unwrap();

        assert_eq!(report.sessions.len(), 1);
        assert_eq!(report.sessions[0].package_area.as_deref(), Some("claudine"));
        assert_eq!(report.sessions[0].package.as_deref(), Some("claudine-cli"));
        assert_eq!(
            report.sessions[0].session_id.as_deref(),
            Some("claudine-session")
        );
        assert_eq!(report.sessions[0].claudine_pid, Some(54_321));
        assert_eq!(report.sessions[0].agent_pid, Some(12_345));
    }

    /// Review-1 Finding 3 — event-row queries (errors, session-detail events)
    /// project the `events` table PID columns into their DTOs.
    #[test]
    fn errors_and_session_detail_project_pid_columns() {
        let workspace = tempdir().unwrap();
        let logs_dir = workspace.path().join("logs");
        let db_path = workspace.path().join("metrics.db");
        fs::create_dir_all(&logs_dir).unwrap();

        let date = chrono::NaiveDate::from_ymd_opt(2026, 3, 8).unwrap();
        let log_path = logs_dir.join("2026-03-08.jsonl");

        let env = EnvironmentContext {
            claudine_pid: Some(54_321),
            ..EnvironmentContext::default()
        };

        let start = EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::SessionStart,
            timestamp: Utc.with_ymd_and_hms(2026, 3, 8, 18, 0, 0).unwrap(),
            session_id: Some("pid-session".to_string()),
            cwd: Some("/repo".to_string()),
            tool_name: None,
            tool_input: None,
            tool_response: None,
            error: None,
            prompt: None,
            agent_type: None,
            notification_type: None,
            notification_message: None,
            agent_pid: Some(12_345),
            extra: Default::default(),
            env: env.clone(),
        };

        let tool_error = EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::ToolError,
            timestamp: Utc.with_ymd_and_hms(2026, 3, 8, 18, 0, 5).unwrap(),
            session_id: Some("pid-session".to_string()),
            cwd: Some("/repo".to_string()),
            tool_name: Some("Bash".to_string()),
            tool_input: None,
            tool_response: None,
            error: Some("boom".to_string()),
            prompt: None,
            agent_type: None,
            notification_type: None,
            notification_message: None,
            agent_pid: Some(12_345),
            extra: Default::default(),
            env,
        };

        fs::write(
            &log_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&start).unwrap(),
                serde_json::to_string(&tool_error).unwrap()
            ),
        )
        .unwrap();

        let mut store = ReportingStore::open(&logs_dir, &db_path).unwrap();
        store.sync(SyncRequest::All).unwrap();

        let errors = store
            .errors(DateRange::single(date), &ReportingFilters::default(), 10)
            .unwrap();
        assert_eq!(errors.errors.len(), 1);
        assert_eq!(errors.errors[0].claudine_pid, Some(54_321));
        assert_eq!(errors.errors[0].agent_pid, Some(12_345));

        let detail = store.session_detail("pid-session").unwrap();
        let error_event = detail
            .events
            .iter()
            .find(|e| e.event == AgenticEvent::ToolError)
            .expect("tool_error event must be present");
        assert_eq!(error_event.claudine_pid, Some(54_321));
        assert_eq!(error_event.agent_pid, Some(12_345));
        assert_eq!(detail.errors[0].agent_pid, Some(12_345));
    }

    /// End-to-end model-catalog reporting path: a SessionEnd summary row
    /// (built by the real summary builder, so the JSONL shape cannot
    /// drift from what ingest expects) is exploded into `session_signals`
    /// and its `family_latest` stamp lifted onto `sessions`, then both
    /// surface through `ReportingStore::drift`.
    #[test]
    fn drift_report_surfaces_signals_and_alias_stamps() {
        use crate::model_catalog::FamilyLatestStamp;
        use crate::signals::{DriftObservation, ObservedSignal, SignalEvent, SignalSource};
        use crate::stream::StreamProtocol;
        use crate::stream::reporting::summary_to_event_meta_with_context;
        use crate::stream::summary::StreamExecutionSummary;

        let workspace = tempdir().unwrap();
        let logs_dir = workspace.path().join("logs");
        let db_path = workspace.path().join("metrics.db");
        fs::create_dir_all(&logs_dir).unwrap();

        let summary = StreamExecutionSummary {
            provider: Provider::Claude,
            session_id: Some("drift-session".to_string()),
            ..StreamExecutionSummary::default()
        };
        let signals = vec![ObservedSignal {
            event: SignalEvent::ModelCatalogDrift {
                unexpected: vec!["mystery-model".to_string()],
                missing: vec!["claude-legacy".to_string()],
                observed_via: DriftObservation::Listing,
            },
            source: SignalSource::Wrapper,
            first_seen: Utc.with_ymd_and_hms(2026, 3, 9, 11, 59, 0).unwrap(),
            occurrences: 3,
            context: Default::default(),
        }];
        let stamp = FamilyLatestStamp {
            alias: "opus".to_string(),
            identity_key: "anthropic/claude-opus@4.5".to_string(),
            family_key: "anthropic/claude-opus".to_string(),
            artifact_generated_at: "2026-03-01T00:00:00+00:00".to_string(),
            stale: false,
            age_days: None,
        };
        let mut meta = summary_to_event_meta_with_context(
            &summary,
            StreamProtocol::StreamJson,
            &EnvironmentContext::default(),
            None,
            None,
            &signals,
            Some(&stamp),
        );
        meta.timestamp = Utc.with_ymd_and_hms(2026, 3, 9, 12, 0, 0).unwrap();

        fs::write(
            logs_dir.join("2026-03-09.jsonl"),
            format!("{}\n", serde_json::to_string(&meta).unwrap()),
        )
        .unwrap();

        let mut store = ReportingStore::open(&logs_dir, &db_path).unwrap();
        store.sync(SyncRequest::All).unwrap();

        let date = chrono::NaiveDate::from_ymd_opt(2026, 3, 9).unwrap();
        let report = store
            .drift(DateRange::single(date), &ReportingFilters::default(), 10)
            .unwrap();

        assert_eq!(report.signals.len(), 1);
        let signal = &report.signals[0];
        assert_eq!(signal.provider, Provider::Claude);
        assert_eq!(signal.kind, "model_catalog_drift");
        assert_eq!(signal.session_count, 1);
        assert_eq!(signal.occurrences, 3);
        assert_eq!(signal.unexpected, vec!["mystery-model".to_string()]);
        assert_eq!(signal.missing, vec!["claude-legacy".to_string()]);
        assert_eq!(signal.observed_via.as_deref(), Some("listing"));

        assert_eq!(report.aliases.len(), 1);
        let alias = &report.aliases[0];
        assert_eq!(alias.session_id.as_deref(), Some("drift-session"));
        assert_eq!(alias.alias, "opus");
        assert_eq!(alias.identity_key, "anthropic/claude-opus@4.5");
        assert_eq!(alias.family_key.as_deref(), Some("anthropic/claude-opus"));
        assert!(!alias.stale);

        // A provider filter for another provider excludes both sections.
        let filtered = store
            .drift(
                DateRange::single(date),
                &ReportingFilters {
                    provider: Some(Provider::Gemini),
                    ..ReportingFilters::default()
                },
                10,
            )
            .unwrap();
        assert!(filtered.signals.is_empty());
        assert!(filtered.aliases.is_empty());
    }
}
