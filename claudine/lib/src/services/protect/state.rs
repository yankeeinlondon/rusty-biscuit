#![allow(deprecated)] // ProtectInput is deprecated but still used in the legacy evaluation path.

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::events::Provider;

use super::config::{ProtectPhase, ProtectRuntimeMode, RiskLevel};
use super::decision::{ProtectDecision, ProtectOutcome};
#[allow(deprecated)]
use super::evaluate::ProtectInput;

pub(crate) const GLOBAL_SESSION_KEY: &str = "__global__";

/// In-memory rolling state retained by the protect evaluator.
#[derive(Debug, Clone, Default)]
pub struct ProtectState {
    /// Total decisions evaluated by this service instance.
    pub decision_count: u64,
    /// Bounded rolling records for post-run audits.
    pub recent: VecDeque<ProtectDecisionRecord>,
    /// Per-session completion retries used for loop protection.
    pub completion_retries_by_session: HashMap<String, u8>,
}

impl ProtectState {
    pub(crate) fn record(&mut self, input: &ProtectInput, decision: &ProtectDecision) {
        self.decision_count += 1;

        let completion_retry_count = if input.phase == ProtectPhase::Completion {
            Some(
                self.completion_retries_by_session
                    .get(input.session_id.as_deref().unwrap_or(GLOBAL_SESSION_KEY))
                    .copied()
                    .unwrap_or(0),
            )
        } else {
            None
        };

        self.recent.push_back(ProtectDecisionRecord {
            provider: input.provider,
            phase: input.phase,
            mode: input.runtime_mode,
            risk: input.risk,
            outcome: decision.outcome.clone(),
            degraded: decision.degraded,
            degraded_from: decision.degraded_from.clone(),
            reason: decision.reason.clone(),
            session_id: input.session_id.clone(),
            completion_retry_count,
        });
    }

    /// Snapshot decision records as an owned list.
    pub fn snapshot_records(&self) -> Vec<ProtectDecisionRecord> {
        self.recent.iter().cloned().collect()
    }

    /// Export full protect state for telemetry/reporting.
    pub fn export_state(&self) -> ProtectStateExport {
        ProtectStateExport {
            decision_count: self.decision_count,
            records: self.snapshot_records(),
            completion_retries_by_session: self.completion_retries_by_session.clone(),
        }
    }

    /// Export records as JSON Lines.
    pub fn export_records_jsonl(&self) -> Result<String> {
        let mut out = String::new();
        for record in &self.recent {
            out.push_str(&serde_json::to_string(record)?);
            out.push('\n');
        }
        Ok(out)
    }
}

/// Export shape for protect state snapshots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectStateExport {
    pub decision_count: u64,
    pub records: Vec<ProtectDecisionRecord>,
    pub completion_retries_by_session: HashMap<String, u8>,
}

/// Lightweight decision log entry for telemetry/report generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectDecisionRecord {
    pub provider: Provider,
    pub phase: ProtectPhase,
    pub mode: ProtectRuntimeMode,
    pub risk: RiskLevel,
    pub outcome: ProtectOutcome,
    pub degraded: bool,
    #[serde(default)]
    pub degraded_from: Option<ProtectOutcome>,
    pub reason: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub completion_retry_count: Option<u8>,
}
