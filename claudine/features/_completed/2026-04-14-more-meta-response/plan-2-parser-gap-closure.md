# Plan 2: Parser Gap Closure (Codex + Claude)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Prerequisite:** Plan 1 (hook handler hang) must ship before this plan's Task 5 manual verification. Tasks 1–4 can run in parallel with Plan 1.

**Goal:** Close the Codex and Claude protocol gaps revealed by the 2026-04-14 capture so every event in their real wire streams maps to a typed `SemanticEvent` instead of falling through to `ProviderExtension` as raw JSON. Fixes the "every event from Codex is missed" symptom, the `claude/user · {"message":{"content":[…` tool-result replay leak, and the `billing_error` → "rate limit" badge misclassification.

**Architecture:** Three protocol changes and two semantic-parser changes, all grounded in the captured fixtures at `claudine/agent-output/{codex,claude}.out`. Each change is test-first: new golden fixtures drive the tests, parsers are extended to make them pass, and round-trip fidelity keeps the `ProviderExtension` escape hatch honest. No changes to the `SemanticEvent` enum shape; no changes to the live sink or reporting layer (those stay in Plan 3). The `billing_error` vs `rate_limit` classification is resolved by honoring the `error` field that already exists in the Claude protocol model — no new heuristics.

**Tech Stack:** Rust, serde, claudine stream protocol (`claudine/lib/src/stream/protocol/{codex,claude}.rs`) and semantic parsers (`claudine/lib/src/stream/{codex,claude}_semantic.rs`).

**Captured evidence (2026-04-14):**
- `claudine/agent-output/codex.out` (64 lines): 26× `item.started` + 26× `item.completed` with `item.type = "command_execution"`, 9× `item.completed` with `item.type = "agent_message"`, 1× `thread.started`, 1× `turn.started`, 1× `turn.completed`. Real wire format uses `command` + `aggregated_output` + `exit_code` + `status` on `command_execution` items — fields the current `CodexToolItemFields` does not read.
- `claudine/agent-output/claude.out` (7 lines, session died from `billing_error`): `system/hook_started`, `system/hook_response` (no `--include-hook-events` needed — docs lag reality), `system/init`, `assistant` (carrying `error: "billing_error"` on the envelope), `result` with `is_error: true`, `permission_denials: []`, `terminal_reason: "completed"`, `fast_mode_state: "off"`.
- `claudine/claudine-output/codex.err` shows every `codex/item.*` rendered as raw-JSON truncated at 80 chars — confirming total parser miss.

---

## File Map

| File | Change | Purpose |
|------|--------|---------|
| `claudine/lib/tests/fixtures/providers/codex.ndjson` | Create | Promote the captured `agent-output/codex.out` to a permanent test fixture |
| `claudine/lib/tests/fixtures/providers/claude.ndjson` | Create | Same for Claude |
| `claudine/lib/src/stream/protocol/codex.rs` | Modify | Add `CommandExecution` as a `#[serde(rename = "command_execution")]` alias alongside `CommandExec`; extend `CodexToolItemFields` with `command` + `aggregated_output` fields and update `resolved_input`/`resolved_output` to read them as fallbacks |
| `claudine/lib/src/stream/codex_semantic.rs` | Modify | Route the new variant through existing `handle_item_started` / `handle_item_completed`; re-examine `CodexAgentMessage` shape against the fixture and fix `collected_text()` if drifted |
| `claudine/lib/src/stream/protocol/claude.rs` | Modify | Add `User` variant for `{type:"user"}` events; extend `ClaudeSystemEvent` to recognize `subtype = "hook_started" / "hook_response"`; add `error: Option<String>` to the `assistant` event envelope; extend `ClaudeResult` with `permission_denials`, `terminal_reason`, `fast_mode_state` (accepted but passed through `extra`) |
| `claudine/lib/src/stream/claude_semantic.rs` | Modify | Route `User` → `ToolResult` (correlating via `tool_use_id`); emit `SemanticEvent::Error { terminal: true }` on `assistant.error = "billing_error"` with an explicit non-rate-limit classification; route `hook_started` / `hook_response` to `SemanticEvent::Info` (not `ProviderExtension`); emit a final terminal `Error` with a clear message on `result.is_error = true` |
| `claudine/lib/src/stream/badges.rs` | Modify | Stop classifying `billing_error` as a rate-limit badge; add a distinct "billing" badge; include `rate_limit_info.status` and duration-until-`resetsAt` on genuine rate-limit badges |

---

## Preconditions

- [ ] **Step 0: Working tree check**

Run: `git status --short`
Expected: no modifications under `claudine/lib/src/stream/`, `claudine/lib/tests/`, or `claudine/claudine-output/`.

- [ ] **Step 0.1: Plan 1 readiness**

If Plan 1 has not yet shipped, Tasks 1–4 can still proceed (they only touch the library). Task 5 (manual verification with a real agent) must wait until Plan 1 is merged, because the current 30 s hook hang would mask any test-run signal.

---

## Task 1: Promote captures to permanent test fixtures

**Files:**
- Create: `claudine/lib/tests/fixtures/providers/codex.ndjson`
- Create: `claudine/lib/tests/fixtures/providers/claude.ndjson`

- [ ] **Step 1: Copy the captures**

