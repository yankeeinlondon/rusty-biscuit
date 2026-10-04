---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-wt-skill/worktree/fixes/2026-10-03-broken-worktree-entries/spec.md"
plan: "worktree/fixes/2026-10-03-broken-worktree-entries/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/worktree/remove.md
source_files_during_phase_2:
  - worktree/lib/src/availability.rs
  - worktree/lib/src/remove/admin_entry.rs
  - worktree/lib/src/remove/mod.rs
  - worktree/lib/src/lib.rs
  - worktree/lib/src/worktree.rs
  - worktree/lib/src/listing.rs
  - worktree/cli/src/commands/list_table.rs
  - worktree/cli/tests/list_table.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/worktree/list.md
source_files_during_phase_3:
  - worktree/lib/src/fast_forward.rs
  - worktree/cli/src/commands/list_table.rs
  - worktree/cli/src/commands/list.rs
  - worktree/cli/tests/list_table.rs
  - worktree/cli/tests/list_output.rs
  - worktree/cli/tests/snapshots/list_table__closing_notes.snap
  - worktree/cli/tests/snapshots/list_table__unavailable_and_unknown_rows.snap
  - worktree/cli/tests/snapshots/list_table__unavailable_notes.snap
  - worktree/cli/tests/snapshots/list_table__unavailable_note_quoting.snap
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/worktree/list.md
source_files_during_phase_4:
  - worktree/lib/src/remove/admin_entry.rs
  - worktree/lib/src/remove/repair.rs
  - worktree/lib/src/remove/missing.rs
  - worktree/lib/src/remove/mod.rs
  - worktree/lib/src/remove/inventory.rs
  - worktree/lib/src/copy_record.rs
  - worktree/lib/src/git.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/worktree/remove.md
source_files_during_phase_5:
  - worktree/cli/src/commands/remove/mod.rs
  - worktree/cli/src/commands/remove/report.rs
  - worktree/cli/src/commands/list_table.rs
  - worktree/cli/src/exit.rs
  - worktree/cli/tests/remove.rs
  - worktree/cli/tests/level2_remove.rs
  - worktree/lib/src/remove/handoff.rs
  - worktree/lib/src/error.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/worktree/remove.md
packages:
  - worktree
  - worktree-cli
---

# Implementation Log for 2026-10-03-broken-worktree-entries (6 phases)

## Phase 1

Phase 1 is rulings, a Git spike, and a baseline. No source code changed.

### Baseline (macOS host, before any change)

- `just test` (worktree/): **958 passed, 32 skipped, 0 failed** (68 s). No pre-existing failures.
- `just lint` (worktree/): clean, exit 0, no warnings.
- The staged/modified files in `git status` are from the previous fix
  (`2026-10-03-list-overlap-and-keyless-notice`) and are not part of this work.

### Spike A — Git 2.56.0, macOS, scratch repos under `/tmp/wt-spike*` (no live checkout touched)

Setup: base repo plus linked worktrees made Missing (`rm -rf <wt>`), Unlinked
(`rm <wt>/.git`, branch and detached), Locked+Missing, and one with a garbage
`.git` file.

1. **Porcelain `prunable` line.** Every Missing and Unlinked entry printed
   `prunable gitdir file points to non-existent location`, after `branch`/`detached`,
   with and without `-z`. A bare `prunable` (no reason) was **not** observed.
   The reason strings in the 2.56 binary are all non-empty: `not a valid directory`,
   `gitdir file does not exist`, `unable to read gitdir file (%s)`,
   `invalid gitdir file`, `gitdir file points to non-existent location`. The
   parser must still accept a bare marker (`Some("")`), as the spec requires.
2. **Admin layout** `<common>/worktrees/<id>/`: `commondir gitdir HEAD index logs ORIG_HEAD refs`.
   `gitdir` holds the absolute `<path>/.git` (macOS spells it `/private/tmp/...`).
   **The `index` survives deleting the checkout** for Missing and Unlinked
   entries. `HEAD` holds `ref: refs/heads/<b>` or a SHA. `git rev-parse --git-common-dir`
   from the base returned the relative `.git`, so request `--path-format=absolute`.
3. **Repair restores all postconditions but is not targeted.** Running
   `git -C <base> worktree repair <a>` printed
   `error: unable to locate repository; .git file broken: <a>/.git` and exited **1**,
   yet recreated `<a>/.git` = `gitdir: <base>/.git/worktrees/a`. The back-reference
   stayed `<a>/.git`, `rev-parse --show-toplevel` in `<a>` gave `<a>`, and the
   listing showed the same branch with no `prunable`. On a detached entry the same
   command exited 0. **It also recreated `.git` for another unlinked worktree `b`
   that was not named** (output line `repair: .git file broken: <b>`). From the main
   checkout, Git repairs every broken worktree-side link in addition to the named
   path. A second repair of a healthy entry exited 0 with no output. This is raised
   as ruling **R9** below and set for human review.
4. **Locked + Missing.** A locked entry whose directory is gone is **not** marked
   `prunable`. The listing shows `locked <reason>` only. `git worktree remove <p>`
   and `git worktree remove --force <p>` both refused with exit 128
   (`cannot remove a locked working tree ... use 'remove -f -f'`). One `--force` does
   not bypass a lock, which confirms the plan's "no second `--force`". Note for
   Phase 2/5: because such an entry is not prunable, the classifier never sees it. It
   takes the ordinary path, where status fails, so it lists as `?`, and remove gets
   the R5 error-context path.
5. **Index check via `GIT_INDEX_FILE`.** With the Missing entry's staged file:
   `GIT_INDEX_FILE=<admin>/index git diff-index --cached --name-status refs/heads/<b>`
   printed `A f`, and `--quiet` exited 1. An unchanged index gave `--quiet` exit 0.
   **Trap: an absent index file is silently read as an empty index.** `diff-index`
   exited 0 with no output, because the HEAD was an empty commit. Index presence
   must therefore be proved separately (`symlink_metadata`) before trusting the diff.
   This confirms R2's "missing index with a present admin dir counts as not
   disposable".
6. **Staged work loss is real.** `git worktree remove <missing>` with a staged file
   in the surviving index exited 0 silently and deleted the admin dir with the
   index. R1 protects against exactly this.
7. **A garbage `.git` file is not prunable.** `.git` containing `garbage` was listed
   without `prunable`, and `git status` failed with `fatal: invalid gitfile format`.
   Such entries reach the ordinary path and list as `?` (Unknown), not `✕`. In
   practice the classifier's "`.git` present but damaged" Other case arises only for
   prunable entries.
