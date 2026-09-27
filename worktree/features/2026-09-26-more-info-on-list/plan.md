---
total_phases: 5
created: 2026-09-26
phase: 1
agent: claude/opus
yolo: true
packages: []
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
---

# Plan: More Info on `wt list`

Implements `2026-09-26-more-info-on-list`.

## Summary and Success Criteria

### The work

The feature changes only what `wt list` renders. It adds no Git calls, no
cache format, and no network request. There are five changes:

1. **Library (`worktree`)**: `MergeState` loses `AlreadyIn`. `ahead == 0` becomes
   `Clean`, and `Comparison::merge_state` has two outcomes. Tests that assert
   `AlreadyIn` are updated. A new repo test pins the rule that the parent comparison uses the
   parent's **local** tip, never its `origin/*` copy. The code already does this
   (`fill_worktree_statuses` resolves the parent through `refs.local(parent)`); only
   the test is new.
2. **CLI table (`worktree-cli`, `cli/src/commands/list_table.rs`)**: when
   `terminal.width() >= 100`, the `-> {default}` and `-> parent` comparison cells
   show inline `+ahead` (dim green) and `-behind` (dim red) after the state word
   and before any PR badge. A zero side is omitted, and when both sides are zero no metric appears.
   `merge_markup` has two arms. The `-w/--width` flag stays graph-only.
3. **Dirty source color**: `dirty_dot(DirtySource)` and the dirty-file tree's
   source-file names change from orange to red. They use the same `<red>` as `conflicts`.
4. **Legend**: the Branch line's first two samples change from `├─` to `└─`.
5. **Tests and docs**: L1 snapshots at 99 and 100 columns (plain and colored).
   The L2 tmux styling tests cover the metric colors and the red dot and names. README, `docs/cli/list.md`,
   and the worktree skill are updated.

### Touch points

| File | Change |
| --- | --- |
| `worktree/lib/src/listing.rs` | `MergeState`, `merge_state`, docs, `merge_state_reads_ahead_first`, `the_target_column_reports_already_in_clean_and_conflicts`, new parent-local test |
| `worktree/cli/src/commands/list_table.rs` | `merge_markup` (metrics and gate), `target()`/`parent()` call sites, `RowCells` gets the gate, `legend_markup`, `dirty_dot` |
| `worktree/cli/src/commands/dirty_tree.rs` | source-name color and its unit test |
| `worktree/cli/src/commands/remove/report.rs` | unit-test color expectation (line ~405) |
| `worktree/cli/tests/list_table.rs` | `ALREADY_IN` const, `every_cell_kind_renders_as_ruled`, `styles_follow_the_design`, `the_legend_explains_both_columns`, snapshot(s), new 99/100 tests |
| `worktree/cli/tests/level2_list_verbose.rs` | red dot, metric colors, legend line |
| `worktree/cli/tests/level2_dirty_tree.rs` | red source names, doc comments |
| `worktree/README.md`, `worktree/docs/cli/list.md`, `.claude/skills/worktree/SKILL.md` | column vocabulary, metrics, gate, red, legend |

### Definition of done

All eight **Acceptance** items in the spec hold, as shown by:

- `grep -rn "AlreadyIn\|already in" worktree/lib/src worktree/cli` returns nothing,
  apart from unrelated prose such as `wt go`'s "already in base".
- `grep -rn "orange" worktree/cli/src worktree/cli/tests` returns nothing, except
  `styled_capture_parse.rs`, which tests tmux's palette mapping rather than `wt`.
- `just test` and `just lint` in `worktree/` are green.
- `just test-l2` is green for `level2_list_verbose` and `level2_dirty_tree` in
  tmux (`BISCUIT_TEST_REQUIRED_BACKENDS=tmux`, so a missing backend fails instead of skipping).
- Spike S1 shows that under the real shell wrapper, the metrics appear at ≥ 100
  columns and are hidden at ≤ 99.
- The spec's `status` is `implemented`. The spec is **not** moved to `_completed`,
  because the author does that after review.

## Phase 1 — Rulings and Spikes

### Necessary Rules

These rulings settle points the spec leaves open. Implementers follow them
unless a spike overturns one; any change is recorded in the implementation log.

- **R1: gate constant.** `const METRICS_MIN_WIDTH: u32 = 100;` in
  `list_table.rs`. Metrics show when `terminal.width() >= METRICS_MIN_WIDTH`.
  The value is computed once per render (in `table()`) and passed into `RowCells`.
  Both target columns are gated together. No CLI flag or env var controls it.
- **R2: width source.** The gate reads the `Terminal` passed to
  `list_table::render`/`table` and does not query the width again. If S1 shows
  that `Terminal::default()` misreports width while the wrapper captures stdout, the fix goes
  in `list.rs`, which builds its terminal the way `image_terminal` does for stderr. It does
  not go in the gate. A `biscuit-terminal` change is out of scope unless S1 proves it is needed.
