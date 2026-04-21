---
title: Link Rendering & Read Error Presentation
date: 2026-04-17
status: in-progress
---

# Link Rendering & Read Error Presentation

## Goal

Tighten the non-interactive stderr surface when providers report:

1. Long absolute paths inside tool call / result slots
2. `Read` failures (and eventually other file-tool failures)
3. Redundant `task_progress · Reading <file>` lines that duplicate the
   following `Read(...)` tool call

The current rendering of these three cases is verbose, visually jumbled,
and does not take advantage of OSC8 hyperlink support that already exists
in `biscuit-terminal`'s `Prose` component.

## Problems in current output

```
 → Read(/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/cli/src/commands.rs)
 claude/system/task_progress · Reading darkmatter/cli/src/args.rs
 → Read(/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/cli/src/args.rs)
 ← Read(error File content (30485 tokens) exceeds maximum allowed tokens (25000). Use offset and limit parameters to read
    specific portions of the file, or search for specif…)
 ← Read(successful)
```

1. Absolute paths dominate the line; the interesting tail is hidden.
2. `task_progress` line is immediately superseded by the matching `→ Read`
   tool call.
3. `Read(error …)` stuffs the entire error into the parenthesised slot,
   losing the dedicated `Status::Warning` glyph path and the BlockQuote
   border style the rest of the sink uses for diagnostics.
4. Success/error variants do not identify the file that was read; users
   have to back-scroll to match the arrow pair.

## Design

### 1. Path link helper

Add `format_file_link(raw: &str, cwd: &Path, home: Option<&Path>) -> String`
in a new file `claudine/lib/src/stream/path_link.rs`, re-exported from
`stream::mod`.

Behaviour:

- Returns prose markup, always. Caller is free to wrap with extra
  styling (e.g. `<dim>…</dim>`).
- If `raw` is not an absolute path, return `escape_prose(raw)` unchanged.
- Canonicalise `cwd` and (if present) `home` once via `Path::strip_prefix`
  semantics on the *lexical* form — no `fs::canonicalize` to keep the
  helper hot-path friendly.
- If `raw` starts with `cwd`: emit
  `<blue><a href="{raw}">{rel_to_cwd}</a></blue>` where `rel_to_cwd`
  has no leading `./` or `/`.
- Else if `raw` starts with `home`: emit
  `<blue><a href="{raw}">~/{rel_to_home}</a></blue>`.
- Otherwise: return `escape_prose(raw)`.

Escaping: the `href` value is a URI, so angle brackets and braces in
file names must be percent-encoded. Do that with a tiny inline helper
that only escapes `<`, `>`, `{`, `\`, `"`; the visible text still goes
through `escape_prose` so Prose markup remains safe.

Unit tests (runtime-only) cover:

- Path inside cwd → `href=abs`, visible = `<cwd-relative>`.
- Path inside home but not cwd → `~/...`.
- Path outside both → escaped literal.
- Non-absolute passthrough.
- Paths containing special characters (`<`, spaces, percent signs).

### 2. `render_tool_display` file-tool awareness

In `live_semantic_sink.rs`, extend `render_tool_display` so that when the
tool is one of the known file tools (`Read`, `Write`, `Edit`, `NotebookEdit`,
`read_file`, `write_file`, `replace_file_content`), the summary string
is run through `format_file_link(raw, cwd, home)` rather than
`<dim><i>escape_prose(summary)</i></dim>`.

Data flow:

- `ToolCallDisplay` already carries the raw summary. No change there.
- `LiveSemanticSink` already holds `cwd`; add a `home` field populated
  from `dirs::home_dir()` at construction. Keep as `Option<PathBuf>`.
- Add a helper on the sink: `render_tool_slot(&self, tool_name, summary)
  -> Option<String>`. For file tools it returns the linked form wrapped
  in `<dim>…</dim>` (italic omitted because OSC8 + italic reads poorly
  in several terminals). For other tools it returns the existing
  `<dim><i>…</i></dim>` wrap.

Outgoing tool calls now look like:

```
 → Read(claudine/darkmatter/cli/src/args.rs)
```

where the visible text is an OSC8 link to the absolute path.

### 3. `Read`/file-tool error BlockQuote

Today `render_event` routes every `ToolResult` through
`render_status_prose(... StatusState::ToolUse, desc)` regardless of
status. Split the path:

- On `ToolStatus::Error` **and** a file-tool name:
  1. Render the header via `Status` with `StatusState::Warning`:
     ```
     ← Read(<red>error</red>, <dim>claudine/darkmatter/cli/src/args.rs</dim>)
     ```
     (The path uses the same `format_file_link` helper so the visible
     short form remains OSC8-linked.)
  2. Render an orange-bordered `BlockQuote` below with
     `error_detail` verbatim. Do not locally truncate — `error_detail`
     is already capped at 160 chars in `extract_error_detail`; if the
     upstream provider truncated it, the trailing ellipsis will carry
     through and the user sees `specif…`.
  3. Border glyph: `▌ ` (matches `render_tracing_diagnostic` and the
     thinking-block style; orange colour reserved for recoverable
     warnings).
- On `ToolStatus::Error` for non-file tools: current behaviour keeps the
  inline slot — this matches the short shell-error rendering (exit=1 ·
  sed: …) that already works well.
- On `ToolStatus::Success`: slot becomes
  `<dim><i>successful</i></dim>, <dim>{path-link}</dim>` when the file
  tool path is known. Non-file tools remain `<dim><i>successful</i></dim>`.

Refactor moves: introduce `render_file_tool_error(&self, section,
display, path_summary)` and `render_file_tool_success(&self, section,
display, path_summary)` to keep `render_event` legible. These call a
shared `render_warning_block(section, header_prose, body_text,
border_color)` helper (factor out the existing `render_tracing_diagnostic`
body).

### 4. `task_progress` redundancy suppression

Current path: `claude_semantic.rs` converts `task_progress` to
`SemanticEvent::Info`. `live_semantic_sink::render_event` renders `Info`
immediately via `render_status(Info, msg)`.

New behaviour (Claude-only, gated by `self.provider == Provider::Claude`
**and** `extra["raw_kind"] == "task_progress"` so unrelated Info events
are not delayed):

1. Hold buffered `task_progress` Info in a new `Option<String>` on the
   sink.
2. On each subsequent event, before rendering, consult the buffer:
   - If the next event is `SemanticEvent::ToolCall` whose extracted
     summary file-path tail equals the buffered message's tail (after
     stripping leading verb, e.g. `Reading `/`Running `/`Writing `),
     drop the buffer silently.
   - Otherwise flush the buffered Info through `render_status(Info, ...)`
     first, then process the new event.
3. On `TurnComplete` / `SessionEnd` / sink drop: flush any remaining
   buffered Info.

Matching rule:

- Buffer key: strip leading `Reading `, `Running `, `Writing `,
  `Editing `, `Searching `, `Fetching `, `Listing `; take the
  remainder.
- Tool-call key: the file path for file tools; the command string (with
  shell prefix stripped) for shell tools; the pattern for Glob/Grep.
- Match: substring containment in either direction with a minimum-10
  character shared tail so unrelated lines do not accidentally cancel
  each other.

Edge cases:

- Two `task_progress` lines arriving back-to-back with no intervening
  tool call: flush the older one, buffer the newer.
- Explicit `--verbose` or `--debug`: bypass suppression (keep the
  task_progress visible so operators can trace provider behaviour). Wire
  via the existing `verbosity` gate on the sink.

### Non-goals

- No changes to `ToolCallDisplay` itself — all rendering-side only.
- No change to JSONL log output. Every event still flows through
  `emit_event_log` and the dispatch pipeline exactly as before.
- No cross-provider coverage of #4 yet; other providers do not emit a
  `task_progress` shape. Revisit if Codex / Gemini start producing an
  equivalent pre-tool progress line.

## Testing

Runtime unit tests in:

- `claudine/lib/src/stream/path_link.rs` (new module)
- `claudine/cli/src/commands/wrap/live_semantic_sink.rs` (existing
  `tests` mod) — add:
  - `read_success_renders_path_link_in_slot`
  - `read_error_emits_warning_header_and_blockquote`
  - `task_progress_is_suppressed_when_followed_by_matching_read`
  - `task_progress_flushed_when_followed_by_unrelated_tool_call`
  - `verbose_mode_keeps_task_progress_visible`

Integration fixture replay (`tests/wrap_commands.rs`) gets one new case
replaying the transcript from the user message and asserting the
collapsed/formatted output.

## Implementation order

1. `format_file_link` + tests.
2. Plumb `home` into `LiveSemanticSink`.
3. Slot rewriting for file tools (success + call).
4. Error split with BlockQuote helper.
5. `task_progress` buffering.
6. Integration fixture.
7. `cargo fmt` + `just test` claudine area.
