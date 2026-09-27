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
reviewed: false
review_iterations: 0
clarified: true
implemented: false
related:
    - 2026-09-26-stale-remote-caption
human_review: false
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
- A 404 is never an absence. Providers answer a private repository this way, so only `ls-remote` can prove that a branch is absent (§3).
- Add `credential_env(remote_url) -> Option<CredentialEnv>`, which returns the provider's display name and the variable names from its definition, so `worktree` never hard-codes them.

### 2. Connectivity probe (sniff)

Add `sniff::network::connectivity(target, deadline) -> Connectivity`, a reusable check in two tiers:

1. **No default route:** return `Offline` at once. This uses `detect_default_gateways` (a routing-table read, no network traffic).
2. **Reachability:** a TCP connect to `target` (host and port) within `deadline`. The result is `Reachable`, `Unreachable` (refused, no route to host, DNS failure), or `TimedOut`.

`wt` passes origin's own host and port (443 for an HTTPS or API check, 22 or the configured port for SSH), not a generic internet endpoint. What matters is whether origin is reachable: a LAN Gitea works without internet, and a provider can be down while the internet is up.

### 3. The update flow (`wt list`, every run)

Every `wt list` with an `origin` and a resolvable default branch runs this before rendering. The check and the fetch run in the detached `wt internal-refresh` worker from the start. The foreground only waits for the result, for up to **3 s**, so nothing has to be handed off when time runs out.

1. **Connectivity** (§2) against origin's host, in the foreground. `Offline` or `Unreachable` launches nothing, and the listing renders local state with the last remote information (§4) at once.
2. **Launch the worker** unless one is already running for this repository (its `remote-head.lock` is held), in which case wait on that one. The worker:
   - **Checks for variance.** For a provider sniff supports it calls `branch_head` (§1). For any other remote (a local bare repository, an unrecognized host or SSH alias, a policy-forbidden host), or when `branch_head` fails for a credentials or rate-limit reason, it runs `git ls-remote origin refs/heads/<default>`. `ls-remote` is also the only proof that a branch is absent. Deadline 10 s.
   - **Fetches on variance.** It runs `git fetch origin <default>` through `worktree::live_remote::run_noninteractive` (no credential prompts, SSH batch mode, process-tree kill), with a 60 s deadline, `--no-write-fetch-head` so a user's `FETCH_HEAD` is left alone, and `-c maintenance.auto=false -c gc.auto=0` so it starts no maintenance. A fetch that loses the ref lock to a concurrent `git fetch` re-reads the tracking tip. If the tip now equals the checked SHA, another process did the work, and this counts as success.
   - **Skips the API for an ignored repository.** When `~/.wt.json` lists this repository (§8), it runs `ls-remote` directly and makes no provider request.
   - **Records progress** in the store's `attempt` record (below): phase `checking`, `checking-fallback(reason)` when it switches to `ls-remote` after an API failure, then `fetching`, then an outcome that notes whether `ls-remote` answered.
   - Runs the PR refresh on its own thread, as today.
3. **Spinner.** While waiting, show `{spinner} updating` on stderr. The text follows the attempt's phase:
   - `checking-fallback(no key)`: `{spinner} no API key, using fallback method`;
   - `checking-fallback(rate limited)`: `{spinner} rate limited, using fallback method`;
   - `fetching`: `{spinner} pulling remote updates`.

    The spinner is drawn only when stderr is a terminal, so captured output and the shell wrapper never see it. It appears only after 150 ms, so the common no-variance path doesn't flash.
4. **Wait** until the attempt has an outcome, or 3 s have passed.
5. **Clear the spinner line** and render:
   - with an outcome: the matching §4 row;
   - at 3 s: the "still checking" or "still pulling" row of §4, and the refresh hint (§6). The worker carries on, and the next run shows its result.

**Store.** The remote-head store (format 2) keeps the last successful answer as today, and adds `attempt: { started_at, phase, outcome } | null`:
- `outcome` is `in-sync`, `fetched`, `absent`, `fetch-failed(reason)`, or `check-failed(reason)`;
- the foreground reads only an attempt whose `started_at` is not earlier than its own launch, or the running worker's attempt;
- a failed attempt never overwrites or erases the last successful answer.

The worker writes the store atomically under `<repo hash>.remote-head.lock`, which it holds for the whole attempt. It stamps `checked_at` before the request, and discards the answer when `origin` or the default branch changed during it.

This reverses decision 2 of `2026-09-26-stale-remote-caption` ("Listing never fetches"): `wt list` fetches when a check has just shown that `origin/<default>` is out of date. The fetch touches only `refs/remotes/origin/<default>`.

