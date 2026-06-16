---
agent: open_code/kimi-for-coding/k2p6
created: 2026-06-14
phases: 5
start_phase: 1
yolo: 'true'
source_spec: darkmatter/reviews/2026-06-14-summary-and-suggest/fix.md
source_review: darkmatter/reviews/2026-06-14-summary-and-suggest/review.md
source_files_during_phase_1: []
docs_updated_during_phase_1:
- darkmatter/features/2026-06-14-summary-and-suggest/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
- darkmatter/lib/src/markdown/dsl/mod.rs
- darkmatter/lib/src/markdown/dsl/parser.rs
- darkmatter/cli/src/commands/code_block.rs
- darkmatter/cli/tests/code_block.rs
docs_updated_during_phase_2:
- darkmatter/features/2026-06-14-summary-and-suggest/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
- darkmatter/cli/src/commands.rs
- darkmatter/cli/src/commands/compose.rs
- darkmatter/cli/src/output.rs
docs_updated_during_phase_3:
- darkmatter/features/2026-06-14-summary-and-suggest/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
- darkmatter/lib/src/markdown/compose/types.rs
- darkmatter/lib/src/markdown/compose/perf.rs
- darkmatter/lib/src/markdown/compose/mod.rs
docs_updated_during_phase_4:
- darkmatter/features/2026-06-14-summary-and-suggest/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
- .claude/skills/darkmatter/SKILL.md
source_files_during_phase_5: []
docs_updated_during_phase_5:
- darkmatter/features/2026-06-14-summary-and-suggest/plan.md
- darkmatter/reviews/2026-06-14-summary-and-suggest/fix.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_code:
- darkmatter/cli/src/commands.rs
- darkmatter/cli/src/commands/code_block.rs
- darkmatter/cli/src/commands/compose.rs
- darkmatter/cli/src/output.rs
- darkmatter/cli/tests/code_block.rs
- darkmatter/lib/src/markdown/compose/mod.rs
- darkmatter/lib/src/markdown/compose/perf.rs
- darkmatter/lib/src/markdown/compose/types.rs
- darkmatter/lib/src/markdown/dsl/mod.rs
- darkmatter/lib/src/markdown/dsl/parser.rs
documentation:
- darkmatter/features/2026-06-14-summary-and-suggest/plan.md
- darkmatter/reviews/2026-06-14-summary-and-suggest/fix.md
status: completed
packages:
- darkmatter
- darkmatter-cli
hash: 5936d5709d8cc45b-7f977b58752eb973
---

# Execution Plan: Rendering and Compose Maintenance

Plan derived from `darkmatter/reviews/2026-06-14-summary-and-suggest/fix.md`.
Goal: behavior-preserving maintenance across the Darkmatter package area.

## Constraints

- This plan assumes the `LanguageGrammar` single-authority feature (`darkmatter/features/2026-06-15-grammar/`) has already landed. All grammar resolution must continue to route through `darkmatter::markdown::language_grammar::LanguageGrammar`; no new direct `SyntaxSet::find_syntax_by_*` production calls are allowed outside `language_grammar.rs`.
- No public CLI flag, subcommand, or output-format change.
- No compose operation reorder.
- No rendering output change except accidental divergence revealed by tests.
- Do not run `cargo fmt` unless explicitly requested.
- If a user-visible behavior change is required, stop and update the spec or open a follow-up.

### Code-transclusion behavior note

Because `LanguageGrammar` uses the two-face extended grammar set, code transclusion now recognizes extensions that syntect's bare defaults do not (e.g. `.ts`, `.toml`). For a two-face-only extension, `infer_language` emits the real extension token instead of the fallback token. This is an intended widening, not a regression — composed Markdown fence info strings may change for those files.

---

## Phase 1: Cleanup and Extraction Validation

**Goal:** Remove accidental artifacts, harden frontmatter test coverage, and validate the existing CLI command extraction.

