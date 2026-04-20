# Response Refinement Investigations

Findings recorded against HEAD before the corresponding fix lands.

## 0a — OpenCode Assistant Text Missing From Stdout (P0)

### Fixture shape

The captured fixtures (`opencode-yolo.jsonl`, `opencode-not-yolo.jsonl`)
are pretty-printed JSON (each event spans ~15 lines) but a fresh
`opencode run --format json --model opencode/claude-haiku-4-5 "what is
2+2?"` emits real NDJSON (one JSON object per line). The canonical
assistant-text event shape in both captures is:

```json
{"type":"text","sessionID":"…","part":{"type":"text","text":"…response…",…}}
```

— i.e. the assistant text lives at `part.text`, with no top-level
`text` field.

### Code path audit (HEAD)

Walking the wrap pipeline end-to-end, every stage required to stream
`part.text` to stdout appears to be wired:

- `claudine/lib/src/stream/protocol/opencode.rs:81-103` — `OpenCodeText`
  declares `text`, `content`, and `part: Option<OpenCodeTextPart>`, and
  `resolved_text()` prefers `part.text` before the legacy top-level
  fallbacks. The unit tests at `opencode.rs:395-420`
  (`opencode_text_from_part`, `..._from_top_level`,
  `..._from_content_fallback`) all pass.
- `claudine/lib/src/stream/opencode_semantic.rs:196-208` — `handle_text`
  calls `event.resolved_text()`, skips empty strings, pushes the text
  onto `self.assistant_text`, and emits
  `SemanticEvent::OutputText { text, extra }` via `self.sink`.
- `claudine/lib/src/stream/opencode_semantic.rs:399-405` — the
  `OpenCodeEvent::{Text, TextDelta, AssistantText}` arm dispatches
  into `handle_text`.
- `claudine/cli/src/commands/wrap/live_semantic_sink.rs:503-517` — the
  sink's `on_semantic_event` forwards `SemanticEvent::OutputText` into
  `self.emit_output_text` BEFORE the status-line renderer and the
  dispatch step.
- `claudine/cli/src/commands/wrap/mod.rs:1284-1291` — the structured
  branch installs `output_cb` via `sink.with_output_text_sink(output_cb)`
  inside the `SemanticParserBuilder` closure; this wiring is provider-
  agnostic and applies to OpenCode (which reports
  `supports_structured_stream() == true` at `profile.rs:1526-1528`).
- `claudine/cli/src/commands/wrap/exec.rs:1060-1072` — `output_cb`
  pushes the chunk through `StreamTextRenderer::push`, which calls
  `out.write_all(...)` + `out.flush()` on `StdoutWriter` for every
  complete line (`exec.rs:77-114`) and flushes leftovers on child exit
  (`exec.rs:1114-1119`, via `flush_remaining`).

Nothing in this chain obviously drops `part.text`, and no `flush()` is
missing on the completion path.

### Live reproduction attempted but blocked

Ran `cargo run -q -p claudine-cli -- opencode --model
opencode/claude-haiku-4-5 -- "what is 2+2?"` and observed exit=0 and
empty stdout (matching the spec's original symptom). The stderr trail
shows OpenCode itself failing with `ProviderModelNotFoundError:
openrouter/qwen3-coder` — so OpenCode emits NO `type: "text"` event in
this run. In this environment the empty stdout is "no event produced,"
not "text event dropped in the pipeline."

A direct `opencode run --format json --model opencode/claude-haiku-4-5
"what is 2+2?"` succeeds and produces a proper NDJSON text event with
`part.text`, confirming native OpenCode is healthy and this is
specifically a wrap-time interaction.

### Model-override source (partially pinned)

An earlier revision of this write-up asserted the override came from
`OPENCODE_CONFIG_CONTENT` injected by `apply_system_prompt`
(`profile.rs:1376-1408`). That claim is wrong: that injection only
emits `{"instructions": [<tmp path>]}` — there is **no** `model` key in
the temp config. The wrap pipeline also:

- builds argv in the correct order — `run`, then `--model …`, then
  `--format json`, then `--`-separated positional prompt
  (`composition.rs:362-388, 483-507`), and
- launches with `Command::env_clear().envs(env)` using an explicitly
  built env map (`exec.rs:335-340, 738, 1011`), so no `OPENCODE_*`
  variable from the parent shell can leak through.

Diagnostic probes against this environment identified **one confirmed
contributor** and **two candidates still unverified**:

1. **Confirmed — on-disk OpenCode config.** `~/.config/opencode/`
   resolves (via symlink) to `~/config/opencode/`, whose `config.json`
   contains `"model": "openrouter/qwen3-coder"`. That exact string
   matches the `ProviderModelNotFoundError` payload byte-for-byte, so
   the *source* of the offending model string is the on-disk config,
   **not** Claudine's `OPENCODE_CONFIG_CONTENT` injection.
2. **Unverified — why `--model` does not override on-disk config.**
   Claudine does pass `--model opencode/claude-haiku-4-5` in argv
   (verified above), yet OpenCode resolves `openrouter/qwen3-coder`.
   Possible mechanisms, none yet proven:
   - `OPENCODE_CONFIG_CONTENT` is treated as a *complete replacement*
     for on-disk config inside OpenCode, and the merge strategy between
     that env-supplied config and the `--model` CLI flag has a
     precedence bug or a stale cache.
   - The `opencode/claude-haiku-4-5` namespace is rejected by the
     installed `opencode-gemini-auth` plugin registered in the on-disk
     `config.json`, causing OpenCode to silently fall back to the
     config's default `model` value instead of erroring on the CLI
     flag.
3. **Unverified — argv reordering.** `apply_entrypoint` inserts `run`
   at index 0 (`profile.rs:1410-1418`) and the harness re-assembles
   argv before `prompt_delivery` (`composition.rs:503-507`). Static
   reading shows `--model` lands before `--`, but this has not been
   captured from a live `strace`/`ps` of the spawned child.

**Root cause for the model override is not pinned in this pass.** The
confirmed finding is only that the offending string lives in the
user's on-disk config; the mechanism by which it wins over the
explicit CLI flag is still open.

### Decision

Given the static audit shows `part.text` is already extracted and
forwarded correctly, the spec's three proposed fixes (parser fix, sink
wiring fix, flush fix) do not match the current HEAD state — they
appear to describe issues that existed in an earlier revision. The
regression captured in `opencode-*.jsonl` predates recent commits
(notably `0bb985c5 feat(claudine): add typed protocol models and
resolved_* helpers` and `c5e2e17f refactor(claudine): suppress stderr
noise from provider extensions`).

