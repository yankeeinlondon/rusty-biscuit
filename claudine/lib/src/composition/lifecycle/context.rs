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
//! optional-error `finalize` events. Exposed through the [`Diagnostic`] facets
//! `err.code` / `err.category` / `err.disposition` / `err.origin` /
//! `err.severity` / `err.detail.*` (when the source error is classifiable),
//! plus the deprecated aliases `err.kind` / `err.variant` / `err.msg` —
//! `err.kind` / `err.variant` are deprecated spellings of `err.category` /
//! `err.code`. Parse-time validation (see
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

use darkmatter::markdown::compose::subtree::InjectedGlobal;
use serde_json::{Map, Value};

use super::super::error::CompositionError;
use crate::diagnostics::Diagnostic;
use crate::error::ClaudineError;
use crate::harness::error::HarnessError;

/// Snapshot of the lifecycle-stack-only `err` global.
///
/// Constructed from whichever typed error the runtime surfaces — a
/// [`ClaudineError`], a [`HarnessError`], or a [`CompositionError`]. The
/// `kind` field names the source error type, `variant` names the enum arm,
/// and `msg` carries the human-readable message.
///
/// Visible in lifecycle stack expressions as the deprecated legacy aliases
/// `err.kind` / `err.variant` / `err.msg` plus the [`Diagnostic`] facets
/// `err.code`, `err.category`, `err.disposition`, `err.origin`,
/// `err.severity`, and `err.detail.*` when the source error is classifiable.
/// `err.msg` is always present. `err.kind` / `err.variant` are the deprecated
/// *spellings* of `err.category` / `err.code` (spec success criteria): for a
/// classifiable error they mirror those facet values, and for a facet-less
/// action failure they fall back to the internal source-type / enum-arm labels.
/// When facets are present the promoted handleability conveniences
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

    /// The human-readable error message (the `Display` rendering of the
    /// source error).
    pub msg: String,

    /// The classification facets, when the source is classifiable.
    ///
    /// Populated for every typed [`Diagnostic`] source ([`CompositionError`],
    /// [`ClaudineError`], [`HarnessError`]) and for a recognized
    /// provider/cap/timeout/runaway `error_kind` label routed through
    /// [`Self::from_action_failure`]. `None` only for a generic lifecycle
    /// action verb (`shell`, `set_frontmatter`) that names no error_kind, in
    /// which case only the legacy `kind`/`variant`/`msg` aliases project. Boxed
    /// to keep `LifecycleErrorInfo` small — it is the `Err` type of several hot
    /// `Result`-returning lifecycle helpers.
    pub facets: Option<Box<DiagnosticFacets>>,
}

/// The [`Diagnostic`] facets captured from a typed error, projected into the
/// lifecycle `err` global as `err.code` / `err.category` / `err.disposition` /
/// `err.origin` / `err.severity` / `err.detail.*`.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticFacets {
    /// Stable dotted code (`composition.invalid_file_reference`).
    pub code: &'static str,
    /// Coarse domain slug (`composition`).
    pub category: &'static str,
    /// Generic-strategy slug (`correctable`).
    pub disposition: &'static str,
    /// Who must remediate (`author`).
    pub origin: &'static str,
    /// Operator-facing severity slug (`info`/`warning`/`error`), defaulted from
    /// the disposition and overridable per code (error-catalog §1).
    pub severity: &'static str,
    /// Typed per-instance payload, projected to `err.detail.*`.
    pub detail: Value,
}

impl DiagnosticFacets {
    /// Capture the facets of any [`Diagnostic`]-implementing error.
    pub fn from_diagnostic<D: Diagnostic + ?Sized>(err: &D) -> Self {
        Self {
            code: err.code(),
            category: err.category().as_str(),
            disposition: err.disposition().as_str(),
            origin: err.origin().as_str(),
            // Disambiguate from the `BlockError::severity` supertrait method
            // (`Diagnostic: BlockError`) — we want the projected facet severity.
            severity: Diagnostic::severity(err).as_str(),
            detail: err.detail(),
        }
    }

