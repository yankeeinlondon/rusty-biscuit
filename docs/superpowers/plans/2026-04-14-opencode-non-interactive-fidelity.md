# OpenCode Non-Interactive Fidelity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix four observable defects in `claudine opencode` non-interactive sessions so the status stream accurately reflects OpenCode's completion-only `tool_use` semantics, surfaces tool parameters in the completion line, and renders a graceful terminal-error Status when the user aborts the session with Ctrl+C.

**Architecture:** OpenCode's `run --format json` NDJSON emits `tool_use` *only after* a tool has reached `completed` or `error` (see `claudine/docs/research/non-interactive-sessions/opencode.md:81-82`). Today the OpenCode semantic parser maps `tool_use` onto `SemanticEvent::ToolCall` alone, which (a) renders a `→` "running" icon, (b) leaves the live heartbeat's `in_flight` map growing unbounded (producing "N running" reports for already-complete tools), and (c) never surfaces a completion line that carries the input summary. This plan splits `tool_use` handling into a paired `ToolCall + ToolResult` emission that derives status/output from `part.state`, extends the live sink so tool-completion lines carry a one-line input summary (so parameters are visible), and adds an explicit Ctrl+C short-circuit in the wrapper that emits a `Status::Failure` line via the existing `StreamOutput` before returning.

**Tech Stack:** Rust, serde, tokio, biscuit-terminal (Status / StatusState / StatusTheme), claudine stream + wrap modules.

---

## File Map

| File | Change | Purpose |
|------|--------|---------|
| `claudine/lib/src/stream/opencode_semantic.rs` | Modify | Split `tool_use` handling so it emits paired `ToolCall` + `ToolResult`; keep `tool_start`/`tool_end` on their existing single-event paths |
| `claudine/lib/src/stream/opencode_semantic.rs` | Modify (tests) | Golden event-sequence tests proving `tool_use` now produces paired events; `tool_start` + `tool_end` unchanged |
| `claudine/cli/src/commands/wrap/live_semantic_sink.rs` | Modify | Extend `tool_result_description` to include a one-line input summary when available; thread cached input through `handle_tool_result` path via the existing `tool_uses` map |
| `claudine/lib/src/stream/opencode_semantic.rs` | Modify | Cache the input on `ToolCall` emit and re-attach it to the `ToolResult` `extra["input"]` field so the sink can surface params on the completion line |
| `claudine/cli/src/commands/wrap/mod.rs` | Modify | On `ProcessTermination::Interrupted`, render a `Status::Failure` (circular theme) line via `StreamOutput` describing the interrupt before the existing `guard.emit_terminal(Failure)` call |
| `claudine/cli/src/output.rs` (or nearest helper module) | Modify | Add `format_user_interrupt_status()` helper so the Ctrl+C message is formatted once and testable |
| `claudine/cli/tests/wrap_commands.rs` | Modify | Snapshot / assertion test that the interrupt helper renders the expected Status line |

---

## Preconditions

- [ ] **Step 0: Confirm working tree is clean on the `claudine` branch**

Run: `git status --short`
Expected: only the pre-existing modifications listed in the session's `gitStatus` snapshot (`.vscode/settings.json`, `claudine/cli/src/commands/compose.rs`, `just/features.just`, `unchained-ai/README.md`) — nothing under `claudine/lib/src/stream/` or `claudine/cli/src/commands/wrap/`.

If other files are dirty, stop and surface to the user before continuing.

---

### Task 1: Split `tool_use` handling from `tool_start` in the OpenCode parser

**Files:**
- Modify: `claudine/lib/src/stream/opencode_semantic.rs:228-288` (`handle_tool_use`, `handle_tool_result`, and the dispatch match arm at `:354-359`)
- Modify: `claudine/lib/src/stream/opencode_semantic.rs` (tests module at bottom — add a new test for the paired emit)

- [ ] **Step 1: Write the failing test — `tool_use` produces both a `ToolCall` and a `ToolResult`**

Append to the `mod tests` block at the bottom of `claudine/lib/src/stream/opencode_semantic.rs`:

```rust
#[test]
fn opencode_tool_use_emits_paired_call_and_result() {
    use super::super::parser::SemanticStreamParser;
    use super::super::semantic::{SemanticEvent, SemanticEventSink};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Capture(Arc<Mutex<Vec<SemanticEvent>>>);
    impl SemanticEventSink for Capture {
        fn on_semantic_event(&mut self, event: SemanticEvent) {
            self.0.lock().unwrap().push(event);
        }
    }

    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Capture(events.clone());
    let mut parser = OpenCodeSemanticStreamParser::new(sink, Some("gpt-4o".into()));

    // Real OpenCode wire format: single `tool_use` event after tool completion,
    // carrying the input AND the completed status inside `part.state`.
    parser
        .feed_line(
            r#"{"type":"tool_use","part":{"id":"t1","tool":"bash",
                 "state":{"status":"completed","input":{"command":"ls -la"},"output":"file.txt"}}}"#,
        )
        .unwrap();

    let captured = events.lock().unwrap().clone();
    let kinds: Vec<&str> = captured.iter().map(|e| e.kind_str()).collect();
    assert_eq!(
        kinds,
        vec!["tool_call", "tool_result"],
        "tool_use must emit a paired ToolCall followed by ToolResult"
    );

    let SemanticEvent::ToolCall { name, input, .. } = &captured[0] else {
        panic!("expected ToolCall first");
    };
    assert_eq!(name.as_deref(), Some("bash"));
    assert_eq!(
        input
            .as_ref()
            .and_then(|v| v.get("command"))
            .and_then(|v| v.as_str()),
        Some("ls -la")
    );

    let SemanticEvent::ToolResult { name, status, .. } = &captured[1] else {
        panic!("expected ToolResult second");
    };
    assert_eq!(name.as_deref(), Some("bash"));
    assert_eq!(status.as_deref(), Some("completed"));
}
```

- [ ] **Step 2: Run the failing test**

Run: `cargo test -p claudine --lib stream::opencode_semantic::tests::opencode_tool_use_emits_paired_call_and_result -- --nocapture`
Expected: FAIL. The assertion `kinds == ["tool_call", "tool_result"]` should fail because the current parser only emits a single `tool_call` event for `tool_use`.

- [ ] **Step 3: Split the dispatch match arm**

In `claudine/lib/src/stream/opencode_semantic.rs`, replace the paired match arm:

```rust
            Ok(OpenCodeEvent::ToolUse(tool) | OpenCodeEvent::ToolStart(tool)) => {
                self.handle_tool_use(tool, &raw_kind);
            }
```

with two separate arms so `tool_use` and `tool_start` take different paths:

```rust
            Ok(OpenCodeEvent::ToolUse(tool)) => {
                self.handle_tool_use_completed(tool, &raw_kind);
            }
            Ok(OpenCodeEvent::ToolStart(tool)) => {
                self.handle_tool_use(tool, &raw_kind);
            }
```

- [ ] **Step 4: Implement `handle_tool_use_completed`**

Add the new method directly below the existing `handle_tool_use` in `claudine/lib/src/stream/opencode_semantic.rs`:

```rust
    /// Handle OpenCode's `tool_use` event, which per the run.ts contract is
    /// only emitted *after* a tool reaches `completed` or `error`. We emit a
    /// ToolCall so the live sink counts the invocation and renders the input
    /// preview, then immediately emit a matching ToolResult so the sink's
    /// in-flight accounting doesn't leak (it used to grow forever because
    /// nothing removed the entry — surfaced to the user as "N running" for
    /// long-finished tools).
    fn handle_tool_use_completed(&mut self, tool: OpenCodeTool, raw_kind: &str) {
        self.tool_calls += 1;
        let resolved = tool.resolve();
        super::trace_tool_event(
            Provider::OpenCode,
            self.tool_calls,
            resolved.name.as_deref(),
        );

        // First: ToolCall carrying the input (for the "→ bash · ls -la" line).
        let mut call_extra = self.base_extra(raw_kind);
        if let Some(id) = &resolved.id {
            call_extra.insert("tool_id".into(), Value::from(id.as_str()));
        }
        if let Some(name) = &resolved.name {
            call_extra.insert("tool_name".into(), Value::from(name.as_str()));
        }
        self.sink.on_semantic_event(SemanticEvent::ToolCall {
            name: resolved.name.clone(),
            id: resolved.id.clone(),
            input: resolved.input.clone(),
            extra: Value::Object(call_extra),
        });

        // Second: ToolResult so in_flight accounting balances and the
        // completion line renders with `← bash · completed`.
        let mut result_extra = self.base_extra(raw_kind);
        if let Some(id) = &resolved.id {
            result_extra.insert("tool_id".into(), Value::from(id.as_str()));
        }
        if let Some(name) = &resolved.name {
            result_extra.insert("tool_name".into(), Value::from(name.as_str()));
        }
        if let Some(status) = &resolved.status {
            result_extra.insert("status".into(), Value::from(status.as_str()));
        }
        if let Some(err) = &resolved.error {
            result_extra.insert("error".into(), err.clone());
        }
        // Preserve the original input alongside the result so sink-level
        // renderers can include a parameter preview on the completion line
        // (Task 3).
        if let Some(input) = &resolved.input {
            result_extra.insert("input".into(), input.clone());
        }

        self.sink.on_semantic_event(SemanticEvent::ToolResult {
            name: resolved.name,
            id: resolved.id,
            status: resolved.status,
            exit_code: None,
            output: resolved.output,
            extra: Value::Object(result_extra),
        });
    }
```