8. **Parent-repository discovery.** For an unlinked worktree nested inside the
   base (`base/inner/wt`), `git -C base/inner/wt rev-parse --show-toplevel` returned
   **the base**. Today `dirty_status` on such an entry would report the parent's
   status as the worktree's. The repair postcondition "top-level is the target" is
   load-bearing, and skipping status for prunable entries removes the
   misattribution.

### Path helpers (for R4 and Phase 4)

- `worktree::remove::handoff::canonical(path)` wraps `biscuit_file::canonicalize_simplified`
  (`dunce::canonicalize`). It resolves macOS `/private` aliases and Windows short
  names (via `GetFinalPathNameByHandle`) and strips the `\\?\` verbatim prefix. It
  returns the input unchanged when the path does not exist, so it never canonicalizes
  a missing path into something else.
- `handoff::same_path(a, b)` is private (`canonical(a) == canonical(b)`).
  `handoff::is_within(path, dir)` is public. Phase 4 should make `same_path`
  `pub(crate)` (or move it beside `canonical`) and not write a second comparator.
- `copy_record::canonical_worktree_path` (private) canonicalizes the longest
  existing ancestor and re-appends missing components. It is the model to use if a
  comparison must include a missing tail.
- `lib/src/util.rs` holds only `dasherize`, with no path helpers.
- Git runners in `lib/src/git.rs`: `git_command_in(dir, args)` (what `dirty_status`
  uses), `git_from(base, dir, args)`, `git_from_raw`, `git_from_bytes`, and
  `git_from_bytes_allow_no_match`.
- Name resolution: `worktree::worktree::resolve_worktree(entries, name)` is a pure
  function over parsed entries. R6 calls it directly for each candidate (branch,
  then basename) and accepts a candidate only if it returns `Ok(e)` with `e.path`
  equal to this entry's path.

### Inventory of `DirtyStatus` and `WorktreeEntry` uses

`DirtyStatus` matches and defaults:
- `lib/src/worktree.rs:36` enum; `:49` `WorktreeStatus.dirty`; `:299` `gather_dirtiness`
  (spawns `dirty_status` per entry, `:303`); `:388` `gather_local`; `:443` `commit`;
  `:488-491` `refresh_dirty_status` (used after `--ff`, called from
  `cli/src/commands/list.rs:597`); `:505-535` `dirty_status` (returns `Clean` at
  `:514` on failure — the defect).
- `cli/src/commands/list_table.rs:483-485` legend; `:658` row builder; `:776-780`
  `dirty_dot` (exhaustive match, so it breaks at compile time when `Unknown` is added).
- Tests: `lib/src/listing.rs:1211-1224`; `cli/src/commands/list/tests/pipeline.rs:25,318,646-647`;
  `cli/tests/list_table.rs:21,64,90-97`.
- `cli/src/commands/git_graph*` and `lib/src/fast_forward.rs` do not read
  dirtiness. There is no `unwrap_or(DirtyStatus::Clean)` anywhere.
  `remove/inventory.rs` `dirty` is a separate `Vec<DirtyEntry>`, not `DirtyStatus`.

`WorktreeEntry` struct literals (they break when `prunable` is added):
- `lib/src/worktree.rs:145` and `:170` (the parser's two flushes); `:1593` (test closure).
- `lib/src/listing.rs:505` (test helper `entry`).
- `cli/tests/list_table.rs:66` (test helper `status`).

Other porcelain parsers that ignore `prunable`:
- `lib/src/fast_forward.rs:121` `holder_of` parses porcelain itself and must learn
  `prunable` in Phase 3 (or switch to `parse_worktree_list`).

### Rulings (R1–R9) — plan defaults adopted under `yolo: true`

- **R1** — adopted the spec's recommendation. Inspect the surviving admin index;
  ordinary discard consent is required when it differs from the recorded HEAD;
  failed inspection refuses. Spike item 6 shows the loss is real. The spec stays
  `draft-spec` until the author confirms.
- **R2** — adopted, with one addition from spike item 5: check `<admin>/index` with
  `symlink_metadata` before running `GIT_INDEX_FILE=… git diff-index --cached --quiet <HEAD>`,
  because Git reads an absent index as empty. Exit 0 means disposable, exit 1 means
  staged paths (list them with `--name-status -z`), and any other exit refuses.
  Absent admin dir plus absent checkout is a bare record and disposable.
- **R3** — adopted as written.
- **R4** — adopted. Enumerate `<git-common-dir>/worktrees/*/gitdir`, with the common
  dir from `git -C <base> rev-parse --path-format=absolute --git-common-dir` (spike
  item 2). Compare the back-reference to `<recorded-path>/.git` with
  `handoff::canonical`. Exactly one match is required.
- **R5** — adopted as written. Spike items 4 and 7 identify the cases that reach it
  (locked+missing, garbage `.git`).
- **R6** — adopted. Call `resolve_worktree` per candidate; no copy of its logic.
- **R7** — adopted (no perf work).
- **R8** — adopted (injected inspector seam for Windows reparse points).
- **R9 (new, needs author input before Phase 4)** — `git worktree repair <path>`
  run from the base also repairs every other unlinked worktree's `.git`
  (spike item 3). The spec calls the repair "targeted". Proposed default:
  accept Git's behavior, keep verifying only the target, and say in the refusal or
  report text and docs that repair "may have changed Git metadata for this or other
  worktrees". The options are recorded in the spec's `human_review_items`. Phases 2
  and 3 do not depend on R9.

### Checkpoint 1

The spike notes, rulings R1–R9, and the baseline are recorded above. No source,
docs, or skill-reference files changed except the worktree skill's `remove.md`
Git-facts note: a new section, "Git facts about broken worktree records",
recording spike items 1, 3–8. These facts describe Git, so the note is true
before any code lands.

Gates: `just test` and `just lint` were run as the baseline above. No code
changed afterward, so no rerun was needed. Environment exercised: macOS only. No
cross-OS run, because Phase 1 has no code.

## Phase 2

Parse, classify, and make status honest. Host: macOS (Darwin 27.2, Git 2.56).

### What changed

- **`WorktreeEntry::prunable: Option<String>`** (`lib/src/worktree.rs`).
  `parse_worktree_list` now flushes every entry through one closure, so the
  marker is reset per entry and kept on the last one. A line is a marker only
  when it is exactly `prunable` or starts with `prunable ` (`prunable_reason`).
  The old "We skip bare, detached, prunable lines" comment is gone, and the doc
  now states the marker's semantics.
- **Duplicate markers.** Git writes at most one marker per entry. If two
  appear, both reasons are kept, joined with `"; "`, so neither wins silently.
  This is the defined outcome for the matrix's "duplicate" cell. The field is
  display-only, so presence alone drives every decision, and a duplicated
  marker still leaves the entry prunable (never healthy). Changing
  `parse_worktree_list` to return `Result` (a true refusal) would have made
  every `wt` command fail on display text, which was judged worse.
- **`lib/src/availability.rs` (new).** It provides
  `Availability { Healthy, Missing, Unlinked, Other(OtherCondition) }` and
  `OtherCondition { NotADirectory, Link, PathUninspectable(msg),
  GitEntryUninspectable(msg), GitEntryPresent }`. The entry points are
  `classify(entry)` and `classify_with(entry, inspect)`, where `inspect` is an
  injectable `Fn(&Path) -> io::Result<EntryKind>` (the R8 seam). The real
  `inspect` uses `symlink_metadata`. On Windows, any `FILE_ATTRIBUTE_REPARSE_POINT`
  counts as `Link`, not only name surrogates. A non-prunable entry is `Healthy`
  and is never inspected. Only `NotFound` is absence. `Availability::is_unavailable()`
  is there for Phase 3's rendering.
- **`lib/src/remove/admin_entry.rs` (new).** `read_back_reference(admin_dir)`
  is the strict reader for `<admin>/gitdir`, returning a typed
  `BackReferenceError { Missing, Unreadable, Empty, MultipleLines, ContainsNul,
  NotAGitEntry, NotUtf8 }`. It trims only trailing `\n`/`\r`, as Git does,
  joins a relative path (`worktree.useRelativePaths`) onto the admin dir,
  never canonicalizes, and keeps raw bytes on Unix. **Phase 4 builds
  `admin_entry_for` (R4 association) on top of it, in this file.**
- **`DirtyStatus::Unknown`.** `dirty_status` returns it on a spawn failure or
  nonzero exit, and its doc now says so (drift fixed: it promised a `Clean`
  fallback). A new private `entry_dirtiness(entry)` returns `Unknown` without
  running Git for a prunable entry. `gather_dirtiness` and
  `refresh_dirty_status` (the `--ff` refresh) both go through it.
- **`WorktreeStatus::availability`.** It is classified once per listing in
  `WorktreeList::commit`, next to `dirty`. The `gather_local`/`commit`
  signatures are unchanged.
- **Compile-only CLI touch.** `list_table::dirty_dot` gained
  `Unknown => "<dim>?</dim>"` (the spec's `?` glyph) because the match is
  exhaustive. The `✕` marker, conditional legends, and notes are Phase 3. Until
  then a prunable row renders `?`, not `○`. `cli/tests/list_table.rs`'s helper
  sets `prunable: None, availability: Healthy`.

### Dirtiness audit (plan task)

- `dirty_dot` is the only CLI match. It is exhaustive, and the new arm is
  `?`, neither clean nor dirty-source.
- The legend (`list_table.rs:483`) still lists only the three old dots;
  Phase 3 adds the conditional entries.
- `git_graph*` and `fast_forward.rs` do not read dirtiness (re-confirmed).
- There is no `unwrap_or(DirtyStatus::Clean)` anywhere. `pipeline.rs` asserts
  `Clean` only for healthy fixtures.
- Comparison cells are untouched: they come from `gather_ref_facts`, which
  never sees dirtiness. The new listing test asserts that a comparison still
  exists for the unlinked branch.

### Requirement-to-test mapping

| Requirement | Test |
| ----------- | ---- |
| `prunable` parsed and reset per entry, last entry flushed; robustness matrix (control, absent, bare, empty reason, longer keyword, duplicate, non-UTF-8 lossy, marker text in path/branch, CRLF, trailing blank records) | `worktree::tests::the_prunable_line_matrix_from_real_git_output` (real `git worktree list --porcelain` from a deleted checkout, one edit per cell) |
| `gitdir` back-reference matrix (control, CRLF, no final newline, empty, newlines only, two lines, trailing blank line, trailing content, non-path content, NUL, relative path, non-UTF-8, absent, a directory) | `remove::admin_entry::tests::the_back_reference_matrix_from_a_real_git_fixture` |
| Classifier on real Git state: healthy / Missing / Unlinked | `availability::tests::git_listed_entries_classify_by_what_is_on_disk` |
| Bare marker still classified; healthy entry not inspected | `a_bare_marker_is_still_classified`, `a_healthy_entry_is_not_inspected` |
| Present (garbage) `.git` → `Other(GitEntryPresent)`; file at path → `NotADirectory` | `a_present_git_entry_is_other_even_when_damaged`, `a_file_at_the_recorded_path_is_not_a_directory` |
| Symlink to a dir, and a dangling symlink → `Link`, never followed or absent (Unix) | `a_link_at_the_recorded_path_is_never_followed` |
| Windows junction → `Link` (`#[cfg(windows)]`, `mklink /J`, no privilege) | `a_junction_at_the_recorded_path_is_a_link` |
| Injected permission / I/O errors on path and on `.git` are never absence; injected reparse point | `inspection_errors_are_never_absence`, `an_injected_reparse_point_is_a_link` |
| `dirty_status` nonzero exit and spawn failure → `Unknown` (real conditions on the `git_command_in` path: a non-repo dir and a missing dir) | `worktree::tests::a_failed_status_check_is_unknown_never_clean` |
| Listing: no `git status` (and no repair/prune/remove) for prunable entries; `Unknown` + availability recorded; healthy entries still measured; comparisons intact; disk untouched; `--ff` refresh of a prunable entry runs no Git | `listing::repo_tests::prunable_entries_are_unknown_and_never_run_git_status` |

Regression proof: with `dirty_status` reverted to the `Clean` fallback and
the prunable skip disabled, both `a_failed_status_check_is_unknown_never_clean`
and `prunable_entries_are_unknown_and_never_run_git_status` fail. With the fix
they pass.

Matrix cell left to Phase 4: "two admin entries back-reference the same
target" is an association outcome (refuse ambiguity), so it belongs with
`admin_entry_for`. The per-file reader cannot see it.

Code-smell grep on the `gitdir` read: no `unwrap_or_default()`, `.ok()`, or
`filter_map`.

Tier placement: every new test is a `#[cfg(test)]` unit test in the `worktree`
lib, which L1 runs. One test was first named `real_git_entries_…`. The `real_`
segment marker took it out of L1 (the skipped count rose from 32 to 33), so it
was renamed `git_listed_entries_…`. The helper `real_porcelain_records` was
renamed `git_porcelain_records` for the same reason.

### Gates

- `just test` (worktree/): **970 passed, 32 skipped** (baseline 958/32; +12
  new tests on macOS. The junction test is Windows-only, and the symlink test
  is Unix-only). Four further full runs: three green.
  One earlier run stopped fail-fast at 686/970 with 2 failures whose names were
  not captured. Every rerun, before and after, was green, and the change in
  between only renamed a test helper, so this is recorded as an unidentified
  load-related flake, not attributed to this phase.
- An unfiltered `cargo nextest run -p worktree -p worktree-cli --no-fail-fast`
  (which also runs perf gates outside L1) once failed
  `perf_pr_request::perf_a_held_live_head_check_costs_the_listing_only_its_wait`
  on timing under full parallel load. It passed in the run before. It is a
  perf gate unrelated to this phase.
- `just lint` (worktree/): clean.


## Phase 3

`wt list` markers, legend, and notes, plus the `--ff` holder refusal. Host:
macOS (Darwin 27.2, Git 2.56).

### What changed

- **Worktree glyph** (`cli/src/commands/list_table.rs`). `worktree_marker`
  renders `✕` (red) for every unavailable `availability`, whatever the
  dirtiness, and otherwise `dirty_dot`, which gives `?` (dim) for
  `DirtyStatus::Unknown`. Phase 2's interim `?` for prunable rows is gone.
- **Conditional legend.** `legend_markup(facts)` now takes the facts and
  returns a `Vec`. `✕ git can't read this worktree` and `? couldn't check`
  appear only when some table row shows that glyph. A `✕` row never adds the
  `?` entry. **Departure from the plan's wording:** these entries go on a
  second Worktree legend line, aligned under the first entry, instead of
  being appended to the first line. Appended, the line was 126 columns, wider
  than a 120-column terminal, so it overflowed the table it sizes. On its own
  line the legend is no wider than before. The table is still sized from the
  rendered legend (`legend_width(facts, …)`).
- **Notes.** `render_notes` appends one dim `unavailable_note` per unavailable
  worktree after the existing `--ff`/§9/§8 notes. Their order is table row
  order (`TableFacts::row_statuses`, each worktree once). A `?` row gets no
  note. Wording:
  - Missing: `<name>: its directory is gone; wt remove <arg> checks whether its remaining Git record can be removed safely.`
  - Unlinked: `<name>: its .git file is missing; wt remove <arg> attempts to restore the link before checking its files. To restore it without removing it, run git -C <base> worktree repair <path>.`
  - Other: `<name>: Git can't read this worktree: <observed condition>[; Git reports: <reason>].`
    The observed conditions are: not a directory; a link that may have
    replaced the checkout; path uninspectable (OS message); `.git`
    uninspectable (OS message); `.git` exists but Git can't use it. Only the
    Unlinked note claims `.git` is missing.
- **R6 name selection.** `remove_argument` tries the branch, then the
  basename. It calls `resolve_worktree` over every listed entry and accepts a
  candidate only if it resolves to this entry's path. Otherwise the note
  names `wt remove` without an argument and ends with
  `No name selects it for wt remove: <reasons>.`, where the reasons are
  "X also matches <paths>", "X selects <path>", or "X can't be typed the same
  way in every shell".
- **Shell quoting.** No quoting helper existed (`powershell_single_quoted`
  in `shell_integration.rs` covers one shell only). The new `shell_word`
  leaves a value bare when it holds only `[A-Za-z0-9_./:-]` and otherwise
  wraps it in single quotes. It returns `None` when no one spelling reads back
  unchanged in bash, zsh, fish, and PowerShell, which covers:
  - a leading `-` or `~`;
  - `'` or PowerShell's `‘’‚‛`;
  - control characters;
  - `\\` or a trailing `\`, which fish treats as escapes inside single quotes.

  `wt list` cannot know the caller's shell (stderr is captured by the
  wrapper), so the note shows one spelling that works in every supported
  shell or none at all. All text goes through `Prose::escape_text` and
  `command_badge`. Nothing displayed is executed.
- **`--ff` holder refusal** (`lib/src/fast_forward.rs`). `holder_of` now uses
  `parse_worktree_list` and returns the new
  `FfRefusal::UnavailableHolder(path)` for a `prunable` sole holder.
  `fast_forward_default` applies that refusal only when a move is needed, so an
  in-sync branch with a broken holder is still `UpToDate` (no note on every
  `--ff`). The CLI maps it to `FfNotice::UnavailableHolder`:
  `main wasn't fast-forwarded: Git can't read the worktree that has it checked out, <path>.`
  **Defect found:** before this, a prunable holder reached
  `git -C <holder> symbolic-ref` and then `merge --ff-only`. For an unlinked
  directory nested in another repository, Git discovers the parent (Phase 1
  spike item 8), so the merge could have run against an unrelated checkout.
  Healthy-holder behavior is unchanged (all 14 existing fast-forward tests pass).
- **Comparison cells are unchanged.** Unavailable and `?` rows show the same
  target/parent cells as a healthy row with the same comparisons. Nothing
  derived from dirtiness or availability feeds them.
- **Listing side effects.** No repair, prune, or record removal. Cache and
  worker paths are untouched (only `list_table.rs` rendering and the
  `ff_notice` mapping in `list.rs` changed in the CLI).

### Requirement-to-test mapping

| Requirement | Test |
| ----------- | ---- |
| `✕` for every unavailable kind, `?` for Unknown on a readable row, `○` only for checked-clean | `list_table::a_row_git_cannot_read_is_a_cross_never_clean`, snapshot `unavailable_and_unknown_rows` |
| `✕` outranks every dirtiness value; a `✕` row adds no `?` legend entry | `unavailability_outranks_any_dirtiness` |
| Colors: red `✕`, dim `?` | `the_cross_is_red_and_the_question_mark_dim` |
| Conditional legend: none / `✕` only / `?` only / both; Branch legend unchanged | `legend_entries_appear_only_for_markers_in_the_table`; the existing `the_legend_explains_both_columns` still pins the healthy legend |
| Table sized from the rendered legend | `a_narrow_table_is_as_wide_as_its_widest_legend_line` (+ existing `a_narrow_table_is_as_wide_as_the_legend`) |
| Comparisons keep their meaning | `comparisons_of_unavailable_rows_keep_their_meaning` |
| Note text per state, Git reason only when present (bare marker omits it) | snapshot `unavailable_notes` |
| Notes follow the existing notes, in table row order | `unavailable_notes_follow_the_existing_notes_in_table_row_order` |
| Notes are dim; commands in reverse video; no note for healthy or `?` rows | `unavailable_notes_are_dim`, `healthy_listings_have_no_unavailable_notes` |
| `.git` missing claimed only for Unlinked | `a_note_claims_a_missing_git_file_only_for_an_unlinked_row` |
| R6: ambiguous basename, branch selecting another worktree, branch preferred, detached falls back to basename | `notes_never_suggest_a_name_that_selects_another_worktree` |
| Shell quoting: plain, spaces, `'`, `’`, `$(…);`, leading `-`, Windows base path, UNC `\\`, trailing `\` | snapshot `unavailable_note_quoting` |
| Markup escaping of names, paths, and reasons | `names_paths_and_reasons_never_become_markup` |
| `--ff` notice text | `closing_notes_snapshot` (new "holder unavailable" case) |
| End to end through the binary with real Git: `lhg-before`-shaped (detached, `.git` deleted, working file kept) and Missing beside healthy: `✕`/`✕`/`○`, legend, both notes with Git's spelling of the base and target in the repair command, row order, and no repair, recreation, or record change | `list_output::worktrees_git_cannot_read_are_crossed_and_explained_without_repair` |
| `--ff` refuses under an Unlinked holder nested in the base: no ref moves, no `merge`/`update-ref`/`symbolic-ref`/`repair`/`prune` call (recorder), link not recreated, base untouched | `fast_forward::tests::refuses_a_move_under_an_unlinked_holder_without_repairing_it` |
| `--ff` refuses under a Missing holder; record kept, nothing recreated | `refuses_a_move_under_a_missing_holder_and_keeps_its_record` |
| An unavailable holder of an in-sync branch is `UpToDate` | `an_unavailable_holder_of_an_up_to_date_branch_is_not_a_refusal` |
| No `git status`/repair for prunable entries; status failures injected via `git_command_in` | Phase 2: `listing::repo_tests::prunable_entries_are_unknown_and_never_run_git_status`, `worktree::tests::a_failed_status_check_is_unknown_never_clean` |

Regression proof:
- With `holder_of` treating a prunable holder as healthy, both refusal tests
  fail.
- With `worktree_marker` reverted to `dirty_dot`, six tests fail, including
  the end-to-end test.

Both files were restored afterward.

Snapshot hygiene: every new or changed `.snap` was read before acceptance.
Review caught two defects:
- A note without a usable name read as a broken sentence. It was reworded so
  the reason is its own closing sentence.
- A Windows path used as a *target* gives a host-dependent `file_name`. Those
  cases now put the Windows spellings on the base path, which is shown whole
  on every host.

`assertion_line` was stripped from the four touched snapshots to match the
repository's other snapshots.

Tier placement: new tests are in `cli/tests/list_table.rs` and
`cli/tests/list_output.rs` (autotests; L1) and the `worktree` lib unit tests.
No test name or module segment has a tier marker.

Input robustness: no new parser. `holder_of` now reuses
`parse_worktree_list`, whose `prunable` matrix was covered in Phase 2.

### Gates

- `just test` (worktree/): **989 passed, 32 skipped** (Phase 2: 970/32; +19).
- `just lint` (worktree/): clean.

## Phase 4

Library engines for removal: association, repair with verification, and the
missing-directory record removal. Library only; `wt remove` wiring is
Phase 5. Host: macOS (Darwin 27.2, Git 2.56).

### Rulings applied

- **R9 still awaits the author.** The recorded default (option A) is in
  place. `repair_unlinked` verifies only the target, and its module and
  `RepairRefusal` docs say an attempt "may have changed Git metadata for this
  or other worktrees". Options B and C would change only the guard before
  `git.repair(..)`, so Phase 5 is not blocked either way.
- **R1/R2** are implemented as recorded. The index must exist as a regular
  file (Git reads an absent index as empty). It is diffed against the
  recorded HEAD. Unreadable means refuse.
- **R2's "absent admin directory = bare record, disposable"** cannot occur:
  Git builds the porcelain listing from those admin directories, so a listed
  entry always has one. A record with no match refuses (`AssociationError::NotFound`)
  and is never treated as disposable.

### What changed

- **`remove/admin_entry.rs`**:
  - `common_git_dir(base)` uses `rev-parse --path-format=absolute --git-common-dir`.
  - `admin_entry_for(base, target)` and `admin_entry_in(common, target)` return
    `AdminEntry { dir }` (with `index()`) or `AssociationError { CommonDirUnknown,
    Unlistable, Unreadable, NotFound, Ambiguous }`.
  - Every directory under `worktrees/` is read. A plain file is skipped, as Git
    skips it. A linked or unreadable record refuses.
  - `same_location` is the shared comparator.
  - **Departure from R4's wording:** R4 compared back-references to
    `<path>/.git`. The code compares the checkout directories instead
    (`back-reference.parent()` against the target). The reader already proves
    the last component is `.git`. Found by test: another record whose checkout
    was replaced by a *file* made `canonicalize(<file>/.git)` fail with ENOTDIR,
    which blocked association, and so removal, for every other worktree. The
    `.git` form compared the same location and failed on an unrelated record.
- **`copy_record::canonical_worktree_path`** is now `pub(crate)` (a doc was
  added) and is reused, not copied. It canonicalizes the longest existing
  ancestor and re-appends the missing tail, so a Missing target still compares
  equal across `/private` aliases and Windows short names. No missing path is
  ever canonicalized into something else.
- **`remove/repair.rs` (new)**:
  - `RepairGit` is the seam. It has three calls: `repair`, `rev_parse_path`,
    and `worktree_list`. The real implementation is `Git`.
  - `repair_unlinked(base, entry, git)` refuses without running anything
    unless the entry is still `Unlinked` and exactly one record is associated.
    Otherwise it runs `git -C <base> worktree repair <path>` from the base and
    collects every failed `Postcondition { GitDir, CommonDir, Unresolvable,
    BackReference, TopLevel, Listing }`.
  - The exit code, stdout, stderr, and spawn error are kept in `RepairAttempt`
    (with `diagnostics()` for display) and never decide the result.
    `RepairRefusal::repair_attempted()` tells Phase 5 when to say metadata may
    have changed.
- **`remove/missing.rs` (new)**:
  - `recorded_head` gives the branch tip from refs, or the recorded HEAD when
    detached.
  - `inspect_missing` returns `MissingCheckout { admin, head, staged }`, or
    `MissingRefusal`.
  - `remove_missing_record` re-lists, checks identity and confirmed absence,
    runs plain `git worktree remove <path>` (no `--force`, no `check_not_in_use`),
    then calls `copy_record::delete_for` only on success, returning its warning.
- **`remove/mod.rs`**: `CheckoutState { Healthy, Missing(MissingCheckout),
  Repaired(RepairReport) }` and `prepare(base, entry, git)`. `prepare`
  classifies afresh and dispatches, with `PrepareRefusal { Unavailable,
  Missing, Repair }`.
  - **Departure:** the plan places `CheckoutState` "in `Facts`". `Facts` is the
    CLI's (`cli/src/commands/remove/mod.rs`), and Phase 4 is library-only, so
    the type and `prepare` are delivered here and Phase 5 stores the state in
    `Facts`. `MissingCheckout` is a distinct type, not an empty `Inventory`.
  - **Order of effects:** record removal, then copy-record cleanup, happens in
    `remove_missing_record`. The branch steps run afterward in the CLI's
    existing `execute` (Phase 5). A failed record removal returns `Err` before
    any cleanup (tested).
- **`git.rs`**:
  - `git_from_bytes_with_env`: the `env` slice was threaded through the private
    `git_from_bytes_status`, used here for `GIT_INDEX_FILE`.
  - `git_from_output`: OS-string arguments, the whole `Output` returned whatever
    the exit status, stdin closed, `GIT_TERMINAL_PROMPT=0`. The recorder logs
    its arguments lossily.
  - Paths therefore reach Git byte-exact, never as a lossy `String`. The
    existing `remove_worktree` still uses `display().to_string()` (untouched,
    Rule 3).
- **`remove/inventory.rs`**: `path_from_git` is now `pub(crate)` and reused
  for `diff-index -z` paths.

### Requirement-to-test mapping

All are `#[cfg(test)]` unit tests in the `worktree` lib (L1). They use real
Git in disposable `TestRepo`s, and no network.

| Requirement | Test |
| ----------- | ---- |
| Association: control row, plain file skipped, another record's checkout replaced by a file, duplicate back-reference → `Ambiguous` (matrix "duplicate" cell left from Phase 2), empty and missing `gitdir` in another record → refuse, relative back-reference, non-matching → `NotFound` | `admin_entry::tests::association_requires_exactly_one_readable_record` |
| Missing target associates by Git's spelling | `a_missing_checkout_is_associated_by_its_spelling` |
| Common dir from Git when base `.git` is a file (`--separate-git-dir`) | `the_common_directory_comes_from_git_not_from_the_base_layout` |
| Real repair (attached, with a tracked edit kept), verified by postconditions | `repair::tests::git_repair_of_an_unlinked_branch_is_verified_by_its_postconditions` |
| Real repair of a detached entry; same HEAD | `git_repair_of_an_unlinked_detached_worktree_is_verified` |
| Nonzero exit + valid postconditions → success; diagnostics kept | `a_nonzero_exit_with_valid_postconditions_is_success` |
| Zero exit + no change → refuse; files, record, branch intact | `a_zero_exit_without_a_change_refuses_and_touches_nothing` |
| Spawn failure named, never "repaired" | `a_spawn_failure_is_named_and_never_claimed_as_repaired` |
| Wrong admin entry in the same repository | `a_link_to_another_record_in_the_same_repository_refuses` |
| Foreign repository (`GitDir` + `CommonDir`) | `a_link_into_a_foreign_repository_refuses` |
| Parent-repository discovery (nested in base, real Git) | `discovering_the_parent_repository_is_not_success` |
| Top-level check alone is load-bearing | `a_wrong_top_level_alone_refuses` |
| Identity change during repair | `a_changed_identity_refuses` |
| No repair unless Unlinked (healthy, missing) | `no_repair_runs_unless_the_target_is_unlinked` |
| Ambiguous association refuses before repair | `an_ambiguous_record_refuses_before_repair` |
| Disposable missing record removed alone: no `status`/`prune`/`--force`, one `worktree remove`, other worktree and branch intact, copy record cleaned after | `missing::tests::a_disposable_record_is_removed_alone_without_inspecting_a_checkout` |
| Staged work in the surviving index (`M `, `A `; unstaged file not reported) | `staged_work_in_the_surviving_index_is_reported` |
| Detached: compared with recorded HEAD even after its branch is deleted | `a_detached_record_is_compared_with_its_recorded_head` |
| Garbage, absent, and directory index → refuse | `an_index_that_cannot_prove_staged_work_disposable_refuses` |
| Only a confirmed-absent directory is inspected | `only_a_confirmed_absent_directory_is_inspected` |
| Reappeared directory (with new work) and reappeared link (Unix) refuse | `a_directory_that_reappears_is_never_removed`, `a_link_that_appears_is_never_followed_or_removed` |
| Changed identity at the path refuses | `a_changed_entry_at_the_path_refuses` |
| Record-removal failure (a lock; one plain call) keeps record and copy record | `a_failed_record_removal_keeps_the_record_and_its_copy_record` |
| `--name-status -z` parser strictness | `name_status_output_is_parsed_strictly` |
| `prepare` dispatch: Healthy / Missing / Unlinked→Repaired / file at path→`Unavailable(NotADirectory)` | `remove::tests::preparation_follows_the_checkout_state` |

Regression proof for the association fix:
`preparation_follows_the_checkout_state` failed with
`Unreadable { dir: …/worktrees/d, error: "Not a directory (os error 20)" }`
under the `.git` comparison and passes with the checkout comparison. The
association matrix now pins that cell.

Input robustness: the only new parser is the `diff-index --name-status -z`
reader. It rejects a status without a path and a multi-letter status, and
never skips a record. The `gitdir` matrix's last open cell (two records
naming one target) is now covered. Code-smell grep over `admin_entry.rs`,
`repair.rs`, and `missing.rs`: no `unwrap_or_default()`, `.ok()`, or
`filter_map` on a load-bearing read.

Tier placement: two tests first named `real_repair_…` were renamed
`git_repair_…` before the first run (the `real_` marker would strand them).
No test name or module carries a tier marker.

### Gates

- **macOS** `just test` (worktree/): **1015 passed, 32 skipped** (Phase 3:
  989/32; +26 new).
  - Two earlier full runs each stopped fail-fast on one timing-held test in
    `worktree-cli::list_prs`. The first was a timeout whose name was not
    captured. The second was
    `a_failed_assertion_while_a_live_head_check_is_held_still_frees_both_locks`.
  - `list_prs` alone passed 34/34, and the next full run was fully green.
  - This phase changed no CLI code and nothing on the list path, so the two
    stops are recorded as load-related flakes, like Phase 2's.
- **macOS** `just lint`: clean. Clippy's `type_complexity` on the test seam
  was fixed with two type aliases.
- **Linux** (`just cross-check worktree --os linux remove::`, build-linux):
  90/90 `remove::` tests passed.
- **Windows native** (`just cross-check worktree --os windows remove::`):
  - The archive **compiled**, but the test run was **blocked by the host's
    storage preflight**: 22.3 GiB free against the 50 GiB required, and the
    automatic sweep freed 0. I did not override it or delete anything on the
    shared host.
  - That build reported `worktree (lib test) generated 1 warning`, and the
    log did not capture its text.
  - A local `cargo check -p worktree --tests --target x86_64-pc-windows-gnu`
    (all `cfg(windows)` paths) is warning-free, so the warning's source is
    unidentified. It may predate this phase.
  - Windows evidence for these tests is therefore still owed: by CI on push
    to `main`, or by a cross-check once the host has headroom (`just windows-sweep`).
- WSL2 was not exercised. CI's nightly run covers it.

## Phase 5

Wiring `wt remove`. Host: macOS (Darwin 27.2, Git 2.56). R9 still has the
recorded default (option A); every post-repair message says repair "may have
changed Git metadata for this or other worktrees" (refusal) or "may also have
restored other worktrees' links" (success).

### What changed

- **`cli/src/commands/remove/mod.rs`**:
  - `run` calls `remove::prepare(base, &entry, &repair::Git)` after the
    main-checkout guard, the inside-without-wrapper guard, and
    `set_current_dir(base)`, and before `Facts::local`. Every
    `PrepareRefusal` becomes `RefusedToLoseWork` (exit 3) through
    `prepare_refusal_markup`. That markup uses the spec's wording for an
    unverified repair, says each failed postcondition once (identical
    `Unresolvable` messages are merged), and lists the captured repair output,
    including a named spawn failure. It never advises `git worktree prune`.
  - `Facts` gains `checkout: CheckoutState`. `Facts::local(base, entry,
    checkout)` skips `collect_inventory`, include discovery, and the head
    lookup for `Missing`, taking `MissingCheckout.head` instead. `inventory`
    is left empty and documented as never meaning "checked clean".
    `situation()`, the report, the discard question, and `refusal_markup`
    branch on `facts.missing()`.
  - `Repaired` prints `Restored the link for <name> so its files could be
    checked.` before the report. Refusals, the unprovable-remote refusal, and
    cancellation then append `kept_link_note`, and a failed inventory after a
    repair says the link was left in place.
  - A Missing target never hands off. `execute` runs `remove_missing_record`
    (no `--force`, no Windows rename probe), prints `Removed the record of
    worktree <name> (its directory at <path> was already gone)`, and only then
    runs branch steps. `missing_record_error` maps `Reappeared` to exit 3 with
    "run again", `RecordRemoval` to its own category with context ("no branch
    was deleted"), and the rest to exit 3.
  - `in_context(error, operation)` prefixes target, path, and operation onto
    `GitCommand`, `GitParse`, `IncludeSetDiscovery`, and `Io`, and returns
    every other variant unchanged, so exit codes never move. It is applied to
    `Facts::local`, `remove_worktree`, branch deletion (partial success:
    "removed worktree X, but could not delete branch B"), and reading the
    handoff link.
  - `run_handoff` never calls `prepare`. It refuses with exit 3 unless
    `availability::classify` is `Healthy`, then binds `checkout_git_dir`
    into the fresh state.
- **`lib/src/remove/handoff.rs`**: `HandoffState.git_dir` and
  `checkout_git_dir(base, target)` (`rev-parse --path-format=absolute
  --git-dir` run from the base, canonical). `verify` names a mismatch
  "worktree link". `HANDOFF_FORMAT_VERSION` is now 4, so a v3 record from an
  older `wt` is `Missing` (exit 4, start over).
  - **Departure:** the plan says the inventory *fingerprint* should
    incorporate checkout state. It is a separate bound field instead: the
    fingerprint describes file contents, and a redirected `.git` changes
    neither the contents nor the records' back-references. Only the forward
    link shows it.
  - A Missing target cannot reach a handoff record at all (it never hands
    off), so "a Missing target that reappears cannot reuse approval" holds by
    construction. `remove_missing_record`'s reappearance check covers the
    single-run case.
- **`cli/src/commands/remove/report.rs`**: `ReportInput.missing` and
  `missing_markup`. When the index matches HEAD, it says so. Otherwise it
  lists the staged paths (a count above 10).
- **`cli/src/commands/list_table.rs`**: `shell_word` is now `pub(crate)`,
  and `observed_condition` was extracted from `unavailable_note`. The remove
  refusals reuse both, not copies. The rendered note text is byte-identical
  (the list snapshots are unchanged).
- **Exit-code contract:** the `RefusedToLoseWork` and `BlockedByEnvironment`
  docs in `lib/src/error.rs` and the `cli/src/exit.rs` module doc now say
  "nothing removed", not "nothing changed", and name the repair exception. The
  mapping itself is unchanged: cancel 0, refusal 3, environment/in-use 4.

### Findings

- **A shell standing in an unlinked worktree can't reach the repository**
  unless the worktree is nested in another checkout of it. `find_worktree`
  runs Git in the caller's directory and fails with a bare `fatal: not a git
  repository` before `prepare` runs. This was found by the first handoff test
  (`wts/` is a sibling of the repository). It predates this fix, and it is
  outside the plan: resolving the repository from somewhere other than the
  caller's directory is a base-discovery change. Removal from any other
  directory works. The handoff-after-repair tests nest the worktree in the
  base checkout (`.nested/`), the only layout where this path is reachable.
  Recorded in the skill as a trap.
- **A locked worktree whose directory is gone is not `prunable`** (Phase 1
  fact), so it takes the ordinary path and fails in `collect_inventory`. It
  now exits 1 with target/path/operation context, and everything is kept.
  Pinned by a test.
- No comment drift was found in the touched symbols beyond the
  error/exit-code promises the plan named, which were rewritten.

### Requirement-to-test mapping

L1 through the real binary (`cli/tests/remove.rs`, autotests; no tier
marker in any name) unless noted.

| Requirement | Test |
| ----------- | ---- |
| Missing: only that record removed, safe branch deleted, other worktree intact, nothing recreated, no "checked clean" claim | `a_missing_directory_has_only_its_record_removed_and_its_safe_branch_deleted` |
| Missing: staged index → consent (exit 3 without a terminal, record + branch kept); `--force-worktree` removes | `staged_work_left_in_a_missing_directory_record_needs_consent` |
| Missing: detached | `a_missing_detached_directory_has_its_record_removed` |
| Missing: unsafe branch kept with a warning | `a_missing_directory_keeps_an_unsafe_branch_without_a_terminal` |
| Missing: unprovable index refuses with every force flag; no `prune` advice | `a_missing_directory_whose_index_is_gone_refuses_even_with_every_force_flag` |
| Missing: reappearance, record-removal failure, cleanup only after success | lib `missing::tests` (Phase 4; a race can't be staged through the binary) |
| Unlinked, `lhg-before`-shaped (detached, `.git` gone, working file kept): repaired, ordinary inventory, exit 3 without a terminal, link left in place and said so, never "not a git repository"; `--force-worktree` then removes | `an_unlinked_worktree_is_repaired_then_its_files_are_protected` |
| Unlinked clean: repaired, removed, safe branch deleted | `a_clean_unlinked_worktree_is_repaired_and_removed_with_its_safe_branch` |
| Failed repair (real Git, read-only directory; Unix): exit 3 with every force flag, the spec's three sentences, postconditions said once, repair output, no "Restored", no `prune`, no "Nothing was changed"; files, record, and branch intact | `a_repair_that_is_not_verified_refuses_even_with_every_force_flag` |
| Injected failed/incorrect repairs (no change, wrong record, foreign repository, parent discovery, identity change) and exit status either way | lib `repair::tests` (Phase 4) |
| Other: a file at the path refuses with every force flag; the file is intact | `a_worktree_replaced_by_a_file_refuses_even_with_every_force_flag` |
| Other: a link at the path is never followed or repaired through (Unix) | `a_worktree_replaced_by_a_link_is_never_followed` |
| Error context: status failure names target and operation, keeps exit 1 and Git's reason, removes nothing | `a_status_failure_names_the_target_and_operation_and_removes_nothing` |
| Error context: locked + missing | `a_locked_worktree_whose_directory_is_gone_fails_with_context_and_keeps_everything` |
| Context never changes an exit code (all categories, incl. 3/4/0) | unit `commands::remove::tests::context_never_changes_an_exit_code` |
| Handoff after repair: ordinary checks, removed by the second run | `a_repaired_worktree_hands_off_and_finishes_with_the_ordinary_checks` |
| Handoff: link broken between runs → exit 3, no second repair | `a_link_broken_between_the_runs_refuses_without_another_repair` |
| Handoff: link redirected to another record (after a repair, and on a healthy worktree) → exit 3 "worktree link" | `a_link_redirected_between_the_runs_refuses_with_nothing_removed` |
| `git_dir` follows the forward link (redirect, broken) | lib `handoff::tests::the_checkout_git_dir_follows_the_link_not_the_record`; `every_changed_field_refuses` (new "worktree link" row); round-trip test covers the new field |
| Cancellation after a repair keeps the link and says so (real terminal) | L2 `level2_remove::level2_declining_after_a_repair_keeps_the_restored_link_and_the_files` |
| Repair, then a move-first removal through the bash wrapper (real terminal) | L2 `level2_a_repaired_worktree_moves_first_through_the_wrapper` |

Regression proof:
- With `prepare` replaced by `Ok(CheckoutState::Healthy)`, 13 of the 14 new
  binary tests fail. The two that pass are the error-context tests, which
  don't depend on `prepare`.
- With `in_context` removed from `Facts::local`, exactly those two fail.
- The file was restored after each check.

Input robustness: no new parser. `checkout_git_dir` trusts Git's answer, and
any failure is a refusal (second run) or a contextual error (first run).
Code-smell grep over the added CLI lines: no `unwrap_or_default()`,
`.ok()`, or `filter_map`.

### Gates

- **macOS** `just test` (worktree/): **1033 passed, 32 skipped** (Phase 4:
  1015/32; +18).
- **macOS** `just lint`: clean.
- **macOS** `just test-l2`: **34 passed** (2 new). The recipe's harness
  broker pre-spawns its Apple Terminal window as it always does. The new
  scenes run in a detached tmux pane, which never takes focus.
- **Windows**:
  - A local `cargo check -p worktree -p worktree-cli --tests --target
    x86_64-pc-windows-gnu` compiles. Its only warning is the existing unused
    `seed_fresh_answers` in `cli/tests/list_remote_head.rs`, which this phase
    did not touch.
  - `just cross-check worktree-cli --os windows …` on build-win-native
    **compiled and archived the MSVC test binaries**, but the run was
    **blocked by the host's storage preflight**: 17.6 GiB free against the
    50 GiB required. I did not override it or delete anything on the shared
    host.
  - The build log identifies Phase 4's unexplained "1 warning": it is the
    MSVC linker's `Creating library … .lib` stdout notice, which is harmless.
  - Native Windows test evidence is still owed (CI on push to `main`, or a
    cross-check once the host has room).
- **Linux** (build-linux):
  - `just cross-check worktree --os linux remove`: 92/92 lib `remove` tests.
  - `just cross-check worktree-cli --os linux …`: every new binary test passed
    (the read-only-directory test ran; it did not skip), as did the CLI
    `remove` unit tests (60 + 12 runs across two filters).
  - The first CLI attempt failed to *link* two untouched test binaries
    (`level2_list_verbose` among them) on the remote. An identical rerun
    linked and passed, so it is recorded as a transient remote link failure.
    The plain `remove` filter matches test names, not the binary, so the
    remaining new tests were run by name.
- WSL2 was not exercised; CI's nightly run covers it.
