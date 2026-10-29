//! The values of the lifecycle globals `err` and `timing`.
//!
//! These globals supplement the document state at event time. Which scopes
//! declare them is the host's binding policy; this module owns their values.
//! Both are eager: they are already captured by the time the event fires.
//!
//! ## `err`
//!
//! Carries the active failure (if any) into the error-carrying events. A
//! classifiable failure exposes the facets of its [`DiagnosticSnapshot`] —
//! `err.code` / `err.category` / `err.disposition` / `err.origin` /
//! `err.severity` / `err.detail.*` and the one-level `err.cause.*` — plus the
//! deprecated aliases `err.kind` / `err.variant` / `err.msg`, where
//! `err.kind` / `err.variant` are deprecated spellings of `err.category` /
//! `err.code`. Which error in a chain answers all of this is the host's
//! [`DiagnosticCatalog`], so `err.*`, terminal rendering, and machine output
//! read one value.
//!
//! ## `timing`
//!
//! Carries observed durations. All fields are optional so the public shape
//! never commits to a value the runtime may not have captured.

use std::error::Error as StdError;
use std::sync::Arc;

use serde_json::Value;

use super::envelope::{DiagnosticCatalog, DiagnosticSnapshot, concise_message};
use super::error::LifecycleExprError;
use crate::markdown::MarkdownError;
use crate::markdown::compose::expression::ExpressionError;

/// Why an event-time lifecycle evaluation failed.
///
/// Typed so a renderer chooses a remediation without matching message text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum LifecycleEvaluationReason {
    /// An expression raised: a malformed span, an unknown function, an
    /// unavailable global, or a function error.
    #[default]
    Expression,
    /// A `break` fired outside any loop.
    BreakWithoutLoop,
}

/// Snapshot of the lifecycle-stack-only `err` global.
///
/// When the failure is classifiable it carries the shared
/// [`DiagnosticSnapshot`], the single owned projection of the selected
/// diagnostic; `kind`/`variant` retain the internal source-type / enum-arm
/// labels used only for the facet-less fallback.
///
/// Visible in lifecycle stack expressions as the deprecated legacy aliases
/// `err.kind` / `err.variant` / `err.msg` plus the diagnostic facets
/// `err.code`, `err.category`, `err.disposition`, `err.origin`,
/// `err.severity`, and `err.detail.*` when the source error is classifiable.
/// `err.msg` is always present. `err.kind` / `err.variant` are the deprecated
/// *spellings* of `err.category` / `err.code`: for a classifiable error they
/// mirror those facet values, and for a facet-less action failure they fall
/// back to the internal source-type / enum-arm labels. When the snapshot is
/// present the promoted handleability conveniences project too — the predicate
/// sugar `err.is_transient` / `err.is_throttled` / `err.is_correctable` and
/// `err.reset_at` / `err.retry_after_ms` lifted from `detail` — as terse
/// aliases over the canonical `disposition`/`detail.*`. A bare `err` resolves
/// to the whole object.
#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleErrorInfo {
    /// The source error type name (e.g. `"LifecycleAction"`, or a host error
    /// type). Projected under the deprecated `err.kind` alias **only** for a
    /// facet-less failure; a classifiable error projects its `category` there
    /// instead (see [`Self::to_value`]).
    pub kind: &'static str,

    /// The enum variant name or action verb (e.g. `"Io"`, `"shell"`).
    /// Projected under the deprecated `err.variant` alias **only** for a
    /// facet-less failure; a classifiable error projects its `code` there
    /// instead (see [`Self::to_value`]).
    pub variant: String,

    /// The concise, notification-safe message of the *selected* diagnostic —
    /// single-line, escape-free, and length-clamped, because this string feeds
    /// TTS, outbound messaging routes, and desktop notifications verbatim.
    ///
    /// It is the effective diagnostic's `Display`, not the wrapper's and not
    /// its multi-line rendered block. Equal to `snapshot.message` when the
    /// snapshot is present, so `err.msg` reads one value regardless.
    pub msg: String,

    /// The shared owned projection of the selected diagnostic, when the source
    /// is classifiable.
    ///
    /// It carries the facets, the catalog-shaped `detail`, and the one-level
    /// `err.cause.*` projection in a single value, so `err.*`, terminal
    /// rendering, and machine output cannot disagree. `None` only for a
    /// failure the catalog does not classify (a generic action verb such as
    /// `shell` or `set_frontmatter`), in which case only the legacy
    /// `kind`/`variant`/`msg` aliases project. Boxed to keep
    /// `LifecycleErrorInfo` small — it is the `Err` type of several hot
    /// `Result`-returning lifecycle helpers.
    pub snapshot: Option<Box<DiagnosticSnapshot>>,

    /// Dotted lifecycle property that raised (`success.say`,
    /// `start.stack[0].when`), attached with [`Self::at_property`]. Not
    /// projected into `err.*`.
    pub property: Option<String>,

    /// Why an event-time evaluation failed, recovered by downcasting the
    /// reported error itself to [`LifecycleExprError`] (the source chain is not
    /// walked), or set by an executor that refuses a directive it cannot run
    /// yet. Not projected into `err.*`.
    pub reason: LifecycleEvaluationReason,

    /// The typed error an event-time or sequence task-value evaluation raised,
    /// kept so a library caller can inspect the original Darkmatter failure.
    /// Not projected into `err.*`; `None` for every other failure.
    pub cause: Option<LifecycleCause>,
}

