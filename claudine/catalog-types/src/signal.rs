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

use std::borrow::Cow;

use chrono::{DateTime, Utc};
use serde::{Serialize, Serializer};
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
    /// Bespoke-only (no detection records); emitted by the wrapper's
    /// model-catalog comparison via `SignalHub::emit_bespoke`.
    ModelCatalogDrift,
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
    /// Wrapper-computed observations (set differences, catalog
    /// comparisons) not tied to any child output channel.
    Wrapper,
}

/// The five ratified detection-record match operators (design doc "Record
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
    /// The addressed field is present and non-null; carries no match value.
    Exists,
}

/// How a taxonomy signal is detected.
///
/// - `Declarative` — a single-payload generic matcher (`match_path` + `op`)
///   compiled into the detection engine.
/// - `Bespoke` — a hand-written behavior-half detector: anything needing
///   cross-record or temporal state. Every bespoke record MUST have a
///   runtime emitter (and a `signals check` replayer proving it fires on its
///   evidence).
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

/// The model scope a cap applies to.
///
/// `All` is account-wide; `Specific` names a single model tier via a
/// provider-neutral token (reconcilable against the model catalog at report
/// time), so a new vendor tier needs no code change. The token is a
/// `Cow<'static, str>` so the SAME type serves both the runtime cap event
/// (an owned token extracted from a payload) and the static Layer-A
/// [`CapPolicy`] catalog (a borrowed `&'static str`). Serializes to a bare
/// string — `"all"` or the model token — for clean provenance in the JSONL
/// logs and reports.
#[derive(Debug, Clone, PartialEq)]
pub enum CapScope {
    All,
    Specific(Cow<'static, str>),
}

impl CapScope {
    /// Const constructor for a model-scoped cap in static catalog data.
    pub const fn specific(model: &'static str) -> Self {
        CapScope::Specific(Cow::Borrowed(model))
    }
}

impl Serialize for CapScope {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            CapScope::All => serializer.serialize_str("all"),
            CapScope::Specific(model) => serializer.serialize_str(model.as_ref()),
        }
    }
}

/// One cap policy a provider imposes: a `(model-scope, timeframe)` pair — the
/// provider-neutral identity of a rate window (design: Layer A cap-policy
/// catalog, `more-struture`). The relationship is 1:M — a single provider
/// imposes many cap policies (Claude Max: `(All, 5h)`, `(All, 7d)`,
/// `(Opus, 7d)`, `(Sonnet, 7d)`).
///
/// Static catalog data generated by `claudine-gen`, so `model`'s token
/// borrows; `timeframe` is a [`Quantity`] in [`Unit::DurationSecs`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CapPolicy {
    pub model: CapScope,
    pub timeframe: Quantity,
}

/// How a model-catalog drift was observed: a full dynamic model listing
/// (set comparison against the baseline) or a single resolved model that
/// the baseline does not contain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DriftObservation {
    Listing,
    ResolvedModel,
}

/// Taxonomy-typed signal payload — one variant per [`SignalKind`] member.
///
/// All timestamps are `DateTime<Utc>`: normalization to UTC is the
/// detection engine's job; the type states the contract.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SignalEvent {
    UsageCapApproaching {
        model: CapScope,
        timeframe: Option<Quantity>,
        remaining: Option<Quantity>,
        resets_at: Option<DateTime<Utc>>,
        message: Option<String>,
    },
    UsageCapped {
        model: CapScope,
        timeframe: Option<Quantity>,
        remaining: Option<Quantity>,
        lifts_at: Option<DateTime<Utc>>,
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
        /// Provider-supplied URL for adding credits, when present — a
        /// lifecycle rule can point the user straight at it.
        top_up_url: Option<String>,
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
        /// Tokens served from the provider's prompt cache.
        cache_read: Option<Quantity>,
        /// Tokens written to the provider's prompt cache this turn.
        cache_write: Option<Quantity>,
        /// Billed cost for the turn, when the provider reports it.
        cost: Option<Quantity>,
        /// Reasoning/thinking tokens, when reported separately from output.
        reasoning: Option<Quantity>,
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
    /// The provider's live model surface disagrees with the generated
    /// expected-offerings baseline. Bespoke-only: emitted by the wrapper's
    /// catalog comparison, never by detection records.
    ModelCatalogDrift {
        /// Model ids observed from the provider but absent from the
        /// generated expected-offerings baseline.
        unexpected: Vec<String>,
        /// Expected ids the provider's dynamic listing no longer returns;
        /// empty when the observation is a single resolved model.
        missing: Vec<String>,
        observed_via: DriftObservation,
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
        message: Option<String>,
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
        message: Option<String>,
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
            Self::ModelCatalogDrift { .. } => SignalKind::ModelCatalogDrift,
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

/// Qwen's provider-native loop-detection vocabulary — the `LoopType` token
/// embedded in a `runaway_repetition` terminal error `result`.
///
/// Canonical source for the tokens the qwen bespoke guard scans for; the
/// research record `stream-runaway_repetition-result-loop` mirrors this list
/// as its `vocabulary:` (guarded by a mirror test). The token wire form is
/// SCREAMING_SNAKE (e.g. `CONSECUTIVE_IDENTICAL_TOOL_CALLS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum QwenLoopType {
    ConsecutiveIdenticalToolCalls,
    ChantingIdenticalSentences,
    RepetitiveThoughts,
    ReadFileLoop,
    ActionStagnation,
    ShellCommandStagnation,
    GlobalToolCallDuplicate,
    AlternatingToolCallPattern,
    TurnToolCallCap,
    InvalidToolParamsStagnation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qwen_loop_type_tokens_are_screaming_snake() {
        assert_eq!(
            QwenLoopType::VARIANTS,
            &[
                "CONSECUTIVE_IDENTICAL_TOOL_CALLS",
                "CHANTING_IDENTICAL_SENTENCES",
                "REPETITIVE_THOUGHTS",
                "READ_FILE_LOOP",
                "ACTION_STAGNATION",
                "SHELL_COMMAND_STAGNATION",
                "GLOBAL_TOOL_CALL_DUPLICATE",
                "ALTERNATING_TOOL_CALL_PATTERN",
                "TURN_TOOL_CALL_CAP",
                "INVALID_TOOL_PARAMS_STAGNATION",
            ]
        );
    }

    #[test]
    fn kind_matches_payload_variant() {
        let cases: Vec<(SignalEvent, SignalKind)> = vec![
            (
                SignalEvent::UsageCapped {
                    model: CapScope::All,
                    timeframe: None,
                    remaining: None,
                    lifts_at: None,
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
        assert_eq!(SignalKind::VARIANTS.len(), 30);
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
            model: CapScope::Specific("opus".into()),
            timeframe: None,
            remaining: None,
            lifts_at: None,
            message: None,
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["kind"], "usage_capped");
        // CapScope serializes to a bare model token.
        assert_eq!(json["model"], "opus");
    }

    #[test]
    fn cap_scope_serializes_to_a_bare_string() {
        assert_eq!(serde_json::to_value(CapScope::All).unwrap(), "all");
        assert_eq!(
            serde_json::to_value(CapScope::Specific("sonnet".into())).unwrap(),
            "sonnet"
        );
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
    }
}
