---
spec: /Volumes/coding/wt/rusty-biscuit/fix-wt-touchup/worktree/features/2026-10-02-fresh-prs/spec.md
plan: worktree/features/2026-10-02-fresh-prs/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
    - worktree/lib/src/pull_requests.rs
    - worktree/lib/src/remote_head.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - worktree/lib/src/remote_head.rs
    - worktree/cli/src/args.rs
    - worktree/cli/src/main.rs
    - worktree/cli/src/commands/refresh_worker.rs
    - worktree/cli/src/commands/list.rs
    - worktree/cli/src/commands/list/wait.rs
    - worktree/cli/src/commands/list/wait/tests.rs
    - worktree/cli/src/commands/list/tests.rs
    - worktree/cli/src/commands/list_table.rs
    - worktree/cli/tests/list_table.rs
    - worktree/cli/tests/snapshots/list_table__pr_presentation_every_row.snap
    - worktree/cli/tests/list_prs.rs
    - worktree/cli/tests/list_flags.rs
    - worktree/cli/tests/list_remote_head.rs
    - worktree/cli/tests/level2_list_verbose.rs
    - worktree/cli/tests/perf_support/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/worktree/SKILL.md
source_files_during_phase_3:
    - worktree/cli/src/commands/list/tests.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
    - worktree
    - worktree-cli
---

# Implementation Log for 2026-10-02-fresh-prs (5 phases)

## Phase 1

Scope: the `worktree` library only. As the plan expects, `worktree-cli` does not compile at the end of this phase. Phases 2 and 3 move it onto the new surface.

### What changed

`lib/src/pull_requests.rs`

- Store **format 5**: `writer` removed from `StoreFile`. `Writer` and `Publication` are deleted. `stored_publication(store, origin, now) -> Option<String>` returns the id alone, under `select_cached`'s rules.
- The store reader is strict, per the Input Robustness Matrix. Every field must be present: a missing key, a wrongly typed field or element, a duplicate key, or trailing content makes the file a miss. `source_repo` and each PR's `url`/`source_repo` now use a `present` deserializer, so a missing key is a miss while `null` is still `None` (serde's `Option` would have read missing as `None`). `publication` must be a valid attempt id (32 lowercase hex), so `""` is a miss.
- Ruling 2 (unknown fields): the store has always ignored unknown fields, and it still does. A format-5 file with a stray `writer` key reads normally. A format-4 file is a miss because of its `format_version`.
- `refresh(store, main, clock, connect)`: `force` is gone, and so is the freshness recheck. Winning the lock means making the request. `RefreshOutcome::AlreadyFresh` is removed and `RefreshOutcome::Unsupported` is added.
- `fetch_and_publish`, `LIST_DEADLINE`, and `Answer::into_listing` (used only there) are removed. `FRESHNESS_WINDOW` stays, as the threshold for showing the age line.
- Unsupported (ruling 7): new `PrRequestError { Unsupported, Failed(PrFailure) }` is the error type of `OpenPrSource::fetch`. `PrRequestError::from_unavailable` takes sniff's `PrUnavailable::Unsupported` out before delegating to `PrFailure::from_unavailable`. That function keeps its signature but is now private, so `Unsupported` cannot reach a `PrFailure` through any public path.
- Module and symbol docs were rewritten: the worker is the only writer, every refresh that wins the lock asks, and there is no two-writer ordering rule.

`lib/src/remote_head.rs`

- `PrStatus::SkippedFresh` removed, `PrStatus::Unsupported` added (serialized `{"kind": "unsupported"}`). Ruling 1: the receipt format is not versioned, so a receipt with `skipped-fresh` reads as missing.
- `is_attempt_id` is now `pub(crate)` so the PR store can validate `publication`.
- New `pub(crate) fn present`, shared by both readers. `PrFailure`'s `key` fields use it, so a receipt whose credentials failure leaves out `key` is missing instead of reading as "no variable named". Found by the receipt matrix test.
- The `PrFailure` doc no longer mentions the foreground request.

### Requirement-to-test mapping (all L1, `lib/src/*` unit tests)

