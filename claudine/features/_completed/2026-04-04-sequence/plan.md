# Sequence Feature Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `claudine sequence <file>` command that serially composes a Markdown document once per step in a user-defined sequence, injecting step-specific state into each composition run.

**Architecture:** Sequence is a thin orchestration layer around the existing wrapper-grade composition executor. The library side (`composition::sequence`) handles parsing, normalization, validation, and overlay generation. The CLI side (`commands/sequence.rs` + `wrap/sequence.rs`) wires up the command and drives the serial step loop. No new execution engine is introduced — each step is a standard one-shot composition run.

**Tech Stack:** Rust, clap (derive), serde_json, biscuit-file (YAML), darkmatter (composition), existing claudine composition pipeline.

---

## File Structure

```
claudine/lib/src/composition/
├── mod.rs                 # MODIFY — add `pub mod sequence;` and re-exports
├── error.rs               # MODIFY — add sequence-specific error variants
├── prepare.rs             # MODIFY — refactor to accept PrepareOptions struct
├── sequence.rs            # CREATE — parser, normalizer, overlay builder
└── types.rs               # MODIFY — add sequence data types

claudine/cli/src/
├── args.rs                # MODIFY — add Commands::Sequence variant
├── main.rs                # MODIFY — add Sequence dispatch
├── commands/
│   ├── mod.rs             # MODIFY — add `pub mod sequence;`
│   ├── sequence.rs        # CREATE — CLI args, entry point, delegates to wrap
│   └── wrap/
│       ├── mod.rs         # MODIFY — add `pub(crate) mod sequence;`
│       ├── composition.rs # MODIFY — extract reusable single-step executor
│       └── sequence.rs    # CREATE — serial step loop, progress reporting
```

---

## Task 1: Add Sequence Types to `composition/types.rs`

**Files:**
- Modify: `claudine/lib/src/composition/types.rs`
- Test: `claudine/lib/src/composition/types.rs` (inline tests)

- [ ] **Step 1: Write tests for sequence types**

