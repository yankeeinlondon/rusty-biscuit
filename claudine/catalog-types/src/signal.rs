//! Claudine-owned signal taxonomy shared by research-fed detection records,
//! the generic detection engine, and the bespoke temporal guards.
//!
//! Design authority:
//! `claudine/features/2026-07-02-provider-metadata/design/signal-detection.md`.
//!
//! Invariant: research sidecar schemas mirror these member lists via
//! `VariantNames`, and the [`SignalEvent`]↔[`SignalKind`] correspondence is
//! enforced by the exhaustive [`SignalEvent::kind`] accessor — adding a kind
//! without a payload variant (or vice versa) is a compile error.

use chrono::{DateTime, Utc};
use serde::Serialize;
use strum::{EnumIter, IntoStaticStr, VariantNames};

use crate::vocab::Unit;

/// The canonical signal taxonomy.
///
/// The sink dedups on kind (with session and correlation window), hence
/// `Hash`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, EnumIter, IntoStaticStr, VariantNames,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SignalKind {
    // Limits / billing.
    UsageCapApproaching,
    UsageCapped,
    RateLimited,
    ProviderOverloaded,
    RetriesExhausted,
    NoFunds,
    // Auth.
    AuthInvalid,
    AuthKindDetected,
    // Permissions.
    PermissionDeniedRead,
    PermissionDeniedWrite,
    // Model / session identity.
    TokensConsumed,
    ModelResolved,
    ModelFallback,
    ProviderVersion,
    // Progress / retry.
    GenerationRetried,
    StalledGeneration,
    RepeatedStreamError,
    // Volume/time guards — permanently bespoke. These deliberately mirror
    // the `EarlyTermination` error_kind strings in
    // `lib/src/stream/logs/opencode/reasoning.rs`.
    Timeout,
    StepTimeout,
    ExitExpression,
    RunawayRepetition,
    RunawayVolume,
    // Outcome / protocol.
    UnsupportedProtocolVersion,
    TurnLimitReached,
    SessionTimeLimitReached,
    Interrupted,
    SessionTainted,
    /// Reserved (no emitter until the durable-HITL/resume-round-2 work).
    HumanInputRequested,
    /// Reserved (no emitter until the durable-HITL/resume-round-2 work).
    SessionResumable,
}

/// Where a signal was observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SignalSource {
    Stream,
    SessionLog,
    AppLog,
    Sqlite,
    Hook,
    /// Free-form stderr diagnostics — no structure guarantee, unlike
    /// [`SignalSource::StderrPromoted`].
    StderrDiagnostic,
    /// Promoted-structured stderr (OpenCode `--print-logs`) — a contract
    /// channel, distinct from [`SignalSource::StderrDiagnostic`].
    StderrPromoted,
    /// ACP `session/update` streams (plan §E.4 mandate).
    Acp,
    /// Wrapper-synthesized payload carrying the child's exit code plus a
    /// stderr tail — covers exits that bypass a `result` event (the Qwen
    /// 53/55/130 path).
    Exit,
}

/// The four ratified detection-record match operators (design doc "Record
/// grammar").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MatchOp {
    /// Typed value equality.
    Eq,
    /// Value-set membership.
    In,
    /// Case-insensitive substring.
    SubstringCi,
    /// Anchored regular expression, compiled at generate time.
    Regex,
}

/// How a taxonomy signal is detected: single-payload matching is
/// declarative; anything needing cross-record or temporal state is bespoke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DetectionMode {
    Declarative,
    Bespoke,
}

/// A measured or extracted value with an explicit unit.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Quantity {
    pub value: f64,
    pub unit: Unit,
}

/// Usage-limit window a cap signal applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum UsageWindow {
    FiveHour,
    SevenDay,
    SevenDayOpus,
    Monthly,
    Unknown,
}

