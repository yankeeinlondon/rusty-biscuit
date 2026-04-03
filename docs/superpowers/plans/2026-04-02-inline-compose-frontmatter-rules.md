# Inline Compose Frontmatter Rules Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure inline-compose preserves original frontmatter, merges agent-added properties, warns on reverted modifications, and always sets `last_updated`.

**Architecture:** The library (`closure.rs`) gains a new return type `InlineClosureResult`, a `serialize_frontmatter_property` helper, and modified signatures on `apply_inline_closure` / `rewrite_inline_document` / `upsert_last_updated_in_frontmatter` to accept and inject new properties. Both CLI call sites read post-run frontmatter, pass it to the closure, and render warnings from the result.

**Tech Stack:** Rust, `serde_json`, `biscuit_file::serde_yaml_ng`, `indexmap::IndexMap`, `biscuit_terminal::components::status::{Status, StatusState}`

---

### Task 1: Add `InlineClosureResult` type and `serialize_frontmatter_property` helper

**Files:**
- Modify: `claudine/lib/src/composition/closure.rs:1-12` (imports and public API section)
- Modify: `claudine/lib/src/composition/mod.rs:24` (re-export)

- [ ] **Step 1: Write tests for `serialize_frontmatter_property`**

Add these tests at the bottom of the existing `#[cfg(test)] mod tests` block in `closure.rs`:

```rust
// -- serialize_frontmatter_property ----------------------------------------

#[test]
fn serialize_string_property() {
    let value = serde_json::json!("hello world");
    let result = serialize_frontmatter_property("title", &value);
    assert_eq!(result, "title: hello world\n");
}

#[test]
fn serialize_number_property() {
    let result = serialize_frontmatter_property("count", &serde_json::json!(42));
    assert_eq!(result, "count: 42\n");
}

#[test]
fn serialize_bool_property() {
    let result = serialize_frontmatter_property("draft", &serde_json::json!(true));
    assert_eq!(result, "draft: true\n");
}

#[test]
fn serialize_null_property() {
    let result = serialize_frontmatter_property("removed", &serde_json::json!(null));
    assert_eq!(result, "removed: null\n");
}

#[test]
fn serialize_array_property() {
    let value = serde_json::json!(["rust", "markdown"]);
    let result = serialize_frontmatter_property("tags", &value);
    // serde_yaml_ng serializes arrays with block style
    assert!(result.starts_with("tags:\n"));
    assert!(result.contains("- rust"));
    assert!(result.contains("- markdown"));
}

#[test]
fn serialize_object_property() {
    let value = serde_json::json!({"version": "1.0"});
    let result = serialize_frontmatter_property("meta", &value);
    assert!(result.starts_with("meta:\n"));
    assert!(result.contains("version:"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p claudine -- "serialize_string_property" "serialize_number_property" "serialize_bool_property" "serialize_null_property" "serialize_array_property" "serialize_object_property"`
Expected: Compilation error — `serialize_frontmatter_property` not defined.

- [ ] **Step 3: Add `InlineClosureResult` and `serialize_frontmatter_property`**

At the top of `closure.rs`, add `indexmap` to the imports:

```rust
use indexmap::IndexMap;
```

After the existing `extract_replacement_body` function (line 38) and before `apply_inline_closure` (line 42), add:

```rust
/// Result of applying inline closure, reporting frontmatter changes.
#[derive(Debug, Clone, Default)]
pub struct InlineClosureResult {
    /// Keys that were added by the agent and merged into the document.
    pub new_properties: Vec<String>,
    /// Keys that were modified by the agent and reverted to original values.
    pub reverted_properties: Vec<String>,
}
```

In the private helpers section (after `trim_line_ending`), add:

