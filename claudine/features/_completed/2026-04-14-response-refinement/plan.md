# Response Refinement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix obvious-error and inconsistent-formatting problems in non-interactive provider wrap output across Claude, Codex, Gemini, and OpenCode by introducing a shared `ToolCallDisplay` contract, a 9-section rendered output model with structural spacing, per-provider parser fixes (Claude rate-limit heuristic + hook ordering, Codex tool-event field extraction, Gemini markdown streaming, OpenCode assistant-text restoration + YOLO + tool-call accounting), and removing raw-JSON / hard-coded truncation noise from stderr.

**Architecture:** A single formatter on `LiveSemanticSink` owns all `🔧 →` / `🔧 ←` rendering through a new `ToolCallDisplay` value built from each parser's `SemanticEvent::ToolCall` / `SemanticEvent::ToolResult`. Per-provider parsers (`claudine/lib/src/stream/{claude,codex,gemini,opencode}_semantic.rs`) populate semantic events with raw fields; the sink humanizes names, extracts per-tool summaries, and renders status (success/error). A new `Section` enum threads through the sink so structural blank-line dedup happens in `LiveSemanticSink`/`StreamOutput` rather than at parser level. Width and word-wrapping come from `biscuit_terminal::components::status::Status` via `Layout` (no truncation).

**Tech Stack:** Rust 2024 edition, `serde` / `serde_json` for protocol models, `biscuit-terminal` for `Status` / `Prose` / `BlockQuote` / `Layout` rendering, `tokio` for async dispatch, `nextest` (or `cargo test -p`) for tests. Fixtures live in `claudine/features/2026-04-14-response-refinement/*.jsonl` and `claudine/lib/tests/fixtures/providers/`.

---

## Phase Index

| Phase | Title | Depends on | Parallelizable |
|------|-------|-----------|----------------|
| 0 | Investigations (reproduce against HEAD before fixes) | none | yes (0a–0e) |
| 1 | Tool-Call Display Contract (Child 1) | Phase 0d (truncation) | no |
| 2a | Claude: hook ordering + rate-limit heuristic (Child 2) | Phase 1 | yes (with 2b, 2c) |
| 2b | Gemini markdown rendering (Child 4) | Phase 0c, Phase 1 | yes |
| 2c | OpenCode parser fixes + YOLO (Child 5) | Phase 0a, 0b, Phase 1 | yes |
| 2d | Codex tool-event field extraction (per-provider Codex) | Phase 0e, Phase 1 | yes |
| 3 | Section model + spacing normalization (Child 3) | Phases 1, 2a–2d | no |

---

## File Structure

**New files:**
- `claudine/lib/src/stream/tool_display.rs` — `ToolCallDisplay`, `ToolDirection`, `ToolStatus`, humanization (Tier 1 lookup + Tier 2 algorithmic), per-tool summary extractors. Owned by the lib so parsers and the CLI sink both depend on it.
- `claudine/cli/src/commands/wrap/section.rs` — `Section` enum (the 9-section model), `SectionStream` writer that wraps `StreamOutput` and enforces "at most one blank between adjacent sections".
- `claudine/lib/src/stream/thinking.rs` — small helper that renders `SemanticEvent::Reasoning` as a biscuit-terminal `BlockQuote` with grey vertical line + dim-italic prose. Used by the sink section-6 path.
- `claudine/features/2026-04-14-response-refinement/investigations.md` — the artifact each Phase 0 task writes its findings into.
- `claudine/lib/tests/fixtures/providers/opencode-assistant-text.ndjson` — fixture extracted from `opencode-yolo.jsonl` for the missing-text regression.
- `claudine/lib/tests/fixtures/providers/gemini-markdown-list.ndjson` — fixture for mid-list-item streaming.

**Modified files:**
- `claudine/cli/src/commands/wrap/live_semantic_sink.rs` — replace `tool_call_description` / `tool_result_description` with the `ToolCallDisplay` formatter; add section routing; add Claude rate-limit suppression heuristic; remove the hardcoded `truncate(_, 60)` / `truncate(_, 80)` caps in favor of `Layout` wrapping.
- `claudine/lib/src/stream/claude_semantic.rs` — reorder `SessionStart` emission to trail hook events (or document streaming-preservation fallback); leave `handle_rate_limit` emitting `Warning` (sink decides whether to render).
- `claudine/lib/src/stream/codex_semantic.rs` — populate `name` / `input` / `output` / `status` / `exit_code` on `ToolCall` / `ToolResult` for `command_execution` and other Codex tool item types.
- `claudine/lib/src/stream/gemini_semantic.rs` — buffer streamed text deltas until a logical break (paragraph or list-item boundary) before emitting `OutputText`, OR add an `OutputText { is_final: bool }`-style hint that Darkmatter can use to defer markdown rendering. Final approach decided by Phase 0c.
- `claudine/lib/src/stream/opencode_semantic.rs` — change `handle_tool_use_completed` to emit only `ToolResult` (drop the synthesized `ToolCall`); fix `handle_text` to extract from `part.text` shape.
- `claudine/cli/src/commands/wrap/profile.rs` — `OpencodeWrapper::apply_yolo` returns warning only when interactive; non-interactive forwards `--dangerously-skip-permissions`. Update the warning copy.
- `claudine/cli/src/commands/wrap/exec.rs` — confirm `OutputTextCallback` is wired for OpenCode and flush stdout on completion.
- `claudine/lib/src/stream/badges.rs` — no functional change; review only.

**Test files:**
- `claudine/lib/src/stream/tool_display.rs` — unit tests for humanization tiers, summary extraction, status priority, error-styling marker.
- `claudine/cli/src/commands/wrap/section.rs` — unit tests for spacing dedup and section-order assertions.
- `claudine/lib/src/stream/claude_semantic.rs` — add tests for hook-vs-session ordering and that rate-limit `Warning` is still emitted.
- `claudine/lib/src/stream/codex_semantic.rs` — add tests for tool-name + input + output extraction on `command_execution`.
- `claudine/lib/src/stream/gemini_semantic.rs` — add streaming-list-item buffering test against the new fixture.
- `claudine/lib/src/stream/opencode_semantic.rs` — add tests for text extraction from `part.text` shape and single-event tool-use accounting.
- `claudine/cli/src/commands/wrap/live_semantic_sink.rs` — add tests for `ToolCallDisplay` rendering, Claude rate-limit suppression by env var, and full-fixture spacing assertions per provider.
- `claudine/cli/src/commands/wrap/profile.rs` — add tests for OpenCode YOLO non-interactive vs. interactive behavior.

---

## Phase 0 — Investigations (Reproduce Symptoms Against HEAD)

These are spec'd investigation tasks. Each writes findings to `investigations.md` so the implementation phases can act on real diagnostics rather than the spec's hypotheses.

### Task 0.0: Create the investigations log

**Files:**
- Create: `claudine/features/2026-04-14-response-refinement/investigations.md`

- [ ] **Step 1: Create the investigations file with section headers**

```markdown
# Response Refinement Investigations

Findings recorded against HEAD before the corresponding fix lands.

## 0a — OpenCode Assistant Text Missing From Stdout (P0)

_Reproduce per Phase 0a; record root cause and chosen fix._

## 0b — OpenCode Mis-Routed Render (`⚙ firecrawl_firecrawl_search {…JSON…}`)

_Reproduce per Phase 0b; record the actual render path._

## 0c — Gemini Markdown List Truncation

_Reproduce per Phase 0c; record whether the fix lives in the parser or in Darkmatter._

## 0d — Hard-Coded Truncation Cap Location

_Locate the cap removed during Child 1; record file:line and any tests pinning it._

## 0e — Codex Tool-Event Field Extraction

_Audit `codex_semantic.rs`; list which tool fields are dropped today._
```

- [ ] **Step 2: Commit**

```bash
git add claudine/features/2026-04-14-response-refinement/investigations.md
git commit -m "docs(claudine): seed response-refinement investigations log"
```

### Task 0a: Reproduce OpenCode assistant-text drop (P0)

**Files:**
- Read: `claudine/features/2026-04-14-response-refinement/opencode-yolo.jsonl`, `claudine/features/2026-04-14-response-refinement/opencode-not-yolo.jsonl`
- Read: `claudine/lib/src/stream/opencode_semantic.rs:handle_text` (around line 195–208)
- Read: `claudine/cli/src/commands/wrap/mod.rs:1269-1310` (where `with_output_text_sink` is wired)
- Write: append findings to `investigations.md` § 0a

- [ ] **Step 1: Inspect the captured fixtures**

Run:
```bash
grep -n '"text"\|"part"\|"type"' claudine/features/2026-04-14-response-refinement/opencode-yolo.jsonl | head -50
```
Expected: at least one event line whose `part` object has a `text` field — confirming the `part.text` shape claim in the spec.

- [ ] **Step 2: Trace a fresh OpenCode run end-to-end**

Run (substitute a real prompt):
```bash
RUST_LOG=claudine=trace cargo run -p claudine-cli -- opencode --model "$OPENCODE_MODEL" --yolo -- "what is 2+2?" 2> /tmp/opencode-trace.log
```
Expected: `/tmp/opencode-trace.log` shows the raw `text`/`part.text` event traversing the parser. If `OutputText` is never emitted, the parser is dropping it; if it is emitted but stdout is empty, the sink's `emit_output_text` is unwired or unflushed.

- [ ] **Step 3: Confirm OpenCode native renders the text**

Run:
```bash
opencode run --model "$OPENCODE_MODEL" -- "what is 2+2?"
```
Expected: assistant text printed to stdout. If yes, regression is in claudine.

