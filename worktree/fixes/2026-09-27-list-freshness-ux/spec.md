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
status: finalized-spec
reviewed: true
reviewed_by: codex/gpt-6-sol
reviewed_on: 2026-09-27
review_iterations: 0
clarified: true
implemented: false
related:
    - 2026-09-26-stale-remote-caption
human_review: false
message_to_agent: |-
    Phase 3 is done: every Phase 3 task is checked off; `just test` (590 passed) and `just lint` pass in
    worktree/, and `just test-l2` passes on macOS (20). Read "## Phase 3" in implementation-log.md for the
    exact APIs and deviations. What Phase 4 needs:

    - The worker is `wt internal-refresh <main> [--attempt <id>] [--force]`. `refresh_worker::launch(main)`
      still passes neither; Phase 4 replaces it with the Rule 18 `LaunchArgs` launcher. `--attempt` must be
      32 lowercase hex (`remote_head::new_attempt_id()`); an invalid id makes `begin_attempt` fail, so the
      head half ends `WriteFailed` and writes nothing.
    - `remote_update::run_attempt` has NO `force` parameter (Rule 7 gives the head half no freshness skip).
      `--force` only makes the PR half ignore its 60 s window and makes the worker write the receipt.
    - `remote_head::refresh_remote_head` is GONE. `pull_requests::RefreshOutcome` is PR-only,
      `Failed(PrFailure)`, and not `Copy`; `pull_requests::refresh` takes `force: bool` before `connect`.
    - Receipt `prs` has a new `PrStatus::Ignored` (repository in ~/.wt.json: no PR request, no badges).
      `api_preference::Preferences::ignores_origin(origin)` is the check.
    - `remote_head::refresh_lock_held(store)` exists for the Rule 6 "contended" test, BUT it takes the
      lock for an instant: a worker that tries to lock at that moment exits as `Contended` and writes
      nothing. In the foreground wait, never probe before the launched worker's own attempt record
      (`attempt.id == token`) has appeared or its `Child` has exited; otherwise the probe can make your own
      worker lose its lock. Prefer reading the attempt record; probe only to decide adoption.
    - `live_remote::tracking_ref_changed_at(base, branch)` is ready for the caption's reflog row.
    - Tests already migrated (so skip them in Wave 3): `list_prs` (all), `list_remote_head` (all),
      `level2_list_verbose::level2_list_stale_pr_answer…`, and S4's `DesignFixture` git-config isolation.
      `FakeGitea` now answers `/branches/` 404 at once and counts it in `branch_requests()`, so
      `requests()` is PR-only. Through `ProxyStub` (HTTPS) each worker is 2 connections (one per half).
      Still unmigrated: every test whose expectation depends on `wt list` launching or waiting (the launch
      rule itself is unchanged in Phase 3), and perf_* bounds.
    - Cross-OS: the new lib tests pass on native Windows (99/99 of the touched modules); see the log.
---

# `wt list` should know, not guess, whether `origin/<default>` is current

## Problem

`2026-09-26-stale-remote-caption` made the caption honest, but the result is still not useful:

```text
main is in sync with local tracking ref origin/main.  origin/main matched the remote when checked 1 min ago.
```

1. **It reads as two sentences about the same thing.** The first compares `main` with the local `origin/main`, and the second compares that ref with the remote. When they agree, the local-versus-remote distinction is bookkeeping, not information.
2. **It reports numbers it knows are wrong.** When the background check finds that the remote differs, the caption says so and asks the user to fetch. But the check can predate a later fetch, so even "differs" does not say which side is newer.
3. **The check is slow because of how it is made, so it runs in the background.** `git ls-remote` over SSH took 0.92–1.02 s on 2026-09-27 (GitHub, from macOS), and 0.67–0.69 s of that was the SSH handshake. A single provider REST request for the same ref took 0.05–0.15 s. Because the check runs in the background, a listing shows an answer from an earlier run, never a current one.
4. **Nothing ever brings `origin/<default>` up to date.** A `git fetch origin main` took 0.96–1.00 s over SSH with nothing to download. Updates are expected to take 1–10 s, mostly near 1 s. Until the user fetches by hand, the merge metrics stay wrong.