/// The typed error behind a lifecycle evaluation failure, shared by every copy
/// of the [`LifecycleErrorInfo`] and host error that report it.
///
/// Its [`Error::source`](StdError::source) is the Darkmatter error itself
/// ([`ExpressionError`] or [`MarkdownError`]), so a caller can downcast it from
/// any error chain that carries this cause.
#[derive(Debug, Clone)]
pub struct LifecycleCause(Arc<LifecycleExprError>);

impl LifecycleCause {
    /// Share `error` as a cause.
    pub fn new(error: LifecycleExprError) -> Self {
        Self(Arc::new(error))
    }

    /// The shared evaluation error.
    pub fn error(&self) -> &LifecycleExprError {
        &self.0
    }

    /// The Darkmatter expression error, wherever the evaluation raised it: a
    /// direct evaluation, or the cause of an interpolation failure.
    pub fn expression_error(&self) -> Option<&ExpressionError> {
        match self.0.as_ref() {
            LifecycleExprError::Evaluate(error) => Some(error),
            LifecycleExprError::Compose(error) => match error.as_ref() {
                MarkdownError::Interpolation { cause, .. } => Some(cause),
                _ => None,
            },
            LifecycleExprError::Prose(_) | LifecycleExprError::Context(_) => None,
        }
    }
}

/// Two causes are equal when they render the same failure: equality of a
/// [`LifecycleErrorInfo`] compares what it reports, not error identity.
impl PartialEq for LifecycleCause {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.to_string() == other.0.to_string()
    }
}

impl std::fmt::Display for LifecycleCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl StdError for LifecycleCause {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self.0.as_ref() {
            LifecycleExprError::Evaluate(error) => Some(error.as_ref()),
            LifecycleExprError::Compose(error) => Some(error.as_ref()),
            LifecycleExprError::Context(error) => Some(error),
            LifecycleExprError::Prose(_) => None,
        }
    }
}

