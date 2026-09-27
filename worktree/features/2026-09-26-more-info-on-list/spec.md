---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
status: draft-spec
reviewed: true
reviewed_by: codex/gpt-6-sol
reviewed_on: 2026-09-26
review_iterations: 0
clarified: false
implemented: false
human_review: false
message_to_agent: |-
    Phase 1 (spikes) overturned no ruling. S1: `Terminal::default().width()` reports the
    real pane width when the shell wrapper captures stdout (terminal_size 0.4.4 falls back
    stdout -> stderr -> stdin on Unix and Windows; verified at 99/100/120 columns in tmux),
    so no `list.rs` change is needed. S2: for L1 color assertions use the substrings
    `\u{1b}[2m\u{1b}[32m+N` (dim green) and `\u{1b}[2m\u{1b}[31m-N` (dim red); `<dim>`
    emits `\u{1b}[2m` twice, so do not assert the whole prefix. ColorDepth::None renders
    the plain `+N -N`. The L2 tmux pane is 120x40 by default; Phase 4 adds a `GREEN`
    constant (Color::Indexed(2)). Details are in implementation-log.md.
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
`2026-09-24-ux-improvements`. The `worktree-cli` package renders it in
[list_table.rs](../../cli/src/commands/list_table.rs). The `worktree` library's
[MergeState](../../lib/src/listing.rs) describes whether a comparison merges
cleanly, and its [Comparison](../../lib/src/listing.rs) already holds the
commit counts this feature displays.

**Change to the earlier design:** That design deliberately omitted per-branch
counts and specified no width rule for this table. This feature reverses those
two choices for the comparison cells only. It keeps the selected default target,
the local fork-parent target, the cache keyed by both commit IDs, the table's four
columns, and PR badge placement. Showing existing comparison counts needs no
new Git calls, cache format, or network request.

## 1. Remove `already in`; it becomes `clean`

Today a cell reads `already in` when the branch has no commits the target
lacks (`ahead == 0`), and `clean` when it has commits that merge without
conflict. The distinction moves into the metrics (item 2): a branch with
no commits to merge shows `clean`, with `-N` if the target is ahead and no
metric if the tips match. On a narrow terminal the count is intentionally
hidden, so `clean` means only that the merge would not conflict; it does not
claim the branch has commits to contribute.

- `MergeState` loses its `AlreadyIn` variant and keeps `Clean` and
  `Conflicts`. `ahead == 0` is `Clean`, since such a merge cannot conflict.
- `merge_markup` has two state arms. The Branch legend's wording still describes
  merge readiness; item 5 changes only its connector samples.
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
- Metrics follow the state word and precede any PR badge in the same cell:
  `clean +4 -2 [PR #99]`. PR badges retain their current target-based placement
  and link behavior.
- Cells that show `—`, `?`, `parent deleted`, or nothing remain unchanged and
  never acquire metrics. The default-branch and detached rows, and parent rows
  without a worktree, keep their current behavior.
- These are commit counts against the column's named target, not a count of
  uncommitted files or a measurement against the branch's own remote copy.
  They use the existing cached `Comparison`; no additional comparison or fetch
  runs to render them.

### Width gate

Metrics are shown only when `Terminal::width()` is at least **100 columns**.
At 99 columns or less, each comparison cell retains its state word and any PR
badge, without counts. This gate applies to both target columns together and
uses the same `Terminal` passed to the table renderer, so it also works when
output is captured or a test supplies a fixed width. The `-w` / `--width` flag
controls the optional git graph, not this gate. At wider widths the existing
`biscuit-terminal` Table still handles long branch names and multiple badges
with its normal width and wrapping rules; the 100-column gate does not promise
that arbitrary row content fits on one line. Test the state and badge at 99,
then the counts and badge order at 100, in both plain and colored output.

### Placement: inline

Inline placement was chosen (see "Open decisions"). The second prototype is
retained below to explain the choice, not as an implementation option.

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