> Reader's note: the earlier draft put a default-route and TCP probe in front of every update. That probe could incorrectly reject a local bare repository, a LAN remote with no default route, an SSH alias, or a server whose API port differs from its Git port. This review removes the probe. The bounded API or Git request is the reachability test; a failure leaves the last successful observation intact.

## Fix

### 1. Provider branch-head endpoint (schematic, sniff)

Add one endpoint per provider to `schematic-definitions`, then regenerate `schematic-schema`:

| Provider | Endpoint | Response field |
|---|---|---|
| GitHub | `GetBranchReference`: `GET /repos/{owner}/{repo}/git/ref/heads/{branch}`, the heads counterpart of `GetTagReference` | `object.sha` |
| Gitea | `GetBranch`: `GET /repos/{owner}/{repo}/branches/{branch}` | `commit.id` |
| GitLab | `GetBranch`: `GET /projects/{id}/repository/branches/{branch}` | `commit.id` |
| Bitbucket | `GetBranch`: `GET /repositories/{workspace}/{repo_slug}/refs/branches/{name}` | `target.hash` |

Authentication uses each definition's existing `env_auth` / `env_mapping`: `GITHUB_TOKEN` or `GH_TOKEN`; `GITLAB_TOKEN` or `GITLAB_PRIVATE_TOKEN`; `GITEA_TOKEN`; `BITBUCKET_USERNAME` with `BITBUCKET_APP_PASSWORD`. No new variables are introduced.

In `sniff::remote::blocking`, add `branch_head(remote_url, branch, deadline) -> Result<BranchHead, PrUnavailable>` beside `open_pull_requests`, built the same way (`client_for_url`, `run_with_deadline`, the same unauthenticated fallback, the same error classification):

- `BranchHead` carries the SHA (validated as 40 or 64 lowercase hex). Failures use the functional credentials variants in §5.
- A 404 is never an absence. Providers can answer a private repository this way, so only a successful, complete `ls-remote` response without the requested ref can prove absence (§3).
- Add `credential_env(remote_url) -> Option<CredentialEnv>`, which returns the provider's display name and the variable names from its definition, so `worktree` never hard-codes them.
- Encode branch names as endpoint path parameters, including `/`, spaces, and Unicode; do not let a branch name change the repository path or query. Use the parsed *fetch* identity of `origin` for the API request and revalidate it before publication, because a separate push URL may name another repository.

### 2. Reachability and failure

Do not run a separate connectivity probe. A route or TCP connection does not establish that the configured Git remote or its provider API will answer, and may reject a working local or SSH remote. Run the requested operation with its own deadline and classify its result. A network, DNS, authentication, spawn, or timeout failure does not prove branch absence and never clears the previous successful answer. Show a specific reason only when the operation establishes one; otherwise say that `origin` could not be checked.

### 3. The update flow (`wt list`, every run)

Every `wt list` with an `origin` and a resolvable default branch starts this before rendering. The check and the fetch run in the detached `wt internal-refresh` worker. The foreground waits for up to **3 s** for its result, then renders using the best available facts. This intentionally changes the earlier 1 s full-command performance contract; the 3 s wait is the new cap for remote work, and local listing and rendering still have their own measured cost.

