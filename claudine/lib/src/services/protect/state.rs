use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::events::Provider;
use crate::permissions::PolicyCertainty;

use super::config::ProtectPhase;
use super::decision::{ProtectEvaluation, ProtectFindingSource, ProtectOutcome, ProtectPolicyMode};

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
    pub(crate) fn record_evaluation(
        &mut self,
        provider: Provider,
        phase: ProtectPhase,
        eval: &ProtectEvaluation,
        session_id: Option<&str>,
    ) {
        self.decision_count += 1;

        let completion_retry_count = if phase == ProtectPhase::Completion {
            Some(
                self.completion_retries_by_session
                    .get(session_id.unwrap_or(GLOBAL_SESSION_KEY))
                    .copied()
                    .unwrap_or(0),
            )
        } else {
            None
        };

        // Worst certainty across all findings
        let certainty_summary = eval
            .findings
            .iter()
            .map(|f| f.result.certainty)
            .min_by_key(|c| match c {
                PolicyCertainty::Exact => 2,
                PolicyCertainty::BestEffort => 1,
                PolicyCertainty::Unknown => 0,
            })
            .unwrap_or(PolicyCertainty::Unknown);

        // Unique finding sources
        let mut finding_sources: Vec<ProtectFindingSource> = eval
            .findings
            .iter()
            .map(|f| f.source.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        finding_sources.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));

        // Unique source IDs from matched rules
        let mut source_ids: Vec<String> = eval
            .findings
            .iter()
            .flat_map(|f| {
                f.result
                    .matched_rules
                    .iter()
                    .map(|r| r.provenance.source_id.clone())
            })
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        source_ids.sort();

        self.recent.push_back(ProtectDecisionRecord {
            provider,
            phase,
            outcome: eval.decision.outcome.clone(),
            desired_outcome: eval.decision.desired_outcome.clone(),
            degraded: eval.decision.degraded,
            reason: eval.decision.reason.clone(),
            session_id: session_id.map(ToOwned::to_owned),
            policy_mode: eval.policy_mode.clone(),
            intent_count: eval.findings.len(),
            finding_sources,
            certainty_summary,
            source_ids,
            redaction_applied: eval.redaction.is_some(),
            provider_warnings: eval.warnings.len(),
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
pub struct ProtectStateExport {
    pub decision_count: u64,
    pub records: Vec<ProtectDecisionRecord>,
    pub completion_retries_by_session: HashMap<String, u8>,
}

/// Lightweight decision log entry for telemetry/report generation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtectDecisionRecord {
    pub provider: Provider,
    pub phase: ProtectPhase,
    pub outcome: ProtectOutcome,
    pub desired_outcome: ProtectOutcome,
    pub degraded: bool,
    pub reason: String,
    #[serde(default)]
    pub session_id: Option<String>,
    pub policy_mode: ProtectPolicyMode,
    pub intent_count: usize,
    pub finding_sources: Vec<ProtectFindingSource>,
    pub certainty_summary: PolicyCertainty,
    pub source_ids: Vec<String>,
    pub redaction_applied: bool,
    pub provider_warnings: usize,
    #[serde(default)]
    pub completion_retry_count: Option<u8>,
}