- [ ] **Step 5: Run the target test again to verify it passes**

Run: `cargo test -p claudine --lib stream::opencode_semantic::tests::opencode_tool_use_emits_paired_call_and_result -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Run the full OpenCode parser test module to catch regressions**

Run: `cargo test -p claudine --lib stream::opencode_semantic::tests`
Expected: all tests pass, including the pre-existing `opencode_tool_result_from_part_content_and_status` / `opencode_tool_all_fields_from_part` / `opencode_tool_args_params_aliases` cases.

- [ ] **Step 7: Commit**

```bash
git add claudine/lib/src/stream/opencode_semantic.rs
git commit -m "fix(claudine): emit paired ToolCall+ToolResult for OpenCode tool_use

OpenCode's run --format json only emits tool_use *after* the tool has
completed or errored, not before. Mapping it to ToolCall alone caused
the live sink's in_flight map to grow without bound and surfaced as
\"N running\" heartbeat lines for already-completed tools. Split the
dispatch so tool_use emits a paired ToolCall + ToolResult and tool_start
keeps its original pre-completion semantics."
```

---

### Task 2: Verify golden stderr snapshot still covers OpenCode

**Files:**
- Modify: `claudine/cli/src/commands/wrap/live_semantic_sink.rs:1054-1070` (existing `opencode_stderr_snapshot` test — adjust to also exercise the new paired-emit path)

The existing `opencode_stderr_snapshot` fixture at line 1057-1061 uses `tool_start` + `tool_end`, which keeps the original path intact. Add a second fixture that exercises `tool_use` to lock in the fix end-to-end through the sink.

- [ ] **Step 1: Write the failing test**

Add a new test in the same `golden_stderr` module in `claudine/cli/src/commands/wrap/live_semantic_sink.rs`, directly below `opencode_stderr_snapshot`:

```rust
        #[test]
        fn opencode_tool_use_completion_shows_both_arrows() {
            let lines = replay_to_stderr(Provider::OpenCode, &[
                r#"{"type":"step_start","sessionID":"ses_1"}"#,
                r#"{"type":"tool_use","part":{"id":"t1","tool":"bash",
                     "state":{"status":"completed","input":{"command":"ls -la"},"output":"file.txt"}}}"#,
            ], Some("gpt-4o".into()));
            let joined = lines.join("\n");
            assert!(
                joined.contains('\u{2192}') && joined.contains('\u{2190}'),
                "expected both → and ← arrows in {joined:?}"
            );
            assert!(joined.contains("bash"));
            assert!(
                joined.contains("ls -la"),
                "expected input preview on the tool line: {joined:?}"
            );
        }
