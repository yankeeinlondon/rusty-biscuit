---
status: draft-spec
reviewed: false
clarified: false
implemented: false
related:
    - 2026-09-24-ux-improvements
    - 2026-09-25-list-remove-performance
---

# More Info on `wt list`

The two comparison columns of `wt list` (`-> {default}` and `-> parent`) now
say only *whether* a branch merges. This feature makes them say *how far
apart* the branches are and drops the `already in` state.

The `-> {default}` header is unchanged: a violet `origin/main` badge when the
remote-tracking ref is the target, a blue `main` badge otherwise. An
abbreviated `o/main` was considered and rejected, because the column's cells
are always at least as wide as the full name, so shortening the header saves
no width.

The table this builds on is item 5 ("Table Design") of
`2026-09-24-ux-improvements`. Rendering lives in
`cli/src/commands/list_table.rs`; the merge states live in
`worktree::listing::MergeState` (`lib/src/listing.rs`).

## 1. Remove `already in`; it becomes `clean`

Today a cell reads `already in` when the branch has no commits the target
lacks (`ahead == 0`), and `clean` when it has commits that merge without
conflict. The distinction moves into the metrics (item 2): a branch with
nothing to merge shows `clean` and no `+N`.

- `MergeState` loses its `AlreadyIn` variant and keeps `Clean` and
  `Conflicts`. `ahead == 0` is `Clean`, since such a merge cannot conflict.
- `merge_markup` has two arms. The legend is unchanged, since it never
  mentioned `already in`.
- Update the lib tests that assert `AlreadyIn` (`listing.rs`:
  `state(0, 5, false)`, `state(0, 0, true)`, and
  `the_target_column_reports_already_in_clean_and_conflicts`) and the CLI
  snapshots in `cli/tests/list_table.rs`.

## 2. Ahead/behind metrics

Each `Comparison` already carries `ahead` and `behind`. Show them in the
`-> {default}` and `-> parent` cells:

- `+N` = commits on the row's branch that the target lacks (`ahead`).
- `-N` = commits on the target that the branch lacks (`behind`).
- A zero side is omitted: `+3`, `-5`, `+3 -5`.
- When both are zero (the two tips are the same commit) no metric is shown,
  only `clean`.
- Colors: `+N` green, `-N` red, both dim. The number is the only signal, so
  `NO_COLOR` output stays readable.
- `conflicts` rows also show metrics (`conflicts +2 -14`).
- Cells that show `—`, `?`, `parent deleted`, or nothing are unchanged.

### Width gate

Metrics are shown only when the terminal is at least **100 columns** wide. Below
that, the table renders as it does after item 1 (state word only), because
the Branch column's tree guides and PR badges already fill narrow terminals.
The width comes from the same `Terminal` the table renders against, so it
stays pure and testable. Snapshot both sides of the gate (99 and 100 columns).

### Placement: two prototypes to choose from

Option A was chosen (see "Open decisions"); option B is kept for the record.

**Option A: inline, after the state word**

```txt
╭───────────────────┬───────────────────────┬──────────────────────┬──────────────────╮
│ Worktree          │ Branch                │ -> [origin/main]     │ -> parent        │
├───────────────────┼───────────────────────┼──────────────────────┼──────────────────┤
│ ○ base repo       │ [main]                │ —                    │ —                │
│ ● fix-wt-ux       │ ├─ fix/wt-ux          │ clean +4 -2 [PR #99] │ —                │
│ ● feat-theme      │ ├─ feat/theme         │ clean +1             │ —                │
│ ○ feat-dark-fixes │ │  └─ feat/dark-fixes │ clean +3 -7          │ conflicts +2 -1  │
│ ○ old-cleanup     │ └─ chore/old-cleanup  │ clean -12            │ —                │
│ ● spike-parser    │ └┄ spike/parser       │ conflicts +6 -30     │ parent deleted   │
╰───────────────────┴───────────────────────┴──────────────────────┴──────────────────╯
```

This adds no column and only widens two cells. The numbers are ragged,
though, because the state words differ in length.

**Option B: a column of its own for each target**

