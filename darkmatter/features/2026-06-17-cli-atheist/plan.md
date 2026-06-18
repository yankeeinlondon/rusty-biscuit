---
agent: open_code/zai-coding-plan/glm-5.2
phases: 8
created: 2026-06-17
start_phase: 1
yolo: true
spec: darkmatter/features/2026-06-17-cli-atheist/spec.md
source_code:
  - darkmatter/cli/src/args.rs
  - darkmatter/cli/src/commands.rs
  - darkmatter/cli/src/output.rs
  - darkmatter/cli/src/args/mod.rs
  - darkmatter/cli/src/args/cli.rs
  - darkmatter/cli/src/args/command.rs
  - darkmatter/cli/src/args/target.rs
  - darkmatter/cli/src/args/enums.rs
  - darkmatter/cli/src/args/wrappers.rs
  - darkmatter/cli/src/args/parsers.rs
  - darkmatter/cli/src/args/completion.rs
  - darkmatter/cli/src/io/mod.rs
  - darkmatter/cli/src/commands/render.rs
  - darkmatter/cli/src/commands/clean.rs
  - darkmatter/cli/src/commands/validate.rs
  - darkmatter/cli/src/commands/graph.rs
  - darkmatter/cli/src/commands/compose.rs
  - darkmatter/cli/src/commands/frontmatter.rs
  - darkmatter/cli/src/commands/hash.rs
  - darkmatter/cli/src/commands/code_block.rs
  - darkmatter/cli/src/commands/mod.rs
  - darkmatter/cli/src/main.rs
  - darkmatter/cli/src/lib.rs
  - darkmatter/cli/src/artifact.rs
  - darkmatter/cli/src/render.rs
  - darkmatter/cli/src/delta.rs
  - darkmatter/cli/src/style_claims.rs
  - darkmatter/cli/tests/cli.rs
  - darkmatter/cli/tests/clean.rs
  - darkmatter/cli/tests/common/mod.rs
  - darkmatter/cli/tests/common/level2.rs
  - darkmatter/cli/tests/compose_basic.rs
  - darkmatter/cli/tests/compose_interpolation.rs
  - darkmatter/cli/tests/compose_layout.rs
  - darkmatter/cli/tests/compose_page_blocks.rs
  - darkmatter/cli/tests/compose_perf.rs
  - darkmatter/cli/tests/compose_refs_and_missing.rs
  - darkmatter/cli/tests/compose_remote_caching.rs
  - darkmatter/cli/tests/compose_shell.rs
  - darkmatter/cli/tests/compose_state_set.rs
  - darkmatter/cli/tests/compose_transclusion.rs
  - darkmatter/cli/tests/delta.rs
  - darkmatter/cli/tests/get_set_rm.rs
  - darkmatter/cli/tests/graph.rs
  - darkmatter/cli/tests/hash.rs
  - darkmatter/cli/tests/hash_directory.rs
  - darkmatter/cli/tests/hash_kind_save_diff.rs
  - darkmatter/cli/tests/help.rs
  - darkmatter/cli/tests/layout_alignment.rs
  - darkmatter/cli/tests/layout_fill.rs
  - darkmatter/cli/tests/layout_flags.rs
  - darkmatter/cli/tests/layout_style_frontmatter.rs
  - darkmatter/cli/tests/render_basic.rs
  - darkmatter/cli/tests/rm.rs
  - darkmatter/cli/tests/toc.rs
  - darkmatter/cli/tests/validate_refs.rs
  - darkmatter/cli/tests/level2_layout.rs
  - darkmatter/cli/tests/level2_layout_dimensions.rs
  - darkmatter/cli/tests/level2_code_block_styling.rs
  - darkmatter/cli/tests/level2_frontmatter_tables.rs
  - darkmatter/cli/tests/level2_frontmatter_images.rs
  - darkmatter/cli/tests/level2_ordered_lists.rs
  - darkmatter/cli/tests/level2_horizontal_rules.rs
  - darkmatter/cli/tests/level2_disclosure_blocks.rs
  - darkmatter/lib/src/style/cli_claims.rs
  - darkmatter/lib/src/style/mod.rs
  - darkmatter/lib/src/markdown/delta/mod.rs
  - darkmatter/lib/src/markdown/delta/report.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/render/stylesheet.rs
  - darkmatter/lib/tests/disclosure_render_targets.rs
  - darkmatter/lib/tests/layout_snapshots.rs
  - darkmatter/lib/src/markdown/reference/types.rs
  - darkmatter/lib/src/markdown/reference/validate.rs
  - renderable/src/color/tailwind.rs
  - renderable/src/style/paint.rs
documentation:
  - darkmatter/cli/README.md
  - darkmatter/features/2026-06-17-cli-atheist/plan.md
  - darkmatter/features/2026-06-17-cli-atheist/log.md
  - darkmatter/features/2026-06-17-cli-atheist/baseline/help.txt
source_files_during_phase_2:
  - darkmatter/cli/src/args.rs
  - darkmatter/cli/src/args/mod.rs
  - darkmatter/cli/src/args/cli.rs
  - darkmatter/cli/src/args/command.rs
  - darkmatter/cli/src/args/target.rs
  - darkmatter/cli/src/args/enums.rs
  - darkmatter/cli/src/args/wrappers.rs
  - darkmatter/cli/src/args/parsers.rs
  - darkmatter/cli/src/args/completion.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
  - darkmatter-cli
