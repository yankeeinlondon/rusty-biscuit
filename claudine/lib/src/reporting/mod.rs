pub mod ingest;
pub mod metrics;
pub mod paths;
pub mod queries;
pub mod schema;
pub mod types;

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::Result;

pub use types::{
    DailySummary, DailyToolStat, DateRange, DerivedMetrics, ErrorRecord, ErrorsReport,
    LabeledCount, ProviderSplit, RepoActivity, ReportingFilters, ReposReport, SessionInfo,
    SessionsReport, SyncFailure, SyncRequest, SyncSummary, ToolActionClass, ToolsReport,
    TrendPoint, TrendsReport,
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

    /// Return the directory containing source JSONL logs.
    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }

    /// Return the SQLite database path.
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
