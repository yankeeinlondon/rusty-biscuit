# More is More — Technical Design & Review

**Spec:** [`spec.md`](./spec.md)
**Reviewer:** Claudine subsystem audit, 2026-04-16
**Branch:** `claudine`
**Scope:** four observed regressions following the 2026-04-16 composition-rendering unification (`fix: 2026-04-16-consistent-rendering`).

## 1. Executive Summary

The composition-rendering unification refactored stderr emission to a single
9-section model and a shared `LiveSemanticSink`. In doing so it silently
inherited four pre-existing limitations of the typed semantic-event pipeline
that had been masked by the older legacy paths:

| # | Symptom | Root cause | Severity |
|---|---------|-----------|----------|
| 1 | `Bash` / `Zsh` shown without parameters; `Task` shown without task body | `extract_tool_summary()` lacks a `Task` hook; rendering format `→ Bash · {cmd}` does not match the spec-mandated `Bash(bash {params})` shape | High |
| 2 | OpenCode "thinking" prose missing | `OpenCodeEvent` enum has no `Reasoning` variant; events with `"type":"reasoning"` fall through to `ProviderExtension`, where they render as a one-line `Status::Info` instead of a `BlockQuote` | High |
| 3 | Errors render as a single `Status::Failure` line; no error classification | `LiveSemanticSink::render_event()` routes `SemanticEvent::Error` through `Status` only; there is no error-kind taxonomy on the event | High |
| 4 | Heartbeats every 30 s for >1 000 s, then on `^C` the buffered text appears | `StreamTextRenderer::block_buffer` is only flushed on a paragraph boundary, code-fence close, or EOF — a dangling final paragraph stays buffered forever if the provider never closes its stream | Critical |

All four are tractable with targeted, low-risk changes. The plumbing already
exists (`AgentErrorReport` already renders a red BlockQuote with `▌ ` border;
the section-aware stderr emitter already supports `Section::Thinking`).

---

## 2. Affected Code Map

| Concern | Primary file | Secondary files |
|---------|--------------|-----------------|
| Tool-call rendering | [`claudine/lib/src/stream/tool_display.rs`](../../lib/src/stream/tool_display.rs) | [`claudine/cli/src/commands/wrap/live_semantic_sink.rs::render_tool_display`](../../cli/src/commands/wrap/live_semantic_sink.rs) |
| OpenCode reasoning gap | [`claudine/lib/src/stream/protocol/opencode.rs`](../../lib/src/stream/protocol/opencode.rs) | [`claudine/lib/src/stream/opencode_semantic.rs`](../../lib/src/stream/opencode_semantic.rs) |
| Thinking BlockQuote style | [`claudine/lib/src/stream/thinking.rs`](../../lib/src/stream/thinking.rs) | (uses default `│ ` — needs `▌ `) |
| Warning/error rendering | [`claudine/cli/src/commands/wrap/live_semantic_sink.rs::render_event`](../../cli/src/commands/wrap/live_semantic_sink.rs) | [`claudine/lib/src/stream/semantic.rs::SemanticEvent::Error`](../../lib/src/stream/semantic.rs); [`claudine/cli/src/output/error_report.rs::AgentErrorReport`](../../cli/src/output/error_report.rs) |
| Hanging output / heartbeat | [`claudine/cli/src/commands/wrap/exec.rs::StreamTextRenderer`](../../cli/src/commands/wrap/exec.rs) | [`claudine/lib/src/stream/progress.rs`](../../lib/src/stream/progress.rs) |

---

## 3. Issue 1 — Tool Call Parameters

### 3.1 Current Behavior

`LiveSemanticSink::render_tool_display()` builds the line as:

```text
→ {Display Name} · {dim-italic summary}
```

Examples:

```text
→ Bash · ls -la
→ Bash · git status
← Bash · successful
→ Task · Investigate hanging behavior
```

The `{summary}` slot is produced by
[`extract_tool_summary()`](../../lib/src/stream/tool_display.rs:173) — it has
explicit handlers for `Bash` (preferred key `command`), `Read`/`Write`/`Edit`
(`file_path`), `Glob`/`Grep` (`pattern`), and `WebSearch`/`WebFetch` (`query`).
There is **no** explicit handler for `Task`, so it falls through to the generic
"first non-empty top-level string" branch — that may pick `description`,
`prompt`, or any other string field depending on which the provider serialized
first.

