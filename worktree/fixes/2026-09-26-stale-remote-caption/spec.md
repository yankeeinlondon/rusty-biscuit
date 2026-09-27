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
reviewed_by: codex/default
reviewed_on: 2026-09-26
review_iterations: 0
clarified: false
implemented: false
related:
    - 2026-09-25-list-remove-performance
    - 2026-09-24-ux-improvements
human_review: false
message_to_agent: |-
    Phase 1 (spikes and baseline) found no conflict with plan Rules 1-14. Details are in
    implementation-log.md under "## Phase 1". Points that affect later phases:
    - Baseline on macOS: `just test` 466 passed / 18 skipped, `just lint` clean. Any new failure is yours.
    - S1: a raw std::net::TcpListener is enough for both fixtures. The hold listener sees one
      connection, and at a 500 ms deadline the process-group kill returns in about 0.50 s and the
      listener sees EOF immediately. Killing only the git pid keeps the connection open, so keep
      kill_tree and assert the listener observes the close. The 401 responder bytes are in the log.
      Git exits 128 in about 25 ms with no prompt.
    - S3: `ls-remote <remote> refs/heads/<b>` exits 0 with empty stdout for a missing branch (local
      and HTTP), and does not match refs/heads/foo/<b>. A bare `<b>` pattern DOES tail-match, so
      always pass the full refname and keep the exact-name comparison.
    - S2: git ls-remote honors HTTPS_PROXY/HTTP_PROXY, so ProxyStub and FakeGitea (which counts
      every request regardless of path, and holds them all when held) will receive the live-head
      worker's request. Every test that counts connections/requests, or asserts "no launch", must
      seed a fresh remote-head store (Rule 12). The per-test table is in the log.
    - Repo-root docs/dependencies.md line 212 names `wt internal-refresh-prs`. Phase 5's drift grep
      (`worktree .claude`) would miss it, so update it too.
---

# `wt list` caption trusts a stale `origin/<default>`

## Problem

`wt list`'s caption compares the local default branch with the remote-tracking ref `origin/<default>` as of the last fetch. It never asks the remote, and it does not say how old its information is. Observed on 2026-09-26 in this repository:

| Ref | SHA |
|---|---|
| local `main` | `1314bd63f` |
| local `origin/main` (last fetch 14:19) | `1314bd63f` |
| live `refs/heads/main` on `origin` (`git ls-remote`) | `69b207ee3` |

The caption read "main is in sync with origin/main" while `origin` had moved on. The caption is the only place where `wt list` talks about the remote, so users read it as "the remote", not "my last fetch".

A live check cannot run in the foreground. `git ls-remote origin refs/heads/main` over SSH to GitHub took about 1.0 s in each of three runs. That is more than three times the 300 ms the PR lookup is allowed in the foreground, The existing PR miss path can wait up to its 300 ms deadline; the new live-head check must add no foreground network wait.

## Fix

Check the remote default branch in a detached background worker and retain its last successful answer. Always describe the ahead/behind comparison as being against the **local tracking ref**, and qualify the remote observation with its age. An unavailable or old answer must never become an unqualified claim about the current remote.