1. **Launch the worker.** The existing remote-head lock prevents duplicate checks and fetches. If a worker already holds it for this repository, adopt its current attempt and wait on that one. A contender that exits before acquiring the lock must never be mistaken for a completed check. The worker:
   - **Checks for variance.** For a provider sniff supports it calls `branch_head` (§1). For any other remote (including a local bare repository or SSH alias), or when the provider returns 404, a credentials failure, or a rate limit, it runs `git ls-remote origin refs/heads/<default>`. Only a successful, complete Git response without the exact ref proves absence. Bound the entire check, including fallback, to 10 s. Other provider errors may also fall back within that same budget; none may become absence without the Git proof.
   - **Publishes the successful check before fetching.** Keep the observed SHA (including verified absence) and check time even if a later fetch fails. A successful branch-head answer is evidence about the remote; a fetch failure cannot erase it. The caption compares that observation with the current tracking tip and does not claim which is newer.
   - **Fetches on variance.** It runs `git -c maintenance.auto=false -c gc.auto=0 fetch --no-write-fetch-head --no-tags origin +refs/heads/<default>:refs/remotes/origin/<default>` through `worktree::live_remote::run_noninteractive` (no credential prompts, SSH batch mode, process-tree kill), with a 60 s deadline. Pass the refspec as one argument and validate the branch as a Git branch name before constructing it. The explicit destination ensures only the chosen tracking ref is updated; the leading `+` permits a remote rewind. Object downloads still change Git's object database. After fetch, re-read the actual tracking SHA: it may differ from the checked SHA if the remote moved between check and fetch. A concurrent fetch that reaches the same *current* remote SHA also counts as success; do not label a different SHA as current merely because it equals the earlier check.
   - **Skips the API for an ignored repository.** When `~/.wt.json` lists this repository (§8), it runs `ls-remote` directly and makes no provider request.
   - **Records progress** in the store's `attempt` record (below): phase `checking`, `checking-fallback(reason)` when it switches to `ls-remote` after an API failure, then `fetching`, then a terminal outcome.
   - Runs the PR refresh on its own thread, as today.
3. **Spinner.** While waiting, show `{spinner} updating` on stderr. The text follows the attempt's phase:
   - `checking-fallback(no key)`: `{spinner} no API key, using fallback method`;
   - `checking-fallback(rate limited)`: `{spinner} rate limited, using fallback method`;
   - `fetching`: `{spinner} pulling remote updates`.

    The spinner is drawn only when stderr is a terminal, so captured output and the shell wrapper never see it. It appears only after 150 ms, so the common no-variance path doesn't flash.
4. **Wait** until the attempt has a terminal outcome, or 3 s have passed. Identify an attempt by a unique token plus the origin digest and default branch, not by second-resolution timestamps. If launch fails or the worker exits before starting, render the previous answer with an unavailable reason immediately. A timed-out foreground must never show a terminal outcome from a different repository, branch, or earlier run.
5. **Clear the spinner line** and render:
   - with an outcome: the matching §4 row;
   - at 3 s: the "still checking" or "still pulling" row of §4, and the refresh hint (§6). The worker carries on, and the next run shows its result.

Gather the local refs, comparison counts, and graph *after* the wait. A fetch completed during the wait changes those inputs; retaining a pre-wait snapshot would show new remote wording beside old metrics. If a fetch is still running at 3 s, render one coherent local snapshot and keep the checked observation separate. Continue to use the worktree library's [listing facts](../../lib/src/listing.rs) and the CLI's [pure table renderer](../../cli/src/commands/list_table.rs); render terminal text through `biscuit-terminal`'s `TerminalRenderable` components.

**Store.** The remote-head store (format 2) keeps the last successful answer and adds `attempt: { id, origin_digest, branch, started_at, phase, outcome } | null`. `id` is a unique attempt token; `phase` and `outcome` are separate. Terminal outcomes are `in-sync`, `fetched`, `absent`, `fetch-failed(reason)`, and `check-failed(reason)`. Validate every field and discard an attempt with a future timestamp, wrong origin or branch, or age beyond the combined 10 s check and 60 s fetch deadlines plus a short publication allowance. Read format 1 as a last successful answer with no attempt, so an upgrade does not erase valid evidence. A check failure preserves the previous answer; a fetch failure preserves the **newly checked** answer. Cache writes and spawn failures leave a bounded wait and a truthful unavailable state.

The worker writes the store atomically under `<repo hash>.remote-head.lock`, which it holds for the whole attempt. It stamps `checked_at` before the request, and discards the answer when `origin` or the default branch changed during it. In that case the attempt ends as unavailable, without publishing a successful answer. The PR half retains its separate lock and can finish independently.

