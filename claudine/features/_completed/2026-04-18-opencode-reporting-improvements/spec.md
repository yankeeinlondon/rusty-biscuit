# OpenCode Reporting — Specification

This specification describes three defects in Claudine's non-interactive stderr
output for the OpenCode provider and the contract that a fix must satisfy. No
code changes are made by this document; it is the input to a follow-up
implementation plan.

## Reference Output

The current output for a short OpenCode session is:

```text
󰀨 Skipped malformed OpenCode command: /Users/ken/.config/opencode/commands/catalog.md
󰀨 Skipped malformed OpenCode command: /Users/ken/.config/opencode/commands/homelab.md
[... five more skipped lines ...]

- OpenCode session ID ses_261302fb

 step_start


 ← Read(successful)
 step_finish
 step_start


 ← Read(successful)
 ← Read(successful)
 ← Read(successful)
 ← Read(error File not found: /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/features/2026-04-
    17-reporting-consistency/spec.md)
 step_finish
 step_start


 ← Bash(successful)
 step_finish
 step_start
 ← Bash(successful)
 step_finish
 step_start

I'll start by reading the lessons learned file [... assistant text ...]

 step_finish

✓ 50s · 17K input tokens · 2K output tokens · 267K cached tokens · $0.05 cost basis · 8 tool calls
⚠ Config — Skipped 7 malformed OpenCode assets
```

Three defects are visible:

1. Excess blank lines inside the Tool Use & Events section around `step_start`.
2. Successful tool results render without the filename or command that
   would give them meaning — `Read(successful)` and `Bash(successful)` with
   no slot content.
3. Malformed-asset notices are reported twice: once per line during the
   session and again as a single trailer warning.

## Defect 1 — Blank-Line Noise In The Tool Section

### Current behaviour

The OpenCode semantic parser emits `SemanticEvent::Info { message: "step_start" }`
and `SemanticEvent::Info { message: "step_finish" }` for every
OpenCode `step_start` / `step_finish` wire event
([`claudine/lib/src/stream/opencode_semantic.rs:108`][osem-start] and
[`:127`][osem-finish]). Both carry `extra["step_phase"] = "start"` /
`"finish"`.

The live sink renders every `SemanticEvent::Info` through `render_status` at
[`live_semantic_sink.rs:728`][lss-info], which emits a literal
` step_start` / ` step_finish` Status line tagged
`Section::ToolUseAndEvents`.

Two problems stack on top of each other:

- The phase markers are visual noise. `step_start` / `step_finish` are
  internal OpenCode phase boundaries that do not correspond to any user-
  visible semantic action. The user asked specifically that the output
  carry meaning for humans.
- Around the `step_start` line, one or two additional blank rows appear
  before the next `← Tool` line. The `SectionTracker` in
  [`section.rs`][section] already guarantees *"consecutive blank lines
  inside a section collapse to one"* but the noise survives because it
  originates between a non-blank `step_start` line and a non-blank tool
  line. Whatever is emitting that blank (trailing `\n` in the rendered
  Status, a scroll-guard from `StdoutWriter`, or an intermediate event
  such as an empty-text fragment) is not a blank row produced through
  `emit_section_line` — the tracker never sees it as blank, so it cannot
  dedupe it.

### Required behaviour

- The Tool Use & Events section MUST contain at most one blank line
  between any two adjacent rendered lines, and MUST NOT emit a blank
  line before the first rendered tool event or after the last one. This
  applies to every provider, not just OpenCode — the tracker contract
  in [`section.rs`][section] already states this rule; the OpenCode
  path currently violates it.
- The `step_start` and `step_finish` Info events MUST be suppressed
  from the rendered stderr surface. They SHOULD continue to flow
  through the JSONL event log and the `LiveMetrics` heartbeat so
  downstream reporting is unaffected.
- Suppression MUST be gated on `extra["step_phase"]` so unrelated Info
  events (both from OpenCode and from other providers) are not
  affected.
- After the phase markers are suppressed, any remaining blank rows
  around tool lines MUST be investigated — if there are still extra
  blanks between two real tool events, that is a separate leak and
  must be plugged so the `at most one blank` rule holds.

### Fix location

- [`claudine/cli/src/commands/wrap/live_semantic_sink.rs:728`][lss-info]
  — add a guard that returns without rendering when
  `provider == Provider::OpenCode` and `extra.get("step_phase")` is
  present. Alternative: filter at the OpenCode parser in
  [`opencode_semantic.rs:108`][osem-start] / [`:127`][osem-finish] by
  emitting an event variant that is JSONL-only. The sink-level guard
  is preferred because it keeps the parser lossless and matches the
  existing suppression patterns for malformed-line warnings and
  subscription rate-limit warnings in the same match arm.
- Secondary audit: once the phase markers are gone, run a fresh
  OpenCode session and verify the *"at most one blank line"* invariant
  empirically. If blanks remain, trace which code path is writing them
  outside the `SectionTracker`.

## Defect 2 — Scarcity Of Tool Metadata