```bash
mkdir -p claudine/lib/tests/fixtures/providers
cp claudine/agent-output/codex.out  claudine/lib/tests/fixtures/providers/codex.ndjson
cp claudine/agent-output/claude.out  claudine/lib/tests/fixtures/providers/claude.ndjson
```

Add a one-line note at the top? Do NOT — NDJSON must be pure JSONL so parsers can ingest without preprocessing. Provenance is recorded in the commit message.

- [ ] **Step 2: Verify files are well-formed NDJSON**

Run:
```bash
python3 -c "
import json
for name in ['codex', 'claude']:
    with open(f'claudine/lib/tests/fixtures/providers/{name}.ndjson') as f:
        for i, line in enumerate(f, 1):
            line = line.strip()
            if not line: continue
            try:
                json.loads(line)
            except Exception as e:
                print(f'{name} line {i}: {e}')
                raise SystemExit(1)
    print(f'{name}: ok')
"
```

Expected output: `codex: ok` and `claude: ok`. If either fails, the capture was corrupted during redirect — re-capture.

- [ ] **Step 3: Commit**

```bash
git add claudine/lib/tests/fixtures/providers/codex.ndjson \
        claudine/lib/tests/fixtures/providers/claude.ndjson
git commit -m "test(claudine): add Codex + Claude wire-format fixtures

Captured 2026-04-14 by running the prompt:

  can you find any other versions of the
  claudine/features/_unscheduled/improved-sequences/spec.md?

against codex exec --json and claude -p --output-format stream-json
--verbose. Used as input to Plan 2 parser gap-closure tests — these
fixtures encode the current real wire formats that the protocol
models must handle."
```

---

## Task 2: Codex protocol — `command_execution` + `command` / `aggregated_output` fields

**Files:**
- Modify: `claudine/lib/src/stream/protocol/codex.rs`

- [ ] **Step 1: Write the failing protocol test**

Append to the `#[cfg(test)] mod tests` block at the bottom of `claudine/lib/src/stream/protocol/codex.rs`:

```rust
    #[test]
    fn codex_command_execution_started_deserializes() {
        let line = r#"{"type":"item.started","item":{"id":"item_1","type":"command_execution","command":"/bin/zsh -lc 'ls'","aggregated_output":""}}"#;
        let event: CodexEvent = serde_json::from_str(line).expect("valid event");
        let CodexEvent::ItemStarted(env) = event else {
            panic!("expected ItemStarted");
        };
        let item = env.item.expect("item");
        assert!(matches!(item, CodexItem::CommandExecution(_) | CodexItem::CommandExec(_)),
            "expected CommandExecution or CommandExec variant, got {item:?}");
        let fields = item.as_tool_fields().expect("tool fields");
        assert_eq!(
            fields.resolved_input().and_then(|v| v.as_str()).or_else(|| {
                fields.resolved_input().and_then(|v| v.get("command")).and_then(|v| v.as_str())
            }),
            Some("/bin/zsh -lc 'ls'"),
            "command must be exposed via resolved_input (either as string value or nested command key): fields = {fields:?}"
        );
    }

    #[test]
    fn codex_command_execution_completed_exposes_output_and_status() {
        let line = r#"{"type":"item.completed","item":{"id":"item_1","type":"command_execution","command":"ls","aggregated_output":"file.txt\n","exit_code":0,"status":"success"}}"#;
        let event: CodexEvent = serde_json::from_str(line).expect("valid event");
        let CodexEvent::ItemCompleted(env) = event else { panic!("expected ItemCompleted"); };
        let item = env.item.expect("item");
        let fields = item.as_tool_fields().expect("tool fields");
        assert_eq!(fields.exit_code, Some(0));
        assert_eq!(fields.status.as_deref(), Some("success"));
        let output = fields.resolved_output().expect("output");
        assert_eq!(
            output.as_str().or_else(|| output.get("aggregated_output").and_then(|v| v.as_str())),
            Some("file.txt\n"),
            "aggregated_output must be exposed via resolved_output"
        );
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p claudine --lib stream::protocol::codex::tests::codex_command_execution_started_deserializes`
Expected: FAIL — `CodexItem::CommandExecution` variant does not exist; deserialization of `type: "command_execution"` falls into `CodexItem::Unknown`.

- [ ] **Step 3: Add the `CommandExecution` variant**

In `claudine/lib/src/stream/protocol/codex.rs`, find the `CodexItem` enum. The variants use `#[serde(tag = "type", rename_all = "snake_case")]` — so `CommandExec` currently maps to `"command_exec"` and `CommandExecution` would map to `"command_execution"`. Because `serde` with `rename_all = "snake_case"` can only map one rename per variant, use an explicit per-variant rename with `#[serde(alias = "...")]` to accept both:

```rust
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CodexItem {
    AgentMessage(CodexAgentMessage),
    ToolUse(CodexToolItemFields),
    ToolCall(CodexToolItemFields),
    McpToolCall(CodexToolItemFields),
    WebSearch(CodexToolItemFields),
    #[serde(alias = "command_execution")]
    CommandExec(CodexToolItemFields),
    PatchApply(CodexToolItemFields),
    ImageGeneration(CodexToolItemFields),
    ViewImage(CodexToolItemFields),
    PermissionRequest(CodexPermissionItem),
    ApprovalRequest(CodexPermissionItem),
    UserInputRequest(CodexPermissionItem),
    Reasoning(CodexReasoning),
    FileChange(CodexFileChange),
    PlanUpdate(CodexPlanUpdate),
    TodoList(CodexPlanUpdate),
    #[serde(other)]
    Unknown,
}
```

