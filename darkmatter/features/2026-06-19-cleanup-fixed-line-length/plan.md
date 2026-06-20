---
agent: open_code/minimax/MiniMax-M3
phases: 5
created: 2026-06-19
start_phase: 1
yolo: "true"
source_spec: spec.md
packages:
  - darkmatter
  - darkmatter-cli
source_files_during_phase_1:
  - darkmatter/lib/Cargo.toml
  - darkmatter/lib/src/markdown/cleanup.rs
  - darkmatter/lib/src/markdown/mod.rs
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/cleanup.rs
  - darkmatter/lib/src/markdown/mod.rs
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/cleanup.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
  - darkmatter/lib/src/markdown/compose/tests.rs
source_files_during_phase_4:
  - darkmatter/cli/src/args/cli.rs
  - darkmatter/cli/src/args/command.rs
  - darkmatter/cli/src/args/parsers.rs
  - darkmatter/cli/src/args/completion.rs
  - darkmatter/cli/src/args/mod.rs
  - darkmatter/cli/src/commands/clean.rs
  - darkmatter/cli/src/commands/mod.rs
  - darkmatter/cli/src/main.rs
  - darkmatter/lib/src/markdown/cleanup.rs
source_files_during_phase_5:
  - darkmatter/cli/tests/clean.rs
source_code:
  - darkmatter/cli/src/args/cli.rs
  - darkmatter/cli/src/args/command.rs
  - darkmatter/cli/src/args/completion.rs
  - darkmatter/cli/src/args/mod.rs
  - darkmatter/cli/src/args/parsers.rs
  - darkmatter/cli/src/commands/clean.rs
  - darkmatter/cli/src/commands/mod.rs
  - darkmatter/cli/src/main.rs
  - darkmatter/cli/tests/clean.rs
  - darkmatter/cli/tests/common/mod.rs
  - darkmatter/lib/src/markdown/cleanup.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
  - darkmatter/lib/src/markdown/compose/preflight/mod.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/src/markdown/compose/tests.rs
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/tests/shell_block_integration.rs
documentation:
  - darkmatter/cli/README.md
  - darkmatter/docs/cli/clean.md
  - darkmatter/docs/darkmatter-compose-pipeline.md
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
docs_updated_during_phase_3:
  - darkmatter/docs/darkmatter-compose-pipeline.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
docs_updated_during_phase_5:
  - darkmatter/cli/README.md
  - darkmatter/docs/cli/clean.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/darkmatter/compose.md
  - .opencode/skill/darkmatter/SKILL.md
  - .opencode/skill/darkmatter/compose.md
---

# Execution Plan: Cleanup Fixed Line Length

## Overview

Extend Darkmatter's `md clean` operation so it can flatten Markdown files whose
authors (or LLMs) wrapped lines at a fixed column — e.g. 80 or 100 characters
— back to the canonical one-paragraph-per-block form that Markdown is built
for. The default behavior strips single newlines with Markdown nuance
(preserving blank-line semantics and code-fence/list-marker safety) so the
existing `md clean <file>` workflow automatically produces notationally
velocity-friendly output. Two additional library/CLI switches let callers
preserve their existing line breaks (`--ignore-incidental-newlines`) or
re-wrap the document to a target column (`--fixed-width <ch>`).

The plan proceeds bottom-up: a library primitive for incidental-newline
removal, a library primitive for fixed-width reflow, compose-pipeline
integration, CLI flag plumbing, then end-to-end validation and documentation.

**Success criteria:**

- `md clean foo.md` on a document with 80-column wrapped prose collapses every
  single-newline run to either nothing (when the preceding line ends in
  whitespace) or a single space (otherwise), without disturbing blank lines,
  fenced/indented code blocks, inline code, list indentation, blockquote
  prefixes, table rows, or HTML blocks.
- `md clean foo.md --fixed-width 80` reflows the document's prose to an
  80-column target while preserving block structure (headings, lists,
  blockquotes, code fences, tables, HTML blocks, transclusion directives).
- `md clean foo.md --ignore-incidental-newlines` makes no incidental-newline
  mutations; existing cleanup behavior (whitespace, list spacing, indent) is
  unchanged.
- Both new library primitives are unit-tested with paragraph, list,
  blockquote, table, and code-block fixtures, including non-ASCII text where
  `UnicodeWidthStr::width` differs from byte length.
- ComposeOptions gains `with_incidental_newline_mode` and
  `with_fixed_width` builders; the Cleanup compose stage applies both in the
  correct order so the existing perf budget still holds (single-pass document
  scan, no extra allocations beyond the output buffer).