**Scope of Task 2c.1 (explicit):**

- **In scope:** Add a fixture-driven regression test that feeds
  `opencode-yolo.jsonl` (converted to NDJSON) through the OpenCode
  parser and asserts at least one `SemanticEvent::OutputText` carrying
  the expected `part.text` is emitted. The captured fixtures are
  pretty-printed for readability; Task 2c.1 will re-emit them as
  NDJSON via `jq -c . < opencode-yolo.jsonl` (or a serde round-trip)
  before using them in tests.
- **NOT in scope for 2c.1 — deferred to Task 2c.1b (to be filed):**
  Root-causing and fixing the `--model` / on-disk-config / 
  `OPENCODE_CONFIG_CONTENT` interaction that produces the empty-stdout
  symptom in the field. Task 2c.1 must root-cause the model-override
  path before any fix ships, and that isolation work is a separate
  engineering pass from the regression-test task.

This is nearest to the "Sink wiring fix" bucket in the spec — the
pipeline works end-to-end on synthetic input; the live drop is a
configuration-path issue upstream of the parser rather than a
`with_output_text_sink` wiring miss. Record as **Sink wiring fix**
(adjusted scope: the wiring is already correct; the deliverable is
the fixture-locked regression test). The separate model-override
investigation is tracked out of this task so 2c.1's deliverable does
not silently double in scope.

**Status update (2026-04-15):** The parser-level regression test is in place. The model-override root cause (Task 2c.1b) remains open — the empty-stdout symptom is a configuration-path issue (on-disk OpenCode config overriding `--model`), not a stream-pipeline bug. This investigation trail should be considered **partially closed**: the stream pipeline is verified correct; the remaining fix belongs to the OpenCode model-selection work, not the response-refinement feature.

### Files referenced

- `claudine/lib/src/stream/protocol/opencode.rs:81-103, 395-420`
- `claudine/lib/src/stream/opencode_semantic.rs:196-208, 399-405`
- `claudine/cli/src/commands/wrap/live_semantic_sink.rs:99, 229-232, 503-517`
- `claudine/cli/src/commands/wrap/mod.rs:1138-1147, 1269-1308`
- `claudine/cli/src/commands/wrap/exec.rs:77-114, 172-183, 1043-1121`
- `claudine/cli/src/commands/wrap/profile.rs:1347-1541`
- Fixtures: `claudine/features/2026-04-14-response-refinement/opencode-yolo.jsonl`,
  `…/opencode-not-yolo.jsonl`
- Live reproduction logs: `/tmp/opencode-stdout.log` (empty),
  `/tmp/oc2-stderr.log` (OpenCode `ProviderModelNotFoundError`),
  `/tmp/opencode-raw.jsonl` (native NDJSON, 3 events including one
  `part.text`).

## 0b — OpenCode Mis-Routed Render (`⚙ firecrawl_firecrawl_search {…JSON…}`)

### Finding: the `⚙` line is not rendered by Claudine at all

The mis-routed stderr line is **OpenCode's own default-mode TUI formatter
output**, written by the `opencode` binary directly to stderr. Claudine's
sink never emits it, so there is no mis-route inside the parser/sink
pipeline. The reason the line reaches the user is that Claudine's
stderr noise-prefix filter for OpenCode does not cover the `⚙ `
(U+2699 + space) prefix, so the line passes through the stderr
passthrough layer unfiltered.

### Byte-level reproduction (direct, without Claudine)

Running OpenCode natively against a Firecrawl MCP prompt, default
format:

```
opencode run --model opencode/claude-haiku-4-5 \
  "search the web using firecrawl for: NFL draft 2026 date" \
  2>/tmp/oc-firecrawl-stderr.log 1>/tmp/oc-firecrawl-stdout.log
```

stderr contains (hex-dump of the relevant line):

