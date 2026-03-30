//! Effective state resolution for compose pipeline.
//!
//! This module provides helpers for merging frontmatter with external state
//! to produce the effective state used by replacement and interpolation stages.

use super::super::frontmatter::MergeStrategy;
use super::context::ContextMergeDiagnostic;
use super::context::merge::CtxMergeError;
use super::types::ComposeContext;
use serde_json::Value;
use std::collections::HashMap;

/// Deeply merges two JSON values.
///
/// `overlay` wins for non-object conflicts. When both values are objects,
/// keys are merged recursively.
pub(crate) fn deep_merge(base: &Value, overlay: &Value) -> Value {
    match (base, overlay) {
        (Value::Object(base_obj), Value::Object(overlay_obj)) => {
            let mut merged = base_obj.clone();
            for (key, overlay_value) in overlay_obj {
                let next = match merged.get(key) {
                    Some(base_value) => deep_merge(base_value, overlay_value),
                    None => overlay_value.clone(),
                };
                merged.insert(key.clone(), next);
            }
            Value::Object(merged)
        }
        // Null overlay means "not set" — preserve the base value.
        (_, Value::Null) => base.clone(),
        _ => overlay.clone(),
    }
}

/// Merges two `replace` maps with deep merge semantics.
///
/// Values from `overlay` take precedence on conflict.
pub(crate) fn merge_replace_maps(
    base: Option<&serde_json::Map<String, Value>>,
    overlay: Option<&serde_json::Map<String, Value>>,
) -> serde_json::Map<String, Value> {
    let base = base.cloned().unwrap_or_default();
    let overlay = overlay.cloned().unwrap_or_default();
    match deep_merge(&Value::Object(base), &Value::Object(overlay)) {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    }
}

/// Resolved state available to replacement and interpolation stages.
///
/// This struct holds the merged result of:
/// 1. Document frontmatter
/// 2. External state (if provided)
/// 3. Runtime context (accessible via `ctx.*` paths)
/// 4. Environment variables (accessible via `env.*` paths)
#[derive(Debug, Clone)]
pub struct EffectiveState {
    /// Merged frontmatter + external state.
    data: HashMap<String, Value>,

    /// Runtime context for `ctx.*` lookups.
    context: ComposeContext,

    /// Diagnostics from context capture and merge.
    ctx_diagnostics: Vec<ContextMergeDiagnostic>,
}

impl EffectiveState {
    /// Creates effective state by merging frontmatter with optional external state.
    ///
    /// ## Merge Behavior
    ///
    /// Uses `PreferExternal` strategy: external state values override
    /// frontmatter values when keys conflict.
    pub fn new(
        frontmatter: &HashMap<String, Value>,
        external_state: Option<&Value>,
        context: ComposeContext,
    ) -> Self {
        let data = match external_state.and_then(Value::as_object) {
            Some(external_obj) => {
                let merged = deep_merge(
                    &Value::Object(external_obj.clone()),
                    &Value::Object(frontmatter.clone().into_iter().collect()),
                );
                match merged {
                    Value::Object(obj) => obj.into_iter().collect(),
                    _ => frontmatter.clone(),
                }
            }
            None => frontmatter.clone(),
        };

        Self {
            data,
            context,
            ctx_diagnostics: Vec::new(),
        }
    }

    /// Returns context merge diagnostics.
    pub fn ctx_diagnostics(&self) -> &[ContextMergeDiagnostic] {
        &self.ctx_diagnostics
    }

    /// Looks up a value by path.
    ///
    /// Supports:
    /// - Simple keys: `"title"` -> frontmatter/state lookup
    /// - Nested keys: `"user.name"` -> nested object lookup
    /// - Context keys: `"ctx.today"` -> runtime context
    /// - Environment keys: `"env.HOME"` -> environment variable
    ///
    /// Returns `None` if the key doesn't exist or the path is invalid.
    pub fn get(&self, path: &str) -> Option<Value> {
        // Handle special prefixes
        if let Some(ctx_key) = path.strip_prefix("ctx.") {
            return self.get_context_value(ctx_key);
        }

        if let Some(env_key) = path.strip_prefix("env.") {
            return self.get_env_value(env_key);
        }

        // Handle nested path or simple key in frontmatter, then fall back
        // to the ctx namespace so that `when="repo"` resolves `ctx.repo`
        // when `repo` isn't a frontmatter key.
        self.get_nested_value(path)
            .or_else(|| self.get_context_value(path))
    }

