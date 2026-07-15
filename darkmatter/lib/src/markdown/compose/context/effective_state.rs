//! Effective state resolution for the compose pipeline.
//!
//! Merges frontmatter with external state to produce the effective state used
//! by replacement, interpolation, and condition stages.

use super::runtime::ComposeContext;
use super::ContextMergeDiagnostic;
use super::merge::CtxMergeError;
use crate::markdown::compose::context::catalog::CONTEXT_VARIABLE_DESCRIPTORS;
use crate::markdown::frontmatter::MergeStrategy;
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

/// Deeply merges two JSON values, treating null in the overlay as a literal value.
///
/// Behaves like [`deep_merge`] except that an overlay of `Value::Null`
/// overrides the base (rather than being treated as "not set"). Arrays are
/// leaves: a higher layer array replaces the base array wholesale rather
/// than concatenating. This is the merge used when applying `::file`
/// directive `set=` / `set.NAME=` overrides so that callers can explicitly
/// set a frontmatter property to null.
pub(crate) fn deep_merge_override(base: &Value, overlay: &Value) -> Value {
    match (base, overlay) {
        (Value::Object(base_obj), Value::Object(overlay_obj)) => {
            let mut merged = base_obj.clone();
            for (key, overlay_value) in overlay_obj {
                let next = match merged.get(key) {
                    Some(base_value) => deep_merge_override(base_value, overlay_value),
                    None => overlay_value.clone(),
                };
                merged.insert(key.clone(), next);
            }
            Value::Object(merged)
        }
        _ => overlay.clone(),
    }
}

/// Applies three-layer frontmatter overrides from a `::file` transclusion directive.
///
/// Layers, from lowest to highest precedence:
///
/// 1. `base_fm` — the child file's authored frontmatter.
/// 2. `set_object` — the object-form payload from `set='{ … }'`, deep-merged onto the base.
/// 3. `set_properties` — each `set.NAME=<value>` pair, applied in order as single-key
///    overlays on top of the accumulator.
///
/// Object values deep-merge; arrays and scalars (including `null`) override the
/// lower layer. Ordering of `set_properties` is preserved so that last-write
/// semantics hold under permissive duplicate-handling mode.
pub(crate) fn apply_set_overrides(
    base_fm: &serde_json::Map<String, Value>,
    set_object: Option<&serde_json::Map<String, Value>>,
    set_properties: &[(String, Value)],
) -> serde_json::Map<String, Value> {
    let mut accumulator = Value::Object(base_fm.clone());

    if let Some(object_overlay) = set_object {
        accumulator = deep_merge_override(&accumulator, &Value::Object(object_overlay.clone()));
    }

    for (name, value) in set_properties {
        let mut single_key = serde_json::Map::with_capacity(1);
        single_key.insert(name.clone(), value.clone());
        accumulator = deep_merge_override(&accumulator, &Value::Object(single_key));
    }

    match accumulator {
        Value::Object(map) => map,
        _ => base_fm.clone(),
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
        // Reserved `doc` namespace — intercepted before normal key lookup and
        // before the bare-name `ctx.*` fallback, so a missing `doc.*` never
        // collapses into `ctx.*`.
        if super::super::expression::doc_namespace::is_doc_namespace(path) {
            return super::super::expression::doc_namespace::resolve_doc_namespace_in_map(
                path, &self.data,
            );
        }

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
        self.context
            .env()
            .get(key)
            .map(|v| Value::String(v.clone()))
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

impl super::super::expression::EvaluationLookup for EffectiveState {
    fn get(&self, path: &str) -> Option<Value> {
        self.get(path)
    }

    fn get_string(&self, path: &str) -> String {
        self.get_string(path)
    }

    fn is_valid_context_variable(&self, name: &str) -> bool {
        if CONTEXT_VARIABLE_DESCRIPTORS.iter().any(|d| d.name == name) {
            return true;
        }
        self.get_context_value(name).is_some()
    }

    fn context_variable_names(&self) -> &[&'static str] {
        use std::sync::LazyLock;
        static NAMES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
            CONTEXT_VARIABLE_DESCRIPTORS
                .iter()
                .map(|d| d.name)
                .collect()
        });
        NAMES.as_slice()
    }
}