Add at the bottom of the existing `types.rs` file (there are no existing tests in this file, so add a `#[cfg(test)] mod tests` block):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sequence_step_overlay_first_step() {
        let overlay = SequenceStepOverlay {
            state: json!("one"),
            previous_state: serde_json::Value::Null,
            next_state: json!("two"),
            is_first: true,
            is_last: false,
            step: 1,
            total_steps: 3,
        };
        assert!(overlay.is_first);
        assert!(!overlay.is_last);
        assert_eq!(overlay.step, 1);
        assert_eq!(overlay.total_steps, 3);
        assert!(overlay.previous_state.is_null());
    }

    #[test]
    fn sequence_step_overlay_last_step() {
        let overlay = SequenceStepOverlay {
            state: json!("three"),
            previous_state: json!("two"),
            next_state: serde_json::Value::Null,
            is_first: false,
            is_last: true,
            step: 3,
            total_steps: 3,
        };
        assert!(!overlay.is_first);
        assert!(overlay.is_last);
        assert_eq!(overlay.step, 3);
        assert!(overlay.next_state.is_null());
    }

    #[test]
    fn sequence_step_overlay_as_overrides_reserves_keys() {
        let overlay = SequenceStepOverlay {
            state: json!("one"),
            previous_state: serde_json::Value::Null,
            next_state: json!("two"),
            is_first: true,
            is_last: false,
            step: 1,
            total_steps: 3,
        };
        let overrides = overlay.as_set_overrides(None);
        let obj = overrides.as_object().unwrap();
        assert_eq!(obj.get("state"), Some(&json!("one")));
        assert_eq!(obj.get("is_first"), Some(&json!(true)));
        assert_eq!(obj.get("is_last"), Some(&json!(false)));
        assert_eq!(obj.get("step"), Some(&json!(1)));
        assert_eq!(obj.get("total_steps"), Some(&json!(3)));
        assert!(obj.get("previous_state").unwrap().is_null());
        assert_eq!(obj.get("next_state"), Some(&json!("two")));
    }

    #[test]
    fn sequence_step_overlay_merges_user_set_but_reserved_wins() {
        let overlay = SequenceStepOverlay {
            state: json!("one"),
            previous_state: serde_json::Value::Null,
            next_state: json!("two"),
            is_first: true,
            is_last: false,
            step: 1,
            total_steps: 3,
        };
        let user_set = json!({
            "color": "red",
            "state": "should-be-overridden",
            "step": 99
        });
        let overrides = overlay.as_set_overrides(Some(user_set));
        let obj = overrides.as_object().unwrap();
        // User key preserved
        assert_eq!(obj.get("color"), Some(&json!("red")));
        // Reserved keys overwritten by overlay
        assert_eq!(obj.get("state"), Some(&json!("one")));
        assert_eq!(obj.get("step"), Some(&json!(1)));
    }

    #[test]
    fn sequence_plan_display_source() {
        let plan = SequencePlan {
            source: SequenceSource::Inline,
            steps: vec![],
            document_fail_fast: true,
        };
        assert!(matches!(plan.source, SequenceSource::Inline));

        let plan2 = SequencePlan {
            source: SequenceSource::External {
                path: std::path::PathBuf::from("data.yaml"),
            },
            steps: vec![],
            document_fail_fast: false,
        };
        assert!(!plan2.document_fail_fast);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p claudine --lib composition::types::tests -- --nocapture 2>&1 | head -30`
Expected: FAIL — `SequenceStepOverlay`, `SequencePlan`, etc. are not defined yet.

- [ ] **Step 3: Implement the sequence types**

Add the following types to `claudine/lib/src/composition/types.rs`, before the existing `#[cfg(test)]` block (or at the end of the non-test code):

```rust
/// Describes where the sequence definition was found.
#[derive(Debug, Clone)]
pub enum SequenceSource {
    /// The sequence was defined inline in the document's frontmatter.
    Inline,
    /// The sequence was loaded from an external YAML file.
    External { path: PathBuf },
}

/// A single step in a sequence.
#[derive(Debug, Clone)]
pub struct SequenceStep {
    /// Zero-based index in the sequence list.
    pub index: usize,
    /// Display name for the step (scalar value or the `name` field of an object).
    pub name: String,
    /// The full state value: a JSON string for scalar steps, a JSON object for object steps.
    pub raw_state: serde_json::Value,
}

/// A validated, normalized sequence plan ready for execution.
#[derive(Debug, Clone)]
pub struct SequencePlan {
    /// Where the sequence definition came from.
    pub source: SequenceSource,
    /// Ordered list of steps.
    pub steps: Vec<SequenceStep>,
    /// The document's `fail_fast` setting (defaults to `true`).
    pub document_fail_fast: bool,
}

/// Per-step overlay values injected into each composition run.
#[derive(Debug, Clone)]
pub struct SequenceStepOverlay {
    /// Current step's state value.
    pub state: serde_json::Value,
    /// Previous step's state value, or `null` for the first step.
    pub previous_state: serde_json::Value,
    /// Next step's state value, or `null` for the last step.
    pub next_state: serde_json::Value,
    /// `true` if this is the first step.
    pub is_first: bool,
    /// `true` if this is the last step.
    pub is_last: bool,
    /// 1-based step number.
    pub step: usize,
    /// Total number of steps in the sequence.
    pub total_steps: usize,
}

impl SequenceStepOverlay {
    /// Reserved overlay keys that must always win over user `--set` values.
    pub const RESERVED_KEYS: &[&str] = &[
        "state",
        "previous_state",
        "next_state",
        "is_first",
        "is_last",
        "step",
        "total_steps",
    ];

    /// Build a `serde_json::Value::Object` suitable for `set_overrides`.
    ///
    /// Merge order: user `--set` first, then overlay (overlay wins on conflict).
    pub fn as_set_overrides(&self, user_set: Option<serde_json::Value>) -> serde_json::Value {
        let mut map = serde_json::Map::new();

        // 1. Start with user --set values
        if let Some(serde_json::Value::Object(user_map)) = user_set {
            for (key, value) in user_map {
                map.insert(key, value);
            }
        }

        // 2. Overlay reserved keys (always win)
        map.insert("state".into(), self.state.clone());
        map.insert("previous_state".into(), self.previous_state.clone());
        map.insert("next_state".into(), self.next_state.clone());
        map.insert("is_first".into(), serde_json::Value::Bool(self.is_first));
        map.insert("is_last".into(), serde_json::Value::Bool(self.is_last));
        map.insert(
            "step".into(),
            serde_json::Value::Number(self.step.into()),
        );
        map.insert(
            "total_steps".into(),
            serde_json::Value::Number(self.total_steps.into()),
        );

        serde_json::Value::Object(map)
    }
}

/// Options for sequence execution at the CLI level.
#[derive(Debug, Clone)]
pub struct SequenceExecutionOptions {
    /// CLI `--fail-fast` override. `None` means use the document default.
    pub fail_fast_override: Option<bool>,
}

/// Summary of a completed sequence run.
#[derive(Debug, Clone)]
pub struct SequenceRunSummary {
    /// Total steps in the sequence.
    pub total_steps: usize,
    /// Number of steps that succeeded.
    pub succeeded: usize,
    /// Number of steps that failed.
    pub failed: usize,
    /// Per-step results.
    pub steps: Vec<SequenceStepResult>,
}

/// Result of a single sequence step.
#[derive(Debug, Clone)]
pub struct SequenceStepResult {
    /// 1-based step number.
    pub step: usize,
    /// Display name of the step.
    pub name: String,
    /// Whether the step succeeded (exit code 0).
    pub success: bool,
    /// Error message if the step failed.
    pub error: Option<String>,
    /// Wall-clock duration of the step.
    pub duration: std::time::Duration,
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p claudine --lib composition::types::tests -- --nocapture`
Expected: All 5 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/composition/types.rs
git commit -m "feat(claudine): add sequence data types to composition module"
```

---

## Task 2: Add Sequence Error Variants to `composition/error.rs`

**Files:**
- Modify: `claudine/lib/src/composition/error.rs`

- [ ] **Step 1: Add error variants**

Add the following variants to the `CompositionError` enum in `claudine/lib/src/composition/error.rs`, after the existing `LifecycleUnknownEffect` variant:

```rust
    // -- Sequence errors -------------------------------------------------------

    /// The `sequence` frontmatter value is not a valid type (must be a list or a string).
    #[error("invalid sequence definition: {0}")]
    SequenceInvalid(String),

    /// The sequence list is empty.
    #[error("sequence list is empty; at least one step is required")]
    SequenceEmpty,

    /// The external YAML file could not be loaded.
    #[error("failed to load external sequence file: {0}")]
    SequenceExternalLoad(String),

    /// The external YAML file has an unexpected root shape.
    #[error("external sequence file has wrong structure: {0}")]
    SequenceExternalWrongType(String),

    /// An object step is missing the required `name` property.
    #[error("sequence step at index {index} is missing required `name` property")]
    SequenceStepNameMissing { index: usize },

    /// The `name` property of an object step is not a string.
    #[error("sequence step at index {index} has `name` of type {found}, expected string")]
    SequenceStepNameWrongType { index: usize, found: String },

    /// A template value is not a string.
    #[error("sequence template key `{key}` has type {found}, expected string")]
    SequenceTemplateWrongType { key: String, found: String },

    /// Templates require all list items to be objects.
    #[error("sequence templates require all list items to be objects (dictionaries)")]
    SequenceTemplateRequiresObjectItems,

    /// A template key collides with a reserved sequence overlay key.
    #[error("sequence template key `{0}` collides with reserved sequence key")]
    SequenceReservedTemplateKey(String),
```

- [ ] **Step 2: Verify the library compiles**

Run: `cargo check -p claudine 2>&1 | tail -5`
Expected: Compiles without errors.

- [ ] **Step 3: Commit**

```bash
git add claudine/lib/src/composition/error.rs
git commit -m "feat(claudine): add sequence-specific error variants"
```

---

## Task 3: Create Sequence Parser/Normalizer (`composition/sequence.rs`)

**Files:**
- Create: `claudine/lib/src/composition/sequence.rs`
- Modify: `claudine/lib/src/composition/mod.rs` — add `pub mod sequence;` and re-exports

This is the largest library task. It handles:
1. Detecting sequence from frontmatter
2. Parsing inline sequences (scalar and object)
3. Resolving and parsing external YAML references
4. Applying templates from external `kind/list/template` YAML
5. Building per-step overlays

### Step 3a: Write tests first

- [ ] **Step 1: Create `sequence.rs` with test module**

Create `claudine/lib/src/composition/sequence.rs` with the full test suite and empty function stubs:

```rust
//! Sequence detection, parsing, normalization, and overlay generation.
//!
//! Provides [`resolve_sequence_plan`] to detect whether a resolved composition
//! source defines a sequence, and if so, parse and normalize it into a typed
//! [`SequencePlan`]. Also provides [`build_step_overlay`] to construct the
//! per-step variable overlay for each composition run.

use std::path::Path;

use super::error::CompositionError;
use super::types::{
    SequencePlan, SequenceSource, SequenceStep, SequenceStepOverlay,
};

/// Detect and resolve a sequence plan from a resolved composition source.
///
/// Returns `Ok(None)` if the source has no `sequence` frontmatter key.
/// Returns `Ok(Some(plan))` if a valid sequence is found.
///
/// ## Errors
///
/// Returns `Err` for invalid sequence definitions: wrong types, missing
/// `name` on object steps, empty lists, or external file load failures.
pub fn resolve_sequence_plan(
    source: &super::types::ResolvedCompositionSource,
) -> Result<Option<SequencePlan>, CompositionError> {
    let fm = source.markdown.frontmatter();
    let sequence_value = match fm.as_map().get("sequence") {
        Some(v) => v.clone(),
        None => return Ok(None),
    };

    let fail_fast = fm
        .as_map()
        .get("fail_fast")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    match sequence_value {
        serde_json::Value::Array(items) => {
            let steps = normalize_inline_list(&items)?;
            Ok(Some(SequencePlan {
                source: SequenceSource::Inline,
                steps,
                document_fail_fast: fail_fast,
            }))
        }
        serde_json::Value::String(ref path_str) => {
            let base_dir = source
                .resolved_path
                .parent()
                .unwrap_or_else(|| Path::new("."));
            let yaml_path = base_dir.join(path_str);
            let plan = load_external_sequence(&yaml_path, fail_fast)?;
            Ok(Some(plan))
        }
        other => Err(CompositionError::SequenceInvalid(format!(
            "expected a list or file path string, got {}",
            json_type_name(&other)
        ))),
    }
}

/// Build a step overlay for the given step index within a plan.
pub fn build_step_overlay(plan: &SequencePlan, step_index: usize) -> SequenceStepOverlay {
    let total = plan.steps.len();
    let current = &plan.steps[step_index];

    let previous_state = if step_index > 0 {
        plan.steps[step_index - 1].raw_state.clone()
    } else {
        serde_json::Value::Null
    };

    let next_state = if step_index + 1 < total {
        plan.steps[step_index + 1].raw_state.clone()
    } else {
        serde_json::Value::Null
    };

    SequenceStepOverlay {
        state: current.raw_state.clone(),
        previous_state,
        next_state,
        is_first: step_index == 0,
        is_last: step_index + 1 == total,
        step: step_index + 1,
        total_steps: total,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Normalize an inline YAML list into typed steps.
fn normalize_inline_list(
    items: &[serde_json::Value],
) -> Result<Vec<SequenceStep>, CompositionError> {
    if items.is_empty() {
        return Err(CompositionError::SequenceEmpty);
    }

    items
        .iter()
        .enumerate()
        .map(|(index, item)| match item {
            serde_json::Value::String(s) => Ok(SequenceStep {
                index,
                name: s.clone(),
                raw_state: item.clone(),
            }),
            serde_json::Value::Object(map) => {
                let name = map
                    .get("name")
                    .ok_or(CompositionError::SequenceStepNameMissing { index })?;
                let name_str = name.as_str().ok_or_else(|| {
                    CompositionError::SequenceStepNameWrongType {
                        index,
                        found: json_type_name(name).to_string(),
                    }
                })?;
                Ok(SequenceStep {
                    index,
                    name: name_str.to_string(),
                    raw_state: item.clone(),
                })
            }
            other => Err(CompositionError::SequenceInvalid(format!(
                "step at index {index} must be a string or object, got {}",
                json_type_name(other)
            ))),
        })
        .collect()
}

/// Load and parse an external YAML sequence file.
fn load_external_sequence(
    yaml_path: &Path,
    document_fail_fast: bool,
) -> Result<SequencePlan, CompositionError> {
    let yaml = biscuit_file::Yaml::new(yaml_path).map_err(|e| {
        CompositionError::SequenceExternalLoad(format!("{}: {e}", yaml_path.display()))
    })?;
    let json_value = yaml.as_json().map_err(|e| {
        CompositionError::SequenceExternalLoad(format!(
            "YAML-to-JSON conversion failed for {}: {e}",
            yaml_path.display()
        ))
    })?;
    let root = json_value
        .as_object()
        .ok_or_else(|| {
            CompositionError::SequenceExternalWrongType(
                "root must be an object".to_string(),
            )
        })?;

    // Detect which external form is used:
    // Form 1: { sequence: [...] }
    // Form 2: { kind: "sequence", list: [...], template?: {...} }
    if let Some(list_value) = root.get("sequence") {
        let items = list_value.as_array().ok_or_else(|| {
            CompositionError::SequenceExternalWrongType(
                "`sequence` must be a list".to_string(),
            )
        })?;
        let steps = normalize_inline_list(items)?;
        return Ok(SequencePlan {
            source: SequenceSource::External {
                path: yaml_path.to_path_buf(),
            },
            steps,
            document_fail_fast,
        });
    }

    // Form 2: kind/list/template
    if let Some(kind_value) = root.get("kind") {
        let kind_str = kind_value.as_str().ok_or_else(|| {
            CompositionError::SequenceExternalWrongType(
                "`kind` must be a string".to_string(),
            )
        })?;
        if kind_str != "sequence" {
            return Err(CompositionError::SequenceExternalWrongType(format!(
                "`kind` must be \"sequence\", got \"{kind_str}\""
            )));
        }
    }

    let list_value = root.get("list").ok_or_else(|| {
        CompositionError::SequenceExternalWrongType(
            "external file must have `sequence` or `list` key".to_string(),
        )
    })?;
    let items = list_value.as_array().ok_or_else(|| {
        CompositionError::SequenceExternalWrongType("`list` must be a list".to_string())
    })?;

    let template = root.get("template").and_then(|v| v.as_object());

    // Validate template keys don't collide with reserved overlay keys
    if let Some(tmpl) = template {
        for key in tmpl.keys() {
            if SequenceStepOverlay::RESERVED_KEYS.contains(&key.as_str()) {
                return Err(CompositionError::SequenceReservedTemplateKey(
                    key.clone(),
                ));
            }
        }
        // Validate template values are all strings
        for (key, value) in tmpl {
            if !value.is_string() {
                return Err(CompositionError::SequenceTemplateWrongType {
                    key: key.clone(),
                    found: json_type_name(value).to_string(),
                });
            }
        }
    }

    let mut steps = normalize_inline_list(items)?;

    // Apply template fields to each step
    if let Some(tmpl) = template {
        for step in &mut steps {
            if !step.raw_state.is_object() {
                return Err(CompositionError::SequenceTemplateRequiresObjectItems);
            }
            let step_map = step.raw_state.as_object().unwrap();
            let mut new_map = step_map.clone();

            for (tmpl_key, tmpl_value) in tmpl {
                let template_str = tmpl_value.as_str().unwrap(); // validated above
                let rendered = render_simple_template(template_str, step_map);
                new_map.entry(tmpl_key.clone()).or_insert(serde_json::Value::String(rendered));
            }

            step.raw_state = serde_json::Value::Object(new_map);
        }
    }

    Ok(SequencePlan {
        source: SequenceSource::External {
            path: yaml_path.to_path_buf(),
        },
        steps,
        document_fail_fast,
    })
}

/// Simple `{{key}}` and `{{key || default}}` template renderer.
///
/// Replaces `{{key}}` with the value from the item's fields.
/// Supports `{{key || default}}` fallback syntax.
fn render_simple_template(
    template: &str,
    fields: &serde_json::Map<String, serde_json::Value>,
) -> String {
    let mut result = template.to_string();
    // Match {{ key }} and {{ key || default }} patterns
    let re = regex::Regex::new(r"\{\{\s*([^{}|]+?)(?:\s*\|\|\s*([^{}]*?))?\s*\}\}").unwrap();

    result = re
        .replace_all(&result, |caps: &regex::Captures| {
            let key = caps[1].trim();
            let default = caps.get(2).map(|m| m.as_str().trim().trim_matches('\''));

            match fields.get(key) {
                Some(serde_json::Value::String(s)) if !s.is_empty() => s.clone(),
                Some(serde_json::Value::Null) | None => {
                    default.unwrap_or("").to_string()
                }
                Some(other) => other.to_string(),
            }
        })
        .to_string();

    result
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkmatter::markdown::{Frontmatter, Markdown};
    use serde_json::json;
    use std::fs;
    use tempfile::TempDir;

    use crate::composition::types::ResolvedCompositionSource;

    fn make_source(
        dir: &TempDir,
        frontmatter: &[(&str, serde_json::Value)],
        content: &str,
    ) -> ResolvedCompositionSource {
        let file = dir.path().join("test.md");
        let mut fm = Frontmatter::new();
        for (key, value) in frontmatter {
            fm.insert(key, value.clone()).unwrap();
        }
        let md = Markdown::with_frontmatter(fm, content);
        fs::write(&file, md.as_string()).unwrap();

        let markdown = Markdown::try_from(file.as_path()).unwrap();
        let original_text = fs::read_to_string(&file).unwrap();
        ResolvedCompositionSource {
            original_ref: file.to_str().unwrap().to_string(),
            resolved_path: file,
            original_text,
            markdown,
        }
    }

    // -- resolve_sequence_plan: no sequence key --------------------------------

    #[test]
    fn no_sequence_key_returns_none() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("title", json!("Test"))], "Content");
        let result = resolve_sequence_plan(&source).unwrap();
        assert!(result.is_none());
    }

    // -- resolve_sequence_plan: inline scalar list ----------------------------

    #[test]
    fn inline_scalar_list_normalizes_correctly() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[("sequence", json!(["one", "two", "three"]))],
            "Prompt: {{state}}",
        );
        let plan = resolve_sequence_plan(&source).unwrap().unwrap();
        assert!(matches!(plan.source, SequenceSource::Inline));
        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.steps[0].name, "one");
        assert_eq!(plan.steps[1].name, "two");
        assert_eq!(plan.steps[2].name, "three");
        assert_eq!(plan.steps[0].raw_state, json!("one"));
        assert!(plan.document_fail_fast); // default
    }

    // -- resolve_sequence_plan: inline object list ----------------------------

    #[test]
    fn inline_object_list_requires_name() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[(
                "sequence",
                json!([
                    {"name": "one", "color": "red"},
                    {"name": "two", "color": "blue"}
                ]),
            )],
            "Prompt",
        );
        let plan = resolve_sequence_plan(&source).unwrap().unwrap();
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].name, "one");
        assert_eq!(
            plan.steps[0].raw_state,
            json!({"name": "one", "color": "red"})
        );
    }

    #[test]
    fn inline_object_step_missing_name_fails() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[("sequence", json!([{"color": "red"}]))],
            "Prompt",
        );
        let err = resolve_sequence_plan(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::SequenceStepNameMissing { index: 0 }),
            "got: {err}"
        );
    }

    #[test]
    fn inline_object_step_name_wrong_type_fails() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[("sequence", json!([{"name": 42}]))],
            "Prompt",
        );
        let err = resolve_sequence_plan(&source).unwrap_err();
        assert!(
            matches!(
                err,
                CompositionError::SequenceStepNameWrongType { index: 0, .. }
            ),
            "got: {err}"
        );
    }

    // -- resolve_sequence_plan: empty list ------------------------------------

    #[test]
    fn empty_list_fails() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("sequence", json!([]))], "Prompt");
        let err = resolve_sequence_plan(&source).unwrap_err();
        assert!(matches!(err, CompositionError::SequenceEmpty), "got: {err}");
    }

    // -- resolve_sequence_plan: invalid type -----------------------------------

    #[test]
    fn invalid_sequence_type_fails() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("sequence", json!(42))], "Prompt");
        let err = resolve_sequence_plan(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::SequenceInvalid(_)),
            "got: {err}"
        );
    }

    // -- resolve_sequence_plan: fail_fast frontmatter -------------------------

    #[test]
    fn fail_fast_false_from_frontmatter() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[
                ("sequence", json!(["one", "two"])),
                ("fail_fast", json!(false)),
            ],
            "Prompt",
        );
        let plan = resolve_sequence_plan(&source).unwrap().unwrap();
        assert!(!plan.document_fail_fast);
    }

    // -- resolve_sequence_plan: external YAML (sequence: form) ----------------

    #[test]
    fn external_sequence_form_loads() {
        let dir = TempDir::new().unwrap();
        let yaml_path = dir.path().join("steps.yaml");
        fs::write(
            &yaml_path,
            "sequence:\n  - name: alpha\n    color: red\n  - name: beta\n    color: blue\n",
        )
        .unwrap();

        let source = make_source(
            &dir,
            &[("sequence", json!("steps.yaml"))],
            "Prompt: {{state.name}}",
        );
        let plan = resolve_sequence_plan(&source).unwrap().unwrap();
        assert!(matches!(plan.source, SequenceSource::External { .. }));
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].name, "alpha");
    }

    // -- resolve_sequence_plan: external YAML (kind/list/template form) -------

    #[test]
    fn external_kind_list_template_loads_and_applies_templates() {
        let dir = TempDir::new().unwrap();
        let yaml_path = dir.path().join("agents.yaml");
        fs::write(
            &yaml_path,
            r#"kind: sequence
template:
  desc: "{{name}} (site: {{site}})"
list:
  - name: Claude Code
    site: https://code.claude.com
  - name: Codex CLI
    site: https://codex.openai.com
"#,
        )
        .unwrap();

        let source = make_source(
            &dir,
            &[("sequence", json!("agents.yaml"))],
            "Research {{state.name}}",
        );
        let plan = resolve_sequence_plan(&source).unwrap().unwrap();
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].name, "Claude Code");

        // Template should have been applied
        let desc0 = plan.steps[0]
            .raw_state
            .as_object()
            .unwrap()
            .get("desc")
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(desc0, "Claude Code (site: https://code.claude.com)");
    }

    #[test]
    fn external_template_with_fallback_default() {
        let dir = TempDir::new().unwrap();
        let yaml_path = dir.path().join("items.yaml");
        fs::write(
            &yaml_path,
            r#"kind: sequence
template:
  summary: "{{name}} - repo: {{repo || 'n/a'}}"
list:
  - name: Tool A
    repo: https://github.com/a
  - name: Tool B
"#,
        )
        .unwrap();

        let source = make_source(
            &dir,
            &[("sequence", json!("items.yaml"))],
            "Prompt",
        );
        let plan = resolve_sequence_plan(&source).unwrap().unwrap();
        let summary0 = plan.steps[0]
            .raw_state
            .as_object()
            .unwrap()
            .get("summary")
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(summary0, "Tool A - repo: https://github.com/a");

        let summary1 = plan.steps[1]
            .raw_state
            .as_object()
            .unwrap()
            .get("summary")
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(summary1, "Tool B - repo: n/a");
    }

    #[test]
    fn external_template_reserved_key_collision_fails() {
        let dir = TempDir::new().unwrap();
        let yaml_path = dir.path().join("bad.yaml");
        fs::write(
            &yaml_path,
            r#"kind: sequence
template:
  state: "{{name}}"
list:
  - name: One
"#,
        )
        .unwrap();

        let source = make_source(
            &dir,
            &[("sequence", json!("bad.yaml"))],
            "Prompt",
        );
        let err = resolve_sequence_plan(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::SequenceReservedTemplateKey(ref k) if k == "state"),
            "got: {err}"
        );
    }

    #[test]
    fn external_template_non_string_value_fails() {
        let dir = TempDir::new().unwrap();
        let yaml_path = dir.path().join("bad.yaml");
        fs::write(
            &yaml_path,
            "kind: sequence\ntemplate:\n  count: 42\nlist:\n  - name: One\n",
        )
        .unwrap();

        let source = make_source(
            &dir,
            &[("sequence", json!("bad.yaml"))],
            "Prompt",
        );
        let err = resolve_sequence_plan(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::SequenceTemplateWrongType { .. }),
            "got: {err}"
        );
    }

    // -- build_step_overlay ---------------------------------------------------

    #[test]
    fn overlay_for_single_step_sequence() {
        let plan = SequencePlan {
            source: SequenceSource::Inline,
            steps: vec![SequenceStep {
                index: 0,
                name: "only".to_string(),
                raw_state: json!("only"),
            }],
            document_fail_fast: true,
        };
        let overlay = build_step_overlay(&plan, 0);
        assert!(overlay.is_first);
        assert!(overlay.is_last);
        assert_eq!(overlay.step, 1);
        assert_eq!(overlay.total_steps, 1);
        assert!(overlay.previous_state.is_null());
        assert!(overlay.next_state.is_null());
    }

    #[test]
    fn overlay_for_middle_step() {
        let plan = SequencePlan {
            source: SequenceSource::Inline,
            steps: vec![
                SequenceStep { index: 0, name: "a".into(), raw_state: json!("a") },
                SequenceStep { index: 1, name: "b".into(), raw_state: json!("b") },
                SequenceStep { index: 2, name: "c".into(), raw_state: json!("c") },
            ],
            document_fail_fast: true,
        };
        let overlay = build_step_overlay(&plan, 1);
        assert!(!overlay.is_first);
        assert!(!overlay.is_last);
        assert_eq!(overlay.step, 2);
        assert_eq!(overlay.total_steps, 3);
        assert_eq!(overlay.state, json!("b"));
        assert_eq!(overlay.previous_state, json!("a"));
        assert_eq!(overlay.next_state, json!("c"));
    }

    // -- render_simple_template -----------------------------------------------

    #[test]
    fn template_replaces_known_keys() {
        let mut fields = serde_json::Map::new();
        fields.insert("name".into(), json!("Foo"));
        fields.insert("site".into(), json!("https://example.com"));
        let result = render_simple_template("{{name}} at {{site}}", &fields);
        assert_eq!(result, "Foo at https://example.com");
    }

    #[test]
    fn template_uses_fallback_for_missing_key() {
        let fields = serde_json::Map::new();
        let result = render_simple_template("repo: {{repo || 'n/a'}}", &fields);
        assert_eq!(result, "repo: n/a");
    }

    #[test]
    fn template_uses_fallback_for_null_value() {
        let mut fields = serde_json::Map::new();
        fields.insert("repo".into(), serde_json::Value::Null);
        let result = render_simple_template("repo: {{ repo || 'none' }}", &fields);
        assert_eq!(result, "repo: none");
    }
}
```

- [ ] **Step 2: Add `pub mod sequence;` to `composition/mod.rs`**

In `claudine/lib/src/composition/mod.rs`, add after the existing `mod types;` line:

```rust
pub mod sequence;
```

And add re-exports at the bottom of the existing `pub use` block:

```rust
pub use sequence::{build_step_overlay, resolve_sequence_plan};
pub use types::{
    SequenceExecutionOptions, SequencePlan, SequenceRunSummary, SequenceSource,
    SequenceStep, SequenceStepOverlay, SequenceStepResult,
};
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p claudine --lib composition::sequence -- --nocapture`
Expected: All tests PASS.

- [ ] **Step 4: Commit**

```bash
git add claudine/lib/src/composition/sequence.rs claudine/lib/src/composition/mod.rs
git commit -m "feat(claudine): add sequence parser, normalizer, and overlay builder"
```

---

## Task 4: Refactor `prepare.rs` to Accept `PrepareOptions`

**Files:**
- Modify: `claudine/lib/src/composition/prepare.rs`
- Modify: `claudine/lib/src/composition/mod.rs` — update re-exports
- Modify: `claudine/cli/src/commands/compose.rs` — update call sites

The tech design calls for a `PrepareOptions` struct so sequence can inject env overrides (for `FAIL_FAST`) alongside `set_overrides` and `pre_approved_commands`.

`ComposeContext::env_mut()` already exists in Darkmatter, so no Darkmatter changes are needed. We use `ComposeOptions::new_with_context(ctx)` with a pre-mutated context.

- [ ] **Step 1: Add `PrepareOptions` struct and update function signatures**

In `claudine/lib/src/composition/prepare.rs`, add this struct before `prepare_direct`:

```rust
use std::collections::BTreeMap;