- [ ] **Step 4: Record findings in `investigations.md` § 0a**

Write the file:line of the dropped extraction OR the unwired stdout writer, plus whether `flush()` is missing on completion. Save the chosen fix as one of:
- "Parser fix — extend `handle_text` to read `part.text` (and any other observed shapes)"
- "Sink wiring fix — `with_output_text_sink` not invoked for OpenCode profile"
- "Stdout flush fix — `StreamOutput` does not flush on child exit"

- [ ] **Step 5: Commit**

```bash
git add claudine/features/2026-04-14-response-refinement/investigations.md
git commit -m "docs(claudine): record opencode assistant-text drop investigation"
```

### Task 0b: Reproduce OpenCode `⚙ firecrawl…` mis-routed render

**Files:**
- Read: `claudine/lib/src/stream/opencode_semantic.rs`
- Read: `claudine/cli/src/commands/wrap/live_semantic_sink.rs:render_event`
- Write: append findings to `investigations.md` § 0b

- [ ] **Step 1: Locate every render path that uses the `⚙` glyph**

Run:
```bash
rg -n '\\u\{2699\}|⚙|StatusState::Info' claudine/cli/src claudine/lib/src biscuit-terminal
```
Expected: identify the exact `Status` state and renderer that emits `⚙`.

- [ ] **Step 2: Trace a Firecrawl OpenCode run**

Run:
```bash
RUST_LOG=claudine=trace cargo run -p claudine-cli -- opencode --model "$OPENCODE_MODEL" --use firecrawl -- "search the web for: NFL draft 2026" 2> /tmp/opencode-mcp-trace.log
```
Expected: a line in stderr matching `⚙ firecrawl_firecrawl_search {…raw JSON…}`. Cross-reference the line with the trace log.

- [ ] **Step 3: Record findings in `investigations.md` § 0b**

Identify whether the line is rendered by:
- `SemanticEvent::ProviderExtension` falling through `summarize_provider_payload` returning `None`,
- `SemanticEvent::Info` with raw input attached,
- a code path in another module (e.g. `composition.rs`).

Document the actual emitter and JSON-printing call so Phase 2c can fix it at the source.

- [ ] **Step 4: Commit**

```bash
git add claudine/features/2026-04-14-response-refinement/investigations.md
git commit -m "docs(claudine): record opencode firecrawl mis-route investigation"
```

### Task 0c: Reproduce Gemini markdown list truncation

**Files:**
- Read: `claudine/lib/src/stream/gemini_semantic.rs:handle_text` (or text-emission path)
- Read: `darkmatter/lib/src` (look for streaming markdown renderer)
- Write: append findings to `investigations.md` § 0c
- Create: `claudine/lib/tests/fixtures/providers/gemini-markdown-list.ndjson`

- [ ] **Step 1: Capture a Gemini run that emits a markdown list**

Run:
```bash
cargo run -p claudine-cli -- gemini -- "list the four NFL conferences in markdown bullet form" > /tmp/gemini-stdout.txt 2> /tmp/gemini-stderr.txt
```
Expected: stdout shows mid-item truncation OR stray blank lines between items, matching the spec's symptom screenshot.

- [ ] **Step 2: Save the raw NDJSON for use in tests**

Capture the matching raw stream (set `CLAUDINE_LOG_RAW=1` or pipe `--debug`) and copy the relevant 4–8 lines to `claudine/lib/tests/fixtures/providers/gemini-markdown-list.ndjson`. Each line must be a full Gemini event JSON.

- [ ] **Step 3: Determine where line-item splitting happens**

Run:
```bash
rg -n 'OutputText|emit_output_text|markdown|fence|StreamTextRenderer' darkmatter/lib/src claudine/cli/src
```
Expected: identify the streaming markdown renderer and whether it treats each `OutputText` as a fully-bounded markdown document.

- [ ] **Step 4: Record findings + chosen fix in `investigations.md` § 0c**

Pick one:
- "Parser fix — buffer-until-logical-break in `gemini_semantic.rs::handle_text`" (cleaner contract; recommended if Darkmatter cannot defer)
- "Renderer fix — Darkmatter streaming continuation for unterminated list items"

Justify the choice from observed behavior.

- [ ] **Step 5: Commit**

```bash
git add claudine/features/2026-04-14-response-refinement/investigations.md \
        claudine/lib/tests/fixtures/providers/gemini-markdown-list.ndjson
git commit -m "docs(claudine): record gemini markdown truncation investigation"
```

### Task 0d: Locate the hard-coded truncation cap

**Files:**
- Read: `claudine/cli/src/commands/wrap/live_semantic_sink.rs:704-711` (the `truncate` helper) and all callers
- Write: append findings to `investigations.md` § 0d

- [ ] **Step 1: Enumerate all `truncate(` callers in the CLI**

Run:
```bash
rg -n 'fn truncate|truncate\(' claudine/cli/src claudine/lib/src
```
Expected: confirm the only callers in the sink are `summarize_input` (60 chars) and `summarize_provider_payload` (80 chars).

- [ ] **Step 2: Verify there is no second cap inside biscuit-terminal `Status`**

Run:
```bash
rg -n 'truncate|\.chars\(\).take\(' biscuit-terminal/lib/src/components/status.rs biscuit-terminal/lib/src/components/prose.rs
```
Expected: `Status` defers to `Layout` for wrapping; no hard cap. If a cap is found, it must be removed in Phase 1.

- [ ] **Step 3: Record findings in `investigations.md` § 0d**

List every file:line that imposes a character cap on the sink rendering pipeline. Phase 1 will remove these in favor of `Layout` wrapping.

- [ ] **Step 4: Commit**

```bash
git add claudine/features/2026-04-14-response-refinement/investigations.md
git commit -m "docs(claudine): record sink truncation-cap inventory"
```

### Task 0e: Audit Codex tool-event fields

**Files:**
- Read: `claudine/lib/src/stream/codex_semantic.rs` (handlers for `item.started` / `item.completed` of `command_execution` and other tool item types)
- Read: `claudine/lib/src/stream/protocol/codex.rs` (the typed envelope)
- Write: append findings to `investigations.md` § 0e

- [ ] **Step 1: List every Codex tool item type**

Run:
```bash
rg -n '"command_exec|"command_execution|"web_search|"file_change|tool_name|item_type' claudine/lib/src/stream/protocol/codex.rs claudine/lib/src/stream/codex_semantic.rs
```
Expected: identify which item types emit `ToolCall` / `ToolResult` and which fields of the typed envelope are populated.

- [ ] **Step 2: Capture a real Codex tool-using session**

Run:
```bash
cargo run -p claudine-cli -- codex -- "list files in this directory and explain what each does" 2> /tmp/codex-trace.log
```
Expected: identify which `ToolCall`/`ToolResult` events have `name`, `input`, `output`, `status`, `exit_code` populated vs. dropped.

- [ ] **Step 3: Record gaps in `investigations.md` § 0e**

For each tool item type, list the fields available in the raw stream that are NOT being copied into `SemanticEvent::ToolCall` / `SemanticEvent::ToolResult`. Phase 2d will close each gap.

- [ ] **Step 4: Commit**

```bash
git add claudine/features/2026-04-14-response-refinement/investigations.md
git commit -m "docs(claudine): record codex tool-event field gaps"
```

---

## Phase 1 — Tool-Call Display Contract (Child 1)

Single-formatter contract that owns all `🔧 →` / `🔧 ←` rendering. Phase 0d must be complete before Step 5.

### Task 1.1: Define `ToolCallDisplay` and direction/status enums

**Files:**
- Create: `claudine/lib/src/stream/tool_display.rs`
- Modify: `claudine/lib/src/stream/mod.rs` (add `pub mod tool_display;`)
- Test: same file

- [ ] **Step 1: Write the failing test for the type and its serde shape**

Add to `claudine/lib/src/stream/tool_display.rs`:

```rust
//! `ToolCallDisplay` — protocol-level model for rendering a tool invocation
//! (request or response) in a single, provider-agnostic way.

use serde::{Deserialize, Serialize};

/// Direction of a tool event from the assistant's perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolDirection {
    Outgoing,
    Incoming,
}

/// Outcome of an incoming tool event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Success,
    Error,
    Pending,
}

/// Display-ready tool event. Per spec: status wins over summary on incoming
/// events; the formatter NEVER writes a glyph literally — it populates a
/// biscuit-terminal `Status::ToolUse` instead.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallDisplay {
    pub direction: ToolDirection,
    pub display_name: String,
    pub summary: Option<String>,
    pub status: Option<ToolStatus>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&ToolDirection::Outgoing).unwrap(),
            "\"outgoing\""
        );
        assert_eq!(
            serde_json::to_string(&ToolStatus::Success).unwrap(),
            "\"success\""
        );
    }

    #[test]
    fn struct_round_trips_via_clone_and_eq() {
        let display = ToolCallDisplay {
            direction: ToolDirection::Incoming,
            display_name: "Firecrawl Search".into(),
            summary: Some("NFL draft 2026 date".into()),
            status: Some(ToolStatus::Success),
        };
        let cloned = display.clone();
        assert_eq!(display, cloned);
    }
}
```

Add to `claudine/lib/src/stream/mod.rs`:

```rust
pub mod tool_display;
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p claudine stream::tool_display::tests`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add claudine/lib/src/stream/tool_display.rs claudine/lib/src/stream/mod.rs
git commit -m "feat(claudine): add ToolCallDisplay protocol type"
```

### Task 1.2: Implement Tier-1 / Tier-2 humanization

**Files:**
- Modify: `claudine/lib/src/stream/tool_display.rs`
- Test: same file

- [ ] **Step 1: Write the failing tests**

Append to `claudine/lib/src/stream/tool_display.rs`:

```rust
/// Resolve a raw tool id like `firecrawl_firecrawl_search` into a
/// human-readable display name. Two-tier strategy:
///
/// 1. Lookup table for known tools / prefixes.
/// 2. Algorithmic fallback (strip provider-redundant prefix, split on `_`,
///    Title Case).
///
/// As a last resort returns the raw id unchanged.
pub fn humanize_tool_name(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    if let Some(name) = humanize_known(raw) {
        return name;
    }
    humanize_algorithmic(raw)
}

fn humanize_known(raw: &str) -> Option<String> {
    // MCP-shape: mcp__<server>__<tool>
    if let Some(rest) = raw.strip_prefix("mcp__") {
        let mut parts = rest.splitn(2, "__");
        let server = parts.next()?;
        let tool = parts.next()?;
        return Some(format!(
            "{} {}",
            title_case_segments(server),
            title_case_segments(tool)
        ));
    }
    if let Some(rest) = raw.strip_prefix("firecrawl_firecrawl_") {
        return Some(format!("Firecrawl {}", title_case_segments(rest)));
    }
    if let Some(rest) = raw.strip_prefix("firecrawl_") {
        return Some(format!("Firecrawl {}", title_case_segments(rest)));
    }
    match raw {
        "google_web_search" => Some("Google Web Search".into()),
        "Bash" | "Edit" | "Read" | "Write" | "Glob" | "Grep" | "WebFetch" | "WebSearch"
        | "Task" => Some(raw.into()),
        _ => None,
    }
}

fn humanize_algorithmic(raw: &str) -> String {
    title_case_segments(raw)
}