impl LifecycleErrorInfo {
    /// Build from the diagnostic `catalog` selects for `error`'s chain,
    /// labeled `kind`/`variant`, falling back to `error`'s own concise prose
    /// when the chain has none.
    pub fn from_selection_with(
        catalog: &dyn DiagnosticCatalog,
        kind: &'static str,
        variant: String,
        error: &(dyn StdError + 'static),
    ) -> Self {
        match catalog.select(error) {
            Some(snapshot) => Self::from_snapshot(kind, variant, snapshot),
            None => Self {
                kind,
                variant,
                msg: concise_message(&error.to_string()),
                snapshot: None,
                property: None,
                reason: LifecycleEvaluationReason::Expression,
                cause: None,
            },
        }
    }

    /// Build from an arbitrary error, preferring the diagnostic `catalog`
    /// selects and falling back to a facet-less action failure labeled `verb`
    /// when nothing in the chain is registered.
    ///
    /// Passing the error keeps its facets, where
    /// `from_action_failure_with(catalog, verb, err.to_string())` would flatten
    /// them to prose and force `err.*` to disagree with what the same failure
    /// renders.
    pub fn from_error_or_action_with(
        catalog: &dyn DiagnosticCatalog,
        verb: impl Into<String>,
        error: &(dyn StdError + 'static),
    ) -> Self {
        let variant = verb.into();
        match catalog.select(error) {
            Some(snapshot) => Self::from_snapshot("LifecycleAction", variant, snapshot),
            None => Self::from_action_failure_with(catalog, variant, error.to_string()),
        }
    }

    /// [`Self::from_error_or_action_with`] for an event-time evaluation
    /// failure, keeping the typed error as [`Self::cause`].
    pub fn from_expr_error_with(
        catalog: &dyn DiagnosticCatalog,
        verb: impl Into<String>,
        error: LifecycleExprError,
    ) -> Self {
        let mut info = Self::from_error_or_action_with(catalog, verb, &error);
        info.cause = Some(LifecycleCause::new(error));
        info
    }

    /// Build the snapshot for a failed lifecycle stack action (a side-effect,
    /// shell, or expression-function action that errored at runtime).
    ///
    /// `kind` is the fixed string `"LifecycleAction"`, `variant` is the action
    /// verb (e.g. `"shell"`, `"set_frontmatter"`), and `msg` is the underlying
    /// error's message. This lets a subsequent `failure` event observe
    /// `err.kind`, `err.variant`, and `err.msg` after a setup-phase action
    /// error routes the run to `failure`.
    ///
    /// When `catalog` recognizes `verb` as a failure label
    /// ([`DiagnosticCatalog::snapshot_for_error_kind`]), the matching facets are
    /// attached so `err.code` / `err.category` / `err.disposition` /
    /// `err.origin` project alongside the legacy aliases. A generic action verb
    /// stays facet-less.
    ///
    /// `msg` passes through [`concise_message`], so a hand-built message here
    /// cannot reach TTS or a webhook multi-line or escape-bearing. The pass is
    /// idempotent, so text that already went through it is unchanged.
    pub fn from_action_failure_with(
        catalog: &dyn DiagnosticCatalog,
        verb: impl Into<String>,
        msg: impl Into<String>,
    ) -> Self {
        let variant = verb.into();
        let msg = concise_message(&msg.into());
        let snapshot = catalog.snapshot_for_error_kind(&variant, msg.clone()).map(Box::new);
        Self {
            kind: "LifecycleAction",
            variant,
            msg,
            snapshot,
            property: None,
            reason: LifecycleEvaluationReason::Expression,
            cause: None,
        }
    }

    /// Wrap an already-projected [`DiagnosticSnapshot`], mirroring `err.msg`
    /// from the snapshot's own concise message so the two never diverge.
    pub fn from_snapshot(kind: &'static str, variant: String, snapshot: DiagnosticSnapshot) -> Self {
        Self {
            kind,
            variant,
            msg: snapshot.message.clone(),
            snapshot: Some(Box::new(snapshot)),
            property: None,
            reason: LifecycleEvaluationReason::Expression,
            cause: None,
        }
    }

    /// Attach the lifecycle property that raised, unless a more specific one
    /// was already recorded.
    pub fn at_property(mut self, property: impl Into<String>) -> Self {
        if self.property.is_none() {
            self.property = Some(property.into());
        }
        self
    }

    /// Render the snapshot as a JSON object for evaluation lookups.
    ///
    /// The legacy `kind`/`variant`/`msg` aliases are always present; the
    /// diagnostic facets (`code`/`category`/`disposition`/`origin`/`severity`/
    /// `detail`) are added only when the source error was classifiable.
    ///
    /// `kind`/`variant` are the deprecated spellings of `category`/`code`: for
    /// a classifiable error they project the `category`/`code` facet values,
    /// and only a facet-less action failure falls back to the internal
    /// source-type/verb labels stored on the snapshot.
    ///
    /// When the snapshot is present, the promoted handleability conveniences
    /// project alongside its facets: the predicate sugar
    /// `is_transient`/`is_throttled`/`is_correctable` (derived from
    /// `disposition`) and `reset_at`/`retry_after_ms` (lifted from `detail`,
    /// `null` when the active code carries no such field). These are sugar
    /// only — `disposition` and `detail.*` stay canonical — so they project
    /// **only** when the snapshot is present.
    pub fn to_value(&self) -> Value {
        let mut obj = serde_json::Map::new();
        // A facet-less action failure has no faceted equivalent, so its
        // aliases fall back to the internal labels.
        match &self.snapshot {
            Some(snapshot) => {
                obj.insert("kind".to_string(), Value::from(snapshot.category.clone()));
                obj.insert("variant".to_string(), Value::from(snapshot.code.clone()));
            }
            None => {
                obj.insert("kind".to_string(), Value::from(self.kind));
                obj.insert("variant".to_string(), Value::from(self.variant.clone()));
            }
        }
        obj.insert("msg".to_string(), Value::from(self.msg.clone()));
        if let Some(snapshot) = &self.snapshot {
            obj.insert("code".to_string(), Value::from(snapshot.code.clone()));
            obj.insert("category".to_string(), Value::from(snapshot.category.clone()));
            obj.insert("disposition".to_string(), Value::from(snapshot.disposition.clone()));
            obj.insert("origin".to_string(), Value::from(snapshot.origin.clone()));
            obj.insert("severity".to_string(), Value::from(snapshot.severity.clone()));
            obj.insert("detail".to_string(), snapshot.detail.clone());

            obj.insert(
                "is_transient".to_string(),
                Value::from(snapshot.disposition == "transient"),
            );
            obj.insert(
                "is_throttled".to_string(),
                Value::from(snapshot.disposition == "throttled"),
            );
            obj.insert(
                "is_correctable".to_string(),
                Value::from(snapshot.disposition == "correctable"),
            );
            obj.insert("reset_at".to_string(), promoted_detail_field(&snapshot.detail, "reset_at"));
            obj.insert(
                "retry_after_ms".to_string(),
                promoted_detail_field(&snapshot.detail, "retry_after_ms"),
            );

            // Present-but-null when the selected diagnostic has no registered
            // cause, so `err.cause.code` reads as unknown rather than as a
            // lookup on a missing key.
            obj.insert(
                "cause".to_string(),
                match &snapshot.cause {
                    Some(cause) => serde_json::to_value(cause).unwrap_or(Value::Null),
                    None => Value::Null,
                },
            );
        }
        Value::Object(obj)
    }
}

/// Lift a promoted convenience field out of a `detail` payload.
///
/// Returns the field's value when `detail` is an object carrying a present,
/// non-null entry under `field`; otherwise [`Value::Null`], keeping the
/// promoted field absent-but-readable for a code that has no such field.
fn promoted_detail_field(detail: &Value, field: &str) -> Value {
    match detail.get(field) {
        Some(v) if !v.is_null() => v.clone(),
        _ => Value::Null,
    }
}

/// Snapshot of the lifecycle-stack-only `timing` global.
///
/// Every field is optional so the runtime never has to synthesize a value
/// it did not actually measure. The field names match the dotted paths
/// lifecycle expressions use (`timing.document_ms`,
/// `timing.total_ms`, `timing.step_ms`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LifecycleTiming {
    /// Wall-clock duration of the current document's composition, in
    /// milliseconds. `None` when composition timing was not collected.
    pub document_ms: Option<u64>,

    /// Wall-clock duration of the whole run so far, in milliseconds. `None`
    /// when the runtime has not captured it.
    pub total_ms: Option<u64>,

    /// Wall-clock duration of the current sequence step, in milliseconds.
    /// `None` outside a sequence or when timing was not collected.
    pub step_ms: Option<u64>,
}

impl LifecycleTiming {
    /// Build a timing snapshot from wall-clock `Instant`s.
    ///
    /// `document_start` is sampled when the current document began executing;
    /// `run_start` is sampled when the enclosing run began and includes
    /// preceding sequence steps or loop iterations when present. Both elapse
    /// against the same monotonic clock at `at`.
    ///
    /// `document_ms` is always populated. `total_ms` is populated only when a
    /// `run_start` is supplied. `step_ms` is left `None`: there is no
    /// sequence-step clock to measure here, so the field is honestly omitted
    /// rather than synthesized.
    pub fn from_instants(
        document_start: std::time::Instant,
        run_start: Option<std::time::Instant>,
        at: std::time::Instant,
    ) -> Self {
        let document_ms = at.saturating_duration_since(document_start).as_millis() as u64;
        let total_ms = run_start.map(|s| at.saturating_duration_since(s).as_millis() as u64);
        Self {
            document_ms: Some(document_ms),
            total_ms,
            step_ms: None,
        }
    }

    /// Render the snapshot as a JSON object for evaluation lookups; an
    /// unmeasured field is absent rather than `null`.
    pub fn to_value(&self) -> Value {
        let mut obj = serde_json::Map::new();
        if let Some(ms) = self.document_ms {
            obj.insert("document_ms".to_string(), Value::from(ms));
        }
        if let Some(ms) = self.total_ms {
            obj.insert("total_ms".to_string(), Value::from(ms));
        }
        if let Some(ms) = self.step_ms {
            obj.insert("step_ms".to_string(), Value::from(ms));
        }
        Value::Object(obj)
    }
}

#[cfg(test)]
mod tests;