    /// Build facets from a locked catalog `code`, deriving
    /// `category`/`disposition`/`origin` from its [`crate::diagnostics::CodeSpec`].
    ///
    /// Used for the provider/cap/timeout/runaway failures that reach the
    /// lifecycle `err` global as a synthesized `error_kind` string rather than
    /// a typed [`Diagnostic`] error (see
    /// [`crate::diagnostics::code_for_error_kind`]). The structured `detail`
    /// payload is not reconstructable from the label alone, so it projects as
    /// `null`; `code`/`category`/`disposition`/`origin` are the handleable
    /// surface here. Returns `None` for a code absent from the catalog.
    pub fn from_code(code: &'static str) -> Option<Self> {
        let spec = crate::diagnostics::code_spec(code)?;
        Some(Self {
            code: spec.code,
            category: spec.category.as_str(),
            disposition: spec.disposition.as_str(),
            origin: spec.origin.as_str(),
            severity: spec.severity().as_str(),
            detail: Value::Null,
        })
    }
}

impl LifecycleErrorInfo {
    /// Build the snapshot from a [`ClaudineError`], capturing its [`Diagnostic`]
    /// facets so `err.code` / `err.detail.*` project alongside the legacy
    /// aliases. Covers the top-level Claudine and provider error surface
    /// (`provider.unavailable`, `io.*`, `config.*`, `usage.*`, …).
    pub fn from_claudine_error(err: &ClaudineError) -> Self {
        Self {
            kind: "ClaudineError",
            variant: variant_name_from_debug(err),
            msg: err.to_string(),
            facets: Some(Box::new(DiagnosticFacets::from_diagnostic(err))),
        }
    }

    /// Build the snapshot from a [`HarnessError`], capturing its [`Diagnostic`]
    /// facets so `err.code` / `err.detail.*` project alongside the legacy
    /// aliases.
    pub fn from_harness_error(err: &HarnessError) -> Self {
        Self {
            kind: "HarnessError",
            variant: variant_name_from_debug(err),
            msg: err.to_string(),
            facets: Some(Box::new(DiagnosticFacets::from_diagnostic(err))),
        }
    }