    /// Gets a string value, coercing types as needed.
    ///
    /// - `null` -> `""`
    /// - `string` -> the string
    /// - `number` -> string representation
    /// - `bool` -> `"true"` or `"false"`
    /// - `array/object` -> JSON string
    pub fn get_string(&self, path: &str) -> String {
        match self.get(path) {
            None => String::new(),
            Some(Value::Null) => String::new(),
            Some(Value::String(s)) => s,
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::Bool(b)) => b.to_string(),
            Some(v) => v.to_string(),
        }
    }

    /// Returns the underlying data map.
    pub fn data(&self) -> &HashMap<String, Value> {
        &self.data
    }

    /// Returns the runtime context.
    pub fn context(&self) -> &ComposeContext {
        &self.context
    }

    /// Returns the `replace` key as a map if present and valid.
    ///
    /// Returns `None` if:
    /// - The `replace` key doesn't exist
    /// - The `replace` value is not a JSON object/map
    pub fn get_replace_map(&self) -> Option<&serde_json::Map<String, Value>> {
        self.data.get("replace").and_then(|v| v.as_object())
    }

    /// Gets a context value by key.
    ///
    /// First tries the materialized `ctx` namespace in `data`, then falls back
    /// to the `values` map on `ComposeContext`, then to legacy field-by-field match.
    fn get_context_value(&self, key: &str) -> Option<Value> {
        // 1. Try materialized data["ctx"][key]
        if let Some(ctx_obj) = self.data.get("ctx")
            && let Some(val) = ctx_obj.get(key)
        {
            return Some(val.clone());
        }

        // 2. Try values map on ComposeContext
        if let Some(val) = self.context.get(key) {
            return Some(val.clone());
        }

        // 3. Legacy field-by-field fallback (safety during transition)
        let value = match key {
            "now" => self.context.now(),
            "now_utc" => self.context.now_utc(),
            "today" => self.context.today(),
            "yesterday" => self.context.yesterday(),
            "tomorrow" => self.context.tomorrow(),
            "day" => self.context.day(),
            "day_abbr" => self.context.day_abbr(),
            "year" => self.context.year(),
            "month" => self.context.month(),
            "month_name" => self.context.month_name(),
            "month_name_abbr" => self.context.month_name_abbr(),
            _ => return None,
        };
        Some(Value::String(value.to_string()))
    }

    /// Gets an environment variable value.
    fn get_env_value(&self, key: &str) -> Option<Value> {
        self.context.env().get(key).map(|v| Value::String(v.clone()))
    }

    /// Gets a value from nested data using dot notation.
    fn get_nested_value(&self, path: &str) -> Option<Value> {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() {
            return None;
        }

        // Start with the first key
        let mut current = self.data.get(parts[0])?.clone();

        // Navigate nested path
        for part in &parts[1..] {
            current = match current {
                Value::Object(obj) => obj.get(*part)?.clone(),
                _ => return None,
            };
        }

        Some(current)
    }
}

/// Builder for creating effective state with specific merge strategies.
#[derive(Debug, Clone)]
pub struct EffectiveStateBuilder {
    frontmatter: HashMap<String, Value>,
    external_state: Option<Value>,
    merge_strategy: MergeStrategy,
    replace_parent_wins: bool,
    context: Option<ComposeContext>,
    allow_ctx_override: bool,
}

impl EffectiveStateBuilder {
    /// Creates a new builder with empty frontmatter.
    pub fn new() -> Self {
        Self {
            frontmatter: HashMap::new(),
            external_state: None,
            merge_strategy: MergeStrategy::PreferExternal,
            replace_parent_wins: false,
            context: None,
            allow_ctx_override: false,
        }
    }

    /// Sets the frontmatter data.
    #[must_use]
    pub fn with_frontmatter(mut self, frontmatter: HashMap<String, Value>) -> Self {
        self.frontmatter = frontmatter;
        self
    }