This keeps the existing `CommandExec` identifier stable (no ripple edit into every `match` arm) while accepting both wire strings. Per serde docs, `#[serde(alias)]` on a tagged enum variant works with `#[serde(tag = "type")]`.

- [ ] **Step 4: Extend `CodexToolItemFields` with `command` and `aggregated_output`**

Locate `CodexToolItemFields` at `claudine/lib/src/stream/protocol/codex.rs` (roughly line 443). Add two fields and update the resolvers:

```rust
#[derive(Debug, Default, Deserialize)]
pub struct CodexToolItemFields {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub input: Option<Value>,
    #[serde(default)]
    pub arguments: Option<Value>,
    #[serde(default)]
    pub parameters: Option<Value>,
    /// `command_execution` items carry the shell string here instead of
    /// structured `input`. Exposed as a fallback in `resolved_input()`
    /// wrapped inside `{"command": "…"}` so downstream renderers get a
    /// uniform object-shaped view.
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub output: Option<Value>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub content: Option<Value>,
    /// `command_execution` items carry the combined stdout+stderr here.
    /// Exposed as a string fallback in `resolved_output()`.
    #[serde(default)]
    pub aggregated_output: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub exit_code: Option<i32>,
}

impl CodexToolItemFields {
    pub fn resolved_tool_name(&self) -> Option<&str> {
        self.tool_name
            .as_deref()
            .or(self.name.as_deref())
            // command_execution items don't carry a tool_name; synthesize
            // a stable "shell" label so the renderer has something to show.
            .or_else(|| self.command.as_ref().map(|_| "shell"))
    }

    pub fn resolved_tool_id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn resolved_input(&self) -> Option<Value> {
        if let Some(v) = self.input.as_ref() {
            return Some(v.clone());
        }
        if let Some(v) = self.arguments.as_ref() {
            return Some(v.clone());
        }
        if let Some(v) = self.parameters.as_ref() {
            return Some(v.clone());
        }
        if let Some(cmd) = self.command.as_ref() {
            return Some(serde_json::json!({ "command": cmd }));
        }
        None
    }

    pub fn resolved_output(&self) -> Option<Value> {
        if let Some(v) = self.output.as_ref() {
            return Some(v.clone());
        }
        if let Some(v) = self.result.as_ref() {
            return Some(v.clone());
        }
        if let Some(v) = self.content.as_ref() {
            return Some(v.clone());
        }
        if let Some(agg) = self.aggregated_output.as_ref() {
            return Some(Value::String(agg.clone()));
        }
        None
    }

    // merge_started unchanged — but extend to also inherit `command` and
    // `aggregated_output` when missing.
    pub fn merge_started(&mut self, started: CodexToolItemFields) {
        if self.id.is_none() { self.id = started.id; }
        if self.name.is_none() { self.name = started.name; }
        if self.tool_name.is_none() { self.tool_name = started.tool_name; }
        if self.input.is_none() { self.input = started.input; }
        if self.arguments.is_none() { self.arguments = started.arguments; }
        if self.parameters.is_none() { self.parameters = started.parameters; }
        if self.command.is_none() { self.command = started.command; }
        if self.output.is_none() { self.output = started.output; }
        if self.result.is_none() { self.result = started.result; }
        if self.content.is_none() { self.content = started.content; }
        if self.aggregated_output.is_none() { self.aggregated_output = started.aggregated_output; }
    }
}
```

NOTE: `resolved_input` / `resolved_output` now return `Option<Value>` instead of `Option<&Value>` because we sometimes synthesize a new `Value` from the `command` / `aggregated_output` strings. If any existing caller expects `Option<&Value>`, update the call site — grep for `resolved_input()` and `resolved_output()` to find them. Expected call sites: `claudine/lib/src/stream/codex_semantic.rs` only. Update the two call sites to clone or own the returned `Value`.

- [ ] **Step 5: Run the protocol tests**

Run: `cargo test -p claudine --lib stream::protocol::codex::tests`
Expected: all tests pass, including the two new tests and all pre-existing ones.

- [ ] **Step 6: Commit**

```bash
git add claudine/lib/src/stream/protocol/codex.rs
git commit -m "feat(claudine): recognize Codex command_execution items

Codex's current exec --json wire format emits item.type =
'command_execution' (not 'command_exec') with command + aggregated_output
+ exit_code + status fields. The parser used to fall through to
CodexItem::Unknown and emit a raw-JSON ProviderExtension for every shell
command. Accept both wire strings via serde(alias), and extend
CodexToolItemFields to expose command as synthesized input and
aggregated_output as synthesized output so the existing semantic parser
routes these events into SemanticEvent::ToolCall / ToolResult."
```

---

## Task 3: Codex semantic parser — route `command_execution` + recheck `agent_message`

**Files:**
- Modify: `claudine/lib/src/stream/codex_semantic.rs`

- [ ] **Step 1: Write a semantic-level fixture replay test**

Append to `claudine/lib/src/stream/codex_semantic.rs` (inside its `#[cfg(test)] mod tests`):

