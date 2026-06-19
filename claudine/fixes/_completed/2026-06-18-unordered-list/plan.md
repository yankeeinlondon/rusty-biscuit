---
agent: open_code/zai-coding-plan/glm-5.2
phases: 4
created: 2026-06-18
start_phase: 1
yolo: "true"
spec: claudine/fixes/2026-06-18-unordered-list/spec.md
status: phase 4 complete
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/cleanup.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/cleanup.rs
docs_updated_during_phase_2:
  - claudine/fixes/2026-06-18-unordered-list/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/composition/prepare.rs
docs_updated_during_phase_3:
  - claudine/fixes/2026-06-18-unordered-list/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4:
  - claudine/fixes/2026-06-18-unordered-list/plan.md
  - claudine/fixes/2026-06-18-unordered-list/spec.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_code:
  - darkmatter/lib/src/markdown/cleanup.rs
  - claudine/lib/src/composition/prepare.rs
documentation:
  - claudine/fixes/2026-06-18-unordered-list/plan.md
  - claudine/fixes/2026-06-18-unordered-list/spec.md
packages:
  - darkmatter
  - claudine
---

# Plan — Tight nested lists must stay tight through cleanup

Execution plan for the bug described in `spec.md`: Darkmatter's
`normalize_list_spacing` Phase 2 (`Normal` mode) inserts a spurious blank line
between a parent list item and the first child of a **tight** sub-list, which
downstream renderers then mis-parse as an indented code block.

The fix is a single one-line predicate change plus accompanying doc/test work.
All source changes are confined to two files:

- `darkmatter/lib/src/markdown/cleanup.rs` (fix + unit tests)
- `claudine/lib/src/composition/prepare.rs` (end-to-end regression test only)

## Guardrails (apply to every task)

- **Do not** change pipeline ordering: `normalize_list_spacing` must continue to
  run against the 2-space `pulldown-cmark-to-cmark` output, **before**
  `fix_list_indentation` rescales to the target width.
- **Do not** touch `fix_list_indentation`, the `Compact` arm, or the `Loose`
  arm. Only the `Normal` arm of Phase 2 changes.
- **Do not** edit claudine production source. Claudine gets a **test only**.
- **Do not** run `cargo fmt` in write mode (repo convention — `main` is the
  formatting authority). Match surrounding style by hand.
- The `indent` values the predicate sees are unitless level offsets (0, 2, 4, …);
  `indent < prev` is correct regardless of the later rescale width.
- All commands run from the relevant package area. Repo convention is nextest
  via `just`; the spec's `cargo test …` invocations are equivalent.

## Dependency graph

```
Phase 1 (core fix + unblock the one red test) ──┬─► Phase 2 (darkmatter regression tests)
                                                └─► Phase 3 (claudine e2e test)
Phase 2 ║ Phase 3   (parallelizable after Phase 1)
Phase 4 (full verification)  ◄── requires Phase 1 + 2 + 3
```

---

## Phase 1 — Core predicate fix (atomic; keeps the suite green)

Goal: change the `Normal`-mode blank-line predicate so descending into a
sub-list no longer inserts a blank line, and fix the one existing test that
currently encodes the bug. This phase is atomic — the predicate change and the
`normal_blank_lines_around_level_transition` rewrite **must land together** or
the suite goes red.

- [x] **1.1 Confirm baseline reproduction.** Create `/tmp/l.md` with
  `- Level 1\n    - Level 2\n        - Level 3\n` and run `md clean /tmp/l.md`.
  Record the **before** output: it currently emits a blank line between every
  level. (Requires the `md` CLI built/installed: `cargo build -p darkmatter-cli`
  if `command -v md` is empty.) This is the observable baseline the fix is
  judged against.

- [x] **1.2 Apply the one-line predicate fix.** In
  `darkmatter/lib/src/markdown/cleanup.rs`, inside `normalize_list_spacing`'s
  Phase 2 `match mode` arm, change the `ListSpacingMode::Normal` branch
  predicate from

  ```rust
  indent != prev || had_continuation
  ```

  to

  ```rust
  // Descents and same-level siblings stay tight; loose items and shallower
  // returns keep their separating blank.
  indent < prev || had_continuation
  ```

  No other state machine field (`prev_item_indent`, `in_list_run`,
  `had_continuation`, `prev_was_blank`) is touched. Source anchor:
  `cleanup.rs:1279`.

