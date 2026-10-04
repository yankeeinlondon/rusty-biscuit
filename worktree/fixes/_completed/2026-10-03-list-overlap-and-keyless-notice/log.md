---
deferred_perf_measurement: false
implementation_2: "2026-10-03T20:24:30-07:00"
---

# Log

## Implementation of Review Findings #2

> **started at:** 2026-10-03T20:24:30-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'worktree/fixes/2026-10-03-list-overlap-and-keyless-notice/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- starting the work on 'Changed origins still show the old head request's warning and fallback notice' at 20:28:13
        - confirmed in `run_pipeline`: the head attempt went to `credential_line` and `fallback_notice` with no `origin_changed` check; `pr_outcome`, `observed_pr_failure`, and `observed_keyless` each carried their own copy of the check
        - found a third unguarded projection in the same function: the caption's head-status suffix (`remote_status` over the followed attempt) still reported the old origin's check (for example `updated from origin just now`), and its "last checked" age read the stored answer bound to the old origin
        - added one identity guard, `RemoteAnswers::observed() -> Option<Observed>` (`None` once `origin` was replaced or removed); `pr_outcome`, `observed_pr_failure`, and `observed_keyless` now take `&Observed`, so no caller can reach request evidence around it
        - `request_notices` builds the PR item, credentials line (head and PR warnings, keyless notice), and closing fallback notice behind that guard; `run_pipeline` reads only its result; warning precedence and fallback coexistence are unchanged for an unchanged origin
        - `caption_status` routes the caption's head status through the same guard: a changed origin reads `couldn't check origin` (the row the worker gives when it notices the change itself), dated only from the tracking ref's reflog instead of the old origin's stored answer
        - unit test `commands::list::tests::gather::a_replaced_or_removed_origin_suppresses_every_request_notice`: 11 projection shapes (caption head status; head warning for rejected, insufficient, keyed and anonymous rate limit, not visible; PR warning; PR item and badges; keyless head; keyless PR; fallback notice) x {unchanged control, replaced, removed}; the stub worker gained `OriginChange::{Replaced, Removed}`, head outcome/API note/credentials, and PR credentials
        - shipped-binary tests in `cli/tests/list_prs.rs`: `a_changed_origin_drops_the_old_head_checks_credentials_warning` and `a_changed_origin_drops_the_old_head_checks_fallback_notice_and_caption`; the PR reply is held (`FakeGitea::before_reply`) until the listing's own head attempt ID has an outcome, bounded by the listing's 3 s wait, then `origin` is changed; each runs an unchanged-origin control through the same checkpoint and asserts the checkpoint was reached and the wait did not time out
        - confirmed both new test sets fail when the guard is disabled (head warning shown after replacement; fallback notice and caption failed too), then restored the source
        - docs: `worktree/docs/cli/list.md` changed-origin paragraph now lists every suppressed notice and the caption row (plus a caption table row and notes in the credentials-warning and fallback-notice sections); `.claude/skills/worktree/list.md` and `list-remote.md` describe the single guard; `worktree/README.md` has no changed-origin paragraph, so no drift; `list_prs.rs` module doc and the `origin_changed` field doc were updated
- class sweep: request-notice projections apply repository-identity validation inconsistently, so evidence about a superseded remote is presented as describing the current repository; sites checked: head credentials warning (`credential_line` head arm), PR credentials warning, keyless notice (`observed_keyless`), closing fallback notice (`fallback_notice`), PR status item (`pr_outcome`), PR badges (`RemoteAnswers::prs` / `TableFacts::badges` / graph badges), caption head-status suffix and its last-known date (`remote_status` / `select_cached_head`), `--ff` suggestion, refresh hint (`timed_out`), wait spinner phase text (`wait::Progress`), `--perf` rows; fixed here: head credentials warning, closing fallback notice, caption head-status suffix and last-known date, plus PR warning / keyless / PR item moved behind the shared guard; clean: PR badges (already rereads only for an unchanged origin), `--ff` suggestion (derives from the guarded caption status and the local tracking ref), refresh hint (says only to run again, true for any origin), spinner (cleared before render, shown before the post-wait origin recheck can know), `--perf` (timings only, no notices)
        - `just test`: 958 passed, 32 skipped (a first run under load average 26 failed two held-worker timing tests, `a_held_live_head_check_holds_the_listing_only_until_its_deadline` and `a_failed_assertion_while_upload_pack_is_held_still_reaps_the_worker_before_the_fixture_goes`; both passed in isolation and the full rerun was clean)
        - `just lint`: passed; `just test-l2 credential`: 1 passed; `just test-l2 keyless`: 1 passed; `just check-tier-coverage worktree`: no stranded tests
        - files changed: `worktree/cli/src/commands/list.rs`, `worktree/cli/src/commands/list/tests.rs`, `worktree/cli/tests/list_prs.rs`, `worktree/docs/cli/list.md`, `.claude/skills/worktree/list.md`, `.claude/skills/worktree/list-remote.md`
- work completed for 'Changed origins still show the old head request's warning and fallback notice' at 20:36:13

### Successful Completion

The implementation of review cycle 2 has completed successfully in 12 minutes. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- no findings were deferred
- the class sweep found one sibling site beyond the review: the caption's head-status suffix and its "last checked" date also described the old origin; it now goes through the same identity guard and reads `couldn't check origin` after a changed or removed origin (a new caption behavior for the author to confirm)

The files changed in this cycle are `worktree/cli/src/commands/list.rs`, `worktree/cli/src/commands/list/tests.rs`, `worktree/cli/tests/list_prs.rs`, `worktree/docs/cli/list.md`, `.claude/skills/worktree/list.md`, and `.claude/skills/worktree/list-remote.md`.