### 3.2 Spec Requirement

> not just `Bash` or `Zsh` but the parameter which were used in the tool call!
> I propose in this case we format this information like
> `Bash(<dim><i>bash {params}</i></dim>)`,
> `Zsh(<dim><i>zsh {params}</i></dim>)`, etc.
>
> similarly when we see `Task` we should see the task information too not just
> that we created a task

Two distinct sub-requirements:

1. **Format change** — drop the `·` separator and wrap the parameters in
   parentheses next to the tool name: `Bash(bash ls -la)` rather than
   `Bash · ls -la`. The shell name itself (`bash`, `zsh`, `pwsh`, …) is
   prefixed inside the parentheses.
2. **Task surface** — Task tool calls must show the task body
   (subject/description/prompt) in the dim-italic slot.

### 3.3 Design

#### 3.3.1 Format change

Update `LiveSemanticSink::render_tool_display()`:

```rust
fn render_tool_display(display: ToolCallDisplay) -> String {
    let arrow = match display.direction {
        ToolDirection::Outgoing => '\u{2192}',
        ToolDirection::Incoming => '\u{2190}',
    };
    let name = escape_prose(&display.display_name);

    let slot = match (display.status, display.summary) {
        (Some(ToolStatus::Success), _) => Some("<dim><i>successful</i></dim>".to_string()),
        (Some(ToolStatus::Error),   _) => Some("<red><b>error</b></red>".to_string()),
        (Some(ToolStatus::Pending), _) => Some("<dim><i>pending</i></dim>".to_string()),
        (None, Some(summary))          => Some(format!("<dim><i>{}</i></dim>", escape_prose(&summary))),
        (None, None)                   => None,
    };

    match slot {
        Some(text) => format!("{arrow} {name}({text})"), // <-- parentheses, no separator
        None       => format!("{arrow} {name}"),
    }
}
```

#### 3.3.2 Bash/Zsh prefix

The shell prefix (`bash `, `zsh `, …) belongs in the *summary*, not in the
display name, so the existing humanizer can stay untouched. Update
[`extract_tool_summary()`](../../lib/src/stream/tool_display.rs:173) so the
shell-tool branches prepend the canonical shell name when it is missing:

```rust
"Bash" | "bash" | "run_command" => {
    if let Some(Value::String(cmd)) = obj.get("command") {
        // Provider-emitted commands almost never include the leading shell
        // name; the spec wants `bash {cmd}` so the user can reason about
        // how it would actually run.
        return Some(format!("bash {cmd}"));
    }
}
```

If/when other shells (`zsh`, `pwsh`, `nu`) are surfaced through provider
tooling, add them to the same branch. **Do not** hardcode a shell prefix in
the `display_name`-side mapping — keep the prefix scoped to the summary so
the result-line ("← Bash(successful)") remains clean.

#### 3.3.3 `Task` hook

Add an explicit `Task` extractor that prefers the human-meaningful fields
in priority order:

```rust
"Task" | "task" => {
    for key in ["description", "subject", "prompt", "task"] {
        if let Some(Value::String(s)) = obj.get(key)
            && !s.is_empty()
        {
            return Some(s.clone());
        }
    }
    // Fall through to generic key search.
}
```

This is explicit, ordered, and easy to extend per provider.

### 3.4 Tests

Add to `tool_display.rs`:

* `bash_summary_prepends_shell_name`
* `task_extracts_description_first`
* `task_falls_back_to_prompt_when_description_absent`

Add to `live_semantic_sink.rs`:

* `tool_call_renders_with_parentheses_format` — assert the line contains
  `Bash(` and ends with `)` (no `·` separator).

### 3.5 Risks & Tradeoffs