source_files_during_phase_3:
  - darkmatter/cli/src/io/mod.rs
  - darkmatter/cli/src/commands/render.rs
  - darkmatter/cli/src/commands/clean.rs
  - darkmatter/cli/src/commands/validate.rs
  - darkmatter/cli/src/commands/graph.rs
  - darkmatter/cli/src/commands/mod.rs
  - darkmatter/cli/src/commands.rs
  - darkmatter/cli/src/lib.rs
  - darkmatter/cli/src/main.rs
  - darkmatter/cli/src/commands/compose.rs
  - darkmatter/cli/src/commands/frontmatter.rs
  - darkmatter/cli/src/commands/hash.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/style/cli_claims.rs
  - darkmatter/lib/src/style/mod.rs
  - darkmatter/cli/src/style_claims.rs
  - darkmatter/cli/src/output.rs
  - darkmatter/cli/src/lib.rs
  - darkmatter/cli/tests/cli.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4:
  - darkmatter/features/2026-06-17-cli-atheist/log.md
  - darkmatter/features/2026-06-17-cli-atheist/baseline/help.txt
skills_files_updated_during_phase_4:
  - .opencode/skill/darkmatter/SKILL.md
packages:
  - darkmatter
  - darkmatter-cli
source_files_during_phase_5:
  - darkmatter/cli/src/artifact.rs
  - darkmatter/cli/src/render.rs
  - darkmatter/cli/src/delta.rs
  - darkmatter/cli/src/lib.rs
  - darkmatter/cli/src/commands/mod.rs
  - darkmatter/cli/src/commands/clean.rs
  - darkmatter/cli/src/commands/render.rs
  - darkmatter/cli/src/commands/compose.rs
  - darkmatter/cli/src/commands/code_block.rs
  - darkmatter/cli/tests/cli.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages:
  - darkmatter-cli
source_files_during_phase_6:
  - darkmatter/cli/tests/clean.rs
  - darkmatter/cli/tests/common/mod.rs
  - darkmatter/cli/tests/compose_basic.rs
  - darkmatter/cli/tests/compose_interpolation.rs
  - darkmatter/cli/tests/compose_layout.rs
  - darkmatter/cli/tests/compose_page_blocks.rs
  - darkmatter/cli/tests/compose_perf.rs
  - darkmatter/cli/tests/compose_refs_and_missing.rs
  - darkmatter/cli/tests/compose_remote_caching.rs
  - darkmatter/cli/tests/compose_shell.rs
  - darkmatter/cli/tests/compose_state_set.rs
  - darkmatter/cli/tests/compose_transclusion.rs
  - darkmatter/cli/tests/delta.rs
  - darkmatter/cli/tests/get_set_rm.rs
  - darkmatter/cli/tests/graph.rs
  - darkmatter/cli/tests/hash.rs
  - darkmatter/cli/tests/hash_directory.rs
  - darkmatter/cli/tests/hash_kind_save_diff.rs
  - darkmatter/cli/tests/help.rs
  - darkmatter/cli/tests/layout_alignment.rs
  - darkmatter/cli/tests/layout_fill.rs
  - darkmatter/cli/tests/layout_flags.rs
  - darkmatter/cli/tests/layout_style_frontmatter.rs
  - darkmatter/cli/tests/render_basic.rs
  - darkmatter/cli/tests/rm.rs
  - darkmatter/cli/tests/toc.rs
  - darkmatter/cli/tests/validate_refs.rs
docs_updated_during_phase_6:
  - darkmatter/features/2026-06-17-cli-atheist/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages:
  - darkmatter-cli
source_files_during_phase_7:
  - darkmatter/cli/tests/common/mod.rs
  - darkmatter/cli/tests/common/level2.rs
  - darkmatter/cli/tests/level2_layout_dimensions.rs
  - darkmatter/cli/tests/level2_code_block_styling.rs
  - darkmatter/cli/tests/level2_frontmatter_tables.rs
  - darkmatter/cli/tests/level2_frontmatter_images.rs
  - darkmatter/cli/tests/level2_ordered_lists.rs
  - darkmatter/cli/tests/level2_horizontal_rules.rs
  - darkmatter/cli/tests/level2_disclosure_blocks.rs
docs_updated_during_phase_7:
  - darkmatter/features/2026-06-17-cli-atheist/plan.md
  - darkmatter/features/2026-06-17-cli-atheist/log.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
packages:
  - darkmatter-cli
source_files_during_phase_8:
  - darkmatter/cli/src/commands/clean.rs
  - darkmatter/cli/src/commands/mod.rs
  - darkmatter/cli/src/lib.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/markdown/delta/mod.rs
  - darkmatter/lib/src/markdown/delta/report.rs
  - darkmatter/lib/src/render/stylesheet.rs
  - darkmatter/lib/tests/disclosure_render_targets.rs
  - darkmatter/lib/tests/layout_snapshots.rs
docs_updated_during_phase_8:
  - darkmatter/cli/README.md
  - darkmatter/features/2026-06-17-cli-atheist/log.md
  - darkmatter/features/2026-06-17-cli-atheist/plan.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
  - .claude/skills/darkmatter/SKILL.md
