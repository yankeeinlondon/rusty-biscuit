# Codex Permission Requests Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface Codex permission-gate activity (PermissionRequest, ApprovalRequest, UserInputRequest) as session counters in `StreamExecutionSummary`, the wrapper stderr summary, and the synthetic JSONL `SessionEnd` summary event.

**Architecture:** Add two provider-neutral optional counters — `permission_prompts` (counts `PermissionRequest` + `ApprovalRequest`) and `user_input_prompts` (counts `UserInputRequest`) — to `StreamExecutionSummary`. The Codex parser increments variant-sensitive counters during `item.started` and populates the summary in `finish()`. Wrapper prose (`format_summary_prose`) and the library stderr helpers append new clauses to the existing primary line. The reporting mapper adds both keys to `extra` in the synthetic `SessionEnd` event. All other providers leave the fields `None`, and `skip_serializing_if = "Option::is_none"` keeps JSON output unchanged.

**Tech Stack:** Rust 2024 edition · serde / serde_json · `cargo test -p claudine` and `cargo test -p claudine-cli` · existing typed Codex protocol in `claudine/lib/src/stream/protocol/codex.rs`.

**Reference documents:**
- `claudine/features/_unscheduled/codex-permission-requests/spec.md`
- `claudine/features/_unscheduled/codex-permission-requests/tech-design.md`

---

## Task 1: Add summary fields + serde tests

**Files:**
- Modify: `claudine/lib/src/stream/summary.rs`

- [ ] **Step 1: Add failing round-trip test for new fields**

Append this test to the `mod tests` block at the bottom of `claudine/lib/src/stream/summary.rs`:

```rust
    #[test]
    fn permission_counters_round_trip() {
        let summary = StreamExecutionSummary {
            permission_prompts: Some(3),
            user_input_prompts: Some(1),
            ..Default::default()
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"permission_prompts\":3"));
        assert!(json.contains("\"user_input_prompts\":1"));
        let restored: StreamExecutionSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.permission_prompts, Some(3));
        assert_eq!(restored.user_input_prompts, Some(1));
    }

    #[test]
    fn permission_counters_skip_none() {
        let summary = StreamExecutionSummary::default();
        let json = serde_json::to_string(&summary).unwrap();
        assert!(!json.contains("permission_prompts"));
        assert!(!json.contains("user_input_prompts"));
    }
```

- [ ] **Step 2: Run the tests to confirm they fail**

Run: `cargo test -p claudine --lib stream::summary::tests::permission_counters`

Expected: compile error (`no field 'permission_prompts' on type 'StreamExecutionSummary'`).

- [ ] **Step 3: Add the two optional fields to `StreamExecutionSummary`**

In `claudine/lib/src/stream/summary.rs`, locate the `tool_calls` field (around line 64) and insert the two new fields immediately after it:

```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_prompts: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_input_prompts: Option<u32>,
```

- [ ] **Step 4: Update `Default` impl**

In the same file, update `impl Default for StreamExecutionSummary` to include the two new fields. Find the `tool_calls: None,` line and insert directly after it:

```rust
            tool_calls: None,
            permission_prompts: None,
            user_input_prompts: None,
```

- [ ] **Step 5: Update the existing `serde_round_trip_full` test literal**

That test (around line 122) constructs a full `StreamExecutionSummary`. Add the two new fields immediately after `tool_calls: Some(5),`:

```rust
            tool_calls: Some(5),
            permission_prompts: None,
            user_input_prompts: None,
```

(The value `None` here is intentional — the full round-trip test already covers populated optional fields via `token_usage` etc.; the dedicated `permission_counters_round_trip` test above covers populated cases for these new fields.)

- [ ] **Step 6: Run the full summary module tests to confirm they pass**

Run: `cargo test -p claudine --lib stream::summary::tests`

Expected: all tests pass, including `permission_counters_round_trip` and `permission_counters_skip_none`.

- [ ] **Step 7: Commit**

```bash
git add claudine/lib/src/stream/summary.rs
git commit -m "feat(claudine): add permission_prompts and user_input_prompts to StreamExecutionSummary"
```

---

## Task 2: Codex parser — variant-sensitive permission counters

**Files:**
- Modify: `claudine/lib/src/stream/codex.rs`

- [ ] **Step 1: Add failing parser tests**

Append these tests inside `#[cfg(test)] mod tests` in `claudine/lib/src/stream/codex.rs`, after the existing `tool_counting` test:

```rust
    #[test]
    fn permission_request_increments_permission_prompts() {
        let mut parser = make_parser();
        parser
            .feed_line(
                r#"{"type":"item.started","item":{"id":"perm-1","type":"permission_request","name":"bash"}}"#,
            )
            .unwrap();
        let summary = parser.finish(0);
        assert_eq!(summary.permission_prompts, Some(1));
        assert_eq!(summary.user_input_prompts, None);
    }

    #[test]
    fn approval_request_increments_permission_prompts() {
        let mut parser = make_parser();
        parser
            .feed_line(
                r#"{"type":"item.started","item":{"id":"appr-1","type":"approval_request","name":"write"}}"#,
            )
            .unwrap();
        let summary = parser.finish(0);
        assert_eq!(summary.permission_prompts, Some(1));
        assert_eq!(summary.user_input_prompts, None);
    }

    #[test]
    fn user_input_request_increments_user_input_prompts() {
        let mut parser = make_parser();
        parser
            .feed_line(
                r#"{"type":"item.started","item":{"id":"input-1","type":"user_input_request","name":"clarify"}}"#,
            )
            .unwrap();
        let summary = parser.finish(0);
        assert_eq!(summary.user_input_prompts, Some(1));
        assert_eq!(summary.permission_prompts, None);
    }

    #[test]
    fn mixed_permission_variants_roll_up_independently() {
        let mut parser = make_parser();
        parser
            .feed_line(
                r#"{"type":"item.started","item":{"id":"perm-1","type":"permission_request","name":"bash"}}"#,
            )
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"item.started","item":{"id":"appr-1","type":"approval_request","name":"write"}}"#,
            )
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"item.started","item":{"id":"input-1","type":"user_input_request","name":"clarify"}}"#,
            )
            .unwrap();
        let summary = parser.finish(0);
        assert_eq!(summary.permission_prompts, Some(2));
        assert_eq!(summary.user_input_prompts, Some(1));
    }

    #[test]
    fn no_permission_activity_leaves_counters_none() {
        let mut parser = make_parser();
        parser.feed_line(r#"{"type":"turn.started"}"#).unwrap();
        parser
            .feed_line(
                r#"{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":5},"status":"completed"}"#,
            )
            .unwrap();
        let summary = parser.finish(0);
        assert_eq!(summary.permission_prompts, None);
        assert_eq!(summary.user_input_prompts, None);
    }
```

- [ ] **Step 2: Run tests to confirm they fail**

Run: `cargo test -p claudine --lib stream::codex::tests::permission_request_increments_permission_prompts stream::codex::tests::approval_request_increments_permission_prompts stream::codex::tests::user_input_request_increments_user_input_prompts stream::codex::tests::mixed_permission_variants_roll_up_independently stream::codex::tests::no_permission_activity_leaves_counters_none`

Expected: compile error (`no field 'permission_prompts' on type 'StreamExecutionSummary'` is gone since Task 1, but parser does not populate it → the four populated-counter assertions fail at runtime).

- [ ] **Step 3: Add counter fields to `CodexStreamParser`**

In `claudine/lib/src/stream/codex.rs`, locate the struct definition (around line 22) and add two new fields directly after `tool_calls: u32,`:

```rust
    tool_calls: u32,
    permission_prompts: u32,
    user_input_prompts: u32,
```

- [ ] **Step 4: Initialize new fields in `new()`**

Find the `Self { ... }` block inside `impl CodexStreamParser::new` (around line 43) and insert after `tool_calls: 0,`:

```rust
            tool_calls: 0,
            permission_prompts: 0,
            user_input_prompts: 0,
```

- [ ] **Step 5: Update `permission_meta` to take a `kind` discriminator and emit it**

Replace the existing `permission_meta` method (currently around line 186) with this signature and body:

```rust
    fn permission_meta(&self, perm: &CodexPermissionItem, kind: &str) -> EventMeta {
        let mut meta = self.session_meta();
        if let Some(name) = perm.name.as_deref() {
            meta.extra
                .insert("tool_name".into(), Value::String(name.to_string()));
        }
        if let Some(id) = perm.id.as_deref() {
            meta.extra
                .insert("tool_id".into(), Value::String(id.to_string()));
        }
        meta.extra
            .insert("permission_kind".into(), Value::String(kind.to_string()));
        meta
    }
```