/// Options for composition preparation.
///
/// Wraps the individual parameters that `prepare_direct` and `prepare_inline`
/// previously accepted as separate arguments, plus `env_overrides` needed
/// by the sequence feature.
#[derive(Debug, Default)]
pub struct PrepareOptions {
    /// Frontmatter `--set` overrides (JSON object).
    pub set_overrides: Option<serde_json::Value>,
    /// Commands pre-approved during pre-flight shell discovery.
    pub pre_approved_commands: Option<std::collections::HashSet<String>>,
    /// Extra environment variables to inject into the composition context.
    ///
    /// These are merged into the `ComposeContext` before composition runs,
    /// making them visible to `{{env.KEY}}` interpolation and `::shell`
    /// directives.
    pub env_overrides: BTreeMap<String, String>,
}
```

Then update the signatures of `prepare_direct` and `prepare_inline` to accept `PrepareOptions` instead of separate args. For backward compatibility, keep the old signatures as thin wrappers or update all call sites.

**Updated `prepare_direct`:**

```rust
pub fn prepare_direct(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
) -> Result<PreparedComposition, CompositionError> {
    let mut ctx = darkmatter::markdown::compose::ComposeContext::capture();
    for (key, value) in &options.env_overrides {
        ctx.env_mut().insert(key.clone(), value.clone());
    }
    let mut compose_opts = ComposeOptions::new_with_context(ctx)
        .with_source_file(&source.resolved_path);
    if let Some(overrides) = options.set_overrides {
        compose_opts = compose_opts.with_set_overrides(overrides);
    }
    if let Some(approved) = options.pre_approved_commands {
        compose_opts = compose_opts.with_pre_approved_commands(approved);
    }
    let (composed, _report) = source
        .markdown
        .compose_with(compose_opts)
        .map_err(|e| CompositionError::ComposeFailed(e.to_string()))?;

    let effective_frontmatter = frontmatter_to_value(composed.frontmatter());
    let effective_agent_hint = composed.frontmatter().as_map().get("agent").cloned();
    let lifecycle = parse_lifecycle_config(&effective_frontmatter)?;

    let source_repo_root = find_git_root_from_path(&source.resolved_path);

    Ok(PreparedComposition {
        mode: CompositionMode::ChainedDocument,
        resolved_path: source.resolved_path.clone(),
        source_repo_root,
        prompt: composed.content().to_string(),
        effective_frontmatter,
        effective_agent_hint,
        closure: CompositionClosurePlan::Direct,
        lifecycle,
    })
}
```

**Updated `prepare_inline`:**

```rust
pub fn prepare_inline(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
) -> Result<PreparedComposition, CompositionError> {
    let fm = source.markdown.frontmatter();

    let prompt_value = fm
        .as_map()
        .get("prompt")
        .ok_or(CompositionError::PromptPropertyMissing)?;

    let prompt_text = match prompt_value {
        serde_json::Value::String(s) => s.clone(),
        other => {
            return Err(CompositionError::PromptPropertyWrongType(
                json_type_name(other).to_string(),
            ));
        }
    };

    let temp_md = Markdown::with_frontmatter(fm.clone(), &prompt_text);
    let mut ctx = darkmatter::markdown::compose::ComposeContext::capture();
    for (key, value) in &options.env_overrides {
        ctx.env_mut().insert(key.clone(), value.clone());
    }
    let mut compose_opts = ComposeOptions::new_with_context(ctx)
        .with_source_file(&source.resolved_path);
    if let Some(overrides) = options.set_overrides {
        compose_opts = compose_opts.with_set_overrides(overrides);
    }
    if let Some(approved) = options.pre_approved_commands {
        compose_opts = compose_opts.with_pre_approved_commands(approved);
    }
    let (composed, _report) = temp_md
        .compose_with(compose_opts)
        .map_err(|e| CompositionError::ComposeFailed(e.to_string()))?;

    let effective_frontmatter = frontmatter_to_value(composed.frontmatter());
    let effective_agent_hint = composed.frontmatter().as_map().get("agent").cloned();
    let lifecycle = parse_lifecycle_config(&effective_frontmatter)?;

    let mut prompt = composed.content().to_string();

    let source_repo_root = find_git_root_from_path(&source.resolved_path);

    let guardrails = load_or_create_guardrails(source_repo_root.as_deref());
    prompt.push_str("\n\n");
    prompt.push_str(&guardrails);

    let original_body_hash = source.markdown.hash_body(false);

    Ok(PreparedComposition {
        mode: CompositionMode::InlineFrontmatterPrompt,
        resolved_path: source.resolved_path.clone(),
        source_repo_root,
        prompt,
        effective_frontmatter,
        effective_agent_hint,
        closure: CompositionClosurePlan::Inline(InlineClosurePlan {
            original_document_text: source.original_text.clone(),
            original_body_hash,
        }),
        lifecycle,
    })
}
```

- [ ] **Step 2: Update the `mod.rs` re-exports**

In `claudine/lib/src/composition/mod.rs`, update the prepare re-export:

```rust
pub use prepare::{PrepareOptions, prepare_direct, prepare_inline};
```

- [ ] **Step 3: Update call sites in `commands/compose.rs`**

In `claudine/cli/src/commands/compose.rs`, update `run_compose_inner`:

```rust
    let prepared = composition::prepare_direct(
        &source,
        composition::PrepareOptions {
            set_overrides,
            pre_approved_commands: Some(preflight.approved_commands),
            ..Default::default()
        },
    )
    .map_err(|e| eyre!("{e}"))?;