```

- [ ] **Step 2: Run it and verify it passes**

Run: `cargo test -p claudine --bin claudine commands::wrap::live_semantic_sink::tests::golden_stderr::opencode_tool_use_completion_shows_both_arrows -- --nocapture`
Expected: PASS — the paired emit from Task 1 is what makes both arrows appear.

- [ ] **Step 3: Commit**

```bash
git add claudine/cli/src/commands/wrap/live_semantic_sink.rs
git commit -m "test(claudine): lock in paired-arrow rendering for OpenCode tool_use"
```

---

### Task 3: Surface tool parameters on the completion (`←`) line

**Files:**
- Modify: `claudine/cli/src/commands/wrap/live_semantic_sink.rs:280-293` (`tool_result_description`)
- Modify: `claudine/cli/src/commands/wrap/live_semantic_sink.rs:395-405` (`render_event` call site — pass `extra["input"]` through)
- Modify: `claudine/cli/src/commands/wrap/live_semantic_sink.rs` (tests)

Rationale: the `→` line already carries an input preview via `tool_call_description`. Issue 3 of the bug report is about being able to see which tool ran *with what arguments* on the single completion line OpenCode emits. Because Task 1 preserves the original input in `extra["input"]` on every `ToolResult`, the sink can pull it out without changing the `SemanticEvent::ToolResult` struct.

- [ ] **Step 1: Write the failing unit test**

Add to the existing `mod tests` block in `claudine/cli/src/commands/wrap/live_semantic_sink.rs`, directly below `tool_result_renders_arrow_left_prefix_with_exit_code`:

```rust
    #[test]
    fn tool_result_renders_input_summary_when_extra_input_present() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::ToolResult {
            name: Some("bash".into()),
            id: Some("t1".into()),
            status: Some("completed".into()),
            exit_code: None,
            output: None,
            extra: json!({ "input": { "command": "ls -la" } }),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(rendered.contains('\u{2190}'), "expected ← arrow");
        assert!(rendered.contains("bash"), "expected tool name");
        assert!(
            rendered.contains("ls -la"),
            "expected input preview on the completion line: {rendered:?}"
        );
        assert!(
            rendered.contains("completed"),
            "expected status label: {rendered:?}"
        );
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p claudine --bin claudine commands::wrap::live_semantic_sink::tests::tool_result_renders_input_summary_when_extra_input_present -- --nocapture`
Expected: FAIL. The current `tool_result_description` only shows `← bash · completed` and ignores `extra`.

- [ ] **Step 3: Extend `tool_result_description` to accept an optional input summary**

Change the signature and body in `claudine/cli/src/commands/wrap/live_semantic_sink.rs:280-293` from:

```rust
    fn tool_result_description(
        name: &Option<String>,
        status: &Option<String>,
        exit_code: Option<i32>,
    ) -> String {
        let name_part = name.as_deref().unwrap_or("(tool)");
        let mut parts = vec![format!("\u{2190} {name_part}")];
        if let Some(code) = exit_code {
            parts.push(format!("exit {code}"));
        } else if let Some(s) = status {
            parts.push(s.clone());
        }
        parts.join(" \u{00b7} ")
    }
```

to:

```rust
    fn tool_result_description(
        name: &Option<String>,
        status: &Option<String>,
        exit_code: Option<i32>,
        input: Option<&Value>,
    ) -> String {
        let name_part = name.as_deref().unwrap_or("(tool)");
        let mut parts = vec![format!("\u{2190} {name_part}")];
        if let Some(summary) = input.and_then(summarize_input) {
            parts.push(summary);
        }
        if let Some(code) = exit_code {
            parts.push(format!("exit {code}"));
        } else if let Some(s) = status {
            parts.push(s.clone());
        }
        parts.join(" \u{00b7} ")
    }
```

- [ ] **Step 4: Update the `ToolResult` render call site to pass `extra["input"]`**

In `render_event` (`claudine/cli/src/commands/wrap/live_semantic_sink.rs:395-405`), change:

```rust
            SemanticEvent::ToolResult {
                name,
                status,
                exit_code,
                ..
            } => {
                self.render_status(
                    StatusState::ToolUse,
                    Self::tool_result_description(name, status, *exit_code),
                );
            }
```

to:

```rust
            SemanticEvent::ToolResult {
                name,
                status,
                exit_code,
                extra,
                ..
            } => {
                let input = extra.get("input");
                self.render_status(
                    StatusState::ToolUse,
                    Self::tool_result_description(name, status, *exit_code, input),
                );
            }
```

- [ ] **Step 5: Fix the existing `tool_result_renders_arrow_left_prefix_with_exit_code` test signature**

That test at `claudine/cli/src/commands/wrap/live_semantic_sink.rs:630-647` constructs a `ToolResult` with `extra: json!({})`, so it already passes `None` to the new `input` parameter via `extra.get("input")`. No change needed to that test's body, but re-run it in Step 6 to confirm.

- [ ] **Step 6: Run both tests to verify pass**

Run:

```bash
cargo test -p claudine --bin claudine commands::wrap::live_semantic_sink::tests::tool_result_renders_input_summary_when_extra_input_present -- --nocapture
cargo test -p claudine --bin claudine commands::wrap::live_semantic_sink::tests::tool_result_renders_arrow_left_prefix_with_exit_code -- --nocapture
```

Expected: both PASS.

- [ ] **Step 7: Run the full wrap test module to guard other callers**

Run: `cargo test -p claudine --bin claudine commands::wrap::live_semantic_sink`
Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add claudine/cli/src/commands/wrap/live_semantic_sink.rs
git commit -m "feat(claudine): surface tool input preview on completion line

When a ToolResult event carries the original input under extra[\"input\"]
(as OpenCode tool_use pairs now do), include a truncated one-line preview
on the `← tool · <input> · <status>` rendering so operators can see what
the tool was invoked with. Other providers continue to render as before
because they don't populate extra[\"input\"]."
```

---

### Task 4: Render a graceful `Status::Failure` on Ctrl+C termination

**Files:**
- Create / Modify: `claudine/cli/src/output.rs` (add `format_user_interrupt_status()`)
- Modify: `claudine/cli/src/commands/wrap/mod.rs:2640-2645` (call the new helper before `emit_terminal(Failure)`)
- Modify: `claudine/cli/tests/wrap_commands.rs` (smoke test the helper)

Rationale: today `ProcessTermination::Interrupted` silently returns the child exit code (130) and calls `guard.emit_terminal(Failure)`. The user has no visible signal that Claudine saw the interrupt. We need a single stderr `Status` line with the circular theme's Failure icon describing the cause, rendered via the same `StreamOutput` the heartbeat uses so it lands cleanly after any in-flight heartbeat line.

- [ ] **Step 1: Locate the output helper module**

Run: `rg --files-with-matches "format_session_start" claudine/cli/src`
Expected: `claudine/cli/src/output.rs` (or equivalent). The existing `format_session_start` referenced at `live_semantic_sink.rs:329` confirms this module is the right home for Status formatters.

- [ ] **Step 2: Write the failing helper test**

Append to `claudine/cli/tests/wrap_commands.rs`:

```rust
#[test]
fn format_user_interrupt_status_renders_failure_icon_and_message() {
    use biscuit_terminal::prelude::strip_escape_codes;

    let rendered = claudine_cli::output::format_user_interrupt_status();
    let plain = strip_escape_codes(&rendered);
    assert!(
        plain.contains("User terminated non-interactive session with CTRL+C"),
        "missing expected interrupt message: {plain:?}"
    );
    // Circular Failure theme renders a filled red circle glyph. We don't
    // pin the exact codepoint, but the line must not look like a generic
    // info line.
    assert!(
        !plain.contains('\u{2192}') && !plain.contains('\u{2190}'),
        "interrupt status must not reuse tool-call arrows: {plain:?}"
    );
}
```

If `claudine_cli` is not publicly exposed, use the existing integration-test pattern (spawning the binary) or promote the helper to a `pub fn` in the same module that houses `format_session_start`. Check `claudine/cli/src/output.rs` for the existing `pub` surface and match its style.

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test -p claudine-cli --test wrap_commands format_user_interrupt_status_renders_failure_icon_and_message -- --nocapture`
Expected: FAIL — `format_user_interrupt_status` does not exist yet.

- [ ] **Step 4: Implement `format_user_interrupt_status`**

Add to `claudine/cli/src/output.rs` (next to the existing `format_session_start`):

```rust
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};