```rust
    #[test]
    fn codex_fixture_command_execution_routes_to_tool_pair() {
        use std::sync::{Arc, Mutex};
        use crate::stream::semantic::{SemanticEvent, SemanticEventSink};
        use crate::stream::parser::SemanticStreamParser;

        #[derive(Default)]
        struct Capture(Arc<Mutex<Vec<SemanticEvent>>>);
        impl SemanticEventSink for Capture {
            fn on_semantic_event(&mut self, event: SemanticEvent) {
                self.0.lock().unwrap().push(event);
            }
        }

        let events = Arc::new(Mutex::new(Vec::new()));
        let mut parser = CodexSemanticStreamParser::new(Capture(events.clone()), Some("codex-mini".into()));

        parser.feed_line(r#"{"type":"item.started","item":{"id":"cmd1","type":"command_execution","command":"ls","aggregated_output":""}}"#).unwrap();
        parser.feed_line(r#"{"type":"item.completed","item":{"id":"cmd1","type":"command_execution","command":"ls","aggregated_output":"file.txt\n","exit_code":0,"status":"success"}}"#).unwrap();

        let captured = events.lock().unwrap().clone();
        let kinds: Vec<&str> = captured.iter().map(|e| e.kind_str()).collect();
        assert_eq!(kinds, vec!["tool_call", "tool_result"],
            "command_execution must route to paired ToolCall + ToolResult, got {kinds:?}");

        let SemanticEvent::ToolResult { status, exit_code, output, .. } = &captured[1] else {
            panic!("expected ToolResult as second event");
        };
        assert_eq!(status.as_deref(), Some("success"));
        assert_eq!(*exit_code, Some(0));
        let output = output.as_ref().expect("output");
        assert_eq!(output.as_str(), Some("file.txt\n"),
            "aggregated_output must be preserved as the ToolResult output");
    }

    #[test]
    fn codex_fixture_agent_message_does_not_leak_as_provider_extension() {
        use std::sync::{Arc, Mutex};
        use crate::stream::semantic::{SemanticEvent, SemanticEventSink};
        use crate::stream::parser::SemanticStreamParser;

        #[derive(Default)]
        struct Capture(Arc<Mutex<Vec<SemanticEvent>>>);
        impl SemanticEventSink for Capture {
            fn on_semantic_event(&mut self, event: SemanticEvent) {
                self.0.lock().unwrap().push(event);
            }
        }

        let events = Arc::new(Mutex::new(Vec::new()));
        let mut parser = CodexSemanticStreamParser::new(Capture(events.clone()), Some("codex-mini".into()));

        // Representative shape from agent-output/codex.out — single-line msg.
        parser.feed_line(r#"{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"Looking at the repo..."}}"#).unwrap();

        let captured = events.lock().unwrap().clone();
        let kinds: Vec<&str> = captured.iter().map(|e| e.kind_str()).collect();
        assert!(
            !kinds.iter().any(|k| *k == "provider_extension"),
            "agent_message must not leak to ProviderExtension; got {kinds:?}"
        );
    }
```

- [ ] **Step 2: Run to confirm they fail**

Run: `cargo test -p claudine --lib stream::codex_semantic::tests::codex_fixture_command_execution_routes_to_tool_pair stream::codex_semantic::tests::codex_fixture_agent_message_does_not_leak_as_provider_extension`
Expected: the command_execution test may already pass after Task 2 (because the protocol fix routes through existing `item.is_tool_item()` logic). The agent_message test will likely pass if `collected_text()` correctly extracts from `text`. If either unexpectedly passes after Task 2, remove the redundant test to avoid bit-rot — but run them both first to confirm.

If either fails, inspect which path was missed and patch `codex_semantic.rs` accordingly. Common likely fixes:
  - Ensure `CodexItem::CommandExec` is listed in the `is_tool_item()` matcher (it already is — confirm).
  - Verify `CodexAgentMessage::collected_text()` pulls from the `text` field when `content` is absent (grep the existing body).

- [ ] **Step 3: Replay the full captured fixture end-to-end**

Append a third test that replays the entire `codex.ndjson` fixture and asserts no line produces a `ProviderExtension`:

```rust
    #[test]
    fn codex_fixture_full_replay_produces_no_provider_extensions() {
        use std::sync::{Arc, Mutex};
        use crate::stream::semantic::{SemanticEvent, SemanticEventSink};
        use crate::stream::parser::SemanticStreamParser;

        #[derive(Default)]
        struct Capture(Arc<Mutex<Vec<SemanticEvent>>>);
        impl SemanticEventSink for Capture {
            fn on_semantic_event(&mut self, event: SemanticEvent) {
                self.0.lock().unwrap().push(event);
            }
        }

        let fixture = std::fs::read_to_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/providers/codex.ndjson"),
        )
        .expect("codex.ndjson must exist — Task 1 should have created it");

        let events = Arc::new(Mutex::new(Vec::new()));
        let mut parser = CodexSemanticStreamParser::new(Capture(events.clone()), Some("codex-mini".into()));

        for (i, line) in fixture.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() { continue; }
            parser
                .feed_line(line)
                .unwrap_or_else(|e| panic!("line {}: {:?}", i + 1, e));
        }

        let captured = events.lock().unwrap().clone();
        let ext: Vec<&SemanticEvent> = captured
            .iter()
            .filter(|e| e.kind_str() == "provider_extension")
            .collect();
        assert!(
            ext.is_empty(),
            "captured fixture must not produce ProviderExtension events; found {} out of {}: {:#?}",
            ext.len(),
            captured.len(),
            ext.iter().take(3).collect::<Vec<_>>()
        );
    }
```

