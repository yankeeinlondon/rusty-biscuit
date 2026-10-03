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
packages:
    - worktree
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