## Assumptions

- The flag spec writes `--ignore-incidental-carraige-returns` (typo:
  "carraige" → "carriage"). The spec explicitly invites a better name; this
  plan ships the flag as `--ignore-incidental-newlines` and surfaces the
  rename as a clarification item (Phase 4). The library enum is named
  `IncidentalNewlineMode` to match the spec's "incidental" framing rather than
  the carriage-return heritage.
- The default `IncidentalNewlineMode` is `Strip` (i.e. `md clean` now strips
  single newlines by default). The previous `md clean` behavior is fully
  preserved when `Strip` is paired with `--ignore-incidental-newlines`, so
  callers that want zero incidental-newline mutation can opt out cleanly.
- "Markdown nuance" in the spec maps to: preserve two-or-more `\n` (paragraph
  boundaries), preserve single `\n` inside fenced/indented code blocks and
  inline code spans, do not split inside HTML blocks, do not touch table row
  separators or list-marker continuation, and do not alter transclusion
  directives (`::file`, `::code`, `::shell`, `::disclosure`, etc.).
- The `ch` unit is the typographic column count: `biscuit_terminal::utils::
  UnicodeWidthStr::width(text)` is the authoritative measure. Tests must
  include a non-ASCII fixture (e.g. `café`) to lock this in.
- Fixed-width reflow uses the two-pass strategy the spec calls "most obvious":
  strip incidental newlines first, then re-wrap prose blocks to the target
  width. A single-pass solution is permitted if it can be shown to produce
  byte-identical output without exceeding the existing cleanup perf budget.
- Fenced/indented code blocks, inline code spans, tables, and HTML blocks are
  never reflowed; their lines pass through verbatim. Reflow applies only to
  paragraphs and (within reason) list-item body text — list markers and
  continuation indentation are preserved exactly as the existing indent
  normalization already does.
- The Cleanup compose phase already runs as a single linear pass over the
  serialized document. The new behavior slots in as additional, ordered passes
  inside the same phase: incidental-newline strip → existing list/indent
  normalization → fixed-width reflow. No new stage, no new perf event.
- `darkmatter/cli/README.md` and `darkmatter/docs/cli/clean.md` are the CLI
  documentation surfaces; the darkmatter skill's `SKILL.md` and `compose.md`
  are the agent skill surfaces that must reflect the new defaults.

## Phase 1 - Library: Incidental-Single-Newline Removal

*Add the Markdown-aware primitive that the new default behavior and the
compose pipeline both depend on, before any CLI surface exists.*

### Tasks

- [x] Add `IncidentalNewlineMode { Strip, Preserve }` to
      `darkmatter/lib/src/markdown/cleanup.rs` with `Debug`, `Clone`, `Copy`,
      `PartialEq`, `Eq` derives; document the semantic difference in a
      `///` doc comment on each variant.