- [ ] **Step 6: Replace the permission branch in `handle_item_started` with a variant-sensitive match**

In `claudine/lib/src/stream/codex.rs`, locate `handle_item_started` (around line 199). Replace the current `if let Some(perm) = item.as_permission() { ... return; }` block with an explicit match on the three permission variants. The final method body should look like:

```rust
    fn handle_item_started(&mut self, env: CodexItemEnvelope) {
        let Some(item) = env.item else {
            return;
        };

        match &item {
            CodexItem::PermissionRequest(perm) | CodexItem::ApprovalRequest(perm) => {
                self.permission_prompts += 1;
                let meta = self.permission_meta(perm, "permission_prompt");
                self.sink.on_permission_request(&meta);
                return;
            }
            CodexItem::UserInputRequest(perm) => {
                self.user_input_prompts += 1;
                let meta = self.permission_meta(perm, "user_input_prompt");
                self.sink.on_permission_request(&meta);
                return;
            }
            _ => {}
        }

        if item.is_tool_item() {
            let fields = item
                .as_tool_fields()
                .expect("is_tool_item implies tool fields");
            self.tool_calls += 1;
            super::trace_tool_event(
                Provider::Codex,
                self.tool_calls,
                fields.resolved_tool_name(),
            );
            let meta = self.tool_meta_from_fields(fields);
            if let Some(id) = fields.id.clone()
                && let Some(owned_fields) = item.into_tool_fields()
            {
                self.tool_items.insert(id, owned_fields);
            }
            self.sink.on_before_tool(&meta);
        }
    }
```

Note: the old `item.as_permission()` helper is no longer used in this method, but it remains valid API surface and is still exercised by the existing protocol-layer test `codex_item_permission_request_typed`. Do not remove it.

- [ ] **Step 7: Populate the new summary fields in `finish()`**

In `claudine/lib/src/stream/codex.rs`, locate `finish` (around line 333). After the existing `tool_calls:` field in the `StreamExecutionSummary { ... }` struct literal (around line 353), add:

```rust
            tool_calls: if self.tool_calls > 0 {
                Some(self.tool_calls)
            } else {
                None
            },
            permission_prompts: if self.permission_prompts > 0 {
                Some(self.permission_prompts)
            } else {
                None
            },
            user_input_prompts: if self.user_input_prompts > 0 {
                Some(self.user_input_prompts)
            } else {
                None
            },
```

- [ ] **Step 8: Run codex parser tests to confirm they pass**

Run: `cargo test -p claudine --lib stream::codex::tests`

Expected: all tests pass, including the five new tests from Step 1 and the pre-existing happy-path / tool-counting / error-handling tests.

- [ ] **Step 9: Commit**

```bash
git add claudine/lib/src/stream/codex.rs
git commit -m "feat(claudine): count Codex permission and user-input prompts in stream parser"
```

---

## Task 3: Wrapper summary prose — append permission clauses

**Files:**
- Modify: `claudine/cli/src/commands/wrap/mod.rs`

- [ ] **Step 1: Add failing prose tests**

Append these tests to the existing `#[cfg(test)] mod tests` block in `claudine/cli/src/commands/wrap/mod.rs` (the same block that contains `format_summary_prose_appends_badge_markup`):