| Requirement | Test |
|---|---|
| Store format 5; format 1–4 files exactly as written are misses | `pull_requests::tests::earlier_formats_exactly_as_they_were_written_are_misses` |
| Store Input Robustness Matrix (absent, null, wrong type, wrong/every element, empty, duplicate, trailing for `format_version`, `publication`, `origin_digest`, `fetched_at`, `source_repo`, the list, one element; with a control row, `[]` as an answer, `null` `source_repo`/`url` as valid, unknown `writer` ignored) | `pull_requests::tests::the_store_reader_walks_the_input_robustness_matrix` |
| `refresh` asks while a fresh answer is stored, including same-second and empty answers | `pull_requests::tests::a_refresh_asks_even_while_a_fresh_answer_is_stored` |
| New publication id per write, even within one second; a failure publishes nothing | `every_publication_stores_a_new_id_even_within_one_second` |
| Contended: no request; the sidecar persists | `a_refresh_makes_no_request_while_another_holds_the_lock`, `a_refresh_during_another_refreshs_request_is_contended` (the lock is held through the request) |
| LockFailed, NoOrigin | `a_refresh_that_cannot_open_its_lock_makes_no_request`, `a_refresh_without_an_origin_makes_no_request` |
| Failed is not stored (every credentials condition) | `a_failed_refresh_leaves_the_stored_answer_untouched`, `each_credentials_failure_is_reported_and_never_stored` |
| Origin changed during the request is not stored | `a_refresh_discards_its_answer_when_origin_changed_during_the_request` |
| Unsupported: sniff mapping, real local-path origin end to end, stored answer untouched | `every_sniff_reason_maps_to_its_condition_with_unsupported_kept_apart`, `a_refresh_for_a_local_path_origin_is_unsupported_and_stores_nothing` (real `SniffOpenPrSource`), `an_unsupported_answer_leaves_a_stored_answer_untouched` |
| Receipt variants round-trip, including `unsupported` | `remote_head::tests::a_receipt_round_trips_with_its_documented_spellings` |
| Receipt Input Robustness Matrix (every field absent, null, `[]`, `{}`, wrong type, empty, wrong id/origin/branch, pre-attempt timestamp, unknown `prs` variant such as `skipped-fresh`, nested `failure`/`kind`/`key`, duplicate keys at the top level and nested, trailing content; control row; `null` key valid; `ok` stays the receipt's answer) | `remote_head::tests::the_receipt_reader_walks_the_input_robustness_matrix` |

Tests removed with the code they tested: `a_foreground_answer_never_replaces_*` (2), `a_foreground_answer_is_shown_but_not_stored_while_a_refresh_holds_the_lock`, `a_failed_foreground_request_leaves_the_store_untouched`, `a_foreground_answer_is_discarded_when_origin_changed_during_the_request`, `a_refresh_skips_the_request_when_the_answer_is_already_fresh`, `a_forced_refresh_asks_even_when_the_answer_is_fresh` (replaced by `a_refresh_asks_even_while_a_fresh_answer_is_stored`). `corrupt_old_format_other_version_and_future_stores_are_misses` became the format test plus the matrix.

### Gates run

- `cargo nextest run -p worktree`: 305 passed, 0 skipped.
- `just _lint worktree` (clippy, `-D warnings`, all targets): clean.
- `just test` / `just lint` for the whole area were **not** run to green: `worktree-cli` does not compile until Phases 2 and 3, as the plan's checkpoint 1 says.
- No OS-specific code was touched (no paths, no `cfg`), so no cross-OS run was needed for this phase.
- Matrix smell grep: in the touched readers, each `.ok()` turns into a miss or a missing receipt, never an empty answer. There is no `#[serde(default)]`, and the only `unwrap_or_default()` is in `unix_now`'s clock.

### Incident

A relative `cd worktree/...` once ran from the main checkout (`/Volumes/coding/personal/rusty-biscuit`) instead of this worktree. A `sed` there added `Deserializer` to that checkout's `worktree/lib/src/remote_head.rs` import line, which was why `just _lint` reported an unused import twice. The diff there was only that line, and it was reverted with `git checkout --` on that one file; the main checkout's `worktree/` is clean again. After that, every command used absolute paths, and all the results above came from this worktree.

## Phase 2

Scope: the worker, the wait, and the rendering vocabulary in `worktree-cli`. The crate could not compile without `list.rs` dropping the deleted library calls (`fetch_and_publish`, `Writer`), so the Phase 3 wiring of `list.rs` landed here too, and its Phase 3 checkboxes are ticked. Phase 3's remaining work is verification, not code.

### What changed

`cli/src/commands/refresh_worker.rs`, `args.rs`, `main.rs`

- `--force` is gone from `wt internal-refresh`, from `LaunchArgs` (now `{ attempt }`), and from `run`. The receipt is written after both halves join on **every** attempt. `receipt_target(main, id, origin)` builds the target and returns `None` only without an origin or a default branch, or for an invalid id. The old-receipt sweep runs before every write.
- `RefreshOutcome::Unsupported` maps to `PrStatus::Unsupported`. A local-path origin is no longer a PR failure in the receipt.

`cli/src/commands/list/wait.rs` (redesigned)

- `wait` returns `WaitEnd { head: HeadEnd, prs: PrEnd, timed_out }` instead of `Waited { end: Finished | TimedOut | Unavailable, worker }`. The two halves are kept apart, so a timeout keeps a finished head's outcome and a publication seen while the head still runs. The `worker` handle was dropped from the result, since the post-wait PR-lock probe it fed is gone.
- The ordinary wait now ends when the head has an outcome **and** the PR half is resolved, when our worker exited (missing receipt: `Failed(Other)`), or at the budget. The `if !request.force { return Finished }` early exit is gone. Forced versus ordinary now only picks the budget and the retries (the PR-contention relaunch and the other-branch head relaunch).
- The publication id is read once, before the first launch. A different nonempty id seen at any poll is `Published` and stays so (ruling 4), whatever the receipt or a later store read says.
- The PR lock is probed only after our receipt says `contended`.
- `StoreEnv::pr_publication` is `stored_publication` directly (id only). `refresh_publication` and the `Writer` filter are removed.
- Every launched attempt's receipt is discarded at return, in both modes (a relaunch included, never an adopted run's).
- Once the head is done and only the PR half is left, `on_phase(Phase::Checking)` is called once, so the spinner shows the generic `updating`.
- One extra rule, not in the plan: if the budget runs out while a contended half's holder has already released its lock without publishing, and the relaunch is still waiting on the head, the PR result is `Failed(Other)` rather than `Pending`. The holder has finished, so "still running" would be wrong.