* The `(` / `)` characters do **not** require escaping in biscuit-terminal
  prose markup (only `<`, `>`, `{`, `\` do). Safe.
* Width: `Bash(bash {very-long-cmd})` will be wider than the previous
  `Bash · {very-long-cmd}` by exactly two characters. `Status::from_prose`
  already wraps via `Layout`, so this is not a hard regression.
* The result-line shape changes too: `← Bash(successful)` instead of
  `← Bash · successful`. This is consistent with the call-line shape and
  was implicit in the spec.

---

## 4. Issue 2 — Missing Thinking Text (OpenCode)

### 4.1 Root Cause

OpenCode's NDJSON wire format emits a **top-level** `"type":"reasoning"`
event (verified in
[`features/2026-04-14-response-refinement/opencode-yolo.jsonl`](../../features/2026-04-14-response-refinement/opencode-yolo.jsonl)):

```json
{"type":"reasoning","sessionID":"ses_…","text":"The user is asking about …"}
```

The current
[`OpenCodeEvent`](../../lib/src/stream/protocol/opencode.rs:17) tagged enum
contains 14 variants — none for reasoning. Typed deserialization fails for
this event type, the parser's `Err(_) => self.emit_provider_extension(…)` arm
fires, and the event is delivered to the sink as
`SemanticEvent::ProviderExtension { provider: OpenCode, kind: "reasoning",
payload: {…} }`.

`LiveSemanticSink::render_event()` then routes that ProviderExtension through
[`provider_extension_description`](../../cli/src/commands/wrap/live_semantic_sink.rs:349)
and renders it as a single dim line:

```text
ⓘ opencode/reasoning · The user is asking about when the NFL draft is in 2026…
```

The user sees a one-line "info" status, not the BlockQuote thinking block
they expect.

### 4.2 Why It Worked Before

It didn't, exactly. The pre-typed-protocol stream parser (legacy
`opencode_value` helper) collapsed reasoning text into the assistant-text
buffer for some OpenCode formats; that side effect produced *something*
visible. The 2026-04-11 protocol refactor replaced that path, and reasoning
silently dropped out of the typed pipeline because the variant was never
added.

### 4.3 Design

#### 4.3.1 Add the typed variant

In [`stream/protocol/opencode.rs`](../../lib/src/stream/protocol/opencode.rs):

```rust
#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeReasoning {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub part: Option<OpenCodeReasoningPart>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeReasoningPart {
    #[serde(default)]
    pub text: Option<String>,
}

