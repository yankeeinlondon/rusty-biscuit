//! Reading an operation saved by an earlier `wt` (recovery record version 1,
//! with reports up to version 6) so it can be migrated to a
//! [`CompactRecord`].
//!
//! The record and its report are read separately. The record alone carries
//! the checkout-removal approval; a report is optional, and neither its
//! version nor its content grants anything. A report that is absent, corrupt,
//! or of another operation never blocks migration.
//!
//! Remote deletion authority is never inferred from a legacy action's remote
//! name, since the record binds no push endpoint or remote head: such a step
//! migrates as [`BranchAction::Unbound`], is never run, and the removal
//! reports it and exits 1.

use super::compact::{
    BranchAction, BranchStep, COMPACT_RECORD_VERSION, CompactRecord, DeletionMode, Progress, StepApproval, StepState,
    UnboundTarget,
};
use super::record::{self, BranchBinding, IntendedAction, Phase, RecordInvalid, RecoveryRecord};
use crate::strict_json;

/// The newest report version a legacy operation can carry.
pub const LEGACY_REPORT_VERSION: u64 = 6;

/// A validated legacy record and what was found of its report.
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyApproval {
    pub record: RecoveryRecord,
    pub report: LegacyReport,
}

/// What a legacy operation's `report.json` was found to be. Informational
/// only: no variant changes what the migration authorizes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyReport {
    Absent,
    Read { version: u64 },
    /// Unreadable, of an unknown version, or of another operation.
    Ignored { reason: String },
}

/// Reads legacy record `record` and, when present, its `report`.
///
/// ## Errors
///
/// The record's own [`RecordInvalid`]; a report never causes an error.
pub fn read(record: &[u8], report: Option<&[u8]>) -> Result<LegacyApproval, RecordInvalid> {
    let record = record::parse(record)?;
    let report = match report {
        None => LegacyReport::Absent,
        Some(bytes) => read_report(bytes, &record),
    };
    Ok(LegacyApproval { record, report })
}

fn read_report(bytes: &[u8], record: &RecoveryRecord) -> LegacyReport {
    let ignored = |reason: &str| LegacyReport::Ignored { reason: reason.to_string() };
    let Some(members) = strict_json::members(bytes) else {
        return ignored("not one JSON object with distinct keys");
    };
    let Some(version) = members.get::<u64>("report_version") else {
        return ignored("report_version is missing or not a number");
    };
    if !(1..=LEGACY_REPORT_VERSION).contains(&version) {
        return ignored("report_version is not one an earlier wt wrote");
    }
    match members.get::<Option<String>>("operation_id") {
        Some(Some(id)) if id == record.operation_id.as_str() => LegacyReport::Read { version },
        Some(None) => LegacyReport::Read { version },
        _ => ignored("the report names another operation"),
    }
}

impl LegacyApproval {
    /// Every requested step finished; nothing is left to migrate.
    pub fn is_completed(&self) -> bool {
        self.record.completed_phase == Phase::Completed
    }

    /// The compact record that continues this operation. Progress is mapped
    /// conservatively (a step recorded as started is not treated as
    /// finished), a local branch step keeps its approved tip and has its
    /// safety revalidated, and every step whose binding cannot be rebuilt is
    /// [`BranchAction::Unbound`].
    pub fn to_compact(&self) -> CompactRecord {
        let legacy = &self.record;
        let progress = match legacy.completed_phase {
            Phase::Prepared => Progress::Approved,
            Phase::Moved | Phase::Associated | Phase::Deleting => Progress::Placed,
            Phase::ContentRemoved => Progress::ContentRemoved,
            Phase::Unregistered | Phase::BranchActions | Phase::Completed => Progress::Unregistered,
        };
        let branch_steps = legacy
            .intended_actions
            .iter()
            .filter_map(|action| match action {
                IntendedAction::RemoveCheckout => None,
                IntendedAction::DeleteLocalBranch { name } if legacy.branch == BranchBinding::Named(name.clone()) => {
                    Some(BranchStep {
                        action: BranchAction::Local { name: name.clone(), expected_tip: legacy.approved_tip.clone() },
                        approval: StepApproval::Conditional,
                        state: StepState::Pending,
                    })
                }
                IntendedAction::DeleteLocalBranch { name } => Some(unbound(UnboundTarget::Local { name: name.clone() })),
                IntendedAction::DeleteRemoteBranch { remote, name } => {
                    Some(unbound(UnboundTarget::Remote { remote: remote.clone(), name: name.clone() }))
                }
            })
            .collect();
        CompactRecord {
            record_version: COMPACT_RECORD_VERSION,
            operation_id: legacy.operation_id.clone(),
            created_at_ms: legacy.created_at_ms,
            requested_name: legacy.requested_name.clone(),
            aliases: legacy.aliases.clone(),
            original: legacy.original.clone(),
            deletion_path: legacy.quarantine.clone(),
            deletion_mode: DeletionMode::Moved,
            checkout_identity: legacy.checkout_identity,
            repository: legacy.repository.clone(),
            unlock_needed: false,
            branch_steps,
            progress,
        }
    }
}

fn unbound(target: UnboundTarget) -> BranchStep {
    BranchStep { action: BranchAction::Unbound { target }, approval: StepApproval::Explicit, state: StepState::Kept }
}

#[cfg(test)]
mod tests;
