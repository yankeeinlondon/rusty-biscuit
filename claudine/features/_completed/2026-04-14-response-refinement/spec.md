# Response Refinement

We have spent a lot of time improving the metadata we're able to get back when running non-interactive prompts but there are still a combination of obvious errors, inconsistent formatting, and just general improvement in formatting. This feature will attempt to address these.

## Four Agents, Same Prompt

![visual overview](four-agents-one-prompt.png)

I asked OpenCode, Gemini CLI, Claude Code, and Codex CLI the same question as a non-interactive session. The question was "when is the NFL draft this year?" This question was asked because the NFL draft date and information around is relatively recently announced and would NOT be in any of the model's training corpus so they would all need to make at least one tool call to get this information.

What we got was problematic for all of them but mainly in different ways. Overall the quality is VERY poor and I don't understand how we're still doing so badly considering how much effort has been made to get this right!

### OpenCode CLI

Opencode had the worst time, it:

- announced a tool call
- then reported the sessions metadata _without_ ever reporting any information to STDOUT
- even more embarrassing, the metadata claimed there were 0 tool calls (even the line above was a tool call)!
- also of note, we're still reporting JSON to the terminal:
    - instead of `⚙ firecrawl_firecrawl_search  {"query":"NFL draft 2026 date","limit":5,"sources":[{"type":"web"}]}`
    - the icon is NOT a tool call icon but this is a tool call
    - even though OpenCode only reports on _returned_ tool calls we had agreed we'd show both directions when we got the tool call's "end" event
    - we'd expect something more like:
        - `🔧 → Firecrawl Search · <dim><i>NFL draft 2026 date</i></dim>`
        - `🔧 ← Firecrawl Search · <dim><i>successful</i></dim>`
- In addition, we see there are three blank lines before we hit the tool call
    - this is pretty hard to debug until the other issues are worked out but it's likely there is a problem here too as all Agent's had issues with proper vertical spacing
    - the goal for vertical spacing is to allow 1 blank line between "sections/groups" in the content but never more

#### Output:

```sh



⚙ firecrawl_firecrawl_search {"query":"NFL draft 2026 date","limit":5,"sources":[{"type":"web"}]}
✓ 15s · no tool calls
```