```rust
/// Serialize a single frontmatter property as a YAML fragment.
///
/// Simple scalars produce `key: value\n`. Complex types (arrays, objects)
/// delegate to `serde_yaml_ng` for the value portion.
fn serialize_frontmatter_property(key: &str, value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => format!("{key}: {s}\n"),
        serde_json::Value::Number(n) => format!("{key}: {n}\n"),
        serde_json::Value::Bool(b) => format!("{key}: {b}\n"),
        serde_json::Value::Null => format!("{key}: null\n"),
        complex => {
            // For arrays and objects, serialize the value via serde_yaml_ng
            // then prefix the first line with the key.
            let yaml_value = biscuit_file::serde_yaml_ng::to_string(complex)
                .unwrap_or_else(|_| format!("{complex}"));
            format!("{key}:\n{yaml_value}")
        }
    }
}
```

In `claudine/lib/src/composition/mod.rs`, update the re-export line to include `InlineClosureResult`:

```rust
pub use types::{
    CompositionClosurePlan, CompositionExecutionRequest, CompositionMode, InlineClosurePlan,
    PreparedComposition, ResolvedCompositionSource, SelectedProvider, SelectionReason,
};
```

Note: `InlineClosureResult` lives in `closure.rs` (a `pub mod`), so it's already accessible as `claudine::composition::closure::InlineClosureResult`. No change to `mod.rs` re-exports is needed.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p claudine -- "serialize_"` 
Expected: All 6 tests pass.

- [ ] **Step 5: Commit**

```
feat(claudine): add InlineClosureResult type and serialize_frontmatter_property helper
```

---

### Task 2: Modify `upsert_last_updated_in_frontmatter` to inject new properties

**Files:**
- Modify: `claudine/lib/src/composition/closure.rs:156-192` (`upsert_last_updated_in_frontmatter`)

- [ ] **Step 1: Write tests for new-property injection**

Add to the test module in `closure.rs`:

```rust
// -- upsert with new properties -------------------------------------------

#[test]
fn upsert_injects_new_properties_before_last_updated() {
    let yaml = "prompt: test\nlast_updated: 2026-01-01\n";
    let new_props = vec![("tags: research\n".to_string())];
    let result = upsert_last_updated_in_frontmatter(yaml, "2026-04-02", "\n", &new_props);
    assert!(result.contains("prompt: test\n"));
    assert!(result.contains("tags: research\n"));
    assert!(result.contains("last_updated: 2026-04-02\n"));
    // tags must appear before last_updated
    let tags_pos = result.find("tags:").unwrap();
    let lu_pos = result.find("last_updated:").unwrap();
    assert!(tags_pos < lu_pos, "new property should appear before last_updated");
}

#[test]
fn upsert_injects_new_properties_when_last_updated_missing() {
    let yaml = "prompt: test\n";
    let new_props = vec!["tags: research\n".to_string()];
    let result = upsert_last_updated_in_frontmatter(yaml, "2026-04-02", "\n", &new_props);
    assert!(result.contains("tags: research\n"));
    assert!(result.contains("last_updated: 2026-04-02\n"));
    let tags_pos = result.find("tags:").unwrap();
    let lu_pos = result.find("last_updated:").unwrap();
    assert!(tags_pos < lu_pos);
}

#[test]
fn upsert_empty_new_properties_behaves_as_before() {
    let yaml = "prompt: test\nlast_updated: 2026-01-01\n";
    let no_props: Vec<String> = vec![];
    let result = upsert_last_updated_in_frontmatter(yaml, "2026-04-02", "\n", &no_props);
    assert_eq!(result, "prompt: test\nlast_updated: 2026-04-02\n");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p claudine -- "upsert_injects" "upsert_empty_new"`
Expected: Compilation error — wrong number of arguments to `upsert_last_updated_in_frontmatter`.

- [ ] **Step 3: Modify `upsert_last_updated_in_frontmatter`**

Change the signature to accept new properties:

```rust
fn upsert_last_updated_in_frontmatter(
    yaml: &str,
    today: &str,
    newline: &str,
    new_properties: &[String],
) -> String {
```

Update the body. The key change: when `last_updated` is found, inject new properties BEFORE writing the rewritten `last_updated` line. When `last_updated` is NOT found, inject new properties before appending `last_updated`:

```rust
fn upsert_last_updated_in_frontmatter(
    yaml: &str,
    today: &str,
    newline: &str,
    new_properties: &[String],
) -> String {
    let mut updated = String::with_capacity(yaml.len() + today.len() + 32);
    let mut found = false;
    let mut had_trailing_newline = yaml.is_empty();

    for line in yaml.split_inclusive('\n') {
        let line_ending = if line.ends_with("\r\n") {
            "\r\n"
        } else if line.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        let content = trim_line_ending(line);

        if let Some(rewritten) = rewrite_last_updated_line(content, today) {
            // Inject new properties just before last_updated
            for prop in new_properties {
                updated.push_str(prop);
            }
            updated.push_str(&rewritten);
            updated.push_str(line_ending);
            found = true;
        } else {
            updated.push_str(line);
        }

        had_trailing_newline = !line_ending.is_empty();
    }

    if !found {
        if !updated.is_empty() && !had_trailing_newline {
            updated.push_str(newline);
        }
        // Inject new properties before last_updated
        for prop in new_properties {
            updated.push_str(prop);
        }
        updated.push_str("last_updated: ");
        updated.push_str(today);
        updated.push_str(newline);
    }

    updated
}
```

- [ ] **Step 4: Fix existing callers of `upsert_last_updated_in_frontmatter`**

In `rewrite_inline_document` (line 79), update the call:

```rust
let yaml = upsert_last_updated_in_frontmatter(parts.yaml, today, newline, &[]);
```

- [ ] **Step 5: Fix existing tests**

The three existing `upsert_*` tests need the extra parameter. Update each call:

```rust
// In upsert_adds_when_missing:
let result = upsert_last_updated_in_frontmatter(yaml, "2026-03-27", "\n", &[]);

// In upsert_replaces_existing:
let result = upsert_last_updated_in_frontmatter(yaml, "2026-03-27", "\n", &[]);

// In upsert_ignores_indented_last_updated:
let result = upsert_last_updated_in_frontmatter(yaml, "2026-03-27", "\n", &[]);
```

- [ ] **Step 6: Run all closure tests to verify everything passes**

Run: `cargo test -p claudine -- "composition::closure"`
Expected: All tests pass (existing + new).

- [ ] **Step 7: Commit**

```
feat(claudine): support new-property injection in upsert_last_updated_in_frontmatter
```

---

### Task 3: Modify `rewrite_inline_document` and `apply_inline_closure` signatures

**Files:**
- Modify: `claudine/lib/src/composition/closure.rs:42-96` (`apply_inline_closure` and `rewrite_inline_document`)

- [ ] **Step 1: Write tests for frontmatter comparison and merge**

Add to the test module in `closure.rs`:

```rust
// -- apply_inline_closure with post-run frontmatter -----------------------

#[test]
fn rewrite_merges_new_properties_before_last_updated() {
    let original = "---\nprompt: test\nlast_updated: 2026-03-18\n---\nOld body\n";
    let new_props = vec![("tags".to_string(), "tags: research\n".to_string())];
    let rewritten =
        rewrite_inline_document(original, "New body\n", "2026-04-02", &new_props).unwrap();
    assert!(rewritten.contains("prompt: test\n"));
    assert!(rewritten.contains("tags: research\n"));
    assert!(rewritten.contains("last_updated: 2026-04-02\n"));
    assert!(rewritten.contains("New body\n"));
    let tags_pos = rewritten.find("tags:").unwrap();
    let lu_pos = rewritten.find("last_updated:").unwrap();
    assert!(tags_pos < lu_pos);
}

#[test]
fn rewrite_no_new_properties_backward_compatible() {
    let original = "---\nprompt: test\nlast_updated: 2026-03-18\n---\nOld body\n";
    let rewritten =
        rewrite_inline_document(original, "New body\n", "2026-04-02", &[]).unwrap();
    assert!(rewritten.contains("prompt: test\n"));
    assert!(rewritten.contains("last_updated: 2026-04-02\n"));
    assert!(rewritten.contains("New body\n"));
    // No extra properties injected
    assert!(!rewritten.contains("tags:"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p claudine -- "rewrite_merges" "rewrite_no_new"`
Expected: Compilation error — wrong number of arguments.