/// Format a terminal-error Status line announcing that the user aborted
/// the wrapped session with Ctrl+C. Rendered via the circular theme so the
/// failure icon matches Claudine's other terminal-error surfaces, and it
/// reads cleanly on any terminal that routes through [`crate::log::terminal`].
pub fn format_user_interrupt_status() -> String {
    Status::new("User terminated non-interactive session with CTRL+C")
        .state(StatusState::Failure)
        .theme(StatusTheme::Circular)
        .render(&crate::log::terminal())
}
```

If the `Status` builder does not expose `theme()` as a fluent setter, use whatever constructor pattern `format_session_start` uses for its Status instance and mirror it. Confirm by reading `format_session_start` first.

- [ ] **Step 5: Run the helper test to verify it passes**

Run: `cargo test -p claudine-cli --test wrap_commands format_user_interrupt_status_renders_failure_icon_and_message -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Wire the helper into the interrupt short-circuit**

In `claudine/cli/src/commands/wrap/mod.rs:2640-2645`, change:

```rust
        if outcome.termination == claudine::harness::ProcessTermination::Interrupted {
            guard.emit_terminal(LifecycleSignal::Failure);
            return Ok(outcome.exit_code);
        }
```

to:

```rust
        if outcome.termination == claudine::harness::ProcessTermination::Interrupted {
            // Surface the interrupt to the user before we let the guard
            // close: without this the wrapper would silently return 130
            // and the operator has no feedback that Claudine noticed.
            eprintln!("{}", crate::output::format_user_interrupt_status());
            guard.emit_terminal(LifecycleSignal::Failure);
            return Ok(outcome.exit_code);
        }
```