impl OpenCodeReasoning {
    pub fn resolved_text(self) -> Option<String> {
        self.part.and_then(|p| p.text).or(self.text)
    }
}
```

Add the variant to the enum, between `AssistantText` and `StepFinish`:

```rust
#[serde(rename = "reasoning")]
Reasoning(OpenCodeReasoning),
```

#### 4.3.2 Wire to the semantic parser

In [`opencode_semantic.rs`](../../lib/src/stream/opencode_semantic.rs):

```rust
fn handle_reasoning(&mut self, event: OpenCodeReasoning, raw_kind: &str) {
    let Some(text) = event.resolved_text() else { return };
    if text.is_empty() { return; }
    self.sink.on_semantic_event(SemanticEvent::Reasoning {
        text,
        extra: Value::Object(self.base_extra(raw_kind)),
    });
}
```

Add the dispatch arm to `feed_line`:

```rust
Ok(OpenCodeEvent::Reasoning(r)) => self.handle_reasoning(r, &raw_kind),
```

Also remove the now-stale module-doc claim that *"Reasoning … aren't
currently exposed in the NDJSON stream"* (line 7).

#### 4.3.3 Use the wider border in `render_thinking_block`

The spec asks for the same wider block character used by System Prompt /
Agent Prompt rendering — that is `▌ ` (verified in
[`output.rs`](../../cli/src/output.rs#L191), error_report, and
shell_expansion_error). Today
[`render_thinking_block`](../../lib/src/stream/thinking.rs:15) uses
`BlockQuote::from(prose)` which inherits the default `│ ` border:

```rust
pub fn render_thinking_block(text: &str, terminal: &Terminal) -> String {
    if text.trim().is_empty() { return String::new(); }
    let prose = Prose::new(format!("<dim><i>{text}</i></dim>"));
    BlockQuote::from(prose)
        .with_border("▌ ")          // <-- new
        .render(terminal)
}
```

The default `left_block_color` is already `Tailwind::Gray500` — no color
change needed. The spec calls out *gray*, not a specific shade, and Gray500
is what the existing thinking renderer already produces.

### 4.4 Tests

In `protocol/opencode.rs`:

* `opencode_reasoning_top_level_text_deserializes`
* `opencode_reasoning_nested_part_text_resolves`
* `opencode_reasoning_unknown_event_type_fails_typed` (no regression)

In `opencode_semantic.rs`:

* `reasoning_event_emits_semantic_reasoning`
* `reasoning_with_empty_text_emits_nothing`

In `live_semantic_sink.rs` (already covers `Reasoning`-routing, but):

* `opencode_reasoning_now_renders_as_blockquote_not_provider_extension` —
  wire a ProviderExtension `kind:"reasoning"` *and* a real
  `SemanticEvent::Reasoning`; assert only the latter produces border-prefixed
  output (`▌ `) on the captured stderr lines.

In `thinking.rs`:

* `block_quote_uses_wider_border_character`

### 4.5 Risks & Tradeoffs

* **Backward-compat:** the old "ProviderExtension(reasoning)" rendering
  disappears. JSONL log shape stays compatible — typed `Reasoning` events
  are already serialized via `extra["semantic_event"]` for the Notification
  agentic-event mapping.
* **Other providers** — Codex already emits Reasoning natively; Claude emits
  it via `thinking_delta`. After this fix all five "thinking-capable"
  providers (Claude, Codex, OpenCode, Gemini, Qwen) flow through the same
  `SemanticEvent::Reasoning → render_thinking_block` path. Gemini and Qwen
  parsers should be audited the same way (out of scope for this fix; capture
  in follow-up).

---

## 5. Issue 3 — Warnings & Errors

### 5.1 Current Behavior

In
[`live_semantic_sink.rs::render_event`](../../cli/src/commands/wrap/live_semantic_sink.rs:493):

```rust
SemanticEvent::Warning { message, extra } => {
    if !message.starts_with("Malformed JSON on line ")
        && !is_suppressed_claude_rate_limit(self.provider, extra) {
        self.render_status(section, StatusState::Warning, message.clone());
    }
}
SemanticEvent::Error { message, .. } => {
    self.render_status(section, StatusState::Failure, message.clone());
}
```

Warnings already use `StatusState::Warning` — that part of the spec is
already satisfied. The error path is the gap: it renders as a single
`Status::Failure` line (small red icon, no body, no border, no
classification).

### 5.2 Spec Requirement

> warnings should use the `Status` struct in WARNING state ✓ (already done)
>
> errors should use the BlockQuote style rendering with a red vertical bar.
> there should be examples of this already in the code base
>
> We also need to be able to clearly identify different types of errors so
> that our "handling" logic can appropriately respond to errors

### 5.3 Existing Reusable Infrastructure

[`AgentErrorReport`](../../cli/src/output/error_report.rs) already renders
exactly the shape the spec asks for, **with** typed classification:

```rust
pub(crate) enum AgentErrorCategory {
    Configuration,   // orange
    AgentNative,     // red
    ApiRemote,       // red
    Interrupted,     // yellow
}
```

It uses `BlockQuote::new(...).with_left_block_color(Color::Tailwind(Tailwind::Red700)).with_border("▌ ")`.
This is the template the live-event path should reuse.

### 5.4 Design

#### 5.4.1 Add an error-kind to the typed event

In [`stream/semantic.rs`](../../lib/src/stream/semantic.rs):

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SemanticErrorKind {
    Configuration,   // model/auth/config issue, mappable to AgentErrorCategory::Configuration
    AgentNative,     // provider runtime error (parse, internal)
    ApiRemote,       // upstream LLM API error (rate limit, 5xx, network)
    Interrupted,     // user interrupt / signal
    Unknown,         // backstop; renders as AgentNative
}

impl Default for SemanticErrorKind {
    fn default() -> Self { Self::Unknown }
}

Error {
    message: String,
    kind: SemanticErrorKind,           // <-- new
    terminal: bool,
    extra: Value,
},
```