This reverses decision 2 of `2026-09-26-stale-remote-caption` ("Listing never fetches"): `wt list` fetches when a check has just shown that `origin/<default>` is out of date. The fetch touches only `refs/remotes/origin/<default>`.

### 4. Caption

One sentence. The comparison phrase (`is N commits behind`, `is N commits ahead of`, `is in sync with`, `has diverged from … (N ahead, M behind)`) drops "local tracking ref"; counts keep their colors. Everything after the comparison is dim italic.

| State | Caption |
|---|---|
| checked this run, no variance | `main is 3 commits behind origin/main (checked just now)` |
| variance; the fetch succeeded | `main is 3 commits behind origin/main (updated from origin just now)`, counts from the new tip |
| variance; the fetch failed | `main is 3 commits behind local origin/main (origin differed when checked just now; fetch didn't finish within 60 s)` |
| still checking at 3 s | `main is 3 commits behind origin/main (origin hasn't answered yet; still checking in the background; last checked with origin 2 h ago)` |
| still pulling at 3 s | `main is 3 commits behind local origin/main (origin differed when checked just now; pulling remote updates in the background)` |
| check failed; an earlier answer exists | `main is 3 commits behind origin/main (couldn't check origin; last checked with origin 2 h ago)` |
| check failed; no earlier answer; `origin/main` has a reflog | `main is 3 commits behind origin/main (couldn't check origin; tracking ref last changed 3 h ago)` |
| check failed; nothing known | `main is 3 commits behind origin/main (couldn't check origin; never checked with origin)` |
| branch absent on origin, tracking ref still exists | `main is in sync with origin/main (main was absent on origin when checked just now; origin/main is a local tracking ref)` |
| no local tracking ref, no local default branch, no `origin` | unchanged from `2026-09-26-stale-remote-caption` |

- **Reasons.** Distinguish a check timeout (`origin didn't answer within 10 s`), a fetch timeout (`fetch didn't finish within 60 s`), and a confirmed credentials failure. Other failed checks read `couldn't check origin`; they do not claim the host was unreachable or the machine was offline. Classify Git failures conservatively from sanitized stderr with `LC_ALL=C`; never render raw stderr, remote URLs, credential values, or arbitrary provider messages.
- **Tracking-ref age.** `git reflog -1 --format=%ct refs/remotes/origin/<default>` runs only when there is no stored answer. It dates the last *change* to that ref, not the last fetch or check. A missing or disabled reflog falls through to "never checked with origin" without implying the ref is new.
- **Absence.** A verified remote absence does not delete the local tracking ref. If it remains, the comparison is still against that local ref and must say so. If it has been pruned, show only the remote-absence observation. Never print "in sync with origin" for an absent remote branch.
- **Unchanged:** age units, future-date and other-branch rejection, the no-origin rule, and wrapping.

### 5. Credentials warning

When `origin` is a supported provider and this run's branch-head check or PR request failed for a *confirmed* credentials or rate-limit reason, print one dim line after the caption. A background PR failure from another invocation is not this run's evidence. Conditions are functional; mapping a provider's status codes onto them is sniff's job (below), never `wt`'s.

`{provider}` is the provider's display name. `{key}` is the variable that was used, or, when none was set, the variables the provider's definition accepts (for example `GITHUB_TOKEN or GH_TOKEN`).

| Condition | Message |
|---|---|
| Updated information was retrieved (with or without an API key) | *(none)* |
| No API key is set, the repository isn't visible without one, and the `ls-remote` fallback answered | *(no line here; the fallback notice in §8 applies)* |
| No API key is set, the repository isn't visible without one, and the fallback failed too | `{provider} did not show this repository, and Git could not check it. If it is private, set {key} and try again.` |
| An API key is set, but the provider didn't accept it | `{provider} didn't accept {key}; it may be invalid, expired, or revoked. Replace it and try again.` |
| An API key is set and the provider explicitly denied access | `The API key {key} doesn't have rights to view this repository on {provider}.` |
| No API key is set, and the provider rate limited the request | `{provider} rate limited the request for updated information. Add the {key} API key to get larger rate limits.` |
| An API key is set, and the provider still rate limited the request | `{provider} rate limited the request for updated information. Try again in a few minutes.` |