```

And `run_inline_compose_inner`:

```rust
    let prepared = composition::prepare_inline(
        &source,
        composition::PrepareOptions {
            set_overrides,
            pre_approved_commands: Some(preflight.approved_commands),
            ..Default::default()
        },
    )
    .map_err(|e| eyre!("{e}"))?;
```

- [ ] **Step 4: Update existing tests in `prepare.rs`**

Update all test call sites from:
```rust
prepare_direct(&source, None, None)
```
to:
```rust
prepare_direct(&source, PrepareOptions::default())
```

And from:
```rust
prepare_inline(&source, None, None)
```
to:
```rust
prepare_inline(&source, PrepareOptions::default())
```

And from:
```rust
prepare_direct(&source, set_overrides, Some(commands))
```
to the equivalent `PrepareOptions` struct construction.

- [ ] **Step 5: Run all composition tests**

Run: `cargo test -p claudine --lib composition -- --nocapture`
Expected: All existing tests continue to PASS.

- [ ] **Step 6: Commit**

```bash
git add claudine/lib/src/composition/prepare.rs claudine/lib/src/composition/mod.rs claudine/cli/src/commands/compose.rs
git commit -m "refactor(claudine): accept PrepareOptions struct in prepare_direct/prepare_inline"
```

---

## Task 5: Extract Reusable Single-Step Executor from `wrap/composition.rs`

**Files:**
- Modify: `claudine/cli/src/commands/wrap/composition.rs`

The existing `execute_composition_request` function does both provider selection and execution. The sequence orchestrator needs the execution part without re-running provider selection for each step.

The refactor introduces:
- `SingleCompositionOutcome` — a richer return type that includes exit code, provider, and selection reason
- `execute_single_composition` — the inner execution function that the sequence loop can call per step

- [ ] **Step 1: Add `SingleCompositionOutcome` struct**

Add at the top of `claudine/cli/src/commands/wrap/composition.rs` (after the imports):

```rust
/// Result of executing a single composition step through the wrapper pipeline.
pub(crate) struct SingleCompositionOutcome {
    /// The process exit code.
    pub exit_code: i32,
    /// The provider that ran the step.
    pub provider: Provider,
    /// Why this provider was selected.
    pub selection_reason: SelectionReason,
}
```

- [ ] **Step 2: Refactor `execute_composition_request` to return `SingleCompositionOutcome`**

Rename the existing `execute_composition_request` to `execute_composition_request_inner` returning `Result<SingleCompositionOutcome>`, then keep the public `execute_composition_request` as a thin wrapper that extracts just the exit code:

```rust
pub(crate) fn execute_composition_request(
    request: CompositionExecutionRequest,
    verbose: u8,
) -> Result<i32> {
    let outcome = execute_composition_request_inner(request, verbose)?;
    Ok(outcome.exit_code)
}