    /// Build the snapshot from a [`CompositionError`], capturing its
    /// [`Diagnostic`] facets so `err.code` / `err.detail.*` project alongside the
    /// legacy aliases.
    pub fn from_composition_error(err: &CompositionError) -> Self {
        Self {
            kind: "CompositionError",
            variant: variant_name_from_debug(err),
            msg: err.to_string(),
            facets: Some(Box::new(DiagnosticFacets::from_diagnostic(err))),
        }
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
    pub fn from_action_failure(verb: impl Into<String>, msg: impl Into<String>) -> Self {
        let variant = verb.into();
        let facets = crate::diagnostics::code_for_error_kind(&variant)
            .and_then(DiagnosticFacets::from_code)
            .map(Box::new);
        Self {
            kind: "LifecycleAction",
            variant,
            msg: msg.into(),
            facets,
        }
    }

    /// Render the snapshot as a JSON object for evaluation lookups.
    ///
    /// The legacy `kind`/`variant`/`msg` aliases are always present; the
    /// [`Diagnostic`] facets
    /// (`code`/`category`/`disposition`/`origin`/`severity`/`detail`) are added
    /// only when the source error was classifiable.
    ///
    /// `kind`/`variant` are the deprecated spellings of `category`/`code`: for a
    /// classifiable error they project the `category`/`code` facet values, and
    /// only a facet-less action failure falls back to the internal
    /// source-type/verb labels stored on the snapshot.
    ///
    /// When facets are present, the promoted handleability conveniences from
    /// error-catalog §2.6 project alongside them: the predicate sugar
    /// `is_transient`/`is_throttled`/`is_correctable` (derived from
    /// `disposition`) and `reset_at`/`retry_after_ms` (lifted from
    /// `detail`, `null` when the active code carries no such field). These are
    /// sugar only — `disposition` and `detail.*` stay canonical. They derive
    /// from facets, so they project **only** when facets are present; a
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
        match &self.facets {
            Some(facets) => {
                obj.insert("kind".to_string(), Value::from(facets.category));
                obj.insert("variant".to_string(), Value::from(facets.code));
            }
            None => {
                obj.insert("kind".to_string(), Value::from(self.kind));
                obj.insert("variant".to_string(), Value::from(self.variant.clone()));
            }
        }
        obj.insert("msg".to_string(), Value::from(self.msg.clone()));
        if let Some(facets) = &self.facets {
            obj.insert("code".to_string(), Value::from(facets.code));
            obj.insert("category".to_string(), Value::from(facets.category));
            obj.insert("disposition".to_string(), Value::from(facets.disposition));
            obj.insert("origin".to_string(), Value::from(facets.origin));
            obj.insert("severity".to_string(), Value::from(facets.severity));
            obj.insert("detail".to_string(), facets.detail.clone());

            // Promoted handleability conveniences (error-catalog §2.6): sugar
            // over the canonical `disposition`/`detail.*`, projected only when
            // facets exist.
            obj.insert(
                "is_transient".to_string(),
                Value::from(facets.disposition == "transient"),
            );
            obj.insert(
                "is_throttled".to_string(),
                Value::from(facets.disposition == "throttled"),
            );
            obj.insert(
                "is_correctable".to_string(),
                Value::from(facets.disposition == "correctable"),
            );
            obj.insert("reset_at".to_string(), promoted_detail_field(&facets.detail, "reset_at"));
            obj.insert(
                "retry_after_ms".to_string(),
                promoted_detail_field(&facets.detail, "retry_after_ms"),
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
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn error_info_from_claudine_error_records_kind_variant_msg() {
        let err = ClaudineError::Io(std::io::Error::other("disk full"));
        let info = LifecycleErrorInfo::from_claudine_error(&err);
        assert_eq!(info.kind, "ClaudineError");
        assert_eq!(info.variant, "Io");
        assert!(info.msg.contains("disk full"), "got: {}", info.msg);
    }

    #[test]
    fn error_info_from_harness_error_records_kind_variant_msg() {
        let err = HarnessError::ShellCommandDenied {
            command: "rm -rf /".to_string(),
        };
        let info = LifecycleErrorInfo::from_harness_error(&err);
        assert_eq!(info.kind, "HarnessError");
        assert_eq!(info.variant, "ShellCommandDenied");
        assert!(!info.msg.is_empty());
    }

    #[test]
    fn error_info_from_composition_error_records_kind_variant_msg() {
        let err = CompositionError::NoRunnableProviders;
        let info = LifecycleErrorInfo::from_composition_error(&err);
        assert_eq!(info.kind, "CompositionError");
        assert_eq!(info.variant, "NoRunnableProviders");
        assert!(!info.msg.is_empty());
    }

    #[test]
    fn error_info_to_value_has_kind_variant_msg() {
        let info = LifecycleErrorInfo {
            kind: "ClaudineError",
            variant: "Io".to_string(),
            msg: "disk full".to_string(),
            facets: None,
        };
        let value = info.to_value();
        assert_eq!(value.get("kind"), Some(&json!("ClaudineError")));
        assert_eq!(value.get("variant"), Some(&json!("Io")));
        assert_eq!(value.get("msg"), Some(&json!("disk full")));
    }

    #[test]
    fn composition_error_projects_diagnostic_facets_with_legacy_aliases() {
        let err = CompositionError::SchemaLoad {
            source_path: std::path::PathBuf::from("p.md"),
            message: "bad".to_string(),
        };
        let value = LifecycleErrorInfo::from_composition_error(&err).to_value();
        // Legacy aliases stay present, but now read as the deprecated spellings
        // of the faceted contract: `kind` mirrors `category`, `variant` mirrors
        // `code` (spec success criteria), not the internal Rust type/arm names.
        assert_eq!(value.get("kind"), Some(&json!("composition")));
        assert_eq!(value.get("variant"), Some(&json!("composition.schema_load")));
        assert!(value.get("msg").is_some());
        // New typed facets project alongside.
        assert_eq!(value.get("code"), Some(&json!("composition.schema_load")));
        assert_eq!(value.get("category"), Some(&json!("composition")));
        assert_eq!(value.get("disposition"), Some(&json!("correctable")));
        assert_eq!(value.get("origin"), Some(&json!("author")));
        assert_eq!(value.get("severity"), Some(&json!("error")));
        assert_eq!(value["detail"]["source_path"], json!("p.md"));
        // The deprecated aliases are exactly the faceted fields they alias.
        assert_eq!(value.get("kind"), value.get("category"));
        assert_eq!(value.get("variant"), value.get("code"));
    }

    #[test]
    fn legacy_aliases_mirror_category_and_code_across_classifiable_sources() {
        // The spec contract: for every classifiable source, `err.kind` reads as
        // `err.category` and `err.variant` reads as `err.code`. Cover all three
        // typed-Diagnostic builders plus the label-only `from_code` path.
        let classifiable = [
            LifecycleErrorInfo::from_composition_error(&CompositionError::NoRunnableProviders),
            LifecycleErrorInfo::from_harness_error(&HarnessError::ShellCommandDenied {
                command: "rm -rf /".to_string(),
            }),
            LifecycleErrorInfo::from_claudine_error(&ClaudineError::ProviderNotAvailable(
                "codex".to_string(),
            )),
            LifecycleErrorInfo::from_action_failure("rate_limit", "slow down"),
        ];
        for info in classifiable {
            let value = info.to_value();
            assert_eq!(
                value.get("kind"),
                value.get("category"),
                "err.kind must alias err.category"
            );
            assert_eq!(
                value.get("variant"),
                value.get("code"),
                "err.variant must alias err.code"
            );
        }
    }

    #[test]
    fn generic_action_verb_projects_only_legacy_aliases() {
        // A generic lifecycle action verb (not an error_kind) is not
        // classifiable, so no facet keys appear — only the legacy aliases.
        // Compatibility decision: with no faceted equivalent to alias,
        // `err.kind`/`err.variant` fall back to the internal source-type/verb
        // labels so the failure stays observable during migration.
        let info = LifecycleErrorInfo::from_action_failure("set_frontmatter", "boom");
        let value = info.to_value();
        assert_eq!(value.get("kind"), Some(&json!("LifecycleAction")));
        assert_eq!(value.get("variant"), Some(&json!("set_frontmatter")));
        assert!(value.get("code").is_none(), "no facets for a generic verb");
        assert!(value.get("category").is_none());
        assert!(value.get("severity").is_none(), "severity is facet-gated");
        assert!(value.get("detail").is_none());
        // The promoted sugar derives from facets, so it is absent too — a
        // facet-less verb projects only the legacy aliases (catalog §2.6).
        assert!(value.get("is_transient").is_none());
        assert!(value.get("is_throttled").is_none());
        assert!(value.get("is_correctable").is_none());
        assert!(value.get("reset_at").is_none());
        assert!(value.get("retry_after_ms").is_none());
    }

    #[test]
    fn harness_error_projects_diagnostic_facets() {
        // The "harness" failure surface: a HarnessError now carries facets.
        let info = LifecycleErrorInfo::from_harness_error(&HarnessError::ShellCommandDenied {
            command: "rm -rf /".to_string(),
        });
        let value = info.to_value();
        // `kind`/`variant` are the deprecated aliases of `category`/`code`.
        assert_eq!(value.get("kind"), Some(&json!("composition")));
        assert_eq!(value.get("variant"), Some(&json!("composition.shell_expansion")));
        assert_eq!(value.get("code"), Some(&json!("composition.shell_expansion")));
        assert_eq!(value.get("category"), Some(&json!("composition")));
        assert_eq!(value.get("disposition"), Some(&json!("correctable")));
        assert_eq!(value.get("origin"), Some(&json!("author")));
        assert_eq!(value.get("severity"), Some(&json!("error")));
        assert_eq!(value["detail"]["command"], json!("rm -rf /"));
    }

    #[test]
    fn claudine_provider_error_projects_provider_facets() {
        // The "provider" + "top-level Claudine" surface, via `from_claudine_error`.
        let info = LifecycleErrorInfo::from_claudine_error(&ClaudineError::ProviderNotAvailable(
            "codex".to_string(),
        ));
        let value = info.to_value();
        // `kind` is the deprecated alias of `category`.
        assert_eq!(value.get("kind"), Some(&json!("provider")));
        assert_eq!(value.get("code"), Some(&json!("provider.unavailable")));
        assert_eq!(value.get("category"), Some(&json!("provider")));
        assert_eq!(value.get("origin"), Some(&json!("environment")));
        assert_eq!(value["detail"]["provider"], json!("codex"));
    }

    #[test]
    fn claudine_io_error_projects_io_facets() {
        // A top-level Claudine io failure projects an `io.*` code.
        let info = LifecycleErrorInfo::from_claudine_error(&ClaudineError::Io(
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        ));
        let value = info.to_value();
        assert_eq!(value.get("code"), Some(&json!("io.permission_denied")));
        assert_eq!(value.get("category"), Some(&json!("io")));
    }

    #[test]
    fn action_failure_error_kind_projects_cap_facets() {
        // The "cap" surface: a provider rate-limit reaches the lifecycle `err`
        // as an `error_kind` string carried through `from_action_failure`.
        let info = LifecycleErrorInfo::from_action_failure("rate_limit", "slow down");
        let value = info.to_value();
        assert_eq!(value.get("code"), Some(&json!("cap.rate_limit")));
        assert_eq!(value.get("category"), Some(&json!("cap")));
        assert_eq!(value.get("disposition"), Some(&json!("throttled")));
        assert_eq!(value.get("origin"), Some(&json!("provider")));
    }

    #[test]
    fn action_failure_error_kind_projects_timeout_facets() {
        // The "timeout" surface.
        let info = LifecycleErrorInfo::from_action_failure("step_timeout", "30m of silence");
        let value = info.to_value();
        assert_eq!(value.get("code"), Some(&json!("timeout.step_silence")));
        assert_eq!(value.get("category"), Some(&json!("timeout")));
    }

    #[test]
    fn action_failure_error_kind_projects_runaway_facets() {
        // The "runaway" surface.
        let info = LifecycleErrorInfo::from_action_failure("runaway_volume", "flood");
        let value = info.to_value();
        assert_eq!(value.get("code"), Some(&json!("runaway.volume")));
        assert_eq!(value.get("category"), Some(&json!("runaway")));
        assert_eq!(value.get("disposition"), Some(&json!("unrecoverable")));
    }

    #[test]
    fn action_failure_error_kind_projects_provider_facets() {
        // The "provider" run surface, via an `agent_failure` error_kind.
        let info = LifecycleErrorInfo::from_action_failure("agent_failure", "exit 1");
        let value = info.to_value();
        assert_eq!(value.get("code"), Some(&json!("provider.exited")));
        assert_eq!(value.get("category"), Some(&json!("provider")));
    }

    #[test]
    fn severity_projects_for_classifiable_errors() {
        // Typed-Diagnostic path: a correctable composition error defaults to
        // `error` severity (catalog §1).
        let value =
            LifecycleErrorInfo::from_composition_error(&CompositionError::NoRunnableProviders)
                .to_value();
        assert_eq!(value.get("severity"), Some(&json!("error")));

        // Label-only `from_code` path: a throttled cap defaults to `warning`.
        let value = LifecycleErrorInfo::from_action_failure("rate_limit", "slow down").to_value();
        assert_eq!(value.get("disposition"), Some(&json!("throttled")));
        assert_eq!(value.get("severity"), Some(&json!("warning")));
    }

    #[test]
    fn throttled_cap_promotes_is_throttled_and_null_cap_fields() {
        // A rate-limit cap reaches `err` via `from_action_failure`, so its
        // facets come from the `from_code` path → `detail` is `Null`. The
        // predicate sugar reflects the throttled disposition; the cap fields
        // promote to `null` because the label can't reconstruct them.
        let info = LifecycleErrorInfo::from_action_failure("rate_limit", "slow down");
        let value = info.to_value();
        assert_eq!(value.get("disposition"), Some(&json!("throttled")));
        assert_eq!(value.get("is_throttled"), Some(&json!(true)));
        assert_eq!(value.get("is_transient"), Some(&json!(false)));
        assert_eq!(value.get("is_correctable"), Some(&json!(false)));
        assert_eq!(value.get("reset_at"), Some(&Value::Null));
        assert_eq!(value.get("retry_after_ms"), Some(&Value::Null));
    }

    #[test]
    fn transient_error_promotes_is_transient_only() {
        let info = LifecycleErrorInfo::from_action_failure("step_timeout", "30m of silence");
        let value = info.to_value();
        assert_eq!(value.get("disposition"), Some(&json!("transient")));
        assert_eq!(value.get("is_transient"), Some(&json!(true)));
        assert_eq!(value.get("is_throttled"), Some(&json!(false)));
        assert_eq!(value.get("is_correctable"), Some(&json!(false)));
    }

    #[test]
    fn correctable_error_promotes_is_correctable_only() {
        // A composition error is `correctable`; its `detail` carries no cap
        // fields, so `reset_at`/`retry_after_ms` promote to `null`.
        let value =
            LifecycleErrorInfo::from_composition_error(&CompositionError::NoRunnableProviders)
                .to_value();
        assert_eq!(value.get("disposition"), Some(&json!("correctable")));
        assert_eq!(value.get("is_correctable"), Some(&json!(true)));
        assert_eq!(value.get("is_transient"), Some(&json!(false)));
        assert_eq!(value.get("is_throttled"), Some(&json!(false)));
        assert_eq!(value.get("reset_at"), Some(&Value::Null));
        assert_eq!(value.get("retry_after_ms"), Some(&Value::Null));
    }

    #[test]
    fn cap_detail_promotes_reset_at_and_retry_after_ms_to_top_level() {
        // No typed `Diagnostic` reconstructs a cap `detail` (the cap codes reach
        // `err` via the label-only `from_code` path), so hand-build facets whose
        // `detail` carries the cap fields and assert they promote out of detail.
        let info = LifecycleErrorInfo {
            kind: "LifecycleAction",
            variant: "rate_limit".to_string(),
            msg: "slow down".to_string(),
            facets: Some(Box::new(DiagnosticFacets {
                code: "cap.rate_limit",
                category: "cap",
                disposition: "throttled",
                origin: "provider",
                severity: "warning",
                detail: json!({
                    "provider": "claude",
                    "reset_at": "2026-06-28T17:30:00Z",
                    "retry_after_ms": 5_400_000u64,
                }),
            })),
        };
        let value = info.to_value();
        // Promoted to the top level …
        assert_eq!(value.get("reset_at"), Some(&json!("2026-06-28T17:30:00Z")));
        assert_eq!(value.get("retry_after_ms"), Some(&json!(5_400_000u64)));
        assert_eq!(value.get("is_throttled"), Some(&json!(true)));
        // … while `detail.*` stays canonical.
        assert_eq!(value["detail"]["reset_at"], json!("2026-06-28T17:30:00Z"));
        assert_eq!(value["detail"]["retry_after_ms"], json!(5_400_000u64));
    }

    #[test]
    fn null_detail_field_promotes_to_null() {
        // A facet `detail` object that carries no cap fields yields `null`
        // promoted values — not an error, per the null-sentinel convention.
        let info = LifecycleErrorInfo {
            kind: "LifecycleAction",
            variant: "x".to_string(),
            msg: "m".to_string(),
            facets: Some(Box::new(DiagnosticFacets {
                code: "document.missing_frontmatter",
                category: "document",
                disposition: "correctable",
                origin: "provider",
                severity: "error",
                detail: json!({ "doc": "spec.md", "property": "status" }),
            })),
        };
        let value = info.to_value();
        assert_eq!(value.get("reset_at"), Some(&Value::Null));
        assert_eq!(value.get("retry_after_ms"), Some(&Value::Null));
        assert_eq!(value.get("is_correctable"), Some(&json!(true)));
    }

    #[test]
    fn timing_to_value_omits_missing_fields() {
        let timing = LifecycleTiming {
            document_ms: Some(42),
            total_ms: None,
            step_ms: None,
        };
        let value = timing.to_value();
        assert_eq!(value.get("document_ms"), Some(&json!(42u64)));
        assert!(value.get("total_ms").is_none());
        assert!(value.get("step_ms").is_none());
    }

    #[test]
    fn current_to_value_has_ctx_and_env() {
        let current = LifecycleCurrent {
            ctx: json!({"agent": "claude"}),
            env: json!({"HOME": "/tmp"}),
        };
        let value = current.to_value();
        assert_eq!(value.get("ctx").unwrap().get("agent"), Some(&json!("claude")));
        assert_eq!(value.get("env").unwrap().get("HOME"), Some(&json!("/tmp")));
    }

    /// Build an [`EffectiveState`] over `fm` with a cheap (sniff-free) context.
    fn state(fm: Value) -> darkmatter::markdown::compose::EffectiveState {
        use darkmatter::markdown::compose::{ComposeContext, EffectiveStateBuilder};
        let fm: std::collections::HashMap<String, Value> = fm
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_context(ComposeContext::capture_for_content(std::path::Path::new("."), ""))
            .build()
            .unwrap()
    }

    #[test]
    fn injected_globals_attaches_err_timing_current() {
        let info = LifecycleErrorInfo {
            kind: "ClaudineError",
            variant: "Io".to_string(),
            msg: "disk full".to_string(),
            facets: None,
        };
        let timing = LifecycleTiming {
            document_ms: Some(100),
            total_ms: None,
            step_ms: None,
        };
        let current = LifecycleCurrent {
            ctx: json!({"agent": "codex"}),
            env: json!({"DEBUG": "1"}),
        };
        let globals = lifecycle_injected_globals(Some(&info), Some(&timing), Some(&current));
        assert!(globals.contains_key("err"));
        assert!(globals.contains_key("timing"));
        assert!(globals.contains_key("current"));
    }

    #[test]
    fn injected_globals_omits_unattached() {
        let globals = lifecycle_injected_globals(None, None, None);
        assert!(globals.is_empty());
    }

    #[test]
    fn err_global_resolves_through_dm2_subtree() {
        use darkmatter::markdown::compose::subtree::SubtreeCompose;
        let info = LifecycleErrorInfo {
            kind: "ClaudineError",
            variant: "Io".to_string(),
            msg: "disk full".to_string(),
            facets: None,
        };
        let globals = lifecycle_injected_globals(Some(&info), None, None);
        let state = state(json!({}));
        let resolved = SubtreeCompose::new(&json!("{{err.msg}}"), &state)
            .with_globals(globals)
            .compose()
            .unwrap();
        assert_eq!(resolved, json!("disk full"));
    }

    #[test]
    fn timing_global_resolves_through_dm2_subtree() {
        use darkmatter::markdown::compose::subtree::SubtreeCompose;
        let timing = LifecycleTiming {
            document_ms: Some(100),
            total_ms: None,
            step_ms: None,
        };
        let globals = lifecycle_injected_globals(None, Some(&timing), None);
        let state = state(json!({}));
        let resolved = SubtreeCompose::new(&json!("took {{timing.document_ms}}ms"), &state)
            .with_globals(globals)
            .compose()
            .unwrap();
        assert_eq!(resolved, json!("took 100ms"));
    }

    #[test]
    fn doc_namespace_reaches_literal_err_property_through_dm2() {
        // `doc.err` reaches a literal frontmatter `err` property, not the
        // lifecycle `err` global — `doc` is never an injected global root.
        use darkmatter::markdown::compose::subtree::SubtreeCompose;
        let info = LifecycleErrorInfo {
            kind: "ClaudineError",
            variant: "Io".to_string(),
            msg: "disk full".to_string(),
            facets: None,
        };
        let globals = lifecycle_injected_globals(Some(&info), None, None);
        let state = state(json!({"err": "literal-value"}));
        let resolved = SubtreeCompose::new(&json!("{{doc.err}} / {{err.msg}}"), &state)
            .with_globals(globals)
            .compose()
            .unwrap();
        assert_eq!(resolved, json!("literal-value / disk full"));
    }

    #[test]
    #[serial_test::serial(env_lifecycle_current)]
    fn capture_env_reflects_live_process_environment() {
        let key = "CLAUDINE_TEST_CAPTURE_ENV_LIVE";
        // SAFETY: serialized via #[serial]; no other thread reads this var.
        unsafe { std::env::set_var(key, "live-value") };
        let env = LifecycleCurrent::capture_env();
        unsafe { std::env::remove_var(key) };
        assert_eq!(env.get(key), Some(&json!("live-value")));
    }

    #[test]
    fn capture_env_only_leaves_ctx_empty() {
        let current = LifecycleCurrent::capture_env_only();
        assert_eq!(current.ctx, json!({}));
        assert!(current.env.is_object(), "env is a JSON object snapshot");
    }

    #[test]
    fn capture_at_event_populates_ctx_and_env() {
        let current = LifecycleCurrent::capture_at_event(std::path::Path::new("."));
        // `ctx.today` is always captured (date/time group, zero I/O).
        assert!(
            current.ctx.get("today").and_then(|v| v.as_str()).is_some(),
            "ctx snapshot carries today: {:?}",
            current.ctx
        );
        assert!(current.env.is_object());
    }

    #[test]
    fn timing_from_instants_populates_document_ms_and_total_ms() {
        let start = std::time::Instant::now();
        let run_start = start;
        // Spin until at least 1ms elapsed so the conversion is observably > 0.
        let at = loop {
            let now = std::time::Instant::now();
            if now.duration_since(start).as_millis() >= 1 {
                break now;
            }
        };
        let timing = LifecycleTiming::from_instants(start, Some(run_start), at);
        let doc = timing.document_ms.expect("document_ms is populated");
        let total = timing.total_ms.expect("total_ms populated with run_start");
        assert!(doc >= 1, "document_ms is monotonic non-decreasing: {doc}");
        assert_eq!(doc, total, "document and run start coincide here");
        assert!(timing.step_ms.is_none(), "step_ms stays None outside a sequence");
    }

    #[test]
    fn timing_from_instants_omits_total_ms_without_run_start() {
        let start = std::time::Instant::now();
        let timing = LifecycleTiming::from_instants(start, None, std::time::Instant::now());
        assert!(timing.document_ms.is_some());
        assert!(timing.total_ms.is_none());
    }

    /// Late-binding contract: a stack `when:` clause reacts to an environment
    /// value present in the `current.env` snapshot the production builders
    /// attach at **event time**, distinct from a value snapshotted at "prepare"
    /// time. Resolved through DM2's layered lookup over the injected `current`
    /// global, proving the binding is late.
    #[test]
    #[serial_test::serial(env_lifecycle_current)]
    fn when_clause_reacts_to_env_changed_after_prepare() {
        use darkmatter::markdown::compose::expression::{evaluate, is_truthy, parse};
        use darkmatter::markdown::compose::subtree::LayeredLookup;

        let key = "CLAUDINE_TEST_LATE_BINDING_MYVAR";
        // SAFETY: serialized via #[serial]; no other thread reads this var.

        // "Prepare time": the variable holds an old value. A `current.env`
        // snapshot captured now would carry `old`.
        unsafe { std::env::set_var(key, "old") };
        let prepare_snapshot = LifecycleCurrent::capture_env_only();

        // A side effect / external change happens AFTER prepare.
        unsafe { std::env::set_var(key, "x") };
        // The production builders capture `current` at EVENT time, so they see
        // the post-change value.
        let event_snapshot = LifecycleCurrent::capture_env_only();
        unsafe { std::env::remove_var(key) };

        let base = state(json!({}));
        let expr = parse(&format!("current.env.{key} == 'x'")).expect("parses");

        // Against the event-time snapshot, the guard fires (late binding).
        let event_globals =
            lifecycle_injected_globals(None, None, Some(&event_snapshot));
        let event_lookup = LayeredLookup::new(&base, &event_globals, None);
        let event_value = evaluate(&expr, &event_lookup).expect("evaluates");
        assert!(
            is_truthy(&event_value),
            "when clause fires on the value present at event time"
        );

        // Against the prepare-time snapshot, the same guard does NOT fire,
        // proving the reaction is to the late-bound value, not the prepare one.
        let prepare_globals =
            lifecycle_injected_globals(None, None, Some(&prepare_snapshot));
        let prepare_lookup = LayeredLookup::new(&base, &prepare_globals, None);
        let prepare_value = evaluate(&expr, &prepare_lookup).expect("evaluates");
        assert!(
            !is_truthy(&prepare_value),
            "the prepare-time snapshot still holds the old value"
        );
    }
}
