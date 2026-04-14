# Feature More Meta Response

We went through a good amount of effort researching the schemas that each of the AI Agents we support provides when we run a non-interactive session. This was then used to create a more strongly typed solution with the hope that by extracting more of the metadata from this stream we'd be able to provide a richer experience to callers of non-interactive sessions in Claudine.

Unfortunately it would appear we did NOT do this well enough and the result is we're providing very poor responses to the user. Here's an example from the Codex CLI:

```sh
  - Codex session ID 019d8abb-598

   30s
   60s
   90s
   120s
   150s
   180s
```

I asked an Agent about this and they pointed out the following:

```md
⏺ Looking at the Codex stream parser and the research doc, there are concrete gaps — the disappointing output isn't a limit of
Codex's stream, it's us leaving metadata on the floor:

What Codex emits that we're dropping today:

1. item.updated events — not handled anywhere. The parser's CodexEvent enum (claudine/lib/src/stream/protocol/codex.rs:20) has
   no variant for item.updated, and feed_line in claudine/lib/src/stream/codex.rs:298 falls through to the silent skip arm.
   These carry in-progress status and partial output for long-running commands — perfect heartbeat fodder.
2. file_change items — missing from CodexItem enum (line 171). Falls into Unknown. Every file write during the run is
   invisible.
3. plan_update items — also missing. Codex's plan-tracking is one of its richer signals and we discard it.
4. Reasoning text — CodexReasoning is parsed (line 370) but handle_item_completed doesn't route it through
   StreamChunk::Thinking the way Claude/OpenCode do. The StreamThinkingRenderer wiring already exists in exec.rs:921 — Codex just
   never feeds it.
5. command_execution exit_code/status — CodexToolItemFields (line 381) doesn't capture status or exit_code, which the research
   doc explicitly lists for command_execution.

The visible symptom in your screenshot:
During long reasoning phases, Codex emits no stream events at all, so LiveMetrics stays empty and describe_heartbeat has
nothing beyond elapsed to render. Wiring up item.updated + reasoning-as-thinking would replace the bare 30s / 60s / 90s ladder
with actual progress.

Want me to draft the protocol extensions (add ItemUpdated, FileChange, PlanUpdate, status/exit_code fields) and route
reasoning through the thinking renderer?
```

## Not Just Codex

Too often an example surfaces where one of the Agent providers we support illustrates a problem and we then tackle the problem FOR THAT PROVIDER without adequately considering if the problem is cross-provider. This can lead to some fairly un-DRY code and a never ending debug/fix cycle.

So in the example above, we are talking about what we experienced with Codex CLI but this problem almost surely exists in all of the Agent providers: Claude Code, Gemini CLI, Qwen CLI, Roo Code, Goose, Kimi Code, and OpenCode.

## Goal

The user should receive a rich set of information from _thinking_, metadata, down to the final output. Tool calls in particular are very important to extract metadata from but so too are events like a subscription being "capped" or an API token not being funded, etc.

## Task

- There is detailed information for each provider's streaming outputs in claudine/docs/research/non-interactive-sessions
- We also need to review the current implementation around streaming responses, parsing, and reporting of the extracted metadata
- From this understanding we need to not only tackle the issues described above for Codex but a similar set of issues for the other providers.
    - we should always try to solve the problems in a cross-provider manner where this is feasible
- The `Status` struct from `biscuit-terminal` should be used with it's "circular" style for reporting to STDERR the metadata we receive.
    - some of this will have an INFO state
    - tool calls have their own state
    - it might make sense to have a state added for "subagents"
    - when a tool call or subagent is created we need to indicate to the user by use of arrows whether what they're seeing is the tool call (→) versus tool response (←), subagent start (→) versus subagent stop (←)

## Design

The design below captures decisions made during a human-in-the-loop review. It is deliberately scoped: motivation lives in the sections above, and anything not called out here is either out of scope (see the end of this section) or left to implementation judgment.

### Event Model

Introduce a typed `SemanticEvent` enum alongside the existing `StreamChunk`. Each provider's protocol parser is responsible for translating its JSONL stream into a sequence of `SemanticEvent`s. Unknown-but-parseable events are preserved via a `ProviderExtension` escape hatch rather than being dropped.

Fidelity invariants — these are load-bearing and should be enforced by tests, not just convention:

