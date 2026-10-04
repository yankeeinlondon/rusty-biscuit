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
