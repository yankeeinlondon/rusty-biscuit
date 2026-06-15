//! Adapter that bridges Claudine [`EventMeta`] to Darkmatter's expression
//! evaluator.
//!
//! Darkmatter exposes a shared boolean and interpolation expression engine
//! (`darkmatter::markdown::compose::expression`) keyed off the
//! [`EvaluationLookup`] trait. This module wraps an [`EventMeta`] reference
//! so that dispatch templates, hook `when` conditions, event matchers, and
//! harness validation messages can all evaluate the same expression syntax
//! against the live event payload.
//!
//! ## Adapters
//!
//! - [`EventMetaExpressionLookup`] — used by templates, matchers, and harness
//!   validation. Resolves the full event path surface but deliberately leaves
//!   `ctx.*` unresolved.
//! - [`EventMetaConditionLookup`] — used by hook `when` evaluation. Layers
//!   Darkmatter's lazy `ctx.*` capture on top of [`EventMetaExpressionLookup`]
//!   so that conditions like `ctx.today != ''` resolve correctly.
//!
//! ## Resolution Order (both adapters)
//!
//! 1. `env.NAME` — resolves to the current process environment variable.
//!    Fallback syntax (`env.NAME || "default"`) is handled by Darkmatter's
//!    parser, so this layer never inspects the `||` token itself.
//! 2. `ctx.*` — left unresolved by [`EventMetaExpressionLookup`].
//!    [`EventMetaConditionLookup`] short-circuits `ctx.*` to Darkmatter's
//!    [`CtxLookup`] before falling through to the inner adapter.
//! 3. `extra.<key>[.<nested>...]` — resolves against `EventMeta.extra`,
//!    preserving JSON scalar/object/array values for helpers such as
//!    `length(...)` and `has_key(...)`.
//! 4. `tool_input.<path>` and `tool_response.<path>` — drill into the
//!    JSON payload of the corresponding `EventMeta` field.
//! 5. `os.*`, `hardware.*`, `git.*`, `project.*` — resolve against
//!    [`EventMeta.env`](crate::events::EventMeta::env) using exactly the
//!    same paths that
//!    [`TemplateVariable::key`](crate::dispatch::template::TemplateVariable::key)
//!    has historically exposed.
//! 6. Top-level event fields (`provider`, `event`, `timestamp`,
//!    `session_id`, `cwd`, `tool_name`, `tool_input`, `tool_response`,
//!    `error`, `prompt`, `agent_type`, `notification_type`,
//!    `notification_message`, `extra`) resolve directly off `EventMeta`.

use std::collections::HashMap;
use std::path::Path;

use darkmatter::markdown::compose::expression::{CtxLookup, EvaluationLookup, ResolutionContext};
use serde_json::{Map, Value};

use crate::events::EventMeta;

/// Adapter exposing an [`EventMeta`] to Darkmatter's expression evaluator.
///
/// The lookup borrows the underlying [`EventMeta`] for the duration of an
/// expression evaluation; clone the meta if the lookup needs to outlive
/// the original reference.
///
/// ## Examples
///
/// ```
/// use claudine::dispatch::expression::EventMetaExpressionLookup;
/// use claudine::events::{AgenticEvent, EventMeta};
/// use claudine::provider::Provider;
/// use darkmatter::markdown::compose::expression::EvaluationLookup;
///
/// let meta = EventMeta::new(Provider::Claude, AgenticEvent::SessionStart);
/// let lookup = EventMetaExpressionLookup::new(&meta);
/// assert_eq!(lookup.get_string("provider"), "claude");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct EventMetaExpressionLookup<'a> {
    meta: &'a EventMeta,
}

impl<'a> EventMetaExpressionLookup<'a> {
    /// Wrap an [`EventMeta`] reference for use with Darkmatter's evaluator.
    pub fn new(meta: &'a EventMeta) -> Self {
        Self { meta }
    }

    pub fn meta(&self) -> &'a EventMeta {
        self.meta
    }
}