- **Classification lives in sniff.** Sniff already has typed `MissingCredentials`, `InvalidCredentials`, `RemoteForbidden`, and `RateLimited` errors, but `blocking::classify` folds several into `PrUnavailable::Auth`. Split confirmed cases into `CredentialsRequired`, `CredentialsRejected`, `CredentialsInsufficient`, and `RateLimited { authenticated: bool }`. Preserve a separate ambiguous `NotFoundOrNotPermitted` case. A 404, with or without a token, does not prove whether the repository, branch, or permission is missing. A 403 is `CredentialsInsufficient` only when the provider's response establishes that rather than rate limiting or hiding the resource. Carry the used variable name as metadata, never its value.

  Update `worktree::pull_requests`'s mapping to match.
- **404 is ambiguous at the provider.** It produces no credentials warning by itself. The `ls-remote` fallback (§3) settles whether the branch exists; if Git also fails, report that the remote could not be checked and offer the conditional private-repository hint above only when no key is set.
- **Names, not secrets.** Provider and variable names come from `sniff::remote::blocking::credential_env`. The line names variables only, never their values.
- **When it's shown.** Only when this invocation observed the condition before rendering. A detached worker that finishes later can influence the next listing through its stored result, but must not make this listing claim it saw a failure that it did not.

### 6. Refresh hint

Print one dim line after the git graph (after the PR age line when no graph is drawn), before the verbose section, when this listing rendered while the worker was still working: the 3 s wait ran out, or a PR refresh is still running:

```text
- running this command again will provide updated metrics; alternatively use the --refresh / -r flags to force refresh immediately
```

It keys on unfinished work. A later run may still see a failure or a running request, so phrase the hint as a way to check again, not a promise of new metrics.

### 7. `--refresh` / `-r`

A new global flag on `Cli`, alongside `--width` and `--verbose`, so `wt -r` and `wt list -r` both work. Limit it to the default listing and explicit `list`; reject it with create, go, and remove rather than silently changing those commands. It ignores both the PR and remote-head freshness windows and waits for the full check, fetch, and PR refresh, bounded by their existing 10 s and 60 s deadlines. An existing worker's result may be used only when it was started for the same origin and branch; otherwise begin a forced attempt after it finishes. A failure prints the matching §4 or §5 wording and does not fail the listing. The worker writes one atomic, attempt-token-bound completion receipt after *both* halves finish, with each half's success or failure. The foreground waits for that receipt, not merely the remote-head outcome; when either half's lock is contended, it waits for the holder within the same deadline and checks that half's store before reporting completion. A stale or mismatched receipt is ignored.

### 8. Fallback notice and `--ignore-api`

When this run's check fell back to `git ls-remote` because no API key was set and the fallback answered, print this at the very end of the output, after the git graph and one blank line (after the legend, PR age line, and refresh hint when no graph is drawn). A provider 404 without a key also qualifies when Git answered; a later worker completion does not retroactively add a notice to an already rendered listing:

```text
- Git checked origin using `ls-remote`; this can take longer than the provider API.
- Set {key} to let wt try the provider API, or use --ignore-api to use Git directly for this repository.
```

`{key}` is as in §5. Like the rest of `wt list`'s output, the notice goes to stderr.

**`--ignore-api`** is a new flag on `Cli`, alongside `--refresh`, so `wt --ignore-api` and `wt list --ignore-api` both work:
- Like `--refresh`, accept it only for listing. Reject it with create, go, and remove.
- It records the current repository in `~/.wt.json` before starting the update, so that run uses `ls-remote` directly and prints no notice. Reject this flag when `origin` is absent or cannot be identified; do not create a meaningless entry.
- Identify the repository by normalized provider host, effective port, and repository path from `origin`, with URL user information and credentials removed. Distinct ports or paths must not share a preference; HTTPS and SSH URLs for the same repository should share it when they resolve to the same provider identity. Never persist the raw URL.
- For a listed repository, the worker skips every provider API request, for both the branch head and PRs. It uses `ls-remote` directly, and no notice or credentials line is printed. PR badges are therefore absent for that repository.
- The file has a format version and a list of repositories, and is written atomically under a persistent lock so concurrent `wt` processes do not lose each other's entries. A missing file means nothing is ignored. An unreadable or corrupt file is treated as empty for this listing, but `--ignore-api` must report a write error instead of silently discarding existing preferences.
- Resolve the user home through the same cross-platform mechanism already used for worktree's stores, including native Windows, instead of assuming a literal `~` or one environment variable. Removing an entry, or the file, undoes the choice.

