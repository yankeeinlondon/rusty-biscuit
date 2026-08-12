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
            return nested_pointer(&event_doc_object(self.meta), rest);
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
    let mut current = extra.get(head)?;
    for part in parts {
        current = current.as_object()?.get(part)?;
    }
    Some(current.clone())
}

fn nested_pointer(value: &Value, path: &str) -> Option<Value> {
    let mut current = value;
    for part in path.split('.') {
        current = current.as_object()?.get(part)?;
    }
    Some(current.clone())
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
/// top-level event field plus the grouped environment paths (`os`, `hardware`,
/// `git`, `project`) as a JSON object. This mirrors darkmatter's `doc`
/// namespace (the whole root object) where the event payload is the document.
/// Process environment (`env.*`) is deliberately excluded.
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
    map.insert("os".to_string(), os_doc_object(meta));
    map.insert("hardware".to_string(), hardware_doc_object(meta));
    if let Some(git) = git_doc_object(meta) {
        map.insert("git".to_string(), git);
    }
    if let Some(project) = project_doc_object(meta) {
        map.insert("project".to_string(), project);
    }
    Value::Object(map)
}

fn os_doc_object(meta: &EventMeta) -> Value {
    let mut map = Map::new();
    map.insert(
        "name".to_string(),
        resolve_env_path(meta, "os.name").unwrap_or(Value::Null),
    );
    map.insert(
        "type".to_string(),
        resolve_env_path(meta, "os.type").unwrap_or(Value::Null),
    );
    map.insert(
        "version".to_string(),
        resolve_env_path(meta, "os.version").unwrap_or(Value::Null),
    );
    map.insert(
        "hostname".to_string(),
        resolve_env_path(meta, "os.hostname").unwrap_or(Value::Null),
    );
    Value::Object(map)
}

fn hardware_doc_object(meta: &EventMeta) -> Value {
    let mut map = Map::new();
    map.insert(
        "arch".to_string(),
        resolve_env_path(meta, "hardware.arch").unwrap_or(Value::Null),
    );
    map.insert(
        "cpu".to_string(),
        resolve_env_path(meta, "hardware.cpu").unwrap_or(Value::Null),
    );
    map.insert(
        "cores".to_string(),
        resolve_env_path(meta, "hardware.cores").unwrap_or(Value::Null),
    );
    Value::Object(map)
}

fn git_doc_object(meta: &EventMeta) -> Option<Value> {
    let fields: [(&str, Option<Value>); 8] = [
        ("branch", resolve_env_path(meta, "git.branch")),
        ("is_dirty", resolve_env_path(meta, "git.is_dirty")),
        ("head_sha", resolve_env_path(meta, "git.head_sha")),
        ("head_message", resolve_env_path(meta, "git.head_message")),
        ("remote", resolve_env_path(meta, "git.remote")),
        ("hosting", resolve_env_path(meta, "git.hosting")),
        ("repo_name", resolve_env_path(meta, "git.repo_name")),
        ("repo_org", resolve_env_path(meta, "git.repo_org")),
    ];
    if fields.iter().all(|(_, value)| value.is_none()) {
        return None;
    }
    let mut map = Map::new();
    for (key, value) in fields {
        if let Some(value) = value {
            map.insert(key.to_string(), value);
        }
    }
    Some(Value::Object(map))
}

fn project_doc_object(meta: &EventMeta) -> Option<Value> {
    let fields: [(&str, Option<Value>); 3] = [
        ("language", resolve_env_path(meta, "project.language")),
        ("is_monorepo", resolve_env_path(meta, "project.is_monorepo")),
        (
            "monorepo_tool",
            resolve_env_path(meta, "project.monorepo_tool"),
        ),
    ];
    if fields.iter().all(|(_, value)| value.is_none()) {
        return None;
    }
    let mut map = Map::new();
    for (key, value) in fields {
        if let Some(value) = value {
            map.insert(key.to_string(), value);
        }
    }
    Some(Value::Object(map))
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
        "project.monorepo_standard" => meta
            .env
            .repo
            .as_ref()
            .and_then(|r| r.monorepo_standard.clone())
            .map(Value::String),
        "project.monorepo_orchestrators" => meta
            .env
            .repo
            .as_ref()
            .map(|r| Value::String(r.monorepo_orchestrators.join(", "))),
        // `project.monorepo_tool` is a deprecated alias for
        // `project.monorepo_standard`.
        "project.monorepo_tool" => meta
            .env
            .repo
            .as_ref()
            .and_then(|r| r.monorepo_standard.clone())
            .map(Value::String),

        _ => None,
    }
}

#[cfg(test)]
mod tests;
