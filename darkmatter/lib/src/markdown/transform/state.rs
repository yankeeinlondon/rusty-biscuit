//! Effective state resolution for transform pipeline.
//!
//! This module provides helpers for merging frontmatter with external state
//! to produce the effective state used by replacement and interpolation stages.

use super::super::frontmatter::MergeStrategy;
use super::types::TransformContext;
use serde_json::Value;
use std::collections::HashMap;

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
    context: TransformContext,
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
        context: TransformContext,
    ) -> Self {
        let mut data = frontmatter.clone();

        // Merge external state if present
        if let Some(external) = external_state
            && let Some(obj) = external.as_object()
        {
            for (key, value) in obj {
                // PreferExternal: always overwrite
                data.insert(key.clone(), value.clone());
            }
        }

        Self { data, context }
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

        // Handle nested path or simple key
        self.get_nested_value(path)
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
    pub fn context(&self) -> &TransformContext {
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
    fn get_context_value(&self, key: &str) -> Option<Value> {
        let value = match key {
            "now" => &self.context.now,
            "utc" => &self.context.utc,
            "today" => &self.context.today,
            "yesterday" => &self.context.yesterday,
            "tomorrow" => &self.context.tomorrow,
            "dow" => &self.context.dow,
            "dow_abbr" => &self.context.dow_abbr,
            "year" => &self.context.year,
            "month" => &self.context.month,
            "month_name" => &self.context.month_name,
            "month_name_abbr" => &self.context.month_name_abbr,
            _ => return None,
        };
        Some(Value::String(value.clone()))
    }

    /// Gets an environment variable value.
    fn get_env_value(&self, key: &str) -> Option<Value> {
        self.context.env.get(key).map(|v| Value::String(v.clone()))
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
    context: Option<TransformContext>,
}

impl EffectiveStateBuilder {
    /// Creates a new builder with empty frontmatter.
    pub fn new() -> Self {
        Self {
            frontmatter: HashMap::new(),
            external_state: None,
            merge_strategy: MergeStrategy::PreferExternal,
            context: None,
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

    /// Sets the runtime context.
    #[must_use]
    pub fn with_context(mut self, context: TransformContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Builds the effective state.
    pub fn build(self) -> EffectiveState {
        let context = self.context.unwrap_or_else(TransformContext::capture);

        let mut data = self.frontmatter;

        // Merge external state based on strategy
        if let Some(external) = &self.external_state
            && let Some(obj) = external.as_object()
        {
            for (key, value) in obj {
                match self.merge_strategy {
                    MergeStrategy::PreferExternal => {
                        data.insert(key.clone(), value.clone());
                    }
                    MergeStrategy::PreferDocument => {
                        data.entry(key.clone()).or_insert_with(|| value.clone());
                    }
                    MergeStrategy::ErrorOnConflict => {
                        // For building state, we use PreferExternal on conflict
                        // (error handling is at a higher level)
                        data.insert(key.clone(), value.clone());
                    }
                }
            }
        }

        EffectiveState { data, context }
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

    fn test_context() -> TransformContext {
        TransformContext::fixed_for_testing()
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
        assert_eq!(state.get("ctx.dow"), Some(json!("Saturday")));
        assert_eq!(state.get("ctx.unknown"), None);
    }

    #[test]
    fn test_effective_state_env_lookup() {
        let mut ctx = test_context();
        ctx.env.insert("HOME".to_string(), "/home/user".to_string());
        ctx.env.insert("USER".to_string(), "alice".to_string());

        let fm = HashMap::new();
        let state = EffectiveState::new(&fm, None, ctx);

        assert_eq!(state.get("env.HOME"), Some(json!("/home/user")));
        assert_eq!(state.get("env.USER"), Some(json!("alice")));
        assert_eq!(state.get("env.MISSING"), None);
    }

    #[test]
    fn test_effective_state_external_overrides() {
        let mut fm = HashMap::new();
        fm.insert("title".to_string(), json!("Original"));
        fm.insert("author".to_string(), json!("Alice"));

        let external = json!({
            "title": "Overridden",
            "new_key": "New Value"
        });

        let state = EffectiveState::new(&fm, Some(&external), test_context());

        // External wins on conflict
        assert_eq!(state.get("title"), Some(json!("Overridden")));
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
            .build();

        assert!(state.data().is_empty());
    }

    #[test]
    fn test_builder_with_frontmatter() {
        let mut fm = HashMap::new();
        fm.insert("key".to_string(), json!("value"));

        let state = EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_context(test_context())
            .build();

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
            .build();

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
            .build();

        // External wins with PreferExternal
        assert_eq!(state.get("key"), Some(json!("external")));
    }
}
