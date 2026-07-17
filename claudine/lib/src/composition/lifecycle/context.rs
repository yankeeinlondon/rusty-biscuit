//! Lifecycle execution context: the stack-only globals `err`, `timing`,
//! and `current`.
//!
//! These globals supplement the document state at event-time. They reach the
//! evaluator as Darkmatter **injected globals** (see [`InjectedGlobal`]) layered
//! over the current effective document state — claudine no longer carries a
//! bespoke expression lookup. [`lifecycle_injected_globals`] builds that layer;
//! the lifecycle executor hands it to Darkmatter's subtree compose (DM2) so
//! event-time interpolation reuses the same parsing/interpolation core as main
//! compose.
//!
//! `err`/`timing` are eager (already captured by the time the event fires);
//! `current` is lazy — its JSON snapshot is materialized only when a lifecycle
//! string references `current`, mirroring how `ctx` is captured lazily at
//! compose time.
//!
//! ## `err`
//!
//! Carries the active failure (if any) into `blocked`, `failure`, and the
//! optional-error `finalize` events. Exposed through the [`Diagnostic`](crate::diagnostics::Diagnostic) facets
//! `err.code` / `err.category` / `err.disposition` / `err.origin` /
//! `err.severity` / `err.detail.*` (when the source error is classifiable),
//! the one-level `err.cause.*` projection of the next registered diagnostic
//! below it, plus the deprecated aliases `err.kind` / `err.variant` /
//! `err.msg` — `err.kind` / `err.variant` are deprecated spellings of
//! `err.category` / `err.code`.
//!
//! Which error in a chain answers all of this is decided by
//! [`select_effective_diagnostic`] — the same walk the CLI renders through and
//! the snapshot serializes through, so a route cannot classify one cause while
//! rendering another. Parse-time validation (see
//! [`super::lifecycle::validate_no_err_in_no_error_events`]) rejects `err`
//! references in `initialize`, `start`, `success`, and `loop`.
//!
//! ## `timing`
//!
//! Carries observed durations. All fields are optional so the public shape
//! never commits to a value the runtime may not have captured.
//!
//! ## `current`
//!
//! Carries lazily-captured snapshots of the runtime `ctx` and `env`
//! namespaces at event execution time. Exposed as `current.ctx.<name>`
//! and `current.env.<name>`.

use std::collections::HashMap;
use std::error::Error as StdError;

use darkmatter::markdown::compose::subtree::InjectedGlobal;
use serde_json::{Map, Value};

use super::super::error::CompositionError;
use crate::diagnostics::{DiagnosticSnapshot, select_effective_diagnostic};
use crate::error::ClaudineError;
use crate::harness::concise_message;
use crate::harness::error::HarnessError;