/// Wraps an [`EffectiveState`] with a [`ResolutionContext`] so any surface that
/// evaluates the grammar against the effective state — body interpolation,
/// `when=` conditions, page blocks — can run the read-side expression
/// functions (`frontmatter`, `file_exists`, `markdown_title`,
/// `markdown_body_empty`, `validate_schema`, `absolute`, `relative`).
///
/// Bare `EffectiveState` returns `None` from `resolution_context()`, which
/// disables those functions; this adapter supplies the document-relative base
/// directory, magic search paths, and — when the supplied context enables remote
/// reads — the run's remote-fetch runtime so HTTP(S) URL arguments resolve from
/// the fetch cache. These body-side surfaces are the only ones that carry a
/// remote runtime; frontmatter surfaces are local-only (Decision B). `absolute`
/// and `relative` are local-only path transforms — they require the context for
/// the base directory but never touch the network.
pub(crate) struct ResolvingLookup<'a> {
    state: &'a EffectiveState,
    resolution_context: super::super::expression::ResolutionContext,
}

impl<'a> ResolvingLookup<'a> {
    pub(crate) fn new(
        state: &'a EffectiveState,
        resolution_context: super::super::expression::ResolutionContext,
    ) -> Self {
        Self {
            state,
            resolution_context,
        }
    }
}