`cli/src/commands/list_table.rs`

- `PrOutcome { Published, Ignored, Unsupported, Pending, Failed }` and `TableFacts { pr_outcome: Option<PrOutcome>, timed_out }` replace `unfinished`. `pr_status_markup(outcome, prs, now)` replaces `pr_age_markup` and covers the §5 rows. `TableFacts::badges()` returns no PRs for `Ignored`/`Unsupported`, for the table and the graph alike. Rendering still goes through `Prose` + `UnorderedList`, with no raw escape sequences.

`cli/src/commands/list.rs` (the Phase 3 wiring)

- `ListSeams { launch, wait_budget, forced_budget }`. `PrConnect`, `origin_pr_source`, `RemoteAnswers::pr_failure`, and `unfinished()` (with its post-wait PR-lock probe) are removed.
- `gather_remote` makes no request. After the wait it rechecks `origin_url(main)`, and `RemoteAnswers::origin_changed` drops the old badges and this run's PR diagnosis. It rereads the store on every exit path.
- `pr_outcome(&RemoteAnswers)` maps `PrEnd` (an ignored repository is always `Ignored`). `observed_pr_failure` feeds `credential_line` in every mode. `remote_status` and `followed_attempt` take `&HeadEnd`.
- Behavior change in the git-call count: a listing now makes **two** `git remote get-url origin` calls, one before the launch and the recheck after the wait. The test `every_listing_with_an_origin_launches_once_and_waits_for_both_halves` asserts exactly that.

`lib/src/remote_head.rs`: comments only. The module doc, `remove_stale_receipts`, `HeadStatus`, `PrStatus`, and `Receipt` no longer say "forced only". The Phase 1 handoff deferred this.

### Requirement-to-test mapping

