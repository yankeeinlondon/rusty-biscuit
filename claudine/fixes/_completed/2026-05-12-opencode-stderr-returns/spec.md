---
created: 2026-05-12
provider: OpenCode
severity: regression
related_research:
  - claudine/docs/research/agent-cli/opencode.md
  - claudine/docs/research/agent-logging/opencode.md
related_memory:
  - feedback_claudine_opencode_false_hang
---

# OpenCode: Use the Structured Stderr Log Stream as a First-Class Event Source

## The Problem We Are Solving

`just commit` hangs on OpenCode at a rate that has climbed from less than 5% historically to over 50% recently. The user experience is:

- Some commits succeed visibly, then claudine never moves on
- Or work is clearly mid-flight inside a subagent, then claudine appears stuck
- Ctrl+C exits cleanly with the standard interrupt message — **no watchdog breach, no diagnostic** — because the watchdog cannot fire on a budget the user does not wait out
- Interactive OpenCode sessions never hang; only the non-interactive `compose` path does

The root cause is **not** a runaway watchdog and **not** a faulty timeout. It is that Claudine is starved of activity signal between the events OpenCode actually emits on its NDJSON stream.

## OpenCode's Defining Characteristic

**OpenCode's NDJSON stream emits `tool_use`, `task`, subagent, and step events ONLY when they are DONE — never when they start.** This is fundamentally different from every other supported provider (Claude Code, Codex CLI, Gemini CLI, etc.), all of which emit start/progress events as work begins.

Consequences for any system watching the NDJSON stream alone:

- A 10-minute `cargo test` invoked through the `bash` tool produces **zero** stdout events for the full 10 minutes, then one `tool_use` at the end
- A subagent that runs for 30 minutes produces zero stdout events until the moment it completes, at which point a single `tool_use` for the `task` tool arrives
- Main-loop composition between events produces no stream output at all