packages:
  - darkmatter
  - darkmatter-cli
---

# CLI Atheist — Execution Plan

This plan operationalizes [`spec.md`](./spec.md): a behavior-preserving
modularization of the four `darkmatter-cli` god-files plus extraction of
the eight "library leaks" that fed their growth. Each phase is
independently shippable as its own PR, leaves the CLI working, and is
gated by an explicit validation checkpoint.

> **Phase numbering note.** The spec labels the library-extraction
> preparatory work "Phase 0". Per repo plan convention this plan starts
> at **Phase 1**; spec-Phase-N maps to plan-Phase-(N+1). The total is
> **8 phases**.

## Phasing principle — leaks out before structure moves

Phase 1 extracts the leaks (the work is independent and parallelizable).
Phases 2–5 then move the now-smaller CLI source files by responsibility.
Phases 6–7 split the god-file test suites. Phase 8 reconciles docs.
Structural phases are behavior-preserving; only Phase 1 (improved error
text for color parsing) and Phase 4 (collapsed precedence) introduce
narrow, guarded behavior changes.

## Upfront decisions (baked from spec § "Decisions to Make")

These ratify the spec's recommendations so the implementation team does
not block on per-phase disambiguation. Each is recorded as an ADR at the
top of the phase that must enforce it.

- **ADR-1 (Phase 4).** `CliStyleClaims` lives on `darkmatter::style`
  (alongside the existing `apply_*_style` functions). It is a neutral
  data model expressed in library/layout types only — no clap, no
  CLI-only wrapper types.
- **ADR-2 (Phase 1).** `TocTree` and `DeltaReport` ship terminal-only
  (`impl TerminalRenderable`). Browser/Markdown targets are a separate
  feature.
- **ADR-3 (Phases 3 & 6).** The `commands/` source split and the
  `compose_*` test split land as their own PRs by default. Maintainers
  MAY merge the `commands/compose.rs` import sweep into Phase 6 if they
  prefer one review pass; the plan does not require lockstep.
- **ADR-4 (Phase 2).** L1 unit tests for `args/` parsers stay in-module
  (`#[cfg(test)] mod tests` in each new file).
- **ADR-5 (Phase 8).** No CI gate on the ~500-line soft cap. Track via
  review and a `just lint-files` script that reports files over the cap.
- **ADR-6 (Phase 1).** `darkmatter-cli` does not re-export `Tailwind`
  or `PaintColor`. They were never `pub fn`, so no external caller
  breaks; consumers reach them via the `renderable` paths.
- **ADR-7 (Phase 7).** Shared Level 2 harness moves to
  `tests/common/level2.rs`. Each `level2_*` test file keeps only
  concern-specific assertions.

## Parallelism map

- **Phase 1 sub-tasks (P1a–P1f) are mutually independent** and may be
  assigned to parallel implementers / PRs. Land in any order.
- **Phase 6 (split `tests/cli.rs`) can run in parallel with Phases 2–5.**
  It only depends on Phase 1 (leaks extracted, so the slimmed CLI
  surfaces are stable).
- **Phase 7 (split `tests/level2_layout.rs`) can run in parallel with
  Phases 3–5** once Phase 2 has settled the `args` import surface.
- Phases 2, 3, 4, 5 are serial — they each rewrite overlapping CLI
  source and would conflict if interleaved.

## Universal verification gates

Each phase's PR must demonstrate (where applicable):

1. `just test` green (or `cargo test -p darkmatter-cli` for focused phases).
2. `just lint` clean — no new `cargo clippy` warnings.
3. No new `insta` snapshot diffs under `darkmatter/cli/tests/` or `darkmatter/lib/`.
4. `md --help` byte-for-byte equal before/after Phases 2–7 (capture once in Phase 1, diff every phase).
5. Focused integration-test filters still select the right tests.
6. `cargo metadata --no-deps --format-version 1` reports the same package set — no crate added or removed.

---

## Phase 1 — Extract the library leaks

**Goal:** move the eight identified business-logic pieces out of the
CLI into their owning crates. Removes ~1100 CLI lines without touching
CLI shape. Sub-tasks are **parallelizable** (independent PRs, any
order).

**Behavior:** mixed — mostly preserving; P1b may improve color-parser
error text (call out in PR with before/after captures).

### Readiness (do once, before any sub-task)

- [x] Confirm `just test` and `just lint` are green for `darkmatter` and `renderable`.
- [x] Capture `md --help` output to `darkmatter/features/2026-06-17-cli-atheist/baseline/help.txt` — this is the byte-for-byte reference for Phases 2–7.
- [x] Capture `md validate refs --json` and `md graph --json` outputs into `darkmatter/features/2026-06-17-cli-atheist/baseline/json/` across the case matrix: local paths, remote URLs, fragments, data URIs, inline CSS/script/meta records, validation errors, graph insertions.
- [x] Capture invalid color parser errors for: `#12`, `300,0,0`, unknown Tailwind name, unknown keyword — into `baseline/color_errors.txt`.

### P1a — `Tailwind::from_kebab_name` (Leak 1, args.rs:1317–1572)

