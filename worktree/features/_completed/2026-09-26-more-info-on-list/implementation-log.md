---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-wt-ux/worktree/features/2026-09-26-more-info-on-list/spec.md"
plan: "worktree/features/2026-09-26-more-info-on-list/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages:
    - worktree
    - worktree-cli
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - worktree/lib/src/listing.rs
    - worktree/cli/src/commands/list_table.rs
    - worktree/cli/src/commands/dirty_tree.rs
    - worktree/cli/src/commands/remove/report.rs
    - worktree/cli/tests/list_table.rs
    - worktree/cli/tests/snapshots/list_table__the_spec_example_renders_as_ruled.snap
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - worktree/cli/src/commands/list_table.rs
    - worktree/cli/tests/list_table.rs
    - worktree/cli/tests/list_output.rs
    - worktree/cli/tests/snapshots/list_table__the_spec_example_renders_as_ruled.snap
    - worktree/cli/tests/snapshots/list_table__the_table_at_99_columns_shows_no_counts.snap
    - worktree/cli/tests/snapshots/list_table__the_table_at_100_columns_shows_counts.snap
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
    - worktree/cli/tests/level2_list_verbose.rs
    - worktree/cli/tests/level2_dirty_tree.rs
docs_updated_during_phase_4:
    - worktree/docs/cli/list.md
    - worktree/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/worktree/SKILL.md
source_files_during_phase_5:
    - worktree/cli/tests/list_output.rs
    - worktree/cli/tests/level2_list_verbose.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/worktree/SKILL.md
source_code:
    - worktree/lib/src/listing.rs
    - worktree/cli/src/commands/list_table.rs
    - worktree/cli/src/commands/dirty_tree.rs
    - worktree/cli/src/commands/remove/report.rs
    - worktree/cli/tests/list_table.rs
    - worktree/cli/tests/list_output.rs
    - worktree/cli/tests/level2_list_verbose.rs
    - worktree/cli/tests/level2_dirty_tree.rs
    - worktree/cli/tests/snapshots/list_table__the_spec_example_renders_as_ruled.snap
    - worktree/cli/tests/snapshots/list_table__the_table_at_99_columns_shows_no_counts.snap
    - worktree/cli/tests/snapshots/list_table__the_table_at_100_columns_shows_counts.snap
documentation:
    - worktree/docs/cli/list.md
    - worktree/README.md
    - .claude/skills/worktree/SKILL.md
completed_phase: 5
implemented: true
implementation_1: "2026-09-26T18:02:41-07:00"
---

# Implementation Log for 2026-09-26-more-info-on-list (5 phases)

## Phase 1

Phase 1 is investigation only: two spikes, no production edits. Both spikes
confirm the rulings; **no rule is overturned** and no Phase 3 `list.rs` task is
needed. One small Phase 4 subtask was added (an L2 `GREEN` constant).

### S1: width under the shell wrapper

**Source (`terminal_size` 0.4.4, the version in `Cargo.lock`).**

- Unix (`src/unix.rs`): `terminal_size()` tries stdout, then stderr, then
  stdin; each attempt is `isatty` + `tcgetwinsize`.
- Windows (`src/windows.rs`): the same order over `GetStdHandle(STD_OUTPUT_HANDLE)`,
  `STD_ERROR_HANDLE`, `STD_INPUT_HANDLE`, each through
  `GetConsoleScreenBufferInfo`; width is the visible window
  (`srWindow.Right - srWindow.Left + 1`), not the buffer width.
- `biscuit_terminal::Terminal::width()` is `fixed_width.unwrap_or_else(terminal_width)`,
  and `Terminal::default()` leaves `fixed_width` as `None`, so the width
  `wt list` sees is exactly `terminal_size()` (80 only when all three handles
  fail).

**Wrappers.** Every generated wrapper captures stdout only and leaves stderr on
the terminal: bash/zsh `out="$(WT_SHELL_WRAPPER=1 command wt "$@")"`, fish
`set -l out (… command wt $argv)`, PowerShell `$out = & $wtExe @args` (native
stderr is not redirected without `2>&1`).