Run: `cargo test -p claudine --lib stream::codex_semantic::tests::codex_fixture_full_replay_produces_no_provider_extensions`
Expected: PASS. If it fails, inspect the first `ProviderExtension` event's raw kind — that tells you exactly which Codex event type the parser still misses. Fix iteratively (likely candidates: `item.updated`, or an unexpected new shape). Any additional parser work required here is IN SCOPE for this task.

- [ ] **Step 4: Run the full codex_semantic test suite**

Run: `cargo test -p claudine --lib stream::codex_semantic`
Expected: all tests pass, including pre-existing ones.

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/stream/codex_semantic.rs
git commit -m "fix(claudine): route Codex command_execution + agent_message cleanly

End-to-end replay of the 2026-04-14 Codex fixture now produces zero
ProviderExtension events. Paired ToolCall+ToolResult for every
command_execution; agent_message text accumulates into assistant_text
without leaking a ProviderExtension line."
```

---

## Task 4: Claude protocol + semantic — `user`, `hook_*`, `billing_error`, `result` terminal errors

**Files:**
- Modify: `claudine/lib/src/stream/protocol/claude.rs`
- Modify: `claudine/lib/src/stream/claude_semantic.rs`
- Modify: `claudine/lib/src/stream/badges.rs`

- [ ] **Step 1: Write the failing protocol tests**

Append to `claudine/lib/src/stream/protocol/claude.rs` tests module:

```rust
    #[test]
    fn claude_user_event_deserializes_with_tool_result_content() {
        let line = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tu_1","content":"hello","is_error":false}]},"session_id":"s1"}"#;
        let event: ClaudeEvent = serde_json::from_str(line).expect("valid event");
        let ClaudeEvent::User(user) = event else {
            panic!("expected ClaudeEvent::User, got {event:?}");
        };
        let content = user.message.and_then(|m| m.content).expect("content");
        // content is a Vec<serde_json::Value> that we will walk in the
        // semantic parser looking for tool_result entries.
        assert!(content.iter().any(|c| c.get("type").and_then(|v| v.as_str()) == Some("tool_result")));
    }

    #[test]
    fn claude_system_hook_subtypes_deserialize() {
        for subtype in ["hook_started", "hook_response"] {
            let line = format!(
                r#"{{"type":"system","subtype":"{subtype}","session_id":"s1","hook_name":"SessionStart"}}"#
            );
            let event: ClaudeEvent = serde_json::from_str(&line).expect("valid event");
            assert!(matches!(event, ClaudeEvent::System(_)),
                "subtype {subtype} must parse as System event");
        }
    }

    #[test]
    fn claude_assistant_error_field_preserved() {
        let line = r#"{"type":"assistant","message":{"model":"<synthetic>","content":[{"type":"text","text":"Credit balance is too low"}]},"session_id":"s1","error":"billing_error"}"#;
        let event: ClaudeEvent = serde_json::from_str(line).expect("valid event");
        let ClaudeEvent::Assistant(a) = event else { panic!("expected Assistant") };
        assert_eq!(a.error.as_deref(), Some("billing_error"));
    }

    #[test]
    fn claude_result_fields_billing_error_surface() {
        let line = r#"{"type":"result","subtype":"success","is_error":true,"result":"Credit balance is too low","session_id":"s1","permission_denials":[],"terminal_reason":"completed","fast_mode_state":"off","total_cost_usd":0,"usage":{"input_tokens":0,"output_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"service_tier":"standard"},"modelUsage":{}}"#;
        let event: ClaudeEvent = serde_json::from_str(line).expect("valid event");
        let ClaudeEvent::Result(r) = event else { panic!("expected Result") };
        assert_eq!(r.is_error, Some(true));
        assert_eq!(r.result.as_deref(), Some("Credit balance is too low"));
    }
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p claudine --lib stream::protocol::claude::tests::claude_user_event_deserializes_with_tool_result_content stream::protocol::claude::tests::claude_system_hook_subtypes_deserialize stream::protocol::claude::tests::claude_assistant_error_field_preserved`
Expected: FAIL on each — variants / fields missing.

- [ ] **Step 3: Extend the protocol model**

In `claudine/lib/src/stream/protocol/claude.rs`:

Add a `User` variant to `ClaudeEvent`:

```rust
pub enum ClaudeEvent {
    #[serde(rename = "init")]
    Init(ClaudeInit),
    #[serde(rename = "system")]
    System(ClaudeInit),
    #[serde(rename = "user")]
    User(ClaudeUser),
    #[serde(rename = "assistant")]
    Assistant(ClaudeAssistant),
    // ... rest unchanged
}

#[derive(Debug, Default, Deserialize)]
pub struct ClaudeUser {
    #[serde(default)]
    pub message: Option<ClaudeUserMessage>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default, rename = "parent_tool_use_id")]
    pub parent_tool_use_id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ClaudeUserMessage {
    /// Array of content blocks. Each entry is a `tool_result` (with
    /// `tool_use_id`, `content`, `is_error`) or a plain text block. We
    /// keep this as `Vec<Value>` so the semantic parser can walk it
    /// without requiring a rigid type for every possible inner shape.
    #[serde(default)]
    pub content: Option<Vec<serde_json::Value>>,
}
```

Extend `ClaudeInit` (the `System` payload) to tolerate the `hook_*` subtypes — they already fall into `ClaudeInit` because the enum variant ignores unknown fields when `#[serde(default)]` is set on all fields. Verify by re-running the `claude_system_hook_subtypes_deserialize` test — it should now pass without any `ClaudeInit` changes. If not, add `#[serde(default)]` to the relevant fields.