The numbers align, but the table gets two more columns and two more borders.
Headers are blank because the column to the left names the target. This layout
is not part of the selected design.

## 3. `-> parent` works like `-> {default}`, always local

The `-> parent` column gets the same treatment as `-> {default}`: `clean` /
`conflicts` (item 1), metrics and the width gate (item 2), and inline
placement (item 2).

The parent comparison is always against the parent's **local** branch tip,
never its `origin/*` copy. This is already the case: the `worktree` library's
[fill_worktree_statuses](../../lib/src/worktree.rs) resolves the parent through
local branch refs. The header therefore stays plain text `-> parent`, with no
badge. Add a library test pinning that choice: give the parent different local
and remote-tracking tips,
then assert the child's `ahead` and `behind` against the local tip. Reuse the
existing comparison and cache path.

## 4. Uncommitted-source dot turns red

The Worktree column's `●` for uncommitted source files is orange today
(`dirty_dot`, `DirtyStatus::DirtySource`), and it is too close to the yellow
`●` for uncommitted non-source files to tell apart at a glance. Make it
**red**, the same red the table uses for `conflicts`. The legend picks this up
automatically, since it renders the dots through `dirty_dot`.

The dirty-file tree (`cli/src/commands/dirty_tree.rs`, shown by `wt list`'s
verbose view and by the `wt remove` report) colors source-file names orange
for the same reason. It turns red too, so the dot and the file names keep
meaning the same thing. Non-source files remain yellow and directory labels
remain dim. The red `conflicts` text and connectors keep their existing meaning;
position and shape distinguish them from dirty-file dots and names.

Update the expected colors in `dirty_tree.rs` and `remove/report.rs` unit
tests, L1 `styles_follow_the_design` (`cli/tests/list_table.rs`), and the L2
tmux styling tests (`level2_list_verbose`, `level2_dirty_tree`). Update comments
and test descriptions that still call source files orange.

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

- The Worktree legend still uses "clean" to mean "no uncommitted files";
  comparison cells use it to mean "merges without conflicts." The column
  headings and legend distinguish those meanings. Changing that vocabulary
  is outside this feature.
- Any comparison of a non-default branch with its own `origin/*` copy
  (excluded on purpose by `2026-09-24-ux-improvements`).

## Acceptance

1. No `wt list` output contains `already in`; `MergeState` has no `AlreadyIn`.
2. At 100 or more columns, every successfully compared cell shows its nonzero
   `+ahead` and `-behind` counts, and shows no metric when the tips are equal.
   Counts appear before a PR badge. The table retains four columns.
3. At 99 columns or fewer, no metrics appear; state words and PR badges remain.
   `--width` does not change this threshold.
4. `-> parent` cells follow 1–3, measured against the parent's local tip.
5. L1 snapshots in `cli/tests/list_table.rs` cover zero, one-sided, two-sided,
   conflicting, and unavailable comparisons at 99 and 100 columns, including
   PR badge placement and readable color-free output. The L2 styling test
   (`level2_list_verbose::level2_list_styles_follow_the_design_in_tmux`)
   asserts the dim green and dim red metric colors on rendered cells.
6. A worktree with uncommitted source files shows a red `●` in the table and
   the legend, and its source-file names are red in the dirty-file tree; no
   `wt` output uses orange for them.
7. The Branch legend's connector samples are `└─`, `└─`, `└┄`; no legend
   sample is `├─`.
8. `.claude/skills/worktree/SKILL.md`, `worktree/README.md`, and
   `worktree/docs/cli/list.md` are updated wherever they describe the columns,
   dirty-file colors, or the legend. The earlier specification remains a
   historical design record; this document states the intentional changes.

## Open decisions

- [x] Placement: **option A (inline)**, chosen 2026-09-26 from the rendered
      prototypes. Option B was 98 columns on its own, which leaves almost no
      room above the 100-column gate, and its `-> parent` metric column is
      empty on most rows.