- [ ] **Step 3: Modify `rewrite_inline_document` signature**

```rust
pub fn rewrite_inline_document(
    frontmatter_source: &str,
    body: &str,
    today: &str,
    new_properties: &[(String, String)],
) -> Result<String, String> {
    if let Some(parts) = split_frontmatter_parts(frontmatter_source) {
        let newline = detect_newline(frontmatter_source);
        let prop_lines: Vec<String> = new_properties.iter().map(|(_, v)| v.clone()).collect();
        let yaml = upsert_last_updated_in_frontmatter(parts.yaml, today, newline, &prop_lines);
        let mut document = String::with_capacity(
            parts.opening.len() + yaml.len() + parts.closing.len() + body.len(),
        );
        document.push_str(parts.opening);
        document.push_str(&yaml);
        document.push_str(parts.closing);
        document.push_str(body);
        return Ok(document);
    }

    let mut markdown: darkmatter::markdown::Markdown = frontmatter_source.to_string().into();
    markdown
        .fm_insert("last_updated", today)
        .map_err(|e| format!("failed to update last_updated: {e}"))?;
    *markdown.content_mut() = body.to_string();
    Ok(markdown.as_string())
}
```

- [ ] **Step 4: Modify `apply_inline_closure` signature and implementation**

```rust
use indexmap::IndexMap;

pub fn apply_inline_closure(
    plan: &InlineClosurePlan,
    replacement_body: &str,
    target_path: &Path,
    today: &str,
    post_run_frontmatter: Option<&IndexMap<String, serde_json::Value>>,
) -> Result<InlineClosureResult, CompositionError> {
    if replacement_body.trim().is_empty() {
        return Err(CompositionError::InvalidInlineResponse(
            "replacement body is empty".into(),
        ));
    }

    let replacement_markdown: darkmatter::markdown::Markdown = replacement_body.to_string().into();
    if replacement_markdown.hash_body(false) == plan.original_body_hash {
        return Err(CompositionError::InvalidInlineResponse(
            "replacement body is unchanged".into(),
        ));
    }

    // Compare frontmatter to detect new and modified properties
    let (new_properties, reverted_properties) = match post_run_frontmatter {
        Some(post_run_fm) => compare_frontmatter(&plan.original_document_text, post_run_fm),
        None => (vec![], vec![]),
    };

    let serialized_props: Vec<(String, String)> = new_properties
        .iter()
        .filter_map(|key| {
            post_run_frontmatter
                .and_then(|fm| fm.get(key))
                .map(|value| (key.clone(), serialize_frontmatter_property(key, value)))
        })
        .collect();

    let doc_string =
        rewrite_inline_document(&plan.original_document_text, replacement_body, today, &serialized_props)
            .map_err(CompositionError::InvalidInlineResponse)?;

    crate::config::atomic::atomic_write(target_path, doc_string.as_bytes())
        .map_err(|e| CompositionError::AtomicWriteFailed(e.to_string()))?;

    Ok(InlineClosureResult {
        new_properties,
        reverted_properties,
    })
}
```

- [ ] **Step 5: Add `compare_frontmatter` helper**

In the private helpers section of `closure.rs`:

```rust
/// Compare post-run frontmatter against the original document's frontmatter.
///
/// Returns `(new_keys, modified_keys)` where:
/// - `new_keys`: present in post-run but absent in original
/// - `modified_keys`: present in both but with different values
fn compare_frontmatter(
    original_document_text: &str,
    post_run_fm: &IndexMap<String, serde_json::Value>,
) -> (Vec<String>, Vec<String>) {
    let original_md: darkmatter::markdown::Markdown = original_document_text.to_string().into();
    let original_fm = original_md.frontmatter().as_map();

    let mut new_keys = Vec::new();
    let mut modified_keys = Vec::new();

    for (key, post_value) in post_run_fm {
        // Skip last_updated — managed by the closure itself
        if key == "last_updated" {
            continue;
        }
        match original_fm.get(key) {
            None => new_keys.push(key.clone()),
            Some(original_value) if original_value != post_value => {
                modified_keys.push(key.clone());
            }
            Some(_) => {} // unchanged
        }
    }

    (new_keys, modified_keys)
}
```