```
1b 5b 30 6d                                          # ESC[0m
e2 9a 99                                             # ⚙ (U+2699)
20 1b 5b 30 6d                                       # " " ESC[0m
firecrawl_firecrawl_search {"query":"NFL draft 2026 date",
"limit":5,"sources":[{"type":"web"}]}\n
```

That matches the spec's captured line byte-for-byte (same tool name,
same raw JSON arguments including `"limit":5` which is absent from the
`opencode-yolo.jsonl` fixture — the spec capture was from a *default*-
mode run, not a `--format json` run). The `⚙` glyph is U+2699 rendered
literally by OpenCode's formatter; it is **not** the nerd-font Info
icon (`\u{f449}` a.k.a. `NERD_CIRCULAR_INFO` in
`biscuit-terminal/lib/src/components/status.rs:20`).

### Live Claudine reproduction is blocked by the Task 2c.1b model-override issue

```
RUST_LOG=claudine=trace cargo run -q -p claudine-cli -- \
  opencode --model opencode/claude-haiku-4-5 --use firecrawl -- \
  "search the web for: NFL draft 2026" \
  2>/tmp/opencode-mcp-trace.log 1>/tmp/oc-wrap-stdout.log
```

exits 0 with OpenCode erroring out with
`ProviderModelNotFoundError: openrouter/qwen3-coder` (the same
symptom documented under § 0a, Task 2c.1b). In this environment
OpenCode never actually executes the firecrawl tool under Claudine
wrap, so `/tmp/opencode-mcp-trace.log` contains no `⚙` line and no
`firecrawl_firecrawl_search` trace event. The live reproduction
through Claudine is therefore **blocked by 2c.1b** and must be
re-run after that is fixed. The direct reproduction above is
sufficient to pin the render path, however, because the line is
emitted by OpenCode itself — Claudine's role is only
passthrough/filter.

### Code-reading audit of every `⚙`-adjacent sink path (no match)

The grep for `⚙ | \u{2699} | StatusState::Info` across the sink and
`biscuit-terminal` returns zero direct `⚙` emitters in Claudine's
rendering code:

- `biscuit-terminal/lib/src/components/status.rs:20,47,53` — the
  `StatusState::Info` mapping uses `NERD_CIRCULAR_INFO = "\u{f449}"`
  (a nerd-font codepoint) with fallback `FB_INFO = "\u{2139}"` (ℹ).
  **Neither is `\u{2699}`.**
- `claudine/cli/src/commands/wrap/live_semantic_sink.rs:271-278`
  (`tool_call_description`) — always prefixes with `\u{2192}` (→) and
  uses `\u{00b7}` (·) as separator between tool name and summary. Raw
  JSON is never appended: `summarize_input` at `live_semantic_sink.rs:
  552-580` prefers well-known keys (`query` is on the list) and falls
  back to the first non-empty string value, so with input
  `{"query":"NFL draft 2026 date", …}` it would produce `→
  firecrawl_firecrawl_search · NFL draft 2026 date`, not `⚙ … {raw
  JSON}`.