| Requirement | Test(s) |
|---|---|
| The worker writes a receipt without `--force`, carrying `Ok`, `Failed`, `Ignored`, `Unsupported` | `refresh_worker::tests::every_attempt_writes_a_receipt_with_its_pr_status`, `every_attempt_has_a_receipt_target_bound_to_its_origin_and_branch`; binary: `list_remote_head::a_worker_records_the_given_attempt_and_a_receipt_for_both_halves` (real worker, no `--force`, `prs: unsupported`) |
| … carrying `Contended` | `refresh_worker::tests::a_contended_pr_half_is_in_the_receipt_and_leaves_the_head_half_publishing` |
| Panic isolation | `a_panicking_half_does_not_stop_the_other`, `a_panicking_half_is_recorded_as_failed_in_the_receipt` |
| Sweep only old receipts, never another attempt's young one or another repository's | `writing_a_receipt_sweeps_only_old_receipts` |
| A failed receipt write keeps the published answer | `a_receipt_that_cannot_be_written_leaves_the_published_answer` |
| The PR half asks on every attempt | `the_pr_half_asks_on_every_attempt_even_when_the_answer_is_fresh` |
| An ordinary wait does not end on the head alone | `wait::tests::an_ordinary_wait_does_not_end_on_a_head_outcome_alone` |
| It ends when both are present | `our_attempt_is_followed_through_every_phase_to_both_results`, `the_receipt_reports_each_pr_status` |
| Times out with the head finished and PRs running | `the_budget_ends_the_wait_with_the_head_finished_and_the_pr_half_running` |
| A publication while the head is held | `a_publication_while_the_head_is_held_is_kept_at_the_timeout` |
| A publication outranks a later failed receipt (ruling 4) | `an_observed_publication_outranks_a_later_failed_receipt` |
| Contention: wait for the lock, then the id; ordinary failure / forced single relaunch / second contention fails | `an_ordinary_contended_pr_half_waits_for_the_lock_and_fails_without_a_new_answer`, `an_ordinary_contended_pr_half_takes_the_holders_answer`, `a_forced_wait_accepts_the_answer_a_contending_holder_published`, `a_forced_wait_relaunches_once_when_a_contending_holder_published_nothing`, `a_forced_wait_reports_a_pr_failure_when_the_relaunch_is_contended_too`, plus real stores and the real `refresh`: `a_contending_holders_real_refresh_is_followed_by_its_publication_id`, `a_contending_holders_failed_real_refresh_is_never_a_publication` |
| Same-second and empty publications count; unchanged, corrupt, future-dated, wrong-origin, and older-format stores do not (control row first) | `only_a_new_usable_publication_id_counts_for_a_contended_half` (real store, one edit per cell) |
| A probe cannot contend with our just-launched worker | PR-lock probe times asserted in `an_ordinary_contended_…_fails_without_a_new_answer` and `…_relaunch_is_contended_too`; no PR probes at all without contention in `our_attempt_is_followed_…`; head `early_probes == 0` in the adoption tests |
| Retries share the budget; contention never extends it | `a_forced_relaunch_shares_the_original_budget`, `contention_never_extends_the_ordinary_budget` |
| Adoption reads our launched receipt, including a holder that finished before adoption | `a_running_attempt_is_adopted_and_our_receipt_still_read`, `a_holder_that_finished_before_adoption_is_followed_only_on_our_receipts_word`, `overlapping_forced_runs_each_read_their_own_receipt` |
| Missing or malformed receipt: bounded generic failure, head kept, new publication kept | `a_worker_that_exits_without_a_receipt_is_a_generic_pr_failure`, `a_worker_that_exits_without_a_receipt_keeps_a_new_publication`, `a_receipt_for_another_attempt_or_a_malformed_one_is_missing` (real files: wrong origin, branch, id, pre-attempt timestamp, malformed JSON) |
| Launch failure is bounded | `a_spawn_failure_is_unavailable_at_once` |
| Cleanup: only our launched receipts, in both modes | `discarded()` assertions in the adoption, relaunch, timeout, and forced tests; `a_real_receipt_is_read_and_discarded_by_an_ordinary_wait` |
| Spinner shows `updating` while only PRs are pending | phase lists in `an_ordinary_wait_does_not_end_on_a_head_outcome_alone` and `the_budget_ends_…_pr_half_running` |
| §5 table, every row (badges + status item + hint) | `tests/list_table.rs::pr_presentation_snapshot_every_row` (snapshot `list_table__pr_presentation_every_row.snap`) |
| Age boundaries for pending and failed, empty answer, nothing stored | `the_pr_age_line_appears_once_a_pending_refreshs_badges_are_60_seconds_old`, `a_failed_refresh_dates_the_stored_answer_at_any_age`, `a_stored_empty_answer_shows_no_badges_but_keeps_its_age`, `a_published_answer_has_no_status_item_at_any_age` |
| Ignored/unsupported suppress seeded badges (table and graph) | `an_ignored_or_unsupported_repository_shows_no_badges_even_when_an_answer_is_stored` |
| Dim, PR item before the hint; hint only on timeout | `the_pr_status_item_is_dim_and_precedes_the_hint`, `the_hint_appears_only_when_the_wait_timed_out`, `output_order_snapshot` |
| Post-wait reread, empty answer clears, failure keeps stored (answer / empty / none), origin changed during the wait | `list::tests::gather::*`: `a_pr_answer_the_worker_publishes_during_the_wait_is_shown`, `an_empty_answer_published_during_the_wait_clears_the_badges`, `a_failed_refresh_keeps_the_stored_answer_and_is_this_runs_failure`, `an_origin_changed_during_the_wait_shows_neither_its_badges_nor_its_failure` |