### 4. Caption

One sentence. The comparison phrase (`is N commits behind`, `is N commits ahead of`, `is in sync with`, `has diverged from … (N ahead, M behind)`) drops "local tracking ref"; counts keep their colors. Everything after the comparison is dim italic.

| State | Caption |
|---|---|
| checked this run, no variance | `main is 3 commits behind origin/main (checked just now)` |
| variance; the fetch succeeded | `main is 3 commits behind origin/main (updated from origin just now)`, counts from the new tip |
| variance; the fetch failed | `main is 3 commits behind origin/main as of your last fetch, but origin/main has moved since (fetch failed: origin didn't answer within 60 s)` |
| still checking at 3 s | `main is 3 commits behind origin/main (origin hasn't answered yet; still checking in the background; last checked with origin 2 h ago)` |
| still pulling at 3 s | `main is 3 commits behind origin/main as of your last fetch, but origin/main has moved; pulling remote updates in the background` |
| no check this run; an earlier answer exists | `main is 3 commits behind origin/main (offline; last checked with origin 2 h ago)` |
| no check this run; no earlier answer; `origin/main` has a reflog | `main is 3 commits behind origin/main (offline; updated 3 h ago by your last fetch)` |
| no check this run; nothing known | `main is 3 commits behind origin/main (offline; never checked with origin)` |
| branch absent on origin | `main is in sync with origin/main, but main no longer exists on origin (checked just now)` |
| no local tracking ref, no local default branch, no `origin` | unchanged from `2026-09-26-stale-remote-caption` |