- `claudine/cli/src/commands/wrap/live_semantic_sink.rs:304-310`
  (`provider_extension_description`) — format is
  `<provider>/<kind>[ · <summary>]`; never includes a bare `⚙`, never
  dumps raw JSON (see `summarize_provider_payload` at
  `live_semantic_sink.rs:590-681`, which has a final "first non-empty
  string" fallback).
- `claudine/cli/src/commands/wrap/live_semantic_sink.rs:396-466`
  (`render_event` dispatch) — the only arms that use
  `StatusState::Info` are `FileChange`, `PlanUpdate`, `Info`, and
  `ProviderExtension`. None of those would produce the literal
  `<tool_name> <raw JSON>` shape, and they would all use the
  nerd-font Info icon `\u{f449}` (rendered by a Nerd Font as a
  different glyph than ⚙), not U+2699.

Every emission from Claudine's sink goes through `Status::new(desc).
state(state).render(&wrap_terminal())` at `live_semantic_sink.rs:
264-269`, which always renders the configured Info icon (`\u{f449}` or
`\u{2139}`) — never `\u{2699}`.

### Where the line actually comes from

`claudine/cli/src/commands/wrap/profile.rs:1552-1559` — the noise
filter for OpenCode's default-mode stderr formatter:

```rust
pub(crate) fn opencode_default_tui_noise_prefixes() -> &'static [&'static str] {
    &[
        "\u{2731} ",                          // ✱  — bullet used for Glob/Grep/Read status lines
        "$ ",                                 // bare shell command echo lines
        "> build ",                           // session banner
        "\u{2588}\u{2588}\u{2588}\u{2588} ", // ████  — subheader marker
    ]
}
```

This list is missing `"\u{2699} "` (⚙ + space), the prefix OpenCode
uses for MCP tool-invocation lines in default-format mode. Because the
passthrough filter at `exec.rs:402,787,1124` is a strict prefix match,
the `⚙ firecrawl_firecrawl_search {…}` line is forwarded verbatim from
the child's stderr to the user's stderr. That is the sole "mis-route"
— there is no code path inside Claudine's semantic pipeline that
produces this line.

### Secondary observation: why it shows up at all when `--format json`

When Claudine wraps OpenCode it passes `--format json`
(`profile.rs:1534-1537`). In the `--format json` code path OpenCode
still briefly writes some default-formatter lines to stderr before the
JSON stream on stdout kicks in — the existing noise-prefix list
confirms this is expected behaviour. The existing list suppresses
`✱ `, `$ `, `> build `, and `████ ` lines, which is exactly the shape
of the stderr we captured above aside from the `⚙ ` tool-call line. So
the fix is additive, not a rewrite: add one more prefix.

### What Phase 2c.4 needs to change

Add `"\u{2699} "` (⚙ + space) to
`opencode_default_tui_noise_prefixes()` at
`claudine/cli/src/commands/wrap/profile.rs:1552-1559`, and extend
`opencode_noise_prefixes_cover_captured_symptoms` at
`profile.rs:1995` with the captured `⚙ firecrawl_firecrawl_search {…}`
line so this stays locked. The pipeline-level assertion sketched in
the plan (`assert!(!rendered.contains('\u{2699}'))` on every rendered
sink line) is a useful additional guard but does **not** fix the
passthrough; the prefix-list entry is the load-bearing change.

### Files referenced

- `claudine/cli/src/commands/wrap/profile.rs:1552-1559`
  (`opencode_default_tui_noise_prefixes`),
  `profile.rs:1995-2018`
  (`opencode_noise_prefixes_cover_captured_symptoms`,
  `opencode_profile_advertises_default_tui_noise_prefixes`),
  `profile.rs:1539-1541`
  (OpenCode's `stderr_noise_prefixes` wiring).
- `claudine/cli/src/commands/wrap/live_semantic_sink.rs:264-310,
  391-478, 552-580, 590-681` — audited; none of these emit `\u{2699}`.
- `biscuit-terminal/lib/src/components/status.rs:13-55` — Info icon
  is `\u{f449}` / `\u{2139}`, not `\u{2699}`.
- `claudine/cli/src/commands/wrap/exec.rs:402, 787, 1124` — stderr
  prefix-filter passthrough.
- Live captures:
  - `/tmp/oc-firecrawl-stderr.log` — direct `opencode run` default
    format, contains the exact `⚙ firecrawl_firecrawl_search {…}`
    line (hex bytes `e2 9a 99`).
  - `/tmp/oc-firecrawl-stdout.log` — direct run stdout (assistant
    text).
  - `/tmp/opencode-mcp-trace.log` — Claudine wrap trace; blocked by
    `ProviderModelNotFoundError` per § 0a / Task 2c.1b.

## 0c — Gemini Markdown List Truncation

### Repro command

```
cargo run -q -p claudine-cli -- gemini -- \
  "list the four NFL conferences in markdown bullet form" \
  > /tmp/gemini-stdout.txt
# Native NDJSON (source of the fixture slice):
gemini --output-format stream-json -p \
  "list the four NFL conferences in markdown bullet form" \
  > /tmp/gemini-raw.ndjson
```

### Live reproduction (confirmed)

Ran
`cargo run -q -p claudine-cli -- gemini -- "list the four NFL conferences in markdown bullet form"`
into `/tmp/gemini-stdout.txt`. stdout reproduces the spec symptom
exactly — paragraphs and list items are split across lines at chunk
boundaries:

```
The NFL actually consists of only **two** conferences, the AFC and the NFC. Each of these conferences
 is divided into **four** divisions:

* **North** (AFC North / NFC North)
* **South**
 (AFC South / NFC South)
* **East** (AFC East / NFC East)
* **West**
 (AFC West / NFC West)
```

The `\n (AFC South / NFC South)` and `\n (AFC West / NFC West)` are
the smoking gun: a bullet is split mid-item across two Gemini stream
chunks, and the continuation (" (AFC …)") renders on its own line
instead of concatenating into the bullet.

Also captured the raw underlying stream via native
`gemini --output-format stream-json -p "…" > /tmp/gemini-raw.ndjson`
(12 lines). Gemini emits each assistant-message chunk as a
`{"type":"message","role":"assistant","delta":true}` with `content`
holding a partial slice that straddles markdown structure:

- Chunk A ends with `"…subdivided into"` (no trailing newline, ends
  mid-sentence).
- Chunk B begins with `" **four divisions**.\n\n…\n* **National
  Football Conference"` (leading space joins the prior sentence, ends
  mid-bullet item after a bold run).
- Chunk C begins with `" (NFC)**\n\n…\n* **East**\n* **North**\n*"`
  (leading space completes the AFC bullet, ends with `"*"` — the
  unfinished bullet marker for the next item).
- Chunk D delivers `" **South**\n* **West**"`.

So every chunk boundary lands inside a markdown construct, not at a
logical break.

### Code audit findings

`claudine/lib/src/stream/gemini_semantic.rs:94-111`
(`handle_message`) emits **one** `SemanticEvent::OutputText` per
assistant `message` event. It does not buffer across events. Each chunk's
`content` is passed through `ensure_message_newline` and shipped as a
standalone OutputText, so the parser is faithfully propagating the
chunk boundaries the provider gave it.

`claudine/cli/src/commands/wrap/exec.rs:77-160` (`StreamTextRenderer`)
is where the visible truncation is produced:

- `push` (`exec.rs:77-114`) splits the incoming chunk on `\n` and then,
  at lines 108-113, writes any trailing partial (no-newline) content
  **raw to stdout** and sets `partial_line_committed = true`. That is
  the mechanism by which chunk A's tail (`"…conferences"` / `"…North
  Conference"`) gets emitted raw, and the next chunk's leading
  ` **divisions**.\n` or ` (NFC)**\n` arrives as a new "line" once the
  `\n` lands.