- **Raw-payload invariant.** `ProviderExtension { provider, kind, payload: serde_json::Value }` stores the provider event body verbatim — no intermediate narrowing struct.
- **No-drop invariant.** Parsers must emit *something* for every successfully-parsed JSONL line. Unknown `type` values become `ProviderExtension`; only malformed JSON is dropped, and it is surfaced via `SemanticEvent::Warning`.
- **Typed variants stay extensible.** Every typed `SemanticEvent` variant carries an `extra: serde_json::Value` so adding a new cross-provider variant never drops provider-specific fields.
- **Reporting fidelity.** The JSONL writer serializes the full event (including `ProviderExtension.payload`). SQLite ingest routes unknown kinds into the existing `extra_json` column — no schema change is required for fidelity; schema work would only be motivated by queryability. Enforce via a round-trip test: parse → serialize → parse yields identical `Value`.
- **STDERR fallback.** `progress.rs` gains a default formatter for `ProviderExtension` that prints at minimum `{provider}/{kind}` plus a best-effort one-line summary of the payload.
- **Graduation path.** When ≥2 providers emit a given `kind`, it graduates from `ProviderExtension` to a typed `SemanticEvent` variant. This is additive and non-breaking because the raw payload was never the public contract.

Initial typed `SemanticEvent` variants (cross-provider starting set, based on the research docs at `claudine/docs/research/non-interactive-sessions/`):

- `ToolCall`, `ToolResult` (with `status`, `exit_code` where available)
- `SubagentStart`, `SubagentStop`
- `Reasoning` (routed through the existing `StreamThinkingRenderer`)
- `FileChange`
- `PlanUpdate`
- `Info { level, message }`, `Warning { .. }`, `Error { .. }` (covers "subscription capped", "API token not funded", etc.)
- `ProviderExtension { provider, kind, payload }` (catch-all)

This list is a starting set, not a final contract. It should be refined during implementation as each provider's stream is audited against the research docs.

### STDERR Output Policy

- Rich `Status` (circular style) output is **the default** on STDERR for all wrapped-provider subcommands and composition pipelines (`compose`, `inline-compose`, `sequence`).
- The existing elapsed-time heartbeat (`30s / 60s / 90s / …`) is **retained only as a silence fallback**: it fires only when no semantic event has been emitted for ≥ N seconds. The current `quiet_window` / `force_window` values in `exec.rs` are the starting point for N; the final value is TBD during implementation.
- `Verbosity::Silent` still suppresses all STDERR output.
- No new verbosity flag is introduced for this feature.

### Status UI

- Add a new `StatusState::Subagent` variant to `biscuit-terminal` (coordinated upstream change), with icons defined for all three existing themes (Circular, Rounded, Timeline).
- Arrow semantics:
  - Tool call start → `→`; tool response → `←`.
  - Subagent start → `→`; subagent stop → `←`.
- **Arrows reflect exactly what the provider emits. No synthetic events.** If a provider only emits completion events, only the `←` line is rendered. This is intentional: it honestly reflects provider capability, and a missing `→` line is itself a meaningful signal to the user ("this provider only exposes completions").

### Tool Call / Result Rendering

- `SemanticEvent::ToolCall` emits a `→` STDERR Status line; `SemanticEvent::ToolResult` emits a separate `←` STDERR Status line.
- The renderer performs **no in-place Status update** and requires **no `id` correlation** between `ToolCall` and `ToolResult`. Each event is rendered independently as its own line in the STDERR stream.
- If a provider only emits completion events, an orphan `←` line is rendered. This is consistent with the honest-to-provider rule in the Status UI section — the absence of a `→` line is itself a meaningful signal.
- Provider-supplied correlation identifiers (e.g. Anthropic `tool_use_id`, Codex `call_id`, or analogous fields) **should still be preserved inside the event's `extra: serde_json::Value` payload** when available, for JSONL fidelity and downstream reporting. They are simply not required by the STDERR renderer.

### Error / Warning Classification

- Each provider parser maintains an **explicit, code-level allowlist** that maps raw event `type` / `kind` strings to typed `SemanticEvent::Error` or `SemanticEvent::Warning` variants.
- Events whose raw type is **not** on the allowlist fall through to `SemanticEvent::ProviderExtension`. This preserves the no-drop invariant and keeps classification conservative.
- Allowlists are derived by auditing each provider's stream against `claudine/docs/research/non-interactive-sessions/` during implementation.
- Allowlist changes are **additive**: they move an event from `ProviderExtension` into a typed variant, never the reverse. This matches the graduation path already described in the Event Model section.
- **No heuristic or payload-field sniffing** (`severity`, `level`, `status`, etc.) drives classification. Only the provider's event type/kind string is consulted. Payload fields may still be surfaced inside the typed variant, but they do not decide which variant is emitted.

### Sink Trait