```rust
    #[test]
    fn format_summary_prose_renders_permission_prompts_singular() {
        let summary = claudine::stream::summary::StreamExecutionSummary {
            duration_ms: Some(18_000),
            tool_calls: Some(4),
            permission_prompts: Some(1),
            ..Default::default()
        };
        let rendered = super::format_summary_prose(&summary).unwrap();
        assert!(rendered.contains("1 <i>permission prompt</i>"));
        assert!(!rendered.contains("permission prompts"));
    }

    #[test]
    fn format_summary_prose_renders_permission_prompts_plural() {
        let summary = claudine::stream::summary::StreamExecutionSummary {
            duration_ms: Some(18_000),
            tool_calls: Some(4),
            permission_prompts: Some(3),
            ..Default::default()
        };
        let rendered = super::format_summary_prose(&summary).unwrap();
        assert!(rendered.contains("3 <i>permission prompts</i>"));
    }

    #[test]
    fn format_summary_prose_renders_user_input_prompts_singular() {
        let summary = claudine::stream::summary::StreamExecutionSummary {
            duration_ms: Some(18_000),
            user_input_prompts: Some(1),
            ..Default::default()
        };
        let rendered = super::format_summary_prose(&summary).unwrap();
        assert!(rendered.contains("1 <i>user input prompt</i>"));
        assert!(!rendered.contains("user input prompts"));
    }

    #[test]
    fn format_summary_prose_renders_both_counters() {
        let summary = claudine::stream::summary::StreamExecutionSummary {
            duration_ms: Some(41_000),
            tool_calls: Some(12),
            permission_prompts: Some(2),
            user_input_prompts: Some(1),
            ..Default::default()
        };
        let rendered = super::format_summary_prose(&summary).unwrap();
        assert!(rendered.contains("2 <i>permission prompts</i>"));
        assert!(rendered.contains("1 <i>user input prompt</i>"));
    }

    #[test]
    fn format_summary_prose_omits_permission_clauses_when_unset() {
        let summary = claudine::stream::summary::StreamExecutionSummary {
            duration_ms: Some(18_000),
            tool_calls: Some(4),
            ..Default::default()
        };
        let rendered = super::format_summary_prose(&summary).unwrap();
        assert!(!rendered.contains("permission"));
        assert!(!rendered.contains("user input"));
    }
```

- [ ] **Step 2: Run tests to confirm they fail**

Run: `cargo test -p claudine-cli --lib commands::wrap::tests::format_summary_prose_renders_permission_prompts_singular commands::wrap::tests::format_summary_prose_renders_permission_prompts_plural commands::wrap::tests::format_summary_prose_renders_user_input_prompts_singular commands::wrap::tests::format_summary_prose_renders_both_counters commands::wrap::tests::format_summary_prose_omits_permission_clauses_when_unset`

Expected: the four populated-counter tests fail (the rendered prose does not yet include the new clauses). The `_omits_permission_clauses_when_unset` test should already pass but running the suite confirms the behavior.

(If the test module is named differently — for example `super::tests` rather than `commands::wrap::tests` — use the exact names reported by `cargo test -p claudine-cli --lib format_summary_prose` to target them.)

- [ ] **Step 3: Append permission clauses to `format_summary_prose`**

In `claudine/cli/src/commands/wrap/mod.rs`, locate `format_summary_prose` (around line 3391). Insert the following block immediately after the `match summary.tool_calls { ... }` block (around line 3431) and before the `if parts.is_empty()` check:

```rust
    if let Some(pp) = summary.permission_prompts {
        parts.push(format!(
            "{pp} <i>permission prompt{}</i>",
            if pp == 1 { "" } else { "s" }
        ));
    }

    if let Some(uip) = summary.user_input_prompts {
        parts.push(format!(
            "{uip} <i>user input prompt{}</i>",
            if uip == 1 { "" } else { "s" }
        ));
    }
```

- [ ] **Step 4: Run the wrap prose tests to confirm they pass**

Run: `cargo test -p claudine-cli --lib format_summary_prose`

Expected: all prose tests pass, including the five new ones and the pre-existing `format_summary_prose_appends_badge_markup` / `format_summary_prose_without_badges_has_no_badge_markup` tests.

- [ ] **Step 5: Commit**

```bash
git add claudine/cli/src/commands/wrap/mod.rs
git commit -m "feat(claudine): render Codex permission counters in wrapper summary prose"
```

---

## Task 4: Library stderr formatter — mirror permission_prompts in normal completion

**Files:**
- Modify: `claudine/lib/src/stream/stderr.rs`

Scope: `format_completion_summary` surfaces **permission_prompts only** (not user_input_prompts) to keep the library-level summary terse. `format_compact_completion` remains unchanged so `--quiet` stays one line. The wrapper prose (Task 3) is the authoritative place to see both counters.

- [ ] **Step 1: Add failing stderr formatter tests**

Append these tests to the `#[cfg(test)] mod tests` block in `claudine/lib/src/stream/stderr.rs`:

```rust
    #[test]
    fn completion_summary_includes_permission_prompts() {
        let summary = StreamExecutionSummary {
            duration_ms: Some(18_000),
            tool_calls: Some(4),
            permission_prompts: Some(3),
            ..Default::default()
        };
        let rendered = format_completion_summary(&summary).unwrap();
        assert!(rendered.contains("3 permission prompts"));
    }

    #[test]
    fn completion_summary_singular_permission_prompt() {
        let summary = StreamExecutionSummary {
            duration_ms: Some(18_000),
            permission_prompts: Some(1),
            ..Default::default()
        };
        let rendered = format_completion_summary(&summary).unwrap();
        assert!(rendered.contains("1 permission prompt"));
        assert!(!rendered.contains("permission prompts"));
    }

    #[test]
    fn completion_summary_omits_permission_when_none() {
        let summary = StreamExecutionSummary {
            duration_ms: Some(18_000),
            tool_calls: Some(4),
            ..Default::default()
        };
        let rendered = format_completion_summary(&summary).unwrap();
        assert!(!rendered.contains("permission"));
    }

    #[test]
    fn compact_completion_ignores_permission_prompts() {
        let summary = StreamExecutionSummary {
            duration_ms: Some(18_000),
            permission_prompts: Some(3),
            user_input_prompts: Some(1),
            ..Default::default()
        };
        let rendered = format_compact_completion(&summary).unwrap();
        assert!(!rendered.contains("permission"));
        assert!(!rendered.contains("user input"));
    }
```

- [ ] **Step 2: Run tests to confirm they fail**

Run: `cargo test -p claudine --lib stream::stderr::tests::completion_summary_includes_permission_prompts stream::stderr::tests::completion_summary_singular_permission_prompt stream::stderr::tests::completion_summary_omits_permission_when_none stream::stderr::tests::compact_completion_ignores_permission_prompts`

Expected: `completion_summary_includes_permission_prompts` and `completion_summary_singular_permission_prompt` fail (rendered string does not yet contain `"permission prompt"`). The other two should already pass.

- [ ] **Step 3: Append permission_prompts to `format_completion_summary`**

In `claudine/lib/src/stream/stderr.rs`, locate `format_completion_summary` (around line 49). Insert the following block immediately after the existing `if let Some(tc) = summary.tool_calls { ... }` block (around line 90) and before the `// Error info` block:

```rust
    // Permission prompts (Codex today; other providers leave None)
    if let Some(pp) = summary.permission_prompts {
        parts.push(format!(
            "{pp} permission prompt{}",
            if pp == 1 { "" } else { "s" }
        ));
    }
```

Do not modify `format_compact_completion` — compact mode intentionally omits these counters.

- [ ] **Step 4: Run stderr tests to confirm they pass**

Run: `cargo test -p claudine --lib stream::stderr::tests`

Expected: all tests pass, including the four new tests and every pre-existing test.

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/stream/stderr.rs
git commit -m "feat(claudine): surface Codex permission_prompts in library completion summary"
```

---

## Task 5: Reporting — include counters in synthetic SessionEnd extra

**Files:**
- Modify: `claudine/lib/src/stream/reporting.rs`

- [ ] **Step 1: Add failing reporting tests**

Append these tests to the `#[cfg(test)] mod tests` block in `claudine/lib/src/stream/reporting.rs`:

```rust
    #[test]
    fn summary_event_includes_permission_counters_when_populated() {
        let summary = StreamExecutionSummary {
            provider: Provider::Codex,
            permission_prompts: Some(2),
            user_input_prompts: Some(1),
            ..Default::default()
        };
        let env = EnvironmentContext::default();
        let meta = summary_to_event_meta(&summary, StreamProtocol::Jsonl, &env);
        assert_eq!(meta.extra["permission_prompts"], Value::Number(2.into()));
        assert_eq!(meta.extra["user_input_prompts"], Value::Number(1.into()));
    }

    #[test]
    fn summary_event_omits_permission_counters_when_absent() {
        let summary = StreamExecutionSummary::default();
        let env = EnvironmentContext::default();
        let meta = summary_to_event_meta(&summary, StreamProtocol::Jsonl, &env);
        assert!(!meta.extra.contains_key("permission_prompts"));
        assert!(!meta.extra.contains_key("user_input_prompts"));
    }
```

If the existing test module does not already import `StreamProtocol`, `Provider`, or `EnvironmentContext`, verify how nearby tests (e.g. the test at around line 207 that populates `tool_calls: Some(5)`) import them and mirror that pattern.

- [ ] **Step 2: Run tests to confirm they fail**

Run: `cargo test -p claudine --lib stream::reporting::tests::summary_event_includes_permission_counters_when_populated stream::reporting::tests::summary_event_omits_permission_counters_when_absent`

Expected: `summary_event_includes_permission_counters_when_populated` fails (keys are not in `extra`). The `_omits_` test should already pass.