### Current behaviour

Successful OpenCode tool results render as `← Read(successful)` and
`← Bash(successful)` with no filename or command. Errored reads do
render the path because the provider error message itself contains the
path (e.g. `File not found: <path>`), and the file-tool error renderer
preserves it. This asymmetry is not a wire-format limitation.

Two layers combine to strip the metadata:

**Layer A — OpenCode parser discards cached input on tool_result**

`handle_tool_use` at [`opencode_semantic.rs:245`][osem-use] caches
`(name, input)` in `self.tool_uses` keyed by tool id (line 263-264).

`handle_tool_result` at [`opencode_semantic.rs:316`][osem-result]
retrieves the cached pair via
`self.tool_uses.remove(id)` (line 321) but binds the input to
`_cached_input` (underscore-prefixed — discarded). The resulting
`SemanticEvent::ToolResult.extra` contains `tool_id`, `tool_name`,
`status`, and optional `error` — but never `input`.

Compare with `handle_tool_use_completed` at
[`opencode_semantic.rs:280`][osem-completed], which correctly populates
`extra["input"]` at line 302-304.

**Layer B — `ToolCallDisplay::from_result` drops summary when status is set**

[`tool_display.rs:554`][td-fromresult] has three branches:

```rust
let summary = if is_file_tool_name(raw_name) {
    extra.get("input").and_then(|v| extract_tool_summary(raw_name, v))
} else if parsed_status.is_some() {
    None                                       // <- non-file tools lose summary
} else {
    output.as_ref().or_else(|| extra.get("input"))
        .and_then(|v| extract_tool_summary(raw_name, v))
};
```

For file tools (`Read`/`Write`/`Edit`/`read_file`/…) the summary comes
from `extra["input"]` — but Layer A never populated that, so the
slot stays empty and the renderer produces `← Read(successful)`.

For shell tools (`Bash`/`bash`/`shell`/`run_command`) the `parsed_status.is_some()`
branch explicitly returns `None`, discarding any summary even when
`extra["input"]` would have been present. So `← Bash(successful)`
stays empty regardless of Layer A.

### Required behaviour

On a successful tool result emitted by OpenCode, the rendered stderr
line MUST show the same metadata that the outgoing tool call would
have shown:

- `← Read(successful, <cwd-relative OSC8 link>)` when the input
  carries a `file_path` or `path`.
- `← Bash(bash <command>)` or `← Bash(successful, bash <command>)`
  when the input carries a `command`. The chosen format MUST match
  the format used by the outgoing `→ Bash(bash <command>)` arrow so
  the pair reads symmetrically — see the existing contract described
  in [`non-interactive-sessions.md`][nis] and the
  `→ Name(summary)` / `← Name(slot)` rule reaffirmed in the
  2026-04-16 *more-is-more* fix.
- If the OpenCode stream genuinely provides neither an input nor an
  output that `extract_tool_summary` can handle, the renderer MUST
  fall back to the current `← Name(successful)` form rather than
  fabricating content.

The contract MUST also apply to other shell-shaped tools OpenCode
may route (`shell`, `run_command`, etc.) and to tools that are not
file-shaped (search / grep / list) when `extract_tool_summary`
already knows how to summarize them in
[`tool_display.rs`][td].

### Fix location

Two coordinated changes:

1. **OpenCode parser** — in `handle_tool_result` at
   [`opencode_semantic.rs:316`][osem-result], use the cached input
   retrieved from `self.tool_uses.remove(id)` to populate
   `extra["input"]` on the emitted `ToolResult`, matching the shape
   used by `handle_tool_use_completed` at
   [`opencode_semantic.rs:302`][osem-completed-input]. When the
   wire `tool_result` / `tool_end` payload also carries its own
   input (rare but possible), the wire value wins and the cached
   input is used only as a fallback, so we never overwrite fresher
   data.
2. **ToolCallDisplay::from_result** — in [`tool_display.rs:554`][td-fromresult],
   remove the early-return for non-file tools with a resolved status.
   Replace the three-branch match with a single lookup that attempts
   `extract_tool_summary(raw_name, extra["input"])` first and then
   falls back to `output` when no input-derived summary is available.
   `extract_tool_summary` already returns `None` gracefully for
   tool names it does not know, so existing tools that should
   continue to render slot-less (e.g. `task_progress`-synthesised
   entries) are not affected.

The fix MUST be covered by two regression tests:

- An OpenCode fixture-driven integration test in
  `claudine/lib/tests/semantic_fidelity.rs` (or a dedicated
  `opencode_tool_metadata.rs`) asserting that a `tool_start`
  carrying `input: {"command": "ls"}` followed by `tool_end` with
  `status: "success"` renders `← Bash(bash ls)`.
- A unit test on `ToolCallDisplay::from_result` asserting that
  a non-file tool with `status: "success"` and an `input` slot
  keeps its summary.

## Defect 3 — Double Reporting Of Malformed OpenCode Assets

### Current behaviour

