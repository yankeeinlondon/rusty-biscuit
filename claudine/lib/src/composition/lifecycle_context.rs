//! Lifecycle execution context: the stack-only globals `err`, `timing`,
//! and `current`.
//!
//! These globals are visible only inside lifecycle stack expressions
//! (`when:` clauses and action arguments). Body and frontmatter
//! interpolation continue to resolve `err`, `timing`, and `current` as
//! ordinary frontmatter identifiers through Darkmatter's standard
//! pipeline — this module does not alter that behavior.
//!
//! Phase 3 introduces the data model and the
//! [`LifecycleLookup`] evaluator adapter. Phase 4 wires the lookup into
//! the stack execution engine.
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

// rustfmt doesn't support let-chains yet, so nested ifs are required
#![allow(clippy::collapsible_if)]

use darkmatter::markdown::compose::expression::{EvaluationLookup, ResolutionContext};
use serde_json::{Map, Value};

use super::error::CompositionError;
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
/// `err.msg`. A bare `err` resolves to the whole object.
#[derive(Debug, Clone, PartialEq, Eq)]
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
}

impl LifecycleErrorInfo {
    /// Build the snapshot from a [`ClaudineError`].
    pub fn from_claudine_error(err: &ClaudineError) -> Self {
        Self {
            kind: "ClaudineError",
            variant: variant_name_from_debug(err),
            msg: err.to_string(),
        }
    }

    /// Build the snapshot from a [`HarnessError`].
    pub fn from_harness_error(err: &HarnessError) -> Self {
        Self {
            kind: "HarnessError",
            variant: variant_name_from_debug(err),
            msg: err.to_string(),
        }
    }

    /// Build the snapshot from a [`CompositionError`].
    pub fn from_composition_error(err: &CompositionError) -> Self {
        Self {
            kind: "CompositionError",
            variant: variant_name_from_debug(err),
            msg: err.to_string(),
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
        }
    }

    /// Render the snapshot as a JSON object for evaluation lookups.
    pub fn to_value(&self) -> Value {
        serde_json::json!({
            "kind": self.kind,
            "variant": self.variant,
            "msg": self.msg,
        })
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

/// Darkmatter expression lookup for lifecycle stack evaluation.
///
/// Resolution order:
///
/// 1. Lifecycle globals (`err`, `timing`, `current`) — only meaningful in
///    stack expressions; body/frontmatter interpolation never sees them.
/// 2. Reserved `doc` namespace (bare `doc` → the whole frontmatter object;
///    `doc.<path>` → dotted traversal). Intercepted before `ctx.`/`env.`
///    so `doc.err` reaches a literal `err` property rather than the
///    lifecycle global.
/// 3. `env.NAME` — process environment at evaluation time.
/// 4. `ctx.NAME` — the runtime context (`agent`, `model`, `today`, …).
/// 5. Bare names — frontmatter properties, including nested paths via `.`.
///
/// When constructed with a base directory ([`with_base_dir`](Self::with_base_dir)),
/// read-side expression functions (`file_exists`, `absolute`, …) resolve
/// against the prompt's parent directory.
#[derive(Debug, Clone, Copy)]
pub struct LifecycleLookup<'a> {
    frontmatter: &'a Map<String, Value>,
    err: Option<&'a LifecycleErrorInfo>,
    timing: Option<&'a LifecycleTiming>,
    current: Option<&'a LifecycleCurrent>,
    base_dir: Option<&'a std::path::Path>,
}

impl<'a> LifecycleLookup<'a> {
    /// Create a lookup over the composed frontmatter with no lifecycle
    /// globals attached.
    pub fn new(frontmatter: &'a Map<String, Value>) -> Self {
        Self {
            frontmatter,
            err: None,
            timing: None,
            current: None,
            base_dir: None,
        }
    }

    /// Attach the `err` global snapshot.
    #[must_use]
    pub fn with_err(mut self, err: &'a LifecycleErrorInfo) -> Self {
        self.err = Some(err);
        self
    }

    /// Attach the `timing` global snapshot.
    #[must_use]
    pub fn with_timing(mut self, timing: &'a LifecycleTiming) -> Self {
        self.timing = Some(timing);
        self
    }

    /// Attach the `current` global snapshot.
    #[must_use]
    pub fn with_current(mut self, current: &'a LifecycleCurrent) -> Self {
        self.current = Some(current);
        self
    }

