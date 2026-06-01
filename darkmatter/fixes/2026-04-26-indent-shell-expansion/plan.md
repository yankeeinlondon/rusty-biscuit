---
phases: 5
created: 2026-06-01
start_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - darkmatter/fixes/2026-04-26-indent-shell-expansion/phase-1-findings.md
skills_files_updated_during_phase_1: []
packages:
  - darkmatter
---

# Execution Plan: `::shell` and `::shell-block` Indentation Preservation

## Assumptions

- The code behavior is the source of truth where comments or the spec's likely paths have drifted.
- `::shell-block` currently lives under `darkmatter/lib/src/markdown/compose/shell_blocks/`, not at `darkmatter/lib/src/markdown/compose/shell_blocks.rs`.
- This fix uses the directive or opener line's exact leading whitespace as the indent prefix. It does not infer lazy continuation indentation for column-1 directives because this spec explicitly preserves current column-1 behavior.

## Phase 1: Confirm Current Failure Surface

- [ ] Reproduce the bug with a focused `::shell` fixture composed through the real compose pipeline, with a 4-space-indented directive under a list item and command output containing at least two non-empty lines plus one blank separator line.
- [ ] Reproduce the bug with a focused `::shell-block` fixture using the same nested list shape and multi-line command output.
- [ ] Confirm the root-level `::shell` and `::shell-block` fixtures still emit column-1 output before the change, establishing the no-indent baseline.
- [ ] Inspect existing tests in `darkmatter/lib/src/markdown/compose/shell_expansion/` and `darkmatter/lib/src/markdown/compose/shell_blocks/` for assertions that depend on column-1 replacement behavior.
- [ ] Validation checkpoint: document which existing tests are expected to change, and which root-level tests must remain byte-for-byte unchanged.

## Phase 2: Add Shared Indentation Utility

- [ ] Move or copy the existing `toc_linking::render::indent_text` behavior into a neutral compose utility module, such as `darkmatter/lib/src/markdown/compose/indent.rs` or `parse_utils.rs`, so shell expansion and TOC linking can share one implementation.
- [ ] Keep the helper byte-preserving: prefix every line with the provided indent, preserve tabs and spaces exactly, preserve trailing whitespace and final newlines, and return unchanged text when either the indent or text is empty.
- [ ] Add unit tests for the helper covering multi-line text, blank interior lines, trailing newline behavior, tab indentation, empty text, and empty indent.
- [ ] Update `toc_linking` to call the shared helper without changing TOC behavior.
- [ ] Validation checkpoint: run the focused TOC-linking tests that cover indentation and cache reuse to prove the extraction did not regress the completed TOC fix.

## Phase 3: Capture and Apply `::shell` Directive Indentation

- [ ] Add an `indent: String` field to `ShellDirective` in `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`.
- [ ] Update every `ShellDirective` construction site to populate `indent`; use `String::new()` for frontmatter or synthetic directives where indentation is out of scope.
- [ ] In `shell_expansion/parser.rs`, compute the indent from the source line by taking the exact prefix before the first non-whitespace byte on the line containing `::shell`.
- [ ] Preserve the parser's current directive detection semantics while adding indentation capture; do not normalize tabs, spaces, or mixed whitespace.
- [ ] In `Markdown::run_shell_expansion_stage` in `darkmatter/lib/src/markdown/compose/mod.rs`, apply the shared indentation helper to `execution.combined_output()` before pushing the replacement into `apply_replacements_in_reverse`.
- [ ] Keep `apply_replacements_in_reverse` span-only and byte-range focused; indentation should be part of the replacement string before splicing.
- [ ] Add or update tests for `::shell` covering 4-space list indentation, block-quote marker indentation such as `> > ::shell ...`, tab indentation, blank output lines, merged stderr output when applicable, and root-level no-indent behavior.
- [ ] Validation checkpoint: compose the new `::shell` fixtures and assert every emitted line, including blank separator lines, starts with the captured indent when the directive is indented.

## Phase 4: Capture and Apply `::shell-block` Opener Indentation

- [ ] Extend `block_pairs::BlockPair` or `shell_blocks::types::ShellBlockRegion` to carry the exact leading whitespace from the `::shell-block` opener line.
- [ ] Populate that indent from `BlockPair::opening_text` or during shell-block parsing, preserving tabs and spaces byte-for-byte.
- [ ] Apply the shared indentation helper to `render::render_block_output(&results)` in `shell_blocks::run_shell_blocks_stage` before adding the replacement for the full block span.
- [ ] Decide and encode the empty-block behavior explicitly: an empty rendered block remains empty and should not become an indentation-only string.
- [ ] Add or update tests for `::shell-block` mirroring the `::shell` cases: 4-space list indentation, block-quote marker indentation, root-level no-indent behavior, blank separator lines, and tabs.
- [ ] Validation checkpoint: assert shell-block output lines are children of the parent Markdown container and that root-level shell-block output remains unchanged.

## Phase 5: Structural Validation and Final Review

- [ ] Add CommonMark structural assertions for representative `::shell` and `::shell-block` fixtures, using the existing Markdown parser test style in the codebase, to verify generated lines remain nested under their parent list or block quote rather than becoming siblings.
- [ ] Run the narrow package tests for the touched modules, starting with `cargo test -p darkmatter shell_expansion --color=never` and `cargo test -p darkmatter shell_blocks --color=never`; adjust the exact filters to match actual test names.
- [ ] Run the broader darkmatter compose test surface with `cargo test -p darkmatter compose --color=never` if the focused tests pass.
- [ ] Review nearby rustdoc and inline comments for drift caused by the behavior change, especially comments describing shell replacement, shell-block rendering, and `BlockPair` fields.
- [ ] Confirm no frontmatter shell expansion tests or code paths changed behavior; frontmatter `$(...)` values remain scalar expansion only.
- [ ] Final validation checkpoint: all acceptance criteria in `spec.md` are either directly covered by named tests or explicitly mapped to existing tests that still pass.

## Parallelizable Work

- [ ] Phase 1 fixture discovery for `::shell` and `::shell-block` can run in parallel after the relevant test helpers are identified.
- [ ] Phase 2 shared-helper tests can be implemented in parallel with Phase 3 parser field plumbing once the helper API is agreed.
- [ ] Phase 3 `::shell` tests and Phase 4 `::shell-block` tests can be written in parallel after the shared helper exists.
- [ ] Phase 5 comment review can run in parallel with the final test pass, provided any comment edits are rechecked after code changes settle.

## Risk Controls

- [ ] Avoid adding indentation to cached or source-derived command outputs before caller-local context is known; indentation belongs at the replacement/render boundary.
- [ ] Avoid changing shell execution output semantics in `executor.rs`; the fix should only affect how captured output is spliced into Markdown body content.
- [ ] Avoid applying indentation to frontmatter shell expansion, command discovery keys, command approval prompts, or error diagnostics.
- [ ] Preserve column-1 behavior with explicit tests so existing documents rendered at root do not gain leading whitespace.