- **Reasons.** `offline` in the rows above stands for the reason no check happened:
  - `offline` (no default route);
  - `couldn't reach origin` (unreachable, or a network error);
  - `origin didn't answer within 10 s` (the worker's check deadline);
  - `origin needs credentials wt can't supply` (the API failed for a credentials reason and `ls-remote` failed too; §5 prints the specific line).

  An unrecognized failure reads `couldn't reach origin`. Git failures are classified from stderr with `LC_ALL=C`, and API failures from `PrUnavailable`.
- **"Updated … by your last fetch"** uses `git reflog -1 --format=%ct refs/remotes/origin/<default>`, run only when there is no stored answer. The reflog dates the ref's last change, not the last fetch, which is why the wording says "updated".
- **Unchanged:** age units, future-date and other-branch rejection, the no-origin rule, and wrapping.

### 5. Credentials warning

When `origin` is a supported provider and the branch-head check or the PR request failed for a credentials or rate-limit reason, print one dim line after the caption. Conditions are functional; mapping a provider's status codes onto them is sniff's job (below), never `wt`'s.

`{provider}` is the provider's display name. `{key}` is the variable that was used, or, when none was set, the variables the provider's definition accepts (for example `GITHUB_TOKEN or GH_TOKEN`).

| Condition | Message |
|---|---|
| Updated information was retrieved (with or without an API key) | *(none)* |
| No API key is set, the repository isn't visible without one, and the `ls-remote` fallback answered | *(no line here; the fallback notice in §8 applies)* |
| No API key is set, the repository isn't visible without one, and the fallback failed too | `{provider} won't show this repository without an API key; it's probably private. Set {key} to get updated information.` |
| An API key is set, but the provider didn't accept it | `{provider} didn't accept {key}; it may be invalid, expired, or revoked. Replace it and try again.` |
| An API key is set and accepted, but it can't see this repository | `The API key {key} doesn't have rights to view this repository on {provider}.` |
| No API key is set, and the provider rate limited the request | `{provider} rate limited the request for updated information. Add the {key} API key to get larger rate limits.` |
| An API key is set, and the provider still rate limited the request | `{provider} rate limited the request for updated information. Try again in a few minutes.` |

- **Classification lives in sniff.** Sniff already distinguishes these internally (`MissingCredentials`, `InvalidCredentials`, `RemoteForbidden`, `RateLimited`, and 404 "not found or not permitted"), including each provider's use of 403 for rate limits. But `blocking::classify` folds the first three into one `PrUnavailable::Auth`. Replace that variant with:
  - `CredentialsRequired`: no key set, and the repository wasn't visible (a 404 or 401 without a token);
  - `CredentialsRejected`: a key set, and not accepted (401 with a token);
  - `CredentialsInsufficient`: a key set and accepted, but no access (403, or a 404 with a token);
  - `RateLimited { authenticated: bool }`.

  Update `worktree::pull_requests`'s mapping to match.
- **404 is ambiguous at the provider.** A 404 on the branch endpoint can also mean that the branch doesn't exist. It never counts as an absence (§1); it maps to one of the credentials conditions above, and the `ls-remote` fallback (§3) settles whether the branch exists.
- **Names, not secrets.** Provider and variable names come from `sniff::remote::blocking::credential_env`. The line names variables only, never their values.
- **When it's shown.** On every listing where this run's branch-head check or PR request hit the condition.

### 6. Refresh hint

Print one dim line after the git graph (after the PR age line when no graph is drawn), before the verbose section, when this listing rendered while the worker was still working: the 3 s wait ran out, or a PR refresh is still running:

```text
- running this command again will provide updated metrics; alternatively use the --refresh / -r flags to force refresh immediately
```

It keys on unfinished work, so it appears exactly when running again will show something new.

### 7. `--refresh` / `-r`

A new flag on `Cli`, alongside `--width` and `--verbose`, so `wt -r` and `wt list -r` both work. It ignores the PR freshness window, and waits for the worker's full result (check, fetch, and PR refresh) instead of stopping at 3 s. A failure prints the matching §4 or §5 wording and never fails the command.

### 8. Fallback notice and `--ignore-api`

When this run's check fell back to `git ls-remote` because no API key was set (`CredentialsRequired`) and the fallback answered, print this at the very end of the output, after the git graph and one blank line (after the legend, PR age line, and refresh hint when no graph is drawn):

```text
- update was achieved using `git ls-remote` which is much slower than the API.
- If you add the API key {key} we will be able to process this faster.
- If you prefer to not be bothered by this message again just add the --ignore-api to the end of your next request and this information will be removed.
```

`{key}` is as in §5. Like the rest of `wt list`'s output, the notice goes to stderr.

**`--ignore-api`** is a new flag on `Cli`, alongside `--refresh`, so `wt --ignore-api` and `wt list --ignore-api` both work:
- It records the current repository in `~/.wt.json`. It is saved before the run starts, so the run it's given on already uses `ls-remote` directly and prints no notice.
- The repository is identified by host and path as parsed from `origin` (`github.com/yankeeinlondon/rusty-biscuit`), never the raw URL. That keeps the file readable and free of credentials that a URL can carry.
- For a listed repository, the worker skips every provider API request, for both the branch head and PRs. It uses `ls-remote` directly, and no notice or credentials line is printed. PR badges are therefore absent for that repository.
- The file has a format version and a list of repositories, and is written atomically. A missing file means nothing is ignored. An unreadable or corrupt file is treated as empty and never fails the command.
- `~/.wt.json` resolves the home directory the same way on macOS, Linux, and Windows (`%USERPROFILE%`). Removing an entry, or the file, undoes the choice.

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
| checked out in a worktree | `git -C <that worktree> merge --ff-only origin/<default>` | the caption shows "in sync" |
| checked out, and git refuses because local changes would be overwritten | nothing changes | `main wasn't fast-forwarded: the base checkout has uncommitted changes to files the update touches.` |
| diverged | nothing changes | `main has diverged from origin/main, so it can't be fast-forwarded.` |
| already in sync, or ahead | nothing | no message |
| the update flow failed (offline, no answer, fetch failed) | fast-forwards to the local `origin/<default>` if it's ahead | the caption keeps its usual reason, so the user knows the target may itself be out of date |

- Nothing is forced: `update-ref` names the expected old value, and `merge --ff-only` refuses rather than overwriting local changes. A refusal prints the reason and never fails the command.
- The fast-forward runs before rendering, so the table, counts, and graph reflect it.

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
2. **sniff:** a `wiremock` test per provider for `branch_head`: SHA parsing, invalid SHA rejected, deadline, the anonymous fallback succeeding on a public repository, and each §5 variant from the status codes that provider actually uses (404 and 401 with and without a token, 403, 429, and 403-as-rate-limit where the provider does that). `credential_env` returns each definition's names.
3. **Connectivity (sniff):**
   - no default route returns `Offline` without opening a socket;
   - a loopback listener is `Reachable`;
   - a refused port is `Unreachable`;
   - a black-holed connect is `TimedOut` at the deadline.
4. **Update flow**, with the provider source, connectivity, and clock injected through `ListSeams`, against a real local bare origin and a `pusher` clone:
   - no variance publishes and renders "checked just now", with no fetch;
   - variance fetches, publishes, and renders the new counts, and `FETCH_HEAD` is unchanged;
   - a fetch that times out renders the "moved; fetch failed" row and leaves the stored answer untouched;
   - `Offline` and `Unreachable` make no variance request and render the stored answer;
   - a worker still checking at 3 s renders the "still checking" row and the hint, and the next run shows its result;
   - a fetch still running at 3 s renders the "still pulling" row, and the worker completes and publishes after `wt list` has exited;
   - a second `wt list` while a worker runs launches none and waits on the running one;
   - a check that exceeds the worker's 10 s deadline records `check-failed` and keeps the last answer;
   - an unsupported remote uses `ls-remote`;
   - an API credentials failure falls back to `ls-remote`, and the spinner phase reads `no API key, using fallback method`;
   - a repository listed in `~/.wt.json` makes no provider request of any kind;
   - losing the ref lock to a concurrent fetch that reached the same SHA counts as success;
   - the spinner is never written when stderr isn't a terminal.
5. **Caption snapshots** (`cli/tests/list_table.rs`): every §4 row across all four comparison states, each reason, the reflog row, and non-`main` default names.
6. **Credentials warning:** every §5 row, per provider name, with no variable values in the output; no line when the anonymous request succeeded; the PR lookup's existing tests are updated for the split `Auth` variant.
7. **Hint and flag:** the hint appears only when the listing rendered with work unfinished. `-r` waits for the worker's full result. `-r` appears in `wt --help` and `wt list --help`, and the completion snapshot updates.
8. **Fallback notice and `--ignore-api`:**
   - the notice appears after the graph and a blank line only when the no-key fallback answered, and never for a rate-limit fallback;
   - `--ignore-api` writes the repository entry before the run, and that run shows no notice;
   - a corrupt `~/.wt.json` is treated as empty;
   - the stored identity never contains a URL's user or password;
   - on Windows, the file resolves under `%USERPROFILE%`.
9. **Fast-forward:** against a real local bare origin and a `pusher` clone:
   - the suggestion appears only when the default branch is strictly behind;
   - `--ff` with the default branch not checked out moves the ref, and a concurrent change to it makes the compare-and-swap refuse;
   - `--ff` with the default branch checked out and clean fast-forwards the working tree;
   - dirty files the update touches make it refuse with the message and leave the files untouched;
   - diverged refuses;
   - in sync makes no change and prints nothing;
   - offline fast-forwards to the local `origin/<default>` and keeps the offline reason.
10. **Performance:**
   - With no variance and a provider check answering at its measured speed, the full command stays under the existing 1 s bound, including starting the worker.
   - With the check or the fetch stalled, the command returns within 3 s plus render time, and the captured output returns while the worker is still held (the `list_prs` no-join pattern).
   - `Offline` adds no more than 10 ms.
11. **L2** (`level2_list_verbose`): update the caption assertion; check dim italic on the suffix, the hint, and the warning; and check that the spinner line is cleared before the caption is drawn.

## Packages and documentation

This is a cross-area change: `schematic` (definitions, then regenerated schema), `sniff` (lib), `biscuit-terminal` (lib), and `worktree` (lib and CLI). CI will also schedule the reverse dependencies of sniff and biscuit-terminal.

- **schematic:** the four endpoints and their README / definition docs.
- **sniff:** `branch_head`, `BranchHead`, `credential_env`, the split credentials variants, and `network::connectivity`; the `sniff` skill's remote and network sections.
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

1. **Check the remote on every run, and wait up to 3 s for it.** The connectivity probe runs in the foreground. The check and any fetch run in the detached worker from the start, so work still running at 3 s continues in the background without a hand-off, while the listing reports what it has. (Decided 2026-09-27.)
2. **Fetch when the check finds variance.** Showing numbers known to be wrong helps nobody; the spinner says `pulling remote updates` while it runs. (Decided 2026-09-27; reverses the earlier "listing never fetches".)
3. **Probe origin's host, not the internet.** Reachability of origin is the question; `sniff::network::connectivity` is reusable for any host.
4. **A 404 from a provider API is never an absence.** Only `ls-remote` proves a branch is gone.
5. **When no check happens, report local state and when remote information was last available.** Failures are not stored; the last good answer is never erased.
6. **Credentials come from the existing schematic environment mappings.** The warning names the key only when setting or fixing it would resolve an actual failure; a working anonymous request prints nothing.
7. **A missing key on a private repository falls back to `git ls-remote`, never refuses.** The spinner says so while it happens, and a closing notice explains the trade-off. `--ignore-api` records a per-repository choice in `~/.wt.json` to use git directly and skip the notice. (Decided 2026-09-27.)
8. **The local default branch moves only on request.** `wt` suggests `wt --ff` when it's strictly behind, and `--fast-forward` / `--ff` performs a fast-forward that never forces or overwrites. (Decided 2026-09-27.)

## Open Questions

None. The worker's 10 s check limit and 60 s fetch limit were confirmed on 2026-09-27. They are safety caps so a detached process can't hang while holding `remote-head.lock`. Only `--refresh` and `--ff` wait on them.