impl super::super::expression::EvaluationLookup for ResolvingLookup<'_> {
    fn get(&self, path: &str) -> Option<Value> {
        self.state.get(path)
    }

    fn get_string(&self, path: &str) -> String {
        self.state.get_string(path)
    }

    fn resolution_context(&self) -> Option<super::super::expression::ResolutionContext> {
        Some(self.resolution_context.clone())
    }

    fn is_valid_context_variable(&self, name: &str) -> bool {
        self.state.is_valid_context_variable(name)
    }

    fn context_variable_names(&self) -> &[&'static str] {
        self.state.context_variable_names()
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
            super::merge_ctx(user_ctx, runtime_ctx, self.allow_ctx_override)?;
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
    fn doc_namespace_resolves_against_frontmatter_only() {
        let mut fm = HashMap::new();
        fm.insert("build".to_string(), json!("frontmatter-build"));
        fm.insert("config".to_string(), json!({ "retries": 3 }));
        // A frontmatter property literally named `doc`.
        fm.insert("doc".to_string(), json!({ "child": "literal-doc" }));
        let state = EffectiveState::new(&fm, None, test_context());

        // doc.<path> resolves a frontmatter property.
        assert_eq!(state.get("doc.build"), Some(json!("frontmatter-build")));
        assert_eq!(state.get("doc.config.retries"), Some(json!(3)));

        // bare doc returns the whole frontmatter object.
        let obj = state.get("doc").expect("bare doc resolves");
        assert!(obj.is_object());
        assert_eq!(obj.get("build"), Some(&json!("frontmatter-build")));

        // a literal property named `doc` is reached as doc.doc.
        assert_eq!(state.get("doc.doc"), Some(json!({ "child": "literal-doc" })));
        assert_eq!(state.get("doc.doc.child"), Some(json!("literal-doc")));

        // missing doc.* values do NOT fall back to ctx.* — `today` exists only
        // in the runtime context, never as a frontmatter property.
        assert_eq!(state.get("ctx.today"), Some(json!("2024-06-15")));
        assert_eq!(state.get("doc.today"), None);
    }

    #[test]
    fn bare_doc_resolves_when_no_doc_property_exists() {
        let mut fm = HashMap::new();
        fm.insert("title".to_string(), json!("Hello"));
        let state = EffectiveState::new(&fm, None, test_context());

        // No frontmatter property named `doc`; bare doc is still the object and
        // must not fall back to `ctx.doc`.
        let obj = state.get("doc").expect("bare doc resolves to the object");
        assert_eq!(obj.get("title"), Some(&json!("Hello")));
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
        ctx.env_mut()
            .insert("HOME".to_string(), "/home/user".to_string());
        ctx.env_mut()
            .insert("USER".to_string(), "alice".to_string());

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
        fm.insert("float".to_string(), json!(3.15));
        fm.insert("bool_true".to_string(), json!(true));
        fm.insert("bool_false".to_string(), json!(false));
        fm.insert("null".to_string(), json!(null));
        fm.insert("array".to_string(), json!([1, 2, 3]));

        let state = EffectiveState::new(&fm, None, test_context());

        assert_eq!(state.get_string("string"), "hello");
        assert_eq!(state.get_string("number"), "42");
        assert_eq!(state.get_string("float"), "3.15");
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
        ctx.env_mut()
            .insert("HOME".to_string(), "/home/user".to_string());

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

    // ── deep_merge_override (null-as-literal) ─────────────────────────

    #[test]
    fn test_deep_merge_override_null_is_literal_at_top_level() {
        let base = json!({"x": 5});
        let overlay = json!({"x": null});
        let merged = deep_merge_override(&base, &overlay);
        assert_eq!(merged, json!({"x": null}));
    }

    #[test]
    fn test_deep_merge_override_null_inside_dict_is_literal() {
        let base = json!({"author": {"name": "Alice", "handle": "@alice"}});
        let overlay = json!({"author": {"name": null}});
        let merged = deep_merge_override(&base, &overlay);
        assert_eq!(
            merged,
            json!({"author": {"name": null, "handle": "@alice"}})
        );
    }

    #[test]
    fn test_deep_merge_override_arrays_are_leaves() {
        let base = json!({"tags": ["a", "b"]});
        let overlay = json!({"tags": ["c"]});
        let merged = deep_merge_override(&base, &overlay);
        assert_eq!(merged, json!({"tags": ["c"]}));
    }

    #[test]
    fn test_deep_merge_override_deep_merges_dicts() {
        let base = json!({"a": {"x": 1}});
        let overlay = json!({"a": {"y": 2}});
        let merged = deep_merge_override(&base, &overlay);
        assert_eq!(merged, json!({"a": {"x": 1, "y": 2}}));
    }

    // ── apply_set_overrides ───────────────────────────────────────────

    fn to_map(value: Value) -> serde_json::Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    #[test]
    fn test_apply_set_overrides_worked_example() {
        let base = to_map(json!({
            "name": "Alice",
            "author": { "name": "Alice", "handle": "@alice" },
            "tags": ["red", "green"],
        }));
        let set_object = to_map(json!({
            "author": { "handle": "@bob" },
            "tags": ["blue"],
        }));
        let set_properties: Vec<(String, Value)> = vec![("name".to_string(), json!("Bob"))];

        let effective = apply_set_overrides(&base, Some(&set_object), &set_properties);

        assert_eq!(effective.get("name"), Some(&json!("Bob")));
        assert_eq!(
            effective.get("author"),
            Some(&json!({ "name": "Alice", "handle": "@bob" }))
        );
        assert_eq!(effective.get("tags"), Some(&json!(["blue"])));
    }

    #[test]
    fn test_apply_set_overrides_three_layer_precedence() {
        let base = to_map(json!({"name": "Alice"}));
        let set_object = to_map(json!({"name": "Carol"}));
        let set_properties: Vec<(String, Value)> = vec![("name".to_string(), json!("Bob"))];

        let effective = apply_set_overrides(&base, Some(&set_object), &set_properties);

        assert_eq!(effective.get("name"), Some(&json!("Bob")));
    }

    #[test]
    fn test_apply_set_overrides_null_property_is_literal() {
        let base = to_map(json!({"x": 5}));
        let set_properties: Vec<(String, Value)> = vec![("x".to_string(), Value::Null)];

        let effective = apply_set_overrides(&base, None, &set_properties);

        assert_eq!(effective.get("x"), Some(&Value::Null));
    }

    #[test]
    fn test_apply_set_overrides_dict_deep_merge_via_property() {
        let base = to_map(json!({"a": {"x": 1}}));
        let set_properties: Vec<(String, Value)> = vec![("a".to_string(), json!({"y": 2}))];

        let effective = apply_set_overrides(&base, None, &set_properties);

        assert_eq!(effective.get("a"), Some(&json!({"x": 1, "y": 2})));
    }

    #[test]
    fn test_apply_set_overrides_leaf_override() {
        let base = to_map(json!({"name": "Alice"}));
        let set_properties: Vec<(String, Value)> = vec![("name".to_string(), json!("Bob"))];

        let effective = apply_set_overrides(&base, None, &set_properties);

        assert_eq!(effective.get("name"), Some(&json!("Bob")));
    }

    #[test]
    fn test_apply_set_overrides_array_is_leaf() {
        let base = to_map(json!({"tags": ["a", "b"]}));
        let set_properties: Vec<(String, Value)> = vec![("tags".to_string(), json!(["c"]))];

        let effective = apply_set_overrides(&base, None, &set_properties);

        assert_eq!(effective.get("tags"), Some(&json!(["c"])));
    }

    #[test]
    fn test_apply_set_overrides_no_overlay_is_identity() {
        let base = to_map(json!({"a": 1, "b": {"c": 2}}));
        let effective = apply_set_overrides(&base, None, &[]);
        assert_eq!(Value::Object(effective), Value::Object(base));
    }

    #[test]
    fn test_apply_set_overrides_author_null_leaf_deep_merge() {
        // Spec: set.author='{name: null}' — overlay `author.name: null` but
        // keep the surrounding dict (author.handle) from the base.
        let base = to_map(json!({
            "author": {"name": "Alice", "handle": "@alice"}
        }));
        let set_properties: Vec<(String, Value)> =
            vec![("author".to_string(), json!({"name": null}))];

        let effective = apply_set_overrides(&base, None, &set_properties);

        assert_eq!(
            effective.get("author"),
            Some(&json!({"name": null, "handle": "@alice"}))
        );
    }
}