- [ ] **Step 3: Map new fields into `extra`**

In `claudine/lib/src/stream/reporting.rs`, locate `summary_to_event_meta_with_context`. Insert the following block immediately after the existing `// Tool calls` block (around line 109) and before the `let mut provider_summary = serde_json::Map::new();` line:

```rust
    // Permission activity counters (Codex today; omitted when absent)
    if let Some(pp) = summary.permission_prompts {
        extra.insert("permission_prompts".into(), Value::Number(pp.into()));
    }
    if let Some(uip) = summary.user_input_prompts {
        extra.insert("user_input_prompts".into(), Value::Number(uip.into()));
    }
```

- [ ] **Step 4: Run reporting tests to confirm they pass**

Run: `cargo test -p claudine --lib stream::reporting::tests`

Expected: all tests pass, including the two new tests and every pre-existing test.

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/stream/reporting.rs
git commit -m "feat(claudine): include permission_prompts and user_input_prompts in synthetic session summary"
```

---

## Task 6: Full-package verification + lint

**Files:** none (verification only)

- [ ] **Step 1: Run the full claudine lib test suite**

Run: `cargo test -p claudine`

Expected: all tests pass with no warnings. If any pre-existing test fixture constructs `StreamExecutionSummary { ... }` literally (i.e. without `..Default::default()`) it must be extended with `permission_prompts: None, user_input_prompts: None,`. Use the failure output to locate and fix those sites, then re-run.

- [ ] **Step 2: Run the full claudine-cli test suite**

Run: `cargo test -p claudine-cli`

Expected: all tests pass. Apply the same fixture fix described in Step 1 if needed.

- [ ] **Step 3: Lint**

Run: `just lint` (from the repo root) or, if that recipe is not available for these crates, `cargo clippy -p claudine -p claudine-cli --all-targets -- -D warnings`

Expected: no warnings, no errors.

- [ ] **Step 4: Commit any fixup changes**

If Steps 1–3 required fixing struct-literal fixtures or minor lint hits, commit them:

```bash
git add -u
git commit -m "chore(claudine): update fixtures for new permission counter fields"
```

If nothing needed fixing, skip this step.

---

## Self-Review Checklist (author verified before handoff)

**Spec coverage:**
- spec §1 counters in Codex parser → Task 2
- spec §2 new optional summary fields → Task 1
- spec §3 summary prose line → Task 3 (wrapper) + Task 4 (library)
- spec §4 JSONL summary event → Task 5
- spec decision "keep `UserInputRequest` separate" → Task 1 field shape + Task 2 variant-sensitive dispatch
- spec decision "use neutral names" → `permission_prompts` / `user_input_prompts` (no `codex_` prefix)

**Tech design coverage:**
- §Data Model (two `Option<u32>`, `skip_serializing_if`, default `None`) → Task 1
- §Parser Design (counters, variant match, `permission_kind` meta, populate `finish()`) → Task 2
- §Wrapper Summary Rendering (append clauses, pluralization, keep in main prose line) → Task 3
- §Secondary formatter parity (`format_completion_summary` gets `permission_prompts`, compact stays clean) → Task 4
- §Synthetic JSONL Summary Event (map both fields into `extra`) → Task 5
- §Risks §1 "count only in `handle_item_started`" → Task 2 Step 6 (counters live only in that method)
- §Risks §3 "misleading denial language" → plan never introduces a `permission_denials` / `permission_approvals` field

**Out of scope (confirmed NOT in plan):**
- No `permission_denials` field (tech design §Non-Goals and §Deferred Follow-Ups)
- No SQLite schema migration (§Reporting Impact)
- No badge / warning behavior changes (§Summary prose semantics)
- No provider capability table changes (§Non-Goals)
- No `format_verbose_summary_details_prose` changes (§Wrapper Summary Rendering "Important scope boundary")

**Type consistency:**
- Field names match across tasks: `permission_prompts` / `user_input_prompts` (never renamed)
- `permission_meta(&self, perm, kind: &str)` signature in Task 2 Step 5 matches call sites in Step 6
- `CodexItem::PermissionRequest(_) | CodexItem::ApprovalRequest(_)` grouping matches `permission_prompts` semantics from spec and tech design

---

## Execution Handoff

**Plan complete and saved to `claudine/features/_unscheduled/codex-permission-requests/plan.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

**Which approach?**
