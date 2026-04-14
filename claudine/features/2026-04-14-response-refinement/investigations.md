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

_Reproduce per Phase 0c; record whether the fix lives in the parser or in Darkmatter._

## 0d — Hard-Coded Truncation Cap Location

_Locate the cap removed during Child 1; record file:line and any tests pinning it._

## 0e — Codex Tool-Event Field Extraction

_Audit `codex_semantic.rs`; list which tool fields are dropped today._