OpenCode writes `--print-logs --log-level ERROR` records to stderr
when it fails to load a skill, command, or agent — malformed frontmatter,
missing files, etc. These records are captured by the OpenCode log
bridge in [`logs/opencode.rs`][logs-opencode] and emitted as
`SemanticEvent::Warning` (`on_malformed_asset` at
[`logs/opencode.rs:697`][logs-oc-warn]). The live sink renders each
warning as one `󰀨 Skipped malformed OpenCode command: <path>` line.

At the same time, each warning bumps `diagnostics.malformed_asset_events`
by one (line 706-707). After the stream ends, `derive_badges` in
[`badges.rs:227`][badges] reads that counter and emits a trailer badge:

```text
⚠ Config — Skipped 7 malformed OpenCode assets
```

The per-line warnings are already visible to the user and already carry
the paths. The trailer badge adds no new information — it only
re-states the count — yet it re-emphasises a condition the user has
already seen.

### Required behaviour

- Malformed-asset notices MUST be reported exactly once per session.
- The per-line `SemanticEvent::Warning` surface MUST be retained: it
  is the authoritative source because it carries the asset type
  (command / skill / agent) and the file path, and it lets reviewers
  jump directly to the offending file via OSC8.
- The trailer badge for malformed assets MUST be removed. Other
  trailer badges (rate-limit, API failure) continue to be emitted
  by the same function and are not affected by this change.

### Fix location

- Remove the malformed-asset badge emission at
  [`badges.rs:227`][badges] (the entire `if diagnostics.malformed_asset_events > 0`
  block). Update the two tests
  `stderr_diagnostics_malformed_assets_yields_config_badge` and
  `stderr_diagnostics_single_malformed_asset_uses_singular_noun`
  at [`badges.rs:610`][badges-test] to assert the badge is absent
  even when the counter is non-zero.
- Leave the counter itself intact — it is still observed by JSONL
  reporting and is useful for downstream dashboards; only the
  human-visible trailer line is removed.
- Add a regression test at the
  `live_semantic_sink` / `SummaryRenderer` integration boundary
  that runs a fixture with N malformed-asset records and asserts
  the final stderr contains exactly N `󰀨 Skipped malformed`
  lines and zero `Config — Skipped …` trailer lines.

## Non-Goals

- This document does not propose adding new metadata to the OpenCode
  wire stream. Every fix described here reuses data OpenCode already
  emits.
- Renaming existing tool glyphs, changing section order, or changing
  the trailer-line format for non-malformed-asset badges is
  out of scope.
- The same defect classes may exist for other providers (Gemini,
  Qwen, Kimi) in attenuated forms. Cross-provider audit is not
  covered here — fixes MUST be scoped to OpenCode plus the shared
  `ToolCallDisplay::from_result` and `SectionTracker` code paths
  that OpenCode depends on.

## Acceptance Output

After the three fixes land, the reference session above must render
as:

```text
󰀨 Skipped malformed OpenCode command: /Users/ken/.config/opencode/commands/catalog.md
󰀨 Skipped malformed OpenCode command: /Users/ken/.config/opencode/commands/homelab.md
[... five more skipped lines ...]

- OpenCode session ID ses_261302fb

 ← Read(successful, .claudine/skills/claudine/SKILL.md)
 ← Read(successful, .claudine/skills/claudine/unified-hooks.md)
 ← Read(successful, claudine/features/2026-04-17-reporting-consistency/plan.md)
 ← Read(error, claudine/features/2026-04-17-reporting-consistency/spec.md) File not found
 ← Bash(bash git status --short)
 ← Bash(bash git diff --staged --name-status)

I'll start by reading the lessons learned file [... assistant text ...]

✓ 50s · 17K input tokens · 2K output tokens · 267K cached tokens · $0.05 cost basis · 8 tool calls
```

Specifically:

- No `step_start` / `step_finish` lines.
- No runs of two blank lines anywhere.
- Every `← Read` and `← Bash` carries a useful slot.
- No trailing `⚠ Config — Skipped … malformed OpenCode assets` line.

[osem-start]: ../lib/src/stream/opencode_semantic.rs
[osem-finish]: ../lib/src/stream/opencode_semantic.rs
[osem-use]: ../lib/src/stream/opencode_semantic.rs
[osem-completed]: ../lib/src/stream/opencode_semantic.rs
[osem-completed-input]: ../lib/src/stream/opencode_semantic.rs
[osem-result]: ../lib/src/stream/opencode_semantic.rs
[lss-info]: ../cli/src/commands/wrap/live_semantic_sink.rs
[section]: ../cli/src/commands/wrap/section.rs
[td]: ../lib/src/stream/tool_display.rs
[td-fromresult]: ../lib/src/stream/tool_display.rs
[logs-opencode]: ../lib/src/stream/logs/opencode.rs
[logs-oc-warn]: ../lib/src/stream/logs/opencode.rs
[badges]: ../lib/src/stream/badges.rs
[badges-test]: ../lib/src/stream/badges.rs
[nis]: ./topics/non-interactive-sessions.md