### 9. Fast-forward suggestion and `--fast-forward` / `--ff`

The fetch updates only `origin/<default>`. The local default branch stays where it was until the user moves it.

**Suggestion.** When the local default branch is strictly behind `origin/<default>` (not diverged), add a line to the end-of-output notes, after the graph and the blank line and before the §8 notice:

```text
- main is 3 commits behind origin/main; run wt --ff to fast-forward it.
```

It isn't shown when the default branch is in sync, ahead, or diverged, or when the listing rendered before the fetch finished.

**`--fast-forward` / `--ff`** is a new flag on `Cli`, alongside `--refresh`. It waits for the update flow's result like `--refresh` (the 3 s limit doesn't apply), then moves the local default branch to `origin/<default>` when that is a fast-forward:

| Situation | Action | Output |
|---|---|---|
| default branch isn't checked out in any worktree | `git update-ref refs/heads/<default> <new> <old>`: compare-and-swap, no working tree involved | the caption shows "in sync" |
| checked out in a worktree | `git -C <that worktree> merge --ff-only origin/<default>` after verifying its current branch is still the default | the caption shows "in sync" |
| checked out, and Git refuses because local changes would be overwritten | nothing changes | `main wasn't fast-forwarded: the checkout has uncommitted changes to files the update touches.` |
| diverged | nothing changes | `main has diverged from origin/main, so it can't be fast-forwarded.` |
| no local default branch or no tracking ref | nothing changes | say which ref is missing; do not create a branch |
| already in sync, or ahead | nothing | no message |
| the update flow failed (no answer or fetch failed) | fast-forwards to the local `origin/<default>` if it's ahead | the caption keeps its check or fetch failure reason, so the user knows the target may itself be out of date |

- Re-read both refs and verify ancestry immediately before the move. `update-ref` names the expected old value; `merge --ff-only` refuses rather than overwriting local changes. If the default branch moved to another checkout or changed while waiting, re-resolve its location and refuse when it cannot be made safe. A refusal prints the reason and does not fail the listing.
- Like the other listing flags, reject `--ff` with create, go, and remove. Document whether combining `--ff` and `--refresh` is accepted (one forced update, one fast-forward), and reject conflicting combinations only when they cannot be honored.
- The fast-forward runs before gathering the final listing facts. Rebuild the local ref snapshot and comparison cache after a successful move, so the caption, table, counts, and graph describe the same post-update state. The remote observation retains its actual check time; a failed check never becomes "checked just now" merely because `--ff` moved the local branch.

## Out of scope

- Fetching other branches, pruning, or updating any ref except `refs/remotes/origin/<default>`.
- Interactive credential prompts; the warning in §5 is the remedy.
- GraphQL or combining the PR and branch-head calls into one request.
- A setting to silence the §5 credentials lines. They appear only when a key would fix an actual failure. The fallback notice has `--ignore-api` (§8).
- A command to list or remove `~/.wt.json` entries; editing the file does it.
- Fast-forwarding any branch other than the default branch, or rebasing or merging anything that isn't a fast-forward.

## Acceptance criteria and testing

Use the `rust-testing` skill and the Test Toolkit. All L1 unless noted.