```txt
╭───────────────────┬───────────────────────┬──────────────────┬────────┬────────────────┬───────╮
│ Worktree          │ Branch                │ -> [origin/main] │        │ -> parent      │       │
├───────────────────┼───────────────────────┼──────────────────┼────────┼────────────────┼───────┤
│ ○ base repo       │ [main]                │ —                │        │ —              │       │
│ ● fix-wt-ux       │ ├─ fix/wt-ux          │ clean [PR #99]   │  +4 -2 │ —              │       │
│ ● feat-theme      │ ├─ feat/theme         │ clean            │     +1 │ —              │       │
│ ○ feat-dark-fixes │ │  └─ feat/dark-fixes │ clean            │  +3 -7 │ conflicts      │ +2 -1 │
│ ○ old-cleanup     │ └─ chore/old-cleanup  │ clean            │    -12 │ —              │       │
│ ● spike-parser    │ └┄ spike/parser       │ conflicts        │ +6 -30 │ parent deleted │       │
╰───────────────────┴───────────────────────┴──────────────────┴────────┴────────────────┴───────╯
```

The numbers align (right-aligned, `+` and `-` in fixed sub-columns), but the
table gets two more columns and two more borders. Headers are blank because
the column to the left names the target. Below 100 columns the metric columns
are dropped entirely, not left empty.

The prototype script belongs in the session scratchpad, not the repository.

## 3. `-> parent` works like `-> {default}`, always local

The `-> parent` column gets the same treatment as `-> {default}`: `clean` /
`conflicts` (item 1), metrics and the width gate (item 2), and the same
placement option (item 2).

The parent comparison is always against the parent's **local** branch tip,
never its `origin/*` copy. This is already the case
(`fill_worktree_statuses` resolves the parent through `RefTips::local`), so
the header stays the plain text `-> parent` with no badge. Add a lib test
pinning that choice: a parent whose `origin/*` copy is ahead of its local tip
must be compared against the local tip.

## 4. Uncommitted-source dot turns red

The Worktree column's `●` for uncommitted source files is orange today
(`dirty_dot`, `DirtyStatus::DirtySource`), and it is too close to the yellow
`●` for uncommitted non-source files to tell apart at a glance. Make it
**red**, the same red the table uses for `conflicts`. The legend picks this up
automatically, since it renders the dots through `dirty_dot`.

The dirty-file tree (`cli/src/commands/dirty_tree.rs`, shown by `wt list`'s
verbose view and by the `wt remove` report) colors source-file names orange
for the same reason. It turns red too, so the dot and the file names keep
meaning the same thing.

Update the expected colors in `dirty_tree.rs` and `remove/report.rs` unit
tests, L1 `styles_follow_the_design` (`cli/tests/list_table.rs`), and the L2
tmux styling tests (`level2_list_verbose`, `level2_dirty_tree`).

## 5. Legend shows a connector that can occur on its own

The Branch legend line draws its first two samples with `├─`
(`legend_markup` in `list_table.rs`). In the table, `├─` only ever appears
with a sibling row below it continuing the vertical line. Standing alone in
the legend it reads as a `T` shape the tree never draws. Use `└─`, the
last-child connector, for both samples. It is complete on its own and
matches the `└┄` already used for "parent deleted":

```txt
 Branch     └─ merges cleanly into parent    └─ conflicts with parent    └┄ parent deleted
```

The colors stay as they are (gray, red, dim). Update the legend assertion in
`cli/tests/list_table.rs` and any L2 assertion that reads the legend line.

## Out of scope

- The Worktree legend still uses "clean" to mean "no uncommitted files".
  After item 1 the word means two things in one screen. Revisit if it
  confuses people; it is not changed here.
- Any comparison of a non-default branch with its own `origin/*` copy
  (excluded on purpose by `2026-09-24-ux-improvements`).

## Acceptance

1. No `wt list` output contains `already in`; `MergeState` has no `AlreadyIn`.
2. At 100 or more columns, every compared cell shows `+ahead` and `-behind`,
   omitting zero sides, and shows no metric when the tips are equal.
3. At 99 columns or fewer, no metrics appear, and with option B no metric
   columns appear either.
4. `-> parent` cells follow 1–3, measured against the parent's local tip.
5. L1 snapshots in `cli/tests/list_table.rs` cover each case, and the L2
   styling test (`level2_list_verbose::level2_list_styles_follow_the_design_in_tmux`)
   asserts the metric colors on cells.
6. A worktree with uncommitted source files shows a red `●` in the table and
   the legend, and its source-file names are red in the dirty-file tree; no
   `wt` output uses orange for them.
7. The Branch legend's connector samples are `└─`, `└─`, `└┄`; no legend
   sample is `├─`.
8. `.claude/skills/worktree/SKILL.md` and the worktree README are updated
   wherever they describe the columns.

## Open decisions

- [x] Placement: **option A (inline)**, chosen 2026-09-26 from the rendered
      prototypes. Option B was 98 columns on its own, which leaves almost no
      room above the 100-column gate, and its `-> parent` metric column is
      empty on most rows.