- [ ] **Step 7: Run the full wrap integration test crate to catch regressions**

Run: `cargo test -p claudine-cli --test wrap_commands`
Expected: all tests pass, including the new one.

- [ ] **Step 8: Manual verification (best-effort)**

Using a dev build, start any long-running opencode wrap (`cargo run -p claudine-cli -- opencode -p "sleep 60"` or equivalent that exercises a real opencode binary), wait for the heartbeat to tick, press Ctrl+C once, and confirm that the final stderr line is the circular-failure Status "User terminated non-interactive session with CTRL+C" rather than a bare exit. If the opencode binary is not installed locally, note this in the plan's closing commit and rely on the unit-test coverage plus the existing `ProcessTermination::Interrupted` signal-handling test at `exec.rs`.

- [ ] **Step 9: Commit**

```bash
git add claudine/cli/src/output.rs claudine/cli/src/commands/wrap/mod.rs claudine/cli/tests/wrap_commands.rs
git commit -m "feat(claudine): render Status::Failure on wrap Ctrl+C interrupt

When the wrapped provider is interrupted with Ctrl+C the wrapper used to
return exit 130 silently, leaving the operator with a spinner frozen on
the last heartbeat line. Emit a circular-theme Status::Failure via
StreamOutput with the message \"User terminated non-interactive session
with CTRL+C\" before the lifecycle guard fires its terminal Failure."
```

---

### Task 5: Regression sweep

- [ ] **Step 1: Run the OpenCode-facing cargo lint**

Run: `cargo clippy -p claudine -p claudine-cli --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 2: Run the area just recipe**

Run: `just claudine test`
Expected: all tests pass. If the recipe runs `cargo nextest`, the new tests will show under their parent modules.

- [ ] **Step 3: Run the workspace root test orchestration for claudine only**

Run: `cargo test -p claudine -p claudine-cli`
Expected: all tests pass.

- [ ] **Step 4: Manual stream replay sanity check**

Run (ad-hoc, from the repo root) a quick replay using an existing sample if one is present under `claudine/lib/tests/fixtures/opencode` or similar. If no fixture exists, skip this step. Document in the final commit whether this step was runnable.

- [ ] **Step 5: Final commit if anything surfaced**

If any of the regression sweeps produced doc updates (for example the `claudine/docs/research/non-interactive-sessions/opencode.md` needing a note that Claudine handles `tool_use` as completion-only), amend the relevant docs in a separate commit:

```bash
git add <changed doc paths>
git commit -m "docs(claudine): note paired-emit handling for OpenCode tool_use"
```

Otherwise no-op.

---

## Out of Scope

- Waiting for OpenCode PR `#18249` (live `tool_use` progress). If/when it lands, `handle_tool_use_completed` should grow a status-dependent branch (`running` → ToolCall only, `completed`/`error` → paired emit). That is a follow-up, not part of this plan.
- Re-tuning the heartbeat cadence. The bug was caused by stale `in_flight` entries, which Task 1 fixes at the source.
- Propagating the Ctrl+C status line to the sequence / composition surfaces. Those call `run_child_stream_semantic` through a different top-level driver; the same fix pattern would apply, but the current bug report is scoped to `claudine opencode`.