Add `error` to `ClaudeAssistant`:

```rust
#[derive(Debug, Default, Deserialize)]
pub struct ClaudeAssistant {
    #[serde(default)]
    pub message: Option<ClaudeAssistantMessage>,
    #[serde(default)]
    pub session_id: Option<String>,
    /// Present on terminal-error paths such as billing_error. When set,
    /// the assistant message is typically a synthetic placeholder and
    /// should be classified as a terminal error, not as output text.
    #[serde(default)]
    pub error: Option<String>,
}
```

Extend `ClaudeResult` to accept the additional fields (most were missing previously):

```rust
#[derive(Debug, Default, Deserialize)]
pub struct ClaudeResult {
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub is_error: Option<bool>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub duration_api_ms: Option<u64>,
    #[serde(default)]
    pub num_turns: Option<u32>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub total_cost_usd: Option<f64>,
    #[serde(default)]
    pub usage: Option<Value>,
    #[serde(default)]
    #[serde(rename = "modelUsage")]
    pub model_usage: Option<Value>,
    #[serde(default)]
    pub permission_denials: Option<Vec<Value>>,
    #[serde(default)]
    pub terminal_reason: Option<String>,
    #[serde(default)]
    pub fast_mode_state: Option<String>,
    #[serde(default)]
    pub stop_reason: Option<String>,
}
```

- [ ] **Step 4: Run the protocol tests**

Run: `cargo test -p claudine --lib stream::protocol::claude::tests`
All four new tests must pass, plus all pre-existing ones.

- [ ] **Step 5: Write the failing semantic test — user → tool_result routing**

Append to `claudine/lib/src/stream/claude_semantic.rs` tests module:

```rust
    #[test]
    fn claude_user_event_routes_tool_result_to_semantic_tool_result() {
        use std::sync::{Arc, Mutex};
        use crate::stream::semantic::{SemanticEvent, SemanticEventSink};
        use crate::stream::parser::SemanticStreamParser;

        #[derive(Default)]
        struct Capture(Arc<Mutex<Vec<SemanticEvent>>>);
        impl SemanticEventSink for Capture {
            fn on_semantic_event(&mut self, event: SemanticEvent) {
                self.0.lock().unwrap().push(event);
            }
        }

        let events = Arc::new(Mutex::new(Vec::new()));
        let mut parser = ClaudeSemanticStreamParser::new(Capture(events.clone()), None);

        parser.feed_line(r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tu_1","content":"hello","is_error":false}]},"session_id":"s1"}"#).unwrap();

        let captured = events.lock().unwrap().clone();
        assert!(
            captured.iter().any(|e| e.kind_str() == "tool_result"),
            "expected a ToolResult semantic event; got {:?}",
            captured.iter().map(|e| e.kind_str()).collect::<Vec<_>>()
        );
        assert!(
            !captured.iter().any(|e| e.kind_str() == "provider_extension"),
            "user event must not leak as ProviderExtension"
        );
    }

    #[test]
    fn claude_billing_error_on_assistant_surfaces_terminal_error_not_rate_limit() {
        use std::sync::{Arc, Mutex};
        use crate::stream::semantic::{SemanticEvent, SemanticEventSink};
        use crate::stream::parser::SemanticStreamParser;

        #[derive(Default)]
        struct Capture(Arc<Mutex<Vec<SemanticEvent>>>);
        impl SemanticEventSink for Capture {
            fn on_semantic_event(&mut self, event: SemanticEvent) {
                self.0.lock().unwrap().push(event);
            }
        }

        let events = Arc::new(Mutex::new(Vec::new()));
        let mut parser = ClaudeSemanticStreamParser::new(Capture(events.clone()), None);

        parser.feed_line(r#"{"type":"assistant","message":{"model":"<synthetic>","content":[{"type":"text","text":"Credit balance is too low"}]},"session_id":"s1","error":"billing_error"}"#).unwrap();

        let captured = events.lock().unwrap().clone();
        let terminal_errors: Vec<_> = captured
            .iter()
            .filter(|e| matches!(e, SemanticEvent::Error { terminal: true, .. }))
            .collect();
        assert_eq!(
            terminal_errors.len(), 1,
            "expected exactly one terminal Error from billing_error; got {terminal_errors:?}"
        );
        if let SemanticEvent::Error { message, extra, .. } = terminal_errors[0] {
            assert!(message.to_lowercase().contains("billing") || message.to_lowercase().contains("credit"),
                "billing error message must mention billing/credit: {message:?}");
            assert_eq!(
                extra.get("error_kind").and_then(|v| v.as_str()),
                Some("billing_error"),
                "extra.error_kind must preserve the raw classification"
            );
        }
    }
```

- [ ] **Step 6: Run to verify they fail**