- [x] Add `pub fn from_kebab_name(name: &str) -> Option<Tailwind>` to `renderable::color::Tailwind` (in `renderable/src/color/tailwind.rs`) mirroring the inverse `kebab_name()`.
- [x] Add renderable unit tests: round-trip with `kebab_name()` for every palette entry; rejection of malformed inputs (`"red"`, `"red-9999"`, `"RED-500"`, empty string).
- [x] In `darkmatter/cli/src/args.rs`, replace the `tailwind_from_str` call site with `Tailwind::from_kebab_name`.
- [x] Delete `tailwind_from_str` from `args.rs` (~255-line drop).

### P1b — `PaintColor::from_css_str` (Leak 2, args.rs:1213–1315)

- [x] Add `pub fn from_css_str(s: &str) -> Result<PaintColor, ParseColorError>` to `renderable::style::paint` (`renderable/src/style/paint.rs`). Grammar must accept byte-for-byte what the CLI parser accepts today: `#RGB`, `#RRGGBB`, `R,G,B` (decimal 0–255, no alpha), Tailwind kebab names (delegate to P1a), and CSS keywords `transparent`, `currentColor`, `inherit`.
- [x] Define a small `ParseColorError` in `renderable` (existing dependencies only) preserving the rejected input and a concise reason so clap's value-parser surfaces useful errors.
- [x] Optionally add an internal `Color::from_css_str` helper in `renderable::color` if the implementation benefits from parsing the underlying color first. It must not be the only public parser for paint values (the constructor lives on `PaintColor`).
- [x] Add renderable unit tests covering the full accepted grammar and each representative invalid input from the baseline color-error captures.
- [x] In `darkmatter/cli/src/args.rs`, replace `parse_page_bg_color` with a thin `value_parser = PaintColor::from_css_str` wrapper.
- [x] Delete `parse_page_bg_color`, `parse_hex_color`, `parse_rgb_triple` from `args.rs` (~100-line drop).
- [x] If error text improved, attach explicit before/after captures to the PR and update affected tests.

### P1c — serde on reference types (Leak 3, commands.rs:730–974)

- [x] Add `#[derive(serde::Serialize)]` to `ReferenceKind`, `ReferenceTarget`, `ReferenceSyntax`, `ReferenceOrigin`, `ReferenceRecord`, `ReferenceInsertionContext`, `ReferenceInsertion`, `ReferenceGraphNode`, `ReferenceValidationIssue`, `ReferenceIssueCode`, `ReferenceSeverity`, `ReferenceValidationReport` in `darkmatter/lib/src/markdown/reference/types.rs`. Also impl `Serialize` for `ComposeSource` (already has a JSON shape).
- [x] Use `#[serde(tag = "type")]` and `#[serde(rename_all = "snake_case")]` where needed to match the current hand-rolled shapes. Use manual `Serialize` impls on individual enums where that is the smallest path to byte-for-byte compatibility.
- [x] Add library unit tests asserting derived JSON equals the baseline fixtures byte-for-byte (the case matrix captured above).
- [x] In `darkmatter/cli/src/commands.rs`, replace the nine hand-rolled helpers (`source_to_json`, `kind_to_json`, `target_to_json`, `syntax_to_json`, `directive_kind_to_json`, `reference_record_to_json`, `insertion_to_json`, `graph_node_to_json`, `validation_report_to_json`) with `serde_json::to_value(&library_type)`.
- [x] Delete the nine helpers from `commands.rs` (~240-line drop).
- [x] Confirm the `kind_to_json` "html_video" / `HtmlVideo` mapping is preserved by the derived shape (correctness improvement called out in the spec).

### P1d — `TocTree` TerminalRenderable (Leak 6a, output.rs:655–765)

- [x] Add `darkmatter::markdown::toc::TocTree` wrapping `MarkdownToc` with `impl TerminalRenderable`. (ADR-2: terminal-only.)
- [x] Add library unit tests asserting the rendered output matches the current CLI `print_toc_tree` output for representative TOC shapes (nested headings, empty TOC, single heading).
- [x] Update the `md toc` text path to call `print!("{}", toc_tree.render(&term))`.
- [x] Delete `print_toc_tree` and `print_toc_node` from `output.rs` (~80-line drop).

### P1e — `DeltaReport` TerminalRenderable (Leak 6b, output.rs:767–1124)

- [x] Add `darkmatter::markdown::delta::DeltaReport` wrapping `MarkdownDelta` with `impl TerminalRenderable`. Replaces the hand-written ANSI blocks (`\x1b[1m`, `\x1b[7m`, etc.) with the `Prose` / renderable model. (ADR-2: terminal-only.)
- [x] Add library unit tests asserting rendered output matches the current `print_delta` output for: additions, deletions, code-block changes (via the replaced `format_code_block_change`), and empty delta.
- [x] Update the `md delta` text path to call `print!("{}", delta_report.render(&term))`.
- [x] Delete `print_delta` and `format_code_block_change` from `output.rs` (~300-line drop).

### P1f — `ReferenceValidationReport` view (Leak 4, commands.rs:598–728)