    /// Sets the external state.
    #[must_use]
    pub fn with_external_state(mut self, state: Value) -> Self {
        self.external_state = Some(state);
        self
    }

    /// Sets the merge strategy.
    #[must_use]
    pub fn with_merge_strategy(mut self, strategy: MergeStrategy) -> Self {
        self.merge_strategy = strategy;
        self
    }

    /// Overrides replace-map precedence so external values win over document
    /// values for this build only.
    #[must_use]
    pub fn with_replace_parent_wins(mut self, enabled: bool) -> Self {
        self.replace_parent_wins = enabled;
        self
    }

    /// Sets the runtime context.
    #[must_use]
    pub fn with_context(mut self, context: ComposeContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Allow non-object ctx frontmatter (downgrade error to warning).
    #[must_use]
    pub fn with_allow_ctx_override(mut self, allow: bool) -> Self {
        self.allow_ctx_override = allow;
        self
    }

    /// Builds the effective state.
    ///
    /// After merging frontmatter/external state, materializes the runtime `ctx`
    /// namespace into `data["ctx"]` using the merge policy.
    ///
    /// ## Errors
    ///
    /// Returns `CtxMergeError::InvalidUserCtx` when the document defines `ctx`
    /// as a non-object value and `allow_ctx_override` is false.
    pub fn build(self) -> Result<EffectiveState, CtxMergeError> {
        let context = self.context.unwrap_or_else(ComposeContext::capture);

        let frontmatter_value = Value::Object(self.frontmatter.clone().into_iter().collect());
        let external_value = self
            .external_state
            .as_ref()
            .and_then(Value::as_object)
            .cloned()
            .map(Value::Object)
            .unwrap_or(Value::Object(serde_json::Map::new()));

        let merged = match self.merge_strategy {
            MergeStrategy::PreferExternal => deep_merge(&frontmatter_value, &external_value),
            MergeStrategy::PreferDocument => deep_merge(&external_value, &frontmatter_value),
            MergeStrategy::ErrorOnConflict => deep_merge(&frontmatter_value, &external_value),
        };

        let mut data: HashMap<String, Value> = match merged {
            Value::Object(obj) => obj.into_iter().collect(),
            _ => self.frontmatter.clone(),
        };

        if self.replace_parent_wins {
            let doc_replace = self.frontmatter.get("replace").and_then(Value::as_object);
            let external_replace = self
                .external_state
                .as_ref()
                .and_then(|v| v.get("replace"))
                .and_then(Value::as_object);

            if doc_replace.is_some() || external_replace.is_some() {
                let merged_replace = merge_replace_maps(doc_replace, external_replace);
                data.insert("replace".to_string(), Value::Object(merged_replace));
            }
        }

        // Materialize ctx: merge user-defined ctx with runtime ctx.
        // When allow_ctx_override is false, a non-object user ctx is a hard error.
        let mut ctx_diagnostics = Vec::new();

        // Include capture diagnostics from context
        ctx_diagnostics.extend(context.diagnostics().iter().cloned());

        let user_ctx = data.get("ctx");
        let runtime_ctx = context.as_object();

        let merge_result =
            super::context::merge_ctx(user_ctx, runtime_ctx, self.allow_ctx_override)?;
        data.insert("ctx".to_string(), merge_result.merged_ctx);
        ctx_diagnostics.extend(merge_result.diagnostics);

        Ok(EffectiveState {
            data,
            context,
            ctx_diagnostics,
        })
    }
}

impl Default for EffectiveStateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_context() -> ComposeContext {
        ComposeContext::fixed_for_testing()
    }

    #[test]
    fn test_effective_state_simple_lookup() {
        let mut fm = HashMap::new();
        fm.insert("title".to_string(), json!("Hello World"));
        fm.insert("count".to_string(), json!(42));

        let state = EffectiveState::new(&fm, None, test_context());

        assert_eq!(state.get("title"), Some(json!("Hello World")));
        assert_eq!(state.get("count"), Some(json!(42)));
        assert_eq!(state.get("missing"), None);
    }

