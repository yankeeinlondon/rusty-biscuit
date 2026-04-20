# Execution Plan: More Meta Response

Source: [spec.md](./spec.md) · [tech-design.md](./tech-design.md)

---

## Phase 0 — Upstream Dependency (biscuit-terminal)

No claudine code depends on this, so it can start immediately and run in parallel with Phase 1.

### 0.1 Add `StatusState::Subagent`

**Files:** `biscuit-terminal/lib/src/components/status.rs`

1. Add `Subagent` variant to `StatusState` enum (after `ToolUse`).
2. Add Nerd Font icon constants for all three themes:
   - `NERD_CIRCULAR_SUBAGENT`
   - `NERD_ROUNDED_SUBAGENT`
   - `NERD_TIMELINE_SUBAGENT`
3. Add a Unicode fallback constant `FB_SUBAGENT`.
4. Insert 3 new entries into `ICON_LOOKUP` (one per theme). Choose a color that distinguishes subagents from tools (e.g. `Tailwind::Violet500`).
5. Update the `ICON_LOOKUP` capacity from 21 → 24.
6. Update any exhaustive match arms or serialization tests in the file.

**Validation:**
- `cargo test -p biscuit-terminal` passes.
- `cargo clippy -p biscuit-terminal` clean.
- Manual: instantiate `Status::new(StatusState::Subagent, "test")` and render to confirm icon appears.

---

## Phase 1 — Semantic Event Model & New Traits (library foundation)

All subsequent phases depend on this. No parser changes yet.

### 1.1 Create `semantic.rs`

**File:** `claudine/lib/src/stream/semantic.rs` (new)

1. Define `SemanticEvent` enum with all variants from the tech design:
   `SessionStart`, `TurnStart`, `TurnComplete`, `OutputText`, `Reasoning`, `ToolCall`, `ToolResult`, `PermissionRequest`, `SubagentStart`, `SubagentStop`, `FileChange`, `PlanUpdate`, `Info`, `Warning`, `Error`, `ProviderExtension`.
2. Every typed variant carries `extra: serde_json::Value`.
3. `ProviderExtension` carries `provider: Provider`, `kind: String`, `payload: serde_json::Value`.
4. Derive `Debug`, `Clone`, `Serialize`, `Deserialize` on the enum.
5. Add helper methods:
   - `fn kind_str(&self) -> &str` — returns the variant name as a static string.
   - `fn is_activity(&self) -> bool` — returns true for event families that count as "activity" per the heartbeat design (OutputText, Reasoning, ToolCall, ToolResult, SubagentStart, SubagentStop, FileChange, PlanUpdate, Info, Warning, Error, ProviderExtension).

### 1.2 Define `SemanticEventSink` trait

**File:** `claudine/lib/src/stream/parser.rs` (modify)

1. Add the new trait alongside the existing one (do not delete old traits yet):
   ```rust
   pub trait SemanticEventSink: Send {
       fn on_semantic_event(&mut self, event: SemanticEvent);
   }
   ```
2. Implement `SemanticEventSink` for `NullSink` (no-op).

### 1.3 Update `StreamParser` trait signature

**File:** `claudine/lib/src/stream/parser.rs` (modify)

1. Change `StreamParser::feed_line` return type from `Result<Option<StreamChunk>, StreamParseError>` to `Result<(), StreamParseError>`.
2. Keep `StreamChunk` defined (used by consumers until Phase 3 migration), but it is no longer part of the parser return contract.
3. Keep `StreamParseError::MalformedLine` variant for now — parsers will transition to emitting `Warning` events internally during Phase 2.

### 1.4 Register the new module

**File:** `claudine/lib/src/stream/mod.rs`

1. Add `pub mod semantic;`.
2. Re-export `SemanticEvent` and `SemanticEventSink` at the stream module level.
3. Update `create_parser` signature: parsers now accept `impl SemanticEventSink + 'static` instead of `impl StreamEventSink + 'static`.

### 1.5 Stub adapter: `StreamEventSink` → `SemanticEventSink`

**File:** `claudine/lib/src/stream/parser.rs` (modify)

1. Add a `BridgeSink` struct that wraps a `Box<dyn StreamEventSink>` and implements `SemanticEventSink` by mapping semantic events back to the old callbacks. This allows Phase 1 to compile without rewriting all consumers yet.
2. This bridge is temporary and will be deleted in Phase 3.

**Validation checkpoint (end of Phase 1):**
- `cargo test -p claudine` passes (no parser behavior changes yet, bridge preserves old contract).
- `cargo clippy -p claudine` clean.
- New `semantic.rs` has unit tests for `kind_str()`, `is_activity()`, and serde round-trip of every variant.

---

## Phase 2 — Parser Migration (provider-by-provider)