- [x] Delete `darkmatter/cli/src/commands.rs.orig` if it exists.
- [x] Delete `darkmatter/lib/src/markdown/compose/shell_expansion/store.rs.bak` if it exists.
- [x] Run `rg` across the repo to confirm no code, tests, docs, or build scripts reference the deleted artifact files.
- [x] Audit `darkmatter/lib/src/markdown/compose/mod.rs` around line 4523 for the `DM_DEBUG` frontmatter repro test.
- [x] Audit `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs` around line 201 for the env-controlled debug dump.
- [x] For each debug-only frontmatter test, either delete it or convert it into an explicit regression test asserting frontmatter values, warnings, and error behavior.
- [x] Verify the CLI command module extraction matches the target shape:
  - `darkmatter/cli/src/commands/compose.rs` exists and exposes a `run_*` entrypoint.
  - `darkmatter/cli/src/commands/hash.rs` exists and exposes a `run_*` entrypoint.
  - `darkmatter/cli/src/commands/frontmatter.rs` exists and exposes a `run_*` entrypoint.
  - `darkmatter/cli/src/commands/code_block.rs` exists and exposes a `run_*` entrypoint.
  - `darkmatter/cli/src/commands/schema/*` still exists.
- [x] Confirm `darkmatter/cli/src/commands.rs` is primarily dispatch and shared helpers, with no command implementation duplicated in a submodule.
- [x] Move any single-module helpers out of `commands.rs` into their owning submodule; keep only cross-module helpers in `commands.rs`.

**Validation checkpoint:**
- `cargo check -p darkmatter-cli` passes.
- `cargo test -p darkmatter --lib markdown::compose` passes.
- `cargo test -p darkmatter-cli compose` passes.
- Artifact files are absent from disk and from `rg` results.

---

## Phase 2: Code-Block Serialization and Highlight Parsing

**Goal:** Eliminate duplicated Markdown serialization and highlight-range parsing in the CLI.

### 2a. CodeBlock Markdown Serialization Helper

- [x] Locate both `md code-block --output markdown` paths in `darkmatter/cli/src/commands/code_block.rs` (stdout and `--show`).
- [x] Introduce a private helper `fn code_block_markdown(block: &CodeBlock) -> String` in `darkmatter/cli/src/commands/code_block.rs`.
- [x] Move fence info assembly into the helper.
- [x] Move title quoting and escaping into the helper.
- [x] Move `line-numbering=true` serialization into the helper.
- [x] Move `highlight=...` serialization into the helper.
- [x] Implement safe fence selection: choose a fence run longer than any same-character fence run in the code body.
- [x] Ensure the helper returns a complete fenced block with exactly one trailing newline.
- [x] Wire stdout to print the helper string without adding a second newline.
- [x] Wire `--show` to use the same helper string.
- [x] Add CLI module tests covering:
  - language only
  - quoted title containing a double quote
  - line numbering
  - highlight ranges
  - empty language with metadata
  - code containing triple backticks
  - identical stdout and `--show` artifact content

### 2b. Shared Highlight Parser

- [x] Locate `darkmatter::markdown::dsl::HighlightSpec` and `parse_code_info`.
- [x] Introduce `pub fn parse_highlight_spec(raw: &str) -> Result<HighlightSpec, HighlightSpecParseError>` near `HighlightSpec`.
- [x] Define `HighlightSpecParseError` with enough detail to reconstruct `MarkdownError::InvalidLineRange(...)` for fenced-code parsing and `Invalid --highlight range: ...` for the CLI.
- [x] Update `parse_code_info` to call `parse_highlight_spec` for `highlight=...` metadata.
- [x] Update `md code-block --highlight` to call `parse_highlight_spec` and map the error to the existing CLI message.
- [x] Preserve the current grammar exactly: comma-separated line numbers and inclusive ranges such as `1,4-6`.
- [x] Preserve current behavior for empty comma segments (ignore them).
- [x] Keep malformed metadata surfacing through render preflight as it does today.
- [x] Delete the CLI-local `parse_highlight_cli` parser and its drift-prone documentation.
- [x] Add library tests for `parse_highlight_spec` covering valid lines, valid ranges, empty segments, and invalid ranges.
- [x] Add one CLI-facing test for an invalid `--highlight` range.

**Validation checkpoint:**
- `cargo test -p darkmatter --lib markdown::dsl` passes.
- `cargo test -p darkmatter-cli code_block` passes.
- `cargo test -p darkmatter-cli render` passes.