/// Snapshot of the lifecycle-stack-only `err` global.
///
/// Constructed from whichever typed error the runtime surfaces — a
/// [`ClaudineError`], a [`HarnessError`], or a [`CompositionError`] — or from a
/// synthesized `error_kind` label. When the failure is classifiable it carries
/// the shared [`DiagnosticSnapshot`], the single owned projection of the
/// selected diagnostic (spec §D9); `kind`/`variant` retain the internal
/// source-type / enum-arm labels used only for the facet-less fallback.
///
/// Visible in lifecycle stack expressions as the deprecated legacy aliases
/// `err.kind` / `err.variant` / `err.msg` plus the [`Diagnostic`](crate::diagnostics::Diagnostic) facets
/// `err.code`, `err.category`, `err.disposition`, `err.origin`,
/// `err.severity`, and `err.detail.*` when the source error is classifiable.
/// `err.msg` is always present. `err.kind` / `err.variant` are the deprecated
/// *spellings* of `err.category` / `err.code` (spec success criteria): for a
/// classifiable error they mirror those facet values, and for a facet-less
/// action failure they fall back to the internal source-type / enum-arm labels.
/// When the snapshot is present the promoted handleability conveniences
/// (error-catalog §2.6) project too — the predicate sugar `err.is_transient` /
/// `err.is_throttled` / `err.is_correctable` and `err.reset_at` /
/// `err.retry_after_ms` lifted from `detail` — as terse aliases over the
/// canonical `disposition`/`detail.*`. A bare `err` resolves to the whole
/// object.
#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleErrorInfo {
    /// The source error type name (e.g. `"ClaudineError"`,
    /// `"HarnessError"`, `"CompositionError"`). Projected under the deprecated
    /// `err.kind` alias **only** for a facet-less action failure; a classifiable
    /// error projects its `category` there instead (see [`Self::to_value`]).
    pub kind: &'static str,

    /// The enum variant name (e.g. `"Io"`, `"ShellCommandDenied"`,
    /// `"LifecycleInvalid"`). Projected under the deprecated `err.variant` alias
    /// **only** for a facet-less action failure; a classifiable error projects
    /// its `code` there instead (see [`Self::to_value`]).
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
    /// is classifiable (spec §D9).
    ///
    /// Populated for every typed [`Diagnostic`](crate::diagnostics::Diagnostic) source ([`CompositionError`],
    /// [`ClaudineError`], [`HarnessError`]) via
    /// [`DiagnosticSnapshot::from_diagnostic`], and for a recognized
    /// provider/cap/timeout/runaway `error_kind` label via
    /// [`DiagnosticSnapshot::from_code`]. It carries the facets, the
    /// catalog-shaped `detail`, and the one-level `err.cause.*` projection in a
    /// single value, so `err.*`, terminal rendering, and machine output cannot
    /// disagree. `None` only for a generic lifecycle action verb (`shell`,
    /// `set_frontmatter`) that names no error_kind, in which case only the
    /// legacy `kind`/`variant`/`msg` aliases project. Boxed to keep
    /// `LifecycleErrorInfo` small — it is the `Err` type of several hot
    /// `Result`-returning lifecycle helpers.
    pub snapshot: Option<Box<DiagnosticSnapshot>>,
}

impl LifecycleErrorInfo {
    /// Build the snapshot from a [`ClaudineError`], capturing its [`Diagnostic`](crate::diagnostics::Diagnostic)
    /// facets so `err.code` / `err.detail.*` project alongside the legacy
    /// aliases. Covers the top-level Claudine and provider error surface
    /// (`provider.unavailable`, `io.*`, `config.*`, `usage.*`, …).
    pub fn from_claudine_error(err: &ClaudineError) -> Self {
        Self::from_selection("ClaudineError", variant_name_from_debug(err), err)
    }

    /// Build the snapshot from a [`HarnessError`], capturing its [`Diagnostic`](crate::diagnostics::Diagnostic)
    /// facets so `err.code` / `err.detail.*` project alongside the legacy
    /// aliases.
    pub fn from_harness_error(err: &HarnessError) -> Self {
        Self::from_selection("HarnessError", variant_name_from_debug(err), err)
    }

    /// Build the snapshot from a [`CompositionError`], capturing its
    /// [`Diagnostic`](crate::diagnostics::Diagnostic) facets so `err.code` / `err.detail.*` project alongside the
    /// legacy aliases.
    pub fn from_composition_error(err: &CompositionError) -> Self {
        Self::from_selection("CompositionError", variant_name_from_debug(err), err)
    }