Each parser migration is independent once Phase 1 is complete. Claude and Codex go first (highest value); the remaining four can be parallelized.

### General pattern per parser

For each provider parser (`{provider}.rs` + `protocol/{provider}.rs`):

1. **Expand protocol types** — add missing typed variants/fields identified in the tech design (especially Codex: `ItemUpdated`, `FileChange`, `PlanUpdate`, `Reasoning` routing, `status`/`exit_code` on `CommandExecution`).
2. **Switch sink type** — change the parser struct to hold `Box<dyn SemanticEventSink>` instead of `Box<dyn StreamEventSink>`.
3. **Rewrite `feed_line`** — implement the two-pass dispatch (parse to `Value`, then typed deser, then emit `SemanticEvent`). Eliminate `_ => Ok(None)` silent drops; unknown parseable events become `ProviderExtension`.
4. **Add error/warning allowlist** — explicit mapping of raw event type strings to `SemanticEvent::Error` / `SemanticEvent::Warning`. Non-allowlisted events fall through to `ProviderExtension`.
5. **Malformed JSON → Warning** — emit `SemanticEvent::Warning` for unparseable lines instead of returning `StreamParseError::MalformedLine`. Return `Ok(())`.
6. **OutputText / Reasoning** — emit `SemanticEvent::OutputText` for assistant text and `SemanticEvent::Reasoning` for thinking/reasoning text, replacing the old `StreamChunk` return.
7. **Update protocol tests** — add cases for new variants, field aliases, typed-to-extension fallback, and allowlist classification.

### 2.1 Claude parser

**Files:** `claudine/lib/src/stream/claude.rs`, `claudine/lib/src/stream/protocol/claude.rs`

- Already the richest parser. Primary work: re-route existing callbacks to `SemanticEvent` emissions.
- Map `task_started`/`task_progress`/`task_notification` → `SubagentStart`/`Info`/`SubagentStop` or `ProviderExtension`.
- Map `rate_limit_event`, `system` events → allowlist to `Warning` or `Error`.
- Map `tool_use` → `ToolCall`, `tool_result` → `ToolResult`.
- Map assistant content deltas → `OutputText`, thinking deltas → `Reasoning`.

**Validation:** `cargo test -p claudine -- claude` passes. Round-trip fidelity test for Claude fixtures.

### 2.2 Codex parser (highest priority — motivating failure)

**Files:** `claudine/lib/src/stream/codex.rs`, `claudine/lib/src/stream/protocol/codex.rs`

Protocol expansion (before parser changes):
- Add `ItemUpdated` variant to `CodexEvent`.
- Add `FileChange` variant to `CodexItem` enum.
- Add `PlanUpdate` (or `TodoList`) variant to `CodexItem`.
- Add `status` and `exit_code` fields to `CodexToolItemFields` (for `command_execution`).
- Add `mcp_tool_call`, `collab_tool_call`, `web_search` as typed or extension-ready item types.

Parser changes:
- Route `reasoning` items through `SemanticEvent::Reasoning` (currently parsed but not surfaced to thinking renderer).
- Route `item.updated` → `Info` or `ProviderExtension` depending on content.
- Route `file_change` → `FileChange`, `plan_update` → `PlanUpdate`.

**Validation:** `cargo test -p claudine -- codex` passes. Verify that the previously-bare `30s / 60s / 90s` heartbeat ladder is replaced by actual semantic events in a captured Codex fixture replay.

### 2.3 Gemini parser

**Files:** `claudine/lib/src/stream/gemini.rs`, `claudine/lib/src/stream/protocol/gemini.rs`

- Smaller event set. Map existing events to semantic equivalents.
- Ensure unknown event types become `ProviderExtension`.

### 2.4 OpenCode parser

**Files:** `claudine/lib/src/stream/opencode.rs`, `claudine/lib/src/stream/protocol/opencode.rs`

- Map `step_start`/`step_finish` → `Info` or `ProviderExtension`.
- Route subagent task events → `SubagentStart`/`SubagentStop`.
- Orphan `←` tool lines are valid and expected.

### 2.5 Kimi Code parser

**Files:** `claudine/lib/src/stream/kimi.rs`, `claudine/lib/src/stream/protocol/kimi.rs`

- Migrate to semantic event emissions. Stop silent drops of unknown events.

### 2.6 Qwen Code parser

**Files:** `claudine/lib/src/stream/qwen.rs`, `claudine/lib/src/stream/protocol/qwen.rs`

- Same as Kimi: migrate to semantic events, emit `ProviderExtension` for unknowns.

**Parallelization note:** Steps 2.3–2.6 are independent and can run in parallel once 2.1 and 2.2 establish the pattern.