> Reader’s note: this review replaces the proposed `FETCH_HEAD` age with the age of the successful remote check. Fetch can target another remote or branch, append results, or suppress writing `FETCH_HEAD`; its modification time cannot establish when `origin/<default>` was updated. See [Git’s fetch options](https://git-scm.com/docs/git-fetch). Also, Git keeps most pseudo refs separately for each worktree, so assuming one common-directory file loses linked-worktree activity; see [Git’s worktree refs](https://git-scm.com/docs/git-worktree#_refs). Explicitly labeling local data is reliable even before the first successful check and requires no inferred fetch timestamp.

### 1. Live-head store

- Add `<repo hash>.remote-head.json` beside the existing PR store, keyed by the main checkout, with a format version and `{ origin_digest, branch, sha, checked_at }`.
- Reuse the `worktree` package’s [origin_digest](../../lib/src/pull_requests.rs), which hashes the exact fetch URL returned for `origin` in the main checkout using `biscuit-hash`. Never persist or display that URL. `branch` is the default branch name, without `refs/heads/`; `sha` is its full object ID, or `null` for a verified absence.
- `checked_at` is Unix seconds captured immediately before the request, so a slow request does not make old information appear newer. A usable answer must match the current origin digest and default branch and must not be dated in the future. Missing, unreadable, corrupt, unsupported-format, or invalid-field files are misses. Validate object IDs for Git’s supported object formats rather than assuming a short SHA or only SHA-1.
- A usable answer is fresh for less than 60 seconds and stale at 60 seconds or later. Stale answers remain available, but their displayed age is mandatory. An absent branch is a successful answer, not a miss.
- Request failures, deadline expiry, invalid output, or publication failure preserve the previous store. Only a successful, complete response without the exact requested ref proves absence. Do not treat a failed command or incomplete output as a deleted branch.
- Publish atomically while holding the store lock. Re-read both origin and the default branch before publication; discard the result if either changed. Readers need no lock and must never see a partial document.
- Compare the cached remote SHA against the tracking tip collected for **this listing**, never against a tracking tip saved with the remote answer. A later fetch can make them match without another remote request. A mismatch alone does not prove which observation is newer or that the remote advanced; it may have rewound, been recreated, or been checked before the latest fetch.

### 2. Background refresh

The `worktree-cli` package’s [PR worker](../../cli/src/commands/pr_refresh.rs) becomes the shared hidden command `wt internal-refresh <main checkout>`.

- Preserve its main-checkout validation, working directory, detached-process configuration through `sniff`, null standard streams, removed wrapper/completion environment variables, silent failures, and lack of a foreground join. A spawn or cache error must not fail listing or affect shell-wrapper output.
- Run the PR and live-head refreshes concurrently inside that worker, each with its own deadline, nonblocking lock, freshness recheck, and publication. Separate locks alone do not prevent a sequential PR request from delaying the live check. Each half can complete and publish while the other is blocked, contended, unsupported, or failing.
- The new persistent lock is `<repo hash>.remote-head.lock`, using the same `fs4` mechanism as the PR store. Hold it from the freshness recheck through publication; never unlink it. Contention makes no request. After a successful publication, another worker must observe freshness and skip its request. Failures remain retryable on a later listing; there is no new retry loop or timer.
- Centralize the launch decision in listing so it spawns at most one worker per invocation. Launch when PR data is stale, or when an eligible live-head answer is stale or missing. Eligibility requires a configured origin and resolved default branch; missing origin/default must not repeatedly launch a worker solely for a live-head miss. A tracking ref need not exist for a remote check to be useful.
- A PR miss retains the existing foreground request with a 300 ms deadline. Settle that request before launching the combined worker so a live-head miss does not cause simultaneous foreground and background PR requests. The worker’s PR half rechecks freshness; a failed foreground request may be retried there under the existing refresh rules. Fresh answers in either store skip that half’s request.
- The live request is `git ls-remote origin refs/heads/<default>` from the main checkout, with arguments passed separately, a 10 s deadline, credentials disabled, SSH in batch mode, and process-tree termination at expiry. Use the `worktree` package’s existing [LsRemote and run_noninteractive](../../lib/src/remove/live_remote.rs) transport, moving the shared module to `worktree::live_remote` and updating removal callers. Preserve removal’s existing deadlines and behavior. Audit complete-output handling so a pipe-read failure cannot become a stored absence.
- Address `origin` by name because the displayed tracking ref comes from its fetch destination, not its push destination. PR provider support is irrelevant to the live check: local bare repositories and providers without PR support must work too.

### 3. Caption

In the `worktree` package, [Caption](../../lib/src/listing.rs) represents the local default/tracking comparison. Add its tracking tip SHA, taken from the same ref snapshot that produced its counts. Do not add live network access to the library listing path.

In `worktree-cli`, [TableFacts and caption rendering](../../cli/src/commands/list_table.rs) receive separate remote-observation facts (default branch, optional tracking tip, cached answer). This lets the renderer show a remote status even when the library cannot produce a comparison caption. Rendering remains pure over these facts and an explicit current time; age and future-date validity are evaluated at render time.

The comparison sentence becomes, for example, “main is in sync with local tracking ref origin/main.” Apply the same qualification to ahead, behind, and diverged states. Then append the observation below, substituting actual branch names:

| Remote observation | Additional caption text |
|---|---|
| Usable SHA equals the local tracking tip | “origin/main matched the remote when checked 2 min ago.” |
| Usable SHA differs from the local tracking tip | “origin/main differs from the remote head observed 2 min ago; run git fetch origin to update local tracking refs.” |
| Usable answer reports absence | “main was absent on origin when checked 2 min ago.” |
| Miss or unusable answer | “Remote state has not been verified.” |
| Usable SHA, but no local tracking tip | “No local tracking ref origin/main; the remote branch was present when checked 2 min ago.” |

- Use the same age units as the existing `worktree-cli` [PR age renderer](../../cli/src/commands/list_table.rs): minutes below one hour, hours below two days, then days. For checks younger than one minute use “less than 1 min ago.” Never imply that a background refresh has succeeded before its answer is read on a later invocation.
- Stale answers use the same past-tense wording and age. Failed refreshes may leave an arbitrarily old answer, which must remain visibly old. A stale absent answer must not assert that the branch is still deleted.
- If origin or the default branch cannot be resolved, omit the remote-status clause and do not infer deletion. With no origin, do not display a remote comparison from leftover tracking refs. If origin/default exist but the local default or comparison is unavailable, render only the applicable observation; with no tracking tip and no usable answer say “No local tracking ref origin/main; remote state has not been verified.”
- The mismatch message replaces the draft’s “has moved since the last fetch”: a cached answer may predate a newer fetch, so only a difference is established. No ahead/behind counts are computed against the observed remote SHA, whose commits may not exist locally.
- Use the existing biscuit-terminal `Prose` and `TerminalRenderable` rendering, branch badges, and escaping. Preserve stderr output, color behavior, and wrapping. Do not print raw remote URLs or new protocol lines.

### Out of scope

- No fetch, ref update, remote configuration change, or automatic pruning as a side effect of listing.
- Comparison columns, target selection, graph, and removal safety continue to use their existing inputs. The caption explains the limits of local tracking data; this store is never evidence for deleting a branch.
- No provider-specific remote check, fetch-time inference, offline option, or automatic retry loop is introduced.

## Acceptance criteria and testing

Use the repository’s `rust-testing` skill and Test Toolkit. The checks below are L1: they need isolated files, local Git processes, injected sources, and loopback servers, not real terminal windows or external services. Preserve support for macOS, Linux, native Windows, and WSL2. Run `just test` in the worktree area for implementation validation; no new CI environments or gates are required for this fix.

1. **Store and observation selection (library):** cover fresh/stale boundaries at 59 and 60 seconds, missing/changed origin, changed default branch, future timestamps, corrupt/old-format/invalid-field files, verified absence, and read/write failures. Use an injected clock. An origin or default-branch change during a blocked request discards its answer. Failures preserve previous bytes. Successful competing refreshes make one request; contention makes none. Synchronize through explicit readiness signals with bounded waits, not fixed sleeps.
2. **Detection (real Git):** use an isolated local bare origin and a second clone that pushes changes. After the second clone advances the default branch, refresh and verify the mismatch text. Fetch in the listed checkout and verify the next listing’s behind count and matching observation without another live check. Also cover remote deletion, deletion followed by local pruning, recreation, and a fetch newer than the cached remote observation; the last case must say “differs,” not claim that the remote advanced. Verify both main and linked checkout invocations use the same store.
3. **Caption snapshots:** extend `cli/tests/list_table.rs`, where snapshots compile only once. Cover each observation row, all four comparison states, fresh and stale matching/differing/absent answers, missing local default/tracking refs, missing origin, failed comparison, non-`main` default names, and minute/hour/day boundaries. No missing `FETCH_HEAD` case is required because it is no longer read.
4. **Worker orchestration:** inject request and launch functions. Prove one launch when both stores need refresh, zero live request in the foreground, no launch for two fresh answers or an ineligible live miss alone, foreground PR behavior unchanged, and no simultaneous foreground/worker PR miss request. Block each worker half in turn and prove the other can publish. PR failure, lock contention, and unsupported provider must not suppress the live half; live failure must not suppress PR refresh.
5. **No foreground wait:** extend the existing `list_prs` captured-output test to return while the live request remains held. Seed fresh PR data to isolate the live miss/stale paths, count requests, and use an injected transport or local Git HTTP fixture that actually reaches the holding server. Do not assume the PR client’s proxy fixture automatically intercepts Git’s HTTP transport. Preserve the existing full-command performance bound and add a stage measurement below 300 ms for cached remote selection and launch; retain the existing stale-PR performance proof.
6. **Failure and cleanup:** controlled missing-credential and deadline tests prove no prompt, no stored absence on failure, and process-tree termination. Prove truncated/malformed output cannot become an absent-branch answer. Detached workers and both request halves must be finished or terminated before fixtures are removed, including assertion-failure paths; waiting for the PR lock alone no longer proves the worker is done. Fixtures isolate configuration/cache and disable unrelated service connections.
7. **Command and removal regression:** `internal-refresh` remains hidden from help and completions and ignores invalid/non-main paths. Update references to the old name in active code, tests, documentation, and skills, without rewriting historical specs. Run existing removal transport and safety tests after moving the shared module; keep deletion deadlines and safety rules unchanged.

## Packages and documentation

- `worktree`: new live-head store/refresh module beside [pull_requests](../../lib/src/pull_requests.rs), shared transport moved from removal, and tracking SHA added to the local comparison facts. Update affected field comments to say that counts describe local tracking refs, not the live remote. No `FETCH_HEAD` helper is needed.
- `worktree-cli`: [list orchestration](../../cli/src/commands/list.rs) selects both answers and makes one launch decision; the renamed worker runs independent concurrent refreshes; the table renders qualified observations.
- Update the worktree skill’s list section, `worktree/docs/performance-testing.md`, and the `wt list` README description with the new store, worker, and caption behavior. Reuse existing dependencies; update dependency documentation if implementation proves an additional crate necessary.

## Decisions

These are draft design decisions, not implementation approval or a claim that the fix is implemented.

1. Remote-head requests run only in the background; the measured one-second request cannot fit the foreground budget.
2. Listing never fetches. This preserves read-only Git behavior and avoids competing with a user’s fetch for ref locks.
3. One worker runs two concurrent refreshes with separate stores and locks. Either source can publish without waiting for the other.
4. Report observation age and explicitly label local tracking data. `FETCH_HEAD` timestamps cannot establish branch-specific freshness, and hiding stale-check age would reproduce the original misleading claim.
5. Compare the observed SHA with this listing’s tracking tip. Describe inequality without assuming chronology or ancestry, so a newer fetch cannot turn an older remote answer into a false claim that the remote just moved.
6. Keep missing-origin and missing-ref states distinct from verified remote absence. Continue to show an observation after pruning removes the local tracking ref.

## Open Questions

None blocking this draft. The review resolves the gaps using the existing cache, detached-worker, rendering, and test conventions; implementation review must verify the behavior and performance criteria above.