impl<'a> EvaluationLookup for EventMetaExpressionLookup<'a> {
    fn get(&self, path: &str) -> Option<Value> {
        // Reserved `doc` namespace: bare `doc` is the whole event object;
        // `doc.<path>` resolves the underlying event field. Intercepted before
        // normal key lookup and the `ctx.*` short-circuit so a missing
        // `doc.<path>` never collapses into another namespace.
        if path == "doc" {
            return Some(event_doc_object(self.meta));
        }
        if let Some(rest) = path.strip_prefix("doc.") {
            return self.get(rest);
        }

        if let Some(env_key) = path.strip_prefix("env.") {
            return resolve_env(env_key);
        }

        if path == "ctx" || path.starts_with("ctx.") {
            return None;
        }

        if let Some(extra_path) = path.strip_prefix("extra.") {
            return resolve_extra(&self.meta.extra, extra_path);
        }

        if let Some(input_path) = path.strip_prefix("tool_input.") {
            return self
                .meta
                .tool_input
                .as_ref()
                .and_then(|value| nested_pointer(value, input_path));
        }

        if let Some(response_path) = path.strip_prefix("tool_response.") {
            return self
                .meta
                .tool_response
                .as_ref()
                .and_then(|value| nested_pointer(value, response_path));
        }

        if let Some(value) = resolve_env_path(self.meta, path) {
            return Some(value);
        }

        resolve_top_level(self.meta, path)
    }
}

/// Composite lookup used for hook `when` evaluation.
///
/// Resolves `ctx.*` via Darkmatter's lazy context capture and delegates
/// every other path to [`EventMetaExpressionLookup`] (including the reserved
/// `doc` namespace). This is the only surface where `ctx.*` is honored —
/// templates, matchers, and harness validation deliberately leave `ctx.*`
/// unresolved.
///
/// The lookup also exposes a [`ResolutionContext`] rooted at `work_dir` so
/// read-side expression functions (`file_exists`, `absolute`, `relative`, …)
/// in hook `when=` conditions resolve against the hook's base directory.
#[derive(Debug)]
pub struct EventMetaConditionLookup<'a> {
    inner: EventMetaExpressionLookup<'a>,
    ctx: CtxLookup<'a>,
    base_dir: &'a Path,
}

impl<'a> EventMetaConditionLookup<'a> {
    /// Wrap an [`EventMeta`] reference and working directory for use with
    /// Darkmatter's evaluator, including `ctx.*` resolution and a `work_dir`-
    /// rooted resolution context for read-side functions.
    pub fn new(meta: &'a EventMeta, work_dir: &'a Path) -> Self {
        Self {
            inner: EventMetaExpressionLookup::new(meta),
            ctx: CtxLookup::new(work_dir),
            base_dir: work_dir,
        }
    }

    pub fn meta(&self) -> &'a EventMeta {
        self.inner.meta()
    }
}

impl<'a> EvaluationLookup for EventMetaConditionLookup<'a> {
    fn get(&self, path: &str) -> Option<Value> {
        if path == "ctx" || path.starts_with("ctx.") {
            return self.ctx.get(path);
        }
        self.inner.get(path)
    }

    fn resolution_context(&self) -> Option<ResolutionContext> {
        Some(ResolutionContext::new(self.base_dir.to_path_buf()))
    }
}

fn resolve_env(name: &str) -> Option<Value> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    match std::env::var(trimmed) {
        Ok(value) => Some(Value::String(value)),
        Err(_) => None,
    }
}

fn resolve_extra(extra: &HashMap<String, Value>, path: &str) -> Option<Value> {
    let mut parts = path.split('.');
    let head = parts.next()?;
    if head.is_empty() {
        return None;
    }
    let mut current = extra.get(head)?.clone();
    for part in parts {
        current = match current {
            Value::Object(map) => map.get(part).cloned()?,
            _ => return None,
        };
    }
    Some(current)
}

fn nested_pointer(value: &Value, path: &str) -> Option<Value> {
    let mut current = value.clone();
    for part in path.split('.') {
        current = match current {
            Value::Object(map) => map.get(part).cloned()?,
            _ => return None,
        };
    }
    Some(current)
}