    #[test]
    fn test_effective_state_nested_lookup() {
        let mut fm = HashMap::new();
        fm.insert(
            "user".to_string(),
            json!({
                "name": "Alice",
                "address": {
                    "city": "London"
                }
            }),
        );

        let state = EffectiveState::new(&fm, None, test_context());

        assert_eq!(state.get("user.name"), Some(json!("Alice")));
        assert_eq!(state.get("user.address.city"), Some(json!("London")));
        assert_eq!(state.get("user.missing"), None);
    }

    #[test]
    fn test_effective_state_context_lookup() {
        let fm = HashMap::new();
        let state = EffectiveState::new(&fm, None, test_context());

        assert_eq!(state.get("ctx.today"), Some(json!("2024-06-15")));
        assert_eq!(state.get("ctx.year"), Some(json!("2024")));
        assert_eq!(state.get("ctx.day"), Some(json!("Saturday")));
        assert_eq!(state.get("ctx.unknown"), None);
    }

    #[test]
    fn test_effective_state_env_lookup() {
        let mut ctx = test_context();
        ctx.env_mut().insert("HOME".to_string(), "/home/user".to_string());
        ctx.env_mut().insert("USER".to_string(), "alice".to_string());

        let fm = HashMap::new();
        let state = EffectiveState::new(&fm, None, ctx);

        assert_eq!(state.get("env.HOME"), Some(json!("/home/user")));
        assert_eq!(state.get("env.USER"), Some(json!("alice")));
        assert_eq!(state.get("env.MISSING"), None);
    }

    #[test]
    fn test_effective_state_frontmatter_overrides_external() {
        let mut fm = HashMap::new();
        fm.insert("title".to_string(), json!("Original"));
        fm.insert("author".to_string(), json!("Alice"));

        let external = json!({
            "title": "Overridden",
            "new_key": "New Value"
        });

        let state = EffectiveState::new(&fm, Some(&external), test_context());

        // Frontmatter wins on conflict (external acts as defaults)
        assert_eq!(state.get("title"), Some(json!("Original")));
        // Original preserved when no conflict
        assert_eq!(state.get("author"), Some(json!("Alice")));
        // New keys added
        assert_eq!(state.get("new_key"), Some(json!("New Value")));
    }

    #[test]
    fn test_effective_state_get_string() {
        let mut fm = HashMap::new();
        fm.insert("string".to_string(), json!("hello"));
        fm.insert("number".to_string(), json!(42));
        fm.insert("float".to_string(), json!(3.14));
        fm.insert("bool_true".to_string(), json!(true));
        fm.insert("bool_false".to_string(), json!(false));
        fm.insert("null".to_string(), json!(null));
        fm.insert("array".to_string(), json!([1, 2, 3]));

        let state = EffectiveState::new(&fm, None, test_context());

        assert_eq!(state.get_string("string"), "hello");
        assert_eq!(state.get_string("number"), "42");
        assert_eq!(state.get_string("float"), "3.14");
        assert_eq!(state.get_string("bool_true"), "true");
        assert_eq!(state.get_string("bool_false"), "false");
        assert_eq!(state.get_string("null"), "");
        assert_eq!(state.get_string("missing"), "");
        assert_eq!(state.get_string("array"), "[1,2,3]");
    }

    #[test]
    fn test_builder_default() {
        let state = EffectiveStateBuilder::new()
            .with_context(test_context())
            .build()
            .unwrap();

        // Only the materialized "ctx" key should be present (no user frontmatter)
        assert_eq!(state.data().len(), 1);
        assert!(state.data().contains_key("ctx"));
    }

    #[test]
    fn test_builder_with_frontmatter() {
        let mut fm = HashMap::new();
        fm.insert("key".to_string(), json!("value"));

        let state = EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_context(test_context())
            .build()
            .unwrap();

        assert_eq!(state.get("key"), Some(json!("value")));
    }

    #[test]
    fn test_builder_prefer_document_strategy() {
        let mut fm = HashMap::new();
        fm.insert("key".to_string(), json!("original"));

        let external = json!({"key": "external"});

        let state = EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_external_state(external)
            .with_merge_strategy(MergeStrategy::PreferDocument)
            .with_context(test_context())
            .build()
            .unwrap();

        // Document wins with PreferDocument
        assert_eq!(state.get("key"), Some(json!("original")));
    }