/// Taxonomy-typed signal payload — one variant per [`SignalKind`] member.
///
/// All timestamps are `DateTime<Utc>`: normalization to UTC is the
/// detection engine's job; the type states the contract.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SignalEvent {
    UsageCapApproaching {
        window: UsageWindow,
        resets_at: Option<DateTime<Utc>>,
        remaining: Option<Quantity>,
        message: Option<String>,
    },
    UsageCapped {
        window: UsageWindow,
        lifts_at: Option<DateTime<Utc>>,
        remaining: Option<Quantity>,
        message: Option<String>,
    },
    RateLimited {
        status_code: Option<u16>,
        reset_at: Option<DateTime<Utc>>,
        retry_after: Option<Quantity>,
        message: Option<String>,
    },
    ProviderOverloaded {
        status_code: Option<u16>,
        message: Option<String>,
    },
    RetriesExhausted {
        status_code: Option<u16>,
        attempts: Option<u32>,
        message: Option<String>,
    },
    NoFunds {
        message: Option<String>,
    },
    AuthInvalid {
        auth_kind: Option<String>,
        expired: Option<bool>,
        message: Option<String>,
    },
    AuthKindDetected {
        auth_kind: String,
    },
    PermissionDeniedRead {
        path: Option<String>,
        message: Option<String>,
    },
    PermissionDeniedWrite {
        path: Option<String>,
        message: Option<String>,
    },
    TokensConsumed {
        input: Option<Quantity>,
        output: Option<Quantity>,
        total: Option<Quantity>,
    },
    ModelResolved {
        requested: Option<String>,
        resolved: String,
    },
    ModelFallback {
        from: Option<String>,
        to: String,
        message: Option<String>,
    },
    /// Once observed in-session, record selection narrows to
    /// version-admitting records (design §Version-range selection).
    ProviderVersion {
        version: String,
    },
    /// Kimi Wire 1.10 `StepRetry` is the first producer.
    GenerationRetried {
        attempt: Option<u32>,
        max_attempts: Option<u32>,
        wait: Option<Quantity>,
        error_type: Option<String>,
        status_code: Option<u16>,
    },
    StalledGeneration {
        message: Option<String>,
    },
    RepeatedStreamError {
        count: u32,
    },
    Timeout {
        message: Option<String>,
    },
    StepTimeout {
        message: Option<String>,
    },
    ExitExpression {
        pattern: String,
        scope: Option<String>,
    },
    RunawayRepetition {
        cycle_len: u32,
        repeats: u32,
    },
    RunawayVolume {
        lines: u64,
        bytes: u64,
    },
    UnsupportedProtocolVersion {
        version: String,
        supported: Vec<String>,
    },
    TurnLimitReached {
        limit: Option<u32>,
    },
    SessionTimeLimitReached {
        limit: Option<Quantity>,
    },
    Interrupted {
        message: Option<String>,
    },
    /// Bespoke cross-event rule; first producer is the Goose
    /// error-then-`complete` taint.
    SessionTainted {
        cause: String,
    },
    /// Reserved (no emitter until the durable-HITL/resume-round-2 work).
    HumanInputRequested {
        prompt: Option<String>,
    },
    /// Reserved (no emitter until the durable-HITL/resume-round-2 work).
    SessionResumable {
        session_id: Option<String>,
    },
}