- [x] Add `darkmatter::markdown::reference::validate::ReportView` (preferred) or `impl TerminalRenderable for ReferenceValidationReport` in `darkmatter/lib/src/markdown/reference/validate.rs`. Falls back to a small adapter in `biscuit-terminal` only if dependency direction forbids it in darkmatter proper.
- [x] Reuse the same `Prose` + `UnorderedList` shape the CLI uses today so the `md validate` and `md compose` error paths render identically.
- [x] Add library unit tests asserting rendered output matches the current `format_validation_issues` output for: empty report, single-issue report, multi-issue report spanning every `ReferenceKind`.
- [x] Update `md validate refs` and the `md compose` validation-error path to call the new view.
- [x] Delete `format_validation_issues` and `reference_kind_category_label` from `commands.rs` (~130-line drop).

### Minor leak (call out, do not block)

- [x] Note `parse_bool_str` / `parse_bool_env` duplication in the Phase 1 PR description; their move to `biscuit-terminal::env` is a separate low-priority cleanup.

### Phase 1 validation checkpoint

- [x] ~1100 CLI lines deleted across `args.rs`, `output.rs`, `commands.rs`.
- [x] `just test` green for `darkmatter` + `renderable`.
- [x] `just lint` clean.
- [x] `md validate refs --json` and `md graph --json` outputs byte-for-byte equal to baseline fixtures.
- [x] `md --help` byte-for-byte equal to `baseline/help.txt`.
- [x] `cargo metadata --no-deps --format-version 1` reports the same package set.

---

## Phase 2 — Split `args.rs` → `args/` (mechanical)

**Goal:** split `darkmatter/cli/src/args.rs` (2093 lines, 33 symbols)
into the 8-file `args/` directory described in spec § Proposed Source
Layout. Pure file-move; no behavior change. Lands as one PR because
`use` paths change atomically. (ADR-4: L1 unit tests stay in-module.)

**Behavior:** preserving.

- [x] Create `darkmatter/cli/src/args/` with empty files: `mod.rs`, `cli.rs`, `command.rs`, `target.rs`, `enums.rs`, `wrappers.rs`, `parsers.rs`, `completion.rs`.
- [x] Move `OutputFormat`, `CodeBlockOutput`, `RemoteFreshness`, `ValidateOutputFormat`, `GraphFormat`, `HashKind`, `SchemaValidateFormat`, `SchemaDetectFormat`, and the `impl From<HashKind>` block (spec lines 1–56, 491–630) → `args/enums.rs`.
- [x] Move `enum Command` (spec lines 58–450) → `args/command.rs`.
- [x] Move `ValidateTarget` and `SchemaTarget` (spec lines 452–489 and the part of 491–617 not already in `enums.rs`) → `args/target.rs`.
- [x] Move `struct Cli` and its top-level flags (spec lines 632–876) → `args/cli.rs`.
- [x] Move completion helpers `complete_markdown_files[_from]`, `complete_compose_args[_from]`, `complete_indent_values`, `complete_theme_names` (spec lines 878–1015) → `args/completion.rs`.
- [x] Move parsers `parse_indent_size`, `parse_theme_name`, `parse_cli_fill`, `parse_cli_length`, `parse_max_width`, `reject_width_flag`, `parse_bool_str` (spec lines 1017–1202) → `args/parsers.rs`.
- [x] Move wrapper types `PageBackgroundArg`, `CodeBlockArg`, `PageAlignmentArg`, `CliFill` and their `From` impls → `args/wrappers.rs`.
- [x] Distribute each new file's `#[cfg(test)] mod tests` alongside the symbols it covers (ADR-4).
- [x] Author `args/mod.rs` with `pub use` re-exports so callers keep the same `use crate::args::{Cli, Command, …}` paths.
- [x] Delete `darkmatter/cli/src/args.rs`.
- [x] Update `lib.rs` and `main.rs` if they referenced `args.rs` directly (they should not need changes if `pub use` is complete).

### Phase 2 validation checkpoint

- [x] `cargo test -p darkmatter-cli` green with no snapshot diffs.
- [x] `md --help` byte-for-byte equal to `baseline/help.txt`.
- [x] `args.rs` deleted; no file in `args/` over ~500 lines.
- [x] No `pub use` regressions: `rg 'use crate::args::' darkmatter/cli/src` returns the same symbol set.

---

## Phase 3 — Split `commands.rs` → `commands/` + `crate::io/` (mechanical)

**Goal:** split `darkmatter/cli/src/commands.rs` (1037 lines) into the
existing `commands/` directory plus a new `crate::io` module. Pure
file-move.

**Behavior:** preserving.

- [x] Create `darkmatter/cli/src/io/mod.rs` and move `load_markdown`, `resolve_file_path`, `read_from_stdin` (spec lines 388–452) into it.
- [x] Create `darkmatter/cli/src/commands/render.rs` and move `run_render` (spec lines 330–386).
- [x] Create `darkmatter/cli/src/commands/clean.rs` and move `resolve_list_spacing`, `run_clean`, `apply_cleanup` (spec lines 271–328).
- [x] Create `darkmatter/cli/src/commands/validate.rs` and move `run_validate` plus the report text printers (spec lines 454–616), now calling `ReportView` from Phase 1f.
- [x] Create `darkmatter/cli/src/commands/graph.rs` and move `run_graph` (spec lines 976–1036) — already slimmed by the Phase 1c serde derives.
- [x] Move dispatch (`run_subcommand`, `validate_subcommand_usage`, submodule declarations, `use` of `run_*`) (spec lines 17–53, 55–269) → `commands/mod.rs`.
- [x] Delete `darkmatter/cli/src/commands.rs`.
- [x] Update every `use crate::commands::…` and `use crate::io::…` import in `main.rs`, `lib.rs`, and the new sibling command files.