/// Execute a composition request and return the full structured outcome.
///
/// Used by the sequence orchestrator to get per-step provider and exit
/// information without losing the richer return value.
pub(crate) fn execute_composition_request_inner(
    request: CompositionExecutionRequest,
    verbose: u8,
) -> Result<SingleCompositionOutcome> {
    // ... (move the entire existing body of execute_composition_request here,
    //      but change the final Ok(exit_code) returns to
    //      Ok(SingleCompositionOutcome { exit_code, provider, selection_reason: selected.reason }))
```

At each exit point that currently returns `Ok(exit_code)`, change to:
```rust
Ok(SingleCompositionOutcome {
    exit_code,
    provider,
    selection_reason: selected.reason,
})
```

There are three such exit points in the function (harness, inline-without-harness, direct-without-harness).

- [ ] **Step 3: Verify all existing tests still pass**

Run: `cargo test -p claudine-cli 2>&1 | tail -10`
Run: `cargo check -p claudine-cli 2>&1 | tail -5`
Expected: Compiles and all tests pass.

- [ ] **Step 4: Commit**

```bash
git add claudine/cli/src/commands/wrap/composition.rs
git commit -m "refactor(claudine): extract SingleCompositionOutcome for sequence reuse"
```

---

## Task 6: Create CLI Command (`commands/sequence.rs`)

**Files:**
- Create: `claudine/cli/src/commands/sequence.rs`
- Modify: `claudine/cli/src/commands/mod.rs`
- Modify: `claudine/cli/src/args.rs`
- Modify: `claudine/cli/src/main.rs`

- [ ] **Step 1: Create `commands/sequence.rs`**

```rust
//! Top-level `claudine sequence <file>` command.
//!
//! Resolves the source document, detects and validates the sequence
//! definition, then delegates to the sequence orchestrator for serial
//! step execution.

use clap::Args;
use claudine::composition::{self, SequenceExecutionOptions};
use color_eyre::eyre::{Result, eyre};

use super::compose::SharedComposeArgs;
use crate::log;

/// Run a Markdown document as a serial sequence of composition steps.
#[derive(Debug, Clone, Args)]
pub struct SequenceArgs {
    #[command(flatten)]
    pub shared: SharedComposeArgs,

    /// File reference to the sequence document.
    #[arg(value_name = "FILE")]
    pub file: String,

    /// Override the document's fail-fast behavior for this run.
    ///
    /// When true (default), stop on the first failed step. When false,
    /// continue through all steps and report a mixed-result summary.
    /// Accepts: true, false, 1, 0, yes, no.
    #[arg(long = "fail-fast", value_name = "BOOL", value_parser = parse_boolish)]
    pub fail_fast: Option<bool>,
}

/// Parse a boolish string: true/false, 1/0, yes/no.
fn parse_boolish(s: &str) -> Result<bool, String> {
    match s.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => Err(format!("invalid boolean value '{s}'; expected true/false, 1/0, or yes/no")),
    }
}