1. **schematic:** each new endpoint has a definition test for its path, method, and auth, matching the existing per-provider tests, and the generated client compiles.
2. **sniff:** a `wiremock` test per provider for `branch_head`: SHA parsing, invalid SHA rejected, deadline, branch path encoding, the anonymous fallback succeeding on a public repository, and each §5 variant from the status codes that provider actually uses (404 and 401 with and without a token, 403, 429, and 403-as-rate-limit where the provider does that). `credential_env` returns each definition's names.
3. **Reachability:** a local bare origin and a LAN origin work without a default route; an SSH alias is not rejected by a speculative TCP probe. A refused connection and a black-holed request produce bounded, truthful failure reasons and preserve the previous answer. A provider failure and Git fallback share the 10 s check budget.
4. **Update flow**, with the provider source, worker launcher, and clock injected through `ListSeams`, against a real local bare origin and a `pusher` clone:
   - no variance publishes and renders "checked just now", with no fetch;
   - variance fetches, publishes, and renders the new counts, and `FETCH_HEAD` is unchanged;
   - refs and graph gathered after the wait agree with the rendered caption; a fetch still running at the deadline leaves one coherent local snapshot;
   - a fetch that times out renders the "origin differed; fetch failed" row, retains the newly checked observation, and leaves the tracking ref untouched;
   - unreachable and timed-out origins leave the previous answer usable and visibly dated;
   - a worker still checking at 3 s renders the "still checking" row and the hint, and the next run shows its result;
   - a fetch still running at 3 s renders the "still pulling" row, and the worker completes and publishes after `wt list` has exited;
   - a second `wt list` while a worker runs adopts its matching attempt without making a second network request; a contender that exits early is not reported as a finished check;
   - a check that exceeds the worker's 10 s deadline records `check-failed` and keeps the last answer;
   - an unsupported remote uses `ls-remote`;
   - an API credentials failure falls back to `ls-remote`, and the spinner phase reads `no API key, using fallback method`;
   - a repository listed in `~/.wt.json` makes no provider request of any kind;
   - a remote rewind updates the one tracking ref, another branch's ref and `FETCH_HEAD` remain unchanged, and a remote movement between check and fetch is reported from the fetched tip rather than the earlier SHA;
   - losing the ref lock to a concurrent fetch that reached the current remote SHA counts as success;
   - an upgrade reads a format-1 answer, and failed or stale attempt records never replace it or appear as current;
   - the spinner is never written when stderr isn't a terminal.
5. **Caption snapshots** (`cli/tests/list_table.rs`): every §4 row across all four comparison states, each reason, the reflog row, and non-`main` default names.
6. **Credentials warning:** every §5 row, per provider name, with no variable values in the output; no line when the anonymous request succeeded; a 404 remains ambiguous, with and without a token. Update the PR lookup's existing tests for the split `Auth` variant, and cover a PR failure that completes only after this listing renders.
7. **Hint and flag:** the hint appears only when the listing rendered with work unfinished. `-r` waits for both worker halves, forces fresh requests even when both caches are young, and has bounded behavior when launch or publication fails. `-r` appears in `wt --help` and `wt list --help`, is rejected for unrelated subcommands, and updates the completion snapshot.
8. **Fallback notice and `--ignore-api`:**
   - the notice appears after the graph and a blank line only when the no-key fallback answered, and never for a rate-limit fallback;
   - `--ignore-api` writes the repository entry before the run, and that run shows no notice;
   - a corrupt `~/.wt.json` is treated as empty;
   - the stored identity never contains a URL's user or password, separates different ports, and unifies equivalent SSH and HTTPS provider identities;
   - concurrent writes retain both preferences, and an unreadable preference file is not overwritten by `--ignore-api`;
   - on Windows, the file resolves under `%USERPROFILE%`.
9. **Fast-forward:** against a real local bare origin and a `pusher` clone:
   - the suggestion appears only when the default branch is strictly behind;
   - `--ff` with the default branch not checked out moves the ref, and a concurrent change to it makes the compare-and-swap refuse;
   - `--ff` with the default branch checked out and clean fast-forwards the working tree;
   - dirty files the update touches make it refuse with the message and leave the files untouched;
   - a branch that moves to another worktree or changes while the command waits is re-resolved or refused safely;
   - diverged refuses;
   - in sync makes no change and prints nothing;
   - a failed remote check fast-forwards to the local `origin/<default>` and keeps the check failure reason.