    /// Build the snapshot from an arbitrary error, preferring the effective
    /// diagnostic and falling back to a facet-less action failure labeled
    /// `verb` when nothing in the chain is registered.
    ///
    /// The seam for a boundary that holds a typed error behind an opaque
    /// [`Report`](color_eyre::Report) or a `Box<dyn Error>`: passing the error
    /// keeps its facets, where `from_action_failure(verb, err.to_string())`
    /// would flatten them to prose and force `err.*` to disagree with what the
    /// same failure renders.
    pub fn from_error_or_action(verb: impl Into<String>, error: &(dyn StdError + 'static)) -> Self {
        let variant = verb.into();
        match Self::select(error) {
            Some(snapshot) => Self::from_snapshot("LifecycleAction", variant, snapshot),
            None => Self::from_action_failure(variant, error.to_string()),
        }
    }

    /// Build from the diagnostic [`select_effective_diagnostic`] chooses (via
    /// [`Self::select`]), falling back to `error`'s own prose when the chain has
    /// none.
    fn from_selection(
        kind: &'static str,
        variant: String,
        error: &(dyn StdError + 'static),
    ) -> Self {
        match Self::select(error) {
            Some(snapshot) => Self::from_snapshot(kind, variant, snapshot),
            None => Self {
                kind,
                variant,
                msg: concise_message(&error.to_string()),
                snapshot: None,
            },
        }
    }

    /// Wrap an already-projected [`DiagnosticSnapshot`], mirroring `err.msg`
    /// from the snapshot's own concise message so the two never diverge.
    fn from_snapshot(kind: &'static str, variant: String, snapshot: DiagnosticSnapshot) -> Self {
        Self {
            kind,
            variant,
            msg: snapshot.message.clone(),
            snapshot: Some(Box::new(snapshot)),
        }
    }

    /// Project the diagnostic [`select_effective_diagnostic`] chooses into the
    /// shared owned [`DiagnosticSnapshot`] — the one place `err.*` reads facets,
    /// so it resolves through the same selection the CLI renders and the machine
    /// surface serializes.
    fn select(error: &(dyn StdError + 'static)) -> Option<DiagnosticSnapshot> {
        let selected = select_effective_diagnostic(error)?.diagnostic()?;
        Some(DiagnosticSnapshot::from_diagnostic(selected))
    }

    /// Build a snapshot for a failed lifecycle stack action (a side-effect,
    /// shell, or expression-function action that errored at runtime).
    ///
    /// `kind` is the fixed string `"LifecycleAction"`, `variant` is the
    /// action verb (e.g. `"shell"`, `"set_frontmatter"`), and `msg` is the
    /// underlying typed error's message. This lets a subsequent `failure`
    /// event observe `err.kind`, `err.variant`, and `err.msg` after a
    /// setup-phase action error routes the run to `failure`.
    ///
    /// When `verb` is a recognized provider/cap/timeout/runaway `error_kind`
    /// label (the wrapper attributes a guard trip or provider failure honestly
    /// via [`crate::diagnostics::code_for_error_kind`]), the matching catalog
    /// facets are attached so `err.code` / `err.category` / `err.disposition` /
    /// `err.origin` project alongside the legacy aliases. A generic action verb
    /// (`shell`, `set_frontmatter`) is not an error_kind, so it stays
    /// facet-less.
    ///
    /// `msg` passes through [`concise_message`], the same hygiene
    /// [`failure_message`](crate::harness::failure_message) already applied on
    /// the provider-attempt path — so a hand-built message here cannot reach
    /// TTS or a webhook multi-line or escape-bearing. The pass is idempotent,
    /// which is what lets the provider cascade keep its ratified precedence
    /// (including its budget-reserved `(attempt N)` suffix) unchanged.
    pub fn from_action_failure(verb: impl Into<String>, msg: impl Into<String>) -> Self {
        let variant = verb.into();
        let msg = concise_message(&msg.into());
        // No typed error, so `from_code` seeds a catalog-shaped detail and no
        // cause. The label may name no code, leaving a facet-less fallback.
        let snapshot = crate::diagnostics::code_for_error_kind(&variant)
            .and_then(|code| DiagnosticSnapshot::from_code(code, msg.clone()))
            .map(Box::new);
        Self {
            kind: "LifecycleAction",
            variant,
            msg,
            snapshot,
        }
    }

    /// Render the snapshot as a JSON object for evaluation lookups.
    ///
    /// The legacy `kind`/`variant`/`msg` aliases are always present; the
    /// [`Diagnostic`](crate::diagnostics::Diagnostic) facets
    /// (`code`/`category`/`disposition`/`origin`/`severity`/`detail`) are added
    /// only when the source error was classifiable.
    ///
    /// `kind`/`variant` are the deprecated spellings of `category`/`code`: for a
    /// classifiable error they project the `category`/`code` facet values, and
    /// only a facet-less action failure falls back to the internal
    /// source-type/verb labels stored on the snapshot.
    ///
    /// When the snapshot is present, the promoted handleability conveniences
    /// from error-catalog §2.6 project alongside its facets: the predicate sugar
    /// `is_transient`/`is_throttled`/`is_correctable` (derived from
    /// `disposition`) and `reset_at`/`retry_after_ms` (lifted from
    /// `detail`, `null` when the active code carries no such field). These are
    /// sugar only — `disposition` and `detail.*` stay canonical. They derive
    /// from the snapshot, so they project **only** when it is present; a
    /// facet-less generic action verb keeps projecting only the legacy aliases.
    pub fn to_value(&self) -> Value {
        let mut obj = serde_json::Map::new();
        // Legacy aliases: when the error is classifiable, `err.kind` /
        // `err.variant` are the *deprecated spellings* of `err.category` /
        // `err.code` (spec success criteria), so they mirror those facet values
        // rather than the internal Rust source-type / enum-arm labels. A
        // facet-less action failure (a generic `shell`/`set_frontmatter` verb
        // that maps to no diagnostic code) has no faceted equivalent, so the
        // aliases fall back to the internal labels — these residual cases are
        // exactly the unclassifiable ones the faceted contract does not cover.
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

            // Promoted handleability conveniences (error-catalog §2.6): sugar
            // over the canonical `disposition`/`detail.*`, projected only when
            // the snapshot exists.
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
/// non-null entry under `field`; otherwise [`Value::Null`]. A non-object
/// `detail` (the `from_code` path leaves it `Value::Null`) yields `null`,
/// keeping the promoted field absent-but-readable per error-catalog §2.6.
fn promoted_detail_field(detail: &Value, field: &str) -> Value {
    match detail.get(field) {
        Some(v) if !v.is_null() => v.clone(),
        _ => Value::Null,
    }
}

/// Extract the variant name from a `Debug` rendering of an error.
///
/// `thiserror::Error` derive produces a `Debug` impl whose leading token
/// is the variant name (e.g. `Io(...)`, `ShellCommandDenied { ... }`,
/// `LifecycleInvalid { .. }`). We slice up to the first `(`, `{`, or
/// whitespace to recover that name.
fn variant_name_from_debug<T: std::fmt::Debug>(err: &T) -> String {
    let rendered = format!("{err:?}");
    let end = rendered
        .find(['(', ' ', '{'])
        .unwrap_or(rendered.len());
    rendered[..end].to_string()
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

    /// Wall-clock duration of the whole run so far (compose + provider
    /// invocation), in milliseconds. `None` when the runtime has not
    /// captured it.
    pub total_ms: Option<u64>,

    /// Wall-clock duration of the current sequence step, in milliseconds.
    /// `None` outside a sequence or when timing was not collected.
    pub step_ms: Option<u64>,
}

impl LifecycleTiming {
    /// Build a timing snapshot from wall-clock `Instant`s captured at run
    /// start.
    ///
    /// `document_start` is sampled when the current document began executing
    /// (one per `initialize`/run); `run_start` is sampled when the enclosing
    /// run began and includes preceding sequence steps / loop iterations when
    /// present. Both elapse against the same monotonic clock at event time.
    ///
    /// `document_ms` is always populated. `total_ms` is populated only when a
    /// run-level `run_start` is supplied (it defaults to `document_start` when
    /// the two coincide). `step_ms` is left `None`; the harness loop has no
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

    /// Render the snapshot as a JSON object for evaluation lookups.
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

/// Snapshot of the lifecycle-stack-only `current` global.
///
/// Carries lazily-captured snapshots of the `ctx` and `env` namespaces at
/// event execution time. Exposed as `current.ctx.<name>` and
/// `current.env.<name>`.
#[derive(Debug, Clone, Default)]
pub struct LifecycleCurrent {
    /// Captured `ctx.*` namespace (agent, model, repo, today, …). Defaults
    /// to an empty object.
    pub ctx: Value,

    /// Captured `env.*` namespace (process environment snapshot). Defaults
    /// to an empty object.
    pub env: Value,
}

impl LifecycleCurrent {
    /// Capture the process environment into a JSON object snapshot.
    ///
    /// Reads `std::env::vars()` **at call time**, so a side effect or external
    /// change between `prepare` and a later lifecycle event is observable via
    /// `current.env.<NAME>`. This is the late-binding behavior the spec
    /// promises: unlike `doc`/`ctx`/`env` which are computed once at
    /// composition start, `current.env` reflects the environment as it stands
    /// when the event fires.
    pub fn capture_env() -> Value {
        let mut env = Map::new();
        for (key, value) in std::env::vars() {
            env.insert(key, Value::String(value));
        }
        Value::Object(env)
    }

    /// Build a `current` snapshot with the `env` namespace captured at event
    /// time and an empty `ctx` namespace.
    ///
    /// Used at sites where the runtime `ctx.*` namespace is not readily
    /// reconstructable but the late-bound environment snapshot must still be
    /// exposed.
    pub fn capture_env_only() -> Self {
        Self {
            ctx: Value::Object(Map::new()),
            env: Self::capture_env(),
        }
    }

    /// Build a `current` snapshot with both namespaces captured at event time.
    ///
    /// `env` is the live process environment (see [`Self::capture_env`]).
    /// `ctx` is Darkmatter's full `ctx.*` namespace captured against
    /// `base_dir`, keyed by the bare context name (e.g. `agent`, `model`,
    /// `repo`, `today`) so `current.ctx.<name>` resolves. Agent/model derive
    /// from the `AGENT`/`MODEL` environment variables, so they too reflect
    /// event-time state.
    pub fn capture_at_event(base_dir: &std::path::Path) -> Self {
        let ctx = darkmatter::markdown::compose::ComposeContext::capture_for_dir(base_dir);
        Self {
            ctx: Value::Object(ctx.values().clone()),
            env: Self::capture_env(),
        }
    }

    /// Render the snapshot as a JSON object for evaluation lookups.
    pub fn to_value(&self) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert("ctx".to_string(), self.ctx.clone());
        obj.insert("env".to_string(), self.env.clone());
        Value::Object(obj)
    }
}

/// Build the event-time injected-globals layer handed to Darkmatter's subtree
/// compose (DM2).
///
/// The returned map layers the lifecycle stack-only globals over the current
/// effective document state: `err`/`timing` are eager (their snapshots are
/// already captured when the event fires); `current` is lazy — its JSON
/// snapshot is materialized only when a lifecycle string references `current`,
/// mirroring how `ctx` is captured lazily at compose time.
///
/// An unattached global is simply absent from the map, so a bare
/// `err`/`timing`/`current` reference falls through to the document state (a
/// literal frontmatter property of that name stays reachable). `doc.err`
/// reaches a literal `err` property because the `doc` root is never an injected
/// global.
pub fn lifecycle_injected_globals(
    err: Option<&LifecycleErrorInfo>,
    timing: Option<&LifecycleTiming>,
    current: Option<&LifecycleCurrent>,
) -> HashMap<String, InjectedGlobal> {
    let mut globals = HashMap::new();
    if let Some(err) = err {
        globals.insert("err".to_string(), InjectedGlobal::eager(err.to_value()));
    }
    if let Some(timing) = timing {
        globals.insert(
            "timing".to_string(),
            InjectedGlobal::eager(timing.to_value()),
        );
    }
    if let Some(current) = current {
        // Lazy: clone the captured snapshot into the closure so it materializes
        // its JSON form only if a lifecycle string references `current`.
        let owned = current.clone();
        globals.insert(
            "current".to_string(),
            InjectedGlobal::lazy(move || owned.to_value()),
        );
    }
    globals
}

#[cfg(test)]
mod tests;