---

## Phase 3: Lazy Theme Resolution

**Goal:** Resolve themes only for output arms that need them.

### 3a. Render Theme Resolution

- [x] Locate `run_render` theme resolution in the CLI render path.
- [x] Ensure `OutputFormat::Markdown` does not call `ResolvedTheme::from_cli`.
- [x] Ensure non-TTY `OutputFormat::Auto` does not call `ResolvedTheme::from_cli`.
- [x] Ensure `OutputFormat::Json` does not call `ResolvedTheme::from_cli`.
- [x] Ensure terminal rendering, `MarkdownPlus`, and `Html` derive one `ResolvedTheme` from the same `Terminal` color mode.
- [x] Confirm no dual-source color mode drift is introduced between page theme and code-block theme.

### 3b. Compose Theme Resolution

- [x] Locate `run_compose` theme resolution in the CLI compose path.
- [x] Ensure `OutputFormat::Auto` and `OutputFormat::Markdown` do not call `ResolvedTheme::from_cli`.
- [x] Ensure `OutputFormat::Json` does not call `ResolvedTheme::from_cli`.
- [x] Move `ResolvedTheme::from_cli` resolution into the `MarkdownPlus` and `Html` match arms only.
- [x] Verify composed disclosure blocks still route through the MarkdownPlus fold and emit inline `<details>/<summary>` HTML.

**Validation checkpoint:**
- `cargo test -p darkmatter-cli render` passes.
- `cargo test -p darkmatter-cli compose` passes.
- Snapshot tests for Markdown, JSON, non-TTY auto, MarkdownPlus, and HTML remain unchanged.

---

## Phase 4: Compose Metadata Centralization and Documentation

**Goal:** Create a single source of truth for compose operation metadata and align all compose phase documentation with the live four-phase pipeline.

### 4a. Compose Operation Descriptor Table

- [x] Open `darkmatter/lib/src/markdown/compose/types.rs`.
- [x] Add `ComposeOperationDescriptor`:
  ```rust
  pub struct ComposeOperationDescriptor {
      pub operation: ComposeOperation,
      pub index: usize,
      pub phase: ComposePhase,
      pub default_enabled: bool,
      pub label: &'static str,
      pub perf_kind: Option<perf::PerfMetricKind>,
  }
  ```
- [x] If importing `perf::PerfMetricKind` creates an undesirable dependency, define a small local enum in `types.rs` or place the perf mapping beside the table through an exhaustive `match`.
- [x] Define a static descriptor table covering every `ComposeOperation` variant with stable indices.
- [x] Derive or implement:
  - `ComposeOperation::COUNT`
  - `ComposeOperation::index()`
  - `ComposeOperation::phase()`
  - `ComposeOperation::default_order()`
  - label/display helpers for reports
  - perf mapping for the runner
- [x] Do not expand `ComposeOperation` to include `SchemaValidation`, `EffectiveStateBuild`, `TransclusionParse`, `TransclusionPrepare`, `TransclusionResolve`, or `TransclusionApply`; keep those in `perf.rs` unless a separate feature makes them user-toggleable.
- [x] Update `ComposeOperationSet` consumers to use the descriptor-derived indices.

### 4b. Descriptor Invariant Tests

- [x] Add a test that descriptor indices are contiguous, unique, and equal to `ComposeOperation::COUNT`.
- [x] Add a test that `default_order()` equals descriptor order filtered by `default_enabled`.
- [x] Add an exhaustive mapping test or compile-time match so every operation either maps to a perf metric or is explicitly documented as having no operation-level metric.

### 4c. Compose Phase Documentation

- [x] Update `darkmatter/lib/src/markdown/compose/mod.rs` module docs to list four phases: Inline Pre, Transclusion, Inline Post, and Finalization.
- [x] Update the `run_compose_pipeline_internal` phase comment to mention the four phases.
- [x] Document schema validation as a pre-operation stage that runs after frontmatter interpolation and before frontmatter shell expansion.
- [x] Document `ComposePhase::Finalization` as root-only.
- [x] Update `darkmatter/docs/topics/context-variables.md` or compose topic docs if they list phases.
- [x] Update `.claude/skills/darkmatter/SKILL.md` if the compose pipeline summary drifts.