Run: `cargo test -p claudine --lib stream::claude_semantic::tests::claude_user_event_routes_tool_result_to_semantic_tool_result stream::claude_semantic::tests::claude_billing_error_on_assistant_surfaces_terminal_error_not_rate_limit`
Expected: FAIL.

- [ ] **Step 7: Implement the semantic routing**

In `claudine/lib/src/stream/claude_semantic.rs`:

Add `handle_user` that walks the `content` vector:

```rust
fn handle_user(&mut self, user: ClaudeUser, raw_kind: &str, raw: &Value) {
    let Some(message) = user.message else { return; };
    let Some(content) = message.content else { return; };
    for block in content {
        let Some(block_type) = block.get("type").and_then(Value::as_str) else { continue; };
        match block_type {
            "tool_result" => {
                let tool_use_id = block.get("tool_use_id").and_then(Value::as_str).map(String::from);
                let is_error = block.get("is_error").and_then(Value::as_bool).unwrap_or(false);
                let output = block.get("content").cloned();
                let status = if is_error { Some("error".to_string()) } else { Some("success".to_string()) };

                let mut extra = self.base_extra(raw_kind);
                if let Some(id) = &tool_use_id {
                    extra.insert("tool_id".into(), Value::from(id.clone()));
                }
                if is_error {
                    extra.insert("is_error".into(), Value::Bool(true));
                }
                // Pull tool_name from the earlier tool_use_id → name cache if
                // we have it; otherwise let the sink render without a name.
                let name = tool_use_id
                    .as_deref()
                    .and_then(|id| self.tool_uses.get(id).cloned());

                self.sink.on_semantic_event(SemanticEvent::ToolResult {
                    name,
                    id: tool_use_id,
                    status,
                    exit_code: None,
                    output,
                    extra: Value::Object(extra),
                });
            }
            "text" => {
                // User-role text blocks are the user's own prompt being
                // replayed into context. Not useful on stderr; drop silently.
            }
            _ => {
                // Preserve other unexpected block types via ProviderExtension
                // so nothing is silently lost.
                self.emit_provider_extension(&format!("user.{block_type}"), raw.clone());
            }
        }
    }
}
```

Wire it into `feed_line`:

```rust
            Ok(ClaudeEvent::User(user)) => {
                self.handle_user(user, &raw_kind, &raw);
            }
```

Extend `handle_assistant` to detect the `error` field and emit a terminal `Error` event:

```rust
fn handle_assistant(&mut self, a: ClaudeAssistant, raw_kind: &str) {
    if let Some(err_kind) = &a.error {
        self.is_error = true;
        self.error_kind = Some(err_kind.clone());
        let mut extra = self.base_extra(raw_kind);
        extra.insert("error_kind".into(), Value::from(err_kind.as_str()));
        // Pull the human-readable message from the assistant content if present.
        let message = a
            .message
            .as_ref()
            .and_then(|m| m.content.as_ref())
            .and_then(|content| {
                content.iter().find_map(|c| {
                    if c.get("type").and_then(Value::as_str) == Some("text") {
                        c.get("text").and_then(Value::as_str).map(String::from)
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_else(|| format!("Claude reported {err_kind}"));
        self.error_message = Some(message.clone());
        self.sink.on_semantic_event(SemanticEvent::Error {
            message,
            terminal: true,
            extra: Value::Object(extra),
        });
        return;
    }
    // existing assistant body: extract text content, emit OutputText, cache tool_use blocks for correlation
    // ... (keep the rest of the existing handle_assistant unchanged)
}
```

- [ ] **Step 8: Run the semantic tests**

Run: `cargo test -p claudine --lib stream::claude_semantic::tests`
All tests must pass including pre-existing ones.

- [ ] **Step 9: Add full fixture replay test**

Append:

```rust
    #[test]
    fn claude_fixture_full_replay_produces_no_provider_extensions() {
        use std::sync::{Arc, Mutex};
        use crate::stream::semantic::{SemanticEvent, SemanticEventSink};
        use crate::stream::parser::SemanticStreamParser;

        #[derive(Default)]
        struct Capture(Arc<Mutex<Vec<SemanticEvent>>>);
        impl SemanticEventSink for Capture {
            fn on_semantic_event(&mut self, event: SemanticEvent) {
                self.0.lock().unwrap().push(event);
            }
        }

        let fixture = std::fs::read_to_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/providers/claude.ndjson"),
        )
        .expect("claude.ndjson must exist");

        let events = Arc::new(Mutex::new(Vec::new()));
        let mut parser = ClaudeSemanticStreamParser::new(Capture(events.clone()), None);

        for (i, line) in fixture.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() { continue; }
            parser
                .feed_line(line)
                .unwrap_or_else(|e| panic!("line {}: {:?}", i + 1, e));
        }

        let captured = events.lock().unwrap().clone();
        let ext: Vec<&SemanticEvent> = captured
            .iter()
            .filter(|e| e.kind_str() == "provider_extension")
            .collect();
        assert!(
            ext.is_empty(),
            "captured Claude fixture must produce zero ProviderExtension events; found: {:?}",
            ext.iter().take(3).map(|e| e.kind_str()).collect::<Vec<_>>()
        );
    }
```

Run: `cargo test -p claudine --lib stream::claude_semantic::tests::claude_fixture_full_replay_produces_no_provider_extensions`
Expected: PASS. If any `ProviderExtension` remains, inspect the raw kind to determine which event type slipped through and patch iteratively — likely candidates are `stream_event`, `user_message_replay`, or `compact_boundary` which are in the Claude stream-json taxonomy per `CLAUDE.md` research but may not be present in this short fixture.