The variant is added with `#[serde(default)]` so existing JSONL replay
fixtures without a `kind` field continue to deserialize as `Unknown`. This
is the same evolution discipline used elsewhere in `protocol/`.

Per-provider semantic parsers must populate `kind`:

| Provider | Source | Mapped kind |
|----------|--------|-------------|
| Claude | `result.is_error` + `error.type` (e.g. `overloaded_error`) | `ApiRemote`; `invalid_request_error` → `Configuration` |
| Codex | `error.type=usage_limit_reached` → `ApiRemote`; `auth/account` → `Configuration` | |
| OpenCode | `error_type` literal table; default `AgentNative` | |
| Gemini / Qwen / Kimi | `error.type` text classification (regex on message) | |

Existing `is_suppressed_claude_rate_limit` already encodes one of these
classifications inline; that helper can be replaced once the typed kind is
populated by the parser.

#### 5.4.2 Render via BlockQuote

Add a helper inside `LiveSemanticSink` (or a sibling module if it grows
beyond a few lines):

```rust
fn render_error_block(&mut self, message: &str, kind: SemanticErrorKind) {
    use biscuit_terminal::components::block_quote::BlockQuote;
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::utils::color::{Color, Tailwind};

    let (label, color) = match kind {
        SemanticErrorKind::Configuration => ("Configuration Error", Tailwind::Orange700),
        SemanticErrorKind::Interrupted   => ("Interrupted",         Tailwind::Yellow700),
        SemanticErrorKind::AgentNative
        | SemanticErrorKind::Unknown     => ("Agent Error",         Tailwind::Red700),
        SemanticErrorKind::ApiRemote     => ("API Error",           Tailwind::Red700),
    };

    let body = Prose::new(format!(
        "<red><b>{label}</b></red> <dim>({})</dim>\n{}",
        provider_short(self.provider),
        escape_prose(message),
    ));

    let rendered = BlockQuote::from(body)
        .with_left_block_color(Color::Tailwind(color))
        .with_border("▌ ")
        .render(&self.terminal);

    for line in rendered.lines() {
        self.emit_section_line(Section::ToolUseAndEvents, line);
    }
}
```

And update the dispatch:

```rust
SemanticEvent::Error { message, kind, .. } => {
    self.render_error_block(message, *kind);
}
```

The split-by-lines emission preserves the `SectionTracker`'s blank-line
contract. (Same trick already used by the `Reasoning` arm — see
[`live_semantic_sink.rs:594-601`](../../cli/src/commands/wrap/live_semantic_sink.rs).)

#### 5.4.3 `AgentErrorReport` reuse

`AgentErrorReport` is currently scoped to post-process exit-code formatting
inside `wrap/mod.rs`. We don't try to reuse it directly here — its API takes
a fully-built report with body lists, footers, suggestions, etc. Live event
errors are ephemeral and arrive without that ceremony. Sharing the **rendering
recipe** (`BlockQuote::with_border("▌ ").with_left_block_color(...)`) is the
right abstraction; sharing the type would force every parser to construct
suggestion lists.

That said, the new `SemanticErrorKind` should map cleanly onto
`AgentErrorCategory` (`Configuration ↔ Configuration`, `AgentNative ↔
AgentNative`, etc.). Add a `From<SemanticErrorKind> for AgentErrorCategory`
in `error_report.rs` so the post-process exit-code path can promote a typed
live error to a richer end-of-run report when desirable.

### 5.5 Tests

In `semantic.rs`:

* `error_kind_round_trips_through_serde`
* `error_kind_default_is_unknown_when_field_missing`

In each provider's `*_semantic.rs`:

* Two fixture-based tests asserting the kind mapping (one Configuration,
  one ApiRemote / AgentNative).

In `live_semantic_sink.rs`:

* `error_event_renders_blockquote_with_red_border`
* `interrupted_error_renders_blockquote_with_yellow_border`
* `configuration_error_renders_blockquote_with_orange_border`

### 5.6 Risks & Tradeoffs

* Adding a non-`#[serde(default)]` field to `SemanticEvent::Error` would
  break replay fixtures. Use `#[serde(default)]` on `kind`.
