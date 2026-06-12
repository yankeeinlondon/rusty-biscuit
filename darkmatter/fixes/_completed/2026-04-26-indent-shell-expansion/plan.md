---
phases: 5
created: 2026-06-01
start_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - darkmatter/fixes/2026-04-26-indent-shell-expansion/phase-1-findings.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/indent.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/toc_linking/mod.rs
  - darkmatter/lib/src/markdown/compose/toc_linking/render.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/compose/shell_blocks/types.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/parser.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages:
  - darkmatter
---

# Execution Plan: `::shell` and `::shell-block` Indentation Preservation

## Assumptions

- The code behavior is the source of truth where comments or the spec's likely paths have drifted.
- `::shell-block` currently lives under `darkmatter/lib/src/markdown/compose/shell_blocks/`, not at `darkmatter/lib/src/markdown/compose/shell_blocks.rs`.
- This fix uses the directive or opener line's exact leading whitespace as the indent prefix. It does not infer lazy continuation indentation for column-1 directives because this spec explicitly preserves current column-1 behavior.

## Phase 1: Confirm Current Failure Surface

> Findings recorded in [`phase-1-findings.md`](./phase-1-findings.md).

- [x] Reproduce the bug with a focused `::shell` fixture composed through the real compose pipeline, with a 4-space-indented directive under a list item and command output containing at least two non-empty lines plus one blank separator line.
- [x] Reproduce the bug with a focused `::shell-block` fixture using the same nested list shape and multi-line command output. (Discovered an indented `::shell-block` opener currently fails to *parse* — `parse_opener_params` does not strip the leading whitespace — which is a deeper failure than the column-1 splice; documented for Phase 4.)
- [x] Confirm the root-level `::shell` and `::shell-block` fixtures still emit column-1 output before the change, establishing the no-indent baseline.
- [x] Inspect existing tests in `darkmatter/lib/src/markdown/compose/shell_expansion/` and `darkmatter/lib/src/markdown/compose/shell_blocks/` for assertions that depend on column-1 replacement behavior.
- [x] Validation checkpoint: document which existing tests are expected to change, and which root-level tests must remain byte-for-byte unchanged.

## Phase 2: Add Shared Indentation Utility

- [x] Move or copy the existing `toc_linking::render::indent_text` behavior into a neutral compose utility module, such as `darkmatter/lib/src/markdown/compose/indent.rs` or `parse_utils.rs`, so shell expansion and TOC linking can share one implementation.
- [x] Keep the helper byte-preserving: prefix every line with the provided indent, preserve tabs and spaces exactly, preserve trailing whitespace and final newlines, and return unchanged text when either the indent or text is empty.
- [x] Add unit tests for the helper covering multi-line text, blank interior lines, trailing newline behavior, tab indentation, empty text, and empty indent.
- [x] Update `toc_linking` to call the shared helper without changing TOC behavior.
- [x] Validation checkpoint: run the focused TOC-linking tests that cover indentation and cache reuse to prove the extraction did not regress the completed TOC fix.

## Phase 3: Capture and Apply `::shell` Directive Indentation

- [x] Add an `indent: String` field to `ShellDirective` in `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`.
- [x] Update every `ShellDirective` construction site to populate `indent`; use `String::new()` for frontmatter or synthetic directives where indentation is out of scope.
- [x] In `shell_expansion/parser.rs`, compute the indent from the source line by taking the exact prefix before the first non-whitespace byte on the line containing `::shell`.
- [x] Preserve the parser's current directive detection semantics while adding indentation capture; do not normalize tabs, spaces, or mixed whitespace.
- [x] In `Markdown::run_shell_expansion_stage` in `darkmatter/lib/src/markdown/compose/mod.rs`, apply the shared indentation helper to `execution.combined_output()` before pushing the replacement into `apply_replacements_in_reverse`.
- [x] Keep `apply_replacements_in_reverse` span-only and byte-range focused; indentation should be part of the replacement string before splicing.
- [x] Add or update tests for `::shell` covering 4-space list indentation, tab indentation, blank output lines, and root-level no-indent behavior. (Block-quote `> > ::shell ...` is not reachable: the parser only detects lines whose trimmed form starts with `::shell `, and the captured indent is the prefix before the first non-whitespace byte, which is empty for a `>`-led line. Merged-stderr indentation is covered by the shared splice path; `combined_output()` joins streams before re-indentation.)
- [x] Validation checkpoint: compose the new `::shell` fixtures and assert every emitted line, including blank separator lines, starts with the captured indent when the directive is indented.

