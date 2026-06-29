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
//! optional-error `finalize` events. Exposed as `err.kind`, `err.variant`,
//! `err.msg`. Parse-time validation (see
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

use super::error::CompositionError;
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
/// Visible in lifecycle stack expressions as `err.kind`, `err.variant`,
/// `err.msg` (legacy aliases, always present) plus the [`Diagnostic`] facets
/// `err.code`, `err.category`, `err.disposition`, `err.origin`, and
/// `err.detail.*` when the source error is classifiable. A bare `err` resolves
/// to the whole object.
#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleErrorInfo {
    /// The source error type name (e.g. `"ClaudineError"`,
    /// `"HarnessError"`, `"CompositionError"`).
    pub kind: &'static str,

    /// The enum variant name (e.g. `"Io"`, `"ShellCommandDenied"`,
    /// `"LifecycleInvalid"`).
    pub variant: String,

    /// The human-readable error message (the `Display` rendering of the
    /// source error).
    pub msg: String,

    /// The classification facets, when the source error implements
    /// [`Diagnostic`]. `None` for error types not yet wired into the contract,
    /// in which case only the legacy `kind`/`variant`/`msg` aliases project.
    /// Boxed to keep `LifecycleErrorInfo` small — it is the `Err` type of several
    /// hot `Result`-returning lifecycle helpers.
    pub facets: Option<Box<DiagnosticFacets>>,
}

/// The [`Diagnostic`] facets captured from a typed error, projected into the
/// lifecycle `err` global as `err.code` / `err.category` / `err.disposition` /
/// `err.origin` / `err.detail.*`.
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
            detail: err.detail(),
        }
    }
}

impl LifecycleErrorInfo {
    /// Build the snapshot from a [`ClaudineError`].
    ///
    /// `ClaudineError` does not yet implement [`Diagnostic`], so only the legacy
    /// `kind`/`variant`/`msg` aliases project (facets are `None`).
    pub fn from_claudine_error(err: &ClaudineError) -> Self {
        Self {
            kind: "ClaudineError",
            variant: variant_name_from_debug(err),
            msg: err.to_string(),
            facets: None,
        }
    }

    /// Build the snapshot from a [`HarnessError`].
    ///
    /// `HarnessError` does not yet implement [`Diagnostic`], so only the legacy
    /// `kind`/`variant`/`msg` aliases project (facets are `None`).
    pub fn from_harness_error(err: &HarnessError) -> Self {
        Self {
            kind: "HarnessError",
            variant: variant_name_from_debug(err),
            msg: err.to_string(),
            facets: None,
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
    pub fn from_action_failure(verb: impl Into<String>, msg: impl Into<String>) -> Self {
        Self {
            kind: "LifecycleAction",
            variant: verb.into(),
            msg: msg.into(),
            facets: None,
        }
    }

    /// Render the snapshot as a JSON object for evaluation lookups.
    ///
    /// The legacy `kind`/`variant`/`msg` aliases are always present; the
    /// [`Diagnostic`] facets (`code`/`category`/`disposition`/`origin`/`detail`)
    /// are added only when the source error was classifiable.
    pub fn to_value(&self) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert("kind".to_string(), Value::from(self.kind));
        obj.insert("variant".to_string(), Value::from(self.variant.clone()));
        obj.insert("msg".to_string(), Value::from(self.msg.clone()));
        if let Some(facets) = &self.facets {
            obj.insert("code".to_string(), Value::from(facets.code));
            obj.insert("category".to_string(), Value::from(facets.category));
            obj.insert("disposition".to_string(), Value::from(facets.disposition));
            obj.insert("origin".to_string(), Value::from(facets.origin));
            obj.insert("detail".to_string(), facets.detail.clone());
        }
        Value::Object(obj)
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
        // Legacy aliases stay present.
        assert_eq!(value.get("kind"), Some(&json!("CompositionError")));
        assert_eq!(value.get("variant"), Some(&json!("SchemaLoad")));
        assert!(value.get("msg").is_some());
        // New typed facets project alongside.
        assert_eq!(value.get("code"), Some(&json!("composition.schema_load")));
        assert_eq!(value.get("category"), Some(&json!("composition")));
        assert_eq!(value.get("disposition"), Some(&json!("correctable")));
        assert_eq!(value.get("origin"), Some(&json!("author")));
        assert_eq!(value["detail"]["source_path"], json!("p.md"));
    }

    #[test]
    fn non_diagnostic_error_projects_only_legacy_aliases() {
        // A HarnessError does not implement `Diagnostic` yet, so no facet keys
        // appear — only the legacy aliases.
        let info = LifecycleErrorInfo::from_harness_error(&HarnessError::ShellCommandDenied {
            command: "x".to_string(),
        });
        let value = info.to_value();
        assert!(value.get("kind").is_some());
        assert!(value.get("code").is_none(), "no facets without Diagnostic");
        assert!(value.get("detail").is_none());
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