- [ ] **Step 6: Fix existing callers and tests**

Update the call in `rewrite_inline_document` tests that now need the extra parameter:

```rust
// In rewrite_preserves_block_scalar_frontmatter_layout:
let rewritten = rewrite_inline_document(original, "Fresh body\n", "2026-03-19", &[]).unwrap();

// In rewrite_adds_last_updated_without_reserializing_frontmatter:
let rewritten = rewrite_inline_document(original, "Updated body\n", "2026-03-19", &[]).unwrap();

// In rewrite_updates_quoted_last_updated:
let rewritten = rewrite_inline_document(original, "New body\n", "2026-03-27", &[]).unwrap();

// In rewrite_updates_single_quoted_last_updated:
let rewritten = rewrite_inline_document(original, "New body\n", "2026-03-27", &[]).unwrap();

// In rewrite_preserves_crlf_line_endings:
let rewritten = rewrite_inline_document(original, "New body\r\n", "2026-03-27", &[]).unwrap();
```

Update `apply_inline_closure` test callers to pass `None` for post-run frontmatter:

```rust
// In apply_inline_closure_rejects_unchanged_body:
let err = apply_inline_closure(
    &plan,
    "Original body",
    Path::new("/tmp/nonexistent"),
    "2026-03-27",
    None,
)
.unwrap_err();

// In apply_closure_rejects_empty_body:
let err = apply_inline_closure(&plan, "  ", Path::new("/tmp/nonexistent"), "2026-03-27", None);
```

- [ ] **Step 7: Run all closure tests**

Run: `cargo test -p claudine -- "composition::closure"`
Expected: All tests pass.

- [ ] **Step 8: Commit**

```
feat(claudine): add frontmatter comparison and merge to inline closure
```

---

### Task 4: Update non-harness CLI path (`composition.rs`)

**Files:**
- Modify: `claudine/cli/src/commands/wrap/composition.rs:737-810`

- [ ] **Step 1: Add imports**

At the top of `composition.rs`, add the `Status` import:

```rust
use biscuit_terminal::components::status::{Status, StatusState};
```

- [ ] **Step 2: Update the non-harness inline closure call**

Replace the block starting at line 737 (`if agent_exit == 0 {`) through to the end of the closure handling (line 810). The key changes are:

1. Before calling `apply_inline_closure`, read the post-run file and parse its frontmatter
2. Pass the post-run frontmatter to `apply_inline_closure`
3. Render warnings for reverted properties and confirmations for new properties

```rust
    if agent_exit == 0 {
        let replacement_body = match claudine::composition::closure::extract_replacement_body(
            &final_response,
        ) {
            Ok(body) => body,
            Err(error) => {
                if show_checks {
                    log::message(&crate::output::fm_check_fail(
                        &format!(
                            "the referenced file -- {display_path} -- did not receive a valid replacement body: {error}"
                        ),
                        term,
                    ));
                }
                final_exit = 1;
                String::new()
            }
        };

        if final_exit == 0 {
            // Read post-run frontmatter for comparison (best-effort)
            let post_run_fm = std::fs::read_to_string(resolved_path)
                .ok()
                .map(|text| {
                    let md: darkmatter::markdown::Markdown = text.into();
                    md.frontmatter().as_map().clone()
                });

            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            match claudine::composition::closure::apply_inline_closure(
                closure_plan,
                &replacement_body,
                resolved_path,
                &today,
                post_run_fm.as_ref(),
            ) {
                Ok(result) => {
                    if show_checks {
                        log::message(&crate::output::fm_check_ok(
                            "Applied the captured replacement body to the target document",
                            term,
                        ));
                        log::message(&crate::output::fm_check_ok(
                            "Preserved original frontmatter and updated <bold>last_updated</bold>",
                            term,
                        ));

                        for key in &result.new_properties {
                            log::message(&crate::output::fm_check_ok(
                                &format!("Merged new frontmatter property <bold>\"{key}\"</bold>"),
                                term,
                            ));
                        }

                        for key in &result.reverted_properties {
                            let status = Status::from_prose(format!(
                                "Agent modified frontmatter property <b>\"{key}\"</b> — reverted to original value"
                            ))
                            .state(StatusState::Warning);
                            log::message(&status.render(term));
                        }
                    }

                    // Post-processing: run Darkmatter cleanup on the
                    // generated markdown for higher-quality output.
                    match cleanup_inline_output(resolved_path) {
                        Ok(true) => {
                            if show_checks {
                                log::message(&crate::output::fm_check_ok(
                                    "Cleaned up generated markdown formatting",
                                    term,
                                ));
                            }
                        }
                        Ok(false) => {} // no changes needed
                        Err(error) => {
                            if show_checks {
                                log::message(&crate::output::fm_check_fail(
                                    &format!("markdown cleanup failed: {error}"),
                                    term,
                                ));
                            }
                            // Non-fatal: the document was already written
                            // successfully, cleanup is a quality pass.
                        }
                    }
                }
                Err(error) => {
                    if show_checks {
                        log::message(&crate::output::fm_check_fail(
                            &format!("failed to rewrite {display_path}: {error}"),
                            term,
                        ));
                    }
                    final_exit = 1;
                }
            }
        }
    }
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p claudine-cli`
Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```
feat(claudine): merge new frontmatter and warn on reverted in non-harness inline path
```