10. **Performance:**
   - The pre-network local gather and render retain the existing 1 s bound. The intentional remote wait adds at most 3 s to ordinary listing when the worker stalls; state the resulting full-command bound separately in `worktree/docs/performance-testing.md`.
   - With the check or the fetch stalled, the command returns within 3 s plus render time, and the captured output returns while the worker is still held (the `list_prs` no-join pattern).
   - `--refresh` and `--ff` obey the 10 s check and 60 s fetch deadlines and finish or report failure without an unbounded wait on a worker or cache lock.
11. **L2** (`level2_list_verbose`): update the caption assertion; check dim italic on the suffix, the hint, and the warning; and check that the spinner line is cleared before the caption is drawn.

## Packages and documentation

This is a cross-area change: `schematic` (definitions, then regenerated schema), `sniff` (lib), `biscuit-terminal` (lib), and `worktree` (lib and CLI). CI scope is determined by the repository planner; unchanged reverse dependencies are compiled in their producer's check cell where required, rather than scheduled as separate work.

- **schematic:** the four endpoints and their README / definition docs.
- **sniff:** `branch_head`, `BranchHead`, `credential_env`, and the split credentials variants; the `sniff` skill's remote section.
- **biscuit-terminal:** a `Spinner` component (stderr, terminal-only, delayed start, replaceable text, clears its line), with its docs page and skill entry.
- **worktree:**
  - the §3 update flow and fetch-then-publish;
  - `wt internal-refresh` running the check, the fetch, and the `attempt` record;
  - `list.rs` orchestration and the `--refresh`, `--ignore-api`, and `--fast-forward` flags;
  - the fast-forward (`update-ref` or `merge --ff-only`);
  - the `~/.wt.json` reader and writer;
  - `list_table.rs` rendering;
  - `worktree/docs/cli/list.md`, `worktree/docs/performance-testing.md`, the README's `wt list` description, and the worktree skill's `wt list` section, including that listing now checks and fetches.

## Decisions

1. **Check the remote on every run, and wait up to 3 s for it.** The check and any fetch run in the detached worker from the start, so work still running at 3 s continues in the background while the listing reports what it has. This deliberately replaces the earlier 1 s full-command target with a bounded remote wait. (Decided 2026-09-27; clarified in review.)
2. **Fetch when the check finds variance.** Showing numbers known to be wrong helps nobody; the spinner says `pulling remote updates` while it runs. (Decided 2026-09-27; reverses the earlier "listing never fetches".)
3. **Use the actual request as the reachability test.** A separate route or port probe can reject a valid local, LAN, or SSH remote, and a successful probe cannot prove the API or Git operation will answer. (Revised in review.)
4. **A 404 from a provider API is ambiguous.** Only a successful, complete `ls-remote` response without the exact branch proves absence; 404 alone proves neither missing credentials nor a missing branch.
5. **When a check fails, report local state and when remote information was last available.** Failed attempt status may be stored to communicate with the foreground, but the last good remote answer is never erased.
6. **Credentials come from the existing schematic environment mappings.** The warning names the key only when setting or fixing it would resolve an actual failure; a working anonymous request prints nothing.
7. **A missing key on a private repository falls back to `git ls-remote`, never refuses.** The spinner says so while it happens, and a closing notice explains the trade-off. `--ignore-api` records a per-repository choice in `~/.wt.json` to use git directly and skip the notice. (Decided 2026-09-27.)
8. **The local default branch moves only on request.** `wt` suggests `wt --ff` when it's strictly behind, and `--fast-forward` / `--ff` performs a fast-forward that never forces or overwrites. (Decided 2026-09-27.)

## Open Questions

None. The worker's 10 s check limit and 60 s fetch limit were confirmed on 2026-09-27. They are safety caps so a detached process cannot hang while holding `remote-head.lock`. Ordinary listing waits at most 3 s; `--refresh` and `--ff` may wait for the full bounded work.