## Phase 4: Capture and Apply `::shell-block` Opener Indentation

- [x] Extend `block_pairs::BlockPair` or `shell_blocks::types::ShellBlockRegion` to carry the exact leading whitespace from the `::shell-block` opener line. (Added `indent: String` to `ShellBlockRegion`.)
- [x] Populate that indent from `BlockPair::opening_text` or during shell-block parsing, preserving tabs and spaces byte-for-byte. (Captured in `parse_shell_block_region` from the opener line's leading-whitespace prefix; the same change strips that whitespace before `parse_opener_params` so an indented opener no longer fails to parse — the hard parse failure documented in Phase 1.)
- [x] Apply the shared indentation helper to `render::render_block_output(&results)` in `shell_blocks::run_shell_blocks_stage` before adding the replacement for the full block span.
- [x] Decide and encode the empty-block behavior explicitly: an empty rendered block remains empty and should not become an indentation-only string. (`indent_text` returns empty text unchanged; the `commands.is_empty()` branch already pushes `String::new()`.)
- [x] Add or update tests for `::shell-block` mirroring the `::shell` cases: 4-space list indentation, block-quote marker indentation, root-level no-indent behavior, blank separator lines, and tabs. (Block-quote: a `>`-led `::shell-block` opener is not recognized as a directive — `scan_block_pairs` requires the trimmed line to start with `::shell-block` — so it is left as literal text, matching the `::shell` Phase 3 leading-whitespace semantics. Blank separator lines are covered by the multi-command 4-space/tab list tests.)
- [x] Validation checkpoint: assert shell-block output lines start with the captured indent and that root-level shell-block output remains unchanged. (CommonMark structural container assertions are deferred to Phase 5.)

## Phase 5: Structural Validation and Final Review

- [x] Add CommonMark structural assertions for representative `::shell` and `::shell-block` fixtures, using the existing Markdown parser test style in the codebase, to verify generated lines remain nested under their parent list or block quote rather than becoming siblings. (Added `indented_shell_output_is_nested_under_list_item_in_commonmark` / `root_level_shell_output_is_a_sibling_block_in_commonmark` and the `::shell-block` mirrors. They render the composed body to HTML and assert a single `<ul>`/two `<li>` for the nested case versus two `<ul>` siblings for the column-1 baseline — the same HTML-roundtrip style as the existing TOC fix's `ac4_roundtrip_commonmark_list_nesting`. Block-quote markers are unreachable for both directive forms, as documented in Phases 3-4.)
- [x] Run the narrow package tests for the touched modules, starting with `cargo test -p darkmatter shell_expansion --color=never` and `cargo test -p darkmatter shell_blocks --color=never`; adjust the exact filters to match actual test names. (Ran as `--lib` filters: shell_expansion 362 passed, shell_blocks 82 passed.)
- [x] Run the broader darkmatter compose test surface with `cargo test -p darkmatter compose --color=never` if the focused tests pass. (1537 passed.)
- [x] Review nearby rustdoc and inline comments for drift caused by the behavior change, especially comments describing shell replacement, shell-block rendering, and `BlockPair` fields. (No drift: the `run_shell_expansion_stage` splice comment, `apply_replacements_in_reverse` span-only doc, `ShellBlockRegion::indent` doc, and `render_block_output` re-indent comment all accurately describe the new behavior. `block_pairs.rs` was not modified, so no `BlockPair` field-doc drift.)
- [x] Confirm no frontmatter shell expansion tests or code paths changed behavior; frontmatter `$(...)` values remain scalar expansion only. (`frontmatter_shell_expansion.rs` only gained `indent: String::new()` at its `ShellDirective` construction site; all frontmatter shell-expansion integration tests pass unchanged.)
- [x] Final validation checkpoint: all acceptance criteria in `spec.md` are either directly covered by named tests or explicitly mapped to existing tests that still pass.

## Parallelizable Work

- [ ] Phase 1 fixture discovery for `::shell` and `::shell-block` can run in parallel after the relevant test helpers are identified.
- [ ] Phase 2 shared-helper tests can be implemented in parallel with Phase 3 parser field plumbing once the helper API is agreed.
- [ ] Phase 3 `::shell` tests and Phase 4 `::shell-block` tests can be written in parallel after the shared helper exists.
- [x] Phase 5 comment review can run in parallel with the final test pass, provided any comment edits are rechecked after code changes settle. (Comment review found no drift, so no edits were needed; the final test pass ran clean.)

## Risk Controls

- [x] Avoid adding indentation to cached or source-derived command outputs before caller-local context is known; indentation belongs at the replacement/render boundary. (Indent is applied via `indent_text` at the splice boundary in `run_shell_expansion_stage` and `run_shell_blocks_stage`, never on cached `combined_output()`/`render_block_output` inputs.)
- [x] Avoid changing shell execution output semantics in `executor.rs`; the fix should only affect how captured output is spliced into Markdown body content. (`executor.rs` only gained `indent: String::new()` in test helpers; no execution-output behavior changed.)
- [x] Avoid applying indentation to frontmatter shell expansion, command discovery keys, command approval prompts, or error diagnostics. (Frontmatter and discovery construction sites use `String::new()`; only body splice/render sites re-indent.)
- [x] Preserve column-1 behavior with explicit tests so existing documents rendered at root do not gain leading whitespace. (`root_level_shell_output_has_no_indent`, `root_level_block_output_has_no_indent`, and the new `root_level_*_is_a_sibling_block_in_commonmark` structural tests all assert no leading whitespace at column 1.)

## Post-Review Follow-up (`review-1.md`)

The original implementation deferred the two block-quote acceptance criteria as
"unreachable" and left the spec's trailing-blank note unverified. `review-1.md`
flagged both. Resolution:

- **Block-quote directives are now implemented** (supersedes the "unreachable"
  notes in Phases 3-5). A shared `parse_utils::directive_prefix_len` /
  `strip_blockquote_prefix` pair captures the leading run of indentation
  whitespace and `>` markers. `::shell` (`shell_expansion/parser.rs`),
  the block scanner (`block_pairs.rs`), the shell-block region parser
  (`shell_blocks/parser.rs`), and the body splitter (`shell_blocks/body.rs`) all
  use it, so `> > ::shell ...` and `> ::shell-block ...` are recognized, executed
  with markers stripped, and re-quoted on every output line. Covered by parser
  unit tests at each layer plus compose + CommonMark `<blockquote>`-nesting tests
  for both directive forms. The negative `blockquote_marked_shell_block_is_not_a_directive`
  test was replaced by positive equivalents.
- **Trailing-blank requirement was corrected in the spec, not the code.** A
  trailing-newline output keeps its bare final newline rather than materializing
  a whitespace-only `"    "` line: the two are CommonMark-equivalent at the end
  of a container and the shared `indent_text` helper is deliberately
  byte-preserving. `spec.md` requirement #3 and the trailing-newline note were
  rewritten to match; `indented_shell_trailing_newline_does_not_become_indent_only_line`
  and its `::shell-block` mirror lock the behavior.