- `process_line` (`exec.rs:118-160`) recognises bullet lines via
  `is_stream_safe_list_item` (`exec.rs:200-205`) and calls
  `flush_block` **per bullet** (lines 151-156). That means every list
  item is rendered through Darkmatter as a standalone document rather
  than as part of a cohesive list, and any chunk whose continuation
  arrives *without* the leading `* ` marker (because the marker was in
  the prior chunk) renders as plain prose on its own line.
- `render_markdown` (`exec.rs:185-197`) calls
  `render_assistant_markdown_with_options(text, …)` for the flushed
  block. That helper invokes Darkmatter's terminal renderer on whatever
  text slice it is handed, with no continuation state. Darkmatter
  itself has no concept of "continue from the previous block"
  (`rg streaming|stream_text|stream_markdown darkmatter/lib/src`
  returns only `markdown/output/html.rs` unrelated). So a bullet
  rendered in isolation gets paragraph-like treatment, not list-item
  treatment.

Net: the visible breakage is produced **in Claudine's CLI**, not in
Gemini's parser and not inside Darkmatter. The parser emits exactly
what Gemini streamed; the renderer's per-line flushing and per-item
Darkmatter calls are what fragments the output.

### Decision

**Parser fix — buffer-until-logical-break in
`claudine/lib/src/stream/gemini_semantic.rs::handle_message`.**

Justification:

- The raw stream confirms the hypothesis from the spec: Gemini chunks
  split inside markdown structure (mid-sentence, mid-bullet, between
  bullet marker and text). Claudine's `StreamTextRenderer` is designed
  around complete lines and list items; it cannot reconstruct logical
  breaks from chunks that do not respect them.
- A renderer fix would require teaching Darkmatter a streaming
  "continuation" mode and threading render state across calls. That is
  a much larger change, touches a library that is used outside the
  Gemini path, and does not fix the raw-partial-line committal in
  `StreamTextRenderer::push` (which fires before Darkmatter is even
  called).
- Buffering inside `handle_message` keeps the streaming contract at
  the parser boundary — by the time `SemanticEvent::OutputText` is
  emitted, the text slice ends at a logical break (blank line or turn
  end; see flush rules and code-fence note below for why those two
  suffice). The renderer then sees the same shape it already gets
  from Claude/Codex/OpenCode, and `flush_block` /
  `is_stream_safe_list_item` work correctly.
- The `delta: true` flag on Gemini's streaming chunks is already
  captured in `GeminiMessage::delta`
  (`claudine/lib/src/stream/protocol/gemini.rs:41-49`), so the parser
  has the signal it needs to buffer vs. flush without guessing.
- The `result` terminal event (raw-kind `result`) is a safe "flush
  anything buffered" point.

Flush rules Task 2b.1 should implement (spec-level, not code):

1. **Streaming (`delta: true`) only** — append `content` to an internal
   `pending_text` buffer.
2. Find the last index of a **blank-line** break (`"\n\n"`) in
   `pending_text`. Emit `pending_text[..=flush_idx]` as OutputText and
   retain the tail. (Code-fence integrity is intentionally **not**
   tracked at the parser — see note below.)
3. On `result` (turn end) or `error`, flush any remaining
   `pending_text` verbatim as a final OutputText before emitting the
   terminal event.

**Non-delta (single-shot) messages bypass the buffer entirely.** When
`handle_message` is called with `delta: false` (or the flag absent),
emit `SemanticEvent::OutputText` for that message's content
immediately and do not touch `pending_text` — those messages already
arrive as whole logical blocks and must not be held back waiting for
a blank-line break that may never come.

These flush rules **supersede** the `"\n\n"`-only sketch in
plan.md:1263; Task 2b.1 should implement the rules in this section
(blank-line break for deltas, immediate emit for non-deltas, flush
on terminal event) rather than plan.md's shorter sketch.

