# Prompt Reporting Encapsulation — Design

**Date:** 2026-05-25
**Status:** Draft
**Stage:** 0 of 3 (see [Trajectory](#trajectory))

## Goal

Replace the nine-file, twenty-one-symbol `prompt_reporting` module with two
encapsulated report types — `SystemPromptReport` and `AgentPromptReport` —
backed by a single `ReportMode` enum. Make the change without disturbing
visual output or external behavior.

## Motivation

Twenty-one functions, three enums, and two boolean-bag config structs are
stitched together by call sites that must choose between
`report_system_prompt` and `report_system_prompt_empty` based on which
variant of `EffectiveSystemPrompt` they hold. The precedence resolver
produces only four distinct config shapes; the `truncation` field is always
`FrontBack` in practice; and combinations like
`show_header=false, show_summary=true` are representable but unreachable.
The booleans carry no information that a four-way enum would not carry more
precisely.

This will compound when the reports are migrated to `TerminalRenderable` +
`BrowserRenderable` (Stage 1, Stage 3): a flat module of free functions has
no obvious place to hang a trait impl. Encapsulating first makes every
subsequent stage a small addition rather than a restructure.

## Non-goals

- **Introducing `SessionConfig`.** A merged CLI/ENV/disk session-state owner
  is the right next step (see [Trajectory](#trajectory)), but it touches
  every CLI entry point and is its own design.
- **Adopting the render-tree IR.** The two report types implement `render`
  as a method today. Implementing `TerminalRenderable` is Stage 1; projecting
  to `RenderNode` is Stage 3. Stage 0 only requires that the eventual trait
  impl be a drop-in.
- **Changing visual output.** Header glyphs, block-quote colors, summary
  prose, truncation thresholds, and the chrome-width arithmetic all stay
  identical. Existing assertions in the module's tests are the gate.
- **Changing precedence behavior.** The precedence chain (CLI → env →
  length heuristic → frontmatter → unchanged-prompt suppression → default)
  is preserved exactly. Only its return type changes.

## Design

### Types

```rust
/// Resolved verbosity for a prompt report. Replaces the boolean bags and
/// the (PromptVerbosity, PromptReportFormat) pair.
pub enum ReportMode {
    Silent,
    Summary,                                  // header + prose summary
    Partial { truncation: TruncationMode },   // header + summary + truncated body
    Full,                                     // header + summary + full body
}

pub struct SystemPromptReport<'a> {
    resolved: &'a ResolvedSystemPrompt,
    mode: ReportMode,
    base: Option<&'a Path>,
}

pub struct AgentPromptReport<'a> {
    text: &'a str,
    mode: ReportMode,
}
```

### API

```rust
impl<'a> SystemPromptReport<'a> {
    pub fn new(
        resolved: &'a ResolvedSystemPrompt,
        mode: ReportMode,
        base: Option<&'a Path>,
    ) -> Self;

    /// Render the report. Returns `None` when nothing should be shown
    /// (`Silent` mode, or `ResolvedSystemPrompt::{None, Disabled}` in any
    /// mode below `Full`).
    pub fn render(&self, term: &Terminal) -> Option<String>;
}

impl<'a> AgentPromptReport<'a> {
    pub fn new(text: &'a str, mode: ReportMode) -> Self;
    pub fn render(&self, term: &Terminal) -> Option<String>;
}
```

`render` dispatches internally over the `ResolvedSystemPrompt` variants —
the `report_*` / `report_*_empty` split goes away. Inputs are borrowed
because callers already own them; the report does not need to take
ownership.

### Resolvers

`precedence.rs` keeps its job and its precedence chain. Its return type
changes from `SystemPromptReportConfig` / `UserPromptReportConfig` to
`ReportMode`:

```rust
pub fn resolve_system_prompt_report_mode(
    cli_silent: bool,
    cli_quiet: bool,
    cli_verbose: bool,
    env_verbosity: Option<ReportMode>,
    prompt_line_count: usize,
    frontmatter_verbosity: Option<ReportMode>,
    prompt_unchanged: bool,
) -> ReportMode;

pub fn resolve_agent_prompt_report_mode(
    cli_silent: bool,
    cli_verbose: bool,
    prompt_line_count: usize,
) -> ReportMode;
```

`parse_frontmatter_verbosity` returns `Option<ReportMode>` directly. The
string parser maps `"silent" → Silent`, `"quiet" → Summary`, `"verbose" →
Full`. `Partial` is reachable from code but not from the verbosity-string
input (consistent with today; the variant exists for future CLI surface).

### Type tally

| Removed | Added | Kept | Renamed |
|---|---|---|---|
| `SystemPromptReportConfig` | `ReportMode` | `TruncationMode` | `EffectiveSystemPrompt` → `ResolvedSystemPrompt` |
| `UserPromptReportConfig` | `SystemPromptReport<'_>` | | |
| `PromptReportFormat` | `AgentPromptReport<'_>` | | |
| `PromptVerbosity` | | | |

### Public surface

From 21 re-exports to 7:

- `SystemPromptReport`, `AgentPromptReport`
- `ReportMode`, `TruncationMode`
- `resolve_system_prompt_report_mode`, `resolve_agent_prompt_report_mode`
- `parse_frontmatter_verbosity`

All header/summary/body builders and block-quote helpers become module-private.

### Call-site change

Before (`cli/src/output/mod.rs`, ~50 lines including the boolean threading):

```rust
let config = resolve_system_prompt_report_config_with_change(
    silent, quiet, verbose, env_verbosity, line_count, fm, unchanged,
);
if let Some(out) = report_system_prompt_with_base(resolved, config, scope, term) {
    log::message(&out);
} else if let Some(out) = report_system_prompt_empty(resolved, config, term) {
    log::message(&out);
}
```

After:

```rust
let mode = resolve_system_prompt_report_mode(
    silent, quiet, verbose, env_verbosity, line_count, fm, unchanged,
);
if let Some(out) = SystemPromptReport::new(resolved, mode, scope).render(term) {
    log::message(&out);
}
```

The dual-entry-point split disappears. The boolean threading from CLI flags
through to the renderer still exists in Stage 0 — it gets consolidated when
`SessionConfig` lands (Stage 2).

### `ResolvedSystemPrompt` rename

`EffectiveSystemPrompt` is renamed to `ResolvedSystemPrompt`. The word
"effective" adds no information — every value is "effective" by definition.
"Resolved" pairs naturally with the existing `PreparedSystemPrompt` (its
`Ready` payload) and with the resolver functions that produce it.

Mechanical find/replace across 8 files, 72 references:

- `claudine/lib/src/system_prompt/types.rs` (definition)
- `claudine/lib/src/system_prompt/prepare.rs`
- `claudine/lib/src/prompt_reporting/system_prompt.rs`
- `claudine/cli/src/commands/wrap/mod.rs`
- `claudine/cli/src/commands/wrap/system_prompt.rs`
- `claudine/cli/src/commands/wrap/composition/mod.rs`
- `claudine/cli/src/output/mod.rs`
- `claudine/cli/tests/system_prompt_perf_bench.rs`

No semantic change.

## Risks

- **Test breakage from the rename.** 72 references; `cargo check` will
  surface anything missed. Risk: low.
- **Public-surface contraction.** The dropped re-exports may be used by
  downstream code outside this repo. Risk: low (this is a workspace-internal
  module not on crates.io). Mitigation: a `pub use` shim layer is rejected
  — it would defeat the encapsulation goal. Internal call sites are the
  only consumers we have to satisfy.
- **`PromptVerbosity` → `ReportMode` mapping in env-var parsing.** Anyone
  who set `CLAUDINE_SYSTEM_PROMPT=quiet` expecting it to mean "summary"
  will continue to get summary; the parser's user-facing strings do not
  change. Risk: none.
- **The `Partial` variant of `ReportMode` has no producer today.** It is
  representable in the new enum and exercised by the body renderer, but
  precedence never returns it. Kept because it is currently public surface
  (`PromptReportFormat::PartialPrompt`) and removing it is a behavior cut,
  not a refactor. Future CLI work can wire a producer.

## Acceptance criteria

- `prompt_reporting::*` public surface is `SystemPromptReport`,
  `AgentPromptReport`, `ReportMode`, `TruncationMode`,
  `resolve_system_prompt_report_mode`, `resolve_agent_prompt_report_mode`,
  `parse_frontmatter_verbosity`. Nothing else.
- `EffectiveSystemPrompt` no longer appears in any source file.
- `cli/src/output/mod.rs::log_system_prompt_with_scope` and
  `log_compose_prompt` each call exactly one resolver and construct one
  report; the dual-entry-point branching is gone.
- All existing `prompt_reporting/*` and `cli/src/output/*` tests pass without
  modification beyond the API rename. New tests cover the
  `ResolvedSystemPrompt::{None, Disabled}` dispatch through
  `SystemPromptReport::render`.
- `cargo doc -p claudine` produces no warnings about broken intra-doc links.

## Trajectory

This is Stage 0 of a four-stage path. Stages 1–3 are acknowledged here so
that Stage 0 does not paint them into a corner; each gets its own design
when its turn comes.

| Stage | Scope |
|---|---|
| **0 — Encapsulation (this design)** | Report types, `ReportMode`, `ResolvedSystemPrompt` rename. |
| **1 — Renderable trait** | Implement `TerminalRenderable` on both report types. `render` becomes the trait impl body. No behavior change. |
| **2 — `SessionConfig`** | Introduce a session-state owner that merges CLI flags, env vars, and disk config once at startup. Expose `.system_prompt_report(&ResolvedSystemPrompt) -> SystemPromptReport` and `.agent_prompt_report(&str) -> AgentPromptReport`. Migrate call sites from boolean-threading to `&SessionConfig`. |
| **3 — `BrowserRenderable` + tree IR** | Add `BrowserRenderable` impls; later migrate to `TreeRenderable` per `renderable/docs/tree-rendering.md`. |

Stage 0 keeps the report types' constructors small and borrow-based so that
Stage 2 can add `SessionConfig` accessors without rewriting either type.