    /// Root read-side expression functions at `base_dir` (typically the
    /// prompt document's parent directory). `None` leaves the lookup
    /// context-free.
    #[must_use]
    pub fn with_base_dir(mut self, base_dir: Option<&'a std::path::Path>) -> Self {
        self.base_dir = base_dir;
        self
    }
}

impl EvaluationLookup for LifecycleLookup<'_> {
    fn get(&self, path: &str) -> Option<Value> {
        // 1. Lifecycle globals. When the snapshot is attached, resolve
        //    against it. When not attached, fall through to frontmatter so
        //    a literal `err`/`timing`/`current` property stays reachable
        //    (body/frontmatter interpolation contract).
        //    `doc.err` is intercepted below so a literal `err` property
        //    stays reachable through the `doc.` namespace even when the
        //    global is attached.
        if path == "err" {
            if let Some(info) = self.err {
                return Some(info.to_value());
            }
            // fall through to frontmatter
        } else if let Some(rest) = path.strip_prefix("err.") {
            if let Some(info) = self.err {
                let value = info.to_value();
                return walk_json(value, rest);
            }
            // fall through to frontmatter
        } else if path == "timing" {
            if let Some(timing) = self.timing {
                return Some(timing.to_value());
            }
            // fall through to frontmatter
        } else if let Some(rest) = path.strip_prefix("timing.") {
            if let Some(timing) = self.timing {
                let value = timing.to_value();
                return walk_json(value, rest);
            }
            // fall through to frontmatter
        } else if path == "current" {
            if let Some(current) = self.current {
                return Some(current.to_value());
            }
            // fall through to frontmatter
        } else if let Some(rest) = path.strip_prefix("current.") {
            if let Some(current) = self.current {
                let value = current.to_value();
                return walk_json(value, rest);
            }
            // fall through to frontmatter
        }

        // 2. Reserved `doc` namespace — intercepted before `ctx.`/`env.` so
        //    `doc.err` reaches a literal property rather than the lifecycle
        //    global.
        if path == "doc" || path.starts_with("doc.") {
            return walk_json(
                Value::Object(self.frontmatter.clone()),
                path.trim_start_matches("doc").trim_start_matches('.'),
            );
        }

        // 3. env.NAME — process environment at evaluation time.
        if let Some(name) = path.strip_prefix("env.") {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                return None;
            }
            return std::env::var(trimmed).ok().map(Value::String);
        }

        // 4. ctx.NAME — runtime context. Falls back to the captured
        //    `current.ctx` snapshot when available.
        if let Some(name) = path.strip_prefix("ctx.") {
            if let Some(current) = self.current {
                let value = current.ctx.clone();
                return walk_json(value, name);
            }
            return None;
        }

        // 5. Bare names — frontmatter properties.
        if let Some(value) = self.frontmatter.get(path) {
            return Some(value.clone());
        }

        // Dotted traversal into frontmatter.
        let mut parts = path.split('.');
        let head = parts.next()?;
        let mut current = self.frontmatter.get(head)?.clone();
        for part in parts {
            current = match current {
                Value::Object(map) => map.get(part).cloned()?,
                _ => return None,
            };
        }
        Some(current)
    }

    fn resolution_context(&self) -> Option<ResolutionContext> {
        self.base_dir
            .map(|dir| ResolutionContext::new(dir.to_path_buf()))
    }
}