- [x] **1.3 Correct the `normalize_list_spacing` doc comment.** At
  `cleanup.rs:1213-1221`, the bullet currently reads "_Normal_: blank lines at
  indentation level transitions and after sub-lists return to prose". Rewrite
  it to describe the fixed behavior, e.g. "_Normal_: blank lines when returning
  from a sub-list (shallower transition) or for loose list items, and before
  prose that follows a list". This satisfies the acceptance criterion that the
  doc no longer says "level transitions".

- [x] **1.4 Rewrite `normal_blank_lines_around_level_transition`.** This test
  (at `cleanup.rs:2810`) currently asserts `lessons:\n\n    - @docs`, which is
  exactly the bug. Update it so the **descent** assertion becomes
  `lessons:\n    - @docs` (single newline, no blank). Keep the **shallower
  return** assertion `commits.md\n\n2.` — that behavior is intentionally
  preserved by this fix. If keeping both in one test muddies intent, split into
  `normal_descent_into_sublist_is_tight` and
  `normal_return_from_sublist_inserts_blank`.

**Phase 1 checkpoint:**
- `cargo nextest run -p darkmatter --lib markdown::cleanup` is fully green
  (the rewritten test passes; no other cleanup test regresses).
- Re-running `md clean /tmp/l.md` now emits **no** blank lines between the
  three levels. If a blank line remains, the fix is incomplete — do not
  proceed.

---

## Phase 2 — Strengthen & expand darkmatter cleanup regression tests

Goal: add the negative assertions that would have caught this regression and
the new structural guards from spec §Testing Requirements. **Parallelizable
with Phase 3** once Phase 1 is complete (independent files).

- [x] **2.1 Strengthen the nested-indent tests with negative assertions.** In
  `test_nested_list_preserves_4_space_indentation` and the
  `test_cleanup_with_indent_forces_*` variants, add (in addition to the
  existing positive indent assertion) the **absence** assertion that would have
  caught this bug:

  ```rust
  assert!(
      !cleaned.contains("\n\n    - Level 2"),
      "no blank line before a tight child, got:\n{}",
      cleaned
  );
  ```

  Apply the analogous `\n\n        - Level 3` negative assertion at the deeper
  level.

- [x] **2.2 Add `tight_nested_list_stays_tight_after_cleanup`.** Model it on
  the `## Closure` payload: a heading, two top-level `- ` siblings where only
  the second has a tight 4-space-indented sub-list of four children (one child
  containing inline code + bold), followed by a blank line and a `**bold:**`
  paragraph, then another top-level list. Assert the cleaned output contains
  `- Save the following…:\n    - based on` (parent directly followed by child,
  no blank line) **and** that the children remain at the configured indent
  width.

- [x] **2.3 Add `tight_siblings_stay_tight`.** Guard the `indent == prev` case:
  same-indent siblings without continuation content remain blank-free. This
  proves the `!=` → `<` change is not a behavior change for siblings. (Existing
  `tight_list_stays_tight_in_normal_mode` covers the ordered-list flavor; add
  an unordered + nested-sibling variant.)

- [x] **2.4 Add `closing_a_sublist_inserts_blank`.** Guard the `indent < prev`
  case: returning to a strictly shallower indent still inserts a blank line.
  Use the shape

  ```markdown
  - parent:
      - child

  - sibling
  ```

  so the fix is provably scoped to the descent direction only.

- [x] **2.5 Confirm loose-list guard.** Verify
  `normal_loose_list_preserves_blank_lines_between_items` still passes
  unchanged (it exercises the `had_continuation` path). If clearer coverage is
  wanted, add a focused `loose_list_keeps_blank_lines_in_normal_mode` guard so
  the fix cannot be silently over-applied later. Also confirm
  `loose_with_nested_list` and `compact_with_nested_list` remain green.

**Phase 2 checkpoint:**
- `cargo nextest run -p darkmatter --lib markdown::cleanup` green including
  all new tests.
- The new negative assertions (`!contains("\n\n    - ")`) are the ones that
  would have failed against the old predicate — sanity-check by mentally
  reverting 1.2 if unsure.

---

## Phase 3 — Claudine end-to-end regression test

Goal: tie the darkmatter fix to the user-facing incident so the specific
failure mode (`claudine compose … --dry-run` corrupting the `## Closure`
section) can never silently regress. Claudine gets a **test only** — no
production source edits. **Parallelizable with Phase 2** (different file).