- **R3: metric markup.** `+N` is `<dim><green>+N</green></dim>` and `-N` is
  `<dim><red>-N</red></dim>`. Parts are joined by one space:
  `state [+A] [-B] [badges]`. The `-` is ASCII hyphen-minus, not U+2212, so
  `NO_COLOR` output and grep work. The state word keeps its current style
  (`clean` dim italic, `conflicts` red).
- **R4: `merge_markup` signature.** `fn merge_markup(comparison: Comparison,
  show_metrics: bool) -> String`. The `—`, `?`, `parent deleted`, and empty
  cells never call it, so they cannot get metrics, as the spec requires.
- **R5: `merge_state` rule.** It returns `Conflicts` only when `ahead > 0 && !is_clean`,
  and `Clean` otherwise. `ahead == 0` stays checked first, as today, so an
  `is_clean == false` answer with nothing to merge is still `Clean`. The `Clean` doc changes to
  "Merging the branch into the target would not conflict (including when it has
  nothing to merge)". The `Comparison::is_clean` doc stays.
- **R6: connector colors.** `connector_markup` already maps everything except
  `Conflicts` to gray, so a former `AlreadyIn` row keeps its gray connector. No
  change is needed apart from the legend glyphs.
- **R7: the red for dirty source.** This is the Prose `<red>` tag, basic ANSI 31, the same
  as `conflicts`. L1 asserts `\u{1b}[31m●`, and L2 asserts the basic-red fg
  predicate that the `conflicts` checks already use.
- **R8: test renames.** `the_target_column_reports_already_in_clean_and_conflicts`
  becomes `the_target_column_reports_clean_and_conflicts`. `chore/merged` is asserted `Clean`
  with `ahead == 0`. The CLI test const `ALREADY_IN` becomes `NOTHING_TO_MERGE`
  (`ahead: 0, behind: 4`), and an `EQUAL_TIPS` (`0, 0`) const is added.
- **R9: snapshot widths.** The existing fixtures use width 120, which will now show
  metrics, so the snapshot `the_spec_example_renders_as_ruled` is re-accepted.
  New snapshot tests at exactly 99 and 100 are added. Snapshots stay in the integration
  test `cli/tests/list_table.rs`, never in a unit test (see the worktree skill: every
  shared module compiles twice).
- **R10: parent-local test shape.** This is a `repo_tests` test in `listing.rs` using
  `TestRepo`. Create the parent `feat/base` from `main`, push it to `origin`, then
  set the local and remote-tracking tips apart. Add a local commit to `feat/base`, and have
  `pusher` push a different commit to `origin/feat/base` followed by `git fetch`. Create
  the child from `feat/base` with a fork-origin record. Assert that the child's
  `ParentComparison::Compared(Some(c))` has the `ahead`/`behind` counts against the
  **local** tip, and that they differ from what the remote tip would give. Mark it
  `#[serial_test::serial]`, as its sibling tests are.
- **R11: no vocabulary change beyond the spec.** The Worktree legend's "clean"
  stays. The README line about the default-branch target stays except for the state list.
  `docs/git-graph.md`'s "already in the default branch" describes the graph's tag
  rule rather than the table state, so it stays.
- **R12: coordination with `2026-09-26-stale-remote-caption`.** That fix also
  edits `list_table.rs` (the caption) and `docs/cli/list.md`. This feature does not
  touch `caption_markup`. If both are in flight, whichever lands second rebases.
  Neither should reformat the other's region.

### Spikes

Wave 1 runs S1 and S2 in parallel; both are read-and-run investigations with no
production edits.

- [x] **S1: width under the wrapper**
    - Question: when `wt` runs through the POSIX shell wrapper, which captures stdout
      (`$(...)`) while the table goes to stderr, does
      `Terminal::default().width()` report the pane width, or the 80-column fallback?
    - Read `terminal_size` 0.4.4's `terminal_size()` in `~/.cargo/registry` for
      Unix and Windows. Record whether it falls back to stderr/stdin on each OS.
    - Confirm it empirically in a detached tmux pane at 120 columns. Run a scratch binary, or `wt`
      with a temporary debug print, as `x=$(cmd)` and print `Terminal::default().width()`
      to stderr. The pane must not take focus.
    - Outcome: if the width is correct on Unix and the Windows source shows a stderr
      fallback, R2 stands with no code change. Otherwise, add a Phase 3 task to
      build the list terminal from stderr (R2) and add an L2 case that runs through the wrapper.