    #[test]
    fn test_builder_prefer_external_strategy() {
        let mut fm = HashMap::new();
        fm.insert("key".to_string(), json!("original"));

        let external = json!({"key": "external"});

        let state = EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_external_state(external)
            .with_merge_strategy(MergeStrategy::PreferExternal)
            .with_context(test_context())
            .build()
            .unwrap();

        // External wins with PreferExternal
        assert_eq!(state.get("key"), Some(json!("external")));
    }

    #[test]
    fn test_builder_replace_parent_wins_override() {
        let mut fm = HashMap::new();
        fm.insert(
            "replace".to_string(),
            json!({
                "TOKEN": "child",
                "ONLY_CHILD": "yes"
            }),
        );

        let external = json!({
            "replace": {
                "TOKEN": "parent",
                "ONLY_PARENT": "yes"
            }
        });

        let state = EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_external_state(external)
            .with_merge_strategy(MergeStrategy::PreferDocument)
            .with_replace_parent_wins(true)
            .with_context(test_context())
            .build()
            .unwrap();

        let replace = state.get_replace_map().unwrap();
        assert_eq!(replace.get("TOKEN"), Some(&json!("parent")));
        assert_eq!(replace.get("ONLY_CHILD"), Some(&json!("yes")));
        assert_eq!(replace.get("ONLY_PARENT"), Some(&json!("yes")));
    }

    #[test]
    fn test_deep_merge_nested_objects() {
        let base = json!({
            "a": 1,
            "nested": {
                "left": "x",
                "shared": {
                    "base": true
                }
            }
        });
        let overlay = json!({
            "nested": {
                "right": "y",
                "shared": {
                    "overlay": true
                }
            },
            "b": 2
        });

        let merged = deep_merge(&base, &overlay);
        assert_eq!(merged["a"], json!(1));
        assert_eq!(merged["b"], json!(2));
        assert_eq!(merged["nested"]["left"], json!("x"));
        assert_eq!(merged["nested"]["right"], json!("y"));
        assert_eq!(merged["nested"]["shared"]["base"], json!(true));
        assert_eq!(merged["nested"]["shared"]["overlay"], json!(true));
    }

    #[test]
    fn test_merge_replace_maps_child_wins() {
        let parent = serde_json::json!({
            "A": "parent",
            "nested": {
                "left": "x"
            }
        });
        let child = serde_json::json!({
            "A": "child",
            "nested": {
                "right": "y"
            }
        });
        let parent_map = parent.as_object().unwrap();
        let child_map = child.as_object().unwrap();

        let merged = merge_replace_maps(Some(parent_map), Some(child_map));
        assert_eq!(merged.get("A"), Some(&json!("child")));
        assert_eq!(merged["nested"]["left"], json!("x"));
        assert_eq!(merged["nested"]["right"], json!("y"));
    }

    // ── Phase 8: Regression tests for context materialization ─────

    #[test]
    fn test_ctx_today_resolves_via_data_lookup() {
        let fm = HashMap::new();
        let state = EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_context(test_context())
            .build()
            .unwrap();

        // ctx.today should resolve via the materialized data["ctx"] namespace
        assert_eq!(state.get("ctx.today"), Some(json!("2024-06-15")));
        assert_eq!(state.get("ctx.year"), Some(json!("2024")));
        assert_eq!(state.get("ctx.day"), Some(json!("Saturday")));
    }

    #[test]
    fn test_env_lookup_still_works() {
        let mut ctx = test_context();
        ctx.env_mut().insert("HOME".to_string(), "/home/user".to_string());

        let state = EffectiveStateBuilder::new()
            .with_context(ctx)
            .build()
            .unwrap();

        assert_eq!(state.get("env.HOME"), Some(json!("/home/user")));
        assert_eq!(state.get("env.MISSING"), None);
    }

    #[test]
    fn test_ctx_diagnostics_empty_by_default() {
        let state = EffectiveStateBuilder::new()
            .with_context(test_context())
            .build()
            .unwrap();

        // No user ctx means no diagnostics
        assert!(state.ctx_diagnostics().is_empty());
    }
}
