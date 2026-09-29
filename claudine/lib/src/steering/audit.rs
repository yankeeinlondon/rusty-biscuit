//! Steering audit records.
//!
//! Every steering request and its outcome are appended as typed JSONL records
//! to `~/.claudine/logs/steering/<local-date>.jsonl`. A record keeps the full
//! message text after [`Redactor`] masking, never the original; error and
//! provider-echo text is redacted with the same message-aware redactor before
//! it is persisted or handed back for tracing and rendering.
//!
//! Auditing never decides delivery. [`audited_send`] calls the delivery
//! closure exactly once whether or not a record can be written, returns the
//! delivery's own result, and reports write failures as content-free
//! [`AuditFailure`]s; an awaited delivery uses its two halves,
//! [`PendingAudit`]. A result that arrives after the request returned is an
//! append-only `late_result` record correlated by request ID
//! ([`LateResultRecorder`]); it can never become another send.
//!
//! Records are append-only and follow the logs directory's retention, which
//! today keeps files until the user removes them.
//!
//! Topic: `claudine/docs/topics/traces-and-logging.md#steering-audit-records`.

use std::path::PathBuf;

use chrono::{DateTime, Local, Utc};
use serde::Serialize;

use super::contract::{InterruptionConsent, InterruptionOutcome, SendOutcome, SteeringOrigin, SteeringRequest, SteeringResult};
use super::identity::{ConversationGeneration, ExecutionId, OpportunityId, RequestId, SteeringTargetId};
use super::vocabulary::{OperationIntent, ReceiptStrength};
use crate::provider_id::Provider;
use crate::secrets::{RedactedText, Redactor};

#[cfg(test)]
mod tests;

/// Value of every steering record's `record` field.
pub const AUDIT_RECORD: &str = "steering";

/// Version of the steering record shape.
pub const AUDIT_SCHEMA: u32 = 1;

/// Route facts the request itself does not carry.
#[derive(Debug, Clone, Default)]
pub struct AuditContext {
    pub provider: Option<Provider>,
    /// Provider conversation the target was bound to when selected.
    pub conversation: Option<String>,
    pub generation: Option<ConversationGeneration>,
    /// Launch or execution-interface profile.
    pub profile: Option<String>,
    /// Researched mechanism selected for delivery.
    pub mechanism: Option<&'static str>,
    /// The automatic warning opportunity this request serves.
    pub opportunity: Option<OpportunityId>,
}

/// What one delivery attempt reported. Error and echo text are unredacted
/// here; they are redacted before they leave this module.
#[derive(Debug, Clone)]
pub struct DeliveryReport {
    pub result: SteeringResult,
    pub error: Option<String>,
    /// Provider response text that may repeat the message.
    pub provider_echo: Option<String>,
}

impl DeliveryReport {
    pub fn new(result: SteeringResult) -> Self {
        Self { result, error: None, provider_echo: None }
    }
}

/// One JSONL audit line.
#[derive(Debug, Clone, Serialize)]
pub struct AuditRecord {
    pub record: &'static str,
    pub schema: u32,
    pub timestamp: DateTime<Utc>,
    pub request_id: RequestId,
    #[serde(flatten)]
    pub entry: AuditEntry,
}

/// The record kinds, tagged by `kind`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuditEntry {
    Request(Box<RequestEntry>),
    Result(ResultEntry),
    /// An outcome that arrived after the request returned. Correlated by
    /// `request_id`; it is an update, not another send.
    LateResult(ResultEntry),
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestEntry {
    pub target: SteeringTargetId,
    pub execution: Option<ExecutionId>,
    pub origin: SteeringOrigin,
    pub opportunity: Option<OpportunityId>,
    pub operation: OperationIntent,
    pub provider: Option<Provider>,
    pub conversation: Option<String>,
    pub generation: Option<ConversationGeneration>,
    pub profile: Option<String>,
    pub mechanism: Option<&'static str>,
    pub consent: Option<InterruptionConsent>,
    pub may_interrupt: bool,
    /// The full message after masking.
    pub message: RedactedText,
    /// Size of the original message in UTF-8 bytes.
    pub message_bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultEntry {
    pub mechanism: Option<&'static str>,
    pub outcome: SendOutcome,
    pub receipt: ReceiptStrength,
    /// Separate cancellation and replacement outcomes of an interruption.
    pub interruption: Option<InterruptionOutcome>,
    pub error: Option<RedactedText>,
    pub provider_echo: Option<RedactedText>,
}

/// Why an audit record was not written. Carries no message, error, or path
/// text, so it is safe to trace and render.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AuditFailure {
    #[error("steering audit location is unavailable")]
    LocationUnavailable,
    #[error("steering audit record could not be written ({0:?})")]
    Write(std::io::ErrorKind),
    #[error("late steering result does not belong to the recorded request")]
    Uncorrelated,
}

/// The append-only steering audit log.
#[derive(Debug, Clone)]
pub struct SteeringAuditLog {
    dir: Option<PathBuf>,
}

impl SteeringAuditLog {
    /// The per-user location, `~/.claudine/logs/steering/`. When no home
    /// directory resolves, every append reports
    /// [`AuditFailure::LocationUnavailable`].
    pub fn default_location() -> Self {
        Self { dir: crate::reporting::paths::default_steering_logs_dir().ok() }
    }

    /// A log rooted at `dir`.
    pub fn at(dir: impl Into<PathBuf>) -> Self {
        Self { dir: Some(dir.into()) }
    }