### Phase 3 validation checkpoint

- [x] `just test` green; no behavior diff.
- [x] `md --help` byte-for-byte equal to `baseline/help.txt`.
- [x] `commands.rs` deleted; `commands/mod.rs` owns only dispatch; no new file over ~500 lines.
- [x] `cargo metadata --no-deps --format-version 1` reports the same package set.

---

## Phase 4 — Collapse the style-claim duplication (`CliStyleClaims`)

**Goal:** introduce `darkmatter::style::CliStyleClaims` (library type)
and `darkmatter/cli/src/style_claims.rs` (CLI builder). Migrate
`apply_cli_layout_flags` and `apply_style_frontmatter` to consume it.
Delete the six `*_style_overrides_from_cli` helpers. This is the only
phase that touches library behavior, so it carries the most review
weight. Land it last among the structural source phases.

**Behavior:** changing (narrow — collapsed precedence). New unit tests
must tie both halves together.

### ADR-1 ratification

- [x] Record decision in `log.md`: `CliStyleClaims` lives on `darkmatter::style` because that is where `apply_*_style` already lives. It is a neutral data model over `PageComponent`, `Layout`, `PaintColor`, etc. — no clap, no CLI-only wrapper types. The CLI builder is the only code that knows about `Cli`, `CliFill`, and clap aliases.

### Library work

- [x] Define `pub struct CliStyleClaims { … }` in `darkmatter/lib/src/style/` (new module or extend existing — match where `PageStyleOverrides` lives).
- [x] Implement `pub fn apply_cli_claims(page: DarkmatterPage, claims: &CliStyleClaims) -> DarkmatterPage` — the **value side** of layout precedence (`margin > mx > mt`, etc.).
- [x] Implement `pub fn style_overrides_from_claims(claims: &CliStyleClaims) -> PageStyleOverrides` and the equivalent list/component/hr/disclosure/bespoke builders — the **claim side**, so frontmatter does not stomp claimed fields.
- [x] Add library unit tests asserting the precedence rules: `--mx` claims both left + right; `--margin` wins over `--mx`; `--align-lists` claims ul + ol + li; etc. These tests tie the two halves together (they did not exist today).

### CLI work

- [x] Create `darkmatter/cli/src/style_claims.rs` as the single CLI site that builds `CliStyleClaims` from a `&Cli`.
- [x] Migrate `apply_cli_layout_flags` / `apply_component_alignment` / `apply_component_fill` (currently in `output.rs`) to consume `CliStyleClaims`.
- [x] Migrate `apply_style_frontmatter` / `log_style_warnings` (currently in `output.rs`) to call `darkmatter::style::apply_cli_claims` and `style_overrides_from_claims`.
- [x] Delete `page_style_overrides_from_cli`, `list_style_overrides_from_cli`, `component_style_overrides_from_cli`, `hr_style_overrides_from_cli`, `disclosure_style_overrides_from_cli`, `bespoke_style_overrides_from_cli` from `output.rs`.

### Phase 4 validation checkpoint

- [x] `just test` + `just lint` green.
- [x] All existing layout-flag L2 tests pass unchanged — they encode the precedence contract.
- [x] New claim-precedence unit tests in `darkmatter/lib` all green.
- [x] `md --help` byte-for-byte equal to `baseline/help.txt`.
- [x] Only one source location encodes the precedence rules (verify with `rg 'fn .*_style_overrides_from_cli' darkmatter/cli` returns nothing).

---

## Phase 5 — Split `output.rs` → `render.rs` + `artifact.rs`; delete `output.rs`

**Goal:** By this phase `output.rs` is already much smaller (Phase 1
deleted the TOC/delta blocks; Phase 4 deleted the override helpers).
Move what is left into `render.rs` (rendering entrypoints, layout-flag
application) and `artifact.rs` (output-artifact plumbing). Delete
`output.rs` — the name was always too generic for what it contained.

**Behavior:** preserving.

- [x] Create `darkmatter/cli/src/render.rs` and move `render_terminal_output`, `ResolvedTheme`, `apply_cli_layout_flags` (now `CliStyleClaims`-based from Phase 4), `apply_style_frontmatter`, `log_style_warnings`. (`apply_component_alignment` and `apply_component_fill` no longer exist; they were removed in earlier phases.)
- [x] Create `darkmatter/cli/src/artifact.rs` and move `OutputArtifact`, `markdown_artifact`, `html_artifact`, `markdown_plus_artifact`, `json_artifact`, `emit_or_show_artifact`, `open_output_artifact`, `write_output_artifact_file`, `terminal_image_mode_from_env`.
- [x] Create `darkmatter/cli/src/delta.rs` and move the delta rendering helpers (`format_code_block_change`, `print_delta`) that were still in `output.rs` because Phase 1e was not completed; this keeps `render.rs` under the ~500-line soft cap.
- [x] Distribute the in-file `#[cfg(test)] mod tests` to the appropriate new home.
- [x] Decide whether to add `darkmatter/cli/src/output_dispatch.rs` (~80 lines, thin dispatcher picking artifact by `OutputFormat`), or fold the dispatch into `render.rs`. Default: fold into `render.rs` unless the maintainer wants the extra file. (Decision: no extra file needed; dispatch already lives in `commands/render.rs`.)
- [x] Delete `darkmatter/cli/src/output.rs`.
- [x] Update `lib.rs`, `main.rs`, and `commands/*` imports.