fn resolve_top_level(meta: &EventMeta, path: &str) -> Option<Value> {
    match path {
        "provider" => Some(Value::String(meta.provider.as_slug().to_string())),
        "event" => Some(Value::String(meta.event.to_string())),
        "timestamp" => Some(Value::String(meta.timestamp.to_rfc3339())),
        "session_id" => meta.session_id.clone().map(Value::String),
        "cwd" => meta.cwd.clone().map(Value::String),
        "tool_name" => meta.tool_name.clone().map(Value::String),
        "tool_input" => meta.tool_input.clone(),
        "tool_response" => meta.tool_response.clone(),
        "error" => meta.error.clone().map(Value::String),
        "prompt" => meta.prompt.clone().map(Value::String),
        "agent_type" => meta.agent_type.clone().map(Value::String),
        "notification_type" => meta.notification_type.clone().map(Value::String),
        "notification_message" => meta.notification_message.clone().map(Value::String),
        "extra" => Some(extra_as_value(&meta.extra)),
        _ => None,
    }
}

/// Build the reserved `doc` object for the event surface: every resolvable
/// top-level event field as a JSON object. This mirrors darkmatter's `doc`
/// namespace (the whole root object) where the event payload is the document.
fn event_doc_object(meta: &EventMeta) -> Value {
    const TOP_LEVEL_KEYS: [&str; 14] = [
        "provider",
        "event",
        "timestamp",
        "session_id",
        "cwd",
        "tool_name",
        "tool_input",
        "tool_response",
        "error",
        "prompt",
        "agent_type",
        "notification_type",
        "notification_message",
        "extra",
    ];
    let mut map = Map::new();
    for key in TOP_LEVEL_KEYS {
        if let Some(value) = resolve_top_level(meta, key) {
            map.insert(key.to_string(), value);
        }
    }
    Value::Object(map)
}

fn extra_as_value(extra: &HashMap<String, Value>) -> Value {
    let mut map = Map::with_capacity(extra.len());
    for (key, value) in extra {
        map.insert(key.clone(), value.clone());
    }
    Value::Object(map)
}