- [x] **S2: dim+color nesting in Prose and in the L2 parser**
    - Render `<dim><green>+3</green></dim> <dim><red>-5</red></dim>` with
      `Prose` on a true-color terminal and on a `NO_COLOR`/`ColorDepth::None` terminal.
      Record the exact SGR bytes for L1 assertions (expected `\u{1b}[2m` plus
      `\u{1b}[32m` / `\u{1b}[31m`, in some order or merged), and confirm the plain
      text is exactly `+3 -5`.
    - Check that `cli/tests/styled_capture/` exposes `dim` and a basic green/red fg
      predicate for one cell at the same time (it already has `s.dim` and `fg_is`). If a green predicate is
      missing, add a Phase 4 subtask for it.
    - Check the tmux harness pane width used by `level2_list_verbose::list_until`
      (`TmuxHarness::spawn_shell`). If it is under 100 columns, the metric assertions
      need `harness.resize(≥100, rows)` or a sized spawn. Record which.

- [x] **Checkpoint 1**: S1 and S2 findings are recorded in
  `implementation-log.md` in this feature directory, and any rule they overturn is amended there.

## Phase 2 — Library State and Independent Color Changes

Wave 2 has two parallel tasks that touch disjoint files.

- [ ] **Remove `AlreadyIn`** (lib, plus one line in the CLI)
    - `lib/src/listing.rs`: delete the variant, apply the R5 rule and doc, and update
      `merge_state_reads_ahead_first` (`state(0,5,false)` and `state(0,0,true)` become `Clean`).
    - Rename and update the repo test per R8.
    - Delete the `MergeState::AlreadyIn` arm in `list_table.rs::merge_markup` so the
      workspace compiles. This is the only CLI edit in this task, and the metrics come in Phase 3.
    - Update `ALREADY_IN` usages in `cli/tests/list_table.rs` just enough to compile
      and pass: the line-284 assertion now expects `clean`. The full test rework is in Phase 3.
- [ ] **Parent-local test** (lib, same file, so the same agent runs it after the task above)
    - Add the R10 test and run it alone first
      (`cargo nextest run -p worktree -E 'test(parent)'`).
- [ ] **Dirty-tree red** (CLI, disjoint files)
    - `cli/src/commands/dirty_tree.rs`: `<orange>{name}</orange>` becomes `<red>{name}</red>`.
      Update its unit tests (lines ~131–132) and any doc comment that says orange.
    - `cli/src/commands/remove/report.rs` unit test (~405): `<orange>f09.rs</orange>` becomes
      `<red>…</red>`.
- [ ] **Checkpoint 2**: `just test` in `worktree/` is green. The first grep in the Definition
  of done is clean for `lib/`.

## Phase 3 — Table Rendering (metrics, gate, red dot, legend)

Wave 3 is one agent, because every change is in `list_table.rs` and its integration test.

- [ ] **Metrics and gate**
    - Add `METRICS_MIN_WIDTH` (R1). Compute `show_metrics` in `table()` from
      `terminal.width()` and pass it to `RowCells::new` alongside `osc_link_support`.
    - Implement `merge_markup(comparison, show_metrics)` per R3/R4. The metrics come
      after the state word, and `with_badges` then appends the PR badges, which gives the order.
    - Call it from both `target()` and `parent()`.
    - If S1 required it, change `list.rs` to build the list `Terminal` with a stderr-based width (R2).
- [ ] **Red dirty dot**
    - `dirty_dot(DirtySource)` becomes `<red>●</red>`. The legend picks this up automatically.
- [ ] **Legend connectors**
    - In `legend_markup`, the first two samples change from `├─` to `└─`. The colors are unchanged.
- [ ] **L1 tests** (`cli/tests/list_table.rs`)
    - Add `terminal_at(width, color: bool)` or `plain_at(width)`/`color_at(width)`
      helpers. Keep `plain_terminal`/`color_terminal` if they are still used.
    - Extend the example (or add a focused fixture) so the rows cover the following: zero (`EQUAL_TIPS`,
      showing `clean` only), ahead-only (`+N`), behind-only (`NOTHING_TO_MERGE`, showing `clean -4`),
      both sides, conflicting with metrics (`conflicts +1 -3`), unavailable (`?`),
      `parent deleted`, `—`, and a cell with a PR badge.
    - `insta` snapshots: plain at 99 and plain at 100. Re-accept
      `the_spec_example_renders_as_ruled` (R9) after reviewing the diff by eye.
    - Assertions at 99: no `+`/`-` metric tokens in any comparison cell, and state
      words and badges are still present. At 100: exact cell text such as
      `clean +2 -1 [PR]`, with the badge after the metrics. A test asserts that the
      table still has exactly four columns (count `│` separators on a row).
    - Colored 100-column assertions use the SGR bytes from S2 for dim green `+N`
      and dim red `-N`. The `NO_COLOR` (`ColorDepth::None`) rendering reads
      `+2 -1` with no escapes.
    - `styles_follow_the_design`: the source dot becomes `\u{1b}[31m●`. Update the
      "orange" comment.
    - `the_legend_explains_both_columns`: the Branch line becomes `└─ … └─ … └┄ …`.
    - `every_cell_kind_renders_as_ruled`: no `already in`. Assert it is absent from
      the full rendering (Acceptance 1).