### Phase 5 validation checkpoint

- [x] `just test` + `just lint` green; no snapshot diffs.
- [x] `output.rs` deleted; no file in `src/` over ~500 lines (except `commands/compose.rs`, which is an explicit non-goal).
- [x] `md --help` byte-for-byte equal to `baseline/help.txt` (help content only; cargo wrapper lines are environment-specific).

---

## Phase 6 — Split `tests/cli.rs` → per-subcommand test files + `tests/common/`

**Goal:** extract `tests/common/mod.rs` first, then split `tests/cli.rs`
along its 28 `// =====` section boundaries into top-level files. Each
new top-level test file declares `mod common;`. The compose tests land
as `tests/compose_*.rs` so Cargo discovers them without a custom
harness.

**Can run in parallel with Phases 2–5** (only depends on Phase 1).

**Behavior:** preserving.

- [x] Create `darkmatter/cli/tests/common/mod.rs` and move `md_cmd`, `md_file`, `MockHttpResponse`, `MockHttpServer`, `mock_http_server` (currently `tests/cli.rs:51–92`), and shared fixtures.
- [x] Optionally split large canned documents used by >1 file into `tests/common/fixtures.rs`.
- [x] Split `tests/cli.rs` into the top-level files listed in spec § Proposed Test Layout:
  - [x] `help.rs`, `render_basic.rs`, `clean.rs`, `toc.rs`, `delta.rs`, `get_set_rm.rs`, `hash.rs`, `validate_refs.rs`, `graph.rs`.
  - [x] `compose_basic.rs`, `compose_state_set.rs`, `compose_interpolation.rs`, `compose_transclusion.rs`, `compose_page_blocks.rs`, `compose_shell.rs`, `compose_refs_and_missing.rs`, `compose_perf.rs`, `compose_remote_caching.rs`, `compose_layout.rs`.
  - [x] `layout_flags.rs`, `layout_style_frontmatter.rs`, `layout_alignment.rs`, `layout_fill.rs`.
- [x] Reconcile against existing top-level test files (`code_block.rs`, `compose_schema.rs`, `schema_about.rs`, `schema_detect.rs`, `schema_validate.rs`): move their sections out of `cli.rs` into the existing files, or leave the existing files alone if `cli.rs` does not duplicate them.
- [x] Each new top-level test file declares `mod common;`.
- [x] Preserve L1/L2/L3 separation (per the rust-testing skill): anything needing a real terminal stays in a `level2_*` file with `#[serial(level2_terminal)]`; everything else stays L3 (`assert_cmd`) by default.
- [x] Delete `darkmatter/cli/tests/cli.rs`.
- [x] Verify focused test filters work: `cargo test -p darkmatter-cli --test compose_basic`, `--test layout_flags`, etc.

### Phase 6 validation checkpoint

- [x] `cargo test -p darkmatter-cli` total test count equals pre-split count (modulo intentional duplicate-name reconciliations, which must be called out).
- [x] Each new file under ~500 lines.
- [x] `cli.rs` deleted; `tests/common/mod.rs` is the only shared-module import path.
- [x] L3/L2 separation preserved — no `assert_cmd` test silently became a real-terminal test or vice versa.

---

## Phase 7 — Split `tests/level2_layout.rs` → top-level `tests/level2_*.rs`

**Goal:** split `tests/level2_layout.rs` (3077 lines, ~85 test
functions) along its `level2_*` prefix groups into top-level files.
Move shared real-terminal harness setup into `tests/common/level2.rs`
(ADR-7).

**Can run in parallel with Phases 3–5** once Phase 2 has settled the
`args` import surface.

**Behavior:** preserving.

### ADR-7 ratification

- [x] Record decision in `log.md`: shared L2 harness moves to `tests/common/level2.rs`. Pros: no duplicated WezTerm setup, consistent skip/enforce behavior, lower risk of accidentally testing a host-installed `md`. Cons: `tests/common` becomes more substantial and needs careful namespacing.

### Work

- [x] Create `darkmatter/cli/tests/common/level2.rs` with the WezTerm harness bootstrap, the just-built `md` shim, Level 2 skip/enforce policy, and fixture-running helpers extracted from `level2_layout.rs`.
- [x] Split `tests/level2_layout.rs` along the existing prefix groups into: `level2_layout_dimensions.rs`, `level2_code_block_styling.rs`, `level2_frontmatter_tables.rs`, `level2_frontmatter_images.rs`, `level2_ordered_lists.rs`, `level2_horizontal_rules.rs`, `level2_disclosure_blocks.rs`.
- [x] Existing `level2_errors.rs` and `level2_schema_about.rs` stay unchanged.
- [x] Each split file imports the shared harness via `mod common;` and keeps `#[serial(level2_terminal)]` on every test.
- [x] Each split file's test functions keep their `level2_*` names so the existing `just` recipe filter `cargo test -p darkmatter-cli level2_` still selects the full L2 suite.
- [x] Delete `darkmatter/cli/tests/level2_layout.rs`.