fn resolve_env_path(meta: &EventMeta, path: &str) -> Option<Value> {
    match path {
        // OS
        "os.name" => Some(Value::String(meta.env.os.name.clone())),
        "os.type" => Some(Value::String(meta.env.os.os_type.clone())),
        "os.version" => Some(Value::String(meta.env.os.version.clone())),
        "os.hostname" => Some(Value::String(meta.env.os.hostname.clone())),

        // Hardware
        "hardware.arch" => Some(Value::String(meta.env.hardware.arch.clone())),
        "hardware.cpu" => Some(Value::String(meta.env.hardware.cpu.clone())),
        "hardware.cores" => Some(Value::Number(meta.env.hardware.cores.into())),

        // Git
        "git.branch" => meta
            .env
            .git
            .as_ref()
            .and_then(|g| g.branch.clone())
            .map(Value::String),
        "git.is_dirty" => meta.env.git.as_ref().map(|g| Value::Bool(g.is_dirty)),
        "git.head_sha" => meta
            .env
            .git
            .as_ref()
            .and_then(|g| g.head_sha.clone())
            .map(Value::String),
        "git.head_message" => meta
            .env
            .git
            .as_ref()
            .and_then(|g| g.head_message.clone())
            .map(Value::String),
        "git.remote" => meta
            .env
            .git
            .as_ref()
            .and_then(|g| g.remote_name.clone())
            .map(Value::String),
        "git.hosting" => meta
            .env
            .git
            .as_ref()
            .and_then(|g| g.hosting_provider.clone())
            .map(Value::String),
        "git.repo_name" => meta
            .env
            .git
            .as_ref()
            .and_then(|g| g.repo_name.clone())
            .map(Value::String),
        "git.repo_org" => meta
            .env
            .git
            .as_ref()
            .and_then(|g| g.repo_org.clone())
            .map(Value::String),

        // Project
        "project.language" => meta.env.primary_language.clone().map(Value::String),
        "project.is_monorepo" => meta.env.repo.as_ref().map(|r| Value::Bool(r.is_monorepo)),
        "project.monorepo_tool" => meta
            .env
            .repo
            .as_ref()
            .and_then(|r| r.monorepo_tool.clone())
            .map(Value::String),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{
        AgenticEvent, EnvironmentContext, EventMeta, GitContext, HardwareContext, OsContext,
        RepoContext,
    };
    use crate::provider::Provider;
    use chrono::{TimeZone, Utc};
    use darkmatter::markdown::compose::expression::{ParseMode, Parser, evaluate};
    use serde_json::json;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn parse_and_evaluate<L: EvaluationLookup>(
        expression: &str,
        mode: ParseMode,
        lookup: &L,
    ) -> Value {
        let mut parser = Parser::with_mode(expression, mode).expect("parser should construct");
        let parsed = parser.parse().expect("expression should parse");
        evaluate(&parsed, lookup).expect("expression should evaluate")
    }

    fn sample_meta() -> EventMeta {
        let mut extra: HashMap<String, Value> = HashMap::new();
        extra.insert("status".to_string(), json!("success"));
        extra.insert("attempt".to_string(), json!(3));
        extra.insert(
            "tags".to_string(),
            json!(["fast", "urgent", "infrastructure"]),
        );
        extra.insert("nested".to_string(), json!({"inner": {"depth": 7}}));

        EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::BeforeTool,
            timestamp: Utc.with_ymd_and_hms(2026, 4, 29, 10, 30, 0).unwrap(),
            session_id: Some("abc123".to_string()),
            cwd: Some("/tmp/project".to_string()),
            tool_name: Some("Bash".to_string()),
            tool_input: Some(json!({
                "command": "npm test",
                "options": {"cwd": "/repo", "timeout": 60}
            })),
            tool_response: Some(json!({"exit_code": 0})),
            error: None,
            prompt: None,
            agent_type: None,
            notification_type: None,
            notification_message: None,
            extra,
            env: EnvironmentContext {
                os: OsContext {
                    os_type: "macos".to_string(),
                    name: "macOS".to_string(),
                    version: "15.3".to_string(),
                    kernel: "Darwin 25.3.0".to_string(),
                    hostname: "test-host".to_string(),
                    linux_family: None,
                    package_managers: vec!["brew".to_string()],
                },
                hardware: HardwareContext {
                    arch: "aarch64".to_string(),
                    cpu: "Apple M4 Max".to_string(),
                    cores: 16,
                    memory_bytes: 68_719_476_736,
                    memory_available_bytes: 34_359_738_368,
                },
                git: Some(GitContext {
                    repo_root: PathBuf::from("/tmp/project"),
                    branch: Some("main".to_string()),
                    is_dirty: true,
                    staged_count: 2,
                    unstaged_count: 1,
                    untracked_count: 0,
                    head_sha: Some("abc123def".to_string()),
                    head_message: Some("feat: add feature".to_string()),
                    user_name: Some("Test User".to_string()),
                    user_email: None,
                    remote_name: Some("origin".to_string()),
                    remote_url: None,
                    hosting_provider: Some("github".to_string()),
                    repo_name: Some("rusty-biscuit".to_string()),
                    repo_org: Some("anthropics".to_string()),
                }),
                repo: Some(RepoContext {
                    is_monorepo: true,
                    monorepo_tool: Some("cargo_workspace".to_string()),
                    root: PathBuf::from("/tmp/project"),
                    packages: vec!["lib".to_string(), "cli".to_string()],
                }),
                primary_language: Some("Rust".to_string()),
                package_area: Some("claudine".to_string()),
                package: Some("claudine-cli".to_string()),
                claudine_pid: None,
            },
            agent_pid: None,
        }
    }

    #[test]
    fn resolves_top_level_event_fields() {
        let meta = sample_meta();
        let lookup = EventMetaExpressionLookup::new(&meta);

        assert_eq!(lookup.get("provider"), Some(json!("claude")));
        assert_eq!(lookup.get("event"), Some(json!("before_tool")));
        assert_eq!(lookup.get("session_id"), Some(json!("abc123")));
        assert_eq!(lookup.get("cwd"), Some(json!("/tmp/project")));
        assert_eq!(lookup.get("tool_name"), Some(json!("Bash")));
        assert_eq!(lookup.get("error"), None);
        assert_eq!(lookup.get_string("error"), "");
    }

    #[test]
    fn resolves_grouped_environment_paths() {
        let meta = sample_meta();
        let lookup = EventMetaExpressionLookup::new(&meta);

        assert_eq!(lookup.get("os.type"), Some(json!("macos")));
        assert_eq!(lookup.get("hardware.cores"), Some(json!(16)));
        assert_eq!(lookup.get("git.branch"), Some(json!("main")));
        assert_eq!(lookup.get("git.is_dirty"), Some(json!(true)));
        assert_eq!(lookup.get("git.repo_name"), Some(json!("rusty-biscuit")));
        assert_eq!(lookup.get("project.language"), Some(json!("Rust")));
        assert_eq!(lookup.get("project.is_monorepo"), Some(json!(true)));
        assert_eq!(
            lookup.get("project.monorepo_tool"),
            Some(json!("cargo_workspace"))
        );
    }

    #[test]
    fn missing_keys_return_none() {
        let meta = sample_meta();
        let lookup = EventMetaExpressionLookup::new(&meta);

        assert_eq!(lookup.get("missing"), None);
        assert_eq!(lookup.get("git.unknown"), None);
        assert_eq!(lookup.get("ctx.today"), None);
        assert_eq!(lookup.get_string("missing"), "");
    }

    #[test]
    fn extra_paths_resolve_with_typed_values() {
        let meta = sample_meta();
        let lookup = EventMetaExpressionLookup::new(&meta);

        assert_eq!(lookup.get("extra.status"), Some(json!("success")));
        assert_eq!(lookup.get("extra.attempt"), Some(json!(3)));
        assert_eq!(
            lookup.get("extra.nested.inner.depth"),
            Some(json!(7)),
            "nested extra paths should drill into JSON objects"
        );
        assert_eq!(
            lookup.get("extra.tags"),
            Some(json!(["fast", "urgent", "infrastructure"]))
        );
        assert_eq!(lookup.get("extra.missing"), None);
    }

    #[test]
    fn tool_input_paths_resolve_nested_json() {
        let meta = sample_meta();
        let lookup = EventMetaExpressionLookup::new(&meta);

        assert_eq!(lookup.get("tool_input.command"), Some(json!("npm test")));
        assert_eq!(
            lookup.get("tool_input.options.cwd"),
            Some(json!("/repo")),
            "nested tool_input paths should drill in"
        );
        assert_eq!(lookup.get("tool_input.options.timeout"), Some(json!(60)));
        assert_eq!(lookup.get("tool_input.missing"), None);
    }

    #[test]
    fn env_namespace_resolves_via_std_env() {
        let key = "CLAUDINE_EXPRESSION_TEST_PRESENT";
        // SAFETY: tests run sequentially within this module by default; the
        // env var is unique per test scope and is removed before exit.
        unsafe {
            std::env::set_var(key, "value-here");
        }
        let meta = sample_meta();
        let lookup = EventMetaExpressionLookup::new(&meta);
        let resolved = lookup.get(&format!("env.{key}"));
        unsafe {
            std::env::remove_var(key);
        }
        assert_eq!(resolved, Some(json!("value-here")));
    }

    #[test]
    fn env_namespace_returns_none_for_missing_var() {
        let meta = sample_meta();
        let lookup = EventMetaExpressionLookup::new(&meta);

        assert_eq!(
            lookup.get("env.CLAUDINE_EXPRESSION_TEST_DEFINITELY_MISSING"),
            None,
        );
    }

    #[test]
    fn ctx_namespace_is_unresolved() {
        let meta = sample_meta();
        let lookup = EventMetaExpressionLookup::new(&meta);

        assert_eq!(lookup.get("ctx.today"), None);
        assert_eq!(lookup.get("ctx"), None);
    }

    #[test]
    fn evaluator_handles_fallback_for_missing_env_var() {
        let meta = sample_meta();
        let lookup = EventMetaExpressionLookup::new(&meta);

        let value = parse_and_evaluate(
            "env.CLAUDINE_EXPRESSION_TEST_FALLBACK || \"local\"",
            ParseMode::Interpolation,
            &lookup,
        );

        assert_eq!(value, json!("local"));
    }

    #[test]
    fn evaluator_handles_ternary_on_git_is_dirty() {
        let meta = sample_meta();
        let lookup = EventMetaExpressionLookup::new(&meta);

        let value = parse_and_evaluate(
            "git.is_dirty ? \"dirty\" : \"clean\"",
            ParseMode::Interpolation,
            &lookup,
        );

        assert_eq!(value, json!("dirty"));
    }

    #[test]
    fn evaluator_handles_numeric_comparison_on_hardware_cores() {
        let meta = sample_meta();
        let lookup = EventMetaExpressionLookup::new(&meta);

        let value = parse_and_evaluate(
            "hardware.cores > 8 ? \"fast\" : \"slow\"",
            ParseMode::Interpolation,
            &lookup,
        );

        assert_eq!(value, json!("fast"));
    }

    #[test]
    fn evaluator_supports_length_helper_on_extra_array() {
        let meta = sample_meta();
        let lookup = EventMetaExpressionLookup::new(&meta);

        let value = parse_and_evaluate(
            "length(extra.tags) > 2 ? \"many\" : \"few\"",
            ParseMode::Interpolation,
            &lookup,
        );

        assert_eq!(value, json!("many"));
    }

    #[test]
    fn evaluator_handles_string_equality_in_condition_mode() {
        let meta = sample_meta();
        let lookup = EventMetaExpressionLookup::new(&meta);

        let value = parse_and_evaluate(
            "tool_name == 'Bash' && git.branch == 'main'",
            ParseMode::Condition,
            &lookup,
        );

        assert_eq!(value, json!(true));
    }

    #[test]
    fn evaluator_treats_missing_optional_field_as_falsy() {
        let mut meta = sample_meta();
        meta.tool_name = None;
        let lookup = EventMetaExpressionLookup::new(&meta);

        // Interpolation fallback: tool_name is None -> Null -> falsy -> "default"
        let value = parse_and_evaluate(
            "tool_name || \"default\"",
            ParseMode::Interpolation,
            &lookup,
        );
        assert_eq!(value, json!("default"));
    }

    #[test]
    fn evaluator_handles_unary_not_on_git_is_dirty() {
        let mut meta = sample_meta();
        if let Some(git) = meta.env.git.as_mut() {
            git.is_dirty = false;
        }
        let lookup = EventMetaExpressionLookup::new(&meta);

        let value = parse_and_evaluate(
            "provider == 'claude' && !git.is_dirty",
            ParseMode::Condition,
            &lookup,
        );

        assert_eq!(value, json!(true));
    }

    // ------------------------------------------------------------------
    // EventMetaConditionLookup parity + ctx tests
    // ------------------------------------------------------------------

    #[test]
    fn condition_lookup_resolves_top_level_event_fields() {
        let meta = sample_meta();
        let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

        assert_eq!(lookup.get("provider"), Some(json!("claude")));
        assert_eq!(lookup.get("event"), Some(json!("before_tool")));
        assert_eq!(lookup.get("session_id"), Some(json!("abc123")));
        assert_eq!(lookup.get("cwd"), Some(json!("/tmp/project")));
        assert_eq!(lookup.get("tool_name"), Some(json!("Bash")));
        assert_eq!(lookup.get("error"), None);
        assert_eq!(lookup.get_string("error"), "");
    }

    #[test]
    fn condition_lookup_resolves_grouped_environment_paths() {
        let meta = sample_meta();
        let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

        assert_eq!(lookup.get("os.type"), Some(json!("macos")));
        assert_eq!(lookup.get("hardware.cores"), Some(json!(16)));
        assert_eq!(lookup.get("git.branch"), Some(json!("main")));
        assert_eq!(lookup.get("git.is_dirty"), Some(json!(true)));
        assert_eq!(lookup.get("git.repo_name"), Some(json!("rusty-biscuit")));
        assert_eq!(lookup.get("project.language"), Some(json!("Rust")));
        assert_eq!(lookup.get("project.is_monorepo"), Some(json!(true)));
        assert_eq!(
            lookup.get("project.monorepo_tool"),
            Some(json!("cargo_workspace"))
        );
    }

    #[test]
    fn condition_lookup_missing_keys_return_none() {
        let meta = sample_meta();
        let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

        assert_eq!(lookup.get("missing"), None);
        assert_eq!(lookup.get("git.unknown"), None);
        assert_eq!(lookup.get_string("missing"), "");
    }

    #[test]
    fn condition_lookup_extra_paths_resolve_with_typed_values() {
        let meta = sample_meta();
        let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

        assert_eq!(lookup.get("extra.status"), Some(json!("success")));
        assert_eq!(lookup.get("extra.attempt"), Some(json!(3)));
        assert_eq!(
            lookup.get("extra.nested.inner.depth"),
            Some(json!(7)),
            "nested extra paths should drill into JSON objects"
        );
        assert_eq!(
            lookup.get("extra.tags"),
            Some(json!(["fast", "urgent", "infrastructure"]))
        );
        assert_eq!(lookup.get("extra.missing"), None);
    }

    #[test]
    fn condition_lookup_tool_input_paths_resolve_nested_json() {
        let meta = sample_meta();
        let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

        assert_eq!(lookup.get("tool_input.command"), Some(json!("npm test")));
        assert_eq!(
            lookup.get("tool_input.options.cwd"),
            Some(json!("/repo")),
            "nested tool_input paths should drill in"
        );
        assert_eq!(lookup.get("tool_input.options.timeout"), Some(json!(60)));
        assert_eq!(lookup.get("tool_input.missing"), None);
    }

    #[test]
    fn condition_lookup_env_namespace_resolves_via_std_env() {
        let key = "CLAUDINE_CONDITION_TEST_PRESENT";
        // SAFETY: tests run sequentially within this module by default; the
        // env var is unique per test scope and is removed before exit.
        unsafe {
            std::env::set_var(key, "value-here");
        }
        let meta = sample_meta();
        let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));
        let resolved = lookup.get(&format!("env.{key}"));
        unsafe {
            std::env::remove_var(key);
        }
        assert_eq!(resolved, Some(json!("value-here")));
    }

    #[test]
    fn condition_lookup_ctx_today_resolves() {
        let meta = sample_meta();
        let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

        let value = lookup.get("ctx.today");
        assert!(
            value.is_some(),
            "ctx.today should resolve through the composite"
        );
        if let Some(Value::String(s)) = value {
            assert!(!s.is_empty(), "ctx.today should be non-empty");
        } else {
            panic!("ctx.today should return Value::String, got {:?}", value);
        }
    }

    #[test]
    fn condition_lookup_ctx_year_resolves() {
        let meta = sample_meta();
        let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

        let value = lookup.get("ctx.year");
        assert!(
            value.is_some(),
            "ctx.year should resolve through the composite"
        );
        if let Some(Value::String(s)) = value {
            assert!(!s.is_empty(), "ctx.year should be non-empty");
        } else {
            panic!("ctx.year should return Value::String, got {:?}", value);
        }
    }

    #[test]
    fn condition_lookup_falls_through_to_inner() {
        let meta = sample_meta();
        let inner = EventMetaExpressionLookup::new(&meta);
        let composite = EventMetaConditionLookup::new(&meta, Path::new("."));

        // Every non-ctx path should be identical between inner and composite.
        let paths = [
            "provider",
            "event",
            "session_id",
            "cwd",
            "tool_name",
            "tool_input",
            "tool_response",
            "error",
            "extra.status",
            "extra.attempt",
            "extra.nested.inner.depth",
            "tool_input.command",
            "tool_input.options.cwd",
            "os.type",
            "hardware.cores",
            "git.branch",
            "git.is_dirty",
            "project.language",
            "project.is_monorepo",
        ];

        for path in paths {
            assert_eq!(
                composite.get(path),
                inner.get(path),
                "path '{}' should fall through to inner unchanged",
                path
            );
        }
    }

    #[test]
    fn condition_lookup_ctx_short_circuit() {
        // Create a meta where the inner lookup would theoretically produce a
        // value for a fictional ctx.x path (it won't, because ctx.* is
        // hard-coded to None in EventMetaExpressionLookup). The important
        // invariant is that the composite returns None for unknown ctx keys
        // because CtxLookup does not recognise them, and never reaches the
        // inner adapter.
        let meta = sample_meta();
        let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

        assert_eq!(lookup.get("ctx.unknown_key"), None);
        assert_eq!(lookup.get("ctx"), None);
    }

    #[test]
    fn condition_lookup_evaluator_handles_string_equality_in_condition_mode() {
        let meta = sample_meta();
        let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

        let value = parse_and_evaluate(
            "tool_name == 'Bash' && git.branch == 'main'",
            ParseMode::Condition,
            &lookup,
        );

        assert_eq!(value, json!(true));
    }

    #[test]
    fn condition_lookup_evaluator_handles_numeric_comparison_on_hardware_cores() {
        let meta = sample_meta();
        let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

        let value = parse_and_evaluate(
            "hardware.cores > 8 ? \"fast\" : \"slow\"",
            ParseMode::Interpolation,
            &lookup,
        );

        assert_eq!(value, json!("fast"));
    }

    #[test]
    fn doc_namespace_resolves_event_surface() {
        let meta = sample_meta();
        let lookup = EventMetaExpressionLookup::new(&meta);

        // doc.<path> resolves the underlying event field.
        assert_eq!(lookup.get("doc.tool_name"), Some(json!("Bash")));
        assert_eq!(lookup.get("doc.git.branch"), Some(json!("main")));
        assert_eq!(lookup.get("doc.extra.status"), Some(json!("success")));

        // bare doc returns the whole event object.
        let obj = lookup.get("doc").expect("bare doc resolves");
        assert!(obj.is_object());
        assert_eq!(obj.get("provider"), Some(&json!("claude")));
        assert_eq!(obj.get("tool_name"), Some(&json!("Bash")));

        // missing doc.* values do not fall back to another namespace.
        assert_eq!(lookup.get("doc.missing"), None);
    }

    #[test]
    fn condition_lookup_doc_namespace_resolves() {
        let meta = sample_meta();
        let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

        assert_eq!(lookup.get("doc.tool_name"), Some(json!("Bash")));
        assert!(lookup.get("doc").is_some_and(|value| value.is_object()));
    }

    #[test]
    fn condition_lookup_read_side_function_resolves_against_base_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("artifact"), "ready").unwrap();
        let meta = sample_meta();
        let lookup = EventMetaConditionLookup::new(&meta, dir.path());

        assert!(lookup.resolution_context().is_some());
        assert_eq!(
            parse_and_evaluate("file_exists('artifact')", ParseMode::Condition, &lookup),
            json!(true)
        );
        assert_eq!(
            parse_and_evaluate("file_exists('missing')", ParseMode::Condition, &lookup),
            json!(false)
        );
    }

    #[test]
    fn condition_lookup_evaluator_supports_length_helper_on_extra_array() {
        let meta = sample_meta();
        let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

        let value = parse_and_evaluate(
            "length(extra.tags) > 2 ? \"many\" : \"few\"",
            ParseMode::Interpolation,
            &lookup,
        );

        assert_eq!(value, json!("many"));
    }
}