**Empirical (macOS, detached tmux, no focus).** A scratch binary pinned to
`terminal_size = "=0.4.4"` (in `/tmp`, outside the repo) printed the width to
stderr and a `cd:` line to stdout. Results, per pane width:

| Pane | `x=$(probe)` | `probe </dev/null >/dev/null` | `probe 2>/dev/null \| cat` |
| --- | --- | --- | --- |
| 120 | 120 | 120 | 120 (via stdin) |
| 100 | 100 | 100 | 100 |
| 99 | 99 | 99 | 99 |

The captured stdout was the `cd:` line, as the wrapper expects.

**Outcome.** R2 stands with no code change: `Terminal::default()` in
`list.rs::run` already reports the pane width under the wrapper on Unix, and
the Windows source has the same stderr fallback. No `list.rs` change, no
`biscuit-terminal` change, and no wrapper L2 case is required. Windows was not
exercised empirically (source reading met the plan's outcome criterion), so
Phase 5's cross-OS note keeps "no Windows-specific code".

### S2: dim + color nesting in Prose and the L2 parser

Rendered through `Prose::new(markup).render(&terminal)` via a temporary
integration test in `worktree/cli/tests/` (deleted after the run), for
`<i><dim>clean</dim></i> <dim><green>+3</green></dim> <dim><red>-5</red></dim>`:

- `ColorDepth::TrueColor`:
  `"\u{1b}[3m\u{1b}[2mclean\u{1b}[0m\u{1b}[0m \u{1b}[2m\u{1b}[2m\u{1b}[32m+3\u{1b}[0m\u{1b}[2m\u{1b}[0m \u{1b}[2m\u{1b}[2m\u{1b}[31m-5\u{1b}[0m\u{1b}[2m\u{1b}[0m"`
- `ColorDepth::None`: `"clean +3 -5"` exactly (no escapes).

For Phase 3 L1 assertions, the stable substrings are
**`\u{1b}[2m\u{1b}[32m+3`** (dim green) and **`\u{1b}[2m\u{1b}[31m-5`** (dim red).
`<dim>` emits `\u{1b}[2m` twice, so do not assert the full prefix; assert
these substrings. Basic green is SGR 32 and red is SGR 31 (the same red as
`conflicts`, R7).

**L2 parser (`cli/tests/styled_capture/mod.rs`).** `Style` has `dim` (SGR 2,
cleared by 22) and `fg_is(Color)`; SGR 30–37 map to `Color::Indexed(0..=7)`.
A cell's dim and fg are available together, so `|s| s.dim && s.fg_is(GREEN)`
works. `level2_list_verbose.rs` has `RED = Color::Indexed(1)` but no green
constant, so Phase 4 adds `const GREEN: Color = Color::Indexed(2);` (no parser
change). Added as a Phase 4 subtask.

**Pane width.** `level2_list_verbose::list_until` uses
`TmuxHarness::new()` + `spawn_shell()`, which spawns at **120×40**
(`biscuit-test-harness/src/tmux.rs`, `resize` docs). That is ≥ 100, so the
metric assertions need no resize. A shared broker session could differ, so
Phase 4 should assert `harness.pane_cols() >= 100` before relying on metrics
(or `resize(120, 40)`); `TmuxHarness::resize` exists if a 99-column L2 check
is wanted.

### Rulings

R1–R12 stand unchanged.

### Gates

- No source, doc, or skill files changed in this phase, so there is no
  requirement-to-test mapping yet; the tests for R5, R8, and R10 are in Phase 2,
  and R1, R3, R7, and R9 are in Phase 3/4.
- `just test` in `worktree/`: 457 passed, 18 skipped (the tier-gated L2+ tests), none failed.
- `just lint` in `worktree/`: clean.
- The worktree skill was left unchanged. Phase 4 adds the metrics-gate line, and it
  can also record the S1 fact (the width comes from the stderr fallback under the wrapper).

## Phase 2

### Changes

- **`MergeState` is two-state** (`lib/src/listing.rs`). The `AlreadyIn` variant is
  gone. `Comparison::merge_state` returns `Clean` when `ahead == 0 || is_clean`,
  and `Conflicts` otherwise (R5). A one-line WHY comment marks that nothing to
  merge cannot conflict. The `Clean` doc now reads "would not conflict
  (including when it has nothing to merge)".
- **CLI compile fix** (`cli/src/commands/list_table.rs`). Only the
  `MergeState::AlreadyIn` arm of `merge_markup` was deleted. The metrics and gate
  are Phase 3.
- **Dirty-tree red** (`cli/src/commands/dirty_tree.rs`). Source-file names are
  `<red>{name}</red>`. The module doc says "the `wt list` dirty-dot palette", which
  holds once Phase 3 turns the dot red, so it was left as-is. No doc said
  "orange".
- **`remove/report.rs`**: the unit test expects `<red>f09.rs</red>`.

### Requirement-to-test mapping

| Requirement | Test |
| --- | --- |
| R5: `ahead == 0` is `Clean` whatever `is_clean` says; `ahead > 0` follows `is_clean` | `listing::tests::merge_state_reads_ahead_first` (adds `state(0,0,false)` as the boundary case beside `(0,5,false)` and `(0,0,true)`) |
| R8: a fast-forward-merged branch reports `Clean` with `ahead == 0` on a real repo | `listing::repo_tests::the_target_column_reports_clean_and_conflicts` (renamed) |
| R10: `-> parent` uses the parent's local tip, never `origin/*` | `listing::repo_tests::the_parent_column_measures_against_the_parents_local_tip` (new) |
| The table no longer says `already in` (downstream CLI output) | `cli/tests/list_table.rs::every_cell_kind_renders_as_ruled` (`chore/old-cleanup` shows `clean` and not `already in`), plus the re-accepted `the_spec_example_renders_as_ruled` snapshot |
| Dirty source names are red | `dirty_tree::tests::nested_files_group_by_directory`, `remove::report::tests::up_to_ten_dirty_files_are_a_tree_and_more_are_a_bold_red_count` |

**R10 test shape.** `feat/base` forks from `main`, gets one commit, and is pushed.
`feat/child` forks from it (fork record) and gets one commit. Then local
`feat/base` gets one more commit, and `pusher` pushes two different commits to
`origin/feat/base`, followed by a fetch. Against the local tip the child is
`(ahead 1, behind 1)`, and against the remote tip it would be `(1, 2)`. The test
asserts `(1, 1)`.

**Mutation check.** I temporarily changed `worktree.rs`'s parent lookup from
`refs.local(parent)` to `refs.remote("origin/{parent}")`. The new test failed
at the `(1, 1)` assertion. I then restored the file, and `git diff` shows no change.

**Snapshot.** `the_spec_example_renders_as_ruled` was re-accepted. The diff is
one line, the `chore/old-cleanup` target cell `already in` becoming `clean`.

### Deviations and notes

- `cli/tests/list_table.rs`: the const keeps the name `ALREADY_IN` for now. The
  plan puts the rename to `NOTHING_TO_MERGE` and the `EQUAL_TIPS` const in the
  Phase 3 test rework, so this phase changed only the assertion.
- **Known L2 break until Phase 4.** `cli/tests/level2_dirty_tree.rs` still asserts
  that `lib.rs` is `ORANGE`. It will fail under tmux until Phase 4 updates it, as
  planned. L1 is unaffected.
- No OS-specific code was touched, so no cross-check was needed.

### Gates

- `cargo nextest run -p worktree -E 'test(parent) | test(merge_state) | test(clean_and_conflicts)'`: 8 passed.
- `just test` in `worktree/`: 458 passed, 18 skipped (tier-gated), none failed.
- `just lint` in `worktree/`: clean.
- `grep -rn "AlreadyIn\|already in" worktree/lib/src worktree/cli` finds only the
  new absence assertion in `cli/tests/list_table.rs`. `lib/` is clean.
- No pre-existing failures.

## Phase 3

### Changes (`cli/src/commands/list_table.rs`)

- **Gate (R1/R2).** `const METRICS_MIN_WIDTH: u32 = 100`. `table()` computes
  `show_metrics = terminal.width() >= METRICS_MIN_WIDTH` once and passes it to
  `RowCells::new` beside `osc_link_support`. There is no flag or env var, and
  `list.rs` is unchanged because S1 required no change.
- **Metrics (R3/R4).** `merge_markup(comparison, show_metrics)` appends
  ` <dim><green>+A</green></dim>` and ` <dim><red>-B</red></dim>`, omitting a
  zero side, with an ASCII hyphen-minus. Both `target()` and `parent()` pass
  `self.show_metrics`, and `with_badges` then appends the PR badges, so the
  order is `state [+A] [-B] [badges]`. The `—`, `?`, `parent deleted`, and
  empty cells never call `merge_markup`.
- **Red dot (R7).** `dirty_dot(DirtySource)` is `<red>●</red>`, and the legend
  follows.
- **Legend.** The first two Branch samples are `└─`.
- **Doc pass.** `merge_markup` gained a doc (state word, counts, zero omitted),
  and `table()`'s doc names the gate. `dirty_dot`'s "Single-column text
  glyphs" and `legend_markup`'s doc still hold. `RowCells` has no docs, and none
  were added.

### Tests (`cli/tests/list_table.rs`)

- Constants: `ALREADY_IN` became `NOTHING_TO_MERGE` (0, 4) (R8), and
  `EQUAL_TIPS` (0, 0) and `AHEAD_ONLY` (3, 0) were added. In the example,
  `feat/theme` is ahead-only, `chore/old-cleanup` is nothing-to-merge, and
  `release/prep` has equal tips, so the one fixture covers zero, ahead-only,
  behind-only, both, conflicts with counts, parent deleted, `—`, empty cells,
  and badges. `?` is covered by `an_unknown_comparison_never_gets_counts`.
- Helpers: `terminal_at(width, color)` (`plain_terminal` and `color_terminal`
  wrap it at 120), `plain_at`, `target_cells` (the last two columns, counted
  from the right because the tree guide is also a `│`), and `next_line`.

| Requirement | Test |
| --- | --- |
| No counts at ≤ 99; state and badge kept | `up_to_99_columns_cells_keep_state_and_badges_without_counts` (exact cells at 99; no `+N`/`-N` token at 99 or 80), snapshot `the_table_at_99_columns_shows_no_counts` |
| Counts at 100; zero omitted; equal tips show `clean`; badge after counts; `—`/parent deleted/empty unchanged; still four columns | `counts_follow_the_state_word_and_precede_the_badge_from_100_columns`, snapshot `the_table_at_100_columns_shows_counts` |
| `?` never gets counts | `an_unknown_comparison_never_gets_counts` |
| Dim green `+N`, dim red `-N`, order `+` < `-` < badge, and conflicts with counts in color; no green at 99 | `counts_are_dim_green_and_dim_red_in_color` (S2 bytes `\u{1b}[2m\u{1b}[32m+2`, `\u{1b}[2m\u{1b}[31m-1`) |
| `NO_COLOR` reads `+2 -1` with no escapes | `no_color_counts_read_as_plain_text` |
| Red source dot, no orange | `styles_follow_the_design` (`\u{1b}[31m●`, no `38;2;255;165;0`) |
| Legend `└─ … └─ … └┄` | `the_legend_explains_both_columns`, and the real-binary `list_output::list_output_is_the_redesigned_table` |
| `already in` absent from the whole rendering (Acceptance 1) | `every_cell_kind_renders_as_ruled` (also asserts `clean -4`) |

**Snapshots.** I reviewed all three by eye. `the_spec_example_renders_as_ruled` (120
columns) now matches the spec's Option A prototype cell for cell
(`clean +2 -1  PR #99`, `clean +3`, `conflicts +1 -3  PR #104`, `clean -4`,
`clean`, `conflicts +1 -3`, `parent deleted`). The 99 snapshot shows no counts.
At exactly 100 columns, the `feat/dark-fixes` parent cell's `PR #104` badge
wraps under `conflicts +1 -3`. The spec permits this ("the 100-column gate
does not promise that arbitrary row content fits on one line"), and the test
asserts the wrapped badge on the next line. At 80, the `PR #99` badge also
wraps, so the 80-column check asserts only that no count tokens appear.

**Mutation check.** I changed the gate from `>=` to `>`. Four tests failed (the
100-column snapshot, the exact-cell test, the color test, and the `NO_COLOR` test).
Then I restored it.

### Deviations and notes

- **`cli/tests/list_output.rs`** was not in the plan's touch points. It runs the
  real `wt list` binary with captured stderr and asserted the old legend
  (`├─ … ├─`), so its expected legend line is now `└─ … └─`. Captured with no
  TTY, the width falls back to 80, so that test also shows the real binary
  with no counts (`clean`).
- The L2 files (`level2_list_verbose.rs`, `level2_dirty_tree.rs`) still assert
  orange and are Phase 4 work. `level2_list_verbose` will now also fail on its
  source-dot color under tmux until Phase 4.
- `docs/cli/list.md` still shows the `├─` legend and has no counts, which is Phase 4 work.
- The worktree skill is unchanged here. Phase 4 adds the gate line.
- The code is platform-neutral rendering with no OS-specific paths, so there was no cross-check.

### Gates

- `cargo nextest run -p worktree-cli --test list_table`: 19 passed.
- `just test` in `worktree/`: 465 passed, 18 skipped (tier-gated L2+), none failed.
- `just lint` in `worktree/`: clean, with no warnings.
- `grep -rn "orange\|AlreadyIn\|already in" worktree/lib/src worktree/cli/src worktree/cli/tests`
  finds only the L2 files (Phase 4), `styled_capture_parse.rs` (excluded), and the
  two absence assertions in `list_table.rs`.
- No pre-existing failures.

## Phase 4

### Changes

- **`cli/tests/level2_list_verbose.rs`**
    - `ORANGE` is gone and `GREEN = Color::Indexed(2)` sits beside `RED`. The
      `wt-feature` source dot is asserted red, in the row and in the legend.
    - `list_until` asserts `harness.pane_cols() >= 100` before `wt list`
      runs (the default `spawn_shell` pane is 120), so a narrower shared pane
      fails loudly instead of silently dropping the counts.
    - New count assertions in `level2_list_styles_follow_the_design_in_tmux`:
      the `feature-test` target cell is exactly `clean +1 -2  PR #99` (counts
      before the badge), `+1` dim green, `-2` dim red; `clash` reads
      `conflicts +1 -2` with `conflicts` red and not dim and the counts dim
      green / dim red; `docs-work` is behind only (`clean -2`, no `+`).
    - The Branch legend line is now asserted: three `└` connectors (no `├`),
      gray, red, and dim. The old comment promised a legend connector check
      that the test never made; it now matches.
    - Doc comments updated (constants, fixture description, section comments).
- **`cli/tests/level2_dirty_tree.rs`**: `ORANGE` replaced by `RED`
  (`Color::Indexed(1)`); `lib.rs` asserted red; module doc and section comment
  say red.
- **`docs/cli/list.md`**: the example is the reviewed 120-column snapshot
  (counts, no `already in`, `└─` legend). The red source dot, `clean`
  including nothing-to-merge, the `+N`/`-N` counts and the 100-column gate
  (independent of `-w`), the parent's local tip, and badge-after-counts are
  documented. The graph's "already in the default branch" tag rule stays (R11).
- **`README.md`**: the `-> {default}` states are `clean` or `conflicts`, plus
  counts from 100 columns; `-> parent` notes the local branch.
- **`.claude/skills/worktree/SKILL.md`**: one `wt list` bullet (two-state
  `MergeState`, `METRICS_MIN_WIDTH`, width from the render `Terminal` and not
  `--width`, the S1 stderr/stdin fallback fact, local parent tip, shared red),
  and the L2 styling bullet names the count colors and the ≥ 100 pane.
- **`os` skill not changed.** S1's fact (the `terminal_size` stdout, then
  stderr, then stdin fallback) is the same on Unix and Windows, so it is not an
  OS variance; it went into the worktree skill instead.

### Fixture finding

In the L2 design fixture every branch's fork parent is `main`, which is the
default branch, so the `-> parent` column is `—` on every row. The counts are
proven there on the `-> {default}` column only (ahead and behind, behind only,
and conflicts with counts). The parent-column counts share `merge_markup` with
the target column and are proven at L1 (`counts_follow_the_state_word_and_precede_the_badge_from_100_columns`
covers `conflicts +1 -3  PR #104` in the parent cell). I did not grow the
fixture with a nested branch, because it would change the row layout that the
other assertions pin, for no new rendering path.

### Requirement-to-test mapping

| Requirement | Test |
| --- | --- |
| Source dot red in a real terminal (row and legend) | `level2_list_verbose::level2_list_styles_follow_the_design_in_tmux` |
| `+N` dim green, `-N` dim red, before the PR badge, at ≥ 100 columns | same test (`clean +1 -2  PR #99`, span styles) |
| Zero side omitted in a real terminal | same test (`docs-work` reads `clean -2`, no `+`) |
| `conflicts` stays red, not dim, with counts beside it | same test (`clash`) |
| Branch legend `└─ └─ └┄`, gray/red/dim | same test (legend connectors) |
| Dirty-tree source names red | `level2_dirty_tree::level2_dirty_tree_renders_in_tmux` |

**Mutation check.** I removed `<dim>` from the `+N` markup in `merge_markup`.
The styles test failed with `"+1" should be dim green, but '+' has … dim:
false`. I then restored the file, and `git diff` shows no change to
`list_table.rs`.

### Gates

- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux cargo nextest run -p worktree-cli --features terminal-tests -E 'binary(level2_list_verbose) | binary(level2_dirty_tree)'`:
  7 passed, 0 skipped.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2` in `worktree/`: 17 passed (2 slow).
- `just test` in `worktree/`: 465 passed, 18 skipped (tier-gated L2+), none failed.
- `just lint` in `worktree/`: clean, with no warnings.
- `just check-tier-coverage worktree`: 0 stranded.
- `grep -rn orange worktree/cli/src worktree/cli/tests` now finds only
  `styled_capture_parse.rs` (excluded by the plan) and the Phase 3 absence
  assertion's message in `list_table.rs` (`"no orange left"`).
- No pre-existing failures. The change is test and docs only, with no OS-specific
  code, so no cross-check was run.

## Phase 5

### Acceptance audit

| # | Acceptance | Evidence |
| --- | --- | --- |
| 1 | No `already in`; no `MergeState::AlreadyIn` | grep (below) finds only the absence assertion `list_table.rs:288`; `list_table::every_cell_kind_renders_as_ruled`; `listing::merge_state_reads_ahead_first` |
| 2 | ≥ 100 columns: nonzero `+ahead`/`-behind`, none when tips are equal, before the PR badge, four columns | `list_table::counts_follow_the_state_word_and_precede_the_badge_from_100_columns`, snapshots `the_table_at_100_columns_shows_counts` and `the_spec_example_renders_as_ruled`, `level2_list_verbose::level2_list_styles_follow_the_design_in_tmux` |
| 3 | ≤ 99 columns: no counts, states and badges kept; `--width` does not move the gate | `list_table::up_to_99_columns_cells_keep_state_and_badges_without_counts`, snapshot `the_table_at_99_columns_shows_no_counts`; **new** `list_output::a_wide_width_flag_does_not_show_the_counts` (real binary, captured below 100, `-w 200` and `-w 100%`) and **new** `level2_list_verbose::level2_list_width_flag_leaves_the_counts_in_tmux` (120-column pane, `-w 40`, counts still shown) |
| 4 | `-> parent` follows 1–3 against the parent's local tip | `listing::the_parent_column_measures_against_the_parents_local_tip`; the parent cell `conflicts +1 -3  PR #104` in the L1 100-column test |
| 5 | L1 snapshots at 99/100 cover zero, one-sided, two-sided, conflicts, unavailable, badges, `NO_COLOR`; L2 asserts dim green/red | the Phase 3 tests, `list_table::an_unknown_comparison_never_gets_counts`, `no_color_counts_read_as_plain_text`, `counts_are_dim_green_and_dim_red_in_color`; `level2_list_styles_follow_the_design_in_tmux` |
| 6 | Red source dot (table and legend) and red source names; no orange | `list_table::styles_follow_the_design`, `level2_list_styles_follow_the_design_in_tmux`, `level2_dirty_tree::level2_dirty_tree_renders_in_tmux`, the `dirty_tree.rs` and `remove/report.rs` unit tests; grep below |
| 7 | Legend samples `└─ └─ └┄`, no `├─` | `list_table::the_legend_explains_both_columns`, `list_output::list_output_is_the_redesigned_table`, the L2 styles test; `legend_markup` has no `├` |
| 8 | Skill, README, `docs/cli/list.md` updated | Phase 4 edits; Phase 5 adds the `--width` proof to the skill |

### Gap closed

Acceptance 3's "`--width` does not change this threshold" held by construction
(`list.rs` passes the parsed width only to `to_git_graph`), but no test proved
it. I added both directions:

- **L1** `list_output::a_wide_width_flag_does_not_show_the_counts` runs the
  real `wt` binary with captured stderr (below 100 columns, as the existing
  `list_output_is_the_redesigned_table` already relies on). The feature branch
  is one commit ahead, and `list`, `list -w 200`, and `-w 100% list` all read
  `clean`.
- **L2** `level2_list_verbose::level2_list_width_flag_leaves_the_counts_in_tmux`
  runs `wt list -w 40` in the ≥ 100-column tmux pane and asserts
  `clean +1 -2  PR #99` with `+1` dim green. `DesignFixture::list_until` now
  takes the `wt` arguments. Its two existing callers pass `"list"`.

**Mutation checks.** With `METRICS_MIN_WIDTH = 0`, the new L1 test failed, along
with the two existing `list_output` tests. With `METRICS_MIN_WIDTH = 200`, the new
L2 test failed. The constant was restored each time, and `git diff` of
`list_table.rs` is empty.

### Incident: one edit landed in the main checkout

One shell call ran with the main checkout (`/Volumes/coding/personal/rusty-biscuit`)
as its working directory, not this worktree. My relative-path edit to
`list_output.rs` was written there, and one earlier read of
`level2_list_verbose.rs` also came from the main checkout's older copy. I found
the mistake when the test count dropped. I moved the exact 26-line hunk from the
main checkout into this worktree. `git status -- worktree/` in the main checkout
is clean again, and it had no other changes. Every later command used absolute
paths, and every result above comes from this worktree.

### Greps

- `grep -rn "AlreadyIn\|already in" worktree/lib/src worktree/cli`: only
  `cli/tests/list_table.rs:288`, the Acceptance-1 absence assertion.
- `grep -rn orange worktree/cli/src worktree/cli/tests`: only
  `styled_capture_parse.rs:25`, which the plan excludes, and the absence
  assertion message `list_table.rs:381`.
- `legend_markup` contains no `├`.

### Gates

- `just test` in `worktree/`: 466 passed and 18 skipped (the tier-gated L2+ tests). None failed.
- `just lint` in `worktree/`: clean.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2` in `worktree/`: 18 passed and
  0 failed. That is the 17 from Phase 4 plus the new `-w` test.
- `just check-tier-coverage worktree`: 0 stranded.
- **Cross-OS.** The feature is platform-neutral rendering. S1 required no
  change to `list.rs`, and no `#[cfg]` code changed, so no cross-check was
  required or run. The new L1 test depends on captured output rendering below
  100 columns. `list_output_is_the_redesigned_table` already makes that
  assumption on every OS in CI. The PR's Linux and macOS cells cover it, and
  Windows runs it on `main`.
- There were no pre-existing failures.

## Implementation of Review Findings #1

> **started at:** 2026-09-26T18:02:41-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-wt-ux/worktree/features/2026-09-26-more-info-on-list/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'Real-terminal tests do not verify the narrow width gate or parent comparison cells' at 18:02:47
        - discovered: `TmuxHarness::resize` already pins a detached session's window size, so `DesignFixture::list_in_pane` resizes a fresh pane to exactly the requested width and asserts `pane_cols()` took it; `list_until` keeps its at-least-100 check through the same helper, and the new `list_at(cols)` sizes the pane
        - added `DesignFixture::with_child` in `cli/tests/level2_list_verbose.rs`: `wt-child-work` (`child/long-descriptive-name`) forked from `feature-test`, with a fork-origin record naming `feature-test` and a stored PR #105 targeting it, so `-> parent` shows `clean +1 -1` and a badge; the long branch name pushes the table past 100 columns so the parent cell wraps at exactly 100
        - discovered: at a real 100-column pane the child's badge wraps between `PR` and `#105` (a split badge), not whole as in the L1 fixture, because the badge's inner space is a `Table` break opportunity; the test asserts that shape and both halves on the PR badge background, and the rendering is left unchanged (a design question, not this finding)
        - added `level2_list_hides_the_counts_in_a_99_column_pane` (state words and both PR badges kept, no `+N`/`-N` and no green cell in any table row, no wrapped row)
        - added `level2_list_shows_target_and_parent_counts_in_a_100_column_pane` (target and parent counts dim green/dim red before the badge, and the wrapped `#105` line alone in the `-> parent` column)
        - updated the `level2_list_verbose.rs` module doc and the `wt list` styling bullet of `.claude/skills/worktree/SKILL.md`
        - `just test`: 466 passed, 18 skipped, 0 failed
        - `just lint`: clean
        - `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2`: blocked by `level2_graph_in_kitty::level2_graph_fits_a_narrow_kitty_window`, which failed with "nothing drawn where the graph belongs" (the calling session lacks the macOS Screen Recording permission its screenshot needs); unrelated to this change, and nextest's fail-fast then cancelled the rest
        - the tier's other 18 tests, run as `cargo nextest run -p worktree-cli -p worktree --features terminal-tests -E 'test(/(^|::)level2_/) & !binary(level2_graph_in_kitty)'` with the tmux requirement: 18 passed, 0 failed, including both new tests
        - `just check-tier-coverage worktree`: 0 stranded
        - orchestrator re-ran the `level2_list_verbose` target with the tmux requirement: 9 passed, 0 failed
- work completed for 'Real-terminal tests do not verify the narrow width gate or parent comparison cells' at 18:07:06

### Successful Completion

The implementation of review cycle 1 has completed successfully in 4m 25s. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- no findings were deferred
- observation for the next review: at a real 100-column pane the `-> parent` PR badge splits across lines (`PR` / `#105`) because the badge's inner space is a table break opportunity; the new L2 test pins that shape, and whether to make it non-breaking is a design decision left open
- `just test-l2` as a whole stays red on this host only because `level2_graph_in_kitty` lacks the macOS Screen Recording permission; it needs a run from a terminal that has that permission

The files changed in this cycle are `worktree/cli/tests/level2_list_verbose.rs`, `.claude/skills/worktree/SKILL.md`, and this log.