### Claude Code
- duplicative SessionStart and SessionEnd events (two each)
    - these hook events should ideally trail the session ID marker (this is how it's always been and still is for all other Agents); it's possible that we forgive the sequencing if making it normalized presents enough problems.
- invalid `rate limit` warning:
    - It would seem that EVERY response returns a rate limit and then, ironically, proceeds to fulfill the request because, in fact, there IS NOT A RATE LIMIT.
    - What we need to do is check if there is an ANTHROPIC_API_KEY defined in the environment
        - If there is this indicates that the user is using API pricing
        - If there is not then the user is using a Subscription
        - In all of my tests i'm using a Subscription
- now in this case, it looks like it didn't bother to do a web search and left the info for 2026 a bit vague. That's a shame because it would have been good to see what that looked like
- on a positive note, the STDOUT content provided has been correctly treated and formatted as Markdown; this means that:
    - word wrap is applied and there is not premature and awkward truncation or wrapping far too early (as we see elsewhere)
    - it also means that things like hyperlinks, tables, headings, etc. would all be displayed nicely if they were part of the response (they were not)
- like OpenCode, Claude Code also has vertical spacing problems:
    - three blank lines between the final output and the 

#### Schema Notes

- HookResponse
    - type/subtype: "system/hook_response"
    - `outcome`: "success"
    - `exit_code`: 0
- init
    - `type`/`subtype`: "system/init"
    - `tools`: a list of tools including MCP tools
    - `mcp_servers`: all known MCP servers, their `status` (disabled|needs-auth|?)
    - `model`: (e.g, claude-sonnet-4-6)
    - `permissionMode` ("bypassPermissions"|?)
    - `slash_commands`: a list of available slash commands (built-in and user defined)
    - `claude_code_version`: semver (e.g., 2.1.108)
    - `agents`, `skills`
    - `plugins`
    - `memory_paths`: dictionary with "auto" key, value is a filepath
    - `fast_mode_state`: "off" | "on"
- `type`: "assistant"
    - this is where the "Credit balance is too low" message comes from
    - `error` is "billing_error"
- `type`/`subtype`: "result/success"
    - also reports "Credit balance is too low" in `result` prop and `is_error` is true

#### Output:

```sh
 Claude system event: hook_started (SessionStart:startup)
 Claude system event: hook_started (SessionStart:startup)
 Claude system event: hook_response (SessionStart:startup)
 Claude system event: hook_response (SessionStart:startup)
- Claude session ID 4df29a7b-df3 · claude-opus-4-6[1m]


󰀨 rate limit
The 2025 NFL Draft was April 24–26, 2025. For 2026, the NFL Draft is expected in late April (dates haven't been officially
announced yet as of my knowledge cutoff). Check nfl.com for the confirmed 2026 schedule.



✓ 3.9s · 3 input tokens · 64 output tokens · $0.22 cost basis · no tool calls
```

#### Two More Attempts

Because we didn't get a tool call the first time, I changed the query to "when exactly is the NFL draft this year (2026)?" I ran this the first time in normal mode, the second time in YOLO mode.

##### Normal Mode

```sh
 Claude system event: hook_started (SessionStart:startup)
 Claude system event: hook_started (SessionStart:startup)
 Claude system event: hook_response (SessionStart:startup)
 Claude system event: hook_response (SessionStart:startup)
- Claude session ID e7bbd911-dff · claude-opus-4-6[1m]

󰀨 rate limit
 ← (tool) · success

 ← (tool) · error
Let me look that up for you. I wasn't able to run a web search (permission not granted). Based on my knowledge:

The 2026 NFL Draft is scheduled for Pittsburgh, Pennsylvania, expected to take place over three days in late April 2026 (likely
around April 23–25). However, I can't confirm the exact dates without a live search — the NFL sometimes shifts dates slightly.

For confirmed dates, check nfl.com/draft.



✓ 20s · 7 input tokens · 613 output tokens · 70K cached tokens · $0.28 cost basis · no tool calls
```

##### YOLO Mode

```sh
 Claude system event: hook_started (SessionStart:startup)
 Claude system event: hook_started (SessionStart:startup)
 Claude system event: hook_response (SessionStart:startup)
 Claude system event: hook_response (SessionStart:startup)
- Claude session ID b613526d-0f6 · claude-opus-4-6[1m]

󰀨 rate limit
The 2026 NFL Draft is scheduled for April 23–25, 2026 in Pittsburgh, Pennsylvania (at Acrisure Stadium and surrounding areas).
That's about 9 days from now.



✓ 5.2s · 3 input tokens · 154 output tokens · 12K cached tokens · $0.15 cost basis · no tool calls
```

These two additional runs are interesting and show that:

- In normal mode it made a tool call but how many and anything about them is impossible to discern because:
    - There are two tool calls but both are `EndToolCall` not `StartToolCall`; this is dubious
    - I suspect it tried and the first tool call should have been the request but it was denied permission and that explains the return which is in a failed state
    - Neither tool call has any metadata information making it very hard to grok
    - In the end it "guesses the date" based on training data
    - Note that the metadata says "0 tool calls" which is WRONG! I suspect there was 1 tool call but that tool call was rejected.
- In YOLO mode we see no tool calls but it has precise timing suggesting that it must have done a web search but somehow we didn't log it.


### Codex CLI

- the right Tool icon (from `Status` struct) is used for tool calls
- there is ZERO metadata for the tool calls including whether the tool call was successful or not!
- there should have been a blank line between the tool calls the beginning of the final output
- there are 4 blank lines between the final output and the trailing metadata.

#### Output:

```sh
- Codex session ID 019d8d3e-73b

 → (tool)
 ← (tool)
The 2026 NFL Draft is April 23-25, 2026.

It will be held in Pittsburgh, Pennsylvania, at Point State Park and Acrisure Stadium.

Sources: NFL Football Operations draft page, NFL important dates




✓ 15s · 39K input tokens · 381 output tokens · 3K cached tokens · 1 tool call
```


### Gemini CLI

- the right Tool icon (from `Status` struct) is used for tool calls
- The tool calls look pretty good, it would be nice to adjust the formatting a little to make it prettier and more consistent:
    - `Google Web Search · <dim><i>NFL draft 2026 dates</i></dim>`
    - `Google Web Search · <dim><i>successful</i></dim>`
- There _should_ be a blank line between the tool calls and the final output to STDOUT.
- The first line of content looks like it is being rendered as Markdown and the H3 heading is _definitely_ being rendered as markdown.
    - Strangely though the unordered list that follows is truncated WAY too soon and makes the information almost unreadable
    - There is also blank lines between each line item in the unordered list

#### Output:

```sh
- Gemini session ID bfcd5a8f-52c · auto-gemini-3

 → google_web_search · NFL draft 2026 dates
 ← google_web_search · success
The 2026 NFL Draft is scheduled to take place from Thursday, April 23, to Saturday, April 25, 2026, in Pittsburgh, Pennsylvania.

████ Draft Schedule

- Thursday, April 23: Round 1 (

8:00 p.m. ET)

- Friday, April 24: Rounds 2

–3 (7:00 p.m. ET)

- Saturday, April 25:

Rounds 4–7 (12:00 p.m. ET)



✓ 11s · 57K input tokens · 357 output tokens · 23K cached tokens · 1 tool call
```

## Yolo Mode for OpenCode

I was told early on that YOLO mode does not exist for OpenCode CLI but that is true _only_ in an interactive mode! In non-interactive sessions it DOES allow for the `--dangerously-skip-permissions` flag!

That means in Claudine when we add the `--yolo` flag for an non-interactive session we should set it to YOLO mode and we should not display the message: `- Warning: --yolo is not supported for 'opencode' and was ignored` as we do today!

It is ok to include a warning like this when running opencode in an interactive mode:

- `- Warning: --yolo mode is not supported in OpenCode <i>interactive</i> sessions and was ignored`

---

## Scope and Decomposition

The observations above span several independent fixes plus one cross-cutting concern (tool-call rendering). Treat this feature as an **epic** and decompose it into the following children. The contract-first child must ship first because later children populate its struct rather than formatting strings directly.

### Epic Children

1. **Tool-Call Display Contract** (must ship first) — protocol-level `ToolCallDisplay` type + single formatter in `claudine/cli/src/commands/wrap/live_semantic_sink.rs` + humanization rules + width rules.
2. **Claude: hook/session ordering + rate-limit heuristic** — move hook events to trail the session-ID marker (with streaming-preservation constraint) and gate the rate-limit warning on `ANTHROPIC_API_KEY`.
3. **Section model and spacing normalization** — define the 9-section rendered output model and enforce at-most-one blank line between any two adjacent present sections. Must land **last**.
4. **Gemini markdown rendering** — investigation-led fix for mid-list truncation and stray blank lines between list items.
5. **OpenCode: assistant-text to stdout + YOLO + tool-call accounting + mis-routed render** — restore the missing final assistant response to stdout (P0 — OpenCode native works fine without claudine), accept `--dangerously-skip-permissions` for non-interactive, remove synthesized outgoing tool-call, count responses only, and investigate the current `⚙ firecrawl_firecrawl_search {…raw JSON…}` mis-routed render.

### Child Sequencing

```text
Child 1 → (Child 2, Child 4, Child 5 in parallel) → Child 3 last
```

Rationale: Child 3 (section model + spacing) normalizes output that Children 2, 4, and 5 alter. Running it last avoids rework as the upstream children change what each section emits.

## Tool-Call Display Contract

This is Child 1. Everything downstream consumes it.

### Protocol Type (sketch)

```rust
ToolCallDisplay {
    direction: Direction,    // Outgoing | Incoming
    display_name: String,    // humanized
    summary: Option<String>, // extracted context
    status: Option<Status>,  // success | error | pending
}
```

Subsequent per-provider children populate this struct. They do NOT format strings directly; a single formatter in `claudine/cli/src/commands/wrap/live_semantic_sink.rs` owns the rendered output.

### Canonical Rendered Format

- Outgoing: `🔧 → <DisplayName> · <dim><i><summary></i></dim>`
- Incoming: `🔧 ← <DisplayName> · <dim><i><status-or-summary></i></dim>`

### Status vs. Summary Priority (Incoming)

For incoming tool events, **status always wins when present**. The dim-italic slot renders the status word (`successful`, `error`) whenever status is populated; only if status is absent does the formatter fall back to an output summary extracted from the response payload.

### Status Styling

- **Success:** the status word is rendered as dim italic (matches the rest of the slot).
- **Error:** the status word `error` is rendered **red + bold** inside the dim-italic slot. The glyph and overall line format are unchanged.

### Badge / Glyph Ownership

The formatter does **not** write glyphs literally. It populates `biscuit_terminal::Status` using the existing `Status::ToolUse` state, which already renders the 🔧 glyph. The formatter's job is to construct a `Status` value with:

- the correct state (`Status::ToolUse`),
- a humanized description string (direction arrow + display name + ` · ` + summary or status slot).

No new `Status` states are introduced by this feature.

### Humanization (Two-Tier Resolution)

**Tier 1 — lookup table for known tools.** Seed entries at minimum:

- `firecrawl_*` → "Firecrawl <rest>" (e.g., `firecrawl_firecrawl_search` → "Firecrawl Search")
- `google_web_search` → "Google Web Search"
- Common Claude built-ins as-is: `Bash`, `Edit`, `Read`, `Write`, `Glob`, `Grep`, `WebFetch`, `WebSearch`, `Task`
- MCP servers: `mcp__<server>__<tool>` → humanized "<Server> <Tool>"

**Tier 2 — algorithmic fallback.** Strip provider-redundant prefix if any, split on `_`, apply Title Case.

**Last resort:** the raw tool id is the final display.

### Long-Name and Long-Summary Handling

Long display names and long summary slots are **word-wrapped**, not truncated. The formatter leverages biscuit-terminal's built-in wrapping via the `Layout` struct already used by `Status`, `Prose`, and `UnorderedList`. When content exceeds available width, it flows to additional lines; it does NOT get cut at a boundary.

### Width Rule

- **TTY:** `biscuit_terminal::Terminal` provides the available width; biscuit-terminal components wrap accordingly via `Layout`.
- **Non-TTY / piped:** `Terminal`'s optimistic defaults apply; the same `Layout` wrapping applies.

### Per-Tool-Type Context Extraction ("no JSON to the terminal")

Goal: never lose information — extract the meaningful slice of the tool arguments for rendering. Examples:

- Web-search tools → extract `query`
- File-reading tools → extract `path`
- Shell tools → extract `command` (summarized)
- Unknown tool shape → fall back to raw JSON (respecting width / wrapping rules); do NOT hide the JSON entirely, since that removes the ability to iterate.

This is **best-effort with per-tool hooks**, not a hard invariant enforced by assertion.

## Section Model and Spacing Normalization

This is Child 3. It ships **after** Children 1, 2, 4, and 5.

### Section Model

Rendered output for a single non-interactive run consists of at most these nine sections, in this fixed order:

1. **Claudine execution line** — the header line announcing the wrapped invocation. _stderr._
2. **ENV variables** — when shown (e.g., via `--verbose` or equivalent). _stderr._
3. **System Prompt** — the system prompt passed to the provider, when shown. _stderr._
4. **Agent Prompt** — the user/agent prompt passed to the provider, when shown. _stderr._
5. **sessionID, model line** — the single provider-id + model identifier line (e.g., `- Claude session ID … · claude-opus-4-6[1m]`). _stderr._
6. **Thinking prose** — streamed reasoning/thinking content from the provider, when available. _stderr._
7. **Tool use and info/error events** — tool-call lines (Child 1 output) plus provider info/warning/error events. _stderr._
8. **Final STDOUT** — the provider's final assistant response text. _stdout._
9. **Final metadata lines** — the trailer (timing, token counts, cost, tool-call count, etc.). _stderr._

Only section 8 routes to stdout. All other sections route to stderr.

### Thinking Prose Rendering (Section 6)

Thinking events render as a `BlockQuote` with a grey vertical line; the quoted content is dim-italic prose, word-wrapped via `Layout`. Section 6 is routed to stderr. This applies to **all** providers whose stream exposes thinking/reasoning (Claude, Codex, and any future provider that exposes it).

Rationale: _the worst thing of all is a long wait with no feedback._ Showing thinking as its own section gives users continuous signal during long tool-using turns.

### Spacing Rule (Structural)

**At most one blank line between any two adjacent sections present in the rendered output.** Because sections are explicit, this rule is structurally enforceable rather than lexical.

### Trim Location

Spacing is enforced **at the sink level**, inside `claudine/cli/src/commands/wrap/live_semantic_sink.rs`, by deduplicating consecutive blank emissions as they are written to the output surface (stderr or stdout). This matches the existing pattern used for other cross-cutting sink-level concerns.

**Parsers remain lossless.** Per-provider `*_semantic.rs` modules do NOT trim blank lines; they continue to emit whatever the upstream stream provided. Any earlier language framing this as "per-provider trim" is superseded by the sink-level approach.

## Per-Provider Fix Decisions

### OpenCode

Cross-reference: "OpenCode CLI" observations above.

- **Assistant text missing from stdout (P0).** The spec's original OpenCode run produced the session trailer but NO assistant response text reached stdout. Running OpenCode directly (without claudine) renders the text fine, so this is a regression introduced by the claudine wrap pipeline. Restore stdout rendering of the assistant's final text. See Known Implementation-Time Investigations for the diagnostic plan.
- **Tool-call accounting.** OpenCode only emits tool _responses_, never paired _requests_. Current code synthesizes a fake outgoing request for "balanced" rendering. **Remove the synthesis.** Emit only `SemanticEvent::ToolResult` from `handle_tool_use_completed` in `claudine/lib/src/stream/opencode_semantic.rs:267`. Count responses only — this is what lets the trailer metadata match the rendered line count.
- **YOLO flag.** OpenCode DOES accept `--dangerously-skip-permissions` in non-interactive mode. Remove the current warning `- Warning: --yolo is not supported for 'opencode' and was ignored` for non-interactive invocations; pass the flag through. In interactive mode, keep a warning with refined copy: `- Warning: --yolo mode is not supported in OpenCode <i>interactive</i> sessions and was ignored`.
- **Mis-routed render diagnostic.** See Known Implementation-Time Investigations below.
- **Reasoning/thinking display.** Earlier notes about _suppressing_ OpenCode reasoning are superseded. Reasoning, when emitted, renders as section 6 per the section model — shown as a `BlockQuote` on stderr, not suppressed.

### Claude

Cross-reference: "Claude Code" observations above (duplicate SessionStart hook events, invalid rate-limit warning).

- **Hook/session ordering.** Fix the ordering so hook events trail the session-ID marker (matches every other provider). **Constraint (load-bearing):** if reordering requires buffering that breaks live streaming, revert to current ordering and document why. Preserving streaming wins over cosmetic ordering.
- **Rate-limit heuristic.** Gate the warning on `std::env::var("ANTHROPIC_API_KEY")`:
    - **Present** → user is on API pricing; the rate-limit message reflects a real API-pricing quota; keep the warning.
    - **Absent** → user is on a subscription; **suppress the warning on stderr only**. The underlying event still flows through the JSONL log and the dispatch pipeline — suppression is stderr-only, matching the existing `SILENT_PROVIDER_EXTENSION_KINDS` precedent in `live_semantic_sink.rs`.
- **Known limitation.** Bedrock (`CLAUDE_CODE_USE_BEDROCK`) / Vertex (`CLAUDE_CODE_USE_VERTEX`) users may need future refinement. Not in scope for this feature — see Out of Scope.

### Codex

Cross-reference: "Codex CLI" observations above (correct icon, zero tool-call metadata, vertical-spacing issues).

- The current `→ (tool)` / `← (tool)` rendering is missing **all** tool metadata (name, args, status, result). Investigation #5 below is a prerequisite for full Codex parity with Child 1's `ToolCallDisplay`.
- Once Investigation #5 identifies which fields Codex actually exposes, Codex populates `ToolCallDisplay` with its available fields and picks up Child 1's canonical rendering.
- Section-level spacing is addressed by Child 3.

### Gemini

Cross-reference: "Gemini CLI" observations above.

- **Tool-call rendering.** Current format is close; once Child 1 lands, align output to the canonical format:
    - `🔧 → Google Web Search · <dim><i>NFL draft 2026 dates</i></dim>`
    - `🔧 ← Google Web Search · <dim><i>successful</i></dim>`
- **Markdown list rendering.** See Known Implementation-Time Investigations below.

## Known Implementation-Time Investigations

These are spec'd investigation tasks, not unresolved design questions. Each must be reproduced against HEAD with tracing before the corresponding fix lands.

### 1. OpenCode assistant text missing from stdout (P0)

The spec's original OpenCode run produced the trailer metadata but NO assistant response text reached stdout. OpenCode native (run without claudine) renders the response normally, so the drop is introduced somewhere in the claudine wrap pipeline. Candidate root causes to investigate:

- The `emit_output_text` closure on `LiveSemanticSink` may not be wired up for the OpenCode wrap command (compare to the Claude/Codex/Gemini wrap wiring).
- `handle_text` in `claudine/lib/src/stream/opencode_semantic.rs` may fail to extract text when the `text` event payload uses the `part.text` shape observed in the captured fixtures (see event index 6 in both `opencode-yolo.jsonl` and `opencode-not-yolo.jsonl`).
- A stdout writer may be opened but never flushed on non-interactive completion paths.

Reproduce against HEAD with `RUST_LOG` / trace and determine which of the above (or combination) is live. Fix is blocking for Child 5.

### 2. OpenCode mis-routed render: `⚙ firecrawl_firecrawl_search {…raw JSON…}`

The observed rendering uses the `⚙` Info icon (not a tool-call icon) and dumps raw JSON. This does NOT match any traced render path in the current parser/sink pipeline — yet this is current HEAD behavior (not a stale capture). Reproduce against HEAD with tracing to find the mis-route **before** the parser rewrite lands in Child 5. Without this, the OpenCode rewrite risks preserving the bug.

### 3. Gemini markdown root cause

Symptoms: mid-list-item truncation and stray blank lines between list items.

**Root-cause hypothesis (requires verification):** Gemini streams each list item as a separate content chunk, and the markdown renderer treats each chunk as a standalone document, producing isolated per-item rendering plus premature list-break.

**Fix options (decide after reproducing):**

- Buffer-until-logical-break in the Gemini parser, OR
- A Darkmatter streaming-continuation fix.

### 4. Current truncation-limit location

The current rendering truncates summary text "way before" terminal width. This is almost certainly a hardcoded small cap somewhere in the sink / `Status` / Prose pipeline. Locating and removing it is part of Child 1 — the width + word-wrapping rules in the Tool-Call Display Contract cannot be implemented correctly without first removing whatever cap is currently in effect.

### 5. Codex tool-event field extraction

Observed Codex rendering is `→ (tool)` / `← (tool)` — tool name, arguments, status, and result are all missing from the rendered output. Investigation:

- Audit `claudine/lib/src/stream/codex_semantic.rs` against real Codex stream output.
- Identify which fields the Codex stream exposes (tool name, input, output, status, error) that the current parser currently ignores or drops.
- Wire the extracted fields through `ToolCallDisplay` so Codex renders at parity with other providers (canonical `🔧 →` / `🔧 ←` with name + summary + status).

**Blocks** the complete fix for the Codex per-provider subsection above.

## Out of Scope

- **Bedrock/Vertex rate-limit heuristic.** `CLAUDE_CODE_USE_BEDROCK` / `CLAUDE_CODE_USE_VERTEX` users may need a separate heuristic; explicitly deferred.
- **Render-path assertions.** The "no JSON to the terminal" guideline is best-effort with per-tool hooks and raw-JSON fallback — it is NOT enforced by runtime assertion.

## Acceptance Criteria & Testing

### Test Shape

- The `.jsonl` files in `features/2026-04-14-response-refinement/` are **input fixtures**, not tests.
- Tests assert on parser output (`SemanticEvent` streams) and sink rendering (stderr/stdout strings).
- Shape: **given fixture line(s) → parser emits expected semantic events → live sink renders expected lines.**
- Add new tests for each specific symptom documented in the Observed Symptoms sections above.

### Per-Provider Fixture to Assertion Examples

- **OpenCode** (`opencode-yolo.jsonl`, `opencode-not-yolo.jsonl`):
    - A `tool_use` fixture line → exactly one `🔧 ←` line with `Firecrawl Search` display name and the `query` value as the summary; NO synthesized outgoing line; response count in the trailer matches the rendered line count.
    - A `text` fixture event → its full content is written to stdout (regression test for the missing-assistant-text bug).
    - `--yolo` in non-interactive mode → `--dangerously-skip-permissions` is forwarded to the OpenCode process AND no `not supported` warning is emitted.
    - `--yolo` in interactive mode → refined warning emitted verbatim: `- Warning: --yolo mode is not supported in OpenCode <i>interactive</i> sessions and was ignored`.
- **Claude** (`claude.jsonl`):
    - SessionStart hook events render AFTER the session-ID marker (if streaming-preservation permits; otherwise test documents the fallback).
    - With `ANTHROPIC_API_KEY` unset → no `󰀨 rate limit` line in stderr, but the underlying event still appears in the JSONL log.
    - With `ANTHROPIC_API_KEY` set → `󰀨 rate limit` line remains on stderr.
- **Gemini**: a streamed markdown list fixture → list items render contiguously (no stray blank lines between items) and no item is cut off mid-content.
- **Codex**: tool-call events render via the canonical `🔧 →` / `🔧 ←` format with name + summary + status once Investigation #5 is completed and `ToolCallDisplay` is populated.

### Spacing Acceptance Criterion (Testable)

Against the full rendered output (stdout + stderr combined in emission order) for each provider fixture:

- There are no two consecutive blank lines, AND
- Between any two emissions originating from distinct sections (per the 9-section model), there is at most one blank line.

This is asserted structurally — the sink's dedupe-of-consecutive-blank behavior guarantees it; tests verify against captured rendered output.

### Definition of Done

- Child 1 (contract) shipped and all per-provider renders route through the single formatter.
- Each Observed Symptom above has at least one corresponding test asserting the fixed behavior.
- **OpenCode assistant response text reaches stdout for non-interactive runs** (matches OpenCode native output).
- **OpenCode `--yolo` forwards `--dangerously-skip-permissions` in non-interactive mode** with no spurious warning.
- No `󰀨 rate limit` noise on stderr for subscription users; underlying event still present in JSONL.
- No raw JSON payload rendered to the terminal for tools with a registered per-tool hook; unknown tools fall back to word-wrapped raw JSON.
- Section-model spacing rule holds for all four providers against their respective fixtures.
- Thinking prose (section 6) renders as a `BlockQuote` on stderr for every provider that exposes reasoning/thinking in its stream.
- Codex tool-call events render at parity with other providers once Investigation #5 lands.