    /// Appends one record to today's local-date file.
    pub fn append(&self, record: &AuditRecord) -> Result<(), AuditFailure> {
        let dir = self.dir.as_ref().ok_or(AuditFailure::LocationUnavailable)?;
        let path = dir.join(format!("{}.jsonl", Local::now().format("%Y-%m-%d")));
        crate::reporting::jsonl::append_record(record, &path).map_err(|error| AuditFailure::Write(error.kind()))
    }

    fn append_reporting(&self, record: &AuditRecord, failures: &mut Vec<AuditFailure>) {
        if let Err(failure) = self.append(record) {
            tracing::warn!(%failure, "steering audit record not written; the delivery result is unaffected");
            failures.push(failure);
        }
    }
}

/// The outcome of [`audited_send`].
#[derive(Debug)]
pub struct AuditedSend {
    /// Exactly what delivery reported.
    pub result: SteeringResult,
    pub error: Option<RedactedText>,
    pub provider_echo: Option<RedactedText>,
    /// Records that could not be written.
    pub audit_failures: Vec<AuditFailure>,
    /// Appends a later outcome for this request.
    pub late: LateResultRecorder,
}

/// Records a request, delivers it once with its original text, and records
/// the result.
pub fn audited_send(
    log: &SteeringAuditLog,
    request: &SteeringRequest,
    context: &AuditContext,
    deliver: impl FnOnce(&SteeringRequest) -> DeliveryReport,
) -> AuditedSend {
    let pending = PendingAudit::begin(log, request, context);
    let report = deliver(request);
    pending.finish(log, &report)
}

/// The two halves of [`audited_send`] for a delivery that is awaited rather
/// than called: [`PendingAudit::begin`] records the request, and
/// [`PendingAudit::finish`] records the one result. Being consumed by
/// `finish`, it cannot record a second result as a new send.
#[derive(Debug)]
pub struct PendingAudit {
    request_id: RequestId,
    redactor: Redactor,
    audit_failures: Vec<AuditFailure>,
}

impl PendingAudit {
    /// Writes the masked `request` record.
    pub fn begin(log: &SteeringAuditLog, request: &SteeringRequest, context: &AuditContext) -> Self {
        let redactor = Redactor::for_message(request.message.as_str());
        let mut audit_failures = Vec::new();
        log.append_reporting(&request_record(request, context, &redactor), &mut audit_failures);
        Self { request_id: request.id, redactor, audit_failures }
    }

    /// Writes the `result` record for `report` and returns it redacted.
    pub fn finish(mut self, log: &SteeringAuditLog, report: &DeliveryReport) -> AuditedSend {
        let entry = result_entry(report, &self.redactor);
        let (error, provider_echo) = (entry.error.clone(), entry.provider_echo.clone());
        log.append_reporting(&record(self.request_id, AuditEntry::Result(entry)), &mut self.audit_failures);
        AuditedSend {
            result: report.result.clone(),
            error,
            provider_echo,
            audit_failures: self.audit_failures,
            late: LateResultRecorder { request_id: self.request_id, redactor: self.redactor },
        }
    }
}

/// Appends late outcomes for one request. It holds no delivery path.
#[derive(Debug, Clone)]
pub struct LateResultRecorder {
    request_id: RequestId,
    redactor: Redactor,
}

impl LateResultRecorder {
    pub fn request_id(&self) -> RequestId {
        self.request_id
    }

    /// Appends `report` as a `late_result` for this request. A report for
    /// another request is refused rather than written under this one.
    pub fn record(&self, log: &SteeringAuditLog, report: &DeliveryReport) -> Result<(), AuditFailure> {
        if report.result.request_id != self.request_id {
            return Err(AuditFailure::Uncorrelated);
        }
        let entry = AuditEntry::LateResult(result_entry(report, &self.redactor));
        let mut failures = Vec::new();
        log.append_reporting(&record(self.request_id, entry), &mut failures);
        failures.pop().map_or(Ok(()), Err)
    }
}

fn record(request_id: RequestId, entry: AuditEntry) -> AuditRecord {
    AuditRecord { record: AUDIT_RECORD, schema: AUDIT_SCHEMA, timestamp: Utc::now(), request_id, entry }
}

fn request_record(request: &SteeringRequest, context: &AuditContext, redactor: &Redactor) -> AuditRecord {
    let execution = match &request.target {
        SteeringTargetId::Managed { execution } => Some(*execution),
        SteeringTargetId::Native { .. } => None,
    };
    let text = request.message.as_str();
    record(
        request.id,
        AuditEntry::Request(Box::new(RequestEntry {
            target: request.target.clone(),
            execution,
            origin: request.origin,
            opportunity: context.opportunity,
            operation: request.operation,
            provider: context.provider,
            conversation: context.conversation.clone(),
            generation: context.generation,
            profile: context.profile.clone(),
            mechanism: context.mechanism,
            consent: request.consent.clone(),
            may_interrupt: request.may_interrupt(),
            message: redactor.redact(text),
            message_bytes: text.len(),
        })),
    )
}

fn result_entry(report: &DeliveryReport, redactor: &Redactor) -> ResultEntry {
    ResultEntry {
        mechanism: report.result.mechanism,
        outcome: report.result.outcome,
        receipt: report.result.receipt,
        interruption: report.result.interruption,
        error: report.error.as_deref().map(|text| redactor.redact(text)),
        provider_echo: report.provider_echo.as_deref().map(|text| redactor.redact(text)),
    }
}