---

### Task 5: Update harness CLI path (`mod.rs`)

**Files:**
- Modify: `claudine/cli/src/commands/wrap/mod.rs:1860-1940` (`try_inline_closure`)

- [ ] **Step 1: Add imports**

Near the top of `mod.rs`, add:

```rust
use biscuit_terminal::components::status::{Status, StatusState};
```

If `Status` or `StatusState` is already imported via another path, skip this.

- [ ] **Step 2: Update `try_inline_closure`**

The function currently returns `Result<(), Vec<ValidationFailure>>`. Update to read post-run frontmatter and pass it through, then emit warnings. The key changes:

```rust
fn try_inline_closure(
    closure_plan: &claudine::composition::InlineClosurePlan,
    final_response: &str,
    source_path: &Path,
    child_cwd: &Path,
    show_checks: bool,
    term: &Terminal,
) -> Result<(), Vec<claudine::harness::ValidationFailure>> {
    use claudine::harness::{FailurePhase, ValidationEvent, ValidationFailure, ValidationRuleId};

    let display_path = source_path
        .strip_prefix(child_cwd)
        .unwrap_or(source_path)
        .display();

    let replacement_body = match claudine::composition::closure::extract_replacement_body(
        final_response,
    ) {
        Ok(body) => body,
        Err(error) => {
            let message = format!(
                "the referenced file -- {display_path} -- did not receive a valid replacement body: {error}"
            );
            if show_checks {
                log::message(&crate::output::fm_check_fail(&message, term));
            }
            return Err(vec![ValidationFailure {
                rule_id: ValidationRuleId(9000),
                event: ValidationEvent::InlineResponseEmpty,
                phase: FailurePhase::PostCheck,
                subject_key: Some(source_path.display().to_string()),
                message,
            }]);
        }
    };

    // Read post-run frontmatter for comparison (best-effort)
    let post_run_fm = std::fs::read_to_string(source_path)
        .ok()
        .map(|text| {
            let md: darkmatter::markdown::Markdown = text.into();
            md.frontmatter().as_map().clone()
        });

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    match claudine::composition::closure::apply_inline_closure(
        closure_plan,
        &replacement_body,
        source_path,
        &today,
        post_run_fm.as_ref(),
    ) {
        Ok(result) => {
            if show_checks {
                log::message(&crate::output::fm_check_ok(
                    "Applied the captured replacement body to the target document",
                    term,
                ));
                log::message(&crate::output::fm_check_ok(
                    "Preserved original frontmatter and updated <bold>last_updated</bold>",
                    term,
                ));

                for key in &result.new_properties {
                    log::message(&crate::output::fm_check_ok(
                        &format!("Merged new frontmatter property <bold>\"{key}\"</bold>"),
                        term,
                    ));
                }

                for key in &result.reverted_properties {
                    let status = Status::from_prose(format!(
                        "Agent modified frontmatter property <b>\"{key}\"</b> — reverted to original value"
                    ))
                    .state(StatusState::Warning);
                    log::message(&status.render(term));
                }
            }
            Ok(())
        }
        Err(error) => {
            let is_unchanged = error.to_string().contains("unchanged");
            let event = if is_unchanged {
                ValidationEvent::InlineBodyUnchanged
            } else {
                ValidationEvent::InlineResponseEmpty
            };
            let message = format!("failed to rewrite {display_path}: {error}");
            if show_checks {
                log::message(&crate::output::fm_check_fail(&message, term));
            }
            Err(vec![ValidationFailure {
                rule_id: ValidationRuleId(9001),
                event,
                phase: FailurePhase::PostCheck,
                subject_key: Some(source_path.display().to_string()),
                message,
            }])
        }
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p claudine-cli`
Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```
feat(claudine): merge new frontmatter and warn on reverted in harness inline path
```