### 4d. God-File Guardrails

- [x] Record the extraction rules in an existing design or maintenance doc near `render_tree/fold.rs` or `compose/mod.rs`; do not create a broad process doc just for this.
- [x] Ensure the rule is observable: new block extensions extend a dedicated `markdown/render_tree/` module; new compose stages come with a stage module and descriptor metadata; large in-file test suites move to a sibling `tests` module when production code around them changes.

**Validation checkpoint:**
- `cargo test -p darkmatter --lib markdown::compose` passes.
- `cargo test -p darkmatter-cli compose` passes.
- Descriptor invariant tests pass.
- Docs build or `cargo doc -p darkmatter --no-deps` completes without new warnings for touched items.

---

## Phase 5: Final Verification and Acceptance

**Goal:** Confirm the entire maintenance batch is behavior-preserving and meets the acceptance criteria.

- [x] Run `cargo test -p darkmatter --lib markdown::compose`.
- [x] Run `cargo test -p darkmatter --lib markdown::dsl`.
- [x] Run `cargo test -p darkmatter-cli code_block`.
- [x] Run `cargo test -p darkmatter-cli render`.
- [x] Run `cargo test -p darkmatter-cli compose`.
- [x] Run the package-area recipe: `cd darkmatter && just test`.
- [x] Run `rg -n "find_syntax_by_|from_fence_token|SyntaxSet::load_defaults_newlines" darkmatter -S` and confirm production hits outside `language_grammar.rs` are gone; allowed test-only hits remain in `#[cfg(test)]` helpers and syntax-set loading tests.
- [x] Verify the two artifact files are still absent.
- [x] Verify no assertion-free debug repro remains in frontmatter interpolation tests.
- [x] Verify CLI command implementations are split by command family with no duplicated code-block Markdown serialization.
- [x] Verify `parse_code_info` and CLI `--highlight` share one highlight parser.
- [x] Verify Markdown, JSON, and non-TTY auto render paths do not resolve themes.
- [x] Verify compose Markdown/Auto and JSON paths do not resolve themes.
- [x] Verify compose operation metadata has one descriptor source plus invariant tests.
- [x] Verify compose docs consistently describe four phases and root-only finalization.
- [x] Verify existing CLI behavior, compose output, and render snapshots remain stable.
- [x] Update `darkmatter/reviews/2026-06-14-summary-and-suggest/fix.md` frontmatter `status` if needed, or record completion in a tracking comment.

**Acceptance criteria (all must pass):**
- The `LanguageGrammar` single-authority rule is honored: no production direct `SyntaxSet::find_syntax_by_*` lookup exists outside `language_grammar.rs`, and no in-tree caller references `from_fence_token`.
- The two editor artifact files are absent.
- No assertion-free debug repro remains in frontmatter interpolation tests.
- CLI command implementations are split by command family, with no duplicated code-block Markdown serialization.
- `parse_code_info` and CLI `--highlight` share one highlight parser.
- Markdown, JSON, and non-TTY auto render paths do not resolve themes.
- Compose Markdown/Auto and JSON paths do not resolve themes.
- Compose operation metadata has one descriptor source plus invariant tests.
- Compose docs consistently describe four phases and root-only finalization.
- Existing CLI behavior, compose output, and render snapshots remain stable.

---

## Parallelizable Work

Phase 1 cleanup tasks can run in parallel with one another.
Phase 2a (code-block serialization) and Phase 2b (highlight parser) can be worked in parallel after Phase 1.
Phase 3 (lazy theme resolution) can run in parallel with Phase 2 once the CLI command module boundaries are validated.
Phase 4c (documentation) can be drafted in parallel with Phase 4a/4b but should be finalized only after descriptor shape is locked.

## Dependency Summary

| Phase | Depends on |
|-------|-----------|
| 1 | nothing |
| 2 | Phase 1 |
| 3 | Phase 1 |
| 4 | Phase 1, 2, 3 |
| 5 | all prior phases |