Binary tests rewritten because they asserted the removed behavior (minimal, so `just test` stays green; Phase 4 still adds its own cases):

- `list_prs`: `a_fresh_pr_store_makes_no_request_and_shows_its_badges` became `a_fresh_pr_store_is_asked_again_and_a_failed_request_keeps_its_badges`.
- `list_prs`: `concurrent_lists_and_workers_make_one_request_and_a_fresh_answer_stops_the_next` became `…_and_the_next_worker_asks_again`.
- `list_prs`: `a_detached_workers_answer_replaces_the_stale_one_on_the_next_list` now asserts the pending item plus the hint, and two requests.
- `list_prs`: `an_origin_change_during_a_foreground_request_…` became `an_origin_change_during_the_wait_shows_no_badges_from_the_old_origin`.
- `list_flags`: `refresh_waits_for_both_halves_and_asks_again_despite_young_answers` became `…_like_every_listing`.
- `list_remote_head`: `a_forced_worker_records_…` became `a_worker_records_…`.

### Gates run (macOS)

- `just test` (worktree area, `--no-fail-fast`): **823 run, 823 passed, 30 skipped**.
- `just lint`: clean. Two `type_complexity` hits in new tests were fixed with type aliases.
- Every target compiles with `--features terminal-tests --all-targets`.
- `just check-tier-coverage worktree`: 0 stranded. All new tests are L1 unit tests (lib + bin targets) or in the existing `list_table` binary. Two helper modules first named `real_store`/`real_receipts` were renamed `stored_prs`/`receipt_files`: they hold no tests, but a `real_` segment would strand any test added to them later.
- Linux: `just cross-check worktree-cli --os linux`: **518 passed, 60 skipped**. The first attempt failed when build-linux's linker was killed (signal 9, out of memory, a host resource problem); the retry passed.
- Windows/WSL2: not run. No path, `cfg`, or process code changed. The new tests use `File::set_modified` with a write handle and a receipt path under a regular file, both portable. CI covers them after merge.

### Known failures left for Phase 4 (pre-planned, caused by the intended behavior change)

- `just test-l2` (tmux, `BISCUIT_TEST_REQUIRED_BACKENDS=tmux`): 28 run, 25 passed, **3 failed**, all in `level2_list_verbose`:
    - `level2_list_credentials_warning_is_a_dim_line_beneath_the_caption_in_tmux` and `level2_list_clears_the_spinner_before_the_caption_and_shows_a_dim_hint_in_tmux` assert "a fresh PR answer makes no PR request" (`requests == 0`); it is now 1.
    - `level2_list_styles_follow_the_design_in_tmux` asserts "a fresh answer has no age line". Its PR request now goes to the refused proxy and fails, so the pane shows `(couldn't refresh)`.
- `just test-perf`: 30 run, 28 passed, **2 failed**: `perf_list_meets_sla_when_the_pr_request_hits_its_deadline` (Phase 4 removes it) and `perf_list_meets_sla_with_a_stale_answer_and_a_failing_refresh` (Phase 4 updates it).


## Phase 3

Scope: the listing flow in `cli/src/commands/list.rs`. Phase 2 had already landed the code, because the crate could not compile without it, and had ticked the boxes. This phase checked each Phase 3 requirement against the code, added tests for the two behaviors nothing proved, and ran the checkpoint gates. No production code changed.

### Verification against the plan