- [ ] **Step 10: Fix the badge misclassification**

In `claudine/lib/src/stream/badges.rs`, find the badge computation for `rate_limit` (grep for `rate_limit` / `RateLimit`). The bug is that a billing-error may be producing a rate-limit-style badge through a shared code path. Audit:

1. Ensure `billing_error` is classified via a distinct `BadgeKind::Billing` (or equivalent) variant, not merged with `BadgeKind::RateLimit`.
2. For genuine `rate_limit_event` Claude events, format the badge message to include both `status` and duration-until-`resetsAt` (e.g. `"rate limit · approaching · reset in 8m 42s"`).

Add a test that feeds an `assistant.error = billing_error` and asserts no rate-limit badge is produced:

```rust
    #[test]
    fn billing_error_does_not_produce_rate_limit_badge() {
        // Replay the assistant event that carries error: "billing_error"
        // and assert the badge computation produces a "billing" classification
        // — NOT a rate-limit badge — and that the message mentions billing.
        //
        // (Body depends on the current badges.rs API; follow the pattern of
        // the existing tests in that file.)
    }
```

Fill in the body by reading the existing rate-limit test at `claudine/lib/src/stream/badges.rs:389-448` and mirroring its shape. The fix may be as small as adding a guard `if error_kind == "billing_error" { return Badge::billing(...) }` upstream of the rate-limit classifier.

- [ ] **Step 11: Commit**

Bundle into two commits for easier review:

```bash
git add claudine/lib/src/stream/protocol/claude.rs
git commit -m "feat(claudine): extend Claude protocol with User/hook/error variants

Adds ClaudeEvent::User with a content-array message shape for routing
tool_result replays. Adds assistant.error field for billing_error and
similar terminal classifications. Extends ClaudeResult with
permission_denials, terminal_reason, fast_mode_state, and modelUsage —
all present in current Claude Code stream-json output but absent from
the previous protocol model."

git add claudine/lib/src/stream/claude_semantic.rs claudine/lib/src/stream/badges.rs
git commit -m "fix(claudine): route Claude user/error events semantically

User events now walk message.content and emit ToolResult per
tool_result block (correlating by tool_use_id against the existing
tool_uses cache); text blocks drop silently. assistant.error produces
a terminal SemanticEvent::Error instead of leaking as OutputText.
Badge computation distinguishes billing_error from rate_limit so
insufficient-credits sessions no longer surface as 'rate limit' badges
with no context; genuine rate_limit_event now includes status and
duration-until-reset in the message."
```

---

## Task 5: Regression sweep + re-capture

- [ ] **Step 1: Full workspace test run**

Run: `cargo test -p claudine -p claudine-cli`
Expected: all tests pass.

- [ ] **Step 2: Clippy delta check**

Run: `cargo clippy -p claudine -p claudine-cli --all-targets 2>&1 | grep -cE "^error:"`
Compare against the Plan 1 post-commit baseline. Zero net increase required.

- [ ] **Step 3: Re-capture Codex and Claude post-Plan-2 (only after Plan 1 has shipped)**

```bash
mkdir -p claudine/claudine-output/post-plan-2/
claudine codex   -p "find other versions of claudine/features/_unscheduled/improved-sequences/spec.md" 2>claudine/claudine-output/post-plan-2/codex.err >claudine/claudine-output/post-plan-2/codex.out
claudine claude  -p "find other versions of claudine/features/_unscheduled/improved-sequences/spec.md" -y 2>claudine/claudine-output/post-plan-2/claude.err >claudine/claudine-output/post-plan-2/claude.out
```

Grep for the pre-plan signatures — both must return nothing:

```bash
grep -F "codex/item.started · {" claudine/claudine-output/post-plan-2/codex.err    # must be empty
grep -F "codex/item.completed · {" claudine/claudine-output/post-plan-2/codex.err  # must be empty
grep -F "claude/user · {" claudine/claudine-output/post-plan-2/claude.err          # must be empty
```

If any return matches, Plan 2 did NOT close the symptom — surface to the user and do not mark complete.

- [ ] **Step 4: Commit the post-Plan-2 captures**

```bash
git add claudine/claudine-output/post-plan-2/
git commit -m "chore(claudine): post-plan-2 captures for regression reference"
```

---

## Out of Scope

- Gemini, OpenCode, Kimi, Qwen protocol work — those are in Plan 3 alongside sink hardening.
- Suppressing user-prompt echo on Gemini (`message.non_assistant`) — in Plan 3.
- Sink-level `summarize_provider_payload` hardening — in Plan 3.
- OpenCode's stderr TUI-format leak (`✱ Glob …`) — in Plan 3.
- Claude `stream_event` / `user_message_replay` / `compact_boundary` / `status` / `task_*` / `tool_progress` / `auth_status` / `files_persisted` / `tool_use_summary` / `prompt_suggestion` variants — if the full-fixture replay (Task 4 Step 9) surfaces any of these as `ProviderExtension`, patch them IN SCOPE as part of Task 4; otherwise leave them for a future expansion. The research doc `CLAUDE.md` lists all 21 SDKMessage types; this plan only commits to the subset present in the 2026-04-14 fixture.