/// Walk a dotted path into a JSON value, returning `None` if any segment
/// does not resolve.
fn walk_json(value: Value, path: &str) -> Option<Value> {
    if path.is_empty() {
        return Some(value);
    }
    let mut current = value;
    for segment in path.split('.') {
        current = match current {
            Value::Object(map) => map.get(segment).cloned()?,
            _ => return None,
        };
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn map(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

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
        };
        let value = info.to_value();
        assert_eq!(value.get("kind"), Some(&json!("ClaudineError")));
        assert_eq!(value.get("variant"), Some(&json!("Io")));
        assert_eq!(value.get("msg"), Some(&json!("disk full")));
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

    #[test]
    fn lookup_resolves_err_global_when_attached() {
        let fm = map(json!({}));
        let info = LifecycleErrorInfo {
            kind: "ClaudineError",
            variant: "Io".to_string(),
            msg: "disk full".to_string(),
        };
        let lookup = LifecycleLookup::new(&fm).with_err(&info);
        let err_value = lookup.get("err").expect("err resolves");
        assert_eq!(err_value.get("variant"), Some(&json!("Io")));
        assert_eq!(lookup.get("err.variant"), Some(json!("Io")));
        assert_eq!(lookup.get("err.msg"), Some(json!("disk full")));
    }

    #[test]
    fn lookup_err_global_none_when_not_attached() {
        let fm = map(json!({}));
        let lookup = LifecycleLookup::new(&fm);
        assert!(lookup.get("err").is_none());
        assert!(lookup.get("err.variant").is_none());
    }

    #[test]
    fn lookup_resolves_timing_global_when_attached() {
        let fm = map(json!({}));
        let timing = LifecycleTiming {
            document_ms: Some(100),
            total_ms: None,
            step_ms: None,
        };
        let lookup = LifecycleLookup::new(&fm).with_timing(&timing);
        assert_eq!(lookup.get("timing.document_ms"), Some(json!(100u64)));
        assert!(lookup.get("timing.total_ms").is_none());
    }

    #[test]
    fn lookup_resolves_current_global_when_attached() {
        let fm = map(json!({}));
        let current = LifecycleCurrent {
            ctx: json!({"agent": "codex"}),
            env: json!({"DEBUG": "1"}),
        };
        let lookup = LifecycleLookup::new(&fm).with_current(&current);
        assert_eq!(
            lookup.get("current.ctx.agent"),
            Some(json!("codex"))
        );
        assert_eq!(
            lookup.get("current.env.DEBUG"),
            Some(json!("1"))
        );
    }

    #[test]
    fn lookup_ctx_uses_current_snapshot_when_attached() {
        let fm = map(json!({}));
        let current = LifecycleCurrent {
            ctx: json!({"agent": "codex"}),
            env: json!({}),
        };
        let lookup = LifecycleLookup::new(&fm).with_current(&current);
        assert_eq!(lookup.get("ctx.agent"), Some(json!("codex")));
    }

    #[test]
    fn lookup_doc_namespace_reaches_literal_err_property() {
        // `doc.err` must reach a literal frontmatter `err` property, not the
        // lifecycle `err` global. This is the `doc.err` escape hatch.
        let fm = map(json!({"err": "literal-value"}));
        let info = LifecycleErrorInfo {
            kind: "ClaudineError",
            variant: "Io".to_string(),
            msg: "disk full".to_string(),
        };
        let lookup = LifecycleLookup::new(&fm).with_err(&info);
        assert_eq!(lookup.get("doc.err"), Some(json!("literal-value")));
        // Bare `err` still reaches the lifecycle global.
        assert_eq!(
            lookup.get("err").unwrap().get("variant"),
            Some(&json!("Io"))
        );
    }

    #[test]
    fn lookup_bare_err_reaches_frontmatter_when_global_absent() {
        // Without an attached `err` snapshot, a bare `err` reference falls
        // through to frontmatter, so a literal `err` property is visible.
        let fm = map(json!({"err": "frontmatter-value"}));
        let lookup = LifecycleLookup::new(&fm);
        assert_eq!(lookup.get("err"), Some(json!("frontmatter-value")));
    }

    #[test]
    fn lookup_bare_timing_reaches_frontmatter_when_global_absent() {
        let fm = map(json!({"timing": "fast"}));
        let lookup = LifecycleLookup::new(&fm);
        assert_eq!(lookup.get("timing"), Some(json!("fast")));
    }

    #[test]
    fn lookup_bare_current_reaches_frontmatter_when_global_absent() {
        let fm = map(json!({"current": "yes"}));
        let lookup = LifecycleLookup::new(&fm);
        assert_eq!(lookup.get("current"), Some(json!("yes")));
    }

    #[test]
    fn lookup_frontmatter_traversal_handles_nested_paths() {
        let fm = map(json!({"config": {"retries": 3}}));
        let lookup = LifecycleLookup::new(&fm);
        assert_eq!(lookup.get("config.retries"), Some(json!(3)));
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
    /// value present in `current.env` at context-construction time, distinct
    /// from a value snapshotted at "prepare" time. This is the spec's required
    /// "reacts to an environment value changed after prepare" behavior, proven
    /// at the population layer (the snapshot the production builders attach).
    #[test]
    #[serial_test::serial(env_lifecycle_current)]
    fn when_clause_reacts_to_env_changed_after_prepare() {
        use darkmatter::markdown::compose::expression::{evaluate, is_truthy, parse};

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

        let fm = Map::new();
        let expr = parse(&format!("current.env.{key} == 'x'")).expect("parses");

        // Against the event-time snapshot, the guard fires (late binding).
        let event_lookup = LifecycleLookup::new(&fm).with_current(&event_snapshot);
        let event_value = evaluate(&expr, &event_lookup).expect("evaluates");
        assert!(
            is_truthy(&event_value),
            "when clause fires on the value present at event time"
        );

        // Against the prepare-time snapshot, the same guard does NOT fire,
        // proving the reaction is to the late-bound value, not the prepare one.
        let prepare_lookup = LifecycleLookup::new(&fm).with_current(&prepare_snapshot);
        let prepare_value = evaluate(&expr, &prepare_lookup).expect("evaluates");
        assert!(
            !is_truthy(&prepare_value),
            "the prepare-time snapshot still holds the old value"
        );
    }
}