- [x] **3.1 Add `direct_composition_preserves_tight_nested_list`.** In
  `claudine/lib/src/composition/prepare.rs` `#[cfg(test)] mod tests`, reuse the
  existing `make_source(&dir, frontmatter, content)` helper. Build a source
  document with frontmatter and a body containing a tight nested unordered list
  shaped like the `## Closure` section (parent item ending with `:`, two
  top-level siblings where only the second has indented children, followed by
  prose). Call `prepare_direct(&source, PrepareOptions::default()).unwrap()`
  and assert `prepared.prompt` contains the parent item immediately followed —
  single newline, no blank line — by its first indented child, e.g.
  `assert!(prepared.prompt.contains("properties on \"…\":\n    - based on"))`
  and `assert!(!prepared.prompt.contains("properties on \"…\":\n\n    - "))`.

**Phase 3 checkpoint:**
- `cargo nextest run -p claudine --lib composition::prepare` green.
- Temporarily reverting Phase 1.2 must make this test **fail** — that confirms
  it actually exercises the fixed code path and is not a tautology.

---

## Phase 4 — Full verification & acceptance

Goal: run the complete behavioral matrix from spec §Verification and walk the
spec §Acceptance Criteria checklist. Requires Phases 1–3 complete.

- [x] **4.1 Darkmatter cleanup suite.**
  `cargo nextest run -p darkmatter --lib markdown::cleanup` — all green.

- [x] **4.2 Claudine composition suite.**
  `cargo nextest run -p claudine --lib composition::prepare` — all green
  (or `just test-library` from the `claudine/` area).

- [x] **4.3 Behavioral — three-level tight list.**
  `printf -- '- Level 1\n    - Level 2\n        - Level 3\n' > /tmp/tight.md`
  then `md clean /tmp/tight.md` emits **no** blank lines between levels.

- [x] **4.4 Behavioral — `## Closure`-shaped payload.**
  `md clean` on the closure payload yields
  `…properties on "…":\n    - based on…` (single newline between parent and
  first child).

- [x] **4.5 End-to-end — incident prompt composes cleanly.**
  `claudine compose prompts/review-feature.md -y … --dry-run` piped through a
  `grep -A2 'Save the following'` shows the children on the immediately
  following lines, not a blank line. The `## Closure` section renders as a
  nested list (no code block) in a renderer that previously showed a code
  block. (If a live compose is blocked by environment/credentials in this
  non-interactive session, the Phase 3 unit test is the authoritative proxy —
  note any skip in the closeout.)

- [x] **4.6 Mode isolation.** Confirm `Compact` and `Loose` outputs are
  byte-identical to pre-fix (`compact_with_nested_list`,
  `loose_with_nested_list`, `loose_adds_blank_lines_between_all_items` all
  green) — the fix touched only `Normal`.

- [x] **4.7 Walk spec §Acceptance Criteria.** Tick every box in the spec's
  acceptance-criteria checklist against the work above. Any unticked item is a
  blocker for closeout.

**Phase 4 checkpoint (done = shippable):**
- All four behavioral checks (4.3–4.6) pass.
- The full acceptance-criteria list in `spec.md` is satisfied, in particular:
  - `Normal`-mode predicate is `had_continuation || indent < prev`.
  - Parent→child descent never inserts a blank.
  - `normal_blank_lines_around_level_transition` no longer expects the buggy
    blank.
  - Strengthened tests use negative (absence) assertions, not just indent width.
  - `direct_composition_preserves_tight_nested_list` passes.
  - `normalize_list_spacing` doc comment describes fixed behavior.

---

## Out of scope (do not address in this plan)

- Claudine production source changes (`prepare_direct`/`prepare_inline` already
  call `Markdown::compose_with(…)`; the darkmatter fix propagates for free).
- `Compact` / `Loose` modes.
- `pulldown-cmark-to-cmark`'s 2-space default or `fix_list_indentation`'s
  rescale. Both are correct as-is.
- The unrelated `ctx.*` false-positive interpolation warnings and the
  `{{review-file}}` subtraction parse — these are a **separate bug** warranting
  their own spec (see spec.md §Related Finding).

## Key source anchors

- `darkmatter/lib/src/markdown/cleanup.rs:1279` — the predicate to change.
- `darkmatter/lib/src/markdown/cleanup.rs:1213` — doc comment to correct.
- `darkmatter/lib/src/markdown/cleanup.rs:2810` —
  `normal_blank_lines_around_level_transition` (currently encodes the bug).
- `darkmatter/lib/src/markdown/cleanup.rs:2246` —
  `test_nested_list_preserves_4_space_indentation` (strengthen).
- `darkmatter/lib/src/markdown/cleanup.rs:748` — `fix_list_indentation`
  (reference only; do not change).
- `claudine/lib/src/composition/prepare.rs:511` — `make_source` test helper.
- `claudine/lib/src/composition/prepare.rs:109` — `prepare_direct` (call site
  for the new e2e test).