- [x] Implement `pub fn strip_incidental_newlines(content: &str) -> String`
      that walks the document line by line, tracking fenced/indented code,
      inline code spans (using a backtick-count state machine mirroring the
      existing `unescape_emphasis_chars`), HTML block open/close (per
      pulldown-cmark's HTML block rules: type 1-6 closing on a blank line),
      and blockquote prefixes. Inside prose regions it collapses a single
      `\n` to nothing when the previous line ends in whitespace (`\s` set
      including ` `, `\t`) and to a single space otherwise; runs of two or
      more `\n` are passed through unchanged.

- [x] Add a `Markdown` method `pub fn strip_incidental_newlines(&mut self) ->
      &mut Self` that wraps the free function and updates `self.content`,
      matching the builder-style return used by the existing
      `cleanup*` family. Add it next to the other `Markdown::cleanup*`
      methods in `darkmatter/lib/src/markdown/mod.rs`.

- [x] Update `Markdown::cleanup`, `cleanup_compact`, `cleanup_loose`,
      `cleanup_with_indent*` to call `strip_incidental_newlines` on
      `self.content` *before* handing it to `cleanup_content_internal`, so
      `md clean <file>` (which dispatches to `Markdown::cleanup`) now strips
      incidentally-newlined prose by default. Keep the existing function
      bodies byte-for-byte equivalent when the input has no single-newline
      runs (round-trip parity test).

- [x] Add unit tests in
      `darkmatter/lib/src/markdown/cleanup.rs` (in the existing
      `#[cfg(test)] mod tests`) covering: prose with trailing-whitespace
      newlines (drop), prose without trailing-whitespace newlines (space
      replacement), blank-line preservation, fenced code block lines
      preserved verbatim, indented code block lines preserved verbatim,
      inline code spans preserved verbatim, HTML block lines preserved
      verbatim, blockquote-prefix preservation, table row separators
      preserved, list markers preserved across the strip, transclusion
      directives (`::file`, `::code`, `::shell`, `::disclosure`) preserved
      with their arguments intact, and a paragraph that mixes `crlf` and
      `lf` input (treat both as `\n`).

### Validation Checkpoint

- [x] `just test darkmatter` (or `cargo nextest run -p darkmatter --lib
      cleanup`) passes, including the new tests and all existing cleanup
      tests.

- [x] `cargo check -p darkmatter --lib` succeeds with no new warnings.

- [x] Existing `md clean` round-trip fixture tests in
      `darkmatter/cli/tests/clean.rs` still pass (no CLI surface change
      yet, but the underlying `Markdown::cleanup` semantics broadened; if
      any existing fixture relies on incidental newlines being preserved,
      flag it as a behavior change and update the fixture under review).

### Parallelizable Work

- [x] The `IncidentalNewlineMode` enum, the free function, and the
      `Markdown` wrapper method are sequential (enum → function → method).
      The unit tests can be drafted in parallel with the implementation,
      but they must be re-run against the final implementation before
      sign-off.

## Phase 2 - Library: Fixed-Width Reflow

*Add the optional re-wrap primitive that turns `md clean --fixed-width 80`
into a no-op-on-the-front, reflow-on-the-back transformation.*

### Tasks

- [x] Implement `pub fn reflow_to_width(content: &str, width: usize) ->
      String` in `darkmatter/lib/src/markdown/cleanup.rs`. It takes the
      output of `strip_incidental_newlines` (so it can rely on the document
      being in paragraph/block form already) and walks block by block:
      paragraphs reflow to `width` columns using a unicode-aware
      `textwrap`-style breaker (preferring `UnicodeWidthStr::width` to
      measure, breaking on whitespace, never splitting inside inline code
      spans or emphasis markers); fenced/indented code blocks, tables,
      HTML blocks, and transclusion directive lines are emitted verbatim.
      List items are reflowed *within* their continuation indentation —
      markers and nesting whitespace are preserved, body text is wrapped
      to `width - marker_width` columns.

- [x] Add `pub fn cleanup_to_fixed_width(content: &str, width: usize) ->
      String` that composes `strip_incidental_newlines` then
      `reflow_to_width` in one call. This is the single entry point the
      compose pipeline and the CLI will both call.

- [x] Add `pub fn Markdown::cleanup_with_fixed_width(&mut self, width:
      usize) -> &mut Self` in `darkmatter/lib/src/markdown/mod.rs` that
      delegates to the composed free function, matching the builder
      pattern of the other `cleanup*` methods.

- [x] Reject `width == 0` at the library boundary with a panic (matching
      the `indent_size.max(1)` pattern already used in
      `cleanup_content_with_indent`) — the CLI parser will already reject
      `0`, but defensive `width.max(1)` keeps the library safe for
      programmatic callers.

- [x] Add unit tests in `darkmatter/lib/src/markdown/cleanup.rs` covering:
      ASCII paragraph wraps at 20/40/80/100 columns; non-ASCII text
      (`café`, `naïve`) where `width` is column count not byte count;
      very long single word longer than `width` stays on one line; empty
      document; document with only code blocks (no reflow); list with
      mixed marker widths (`-` vs `1.` vs `- [ ]`) preserves alignment;
      table rows pass through unchanged; blockquote reflow keeps the `>`
      prefix; transclusion directives pass through unchanged; an
      `HtmlOptions`-style HTML block is preserved verbatim.

### Validation Checkpoint

- [x] `just test darkmatter` passes including the new reflow tests.

- [x] `cargo check -p darkmatter --lib` succeeds with no new warnings.

- [x] A focused `cargo bench -p darkmatter --bench cleanup` (or whichever
      existing bench covers cleanup) shows the composed
      `strip + reflow` pass adds less than 2x the cost of the existing
      `cleanup_content` on the same fixture. If it exceeds that, profile
      and switch to a single-pass implementation before Phase 3.

### Parallelizable Work

- [x] The reflow primitive and its tests can be drafted in parallel with
      the Phase 1 incidental-newline tests (they depend only on the
      public signature, which is locked at the start of Phase 2).

- [x] Builder method `Markdown::cleanup_with_fixed_width` and the
      composed free function `cleanup_to_fixed_width` are sequential
      after the reflow primitive lands.

## Phase 3 - Compose Options Wiring

*Make the new library primitives first-class options on the compose
pipeline so the CLI in Phase 4 and programmatic compose callers can use
them; preserve every existing default.*

### Tasks

- [x] Add `incidental_newline_mode: IncidentalNewlineMode` (default
      `Strip`) and `fixed_width: Option<usize>` (default `None`) fields
      to `ComposeOptions` in
      `darkmatter/lib/src/markdown/compose/context/options.rs`, in the
      existing "Cleanup" section alongside `list_spacing` and
      `indent_size`.

- [x] Add builder methods
      `with_incidental_newline_mode(IncidentalNewlineMode) -> Self` and
      `with_fixed_width(usize) -> Self` next to `with_list_spacing`,
      matching the existing setter naming convention.

- [x] Update the `Debug` impl of `ComposeOptions` to print both new
      fields next to `list_spacing` and `indent_size`.

- [x] Wire the new fields into
      `Markdown::run_inline_post_operation` in
      `darkmatter/lib/src/markdown/compose/pipeline/phases.rs` for
      `ComposeOperation::Cleanup`: apply
      `strip_incidental_newlines` first (skipped when mode is
      `Preserve`), run the existing list-spacing/indent block, then
      apply `reflow_to_width` last (skipped when `fixed_width` is
      `None`). Update the `report.cleanup_changed` comparison to use
      the post-pipeline content so the delta flag still reflects all
      mutations the phase produced.

- [x] Update `cleanup_changed` tracking in
      `ComposeReport` if any prior compose integration test asserts a
      specific before/after diff that the new strip pass would change.
      If so, the affected test gets updated alongside this phase under
      review (no behavior change for callers that leave defaults as
      `Strip` because the prior cleanup already normalized blank lines
      the new strip pass leaves alone).

- [x] Add focused integration tests in the existing compose pipeline
      test module (search the compose tree for the cleanup-integration
      test file) verifying: default compose strips incidental newlines;
      `with_incidental_newline_mode(Preserve)` leaves a document with
      single-newline prose unchanged; `with_fixed_width(80)` produces
      output whose longest prose line is `<= 80` columns; combining
      `Preserve` + `with_fixed_width(80)` is a no-op for prose that
      already fits.

### Validation Checkpoint

- [x] `just test darkmatter` passes.

- [x] `cargo check -p darkmatter --lib` succeeds with no new warnings.

- [x] No existing compose integration test regresses; if any test was
      updated to track the broadened default, the diff is reviewed in
      this phase (no library surface change without a documented
      rationale).

### Parallelizable Work

- [x] The `with_*` builders, the `Debug` update, and the
      `phases.rs` wiring can be drafted in parallel; integration tests
      must wait for the wiring to land.

## Phase 4 - CLI: Flag Plumbing

*Expose the two new library switches on `md clean` and wire them through
to the existing `run_clean` command.*

### Tasks

- [x] Add `--fixed-width <#>` to the `Clean` subcommand variant in
      `darkmatter/cli/src/args/command.rs`. Mirror the
      `--indent <#>` field shape: `Option<usize>`, with a
      `value_parser = parse_fixed_width` and a `complete_fixed_width_values`
      shell completer.

- [x] Add `--ignore-incidental-newlines` to the `Clean` subcommand
      variant. Per the spec rename, ship the better name; add a comment
      on the field that surfaces the original spec spelling
      (`--ignore-incidental-carraige-returns`) and the rationale for
      renaming so reviewers can object before sign-off.

- [x] Add `pub fn parse_fixed_width(s: &str) -> Result<usize, String>` in
      `darkmatter/cli/src/args/parsers.rs`: positive integer in `[1, 1000]`
      (any sane column width; reject `0` and reject anything that does not
      parse). Document the upper bound in the parser doc comment so future
      contributors know why it exists.

- [x] Add `pub fn complete_fixed_width_values(current: &OsStr) ->
      Vec<CompletionCandidate>` in
      `darkmatter/cli/src/args/completion.rs`, mirroring
      `complete_indent_values` style: suggest `40`, `60`, `80`, `100`,
      `120` as common presets; the `parse_fixed_width` validator still
      accepts any in-range integer.

- [x] Update `darkmatter/cli/src/args/mod.rs` re-exports if the new
      parser / completer follow the existing `pub use` pattern; if they
      are `pub(crate)`, no export change is needed.

- [x] Update `crate::args::Command::Clean` match arm in
      `darkmatter/cli/src/commands/mod.rs` to thread
      `fixed_width` and `ignore_incidental_newlines` into
      `run_clean`.

- [x] Update `pub fn run_clean` in
      `darkmatter/cli/src/commands/clean.rs` to accept the two new
      parameters. When `fixed_width.is_some()`, dispatch to
      `Markdown::cleanup_with_fixed_width(fixed_width)` instead of the
      default `Markdown::cleanup` path. When
      `ignore_incidental_newlines`, route to a new internal
      `apply_cleanup_no_strip` that calls the existing
      `cleanup_content_*` helpers without first stripping incidental
      newlines (preserves the pre-feature behavior bit-for-bit).

- [x] Reject `md clean <file> --fixed-width 80 --ignore-incidental-newlines`
      with a clear error: reflowing to a fixed width without first
      stripping the source's incidental newlines would just re-flow the
      input's own wrapping. Surface the conflict via clap's
      `conflicts_with` attribute on the CLI flag pair so the rejection is
      uniform with the rest of the CLI's flag-conflict handling.

- [x] Update clap-derive unit tests in
      `darkmatter/cli/src/args/cli.rs` to cover the new flags
      parse correctly (one test each for `--fixed-width 80`,
      `--ignore-incidental-newlines`, and the conflict rejection).

### Validation Checkpoint

- [x] `cargo check -p darkmatter-cli` succeeds with no new warnings.

- [x] `just test darkmatter-cli` passes for the parser-level tests.

- [x] `md clean --help` and `md clean --fixed-width --help` show the new
      flags with their doc comments.

### Parallelizable Work

- [x] `parse_fixed_width` + `complete_fixed_width_values` and the
      `IncidentalNewlineMode` flag plumbing are independent and can be
      drafted in parallel. The `run_clean` signature change must land
      after both are in place; the conflict-rejection work depends on
      the `run_clean` signature change.

## Phase 5 - End-to-End Validation and Documentation

*Lock the new behavior in with CLI-level integration tests, update the
documentation surfaces, and refresh the agent skill to reflect the new
defaults.*

### Tasks

- [x] Extend `darkmatter/cli/tests/clean.rs` with: `md clean -` on a
      fixture with 80-column wrapping produces collapsed prose (test
      default strip); `md clean --fixed-width 80 -` produces reflowed
      output whose longest line is `<= 80` columns; `md clean
      --ignore-incidental-newlines -` is a no-op for incidental
      newlines (existing whitespace/list cleanup still applies); the
      conflict between `--fixed-width` and `--ignore-incidental-newlines`
      exits non-zero with a helpful stderr; the new flags compose with
      `--save` and the `--save --verbose` path produces a delta report
      that correctly flags the new mutations.

- [x] Update `darkmatter/cli/README.md` to document the two new flags
      on `md clean`, including the default-strip behavior and the
      fixed-width + ignore-incidental conflict.

- [x] Update `darkmatter/docs/cli/clean.md` (or create it if it does
      not exist yet — confirm with the docs directory before creating)
      with a worked example for each of the three modes (default,
      `--fixed-width 80`, `--ignore-incidental-newlines`).

- [x] Update `.opencode/skill/darkmatter/SKILL.md` with the new
      `Markdown::cleanup*` family entry points and the broadened
      default behavior of `md clean`.

- [x] Update `.opencode/skill/darkmatter/compose.md` (or the compose
      pipeline topic it links to) to mention
      `with_incidental_newline_mode` and `with_fixed_width` as
      `ComposeOptions` builders in the Cleanup section.

- [x] Run the full area check: `just test darkmatter` and
      `just test darkmatter-cli` both green; `just lint darkmatter` and
      `just lint darkmatter-cli` both clean.

- [x] If `AGENTS.md` mentions `md clean` defaults or the cleanup
      pipeline anywhere, update that mention to reflect the new
      default behavior; otherwise, no `AGENTS.md` change is needed.

### Validation Checkpoint

- [x] `just test darkmatter` and `just test darkmatter-cli` both pass
      (per the monorepo's standard test recipe in `just/`).

- [x] `just lint darkmatter` and `just lint darkmatter-cli` pass.

- [x] Manual smoke check (recorded in the PR description, not a script):
      `md clean docs/some-80-col-doc.md` collapses wraps; `md clean
      docs/some-80-col-doc.md --fixed-width 80` reflows to 80; `md clean
      docs/some-80-col-doc.md --ignore-incidental-newlines` is a no-op
      for incidental newlines.

- [x] Documentation surfaces (`cli/README.md`, `docs/cli/clean.md`,
      skill `SKILL.md`, skill `compose.md`) all describe the same
      defaults and the same conflict between the two flags.

### Parallelizable Work

- [x] The CLI integration tests, the README update, and the skill
      updates are all independent and can be drafted in parallel; the
      final lint/test gate is the only serial step.