fn title_case_segments(s: &str) -> String {
    s.split('_')
        .filter(|seg| !seg.is_empty())
        .map(|seg| {
            let mut chars = seg.chars();
            match chars.next() {
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod humanize_tests {
    use super::*;

    #[test]
    fn firecrawl_double_prefix_collapses_to_single_firecrawl() {
        assert_eq!(humanize_tool_name("firecrawl_firecrawl_search"), "Firecrawl Search");
    }

    #[test]
    fn google_web_search_maps_to_canonical_label() {
        assert_eq!(humanize_tool_name("google_web_search"), "Google Web Search");
    }

    #[test]
    fn claude_builtins_pass_through() {
        for name in ["Bash", "Edit", "Read", "Write", "Glob", "Grep", "WebFetch", "WebSearch", "Task"] {
            assert_eq!(humanize_tool_name(name), name);
        }
    }

    #[test]
    fn mcp_prefix_renders_server_and_tool() {
        assert_eq!(
            humanize_tool_name("mcp__firecrawl__deep_research"),
            "Firecrawl Deep Research"
        );
    }

    #[test]
    fn unknown_snake_case_falls_through_to_title_case() {
        assert_eq!(humanize_tool_name("custom_local_tool"), "Custom Local Tool");
    }

    #[test]
    fn empty_input_returns_empty_string() {
        assert_eq!(humanize_tool_name(""), "");
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p claudine stream::tool_display::humanize_tests`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add claudine/lib/src/stream/tool_display.rs
git commit -m "feat(claudine): two-tier tool-name humanization"
```

### Task 1.3: Implement per-tool summary extraction

**Files:**
- Modify: `claudine/lib/src/stream/tool_display.rs`
- Test: same file

- [ ] **Step 1: Write the failing tests**

Append to `claudine/lib/src/stream/tool_display.rs`:

```rust
use serde_json::Value;

/// Extract the meaningful slice of a tool's input arguments for display in
/// the dim-italic slot. Best-effort — falls back to the first non-empty
/// string value, then to a compact JSON one-liner. Width handling is the
/// caller's responsibility.
pub fn extract_tool_summary(tool_name: &str, input: &Value) -> Option<String> {
    if let Some(s) = input.as_str() {
        return Some(s.to_string());
    }
    let obj = input.as_object()?;
    // Per-tool hooks first.
    let preferred_key = match tool_name {
        n if n.contains("search") || n == "WebSearch" || n == "WebFetch" || n == "google_web_search" => Some("query"),
        "Bash" => Some("command"),
        "Read" | "Write" | "Edit" => Some("file_path"),
        "Glob" | "Grep" => Some("pattern"),
        _ => None,
    };
    if let Some(key) = preferred_key {
        if let Some(Value::String(s)) = obj.get(key) {
            return Some(s.clone());
        }
    }
    // Generic well-known keys.
    for key in ["command", "path", "file_path", "dir_path", "pattern", "query", "url", "message"] {
        if let Some(Value::String(s)) = obj.get(key) {
            return Some(s.clone());
        }
    }
    // First non-empty string value.
    for (_, v) in obj.iter() {
        if let Some(s) = v.as_str().filter(|s| !s.is_empty()) {
            return Some(s.to_string());
        }
    }
    None
}

#[cfg(test)]
mod summary_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn web_search_extracts_query() {
        let input = json!({"query": "NFL draft 2026 date", "limit": 5});
        assert_eq!(
            extract_tool_summary("firecrawl_firecrawl_search", &input).as_deref(),
            Some("NFL draft 2026 date")
        );
    }

    #[test]
    fn bash_extracts_command() {
        let input = json!({"command": "ls -la"});
        assert_eq!(extract_tool_summary("Bash", &input).as_deref(), Some("ls -la"));
    }

    #[test]
    fn read_extracts_file_path() {
        let input = json!({"file_path": "/etc/hosts"});
        assert_eq!(extract_tool_summary("Read", &input).as_deref(), Some("/etc/hosts"));
    }

    #[test]
    fn unknown_tool_falls_back_to_first_string() {
        let input = json!({"weirdo": "interesting", "n": 5});
        assert_eq!(extract_tool_summary("custom_unknown", &input).as_deref(), Some("interesting"));
    }

    #[test]
    fn returns_none_for_object_with_no_strings() {
        let input = json!({"a": 1, "b": [1,2]});
        assert!(extract_tool_summary("custom_unknown", &input).is_none());
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p claudine stream::tool_display::summary_tests`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add claudine/lib/src/stream/tool_display.rs
git commit -m "feat(claudine): per-tool summary extraction with fallbacks"
```

### Task 1.4: Construct `ToolCallDisplay` from `SemanticEvent`

**Files:**
- Modify: `claudine/lib/src/stream/tool_display.rs`
- Test: same file

- [ ] **Step 1: Write the failing tests**

Append to `claudine/lib/src/stream/tool_display.rs`:

```rust
use crate::stream::semantic::SemanticEvent;

impl ToolCallDisplay {
    /// Build an outgoing display from a `SemanticEvent::ToolCall`. Returns
    /// `None` for non-matching variants.
    pub fn from_call(event: &SemanticEvent) -> Option<Self> {
        let SemanticEvent::ToolCall { name, input, .. } = event else {
            return None;
        };
        let raw_name = name.as_deref().unwrap_or("");
        let display_name = if raw_name.is_empty() {
            "(tool)".into()
        } else {
            humanize_tool_name(raw_name)
        };
        let summary = input
            .as_ref()
            .and_then(|v| extract_tool_summary(raw_name, v));
        Some(Self {
            direction: ToolDirection::Outgoing,
            display_name,
            summary,
            status: None,
        })
    }

    /// Build an incoming display from a `SemanticEvent::ToolResult`. Per
    /// spec: status always wins over summary in the dim slot when present;
    /// summary is consulted as a fallback only when status is absent.
    pub fn from_result(event: &SemanticEvent) -> Option<Self> {
        let SemanticEvent::ToolResult {
            name,
            status,
            output,
            extra,
            ..
        } = event
        else {
            return None;
        };
        let raw_name = name.as_deref().unwrap_or("");
        let display_name = if raw_name.is_empty() {
            "(tool)".into()
        } else {
            humanize_tool_name(raw_name)
        };
        let parsed_status = status.as_deref().and_then(|s| match s {
            "success" | "completed" | "ok" => Some(ToolStatus::Success),
            "error" | "failure" | "failed" => Some(ToolStatus::Error),
            "pending" | "running" | "in_progress" => Some(ToolStatus::Pending),
            _ => None,
        });
        let summary = if parsed_status.is_some() {
            None
        } else {
            // Status absent: fall back to a derived output summary.
            output
                .as_ref()
                .or_else(|| extra.get("input"))
                .and_then(|v| extract_tool_summary(raw_name, v))
        };
        Some(Self {
            direction: ToolDirection::Incoming,
            display_name,
            summary,
            status: parsed_status,
        })
    }
}

#[cfg(test)]
mod from_event_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn from_call_humanizes_and_extracts_query() {
        let event = SemanticEvent::ToolCall {
            name: Some("firecrawl_firecrawl_search".into()),
            id: None,
            input: Some(json!({"query": "NFL"})),
            extra: json!({}),
        };
        let display = ToolCallDisplay::from_call(&event).unwrap();
        assert_eq!(display.direction, ToolDirection::Outgoing);
        assert_eq!(display.display_name, "Firecrawl Search");
        assert_eq!(display.summary.as_deref(), Some("NFL"));
        assert!(display.status.is_none());
    }

    #[test]
    fn from_result_uses_status_and_drops_summary_when_status_present() {
        let event = SemanticEvent::ToolResult {
            name: Some("Bash".into()),
            id: None,
            status: Some("success".into()),
            exit_code: None,
            output: Some(json!({"stdout": "ok"})),
            extra: json!({}),
        };
        let display = ToolCallDisplay::from_result(&event).unwrap();
        assert_eq!(display.status, Some(ToolStatus::Success));
        assert!(display.summary.is_none(), "status wins over summary");
    }

    #[test]
    fn from_result_falls_back_to_summary_when_status_absent() {
        let event = SemanticEvent::ToolResult {
            name: Some("Bash".into()),
            id: None,
            status: None,
            exit_code: None,
            output: Some(json!({"command": "ls"})),
            extra: json!({}),
        };
        let display = ToolCallDisplay::from_result(&event).unwrap();
        assert!(display.status.is_none());
        assert_eq!(display.summary.as_deref(), Some("ls"));
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p claudine stream::tool_display::from_event_tests`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add claudine/lib/src/stream/tool_display.rs
git commit -m "feat(claudine): build ToolCallDisplay from SemanticEvent"
```

### Task 1.5: Wire `ToolCallDisplay` into `LiveSemanticSink` (single formatter)

**Files:**
- Modify: `claudine/cli/src/commands/wrap/live_semantic_sink.rs:271-297` (replace `tool_call_description` / `tool_result_description`)
- Modify: same file, render path for `ToolCall` / `ToolResult` events
- Test: same file

- [ ] **Step 1: Write the failing test for the canonical render**

Append to `claudine/cli/src/commands/wrap/live_semantic_sink.rs::tests`:

```rust
#[test]
fn tool_call_renders_canonical_format_with_humanized_name_and_query_summary() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("firecrawl_firecrawl_search".into()),
        id: None,
        input: Some(json!({"query": "NFL draft 2026 date"})),
        extra: json!({}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("Firecrawl Search"),
        "expected humanized name in {rendered:?}"
    );
    assert!(
        rendered.contains("NFL draft 2026 date"),
        "expected query summary in {rendered:?}"
    );
    assert!(rendered.contains('\u{2192}'), "expected → arrow");
}

#[test]
fn tool_result_renders_status_word_when_status_present() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("firecrawl_firecrawl_search".into()),
        id: None,
        status: Some("success".into()),
        exit_code: None,
        output: None,
        extra: json!({}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(rendered.contains("Firecrawl Search"));
    assert!(rendered.contains("success"));
    assert!(rendered.contains('\u{2190}'));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p claudine-cli live_semantic_sink::tests::tool_call_renders_canonical_format_with_humanized_name_and_query_summary`
Expected: FAIL (current formatter outputs `firecrawl_firecrawl_search` not `Firecrawl Search`).

- [ ] **Step 3: Replace the description helpers with the new formatter**

In `claudine/cli/src/commands/wrap/live_semantic_sink.rs`, replace `tool_call_description` (lines 271–278) and `tool_result_description` (lines 280–297) with:

```rust
fn render_tool_display(&self, display: ToolCallDisplay) -> String {
    let arrow = match display.direction {
        ToolDirection::Outgoing => '\u{2192}',
        ToolDirection::Incoming => '\u{2190}',
    };
    let slot = match (display.status, display.summary) {
        (Some(ToolStatus::Success), _) => Some(("success".into(), false)),
        (Some(ToolStatus::Error), _) => Some(("error".into(), true)),
        (Some(ToolStatus::Pending), _) => Some(("pending".into(), false)),
        (None, Some(summary)) => Some((summary, false)),
        (None, None) => None,
    };
    match slot {
        Some((text, is_error)) => {
            // Error styling: bold + red. The dim-italic wrapper is applied
            // by `Status` rendering. Per spec, the glyph and overall format
            // are unchanged; only the status word changes color.
            let styled_text = if is_error {
                format!("<red><b>{text}</b></red>")
            } else {
                text
            };
            format!("{arrow} {} \u{00b7} {styled_text}", display.display_name)
        }
        None => format!("{arrow} {}", display.display_name),
    }
}
```

Add the import at the top of the file:

```rust
use claudine::stream::tool_display::{ToolCallDisplay, ToolDirection, ToolStatus};
```

In `render_event`, replace the `ToolCall` and `ToolResult` arms:

```rust
SemanticEvent::ToolCall { .. } => {
    if let Some(display) = ToolCallDisplay::from_call(event) {
        let desc = self.render_tool_display(display);
        self.render_status(StatusState::ToolUse, desc);
    }
}
SemanticEvent::ToolResult { .. } => {
    if let Some(display) = ToolCallDisplay::from_result(event) {
        let desc = self.render_tool_display(display);
        self.render_status(StatusState::ToolUse, desc);
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p claudine-cli live_semantic_sink::tests`
Expected: PASS for the new tests AND every pre-existing tool-call test (some pre-existing tests assert legacy raw names like `bash` — those still pass because Tier-1 lookup preserves `Bash` exactly; tests using `bash` lowercase will need `humanize_tool_name` to title-case — confirm and update tests if necessary).

- [ ] **Step 5: Commit**

```bash
git add claudine/cli/src/commands/wrap/live_semantic_sink.rs
git commit -m "feat(claudine): route tool-call rendering through ToolCallDisplay"
```

### Task 1.6: Remove the hard-coded truncation cap; defer to `Layout` wrapping

**Files:**
- Modify: `claudine/cli/src/commands/wrap/live_semantic_sink.rs:704-711` (the `truncate` helper) and callers
- Test: same file

- [ ] **Step 1: Write the failing test**

Append to `tests`:

```rust
#[test]
fn long_summary_is_not_truncated_to_60_or_80_chars() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    let long = "a".repeat(200);
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("Bash".into()),
        id: None,
        input: Some(json!({"command": long.clone()})),
        extra: json!({}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains(&long),
        "long command must not be truncated; got {rendered:?}"
    );
    assert!(!rendered.contains('\u{2026}'), "no ellipsis expected");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p claudine-cli long_summary_is_not_truncated_to_60_or_80_chars`
Expected: FAIL (today truncate caps at 60 / 80).

- [ ] **Step 3: Remove the cap**

In `claudine/cli/src/commands/wrap/live_semantic_sink.rs`:
- Delete the `truncate` helper (lines 704–711).
- In `summarize_input` change `truncate(s, 60)` → `s.to_string()` everywhere it appears.
- In `summarize_provider_payload` change `truncate(s, 80)` → `s.to_string()` everywhere it appears.

`Status` already wraps via biscuit-terminal `Layout`, so the rendered line word-wraps at terminal width without any character cap.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p claudine-cli`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add claudine/cli/src/commands/wrap/live_semantic_sink.rs
git commit -m "fix(claudine): remove hardcoded summary cap; defer to Layout wrapping"
```

---

## Phase 2 — Per-Provider Children (Parallelizable)

Phases 2a, 2b, 2c, 2d may run in parallel. Each one consumes the `ToolCallDisplay` contract from Phase 1.

### Phase 2a — Claude (Child 2)

#### Task 2a.1: Add the rate-limit env-gated suppression heuristic

**Files:**
- Modify: `claudine/cli/src/commands/wrap/live_semantic_sink.rs` (rate-limit suppression before `render_event`)
- Test: same file

- [ ] **Step 1: Write the failing test**

Append to the sink's `tests` mod:

```rust
#[test]
fn claude_rate_limit_warning_suppressed_when_anthropic_api_key_unset() {
    use claudine::stream::semantic::SemanticEvent;
    let _guard = TestEnvGuard::remove("ANTHROPIC_API_KEY");
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::Warning {
        message: "rate limit".into(),
        extra: json!({"raw_kind": "rate_limit_event"}),
    });
    assert!(
        lines.lock().unwrap().is_empty(),
        "rate-limit Warning must not render to stderr without ANTHROPIC_API_KEY"
    );
    assert!(
        !dispatched.lock().unwrap().is_empty(),
        "underlying dispatch must still fire so JSONL log retains the event"
    );
}

#[test]
fn claude_rate_limit_warning_renders_when_anthropic_api_key_set() {
    let _guard = TestEnvGuard::set("ANTHROPIC_API_KEY", "sk-test");
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::Warning {
        message: "rate limit".into(),
        extra: json!({"raw_kind": "rate_limit_event"}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("rate limit"),
        "rate-limit Warning must render with API key set: {rendered:?}"
    );
}

/// RAII wrapper that restores the prior env var value on drop. Use
/// `serial_test::serial` on each test that depends on it (env vars are
/// process-wide).
struct TestEnvGuard {
    key: &'static str,
    prior: Option<String>,
}
impl TestEnvGuard {
    fn remove(key: &'static str) -> Self {
        let prior = std::env::var(key).ok();
        // SAFETY: tests run serial; no other thread is reading.
        unsafe { std::env::remove_var(key); }
        Self { key, prior }
    }
    fn set(key: &'static str, value: &str) -> Self {
        let prior = std::env::var(key).ok();
        unsafe { std::env::set_var(key, value); }
        Self { key, prior }
    }
}
impl Drop for TestEnvGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.prior {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }
}
```

Add `#[serial]` attributes (requires `serial_test`):

```rust
use serial_test::serial;

#[test]
#[serial]
fn claude_rate_limit_warning_suppressed_when_anthropic_api_key_unset() { /* ... */ }

#[test]
#[serial]
fn claude_rate_limit_warning_renders_when_anthropic_api_key_set() { /* ... */ }
```

Add `serial_test = "0.10"` to `claudine/cli/Cargo.toml` `[dev-dependencies]` if not already present.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p claudine-cli claude_rate_limit_warning -- --test-threads=1`
Expected: FAIL.

- [ ] **Step 3: Implement the suppression in `render_event`**

In `live_semantic_sink.rs`, in the `Warning` arm of `render_event`:

```rust
SemanticEvent::Warning { message, extra } => {
    if !message.starts_with("Malformed JSON on line ")
        && !is_suppressed_claude_rate_limit(self.provider, extra)
    {
        self.render_status(StatusState::Warning, message.clone());
    }
}
```

Add the helper near `is_silent_extension_kind`:

```rust
/// Suppress the Claude rate-limit Warning on stderr when the user is on a
/// subscription (no `ANTHROPIC_API_KEY` set). The dispatch and JSONL log
/// continue to fire — only the stderr render is gated.
fn is_suppressed_claude_rate_limit(provider: Provider, extra: &Value) -> bool {
    if provider != Provider::Claude {
        return false;
    }
    let raw_kind = extra
        .get("raw_kind")
        .and_then(Value::as_str)
        .unwrap_or("");
    if raw_kind != "rate_limit_event" {
        return false;
    }
    std::env::var("ANTHROPIC_API_KEY")
        .map(|v| v.trim().is_empty())
        .unwrap_or(true)
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p claudine-cli claude_rate_limit_warning -- --test-threads=1`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add claudine/cli/src/commands/wrap/live_semantic_sink.rs claudine/cli/Cargo.toml
git commit -m "feat(claudine): gate claude rate-limit warning on ANTHROPIC_API_KEY"
```

#### Task 2a.2: Reorder hook events to trail the session-ID marker

**Files:**
- Modify: `claudine/lib/src/stream/claude_semantic.rs` (handler for `system/hook_started` etc., and `init` handler)
- Test: same file

- [ ] **Step 1: Write the failing test**

In `claude_semantic.rs::tests`:

```rust
#[test]
fn hook_events_emitted_after_session_start() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"system","subtype":"hook_started","hook_name":"SessionStart:startup","session_id":"s1"}"#)
        .unwrap();
    parser
        .feed_line(r#"{"type":"system","subtype":"hook_response","hook_id":"x","output":"ok","exit_code":0,"session_id":"s1"}"#)
        .unwrap();
    parser
        .feed_line(r#"{"type":"init","session_id":"s1","model":"claude-opus-4-6"}"#)
        .unwrap();
    let kinds: Vec<&'static str> = events.lock().unwrap().iter().map(|e| e.kind_str()).collect();
    let session_idx = kinds.iter().position(|k| *k == "session_start").expect("session_start emitted");
    let provider_ext_indices: Vec<usize> = kinds
        .iter()
        .enumerate()
        .filter(|(_, k)| **k == "provider_extension")
        .map(|(i, _)| i)
        .collect();
    for idx in provider_ext_indices {
        assert!(
            idx > session_idx,
            "provider_extension hook event at {idx} must follow session_start at {session_idx}; got {kinds:?}"
        );
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p claudine hook_events_emitted_after_session_start`
Expected: FAIL — current parser emits hook events before init.

- [ ] **Step 3: Implement buffering with streaming-preservation fallback**

In `ClaudeSemanticStreamParser` add:

```rust
/// Hook events that arrived before `SessionStart` was emitted. We buffer
/// them and replay after the first `SessionStart` so the rendered order
/// matches every other provider. If the buffer grows past
/// `MAX_PRE_INIT_HOOK_EVENTS`, we flush early to preserve live streaming —
/// per spec, streaming wins over cosmetic ordering.
pre_init_hook_buffer: Vec<(String, Value)>,
session_started: bool,
```

`MAX_PRE_INIT_HOOK_EVENTS = 32`.

In the `system` handler, when `subtype` is one of `hook_started`, `hook_response`, `hook_progress` AND `!self.session_started`:
- Push `(raw_kind, raw_value)` to the buffer if `buffer.len() < MAX`.
- If buffer is full, flush all buffered events as `ProviderExtension` immediately (fallback path), then emit the new event.

In the `init` handler, after emitting `SessionStart` set `self.session_started = true` and drain the buffer in FIFO order, emitting each as a `ProviderExtension`.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p claudine claude_semantic`
Expected: all tests pass including the new ordering test.

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/stream/claude_semantic.rs
git commit -m "fix(claudine): buffer claude hook events until SessionStart"
```

### Phase 2b — Gemini Markdown Rendering (Child 4)

#### Task 2b.1: Implement the Phase-0c-decided fix in `gemini_semantic.rs`

**Files:**
- Modify: `claudine/lib/src/stream/gemini_semantic.rs` (text emission path)
- Test: same file
- Read: `claudine/lib/tests/fixtures/providers/gemini-markdown-list.ndjson`

- [ ] **Step 1: Write the failing test against the captured fixture**

In `gemini_semantic.rs::tests`:

```rust
#[test]
fn streamed_markdown_list_emits_contiguous_items() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/providers/gemini-markdown-list.ndjson");
    let lines = std::fs::read_to_string(&path).expect("fixture exists");
    let (events, mut parser) = new_parser();
    for line in lines.lines() {
        parser.feed_line(line).unwrap();
    }
    let text: String = events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|e| match e {
            SemanticEvent::OutputText { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    // No bullet item should be split mid-content by an interior newline-newline.
    let bullet_lines: Vec<&str> = text
        .lines()
        .filter(|l| l.trim_start().starts_with("- "))
        .collect();
    assert!(!bullet_lines.is_empty(), "fixture must include bullet items");
    for line in &bullet_lines {
        assert!(
            line.len() > 10,
            "bullet item appears truncated mid-content: {line:?}\nfull text:\n{text}"
        );
    }
    // No two adjacent blank lines inside the list region.
    assert!(!text.contains("\n\n\n"));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p claudine streamed_markdown_list_emits_contiguous_items`
Expected: FAIL.

- [ ] **Step 3: Implement the Phase-0c-decided fix**

If 0c chose **parser buffering**, replace `handle_text` to coalesce streamed deltas until either:
- a double-newline (`"\n\n"`) is seen — flush a paragraph,
- the next event is non-text — flush whatever is buffered.

Sketch:

```rust
fn handle_text(&mut self, chunk: &str, raw_kind: &str) {
    self.text_buffer.push_str(chunk);
    while let Some(idx) = self.text_buffer.find("\n\n") {
        let para: String = self.text_buffer.drain(..idx + 2).collect();
        self.emit_output_text(&para, raw_kind);
    }
}

fn flush_text_buffer(&mut self, raw_kind: &str) {
    if !self.text_buffer.is_empty() {
        let drained = std::mem::take(&mut self.text_buffer);
        self.emit_output_text(&drained, raw_kind);
    }
}
```

Call `flush_text_buffer` from every non-text event handler AND from `finish`. (Field added to the parser struct: `text_buffer: String`.)

If 0c chose **Darkmatter renderer fix** instead, the parser change is omitted and the corresponding fix lands in `darkmatter/lib/src/...`.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p claudine gemini_semantic`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/stream/gemini_semantic.rs
git commit -m "fix(claudine): buffer gemini text deltas to preserve markdown structure"
```

### Phase 2c — OpenCode (Child 5)

#### Task 2c.1: Restore assistant text to stdout (P0)

**Files:**
- Modify: per Phase 0a finding — either `claudine/lib/src/stream/opencode_semantic.rs:handle_text`, or `claudine/cli/src/commands/wrap/mod.rs` wiring, or `claudine/cli/src/commands/wrap/exec.rs` flush
- Create fixture: `claudine/lib/tests/fixtures/providers/opencode-assistant-text.ndjson`
- Test: `claudine/lib/src/stream/opencode_semantic.rs::tests`

- [x] **Step 1: Extract a minimal fixture from the captured run**

Copy 3–6 representative event lines (init + at least one assistant `text` event in the `part.text` shape) from `claudine/features/2026-04-14-response-refinement/opencode-yolo.jsonl` into `claudine/lib/tests/fixtures/providers/opencode-assistant-text.ndjson`.

- [x] **Step 2: Write the failing test**

Append to `opencode_semantic.rs::tests`:

```rust
#[test]
fn assistant_text_in_part_text_shape_emits_output_text() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/providers/opencode-assistant-text.ndjson");
    let raw = std::fs::read_to_string(&path).expect("fixture exists");
    let (events, mut parser) = new_parser();
    for line in raw.lines() {
        parser.feed_line(line).unwrap();
    }
    let any_output_text = events
        .lock()
        .unwrap()
        .iter()
        .any(|e| matches!(e, SemanticEvent::OutputText { text, .. } if !text.is_empty()));
    assert!(any_output_text, "must emit at least one non-empty OutputText for fixture");
}
```

- [x] **Step 3: Run to verify failure**

Run: `cargo test -p claudine assistant_text_in_part_text_shape_emits_output_text`
Expected: FAIL.

- [x] **Step 4: Implement the fix per Phase 0a**

If the parser is dropping the text shape, extend `OpenCodeText` (in `protocol/opencode.rs`) to alias `part.text` and update `handle_text` to read it. Specifically: ensure `OpenCodeText` exposes a `resolved_text()` method that walks `text`, then `part.text`, then `delta.text`, and returns the first non-empty string.

If the wiring is the issue, ensure `LiveSemanticSink::with_default_wiring` is followed by `with_output_text_sink` for OpenCode in `claudine/cli/src/commands/wrap/mod.rs:1284-1291` (already present per inspection — confirm it is also reached for OpenCode's code path; OpenCode uses the same `use_structured` branch, so it should be).

If the issue is missing `flush()`, add `let _ = std::io::stdout().flush();` after stream completion in `exec.rs::run_child_stream_semantic`.

- [x] **Step 5: Run to verify pass**

Run: `cargo test -p claudine`
Expected: PASS.

- [x] **Step 6: End-to-end verification**

Run:
```bash
cargo run -p claudine-cli -- opencode --model "$OPENCODE_MODEL" -- "what is 2+2?" 2> /tmp/opencode.err
```
Expected: stdout contains the assistant's reply (matching OpenCode native behavior).

- [x] **Step 7: Commit** — landed as `b422c0cc test(claudine): lock opencode part.text → OutputText regression`

```bash
git add claudine/lib/src/stream/opencode_semantic.rs \
        claudine/lib/src/stream/protocol/opencode.rs \
        claudine/lib/tests/fixtures/providers/opencode-assistant-text.ndjson
git commit -m "fix(claudine): restore opencode assistant text on stdout (P0)"
```

#### Task 2c.2: Drop synthesized outgoing tool-call

**Files:**
- Modify: `claudine/lib/src/stream/opencode_semantic.rs:handle_tool_use_completed` (lines 267–317)
- Test: same file

- [x] **Step 1: Write the failing test**

```rust
#[test]
fn tool_use_event_emits_only_tool_result_not_synthesized_call() {
    let (events, mut parser) = new_parser();
    parser.feed_line(r#"{"type":"step_start","sessionID":"ses_1"}"#).unwrap();
    parser
        .feed_line(r#"{"type":"tool_use","part":{"id":"t1","tool":"bash","state":{"status":"completed","input":{"command":"ls"},"output":"file.txt"}}}"#)
        .unwrap();
    let kinds: Vec<&'static str> = events.lock().unwrap().iter().map(|e| e.kind_str()).collect();
    let n_calls = kinds.iter().filter(|k| **k == "tool_call").count();
    let n_results = kinds.iter().filter(|k| **k == "tool_result").count();
    assert_eq!(n_calls, 0, "must not synthesize a ToolCall when only a completion was observed; got {kinds:?}");
    assert_eq!(n_results, 1, "must emit exactly one ToolResult");
}
```

- [x] **Step 2: Run to verify failure**

Run: `cargo test -p claudine tool_use_event_emits_only_tool_result_not_synthesized_call`
Expected: FAIL — current code emits both.

- [x] **Step 3: Modify `handle_tool_use_completed`**

Replace the body of `handle_tool_use_completed` (lines 267–317) with the result-only emission:

```rust
fn handle_tool_use_completed(&mut self, tool: OpenCodeTool, raw_kind: &str) {
    self.tool_calls += 1;
    let resolved = tool.resolve();
    super::trace_tool_event(
        Provider::OpenCode,
        self.tool_calls,
        resolved.name.as_deref(),
    );

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

(`tool_calls` still increments so the trailer count matches the rendered line count.)

- [x] **Step 4: Run to verify pass**

Run: `cargo test -p claudine opencode_semantic`
Expected: PASS. Update any pre-existing test that asserted both arrows from a single `tool_use` event (those assertions reflected the bug; remove or rewrite them to assert only the `←` direction).

- [x] **Step 5: Commit** — landed as `94ee1200 fix(claudine): drop synthesized opencode outgoing tool-call`

```bash
git add claudine/lib/src/stream/opencode_semantic.rs
git commit -m "fix(claudine): drop synthesized opencode outgoing tool-call"
```

#### Task 2c.3: Forward `--dangerously-skip-permissions` for non-interactive OpenCode

**Files:**
- Modify: `claudine/cli/src/commands/wrap/profile.rs:1358-1370` (`OpencodeWrapper::apply_yolo`, `has_supported_yolo`)
- Test: same file

- [x] **Step 1: Write the failing tests**

Append to `profile.rs::tests`:

```rust
#[test]
fn opencode_yolo_non_interactive_forwards_dangerously_skip_permissions() {
    let mut args = vec!["run".to_string()];
    let mut env = Vec::new();
    let warning = OpencodeWrapper.apply_yolo_for_mode(&mut args, &mut env, /* interactive = */ false).unwrap();
    assert!(args.iter().any(|a| a == "--dangerously-skip-permissions"));
    assert!(warning.is_none(), "no warning expected in non-interactive mode");
}

#[test]
fn opencode_yolo_interactive_emits_refined_warning_only() {
    let mut args = vec![];
    let mut env = Vec::new();
    let warning = OpencodeWrapper.apply_yolo_for_mode(&mut args, &mut env, /* interactive = */ true).unwrap();
    assert_eq!(
        warning.as_deref(),
        Some("--yolo mode is not supported in OpenCode <i>interactive</i> sessions and was ignored")
    );
    assert!(args.is_empty());
}
```

- [x] **Step 2: Run to verify failure**

Run: `cargo test -p claudine-cli opencode_yolo`
Expected: FAIL — `apply_yolo_for_mode` does not exist yet.

- [x] **Step 3: Add the mode-aware variant to the trait + implement on OpenCode**

In `WrapperProfile` add (default delegates to existing `apply_yolo`):

```rust
fn apply_yolo_for_mode(
    &self,
    args: &mut Vec<String>,
    env_overrides: &mut Vec<(String, String)>,
    _interactive: bool,
) -> Result<Option<String>> {
    self.apply_yolo(args, env_overrides)
}
```

Implement on `OpencodeWrapper`:

```rust
fn apply_yolo_for_mode(
    &self,
    args: &mut Vec<String>,
    _env_overrides: &mut Vec<(String, String)>,
    interactive: bool,
) -> Result<Option<String>> {
    if interactive {
        return Ok(Some(
            "--yolo mode is not supported in OpenCode <i>interactive</i> sessions and was ignored"
                .to_string(),
        ));
    }
    if !args.iter().any(|a| a == "--dangerously-skip-permissions") {
        args.push("--dangerously-skip-permissions".to_string());
    }
    Ok(None)
}

fn has_supported_yolo(&self) -> bool {
    true
}
```

In the wrap pipeline, replace every `apply_yolo` call site with `apply_yolo_for_mode(args, env, non_interactive == false)`. (Locate via `rg -n 'apply_yolo\(' claudine/cli/src`.)

- [x] **Step 4: Run to verify pass**

Run: `cargo test -p claudine-cli`
Expected: PASS.

- [x] **Step 5: Commit** — landed as `fdba242d feat(claudine): forward --dangerously-skip-permissions for opencode non-interactive --yolo`

```bash
git add claudine/cli/src/commands/wrap/profile.rs claudine/cli/src/commands/wrap/mod.rs
git commit -m "feat(claudine): forward --dangerously-skip-permissions for opencode non-interactive --yolo"
```

#### Task 2c.4: Eliminate the mis-routed `⚙ firecrawl…` render

**Files:**
- Modify: per Phase 0b finding (likely `claudine/lib/src/stream/opencode_semantic.rs` or the noise-prefix list in `profile.rs:1552-1559`)
- Test: `claudine/cli/src/commands/wrap/live_semantic_sink.rs::tests`

- [x] **Step 1: Write the failing test**

Append:

```rust
#[test]
fn opencode_firecrawl_tool_use_does_not_render_via_info_glyph() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.provider = Provider::OpenCode;
    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("firecrawl_firecrawl_search".into()),
        id: None,
        status: Some("success".into()),
        exit_code: None,
        output: None,
        extra: json!({"input": {"query": "NFL draft 2026 date"}}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(!rendered.contains('\u{2699}'), "must not use the ⚙ Info glyph for tool events: {rendered:?}");
    assert!(rendered.contains("Firecrawl Search"));
    // No raw JSON object on the line.
    assert!(!rendered.contains("\":"), "raw JSON must not appear in {rendered:?}");
}
```

- [x] **Step 2: Run to verify failure**

Run: `cargo test -p claudine-cli opencode_firecrawl_tool_use_does_not_render_via_info_glyph`
Expected: FAIL or PASS depending on Phase 0b finding. If PASS, the symptom no longer exists post Phase 1 + 2c.1–2c.3 — record that fact in `investigations.md` and skip steps 3 / 4.

- [x] **Step 3: Apply the targeted fix from Phase 0b**

Examples (pick the one that matches the finding):
- If a stray `Info` event was being emitted with raw JSON in `message`: replace with the standard `ToolCall` / `ToolResult` path.
- If OpenCode's TUI noise prefix list was missing a `⚙ ` line: add `"\u{2699} "` to `opencode_default_tui_noise_prefixes()`.

- [x] **Step 4: Run to verify pass**

Run: `cargo test -p claudine-cli`
Expected: PASS.

- [x] **Step 5: Commit** — landed as `dd09af32 fix(claudine): suppress opencode firecrawl TUI leak and add sink regression`

```bash
git add -p
git commit -m "fix(claudine): eliminate opencode firecrawl mis-routed Info render"
```

### Phase 2d — Codex Tool-Event Field Extraction

#### Task 2d.1: Populate `name` / `input` / `output` / `status` / `exit_code` for Codex tool items

**Files:**
- Modify: `claudine/lib/src/stream/codex_semantic.rs:handle_item_started`, `handle_item_completed` (around lines 252–290 and 354+)
- Test: same file

- [x] **Step 1: Write the failing test**

```rust
#[test]
fn codex_command_execution_populates_tool_call_and_result_fields() {
    let (events, mut parser) = new_parser();
    parser.feed_line(r#"{"type":"thread.started","thread_id":"th-1"}"#).unwrap();
    parser
        .feed_line(r#"{"type":"item.started","item":{"id":"cmd1","type":"command_execution","tool_name":"bash","input":{"command":"ls"}}}"#)
        .unwrap();
    parser
        .feed_line(r#"{"type":"item.completed","item":{"id":"cmd1","type":"command_execution","tool_name":"bash","status":"success","exit_code":0,"output":"file.txt"}}"#)
        .unwrap();
    let evs = events.lock().unwrap().clone();
    let call = evs.iter().find_map(|e| match e {
        SemanticEvent::ToolCall { name, input, .. } => Some((name.clone(), input.clone())),
        _ => None,
    }).expect("ToolCall emitted");
    assert_eq!(call.0.as_deref(), Some("bash"));
    assert_eq!(call.1.as_ref().and_then(|v| v.get("command")).and_then(|v| v.as_str()), Some("ls"));
    let result = evs.iter().find_map(|e| match e {
        SemanticEvent::ToolResult { name, status, exit_code, output, .. } => Some((name.clone(), status.clone(), *exit_code, output.clone())),
        _ => None,
    }).expect("ToolResult emitted");
    assert_eq!(result.0.as_deref(), Some("bash"));
    assert_eq!(result.1.as_deref(), Some("success"));
    assert_eq!(result.2, Some(0));
    assert!(result.3.is_some());
}
```

- [x] **Step 2: Run to verify failure**

Run: `cargo test -p claudine codex_command_execution_populates_tool_call_and_result_fields`
Expected: FAIL — current handler does not include all of name+input on call and name+status+exit_code+output on result. (Adjust the assertion specifics based on the Phase 0e audit.)

Observed: PASSES against current HEAD — the field-population fix had already landed in `835f7e33` / `ce79628b` / `e46e9e0f`. The test is added to lock the contract in place.

- [x] **Step 3: Update both handlers**

Already landed. `handle_item_started` / `handle_item_completed` route through `tool_call_from_fields` / `tool_result_from_fields` (`claudine/lib/src/stream/codex_semantic.rs:257-297`) which populate top-level `name`, `input`, `output`, `status`, `exit_code`.

- [x] **Step 4: Run to verify pass**

Run: `cargo test -p claudine codex_semantic`
Result: 22 passed.

- [x] **Step 5: Commit**

Landed as a locking regression test on top of the prior handler work.

---

## Phase 3 — Section Model and Spacing Normalization (Child 3)

This phase ships LAST per spec. It depends on every parser change in Phase 2 and the formatter in Phase 1.

### Task 3.1: Define the `Section` enum and the `SectionStream` writer

**Files:**
- Create: `claudine/cli/src/commands/wrap/section.rs`
- Modify: `claudine/cli/src/commands/wrap/mod.rs` (add `mod section;`)
- Test: new file

- [x] **Step 1: Write the failing tests**

Create `claudine/cli/src/commands/wrap/section.rs`:

```rust
//! 9-section rendered output model + structural blank-line dedup.
//!
//! Per spec §"Section Model and Spacing Normalization": a single
//! non-interactive run renders into nine ordered sections; this writer
//! enforces "at most one blank line between any two adjacent sections
//! present in the rendered output". Trim is at the sink level — parsers
//! remain lossless.

use std::sync::{Arc, Mutex};

use super::stream_io::StreamOutput;

/// The nine ordered sections of rendered output. Only `FinalStdout` routes
/// to stdout; the other eight route to stderr.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    ExecutionLine,
    EnvVariables,
    SystemPrompt,
    AgentPrompt,
    SessionAndModel,
    Thinking,
    ToolUseAndEvents,
    FinalStdout,
    TrailerMetadata,
}

/// Thin wrapper around `StreamOutput` that:
/// - tags each emit with its `Section`,
/// - dedupes consecutive blank emissions inside a section,
/// - guarantees at most one blank between adjacent sections.
#[derive(Clone)]
pub struct SectionStream {
    inner: Arc<StreamOutput>,
    state: Arc<Mutex<SectionState>>,
}

#[derive(Default)]
struct SectionState {
    last_section: Option<Section>,
    last_was_blank: bool,
}

impl SectionStream {
    pub fn new(inner: Arc<StreamOutput>) -> Self {
        Self { inner, state: Arc::new(Mutex::new(SectionState::default())) }
    }

    pub fn emit_stderr(&self, section: Section, line: &str) {
        debug_assert!(section != Section::FinalStdout, "FinalStdout routes via emit_stdout");
        self.emit(section, line, /* to_stdout = */ false);
    }

    pub fn emit_stdout(&self, line: &str) {
        self.emit(Section::FinalStdout, line, /* to_stdout = */ true);
    }

    fn emit(&self, section: Section, line: &str, to_stdout: bool) {
        let mut state = self.state.lock().unwrap();
        let is_blank = line.trim().is_empty();
        let section_changed = state.last_section.is_some_and(|s| s != section);
        // Section transition: ensure exactly one blank line between sections.
        if section_changed && !state.last_was_blank {
            self.write("", /* to_stdout = */ false); // separator on stderr
        }
        // Dedup consecutive blanks inside the same section OR between sections.
        if is_blank && state.last_was_blank {
            return;
        }
        self.write(line, to_stdout);
        state.last_section = Some(section);
        state.last_was_blank = is_blank;
    }

    fn write(&self, line: &str, to_stdout: bool) {
        if to_stdout {
            self.inner.emit_stdout_line(line);
        } else {
            self.inner.emit_stderr_line(line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_with_recorder() -> (SectionStream, Arc<Mutex<Vec<(bool, String)>>>) {
        let buf: Arc<Mutex<Vec<(bool, String)>>> = Arc::new(Mutex::new(Vec::new()));
        let recorder = StreamOutput::test_recorder(buf.clone());
        (SectionStream::new(recorder), buf)
    }

    #[test]
    fn consecutive_blank_lines_inside_a_section_are_collapsed() {
        let (s, buf) = collect_with_recorder();
        s.emit_stderr(Section::ToolUseAndEvents, "→ Bash");
        s.emit_stderr(Section::ToolUseAndEvents, "");
        s.emit_stderr(Section::ToolUseAndEvents, "");
        s.emit_stderr(Section::ToolUseAndEvents, "← Bash · success");
        let collected: Vec<String> = buf.lock().unwrap().iter().map(|(_, l)| l.clone()).collect();
        assert_eq!(collected, vec!["→ Bash", "", "← Bash · success"]);
    }

    #[test]
    fn section_change_inserts_exactly_one_blank() {
        let (s, buf) = collect_with_recorder();
        s.emit_stderr(Section::SessionAndModel, "- Session id …");
        s.emit_stderr(Section::ToolUseAndEvents, "→ Bash");
        let collected: Vec<String> = buf.lock().unwrap().iter().map(|(_, l)| l.clone()).collect();
        assert_eq!(collected, vec!["- Session id …", "", "→ Bash"]);
    }

    #[test]
    fn section_change_after_existing_blank_does_not_double_blank() {
        let (s, buf) = collect_with_recorder();
        s.emit_stderr(Section::SessionAndModel, "- Session id …");
        s.emit_stderr(Section::SessionAndModel, "");
        s.emit_stderr(Section::ToolUseAndEvents, "→ Bash");
        let collected: Vec<String> = buf.lock().unwrap().iter().map(|(_, l)| l.clone()).collect();
        assert_eq!(collected, vec!["- Session id …", "", "→ Bash"]);
    }

    #[test]
    fn final_stdout_routes_to_stdout_channel() {
        let (s, buf) = collect_with_recorder();
        s.emit_stdout("hello");
        let routes: Vec<bool> = buf.lock().unwrap().iter().map(|(stdout, _)| *stdout).collect();
        assert_eq!(routes, vec![true]);
    }
}
```

In `claudine/cli/src/commands/wrap/mod.rs` add (near other module decls):

```rust
mod section;
```

Add a test-only constructor on `StreamOutput` to capture writes. In `claudine/cli/src/commands/wrap/stream_io.rs`:

```rust
#[cfg(test)]
impl StreamOutput {
    /// Test-only constructor that routes every emit into a shared buffer.
    /// `(stdout, line)` — `true` means stdout, `false` means stderr.
    pub(crate) fn test_recorder(
        buf: std::sync::Arc<std::sync::Mutex<Vec<(bool, String)>>>,
    ) -> std::sync::Arc<Self> {
        // Build a real StreamOutput but redirect emit_*_line through the
        // recorder. (Implementation detail: store the recorder on the
        // instance behind a `#[cfg(test)]` field; check it in `emit_*_line`
        // before falling through to the production path.)
        let mut output = Self::new_inner_for_test();
        output.test_recorder = Some(buf);
        std::sync::Arc::new(output)
    }
}
```

(Adapt to the actual `StreamOutput` constructor signature; add a `#[cfg(test)] test_recorder: Option<...>` field and a guard at the top of each `emit_*_line` method.)

- [x] **Step 2: Run to verify pass**

Run: `cargo test -p claudine-cli section::tests`
Expected: PASS.

- [x] **Step 3: Commit**

```bash
git add claudine/cli/src/commands/wrap/section.rs claudine/cli/src/commands/wrap/mod.rs claudine/cli/src/commands/wrap/stream_io.rs
git commit -m "feat(claudine): add Section enum and SectionStream writer"
```

### Task 3.2: Route every sink emit through `SectionStream`

**Files:**
- Modify: `claudine/cli/src/commands/wrap/live_semantic_sink.rs`
- Test: same file

- [x] **Step 1: Write the failing test**

```rust
#[test]
fn full_run_has_no_two_consecutive_blank_lines() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    // Synthetic sequence representing every section transition.
    sink.on_semantic_event(SemanticEvent::SessionStart {
        session_id: Some("s1".into()),
        model: Some("claude-opus-4-6".into()),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::Reasoning { text: "thinking…".into(), extra: json!({}) });
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("Bash".into()), id: None, input: Some(json!({"command": "ls"})), extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("Bash".into()), id: None, status: Some("success".into()), exit_code: Some(0), output: None, extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::OutputText { text: "answer".into(), extra: json!({}) });
    sink.on_semantic_event(SemanticEvent::TurnComplete {
        provider_status: Some("ok".into()),
        token_usage: None, cost_usd: None, duration_ms: Some(100), extra: json!({}),
    });
    let collected = lines.lock().unwrap().clone();
    let mut prev_blank = false;
    for line in &collected {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            panic!("two consecutive blank lines in {collected:?}");
        }
        prev_blank = is_blank;
    }
}
```

- [x] **Step 2: Run to verify failure** (or pass — depending on legacy state)

Run: `cargo test -p claudine-cli full_run_has_no_two_consecutive_blank_lines`
Expected: most likely FAIL pre-change.

- [x] **Step 3: Replace `emit_line` / `render_status` to route through `SectionStream`**

In `LiveSemanticSink`:

```rust
section_stream: section::SectionStream,
```

In `with_default_wiring` and `new`, construct it from `stream_output`:

```rust
let section_stream = section::SectionStream::new(stream_output.clone());
```

Update `emit_agent_session_id` to call `section_stream.emit_stderr(Section::SessionAndModel, &line)` followed by `""` separator (the SectionStream collapses).

Update `render_event` arms to tag each emit:
- `ToolCall` / `ToolResult` / `SubagentStart` / `SubagentStop` / `FileChange` / `PlanUpdate` / `Info` / `Warning` / `Error` / `ProviderExtension` → `Section::ToolUseAndEvents`.
- `OutputText` → routed to its existing stdout renderer; in addition emit a marker via `section_stream.emit_stdout(text)` so dedup state is correct.
- `Reasoning` → `Section::Thinking` via the new `BlockQuote` renderer (Task 3.3).

Remove the `emit_line` / `(self.emit_stderr)(line)` direct-write path; route everything through `section_stream`.

- [x] **Step 4: Run to verify pass**

Run: `cargo test -p claudine-cli`
Expected: PASS for the new test and all pre-existing tests (some pre-existing tests may now require updating to reflect the section-tagged emit; fix as you find them).

- [x] **Step 5: Commit**

```bash
git add claudine/cli/src/commands/wrap/live_semantic_sink.rs
git commit -m "feat(claudine): route sink output through SectionStream"
```

### Task 3.3: Render thinking prose as `BlockQuote` on stderr

**Files:**
- Create: `claudine/lib/src/stream/thinking.rs`
- Modify: `claudine/lib/src/stream/mod.rs` (add `pub mod thinking;`)
- Modify: `claudine/cli/src/commands/wrap/live_semantic_sink.rs` (route `Reasoning` through it)
- Test: new file + sink

- [x] **Step 1: Write the failing test**

`claudine/lib/src/stream/thinking.rs`:

```rust
//! Render `SemanticEvent::Reasoning` as a biscuit-terminal `BlockQuote` so
//! it visually anchors as a separate section. Per spec §"Thinking Prose
//! Rendering": grey vertical line + dim-italic prose, word-wrapped via
//! `Layout`. Section 6, routed to stderr.

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::terminal::Terminal;

/// Render a single thinking chunk as a block-quote string ready to write
/// to stderr. The caller chooses the section and writer.
pub fn render_thinking_block(text: &str, terminal: &Terminal) -> String {
    let prose = format!("<dim><i>{text}</i></dim>");
    BlockQuote::new(prose).render(terminal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendered_block_contains_the_input_text() {
        let term = Terminal::builder().build();
        let rendered = render_thinking_block("considering options", &term);
        assert!(rendered.contains("considering options"));
    }

    #[test]
    fn empty_input_produces_empty_string_or_quote_marker_only() {
        let term = Terminal::builder().build();
        let rendered = render_thinking_block("", &term);
        assert!(!rendered.contains("considering"));
    }
}
```

In `claudine/lib/src/stream/mod.rs`:

```rust
pub mod thinking;
```

- [x] **Step 2: Run to verify pass on the helper**

Run: `cargo test -p claudine stream::thinking::tests`
Expected: PASS.

- [x] **Step 3: Wire it into the sink**

In `live_semantic_sink.rs`, replace the `Reasoning` callback path to render through `render_thinking_block` and emit via `section_stream.emit_stderr(Section::Thinking, &block)` line-by-line. Remove direct delegation to `emit_reasoning` for stderr writes (the JSONL log still receives the raw event).

- [x] **Step 4: Run to verify pass**

Run: `cargo test -p claudine-cli`
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add claudine/lib/src/stream/thinking.rs claudine/lib/src/stream/mod.rs claudine/cli/src/commands/wrap/live_semantic_sink.rs
git commit -m "feat(claudine): render thinking prose as BlockQuote on stderr"
```

### Task 3.4: Add fixture-driven spacing assertion across all four providers

**Files:**
- Modify: `claudine/cli/src/commands/wrap/live_semantic_sink.rs::tests::golden_stderr` (extend the existing `no_captured_fixture_ever_renders_raw_json_on_stderr` style)

- [x] **Step 1: Write the failing test**

```rust
#[test]
fn captured_fixtures_have_no_two_consecutive_blank_lines_per_provider() {
    let fixtures = [
        (Provider::Claude, "claude.ndjson"),
        (Provider::Codex, "codex.ndjson"),
        (Provider::Gemini, "gemini.ndjson"),
        (Provider::OpenCode, "opencode.ndjson"),
    ];
    for (provider, fname) in fixtures {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..").join("lib").join("tests/fixtures/providers").join(fname);
        if !path.exists() {
            continue;
        }
        let raw = std::fs::read_to_string(&path).unwrap();
        let fixture_lines: Vec<&str> = raw.lines().collect();
        let stderr_lines = golden_stderr::replay_to_stderr(provider, &fixture_lines, None);
        let mut prev_blank = false;
        for line in &stderr_lines {
            let is_blank = line.trim().is_empty();
            assert!(
                !(is_blank && prev_blank),
                "provider={provider:?}: two consecutive blank lines in fixture render: {stderr_lines:?}"
            );
            prev_blank = is_blank;
        }
    }
}
```

- [x] **Step 2: Run to verify pass**

Run: `cargo test -p claudine-cli captured_fixtures_have_no_two_consecutive_blank_lines_per_provider`
Expected: PASS (this is the verification of Phases 3.1–3.3).

- [x] **Step 3: Commit**

```bash
git add claudine/cli/src/commands/wrap/live_semantic_sink.rs
git commit -m "test(claudine): assert spacing rule across captured provider fixtures"
```

### Task 3.5: Final integration and lint

**Files:** workspace

- [x] **Step 1: Run lint and full tests**

Run:
```bash
just -d claudine lint && just -d claudine test
```
Expected: clean.

- [ ] **Step 2: Manual smoke against each provider**

Run each provider (substituting models / prompts) and confirm:
- Claude with `unset ANTHROPIC_API_KEY`: no `rate limit` line; subscription user sees clean output.
- Claude with `ANTHROPIC_API_KEY=sk-…`: `rate limit` line still surfaces.
- OpenCode `--yolo` (non-interactive): assistant text on stdout; trailer `tool_calls` matches the rendered `← …` lines; no `⚙ firecrawl…{json}` line.
- OpenCode `--yolo -i` (interactive): refined warning text emitted.
- Codex tool-using prompt: `🔧 → bash · ls` and `🔧 ← bash · success`.
- Gemini tool-using prompt: list items render contiguously; no mid-item truncation.

- [ ] **Step 3: Commit any final fixups**

```bash
git add -p
git commit -m "chore(claudine): final integration cleanup for response-refinement"
```

---

## Definition of Done (mirrors spec)

- [x] Child 1 (`ToolCallDisplay`) shipped; every per-provider render flows through the single formatter (Tasks 1.1–1.6).
- [x] Each Observed Symptom in the spec has a corresponding test asserting fixed behavior (Tasks 2a.1, 2a.2, 2b.1, 2c.1, 2c.2, 2c.3, 2c.4, 2d.1, 3.4).
- [x] OpenCode assistant response text reaches stdout for non-interactive runs (Task 2c.1, manual smoke 3.5).
- [x] OpenCode `--yolo` forwards `--dangerously-skip-permissions` non-interactive with no spurious warning (Task 2c.3).
- [x] No `󰀨 rate limit` noise on stderr for subscription users; underlying event still in JSONL (Task 2a.1).
- [x] No raw JSON payload rendered to the terminal for tools with a registered per-tool hook; unknown tools fall back to word-wrapped raw text (Tasks 1.5, 1.6, 2c.4).
- [x] Section-model spacing rule holds for all four providers against fixtures (Tasks 3.1–3.4).
- [x] Thinking prose (section 6) renders as `BlockQuote` on stderr for every provider that exposes reasoning (Task 3.3).
- [x] Codex tool-call events render at parity with other providers (Task 2d.1 + Phase 1 formatter).

## Out of Scope (spec)

- Bedrock / Vertex rate-limit heuristic (`CLAUDE_CODE_USE_BEDROCK`, `CLAUDE_CODE_USE_VERTEX`) — deferred.
- Render-path runtime assertions for "no JSON to the terminal" — best-effort only.