**Code-fence handling — deferred to the renderer.** An earlier draft of
this section also listed "newline at end of a fenced code block" as a
flush trigger. That rule would require the parser to track fence state
(count of unclosed ```` ``` ```` runs across buffered chunks), which
today lives only in `StreamTextRenderer`. Duplicating that state in
the parser is more complex and more failure-prone than letting the
renderer continue to own fence integrity. Task 2b.1 therefore keeps
the parser's flush condition to blank-line-or-terminal-event only
(rule 2 above), and relies on `StreamTextRenderer` to handle code
fences as it already does. If a future Gemini regression proves the
renderer cannot reconstruct fences from blank-line-delimited
OutputText chunks, revisit this decision in a follow-up.

**Note on `partial_line_committed`.** `StreamTextRenderer::push`
(`exec.rs:108-113`) sets `partial_line_committed = true` whenever it
writes a no-newline tail to stdout. That flag is useful progress
feedback for other providers (Claude, Codex, OpenCode) whose chunks
are already whole lines and where an intra-line pause is meaningful.
Task 2b.1 must **not** disable it globally. The parser-level buffering
introduced here only reduces how often Gemini triggers the partial
branch (because most chunks will already end at `"\n\n"` by the time
they reach the renderer); other providers' behaviour is unchanged.

### Files referenced

- `claudine/lib/src/stream/gemini_semantic.rs:94-111`
  (`handle_message`), `:282-328` (`feed_line` dispatch).
- `claudine/lib/src/stream/protocol/gemini.rs:41-74` (`GeminiMessage`
  + `GeminiMessageContent`, including `delta` capture).
- `claudine/cli/src/commands/wrap/exec.rs:77-160, 200-205`
  (`StreamTextRenderer::push`, `process_line`, `flush_block`,
  `is_stream_safe_list_item`).
- `claudine/cli/src/commands/wrap/exec.rs:185-197`
  (`render_markdown` → `render_assistant_markdown_with_options`).
- Darkmatter audit (negative): `rg 'streaming|stream_text|stream_markdown'
  darkmatter/lib/src` finds only unrelated HTML output code; no
  streaming-continuation primitive exists.
- Fixture: `claudine/lib/tests/fixtures/providers/gemini-markdown-list.ndjson`
  (7-line captured slice of a live run: init + user echo + four
  assistant-message chunks with `delta:true` straddling markdown
  structure + terminal result). Captured live via
  `gemini --output-format stream-json -p …`, not hand-constructed.
- Live reproduction logs: `/tmp/gemini-stdout.txt` (wrapped run,
  visible truncation), `/tmp/gemini-raw.ndjson` (native NDJSON,
  source of the fixture slice).

## 0d — Hard-Coded Truncation Cap Location

### Summary text truncation caps in LiveSemanticSink

The sink rendering pipeline caps summary text extracted from tool inputs and provider-extension payloads at two sites, both in `claudine/cli/src/commands/wrap/live_semantic_sink.rs`:

**`summarize_input` (lines 552–580):** Extracts a summary from tool-call JSON input and caps at **60 characters**. Three call sites within the function:
- Line 554: string value at input root level
- Line 568: string value from well-known key (command, path, file_path, etc.)
- Line 575: first non-empty string value (last-resort fallback)

**`summarize_provider_payload` (lines 590–681):** Extracts summary from provider-extension event payloads and caps at **80 characters**. Four call sites within the function:
- Line 623: string value from known nested path (message, status, name, error.message, etc.)
- Line 656: string value from nested content array (message.content[*].text, etc.)
- Line 663: string value from nested content array (item.content.parts[*].text, etc.)
- Line 675: first non-empty top-level string value (last-resort fallback)

**Helper function:** `truncate(s: &str, max_chars: usize) -> String` (line 704)
- Counts UTF-8 characters, truncates to `max_chars - 1`, appends ellipsis (U+2026, `…`)
- Used **only** by `summarize_input` and `summarize_provider_payload` in production code
- Test functions in the same module do not depend on the cap value

### Secondary caps audit: biscuit-terminal

Searched `biscuit-terminal/lib/src/components/{status,prose}.rs` and the entire `biscuit-terminal/lib/src/` for hardcoded character limits via `truncate`, `.chars().take(N)`, `.split_at()`, and `const MAX_` patterns.

**Finding:** No secondary character-limit caps found.
- Line 1457 in `prose.rs` (`inner_content.truncate(start)`) is vector-truncation (content manipulation), not a character cap.
- Constants `MAX_BYTES` (64) and `MAX_ITERATIONS` (100) in `fonts.rs` are unrelated to text rendering.
- Status and Prose components rely on caller-provided text; no internal caps are imposed.
- Layout wrapping defers entirely to terminal width negotiation.

### Scope confirmation for Phase 1, Task 1.6

Task 1.6 ("Remove the hardcoded truncation cap") must remove both the `summarize_input` and `summarize_provider_payload` caps (lines 554, 568, 575, 623, 656, 663, 675) and defer text wrapping to `biscuit-terminal`'s Layout engine, which has terminal-aware column budgets. No downstream cap removal is required in biscuit-terminal; the entire inventory is upstream in the sink's summary extractors.

## 0e — Codex Tool-Event Field Extraction

### Live trace captured

Ran native `codex exec --json "list files in the current directory, briefly"`
> `/tmp/codex-raw.ndjson` (7 lines) and wrapped
`cargo run -q -p claudine-cli -- codex -- "list files in this directory"`
> `/tmp/codex-stdout.log` / `/tmp/codex-trace.log`. The native capture
contains the three Codex tool/message shapes actually emitted by the
live binary in a plain tool-using session:

- `agent_message` (twice — pre- and post-tool narration)
- `command_execution` (`item.started` + `item.completed`)
- `turn.completed` usage

No `tool_use` / `tool_call` / `mcp_tool_call` / `web_search` /
`patch_apply` / `image_generation` / `view_image` items fired in this
session, so the audit for those variants is code-reading-only. The
live capture confirms one important thing: real Codex
`command_execution` items **do** populate `command`, `aggregated_output`,
`exit_code`, and `status` on both `item.started` and `item.completed`
envelopes, exactly as the existing fixture tests
(`command_execution_status_and_exit_code_preserved`,
`codex_fixture_command_execution_routes_to_tool_pair`) assert.

### Item-type inventory and field gaps

The typed protocol (`claudine/lib/src/stream/protocol/codex.rs:171-193`)
enumerates eight tool-bearing `CodexItem` variants that all share
`CodexToolItemFields` (`codex.rs:443-484`):

```
ToolUse | ToolCall | McpToolCall | WebSearch | CommandExec
  | PatchApply | ImageGeneration | ViewImage
```

plus four non-tool item types the sink dispatches separately
(`PermissionRequest`, `ApprovalRequest`, `UserInputRequest`,
`Reasoning`, `FileChange`, `PlanUpdate`, `TodoList`, `AgentMessage`,
`Unknown`). Only the eight tool-bearing variants route through
`handle_item_started` / `handle_item_completed` into
`SemanticEvent::ToolCall` / `SemanticEvent::ToolResult` via
`tool_call_from_fields` (`codex_semantic.rs:257-271`) and
`tool_result_from_fields` (`codex_semantic.rs:273-295`).

`CodexToolItemFields` carries 13 optional fields:

```
id, name, tool_name, input, arguments, parameters,
output, result, content, status, exit_code, command, aggregated_output
```

with three accessors that fold several of those into a canonical form:

- `resolved_tool_name()` → `tool_name` | `name` | `"shell"` (if
  `command` is set) (`codex.rs:487-492`)
- `resolved_input()` → `input` | `arguments` | `parameters` |
  `{"command": command}` (`codex.rs:498-512`)
- `resolved_output()` → `output` | `result` | `content` |
  `Value::String(aggregated_output)` (`codex.rs:514-528`)

#### Per-variant table (`item.started` → `ToolCall`, `item.completed` → `ToolResult`)

All eight tool variants share one code path, so the field-level
analysis is identical across them. The meaningful axis is whether the
variant's payload tends to populate `input`/`arguments`/`parameters`
(the generic tool shape) or `command`/`aggregated_output` (the shell
shape).

| `item.type`        | Raw Codex fields observed (from code + live trace) | Fields populated on `ToolCall` | Fields populated on `ToolResult` | Gaps vs. raw |
| ------------------ | --- | --- | --- | --- |
| `tool_use`         | `id`, `name`/`tool_name`, `input`/`arguments`/`parameters`, `output`/`result`/`content` | `name`, `id`, `input`, `extra.tool_id`, `extra.tool_name`, `extra.raw_kind`, `extra.semantic_kind`, `extra.session_id` | `name`, `id`, `status` (nil here), `exit_code` (nil here), `output`, `extra.{tool_id,tool_name,status,exit_code,raw_kind,…}` | None at the `SemanticEvent` field level for this shape — the accessors cover every raw field. |
| `tool_call`        | same as `tool_use` | same | same | same |
| `mcp_tool_call`    | same as `tool_use` plus MCP-specific nested server/tool metadata that Codex currently folds into `input` | same | same | MCP server name (if Codex ever emits a separate `server_name` / `server.name` alongside `tool_name`) would be dropped — **no such field exists on `CodexToolItemFields` today**. Recommend: once 2d.1 is reviewing this variant, add `server_name: Option<String>` and re-emit under `extra.mcp_server` if and only if a real Codex build is seen emitting it; do not speculate-add. |
| `web_search`       | `id`, `name`=`"web_search"`, `input={query: …}`, `output` | same | same | None at the field level. |
| `command_execution` / `command_exec` | `id`, `command`, `aggregated_output`, `exit_code`, `status` (live trace: `status="in_progress"` on started, `status="completed"` on completed) | `name="shell"` (synthesized), `id`, `input={"command": …}`, `extra.tool_name="shell"`, `extra.tool_id` | `name="shell"`, `id`, `status`, `exit_code`, `output=Value::String(aggregated_output)`, `extra.status`, `extra.exit_code` | **One gap:** the live trace's `item.started` carries `status="in_progress"` but `tool_call_from_fields` does NOT copy `status` into `extra` (only `tool_result_from_fields` does, at `codex_semantic.rs:281-283`). A "started" `ToolCall` therefore loses the transient `status="in_progress"` signal. Recommend: add `if let Some(status) = &fields.status { extra.insert("status".into(), Value::from(status.as_str())); }` to `tool_call_from_fields` symmetrically with the result side, so consumers can distinguish a started-but-unfinished tool call from a finalised one using only extras. Low priority — the paired `ToolResult` always arrives before rendering completes. |
| `patch_apply`      | `id`, `name`=`"apply_patch"` / `"patch"` (build-dependent), `input` carrying the patch body, `output`/`result`, plus (per Codex source) optional `status` / `exit_code` | same as `tool_use` | same | Same as `command_execution`: the `status` field on the started side is not mirrored into `ToolCall.extra`. Additionally, no dedicated `patch`/`diff` fields exist in `CodexToolItemFields` — if a future Codex build splits the patch payload out of `input` into a top-level `patch` / `diff`, it will be silently dropped. Recommend: keep the generic fallthrough for now; revisit only if a real trace shows the split. |
| `image_generation` | `id`, `name`=`"image_generation"`, `input={prompt: …, size: …}`, `output` (url or base64) | same as `tool_use` | same | None at the field level. The `size` / `quality` sub-fields of `input` survive inside `resolved_input()` as a `Value::Object` and the renderer is free to dig into them. |
| `view_image`       | `id`, `name`=`"view_image"`, `input={path: …}` or `{url: …}`, `output` | same as `tool_use` | same | None at the field level. |

#### Gaps that cross all variants

Three field classes are dropped unconditionally by the semantic layer,
regardless of which tool variant fired:

1. **`arguments` vs. `parameters` vs. `input` provenance is
   flattened.** `resolved_input` returns only the first non-empty value
   in that precedence chain (`codex.rs:498-512`); callers cannot tell
   which of the three keys Codex used. Impact on 2d.1 is nil (the
   renderer only needs a `Value`), but note it here so future
   fidelity work does not re-audit this — add a breadcrumb in
   `extra.input_field = "input" | "arguments" | "parameters" | "command"`
   only if a downstream requirement emerges.

2. **`output` / `result` / `content` provenance is flattened** in
   exactly the same way (`codex.rs:514-528`). Same note applies:
   low-priority, leave alone until 2d.1 surfaces a concrete need.

3. **The raw `status` string on `item.started` tool calls is
   dropped.** `tool_call_from_fields` (`codex_semantic.rs:257-271`)
   copies `tool_id`, `tool_name` into `extra` but never `status`. The
   corresponding `tool_result_from_fields` DOES copy it
   (`codex_semantic.rs:281-283`). Recommend 2d.1 make these two
   symmetric:

   ```rust
   // in tool_call_from_fields, right after the tool_name insert:
   if let Some(status) = &fields.status {
       extra.insert("status".into(), Value::from(status.as_str()));
   }
   ```

   Rationale: the live trace shows `command_execution` started with
   `status="in_progress"` — a useful signal for the `ToolCallDisplay`
   to style an in-flight call differently from a finalised one once
   Task 1.4 lands. This is the single concrete, low-risk field fix
   the audit surfaces.

#### Why the user still sees `→ (tool)` / `← (tool)`

The spec symptom is `→ (tool)` / `← (tool)` — i.e. `name_part` falling
back to the `"(tool)"` literal in `tool_call_description`
(`claudine/cli/src/commands/wrap/live_semantic_sink.rs:271-278, 280-297`).
That only fires when `SemanticEvent::ToolCall.name` is `None`, which
in turn requires `resolved_tool_name()` to return `None` — only
possible when all three of `tool_name`, `name`, and `command` are
absent on the merged (started + completed) `CodexToolItemFields`.

**This audit did not reproduce that symptom from a live Codex session.**
The captured `command_execution` items all carry `command`, so
`resolved_tool_name()` returns `Some("shell")` and the renderer would
have shown `→ shell · /bin/zsh -lc 'ls -1A'`. The observed
`→ (tool)` rendering in the spec therefore corresponds to one of:

- A Codex item variant (most likely `patch_apply` or `image_generation`)
  where a specific build omitted the `name` / `tool_name` fields and
  did not provide a `command` fallback. No fixture for that case
  exists in the repo today.
- A hypothetical built-in tool variant that the typed enum currently
  routes to `Unknown` (provider-extension path), which would not
  render as `→ (tool)` at all — it would render as
  `codex/item.{started,completed}` via
  `provider_extension_description`.

Recommend that Phase 2d.1 additionally:

- **Widen the live-signal coverage before fixing.** Capture a live
  Codex trace that exercises `patch_apply` (e.g. "write a one-line
  patch to `README.md`") and re-audit the merged field shape against
  this table. If `name`/`tool_name` really are empty on some variant,
  add a per-variant default in `resolved_tool_name` (e.g.
  `CodexItem::PatchApply` → `"apply_patch"`) instead of letting the
  renderer fall back to the bare `"(tool)"` literal.
- **Make `tool_call_from_fields` / `tool_result_from_fields`
  symmetric on `status`** as above.
- **Leave `resolved_input` / `resolved_output` alone.** The accessors
  already cover every field on `CodexToolItemFields` and the live
  trace confirms the shape is sound.

### Files referenced

- `claudine/lib/src/stream/protocol/codex.rs:150-193, 443-555`
  (`CodexItem` enum, `CodexToolItemFields`, `resolved_*`,
  `merge_started`).
- `claudine/lib/src/stream/codex_semantic.rs:252-406`
  (`handle_permission_item`, `tool_call_from_fields`,
  `tool_result_from_fields`, `handle_item_started`,
  `handle_item_completed`, `handle_item_updated`).
- `claudine/lib/src/stream/codex_semantic.rs:880-931`
  (`round_trip_fidelity_mixed_fixture`,
  `codex_fixture_command_execution_routes_to_tool_pair`) — existing
  evidence of what the parser preserves today.
- `claudine/lib/src/stream/semantic.rs:55-68` (`SemanticEvent::ToolCall`
  and `SemanticEvent::ToolResult` shapes — bound the field surface).
- `claudine/cli/src/commands/wrap/live_semantic_sink.rs:271-297`
  (`tool_call_description` / `tool_result_description` — source of
  the `→ (tool)` / `← (tool)` fallback string).
- Live captures:
  - `/tmp/codex-raw.ndjson` — 7-line native `codex exec --json` trace
    (thread/turn/agent_message/command_execution pair/turn.completed).
  - `/tmp/codex-trace.log` — wrapped-session stderr (Claudine status
    lines, not raw NDJSON).
  - `/tmp/codex-stdout.log` — wrapped-session stdout (rendered
    assistant markdown).
