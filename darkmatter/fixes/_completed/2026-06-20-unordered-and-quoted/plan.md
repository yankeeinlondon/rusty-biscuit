---
agent: codex/
phases: 4
created: 2026-07-01
start_phase: 1
yolo: false
packages:
    - darkmatter
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/cleanup.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/cleanup.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3: []
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_code:
    - darkmatter/lib/src/markdown/cleanup.rs
documentation:
    - darkmatter/fixes/2026-06-20-unordered-and-quoted/spec.md
---

# Execution Plan: Unordered List Markers Inside Blockquotes

Success means Markdown cleanup preserves authored unordered-list markers for top-level and blockquoted list items, keeps marker extraction/restoration aligned in mixed-marker documents, and does not rewrite list-like text inside fenced code blocks.

Assumption: the duplicate `agent` closure requirement is resolved by using the later value, `codex/`, in frontmatter.

## Phase 1: Orient and Lock the Expected Behavior

- [x] Confirm the implementation surface is still `darkmatter/lib/src/markdown/cleanup.rs`, especially `cleanup_content_internal`, `extract_list_markers`, `restore_list_markers`, `fix_blockquote_formatting`, and `fix_blockquote_line`.
- [x] Identify the nearest existing Level 1 cleanup tests in `darkmatter/lib/src/markdown/cleanup.rs` for unordered marker preservation, blockquote formatting, ordered-list behavior, and fenced-code protection.
- [x] Add focused failing tests for primary blockquote marker preservation: `> - item`, `> * item`, and `> + item` must clean to the same authored marker.
- [x] Add a failing mixed-marker alignment test where a blockquoted list uses one marker and a following top-level list uses another marker; assert each item keeps its own marker.
- [x] Add failing nested and indented blockquote tests for `> > - item`, `>> - item`, and `   > - item`, accepting existing cleanup normalization of the blockquote prefix while preserving `-`.
- [x] Add failing fenced-code protection tests for list-like `* ` lines inside a blockquoted fenced code block, covering backtick fences and tilde fences if cleanup output preserves both forms.
- [x] Validation checkpoint: run a narrow `cargo nextest run -p darkmatter <cleanup-test-filter>` or equivalent package-area command and confirm the new tests fail for the intended marker-restoration reason.

Parallelizable work: the primary marker, mixed-marker, nested/indented blockquote, and fenced-code tests can be authored independently once the local test style is confirmed.

## Phase 2: Implement Blockquote-Aware Marker Restoration

- [x] Add a cleanup-local helper near `restore_list_markers` that splits a rendered line into `(prefix, body)` after `fix_blockquote_formatting` has run.
- [x] Ensure the helper preserves the exact prefix bytes needed for reconstruction, including leading indentation, normalized `> ` segments, nested blockquote prefixes, compact-input output such as `> > `, and whitespace before body content.
- [x] Make the helper return the current top-level behavior for non-blockquote lines: leading whitespace as `prefix` and the first non-whitespace byte as the start of `body`.
- [x] Update `restore_list_markers` to detect normalized unordered bullets with `body.starts_with("* ")` instead of `line.trim_start().starts_with("* ")`.
- [x] Rebuild matching list lines as `prefix + original_marker + body[1..]`, and advance `marker_idx` for every matched top-level or blockquoted unordered list item.
- [x] Update fenced-code state tracking in `restore_list_markers` to use the post-prefix `body`, so blockquoted fences protect their contents.
- [x] Recognize both backtick and tilde fences when the post-prefix body starts with at least three repeated fence characters.
- [x] Keep `extract_list_markers` unchanged unless the Phase 1 tests prove its documented behavior is wrong.
- [x] Review and update the `restore_list_markers` doc comment so it reflects blockquote-aware matching and the restored 1:1 marker correspondence without overstating implementation details.
- [x] Validation checkpoint: rerun the narrow cleanup tests from Phase 1 and confirm they pass.

Parallelizable work: helper implementation and doc-comment revision are mostly independent after the helper contract is decided; the `restore_list_markers` rewrite depends on the helper.

## Phase 3: Regression Sweep and Behavior Boundaries

- [x] Run the existing cleanup/list-marker tests that cover top-level unordered marker restoration.
- [x] Run the existing cleanup tests that cover ordered lists to confirm ordered markers are untouched.
- [x] Run the existing cleanup tests that cover ordinary fenced code blocks outside blockquotes to confirm list-like code content is still protected.
- [x] Run the existing cleanup tests that cover `fix_blockquote_formatting` and `fix_blockquote_line` to confirm blockquote prefix normalization did not regress.
- [x] If any existing comments around touched code drift from behavior, update or remove them in the same change, treating code behavior as authoritative unless proven otherwise.
- [x] Validation checkpoint: run `just test` from `darkmatter/` for Level 1 library and CLI confidence, or document why a narrower nextest command was used instead.

Parallelizable work: the existing test groups can be run or inspected independently after Phase 2 passes the focused tests.

## Phase 4: End-to-End Acceptance and Handoff

- [x] If `claudine` is available locally, run the original dry-run command from the spec and verify the composed blockquote list emits `> - Fix:`, `> - Review File`, and `> - Review Iteration`.
- [x] If `claudine` is unavailable, record that the Level 1 cleanup fixture with the same blockquote list shape is the acceptance substitute.
- [x] Run `just lint` from `darkmatter/` if time and local dependencies permit; otherwise record the skipped lint reason.
- [x] Inspect `git diff -- darkmatter/lib/src/markdown/cleanup.rs darkmatter/fixes/2026-06-20-unordered-and-quoted/plan.md` to verify the implementation is surgical and the plan file contains only this execution plan.
- [x] Final validation checkpoint: summarize changed files, tests run, any skipped validation, and the acceptance result for the implementation team.

Parallelizable work: lint and the optional `claudine` dry-run can run independently after Phase 3 passes.