impl SignalEvent {
    /// The [`SignalKind`] this payload carries.
    ///
    /// Exhaustive by design — adding a taxonomy member must force a
    /// compile error here.
    pub fn kind(&self) -> SignalKind {
        match self {
            Self::UsageCapApproaching { .. } => SignalKind::UsageCapApproaching,
            Self::UsageCapped { .. } => SignalKind::UsageCapped,
            Self::RateLimited { .. } => SignalKind::RateLimited,
            Self::ProviderOverloaded { .. } => SignalKind::ProviderOverloaded,
            Self::RetriesExhausted { .. } => SignalKind::RetriesExhausted,
            Self::NoFunds { .. } => SignalKind::NoFunds,
            Self::AuthInvalid { .. } => SignalKind::AuthInvalid,
            Self::AuthKindDetected { .. } => SignalKind::AuthKindDetected,
            Self::PermissionDeniedRead { .. } => SignalKind::PermissionDeniedRead,
            Self::PermissionDeniedWrite { .. } => SignalKind::PermissionDeniedWrite,
            Self::TokensConsumed { .. } => SignalKind::TokensConsumed,
            Self::ModelResolved { .. } => SignalKind::ModelResolved,
            Self::ModelFallback { .. } => SignalKind::ModelFallback,
            Self::ProviderVersion { .. } => SignalKind::ProviderVersion,
            Self::GenerationRetried { .. } => SignalKind::GenerationRetried,
            Self::StalledGeneration { .. } => SignalKind::StalledGeneration,
            Self::RepeatedStreamError { .. } => SignalKind::RepeatedStreamError,
            Self::Timeout { .. } => SignalKind::Timeout,
            Self::StepTimeout { .. } => SignalKind::StepTimeout,
            Self::ExitExpression { .. } => SignalKind::ExitExpression,
            Self::RunawayRepetition { .. } => SignalKind::RunawayRepetition,
            Self::RunawayVolume { .. } => SignalKind::RunawayVolume,
            Self::UnsupportedProtocolVersion { .. } => SignalKind::UnsupportedProtocolVersion,
            Self::TurnLimitReached { .. } => SignalKind::TurnLimitReached,
            Self::SessionTimeLimitReached { .. } => SignalKind::SessionTimeLimitReached,
            Self::Interrupted { .. } => SignalKind::Interrupted,
            Self::SessionTainted { .. } => SignalKind::SessionTainted,
            Self::HumanInputRequested { .. } => SignalKind::HumanInputRequested,
            Self::SessionResumable { .. } => SignalKind::SessionResumable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_matches_payload_variant() {
        let cases: Vec<(SignalEvent, SignalKind)> = vec![
            (
                SignalEvent::UsageCapped {
                    window: UsageWindow::FiveHour,
                    lifts_at: None,
                    remaining: None,
                    message: None,
                },
                SignalKind::UsageCapped,
            ),
            (
                SignalEvent::RateLimited {
                    status_code: Some(429),
                    reset_at: None,
                    retry_after: None,
                    message: None,
                },
                SignalKind::RateLimited,
            ),
            (
                SignalEvent::AuthKindDetected {
                    auth_kind: "oauth".to_string(),
                },
                SignalKind::AuthKindDetected,
            ),
            (
                SignalEvent::ProviderVersion {
                    version: "1.2.3".to_string(),
                },
                SignalKind::ProviderVersion,
            ),
            (
                SignalEvent::RunawayVolume {
                    lines: 50_000,
                    bytes: 33_554_432,
                },
                SignalKind::RunawayVolume,
            ),
            (
                SignalEvent::SessionTainted {
                    cause: "error-then-complete".to_string(),
                },
                SignalKind::SessionTainted,
            ),
            (
                SignalEvent::HumanInputRequested { prompt: None },
                SignalKind::HumanInputRequested,
            ),
            (
                SignalEvent::SessionResumable { session_id: None },
                SignalKind::SessionResumable,
            ),
        ];
        for (event, kind) in cases {
            assert_eq!(event.kind(), kind);
        }
    }

    #[test]
    fn signal_kind_member_list_is_frozen() {
        assert_eq!(SignalKind::VARIANTS.len(), 29);
        assert!(SignalKind::VARIANTS.contains(&"human_input_requested"));
        assert!(SignalKind::VARIANTS.contains(&"session_resumable"));
    }

    #[test]
    fn serde_names_are_snake_case() {
        assert_eq!(serde_json::to_string(&MatchOp::In).unwrap(), "\"in\"");
        assert_eq!(
            serde_json::to_string(&SignalKind::UsageCapped).unwrap(),
            "\"usage_capped\""
        );
        let event = SignalEvent::UsageCapped {
            window: UsageWindow::SevenDayOpus,
            lifts_at: None,
            remaining: None,
            message: None,
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["kind"], "usage_capped");
        assert_eq!(json["window"], "seven_day_opus");
    }

    #[test]
    fn static_str_names_are_snake_case() {
        assert_eq!(
            <&'static str>::from(SignalKind::UsageCapApproaching),
            "usage_cap_approaching"
        );
        assert_eq!(
            <&'static str>::from(SignalSource::StderrPromoted),
            "stderr_promoted"
        );
        assert_eq!(<&'static str>::from(MatchOp::In), "in");
        assert_eq!(
            <&'static str>::from(DetectionMode::Declarative),
            "declarative"
        );
        assert_eq!(<&'static str>::from(UsageWindow::FiveHour), "five_hour");
    }
}