**Validation checkpoint (end of Phase 2):**
- `cargo test -p claudine` — all parser tests pass.
- Every parser emits `SemanticEvent` for every successfully-parsed line.
- No `_ => Ok(None)` silent-drop arms remain in any parser.
- Round-trip fidelity test passes for every provider's fixtures.

---

## Phase 3 — Consumer Migration (CLI + library consumers)

Depends on: Phase 1 complete. Can begin for already-migrated parsers during Phase 2.

### 3.1 Refactor `progress.rs` to accept `SemanticEvent`

**File:** `claudine/lib/src/stream/progress.rs`

1. Replace `EventMeta`-based methods with `observe_event(&mut self, event: &SemanticEvent, now: Instant)`.
2. Use `SemanticEvent::is_activity()` to update `last_event_at`.
3. Track in-flight tools from `ToolCall`/`ToolResult` events.
4. Track subagent state from `SubagentStart`/`SubagentStop`.
5. Extract `HeartbeatPolicy` struct with named constants: `HEARTBEAT_INTERVAL = 30s`, `SILENCE_WINDOW = 30s`, `FORCE_WINDOW = 120s`.

### 3.2 Extend `reporting.rs` with semantic event serialization

**File:** `claudine/lib/src/stream/reporting.rs`

1. Add `semantic_event_to_event_meta()` function per tech design.
2. Full serialized semantic event goes into `extra["semantic_event"]`.
3. Mark with `extra.synthetic = true`, `extra.synthetic_kind = "stream_semantic_event"`, `extra.semantic_kind = <kind_str>`.
4. `ProviderExtension.payload` preserved untouched.

### 3.3 Replace `LiveStreamSink` with `LiveSemanticSink`

**File:** `claudine/cli/src/commands/wrap/mod.rs`

1. Create `LiveSemanticSink` implementing `SemanticEventSink`.
2. Internally dispatch to:
   - `StreamTextRenderer` for `OutputText` (stdout).
   - `StreamThinkingRenderer` for `Reasoning` (stderr, dimmed).
   - `Status` renderer for tool/subagent/info/warning/error events (stderr).
   - `LiveMetrics::observe_event()` for heartbeat tracking.
   - `AgenticEvent` mapper for hook dispatch (mapping table from tech design §Hook Dispatch Compatibility).
   - Semantic event JSONL writer for stream logging.
3. Implement arrow prefix rendering:
   - `ToolCall` → `StatusState::ToolUse` with `→` prefix.
   - `ToolResult` → `StatusState::ToolUse` with `←` prefix.
   - `SubagentStart` → `StatusState::Subagent` with `→` prefix.
   - `SubagentStop` → `StatusState::Subagent` with `←` prefix.
4. Implement `ProviderExtension` fallback formatter: `{provider}/{kind} · {summary}` with summary extraction order: message → status → name → path → compact JSON.

### 3.4 Simplify `exec.rs`

**File:** `claudine/cli/src/commands/wrap/exec.rs`

1. Remove `StreamChunk` switching logic from the line-read loop.
2. The loop becomes: read line → `parser.feed_line(&line)` → sink handles everything.
3. Feed `StreamThinkingRenderer` from `SemanticEvent::Reasoning` via the sink, not from `StreamChunk::Thinking`.
4. Extract heartbeat timing into `HeartbeatPolicy` (from 3.1).

### 3.5 Migrate composition paths

**Files:** `claudine/cli/src/commands/wrap/composition.rs`, `claudine/cli/src/commands/wrap/sequence.rs`

1. Switch to `LiveSemanticSink` (or a lightweight capture variant).
2. Ensure `run_child_stream_capture()` uses a capture sink that records semantic events for logging but does not render live status.

### 3.6 Delete legacy types

**Files:** `claudine/lib/src/stream/parser.rs`, `claudine/lib/src/stream/mod.rs`, `claudine/cli/src/commands/wrap/mod.rs`

1. Delete `StreamEventSink` trait and all 14 callback methods.
2. Delete `BridgeSink` adapter from Phase 1.5.
3. Delete `StreamChunk` enum.
4. Delete `EventMeta` struct (if fully replaced by semantic events; otherwise keep for summary path).
5. Delete `LiveStreamSink` struct.
6. Delete `NullSink` for old trait; keep `NullSink` for `SemanticEventSink`.
7. Update `create_parser` to no longer reference old trait.
8. Remove `StreamEventSink` from `mod.rs` re-exports.

**Validation checkpoint (end of Phase 3):**
- `cargo test -p claudine` passes.
- `cargo test -p claudine-cli` passes.
- `cargo clippy -p claudine -p claudine-cli` clean.
- No references to `StreamEventSink`, `StreamChunk`, or `LiveStreamSink` remain in the codebase.
- STDERR output for a Claude wrapped session shows `→`/`←` tool lines instead of bare heartbeats.

---