The existing `StreamEventSink` trait in `claudine/lib/src/stream/parser.rs` is **replaced** by a minimal trait exposing a single method:

```rust
pub trait SemanticEventSink {
    fn on_semantic_event(&mut self, event: SemanticEvent);
}
```

- All current consumers — `wrap/mod.rs`, composition pipelines (`compose`, `inline-compose`, `sequence`), the progress / Status renderer, and the JSONL writer — are migrated to the new trait **in the same feature PR**.
- The legacy per-event hooks (`on_before_tool`, `on_after_tool`, `on_subagent_start`, `on_subagent_stop`, `on_permission_request`, `on_warning`, and any `Text` / `Thinking` chunk hooks that overlap with `SemanticEvent` variants) are **deleted** once migration is complete. No deprecation window.
- `StreamChunk` (`Text`, `Thinking`) resolves one of two ways, left to implementation judgment:
  - (a) becomes a variant (or variants) of `SemanticEvent`, or
  - (b) continues to flow through a distinct, narrow channel dedicated to raw token streaming.
- In either case, exactly **one semantic event model** is surfaced to consumers. There is no parallel "old hooks + new events" API.

### Acceptance Criteria

- [ ] `SemanticEvent` enum exists with the initial variants listed above, and `ProviderExtension { provider, kind, payload: serde_json::Value }` preserves unknown events verbatim.
- [ ] Every typed `SemanticEvent` variant carries an `extra: serde_json::Value` field.
- [ ] Each provider parser emits a `SemanticEvent` (typed or `ProviderExtension`) for every successfully-parsed JSONL line; only malformed JSON is dropped, and it is surfaced via `SemanticEvent::Warning`.
- [ ] Round-trip fidelity test (parse → serialize → parse yields identical `Value`) passes for every captured fixture.
- [ ] JSONL reporter serializes full events including `ProviderExtension.payload`.
- [ ] SQLite ingest routes unknown `kind`s into the existing `extra_json` column without schema changes.
- [ ] `progress.rs` has a default `ProviderExtension` formatter that renders at minimum `{provider}/{kind}` plus a one-line payload summary.
- [ ] `biscuit-terminal` exposes `StatusState::Subagent` with icons for Circular, Rounded, and Timeline themes.
- [ ] STDERR rendering uses `→` for tool-call / subagent start and `←` for tool-result / subagent stop, with no synthesized start/stop events.
- [ ] Elapsed-time heartbeat only emits when no semantic event has been observed within the configured silence window.
- [ ] Rich STDERR is the default for wrapped-provider subcommands and composition pipelines (`compose`, `inline-compose`, `sequence`); `Verbosity::Silent` suppresses it.
- [ ] Golden-fixture STDERR snapshots exist per provider. Fixtures are replayed through the parser → `SemanticEvent` → STDERR renderer pipeline, and expected STDERR output is snapshotted and asserted.
- [ ] Each provider's fixture coverage exercises: tool call, tool response, reasoning/thinking, at least one provider-extension event (where applicable), and error/warning paths.
- [ ] Protocol parser tests follow the existing `#[cfg(test)] mod tests` pattern in `stream/protocol/*.rs`.
- [ ] `SemanticEvent::ToolCall` and `SemanticEvent::ToolResult` each render as their own STDERR Status line; no in-place update, no ID correlation required by the renderer. Provider-supplied tool IDs are preserved in `extra` when available.
- [ ] Each provider parser has an explicit allowlist (in code) mapping raw event types to `SemanticEvent::Error` / `SemanticEvent::Warning`. Events not on the allowlist become `ProviderExtension`. No heuristic sniffing of payload fields drives classification.
- [ ] `StreamEventSink` is replaced by a trait exposing a single `on_semantic_event(&mut self, event: SemanticEvent)` method. All consumers (`wrap/mod.rs`, composition pipelines, progress renderer, JSONL writer) are migrated; legacy per-event hooks are deleted.

### Out of Scope

- New verbosity flags beyond `Verbosity::Silent` (no `--quiet`, no `--progress=compact` in this feature).
- Provider-extension graduation decisions — handled case-by-case later as cross-provider patterns emerge.
- Synthetic lifecycle events for providers that only emit completions.
- Schema changes to SQLite reporting tables — the existing `extra_json` column absorbs new kinds. Queryability improvements are a separate feature.
- Preserving the existing `StreamEventSink` trait or its per-event hooks. This feature replaces them.
- Heuristic / payload-field-based classification of provider diagnostics. Only per-provider allowlists are in scope.
- In-place Status updates or spinner-style single-line tool rendering.