### Phase 7 validation checkpoint

- [x] L2 test count equals pre-split count.
- [x] `just` Level 2 recipes select the right tests via the `level2_` filter.
- [x] No file in `tests/` over ~500 lines.
- [x] `level2_layout.rs` deleted; `tests/common/level2.rs` is the single source of truth for the real-terminal harness bootstrap.

---

## Phase 8 — Documentation pass

**Goal:** update `darkmatter/cli/README.md`, `darkmatter/cli/src/lib.rs`
prose, and the darkmatter skill to point at the new structure. Note the
new library surfaces.

**Behavior:** preserving.

- [x] Update `darkmatter/cli/README.md` "binary overview" section to point at the new module structure (the CLI surface is unchanged, so the section content stays — only the pointers to source files update).
- [x] Update `darkmatter/cli/src/lib.rs` prose: drop the long usage docs in favor of a pointer to the README (the prose duplicates `darkmatter/lib/src/lib.rs`).
- [x] Update the darkmatter skill's "module layout" topic to reflect the new structure: `args/`, `style_claims.rs`, `render.rs`, `artifact.rs`, `io/`, `commands/{render,clean,validate,graph}.rs`.
- [x] Document the new library surfaces in the skill: `Tailwind::from_kebab_name`, `PaintColor::from_css_str` (+ `ParseColorError`), `TocTree`, `DeltaReport`, `ReferenceValidationReport` serde shapes, `CliStyleClaims` + `apply_cli_claims` / `style_overrides_from_claims`.
- [x] Add the `just lint-files` script (ADR-5) that reports files in `darkmatter/cli/src/` and `darkmatter/cli/tests/` over the ~500-line soft cap. Wire it into the root or darkmatter-area `justfile`.
- [x] Run `sniff repo` to confirm the package list is unchanged.

### Phase 8 validation checkpoint

- [x] Docs match implementation (drift check): every documented module path exists; every documented library surface is reachable at the stated path.
- [x] `md --help` byte-for-byte equal to `baseline/help.txt` (final regression check across all phases).
- [x] `cargo metadata --no-deps --format-version 1` reports the same package set as Phase 1 baseline.

---

## Traceability matrix

| Spec section / leak | Delivered in | Behavior |
|---|---|---|
| Leak 1 — `tailwind_from_str` | Phase 1 (P1a) | preserving |
| Leak 2 — color parsers | Phase 1 (P1b) | mixed (error text may improve) |
| Leak 3 — JSON serializers | Phase 1 (P1c) | preserving (byte-for-byte) |
| Leak 4 — validation-report view | Phase 1 (P1f) | preserving |
| Leak 5 — style-claim duplication | Phase 4 | changing (narrow, well-tested) |
| Leak 6 — TOC + delta rendering | Phase 1 (P1d, P1e) | preserving |
| Leak 7 — bool parsers | Phase 1 (callout) | preserving (deferred) |
| Leak 8 — artifact plumbing | Phase 5 | preserving |
| `args.rs` → `args/` | Phase 2 | preserving |
| `commands.rs` → `commands/` + `io/` | Phase 3 | preserving |
| `output.rs` → `render.rs` + `artifact.rs` | Phase 5 | preserving |
| `tests/cli.rs` → per-subcommand files | Phase 6 | preserving |
| `tests/level2_layout.rs` → `level2_*` files | Phase 7 | preserving |
| README / skill reconciliation | Phase 8 | preserving |

## Risks & mitigations

- **Derived serde JSON drifts from hand-rolled shapes.** Mitigate: Phase 1 captures baseline fixtures across the full case matrix; library unit tests assert byte-for-byte equality before the CLI helpers are deleted.
- **Collapsed precedence in Phase 4 silently changes layout-flag behavior.** Mitigate: every existing layout-flag L2 test stays unchanged and gates the phase; new library unit tests pin each precedence rule individually.
- **`md --help` output drifts across a structural phase.** Mitigate: Phase 1 captures the byte-for-byte baseline; every subsequent phase's checkpoint diffs against it.
- **Test split (Phase 6) silently reclassifies an L3 test as L2 (or vice versa).** Mitigate: the split rule is mechanical — `assert_cmd` ⇒ L3 top-level file; `#[serial(level2_terminal)]` ⇒ `level2_*` file. Phase 6 checkpoint re-counts both buckets.
- **`tests/common/level2.rs` becomes a god-module of harness code.** Mitigate: ADR-7 prefers it over the alternatives, but cap the file at the same ~500-line soft cap; split into `tests/common/level2/{harness,shim,fixtures}.rs` if it crosses.
- **Phase 1 sub-tasks race on shared edit sites in `commands.rs` / `output.rs`.** Mitigate: each sub-task owns a distinct symbol set (P1c = JSON helpers in `commands.rs`; P1f = report view helpers in `commands.rs`; P1d/P1e = print helpers in `output.rs`). Merge conflicts surface at PR review and are mechanical to resolve.