There is also no terminal `session.complete` event — completion must be inferred from process exit and the last observed events. Issue [#17221](https://github.com/anomalyco/opencode/issues/17221) and PR [#18249](https://github.com/anomalyco/opencode/pull/18249) document upstream awareness of these gaps.

**This is the single most important fact about OpenCode integration. Every part of Claudine that interprets stream silence as a signal must account for it.** Treating OpenCode silence the same way we treat Claude Code silence produces false-positive hangs (and previously caused false-positive kills).

## The Source We Are Ignoring

OpenCode also emits a structured logger stream on **stderr** when launched with `--print-logs`. This stream is the **richest source of internal lifecycle metadata that OpenCode currently exposes**. Among other things, it surfaces:

- Tool dispatch, permission evaluation, and per-step loop boundaries — including for subagents
- Session creation with `parentID` lineage (the only reliable "subagent started" signal in OpenCode)
- Per-LLM-call provider/model identity, including `mode=primary` vs `mode=subagent`
- Retry-classified provider errors with full vendor payloads (rate limit, quota, auth, etc.)
- HTTP request/response spans for timing observability
- Boot banner, project/CWD detection, plugin and config loading

Claudine already launches OpenCode with `--print-logs --log-level ERROR` ([`profile/opencode.rs:157-159`](../../../claudine/cli/src/commands/wrap/profile/opencode.rs)) and has a stderr log bridge wired in ([`policy.rs:175-186`](../../../claudine/cli/src/commands/wrap/policy.rs)), with parsers and classifiers in [`stream/logs/opencode/`](../../../claudine/lib/src/stream/logs/opencode/). The plumbing is complete; we are starving the upstream tap and dropping classifications at the bridge.

Two reasons that surface degraded over time:

1. **`--log-level ERROR` produces almost nothing on a successful run.** Routine activity (tool dispatch, subagent lifecycle, step boundaries, LLM call identity) only appears at INFO. The level was set conservatively before we understood how dependent we'd become on this stream.
2. **Even when log records arrive, they only refresh the byte heartbeat and the rate-limit/stuck-subagent classifier.** They do not flow into the same `SemanticEventSink` that the NDJSON parser uses, so the watchdog, the renderer, and the summary builder never see them as semantic activity.

The result: when the NDJSON stream goes quiet for legitimate reasons (a slow tool, an active subagent, main-loop composition), Claudine has no second source telling it that work is in fact progressing — even though OpenCode is logging that exact information on stderr a few hundred milliseconds at a time.

## What We Are Building

### Goal

**Make the OpenCode stderr structured log a first-class event source that flows into the same `SemanticEventSink` as the NDJSON stream**, so that:

- The watchdog sees real semantic activity, not just byte heartbeats
- The live renderer can describe what is happening during silent NDJSON windows
- Completion signals from either source correctly mark events as done
- The end-of-run summary is enriched with model identity, subagent lineage, and retry counts that are only available on the stderr stream

### Non-Goals

- **Do not tighten `step_timeout`.** A Rust test suite legitimately takes 10+ minutes with no output. Detecting failure sooner is not the objective. Detecting *progress correctly* is.
- **Do not increase breach rates.** This work should reduce false hangs by giving the watchdog more signal to recognize healthy work, not by making the kill decision more aggressive.
- **Do not drop the NDJSON stream.** Both sources carry information the other does not (NDJSON has tool inputs/outputs and token accounting; stderr has provider/model/mode/lineage). We need both.

### Required Behavior Changes

1. **Bump the log level from `ERROR` to `INFO`.**
   - In [`profile/opencode.rs:158-159`](../../../claudine/cli/src/commands/wrap/profile/opencode.rs), change the `--log-level` argument to `INFO` for the `apply_structured_stream` path.
   - Reasoning: `INFO` is OpenCode's installed-build default. It surfaces `service=session`, `service=session.prompt`, `service=llm`, `service=permission`, `service=tool.registry`, and HTTP spans — the records we need. `DEBUG` adds very little above `INFO` in installed builds (most verbose detail is gated on `Installation.isLocal()`).
   - Filter `service=bus` lines aggressively — they dominate INFO volume and carry no information we use (see *Gotchas* in [`opencode.md`](../../../claudine/docs/research/agent-cli/opencode.md#gotchas)).

2. **Promote classified log records to `SemanticEvent`s emitted through the same sink as the NDJSON parser.**
   - The existing `OpenCodeLogBridge` in [`stream/logs/opencode/reasoning.rs`](../../../claudine/lib/src/stream/logs/opencode/reasoning.rs) already parses every stderr line, classifies many, and produces `EarlyTermination` for rate-limit and fatal classes. Extend it (or add a sibling promoter) so that the records below also emit semantic events.
   - The semantic events must flow through the **same `SemanticEventSink`** the `OpenCodeSemanticStreamParser` uses, so the watchdog's `last_event_at`, the live renderer, and the summary builder all see them. Refreshing only `last_byte_at` is insufficient — we need the full event channel.

3. **Specific log records to promote and the semantic events they should emit.**

    The signal-to-event mapping below uses field names from the [research doc](../../../claudine/docs/research/agent-cli/opencode.md#tags-catalog). All field extraction must use the line format documented there: `LEVEL  YYYY-MM-DDTHH:MM:SS +Nms key=value ... message`.

    | Stderr signal | Semantic event to emit | Notes |
    |---|---|---|
    | `service=default ... version=... opencode` (boot banner) | `SessionStart` (if no NDJSON `sessionID` seen yet) | First line of every run; deterministic stream anchor |
    | `service=session id=ses_<X> ... created` **without** `parentID` | `SessionStart` carrying the primary session id | First match wins for the primary session |
    | `service=session id=ses_<CHILD> ... parentID=ses_<PARENT> ... created` | `SubagentStart` keyed on `ses_<CHILD>` | **This is the real subagent-started signal.** Replaces the current atomic synthesis at task-completion in `providers/opencode.rs`, which provides a zero-width window |
    | `service=llm providerID=<P> modelID=<M> ... mode=primary stream` | `Info { message: "llm_call_start", extra: { providerID, modelID, mode, agent, small } }` | First `small=false mode=primary stream` is authoritative primary provider/model |
    | `service=llm ... mode=subagent stream` | `Info` enriched with subagent attribution; cross-reference to the most recent `parentID` session-created line | Tells us *which* model the subagent is using |
    | `service=llm ... stream error` (with `error=<JSON>`) | Existing rate-limit / API-error classification (already handled by bridge) | Count repeated occurrences to detect retry storms |
    | `service=session.prompt session.id=<X> step=N loop` | `Info { message: "step_loop", extra: { session_id, step } }` | Marks the start of a step; complements NDJSON's late `step_finish` |
    | `service=session.prompt session.id=<X> ... exiting loop` | `Info { message: "exiting_loop", extra: { session_id } }` | Step boundary closure; for child sessions, the parent's next `loop` line confirms subagent completion |
    | `service=session.prompt session.id=<CHILD> ... exiting loop` followed by parent `loop` | `SubagentStop` keyed on `ses_<CHILD>` | **This is the real subagent-completion signal.** Pair with `tool_use` of `tool=task` on stdout (`metadata.sessionId` matches) for full attribution |
    | `service=permission permission=<type> pattern=<arg> action={...} evaluated` | `Info { message: "permission_evaluated", extra: { permission, pattern, action } }` | Captures auto-allow/auto-deny decisions in non-interactive mode |
    | `service=default http.method=... http.url=... http.status=... logSpan.http.span.N=<Nms> Sent HTTP response` | `Info { message: "http_response", extra: { method, url, status, duration_ms } }` | Timing observability; one per request |
    | `service=server error=<ClassName> cause=<ClassName>: <msg>\n at ... failed` | `Error { kind: Configuration | Unknown, ... }` | Fatal server-side error with stack frames |

   The exit code is `0` even on fatal-looking errors (see [Gotchas](../../../claudine/docs/research/agent-cli/opencode.md#gotchas)), so these classifications are what allow Claudine to report a non-zero outcome.

4. **Update the watchdog state machine to consume the new events.**
   - `LiveMetricsState.last_event_at` must refresh on every promoted log event, just as it does on NDJSON events today.
   - `LiveMetricsState.in_flight_subagents` must be populated by the **new** `SubagentStart` emitted from the `parentID` session-created log line and cleared by the **new** `SubagentStop` emitted from the child's `exiting loop` line. This replaces the zero-width atomic synthesis in `providers/opencode.rs::handle_tool_use_completed`, which the source itself documents as "not a useful observation window for synthesized subagents."
   - `step_in_flight` may remain driven by NDJSON `step_start`/`step_finish`; the stderr `session.prompt step=N loop` / `exiting loop` records are complementary and refresh activity but should not replace the NDJSON-driven flag (one is the parent session's overall step, the other is per-prompt loop).

5. **Deduplicate events that arrive on both streams.**
   - When a log record and an NDJSON event describe the same lifecycle moment (e.g., a child session completion observed both via stderr `exiting loop` and via the parent's eventual `tool_use` of `tool=task`), the sink must reconcile by stable id (`sessionID` for sessions, `metadata.sessionId` on the `task` tool for subagent attribution).
   - First arrival wins for ordering; the second arrival enriches the existing record rather than emitting a duplicate.

6. **Surface enrichment in the end-of-run summary.**
   - `StreamExecutionSummary` should record the primary provider/model from the first `service=llm mode=primary small=false stream` line (the NDJSON stream cannot tell us this).
   - The subagent roster in stuck-subagent diagnostics should be populated from real `SubagentStart`/`SubagentStop` pairs (per item 4), not from atomic synthesis at task completion.

### What Must Not Change

- `step_timeout` defaults and the user-configured `step_timeout: 8m` in `prompts/commit.md` — **leave them alone**.
- The two-rule watchdog architecture (`timeout` wall-clock + `step_timeout` silence) and its CLI/frontmatter/env precedence.
- The behavior of `OpenCodeLogBridge` for rate-limit and fatal classifications — those already work; we are extending the bridge, not replacing it.
- The byte-heartbeat refresh on every non-empty stderr line — keep it as a backstop. The semantic event channel is additive.

## Gotchas Worth Repeating Up Front

These come from the research doc but matter so much they belong in this spec too:

- **Exit code is 0 even on fatal errors** (e.g. `ProviderModelNotFoundError`). Never use exit code as a success signal. Inspect the stream(s).
- **`service=tool.registry` is registration, not invocation.** The `duration` on those lines is the time to *register* the tool definition at session bootstrap, typically `0` or `1` ms. Actual tool calls produce no `tool.registry` line. The stderr signals adjacent to a tool call are `service=session.prompt ... step=N loop` and `service=permission ... evaluated`. The tool's input/output payload lives on stdout NDJSON.
- **`error=` payloads can be tens of kilobytes per line.** AI SDK `AI_APICallError` objects include `requestBodyValues` with the full system prompt and message history. Allow very long single lines and budget memory accordingly.
- **Timestamps have second precision only.** Sub-second ordering must use the `+Nms` delta.
- **First-run DB migration prints non-log lines** (`sqlite-migration:N\n` or a TTY progress bar) before any structured log appears. Treat as a one-shot pre-amble.
- **Bus chatter dominates INFO output.** In a non-trivial session, ~70-75% of INFO lines are `service=bus type=message.part.delta publishing` or similar. Filter `service=bus` aggressively unless debugging the bus itself.
- **The `--print-logs` stream and the file-based log under OpenCode's data dir share a format but are not identical in scope.** This spec targets the stderr stream from `--print-logs`. The file-based log ([research](../../../claudine/docs/research/agent-logging/opencode.md)) is documented separately and is what OpenCode writes when `--print-logs` is absent. Treat the file log as a fallback diagnostic source, not as the live signal channel — Claudine reads stderr.

## Files Most Likely to Change

- `claudine/cli/src/commands/wrap/profile/opencode.rs` — `--log-level INFO`, optional `service=bus` filter prefix
- `claudine/lib/src/stream/logs/opencode/reasoning.rs` — extend `OpenCodeLogBridge` (or add a `OpenCodeLogPromoter`) to emit `SemanticEvent`s into the sink, alongside the existing classification work
- `claudine/lib/src/stream/logs/opencode/events.rs` — add tag/service classifications for the records listed in §3 (some already exist; many will be new)
- `claudine/lib/src/stream/providers/opencode.rs` — remove or repurpose the atomic `SubagentStart`/`SubagentStop` synthesis at `tool_use=task` completion; rely on the stderr-driven lifecycle. Keep the `task_subagent_id` extraction from `metadata.sessionId` for cross-stream attribution.
- `claudine/lib/src/stream/progress.rs` — confirm `LiveMetricsState` updates fire for the promoted events (existing `observe_event` should already handle this once the events flow)
- `claudine/cli/src/commands/wrap/exec/spawn.rs` — verify the stderr reader thread continues to refresh `last_byte_at` (byte heartbeat stays as a backstop). No semantic changes needed here; the bridge is what changes.

## Required Documentation Updates (Do Not Skip)

The user has been explicit that the importance of the stderr log stream must never be forgotten again. Every documentation surface listed below must clearly state both:

1. **OpenCode is the only supported provider whose NDJSON stream emits tool / task / subagent / step events ONLY when they finish, never when they start.** This makes the stderr structured logger the **primary live activity signal** for OpenCode.
2. **Activity inferred from the stderr log stream must flow into the same `SemanticEventSink` as the NDJSON parser**, not just into the byte-heartbeat. Anything that treats stderr as advisory will recreate the false-hang regression.

The minimum set of documents that must call this out:

- [`.claude/skills/claudine/SKILL.md`](../../../.claude/skills/claudine/SKILL.md) — add a short OpenCode-specific paragraph in the architecture/stream-parsing section, or link to a new sibling doc in the skill that owns this contract.
- A new file under `.claude/skills/claudine/` (suggested: `opencode-event-sources.md`) capturing:
  - The "DONE-only NDJSON" rule and why it is unique to OpenCode
  - The dual-source contract (stderr `--print-logs INFO` + stdout NDJSON, both feeding `SemanticEventSink`)
  - A pointer to the two research docs that justify the design
  - A pointer to this fix and to the related memory `feedback_claudine_opencode_false_hang`
- [`claudine/docs/topics/timeouts.md`](../../../claudine/docs/topics/timeouts.md) — add a callout that OpenCode's `step_timeout` is intended to fire only when **both** the stderr log stream and the NDJSON stream are silent, not when one is silent and the other is active.
- [`claudine/docs/research/agent-cli/opencode.md`](../../../claudine/docs/research/agent-cli/opencode.md) — already canonical for the schemas; ensure the "DONE-only emission" rule is called out near the top of the document (it is currently buried under "Implementation details from the current source"). Add cross-link to this spec.
- The next time `non-interactive-sessions/opencode.md` is touched, the dual-source contract should be the headline behavior, not a footnote.

## Acceptance Criteria

A run of `just commit` against OpenCode + a moderately busy commit:

- Shows continuous activity reporting in the live renderer during subagent work (driven by stderr `session.prompt`, `permission`, and `session ... parentID` records), instead of going opaque for minutes at a time
- Records the primary provider/model in the end-of-run summary (drawn from `service=llm ... mode=primary`)
- Lists active subagents by real start time during a stall, not by an atomically-synthesized zero-width window
- Does not fire `step_timeout` more aggressively than today on legitimate work (long tool calls and subagents are still allowed their full budget)
- Surfaces a structured error (with provider/model attribution and a retry count) when a provider returns repeated `stream error` lines, even though the process exit code is `0`

A regression test must demonstrate that with `--log-level INFO` enabled, a captured stderr fixture containing a parent + subagent session emits the expected `SubagentStart`/`SubagentStop` pair through the semantic sink, and that `LiveMetricsState` reflects the in-flight window for the full duration of the child session.

## References

- Research: [`claudine/docs/research/agent-cli/opencode.md`](../../../claudine/docs/research/agent-cli/opencode.md) — authoritative source for log line format, tag catalog, services, and the dual-source contract. **Required reading.**
- Research: [`claudine/docs/research/agent-logging/opencode.md`](../../../claudine/docs/research/agent-logging/opencode.md) — covers the **file-based** log (not the `--print-logs` stderr stream). Useful as a similar-but-distinct reference; do not conflate the two formats.
- Memory: `feedback_claudine_opencode_false_hang` — prior context that the watchdog must not assume silence is failure.
- Related fix (do not break): [`claudine/fixes/_completed/2026-05-10-opencode-timeout-regression`](../../../claudine/fixes/_completed/) — the byte-heartbeat and per-step grace landed here; this spec extends rather than replaces them.