- [ ] **Doc-comment pass** on the changed symbols (`merge_markup`, `dirty_dot`,
  `legend_markup`, `RowCells`), per CLAUDE.md "Authoring discipline".
- [ ] **Checkpoint 3**: `just test` and `just lint` in `worktree/` are green. Review the snapshots
  by eye against the spec's Option A prototype.

## Phase 4 — L2 Proof and Documentation

Wave 4 runs three parallel tasks on disjoint files.

- [ ] **L2 list styling** (`cli/tests/level2_list_verbose.rs`)
    - Make sure the pane is at least 100 columns wide (S2 finding). The fixture's
      `feature-test` branch needs a nonzero ahead and/or behind so a metric renders.
      Add a commit on main after the fork if needed.
    - In `level2_list_styles_follow_the_design_in_tmux`, assert the `+N` span is
      dim and green, the `-N` span is dim and red, and the metrics come before the PR badge.
      The source-dot assertion (line ~491) becomes red, both in the row and in the
      legend dots (~527–530). Update the Branch legend row reader if it matches `├─`.
    - Update the module and doc comments that say orange (~239, ~257, ~487).
    - Add `const GREEN: Color = Color::Indexed(2);` beside `RED` (S2: no green constant
      exists; the parser already exposes `dim` and `fg_is`). Assert `harness.pane_cols() >= 100`
      before the metric checks, since a shared broker session may not be 120 wide.
    - Test windows or panes must never take focus. tmux is detached by construction.
- [ ] **L2 dirty tree** (`cli/tests/level2_dirty_tree.rs`)
    - `lib.rs` is asserted red instead of `ORANGE`. Delete the `ORANGE` constant if it is now unused,
      and update the comments on lines ~5, ~17, and ~53.
- [ ] **Docs and skill**
    - `worktree/docs/cli/list.md`: redraw the example table with metrics and no
      `already in`. Update the legend sample to `└─`, change the `●` source-dot bullet to red, and in the target-column
      bullets remove `already in`, define `clean` as "would merge without conflicts",
      and describe `+N`/`-N` and the 100-column gate (independent of `--width`).
      Note that `-> parent` measures against the parent's local tip.
    - `worktree/README.md` line ~32: the state list becomes `clean` or `conflicts`, plus
      metrics at ≥ 100 columns.
    - `.claude/skills/worktree/SKILL.md` `wt list` section: add one line on the metrics
      gate (`METRICS_MIN_WIDTH`, the width taken from the render `Terminal`, not `--width`), the
      red dirty-source color shared with `conflicts`, and that `MergeState` is two-state.
      Update the L2 styling bullet to mention the metric colors.
    - If S1 found a non-obvious OS fact about width detection, add it to the `os` skill (CLAUDE.md rule).
- [ ] **Checkpoint 4**: run `BISCUIT_TEST_REQUIRED_BACKENDS=tmux cargo nextest run -p worktree-cli
  --features terminal-tests -E 'binary(level2_list_verbose) | binary(level2_dirty_tree)'`
  and confirm it is green. Run it as a single command, since a quoted filterset breaks the `just test-l2`
  recipe. `just test-l2` in `worktree/` is green overall.

## Phase 5 — Validation and Handoff

Wave 5 is sequential.

- [ ] **Acceptance audit**: walk spec Acceptance 1–8 and map each one to a test
  name or grep result in `implementation-log.md`.
- [ ] **Greps**: run the two greps from the Definition of done, and grep for `├─` in `legend_markup`.
- [ ] **Full local gates**: `just test`, `just test-l2`, and `just lint` in `worktree/`.
  Run `just test worktree-cli` from the repo root if a dependent needs it.
- [ ] **Cross-OS note**: the change is platform-neutral rendering. The CI plan
  (Linux and macOS on the PR) covers it. There is no Windows-specific code, and if S1 needed an R2
  change in `list.rs`, `./scripts/cross-check.sh --os windows worktree-cli` must pass.
- [ ] **Spec status**: set the spec's frontmatter `status: implemented`,
  `implemented: true`, and `implemented_by: claude/opus`. Do not move the directory to
  `_completed` and do not commit, because the author does both.
- [ ] **Checkpoint 5**: the implementation is complete and ready for review.