/// Entry point for `claudine sequence`.
pub fn run_sequence(args: SequenceArgs, verbose: u8) -> Result<()> {
    let code = match run_sequence_inner(args, verbose) {
        Ok(code) => code,
        Err(error) => {
            log::error(&error.to_string());
            1
        }
    };
    std::process::exit(code);
}

fn run_sequence_inner(args: SequenceArgs, verbose: u8) -> Result<i32> {
    let SequenceArgs {
        shared,
        file,
        fail_fast,
    } = args;

    let source = composition::resolve_composition_source(&file).map_err(|e| eyre!("{e}"))?;

    // Detect and validate the sequence plan
    let plan = composition::resolve_sequence_plan(&source)
        .map_err(|e| eyre!("{e}"))?
        .ok_or_else(|| {
            eyre!(
                "file '{}' does not define a `sequence` frontmatter property",
                file
            )
        })?;

    let set_json_str = shared.set.clone();
    let set_overrides = super::compose::parse_set_json(set_json_str.as_deref())?;

    let execution_options = SequenceExecutionOptions {
        fail_fast_override: fail_fast,
    };

    super::wrap::sequence::execute_sequence(
        &source,
        plan,
        &shared,
        set_overrides,
        execution_options,
        verbose,
    )
}
```

- [ ] **Step 2: Make `parse_set_json` accessible from the sequence command**

In `claudine/cli/src/commands/compose.rs`, change the visibility of the existing function from `fn` to `pub(crate) fn`:

```rust
pub(crate) fn parse_set_json(raw: Option<&str>) -> Result<Option<serde_json::Value>> {
```

- [ ] **Step 3: Add `pub mod sequence;` to `commands/mod.rs`**

In `claudine/cli/src/commands/mod.rs`, add:

```rust
pub mod sequence;
```

- [ ] **Step 4: Add `Sequence` variant to `Commands` enum**

In `claudine/cli/src/args.rs`, add after the `InlineCompose` variant:

```rust
    /// Run a serial sequence of composition steps from a single document.
    Sequence(commands::sequence::SequenceArgs),
```

- [ ] **Step 5: Add dispatch in `main.rs`**

In `claudine/cli/src/main.rs`, add after the `Commands::InlineCompose` match arm:

```rust
        Commands::Sequence(args) => commands::sequence::run_sequence(args, cli.verbose),
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo check -p claudine-cli 2>&1 | tail -5`
Expected: Compiles (the wrap/sequence module doesn't exist yet, but we can stub it).

Note: Step 6 may fail if `wrap::sequence` doesn't exist yet. If so, create the stub in the next task and come back to verify.

- [ ] **Step 7: Commit**

```bash
git add claudine/cli/src/commands/sequence.rs claudine/cli/src/commands/mod.rs claudine/cli/src/commands/compose.rs claudine/cli/src/args.rs claudine/cli/src/main.rs
git commit -m "feat(claudine): add claudine sequence CLI command and arg parsing"
```

---

## Task 7: Create Sequence Orchestrator (`wrap/sequence.rs`)

**Files:**
- Create: `claudine/cli/src/commands/wrap/sequence.rs`
- Modify: `claudine/cli/src/commands/wrap/mod.rs`

This is the core orchestration loop that drives serial step execution.

- [ ] **Step 1: Create `wrap/sequence.rs`**

```rust
//! Serial sequence orchestrator.
//!
//! Drives the outer step loop for `claudine sequence`. Each step is a
//! normal composition run executed through the wrapper-grade pipeline.

use std::collections::BTreeSet;
use std::collections::HashSet;

use biscuit_terminal::prelude::Renderable;
use claudine::composition::{
    self, CompositionExecutionRequest, CompositionMode, PrepareOptions,
    SequenceExecutionOptions, SequencePlan, SequenceRunSummary, SequenceStepResult,
    SystemPromptInput,
};
use claudine::composition::sequence::build_step_overlay;
use claudine::composition::types::ResolvedCompositionSource;
use color_eyre::eyre::{Result, eyre};

use crate::commands::compose::SharedComposeArgs;
use crate::log;

/// Execute a full sequence: iterate steps, compose each, and report results.
pub(crate) fn execute_sequence(
    source: &ResolvedCompositionSource,
    plan: SequencePlan,
    shared: &SharedComposeArgs,
    user_set_overrides: Option<serde_json::Value>,
    execution_options: SequenceExecutionOptions,
    verbose: u8,
) -> Result<i32> {
    let term = super::wrap_terminal();
    let silent = shared.silent;
    let quiet = shared.quiet;

    let effective_fail_fast = execution_options
        .fail_fast_override
        .unwrap_or(plan.document_fail_fast);

    let total_steps = plan.steps.len();

    if !silent {
        log::info(&format!(
            "Sequence: {} step(s), fail_fast={}",
            total_steps, effective_fail_fast
        ));
    }

    let mut summary = SequenceRunSummary {
        total_steps,
        succeeded: 0,
        failed: 0,
        steps: Vec::with_capacity(total_steps),
    };

    // Accumulate pre-approved commands across steps so "allow once" persists.
    let mut cumulative_approved: HashSet<String> = HashSet::new();

    for step_index in 0..total_steps {
        let step = &plan.steps[step_index];
        let overlay = build_step_overlay(&plan, step_index);

        if !silent {
            log::info(&format!(
                "[{}/{}] {}",
                step_index + 1,
                total_steps,
                step.name
            ));
        }

        let start = std::time::Instant::now();

        // Build per-step set_overrides: user --set merged with overlay (overlay wins)
        let step_set_overrides = overlay.as_set_overrides(user_set_overrides.clone());

        // Build per-step env overrides
        let mut env_overrides = std::collections::BTreeMap::new();
        env_overrides.insert(
            "FAIL_FAST".to_string(),
            effective_fail_fast.to_string(),
        );

        // Pre-flight shell approval for this step
        let compose_options = {
            let mut opts = darkmatter::markdown::compose::ComposeOptions::new()
                .with_source_file(&source.resolved_path);
            opts = opts.with_set_overrides(step_set_overrides.clone());
            opts
        };

        let approval_options = super::build_harness_shell_options(
            &source.resolved_path,
            None,
            shared.interactive,
        );

        let preflight = composition::resolve_shell_approvals(
            Some(&source.markdown),
            Some(&compose_options),
            None,
            &approval_options,
        )
        .map_err(|e| eyre!("{e}"))?;

        // Merge cumulative approvals
        cumulative_approved.extend(preflight.approved_commands.iter().cloned());

        let prepare_options = PrepareOptions {
            set_overrides: Some(step_set_overrides),
            pre_approved_commands: Some(cumulative_approved.clone()),
            env_overrides,
        };

        // Prepare the composition for this step
        let prepared = match composition::prepare_direct(source, prepare_options) {
            Ok(p) => p,
            Err(e) => {
                let duration = start.elapsed();
                let error_msg = e.to_string();
                if !silent {
                    log::error(&format!(
                        "step {}/{} failed: {}",
                        step_index + 1,
                        total_steps,
                        error_msg
                    ));
                }
                summary.failed += 1;
                summary.steps.push(SequenceStepResult {
                    step: step_index + 1,
                    name: step.name.clone(),
                    success: false,
                    error: Some(error_msg),
                    duration,
                });
                if effective_fail_fast {
                    break;
                }
                continue;
            }
        };

        let system_prompt = shared
            .system_prompt
            .as_ref()
            .map(|prompt| SystemPromptInput::Inline {
                prompt: prompt.clone(),
            })
            .or_else(|| {
                shared
                    .system_prompt_file
                    .as_ref()
                    .map(|path| SystemPromptInput::File { path: path.clone() })
            });

        let request = CompositionExecutionRequest {
            mode: CompositionMode::ChainedDocument,
            file_ref: source.original_ref.clone(),
            prepared,
            explicit_provider: shared.explicit_provider(),
            excluded: shared.excluded(),
            yolo: shared.yolo,
            include: shared.include.clone(),
            model: shared.model.clone(),
            output: shared.output,
            system_prompt,
            timeout: shared.timeout,
            operation: shared.operation.clone(),
            sandbox: shared.sandbox,
            repo: shared.repo,
            dry_run: shared.dry_run,
            mcp: shared.mcp,
            mcp_use: shared.mcp_use.clone(),
            strict: shared.strict,
            session_interactive: shared.interactive,
            quiet: shared.quiet,
            silent: shared.silent,
        };

        let step_result =
            super::composition::execute_composition_request(request, verbose);

        let duration = start.elapsed();

        match step_result {
            Ok(exit_code) if exit_code == 0 => {
                summary.succeeded += 1;
                summary.steps.push(SequenceStepResult {
                    step: step_index + 1,
                    name: step.name.clone(),
                    success: true,
                    error: None,
                    duration,
                });
                if !silent {
                    log::info(&format!(
                        "step {}/{} succeeded",
                        step_index + 1,
                        total_steps
                    ));
                }
            }
            Ok(exit_code) => {
                let error_msg = format!("provider exited with code {exit_code}");
                summary.failed += 1;
                summary.steps.push(SequenceStepResult {
                    step: step_index + 1,
                    name: step.name.clone(),
                    success: false,
                    error: Some(error_msg.clone()),
                    duration,
                });
                if !silent {
                    log::error(&format!(
                        "step {}/{} failed: {}",
                        step_index + 1,
                        total_steps,
                        error_msg
                    ));
                }
                if effective_fail_fast {
                    break;
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                summary.failed += 1;
                summary.steps.push(SequenceStepResult {
                    step: step_index + 1,
                    name: step.name.clone(),
                    success: false,
                    error: Some(error_msg.clone()),
                    duration,
                });
                if !silent {
                    log::error(&format!(
                        "step {}/{} failed: {}",
                        step_index + 1,
                        total_steps,
                        error_msg
                    ));
                }
                if effective_fail_fast {
                    break;
                }
            }
        }
    }

    // -- Final summary --------------------------------------------------------

    if !silent {
        eprintln!();
        if summary.failed == 0 {
            log::info(&format!(
                "Sequence finished: {} succeeded, 0 failed",
                summary.succeeded
            ));
        } else {
            log::error(&format!(
                "Sequence finished: {} succeeded, {} failed",
                summary.succeeded, summary.failed
            ));
        }
    }

    if summary.failed > 0 { Ok(1) } else { Ok(0) }
}
```

- [ ] **Step 2: Expose `explicit_provider()` and `excluded()` on `SharedComposeArgs`**

In `claudine/cli/src/commands/compose.rs`, change the visibility of these methods from private to `pub(crate)`:

```rust
    pub(crate) fn explicit_provider(&self) -> Option<Provider> {
```

```rust
    pub(crate) fn excluded(&self) -> BTreeSet<Provider> {
```

- [ ] **Step 3: Add `pub(crate) mod sequence;` to `wrap/mod.rs`**

In `claudine/cli/src/commands/wrap/mod.rs`, add:

```rust
pub(crate) mod sequence;
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p claudine-cli 2>&1 | tail -10`
Expected: Compiles without errors.

- [ ] **Step 5: Commit**

```bash
git add claudine/cli/src/commands/wrap/sequence.rs claudine/cli/src/commands/wrap/mod.rs claudine/cli/src/commands/compose.rs
git commit -m "feat(claudine): add sequence orchestrator with serial step loop"
```

---

## Task 8: Add Library Tests for Prepare Env Overrides

**Files:**
- Modify: `claudine/lib/src/composition/prepare.rs` — add test

- [ ] **Step 1: Write the test**

Add to the existing `tests` module in `prepare.rs`:

```rust
    #[test]
    fn direct_composition_with_env_overrides() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[("title", json!("Test"))],
            "FAIL_FAST is {{env.FAIL_FAST}}",
        );

        let options = PrepareOptions {
            env_overrides: std::collections::BTreeMap::from([(
                "FAIL_FAST".to_string(),
                "false".to_string(),
            )]),
            ..Default::default()
        };

        let prepared = prepare_direct(&source, options).unwrap();
        assert!(prepared.prompt.contains("FAIL_FAST is false"));
    }
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p claudine --lib composition::prepare::tests::direct_composition_with_env_overrides -- --nocapture`
Expected: PASS — the env override is injected via `ComposeContext::env_mut()`.

- [ ] **Step 3: Commit**

```bash
git add claudine/lib/src/composition/prepare.rs
git commit -m "test(claudine): verify env_overrides work in prepare_direct"
```

---

## Task 9: Verify Full Build and Run All Tests

**Files:** None (verification only)

- [ ] **Step 1: Run the full library test suite**

Run: `cargo test -p claudine -- --nocapture 2>&1 | tail -20`
Expected: All tests PASS.

- [ ] **Step 2: Run the CLI check**

Run: `cargo check -p claudine-cli 2>&1 | tail -5`
Expected: Compiles without errors.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -p claudine -p claudine-cli -- -D warnings 2>&1 | tail -20`
Expected: No warnings.

- [ ] **Step 4: Run fmt check**

Run: `cargo fmt --package claudine --package claudine-cli -- --check 2>&1 | tail -10`
Expected: No formatting issues.

---

## Task 10: Update Documentation

**Files:**
- Modify: `claudine/docs/topics/composition.md`

- [ ] **Step 1: Add sequence section to composition docs**

Add a new `## Sequence Composition` section to `claudine/docs/topics/composition.md` covering:

1. What sequence composition is and when to use it
2. The `claudine sequence <file>` command
3. Inline sequence definition (scalar and object forms)
4. External YAML sequence definition (both `sequence:` and `kind/list/template` forms)
5. Template evaluation rules
6. Available step variables (`state`, `previous_state`, `next_state`, `is_first`, `is_last`, `step`, `total_steps`)
7. Fail-fast behavior (document default vs `--fail-fast` CLI override)
8. The `FAIL_FAST` environment variable
9. Error handling semantics

- [ ] **Step 2: Commit**

```bash
git add claudine/docs/topics/composition.md
git commit -m "docs(claudine): document sequence composition feature"
```