---

### Task 6: End-to-end integration tests

**Files:**
- Modify: `claudine/lib/src/composition/closure.rs` (test module)

- [ ] **Step 1: Write integration test for full closure with new and modified properties**

```rust
#[test]
fn apply_closure_merges_new_and_reports_reverted() {
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    let original = "---\nprompt: original prompt\ntitle: My Doc\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();

    let original_markdown: darkmatter::markdown::Markdown = original.to_string().into();
    let plan = InlineClosurePlan {
        original_document_text: original.to_string(),
        original_body_hash: original_markdown.hash_body(false),
    };

    // Simulate post-run state: title changed, tags added
    let mut post_run_fm = indexmap::IndexMap::new();
    post_run_fm.insert("prompt".to_string(), serde_json::json!("original prompt"));
    post_run_fm.insert("title".to_string(), serde_json::json!("Changed Title"));
    post_run_fm.insert("tags".to_string(), serde_json::json!("research"));

    let result = apply_inline_closure(
        &plan,
        "Brand new body content\n",
        &file,
        "2026-04-02",
        Some(&post_run_fm),
    )
    .unwrap();

    assert_eq!(result.new_properties, vec!["tags"]);
    assert_eq!(result.reverted_properties, vec!["title"]);

    let written = std::fs::read_to_string(&file).unwrap();
    // Original title preserved
    assert!(written.contains("title: My Doc\n"));
    // New property merged
    assert!(written.contains("tags: research\n"));
    // last_updated set
    assert!(written.contains("last_updated: 2026-04-02\n"));
    // New body applied
    assert!(written.contains("Brand new body content\n"));
    // tags appears before last_updated
    let tags_pos = written.find("tags:").unwrap();
    let lu_pos = written.find("last_updated:").unwrap();
    assert!(tags_pos < lu_pos);
}

#[test]
fn apply_closure_none_post_run_backward_compatible() {
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    let original = "---\nprompt: test\nlast_updated: 2026-01-01\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();

    let original_markdown: darkmatter::markdown::Markdown = original.to_string().into();
    let plan = InlineClosurePlan {
        original_document_text: original.to_string(),
        original_body_hash: original_markdown.hash_body(false),
    };

    let result = apply_inline_closure(
        &plan,
        "Updated body\n",
        &file,
        "2026-04-02",
        None,
    )
    .unwrap();

    assert!(result.new_properties.is_empty());
    assert!(result.reverted_properties.is_empty());

    let written = std::fs::read_to_string(&file).unwrap();
    assert!(written.contains("last_updated: 2026-04-02\n"));
    assert!(written.contains("Updated body\n"));
}
```

- [ ] **Step 2: Run all closure tests**

Run: `cargo test -p claudine -- "composition::closure"`
Expected: All tests pass.

- [ ] **Step 3: Run full claudine test suite**

Run: `cargo test -p claudine -p claudine-cli`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```
test(claudine): add integration tests for frontmatter merge and revert in inline closure
```