| Phase 3 requirement | Where it holds | Proof |
|---|---|---|
| `gather_remote` makes no request; `ListSeams { launch, wait_budget, forced_budget }` only | `list.rs::gather_remote`, `ListSeams` | `gather::every_listing_with_an_origin_launches_once_and_waits_for_both_halves` (the git-call recorder sees only the two `remote get-url origin` lookups, no `ls-remote` or fetch) |
| `connect`, `origin_pr_source`, `RemoteAnswers::pr_failure` deleted | — | checkpoint grep (below) |
| One worker whenever there is an `origin`, including an ignored repository (the worker decides); no origin means no worker, PR item, or hint | `gather_remote` launches before reading `ignored`; `pr_outcome` is `None` without `waited`; `timed_out` is set only when there was a wait | `every_listing_with_an_origin_…`, **new** `an_ignored_or_unsupported_pr_half_is_this_runs_outcome_and_no_failure`, `without_an_origin_stored_answers_are_ignored_and_nothing_is_launched` |
| Reread the store on every exit path; an empty answer clears the badges and counts as success | post-wait `select_cached` | `a_pr_answer_the_worker_publishes_during_the_wait_is_shown`, `an_empty_answer_published_during_the_wait_clears_the_badges`, `a_silent_worker_is_waited_for_only_until_the_budget` (timeout), `a_worker_that_cannot_launch_is_unavailable_without_a_wait` (worker failure) |
| Recheck `origin`; a changed one drops the old badges and this run's PR diagnosis, with no second refresh | `RemoteAnswers::origin_changed`, `pr_outcome`, `observed_pr_failure` | `an_origin_changed_during_the_wait_shows_neither_its_badges_nor_its_failure`, `a_changed_origin_never_shows_the_old_answers` |
| Map the wait to `PrOutcome` (§5, rulings 3 and 4); a PR result still unobservable at the timeout shows as pending | `pr_outcome` | gather tests above, plus `a_silent_worker_…` (`Pending`); ruling 4 in `wait::tests::an_observed_publication_outranks_a_later_failed_receipt` |
| The credentials line reads the receipt's PR failure in every mode | `observed_pr_failure` → `credential_line` | `a_failed_refresh_keeps_the_stored_answer_and_is_this_runs_failure` (ordinary `ListFlags`), `a_pr_failure_this_run_observed_gives_a_line_only_for_a_confirmed_condition` |
| A confirmed head condition beats a PR condition; an ambiguous condition never asserts a bad key | `credential_line` (`from_note.or_else(..)`) | **new** `observations::a_confirmed_head_condition_outranks_this_runs_pr_failure` |
| The spinner is cleared on every exit path; `updating` while only PRs are pending | `Progress::finish` right after `wait::wait`, which always returns, before any render | `wait::tests` phase lists (Phase 2); binary `assert_no_spinner` in `list_flags` |
| `--ff`/`-r` keep the 75 s budget; a PR failure never blocks a fast-forward | `forced_budget`; `fast_forward_default` runs after the wait and reads nothing from `waited.prs` | budget: `a_silent_worker_…` (forced row); `list_flags::fast_forward_*` run with the PR half `Unsupported` (local origin). Binary proof with a **failing** PR half: Phase 4 (`list_prs`, as planned) |
| Comment and doc pass for `list.rs`, `wait.rs`, `refresh_worker.rs` | reread: none claims that fresh answers skip requests, that PR requests never delay the listing, or that only forced attempts write receipts | grep for `fresh\|skip\|forced only\|--force\|never delay` gives only accurate hits |

### Tests added (`cli/src/commands/list/tests.rs`, L1, both lib and bin targets)

- `gather::an_ignored_or_unsupported_pr_half_is_this_runs_outcome_and_no_failure`: a receipt saying `ignored` or `unsupported` maps to `PrOutcome::Ignored` / `Unsupported`, launches exactly once, is not a PR failure (so no credentials line), and leaves the head answer and `timed_out == false` intact.
- `observations::a_confirmed_head_condition_outranks_this_runs_pr_failure`: with a PR rate limit observed, a confirmed head `Rejected` or `NotVisible` note is the one line. With no head note, or an ambiguous one (a 404 with a key), the PR failure speaks. Fail-before: putting the PR condition first in `credential_line` fails this test in both targets. `list.rs` was then restored byte for byte (`cmp`).

### Gates run (macOS)

- `just test`: **827 run, 827 passed, 30 skipped** (823 before this phase, plus 2 tests × 2 targets).
- `just lint`: clean (after removing two `clone_on_copy` hits in the new test).
- `just check-tier-coverage worktree`: 0 stranded. The new tests are in the existing `list::tests` unit modules (`gather`, `observations`), with no tier marker.
- Checkpoint grep `fetch_and_publish|LIST_DEADLINE|AlreadyFresh|SkippedFresh|origin_pr_source|Writer|PrConnect` over `worktree/` and the skill: hits only in `fixes/_completed/*` history and this feature's spec, plan, and log.
- Not run in this phase: `just test-l2` (3 known failures) and `just test-perf` (2 known failures). Both are unchanged from the Phase 2 list and are Phase 4's to fix. No cross-OS run: no path, `cfg`, or process code changed, and the new tests are pure or use the existing temp-repo fixture.
- Skill: `.claude/skills/worktree/SKILL.md` already describes the Phase 3 flow (Phase 2 updated it), so it was not changed.