* Classification rules must live in one place per provider (likely a
  `classify_error()` helper next to each `handle_error`). Avoid scattering
  across handlers.
* Width: BlockQuote with 2-char border + 2-char left margin is wider than a
  Status line. Acceptable — errors *should* draw the eye.

---

## 6. Issue 4 — Hanging Output

### 6.1 Symptom Reconstruction

The spec's terminal capture shows ~18 minutes of `Ns · 3 done` heartbeats
followed by an instant burst of buffered text on `^C`:

```text
1080s · 3 done
^CTwo files are staged, both in claudine-cli, related to the wrap command's…
✓ 1081s · 2K input tokens · 296 output tokens · 70K cached tokens · $0.02 cost basis · 3 tool calls
```

296 output tokens (~75 words) of assistant text were *generated by the
agent* but never reached the terminal until `^C` triggered a process kill.

### 6.2 Root Cause

[`StreamTextRenderer::push`](../../cli/src/commands/wrap/exec.rs:77) splits
incoming text on `\n` and dispatches each complete line to
`process_line()`. `process_line` accumulates lines into `block_buffer` and
*only* calls `flush_block` at three boundary kinds:

1. Closing fence (` ``` ` or `~~~`).
2. Blank line (paragraph boundary).
3. Stream-safe list items (`- `, `* `, `+ `, `1. `).

Partial trailing lines stream raw via the `if !self.line_buffer.is_empty()
&& !self.in_code_fence && self.block_buffer.is_empty()` branch — but that
branch **only fires when `block_buffer` is empty**. As soon as a complete
line lands in the block buffer, every subsequent partial line is buffered
silently.

`flush_remaining` runs only when the stdout reader's `for line in
reader.lines()` loop ends — i.e. on EOF. If the provider keeps stdout open
indefinitely (waiting for further input, hung in a post-completion idle
state, deadlocked in a subprocess), EOF never arrives and `flush_remaining`
never runs.

Meanwhile the heartbeat thread has its own clock and fires every
`HeartbeatPolicy::DEFAULT_INTERVAL` (30 s). It only displays the live
metric snapshot — it has no view of the renderer's `block_buffer` at all.
The user sees `Ns · 3 done` because:

* Three tools ran and finished (`done_count = 3`),
* No new tool started after that (`in_flight.is_empty()`),
* Token usage / cost are not present until `TurnComplete`, which the
  provider never emitted.

The `last_event_at` in metrics state still gets refreshed every time an
`OutputText` activity event lands, so the silence-window correctly
suppressed the heartbeat between text bursts; the force-window then
re-emitted at the 120-s cadence — matching the 30-s tick visible in the
capture.

### 6.3 What `^C` Triggers (for context)

`wait_with_signal_handling` returns when SIGINT propagates to the child;
`kill_process_group` then sends SIGTERM to the whole group; the child's
stdout pipe closes; the reader loop ends; `flush_remaining` finally runs;
the buffered paragraph is rendered.

This is **structurally correct** but invisible to the user — by then they
have already lost confidence in the tool.

### 6.4 Design

Three complementary fixes, ordered by intrusiveness. The first is required;
the second is strongly recommended; the third is opportunistic.

#### 6.4.1 (Required) Time-based block flush

Make the renderer flushable from outside the stdout-reader thread, and have
the heartbeat thread call it whenever the silence window has elapsed since
the last *renderer* write — independent of the live-metrics clock.

Concrete plumbing:

```rust
struct StreamTextRenderer {
    // existing fields …
    last_block_growth_at: Option<Instant>,    // <-- new
}

impl StreamTextRenderer {
    fn process_line<W: Write>(&mut self, out: &mut W, line: &str) {
        // existing body …
        self.last_block_growth_at = Some(Instant::now());
    }

    /// Flush any buffered block content if it has been idle for at least
    /// `idle_threshold`. Returns true when something was flushed.
    fn flush_if_idle<W: Write>(&mut self, out: &mut W, idle_threshold: Duration) -> bool {
        if self.block_buffer.is_empty() {
            return false;
        }
        let Some(t) = self.last_block_growth_at else { return false };
        if t.elapsed() < idle_threshold { return false; }
        self.flush_block(out);
        true
    }
}
```

Wrap `text_renderer` in `Arc<Mutex<…>>` (already true) and pass a clone to
the heartbeat thread. The heartbeat then calls `flush_if_idle` against
`StreamOutput::stdout_writer()` *before* it emits its own status line, so
the buffered text always appears above the next heartbeat.

Recommended threshold: **`HeartbeatPolicy::silence_window`** (30 s) — the
same threshold already used to gate heartbeat emission. Keeps tuning in one
place.

#### 6.4.2 (Recommended) Stream complete lines outside fences

The original "wait for paragraph boundary so darkmatter can render the
whole block at once" optimization is sound for fenced blocks (rendering
syntax-highlighted code mid-stream produces a flicker) but is over-eager
for plain paragraphs. The fix: relax `process_line` to flush every
non-fenced, non-list line that *ends with* a sentence terminator (`.`, `!`,
`?`) followed by space-or-newline. That's where users expect a break, and
the markdown structure for prose is unchanged whether it renders one
sentence or ten at a time.

This is a quality-of-life refinement — the time-based flush in §6.4.1 is
the safety net.

#### 6.4.3 (Opportunistic) Stalled-stream warning

After N heartbeats with no `OutputText`, no `ToolCall`, no `ToolResult`,
and no `TurnComplete` (default N = 4 → 2 minutes), the heartbeat should
emit a `Status::Warning` line:

```text
⚠ no provider activity in 2m — provider may be hung; press Ctrl+C to abort
```

Rationale: even with §6.4.1 fixing the *invisibility* problem, the user
still doesn't know whether the provider has actually finished its work and
gone idle, or is genuinely hung. A warning gives them an explicit decision
point instead of a perpetual heartbeat.

Optional knob: `CLAUDINE_STALL_TIMEOUT_SECONDS=120` to make this overrideable
without recompile.

### 6.5 Why Not Just Lower the Heartbeat Cadence?

The heartbeat is a *status indicator*, not a *flush trigger*. Even at 5-s
cadence the buffered text would not appear faster — the bug is that the
text never enters the renderer's "flushable" state.

### 6.6 Tests

In `exec.rs` (existing test module):

* `flush_if_idle_emits_block_after_threshold`
* `flush_if_idle_does_not_emit_when_block_empty`
* `flush_if_idle_resets_growth_clock`

In an integration fixture under `claudine/tests/`:

* `dangling_paragraph_renders_within_silence_window` — feed a parser two
  `OutputText` chunks ending mid-paragraph, drive the heartbeat thread for
  2 × silence_window with a fake clock, assert the buffered paragraph
  reaches the captured stdout before any new heartbeat status.

### 6.7 Risks & Tradeoffs

* **Markdown rendering churn:** flushing an incomplete paragraph means
  darkmatter renders it without later context (e.g. a trailing reference
  link). For plain prose this is cosmetically identical; for tables and
  reference-style links it is subtly worse. Acceptable trade — the
  alternative is invisibility.
* **Lock contention:** the heartbeat thread now takes the renderer mutex
  every 30 s. Negligible — it already takes `live_metrics` and
  `stream_output` locks for shorter durations.
* **Test surface:** time-based behaviors need controllable clocks. A simple
  `Instant`-based threshold avoids needing a clock injection if tests use
  small thresholds (`Duration::from_millis(50)`).

---

## 7. Cross-Cutting Concerns

### 7.1 Section Discipline

Every fix above writes through the existing `Section`-aware emitters:

| Fix | Section |
|-----|---------|
| Tool format | `Section::ToolUseAndEvents` (unchanged) |
| OpenCode reasoning | `Section::Thinking` (already wired in `on_semantic_event`) |
| Error BlockQuote | `Section::ToolUseAndEvents` (errors are part of run activity) |
| Stalled-stream warning | `Section::ToolUseAndEvents` |
| Idle-flushed text | `Section::FinalStdout` (renderer writes through `stdout_output.stdout_writer()`, which already cooperates with the section tracker via `enter_final_stdout`) |

No new sections are required. The 9-section model from the
2026-04-14-response-refinement feature stays intact.

### 7.2 JSONL Reporting

* `SemanticErrorKind` flows through to `extra["semantic_event"]["kind"]`
  automatically via existing `serde_json::to_value` in
  `semantic_event_to_event_meta`.
* Idle-flushed text triggers no new event — the existing `OutputText`
  events that produced it are already logged.
* Stalled-stream warning should be emitted as `SemanticEvent::Warning`
  (so it lands in dispatch + JSONL) rather than as an out-of-band
  `Status` print. This keeps the stderr surface and the JSONL log in
  agreement.

### 7.3 Hook Dispatch

`AgenticEvent` mapping in `LiveSemanticSink::to_agentic` does not need
changes:

* `Error { terminal: true }` still maps to `TurnError`; `kind` is
  carried as additional metadata.
* `Warning` still maps to `Notification`.
* `Reasoning` still maps to `None` (renders only).

### 7.4 Provider Coverage Audit

Recommended follow-up (out of scope for this fix): audit Gemini and Qwen
typed protocol parsers for the same "missing reasoning variant" gap. The
protocol pattern documented in [`stream/protocol/mod.rs`](../../lib/src/stream/protocol/mod.rs)
makes this a one-grep test:

```sh
rg --type rust 'enum.*Event' claudine/lib/src/stream/protocol/ | rg -i reasoning
```

If a provider's enum is missing a `Reasoning` variant but its wire log
contains `"type":"reasoning"` (or equivalent), apply the same pattern as
§4.3.

### 7.5 Documentation

Update on completion:

* [`claudine/docs/topics/composition.md`](../../docs/topics/composition.md) — note the new error
  rendering shape and stalled-stream warning.
* [`claudine/cli/README.md`](../../cli/README.md) — mention typed
  `SemanticErrorKind` and the renderer's idle flush in the
  `2026-04-14-response-refinement` section block.
* [`.claude/skills/claudine/SKILL.md`](../../../.claude/skills/claudine/SKILL.md) — add a
  one-paragraph note under "live stderr surface" about the flush-on-idle
  contract and the typed error kind.

---

## 8. Implementation Order

Recommended phasing for one developer:

1. **Phase 1 — Tool format** (issue 1, ~1 hr). Pure UI change, no
   behavioral risk. Lock down with new tests.
2. **Phase 2 — Hanging fix** (issue 4 §6.4.1 only, ~2 hr). Unblocks the
   most user-visible bug. Defer §6.4.2 and §6.4.3 to follow-up.
3. **Phase 3 — OpenCode reasoning** (issue 2, ~2 hr). Adds the variant,
   the dispatch arm, and fixes the BlockQuote border at the same time.
4. **Phase 4 — Typed error rendering** (issue 3, ~3 hr). Most surface
   area: SemanticEvent change, every provider's `handle_error`, plus the
   live sink renderer.
5. **Phase 5 — Polish & docs** (~1 hr). §6.4.2 sentence-flush, §6.4.3
   stall warning, README/skill updates.

Total: ~9 hours of focused work, minus parallelizable test writing.

## 9. Open Questions

1. **Sentence-flush heuristic vs always-flush** (§6.4.2): the proposed
   rule (flush on `[.!?]` followed by space/newline) captures most prose
   correctly but may misfire on abbreviations (`Dr. Smith`) or numbered
   citations (`see [1].`). Is the cosmetic risk worth the responsiveness
   improvement, or should we ship only the time-based flush from §6.4.1?
2. **Error severity surface in dispatch** (§5.4.1): should
   `SemanticErrorKind` map to `AgenticEvent::TurnError` for *all* terminal
   variants, or only for `AgentNative`/`ApiRemote` (treating
   `Configuration` and `Interrupted` as `Notification`)? The current code
   uses `terminal: bool` as the sole pivot — I would keep that contract
   and let `kind` be purely classificatory.
3. **Backwards-compat for replay fixtures** (§4 + §5): the existing
   `lib/tests/semantic_fidelity.rs` round-trip suite must continue to pass
   after adding the new variants. Verify with
   `cargo test -p claudine semantic_fidelity` before merging each phase.