## Phase 4 — Testing & Fixtures

Depends on: Phases 2 and 3 complete.

### 4.1 Round-trip fidelity tests

**File:** `claudine/lib/src/stream/semantic.rs` (or a dedicated test module)

For each provider:
1. Feed captured fixture lines into the parser.
2. Collect emitted `SemanticEvent`s.
3. Serialize each event to `serde_json::Value`.
4. Deserialize back to `SemanticEvent`.
5. Assert the round-trip `Value` is identical.

Especially important for `ProviderExtension` payloads.

### 4.2 Golden STDERR snapshot tests

**Location:** `claudine/lib/tests/` or `claudine/cli/tests/`

Per provider, create a fixture file containing captured JSONL lines that exercise:
- Tool call + tool result
- Reasoning / thinking
- At least one `ProviderExtension` (where applicable)
- Warning or error path

Replay through: parser → `SemanticEventSink` → Status renderer pipeline. Assert exact STDERR transcript against a golden snapshot file.

### 4.3 Summary regression tests

Verify that:
- `StreamExecutionSummary` fields are unchanged by the migration.
- Assistant text is correct in both direct and capture modes.
- Malformed JSON produces `SemanticEvent::Warning` (not `StreamParseError::MalformedLine`).

### 4.4 Reporting fidelity tests

Verify that:
- `semantic_event_to_event_meta()` preserves `ProviderExtension.payload` in `extra["semantic_event"]`.
- SQLite ingest of semantic event rows works via existing `extra_json` column (no schema change).

**Validation checkpoint (end of Phase 4):**
- All new tests pass.
- `just test` from claudine area passes.
- `just lint` from claudine area passes.

---

## Phase 5 — Integration Validation

### 5.1 Manual wrapped-session smoke tests

For each provider with a working API key/session:
1. Run `claudine <provider> --prompt "list files in current dir"`.
2. Verify STDERR shows `→`/`←` tool call lines with tool names.
3. Verify reasoning/thinking renders dimmed on stderr.
4. Verify heartbeat only fires during actual silence windows.
5. Verify final summary is unchanged.

### 5.2 Composition pipeline smoke tests

1. Run `claudine compose <test-file>` and verify STDERR rendering.
2. Run `claudine inline-compose <test-file>` and verify.
3. Run `claudine sequence <test-file>` and verify.

### 5.3 JSONL log verification

1. Run a wrapped session with logging enabled.
2. Inspect JSONL output: confirm semantic events are present with full payloads.
3. Confirm `ProviderExtension` events are logged with raw `payload` intact.
4. Run `claudine logs sync` and verify SQLite ingest succeeds without errors.

---

## Dependency Graph

```
Phase 0 (biscuit-terminal)  ──────────────────────────────┐
                                                           │
Phase 1 (semantic model + traits)                          │
  │                                                        │
  ├── Phase 2.1 (Claude parser)  ─┐                        │
  ├── Phase 2.2 (Codex parser)  ──┤                        │
  │                                ├── Phase 3 (consumers) ┤
  ├── Phase 2.3 (Gemini)  ────────┤   (needs ≥1 parser     │
  ├── Phase 2.4 (OpenCode)  ──────┤    + Phase 0 for       │
  ├── Phase 2.5 (Kimi)  ──────────┤    Subagent rendering) │
  └── Phase 2.6 (Qwen)  ─────────┘                        │
                                                           │
                               Phase 4 (tests/fixtures) ◄──┘
                                       │
                               Phase 5 (integration)
```

**Key parallelization opportunities:**
- Phase 0 runs entirely in parallel with Phases 1–2.
- Phase 2.3–2.6 are independent of each other (after 2.1–2.2 establish the pattern).
- Phase 3.1 and 3.2 (library consumers) can start once Phase 1 is done, before all parsers are migrated.
- Phase 3.3–3.5 (CLI consumers) need at least one parser migrated plus Phase 0 complete (for `StatusState::Subagent`).

**Critical path:** Phase 1 → Phase 2.1 or 2.2 → Phase 3.3 → Phase 3.6 → Phase 4

---

## Risk Notes

| Risk | Mitigation |
|------|-----------|
| Large surface area — 6 parsers + CLI consumers in one feature | Bridge adapter (1.5) allows incremental migration; parsers can land independently |
| `StreamChunk` removal breaks composition capture mode | Phase 3.5 explicitly covers capture paths; regression tests in 4.3 |
| `biscuit-terminal` change blocks CLI rendering | Phase 0 is independent and small; can land first |
| JSONL log volume increase from `OutputText` events | Accepted per spec; batching is a future optimization |
| Provider format drift during implementation | `ProviderExtension` catch-all ensures no data loss; research docs in `claudine/docs/research/non-interactive-sessions/` are the audit reference |
