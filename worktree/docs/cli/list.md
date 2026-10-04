# `wt list`

Lists all git worktrees along with their status. This is the default command -- running `wt` with no subcommand is equivalent to `wt list`.

Before it lists anything, `wt list` checks whether `origin/<default>` is current (fetching it when it is not) and asks for the open pull requests, both in one short wait; see [Checking origin](#checking-origin). The output is then, in order: a caption, an optional credentials line (a warning, or the keyless notice), the worktree table, a legend, the git graph (image-capable terminals only), a status list holding this run's PR item and the refresh hint, the verbose section (`-v` only), and, after a blank line, any closing notes. Everything is written to stderr.

## Output

```text
  main  is 7 commits behind  origin/main  (updated from origin just now)

┌───────────────────┬──────────────────────────────────┬──────────────────────┬───────────────────────────┐
│ Worktree          │ Branch                           │ ->  origin/main      │ -> parent                 │
├───────────────────┼──────────────────────────────────┼──────────────────────┼───────────────────────────┤
│ ○ base repo       │  main                            │ —                    │ —                         │
│ ● fix-wt-ux       │ ├─ fix/wt-ux                     │ clean +2 -1  PR #99  │ —                         │
│ ● feat-theme      │ ├─ feat/theme                    │ clean +3             │ —                         │
│ ○ feat-dark-fixes │ │  └─ feat/dark-fixes            │ clean +2 -1          │ conflicts +1 -3  PR #104  │
│ ○ old-cleanup     │ └─ chore/old-cleanup             │ clean -4             │ —                         │
│ ○ release-prep    │ release/prep  PR #7 → release/2  │ clean                │ —                         │
│                   │ experiments (deleted)            │                      │                           │
│ ● spike-parser    │ └┄ spike/parser                  │ conflicts +1 -3      │ parent deleted            │
│ ● bisect          │ detached @ a1b2c3d               │ —                    │ —                         │
└───────────────────┴──────────────────────────────────┴──────────────────────┴───────────────────────────┘

 Worktree   ○ clean    ● uncommitted files    ● uncommitted source files
 Branch     └─ merges cleanly into parent     └─ conflicts with parent    └┄ parent deleted

 - main is 7 commits behind origin/main; run wt --ff to fast-forward it.
```

### Checking origin

Merge metrics are only as good as `origin/<default>`, and PR badges only as good as the last answer about open pull requests, so every listing with an `origin` asks `origin` both questions before it lists anything. A detached background `wt` process does the asking, in two halves that run side by side: the **head half** (steps 1 and 2) and the **PR half** (see [PR badges](#pr-badges)). The listing waits for both (step 3).

1. **Check.** The head half asks `origin` for the default branch's current commit. For a provider `sniff` supports (GitHub, GitLab, Gitea, Bitbucket) it asks the provider's API, using the same API key variables as the PR badges (`GITHUB_TOKEN` or `GH_TOKEN`, and so on). For any other remote (a local bare repository, an SSH alias, an unsupported host), or when the API request fails (a missing or rejected key, a 404, a rate limit, or any other error with time left), it runs `git ls-remote origin refs/heads/<default>` instead. The whole check, fallback included, is capped at 10 s. Only a complete `ls-remote` answer without the branch counts as the branch being absent; an API 404 alone proves nothing, because providers answer a private repository that way.
2. **Fetch, only when origin differs.** When the answer differs from `origin/<default>`, the head half fetches that one ref, capped at 60 s:

    ```bash
    git -c maintenance.auto=false -c gc.auto=0 fetch --no-write-fetch-head --no-tags \
      --no-recurse-submodules --refmap= origin +refs/heads/<default>:refs/remotes/origin/<default>
    ```

    Only `refs/remotes/origin/<default>` changes: no tags, no other branches, no `FETCH_HEAD`, and no submodules, whatever your Git configuration says. The leading `+` lets a remote rewind through. Your local default branch is never moved; see `--ff` below. Git runs without credential prompts, with SSH in batch mode, and is killed with its whole process tree at the deadline.
3. **Wait.** `wt list` waits up to **3 s** for both halves, so an ordinary listing costs the slower of the two, never their sum. While it waits, a spinner on stderr says `updating`, `no API key, using fallback method`, `rate limited, using fallback method`, or `pulling remote updates`, and goes back to `updating` whenever only the PR half is left, including after a retried or adopted head operation finishes. It appears only after 150 ms, only when stderr is a terminal, and its line is cleared before anything else is printed, so captured output and the shell wrapper never see it. Work still running at 3 s carries on in the background, the listing adds the refresh hint, and the next listing shows the result.
4. **Local work, at the same time.** The listing does not sit idle while it waits; see [Local work during the wait](#local-work-during-the-wait).

The check's result is stored for later listings (the last successful answer is never erased by a failed check), and at most one check or fetch runs per repository at a time: a listing that finds one already running follows it rather than asking `origin` again. `-r` and `--ff` wait for both halves the same way, only longer: up to 75 s.

Without an `origin` nothing is asked, nothing is waited for, and no PR item or refresh hint is shown.

#### Local work during the wait

The expensive local work (`git status` in every worktree Git can read, the branch comparisons behind the caption and the columns, and the graph and `-v` history) starts as soon as the branch tips are read, and runs while the listing waits. An ordinary listing therefore costs roughly the slower of the wait and the local work, not their sum. A wait that runs out limits only the waiting: local work that takes longer is still finished before anything prints.

That first pass describes the branch tips as they were before the wait, so when the wait ends (and after `--ff`, which runs only once the local work is done), the listing reads every local and `origin/*` tip again and compares the two reads:

```mermaid
flowchart TD
  A[read branch tips] --> B[wait for origin]
  A --> C[local work from those tips]
  B --> J[both done]
  C --> J
  J --> F{--ff?}
  F -- yes --> M[fast-forward]
  F -- no --> R
  M --> R[read branch tips again]
  R --> E{both reads worked<br/>and every tip is the same?}
  E -- yes --> K[keep the first pass]
  E -- no --> G[redo comparisons, caption,<br/>graph and history from the new tips]
  K --> D{--ff moved a<br/>checked-out branch?}
  G --> D
  D -- yes --> S[run git status again<br/>in that checkout only]
  D -- no --> P
  S --> P[save the comparison cache,<br/>prune stale records, print once]
```

- **Unchanged tips** (the usual case): the first pass is used as is, and nothing is measured twice.
- **A tip moved** (a fetch brought a new commit, `origin` rewound, a branch appeared or disappeared, or `--ff` moved the default branch): the comparisons, caption, default-branch target, branch tree, graph, and `-v` history are worked out once more from the second read, so everything printed describes one set of tips. The listing does not retry if tips keep moving; a change after the second read shows in the next listing.
- **A read failed**: a failed read proves nothing, so the first pass is never kept on its strength. If the second read is the one that failed, the listing shows what it can (no caption, as before) and keeps every fork record, since an empty read would make every branch look deleted.
- **Uncommitted files** do not depend on any tip, so `git status` is never repeated for a fetch. It is repeated only in the one checkout whose files `--ff` just moved; a fast-forward of a branch nobody has checked out, an up-to-date branch, or a refused fast-forward measures nothing again.
- **Saved state** (the stored comparisons, and pruning of fork records and include-copy records of deleted branches and removed worktrees) is written once, after the choice, so a discarded first pass leaves nothing behind except its comparisons, which are kept because they describe fixed commits.

#### How the wait knows both halves are done

The background process writes a small **receipt** file when both halves have finished, on every run. It records how each half ended: the head half's outcome, and whether the PR half published an answer, failed (and why), found another process already asking, was skipped for an ignored repository, or had no provider to ask. The listing reads the receipt for the attempt it launched, and tries once, as its wait ends, to delete it.

The wait does not always need the receipt, so it can end before the receipt exists: a new PR answer (see below) and a finished check are enough. For example, a listing that sees both at 0.4 s renders at once, and the background process writes its receipt a moment later. A receipt written after the wait ended, whether the wait finished early or ran out of time, stays in the cache directory until a later background process, before writing its own, deletes this repository's receipts older than 75 s. A leftover receipt is harmless: it names its own attempt, and no other listing reads it.

```mermaid
flowchart LR
    L[wt list] -->|launch, attempt id| W[background wt]
    W --> H[head half: check, fetch when origin differs]
    W --> P[PR half: ask for open PRs]
    H --> R[receipt for this attempt]
    P --> R
    L -->|polls stores and receipt| D{head outcome, and a new PR answer or the receipt?}
    D -->|yes| S[render both answers]
    D -->|process exited, no receipt| F[render what was stored; PR refresh failed]
    D -->|3 s elapsed| T[render newest stored answers plus the refresh hint]
```

The two halves are reported separately, so one never hides the other:

- **The head finished, PRs did not.** The caption shows the finished check (for example `(updated from origin just now)`), never "still checking"; the status list shows the PR item for an unfinished refresh and the refresh hint.
- **PRs published, the head did not.** The new badges show with no PR item; the caption says "still checking" (or "still pulling") and the hint follows.
- **The process exited without a receipt.** What it stored is still shown; the PR refresh counts as failed, with no guessed reason. A receipt with any field missing, of the wrong type, or repeated counts as no receipt.

The PR half publishes its answer before the receipt exists, so a new answer is recognized the moment it is stored: every stored answer carries a fresh random publication id, and a listing that sees an id other than the one stored when it started knows the answer is new, even when it arrived in the same second or lists no PRs. The same write records whether that request was sent with an API key (by variable name only) or without one, so the listing learns it with the answer, without waiting for the receipt; the keyless notice reads it from the first new answer the listing sees and from the `origin` check it followed, never from an older stored answer.

Two listings that overlap (say `wt list` in two checkouts of one repository) never ask for PRs twice at once. The second one's PR half finds the first one asking and makes no request; its listing then waits, within its own 3 s, for the first one to finish, and shows the first one's answer if a new one was published. When nothing new appears the PR refresh counts as failed; `-r` and `--ff` instead try once more, still within their 75 s. Waiting on someone else never extends a listing's budget.

A second try (for PRs here, or for a check held for another origin or branch under `-r` and `--ff`) runs both halves again, so it keeps what the first try already settled: a finished check stays in the caption, and a PR failure the first try explained keeps its reason. Only the second try's own result replaces them; a second try that cannot start, stops, runs out of time, or ends without a receipt leaves them as they were. A second try starts only while the budget lasts: when the first one's holder lets go at or after it, or still holds on, nothing more is launched and the listing ends with what it has plus the refresh hint.

### The default-branch target

Ahead/behind and merge comparisons use one **target**: whichever of the local default branch and `origin/<default>` contains the other, or `origin/<default>` when the two have diverged. Without a remote-tracking ref the local branch is the target. `origin/<default>` is as current as the last fetch, whether `wt list`'s own or yours.

### Caption

The caption is one sentence: how the local default branch compares with `origin/<default>`, then, dim and italic in parentheses, what this run learned from `origin`. When this run's check found the tracking ref already current there is nothing to add, so the sentence ends at the comparison. It needs an `origin` remote: without one, leftover `origin/*` refs produce no caption.

The comparison is `is N commits behind`, `is N commits ahead of`, `is in sync with`, or `has diverged from … (N commits ahead, M commits behind)`. Only the counts are colored. It says `local origin/main` in the two rows where this run could not bring the tracking ref up to date, so the counts may be old.

| This run | Caption |
|---|---|
| Checked; no difference | `main is 3 commits behind origin/main` |
| Origin differed; the fetch succeeded | `main is 3 commits behind origin/main (updated from origin just now)`, counted from the fetched tip |
| Origin differed; the fetch failed | `main is 3 commits behind local origin/main (origin differed when checked just now; fetch didn't finish within 60 s)` |
| Still checking at 3 s | `main is 3 commits behind origin/main (origin hasn't answered yet; still checking in the background; last checked with origin 2 h ago)` |
| Still fetching at 3 s | `main is 3 commits behind local origin/main (origin differed when checked just now; pulling remote updates in the background)` |
| Check failed | `main is 3 commits behind origin/main (couldn't check origin; last checked with origin 2 h ago)` |
| `origin` replaced or removed during the wait | `main is 3 commits behind origin/main (couldn't check origin; tracking ref last changed 5 min ago)`; see [PR badges](#pr-badges) |
| Branch absent on `origin`, tracking ref still present | `main is in sync with origin/main (main was absent on origin when checked just now; origin/main is a local tracking ref)` |

- **Reasons.** A failed check reads `origin didn't answer within 10 s`, `origin didn't accept Git's credentials`, or `couldn't check origin`; a failed fetch reads `fetch didn't finish within 60 s` or `fetch failed`. Other failures never claim that the host was unreachable or the machine offline, and Git's own error text, remote URLs, and credentials are never shown.
- **What was last known.** The still-checking and check-failed rows end with `last checked with origin <age> ago` when an earlier answer is stored. Without one they fall back to `tracking ref last changed <age> ago`, from the ref's reflog; that dates the last change to the ref, not the last fetch or check. With neither, or with a timestamp in the future, they say `never checked with origin`.
- **Missing refs.** Without a local default branch the sentence is `origin/main`, and without a tracking ref `No local tracking ref origin/main`, each followed by the row's parenthesized text when it has one; and when the branch is absent on `origin` and the tracking ref has been pruned, only `main was absent on origin when checked just now` remains. An absent branch never reads as in sync with `origin`.

Ages use the PR age units: `less than 1 min`, then minutes, hours below two days, then days.

### Credentials line

At most one dim credentials line follows the caption. It is either a **warning** about a request that failed for a confirmed credentials or rate-limit reason, or the **keyless notice** about a request that succeeded without an API key. A warning always wins:

1. a warning from the `origin` check;
2. a warning from the PR request;
3. the keyless notice, from either request.

```mermaid
flowchart TD
    A[This listing's results] --> B{origin check failed for a<br/>confirmed credentials reason?}
    B -- yes --> W1[origin check warning]
    B -- no --> C{PR request failed for a<br/>confirmed credentials reason?}
    C -- yes --> W2[PR warning]
    C -- no --> D{either request succeeded,<br/>and was sent without a key?}
    D -- yes --> N[keyless notice]
    D -- no --> E[no line]
```

#### Credentials warning

When `origin` is a supported provider and this run's API request failed for a confirmed credentials or rate-limit reason, the line is a warning. `{key}` is the variable that was used, or, when none was set, every variable the provider accepts (for example `GITHUB_TOKEN or GH_TOKEN`). Only variable names are printed, never their values.

| Condition | Line |
|---|---|
| No key set, the repository isn't visible without one, and `ls-remote` failed too | `GitHub did not show this repository, and Git could not check it. If it is private, set GITHUB_TOKEN or GH_TOKEN and try again.` |
| A key is set, but the provider didn't accept it | `GitHub didn't accept GITHUB_TOKEN; it may be invalid, expired, or revoked. Replace it and try again.` |
| A key is set, and the provider denied access | `The API key GITHUB_TOKEN doesn't have rights to view this repository on GitHub.` |
| No key set, and the provider rate limited the request | `GitHub rate limited the request for updated information. Add the GITHUB_TOKEN or GH_TOKEN API key to get larger rate limits.` |
| A key is set, and the provider still rate limited the request | `GitHub rate limited the request for updated information. Try again in a few minutes.` |

The PR request counts too, in every listing, not only under `-r`: a rejected key or a rate limit on it prints the same line. When both requests failed for a confirmed reason, the `origin` check's line wins, and the PR item's `(couldn't refresh)` does not repeat the reason.

No warning is printed when no key was set and `ls-remote` answered (the closing notice below covers that). A 404 on its own is ambiguous and produces no line. Only failures this listing observed count; a background refresh that fails after the listing rendered never adds a line to it, and neither does a failure for an `origin` replaced or removed during the wait (see [PR badges](#pr-badges)).

#### Keyless notice

When this listing saw the provider answer the `origin` check or the PR request, and that request was sent without an API key, the line says so, so a key you believed was set (but was lost from your shell, or set to an empty value) does not go unnoticed until the provider starts rate limiting:

```text
GitHub answered without an API key; set GH_TOKEN or GITHUB_TOKEN for higher rate limits.
```

The provider name and the variables come from the provider `sniff` identifies, in the order it reads them; only names are printed, never values. "For higher rate limits" is promised only where the provider documents a higher limit for a key: GitHub (github.com), GitLab (gitlab.com), and Bitbucket (bitbucket.org). Gitea and Forgejo hosts (Codeberg included) get `Gitea answered without an API key; set GITEA_TOKEN or FORGEJO_TOKEN or CODEBERG_TOKEN to authenticate API requests.` instead.

What counts is what the refresh worker recorded for the exact answer this listing accepted, never a look at your current environment: the worker that sent the request may have been started by another listing with a different environment. So:

- A key-bearing answer in one request does not hide an anonymous answer in the other, and neither does a generic failure (a timeout, an HTTP 500).
- An anonymous `origin` check still counts when the listing renders while the fetch it led to is running, or after that fetch failed or timed out.
- A PR answer counts as soon as it is published, even when the listing ends before the worker's completion receipt.
- Nothing is shown for a stored answer from an earlier run, an answer whose credentials are unknown (one written by an older `wt`), an answer that arrives after the listing chose what to show, a repository in `~/.wt.json`, an unsupported provider, a local-path `origin`, or an `origin` that changed during the wait. A request that failed without a key adds no notice either; the warnings above cover the failures that matter.

The closing fallback notice below is about a different request and may appear in the same listing.

The `Worktree` legend gains a second line only when the table needs it: `✕ git can't read this worktree` when some row shows `✕` for that reason, and `? couldn't check` when some row shows `?`. A `✕` row whose path is [a link Git still reads through](#a-link-in-place-of-the-worktree) is explained as `✕ its path is a link`, or, beside rows Git can't read, `✕ git can't read this worktree, or its path is a link`. A listing with neither keeps the two lines above:

```text
 Worktree   ○ clean    ● uncommitted files    ● uncommitted source files
            ✕ git can't read this worktree    ? couldn't check
 Branch     └─ merges cleanly into parent     └─ conflicts with parent    └┄ parent deleted
```

The table is never narrower than the legend beneath it; a table with short content widens its last column to match. In the legend the `conflicts` sample sits in the same column as the source-files dot above it.

### Columns

- **Worktree** -- a status marker and the directory name; the main checkout is `base repo`. The current worktree's name is bold and its whole row is highlighted.
    - `○` (dim): `git status` ran and found no uncommitted files
    - `●` (yellow): uncommitted files, none of them source code
    - `●` (red, the same red as `conflicts`): at least one uncommitted source file
    - `✕` (red): Git can't read this worktree, so its files were not checked, or its path is a link that may have replaced it; a [closing note](#closing-notes) says what is wrong and how to recover
    - `?` (dim): `git status` failed for a worktree Git otherwise lists normally, so its files are unknown. A `?` is never shown as clean
- **Branch** -- a tree built from the fork records `wt create` keeps (see `--from` in the [README](../../README.md)):
    - the default branch comes first, as a badge; branches forked from another branch nest under it
    - the connector (`├─`, `└─`) is gray when the branch merges cleanly into its parent and red when it conflicts
    - a parent that has no worktree of its own still gets a row, with empty status cells
    - a deleted parent is shown dim, struck through, and marked `(deleted)`; its children hang from it with dotted connectors (`├┄`, `└┄`)
    - branches without a fork record sit at the root, in worktree order; siblings are ordered by creation time, then name
    - a detached worktree shows `detached @ <sha>` and comes last
- **`-> {target}`** -- the header names the target as a remote (`origin/main`) or local (`main`) badge. The cell compares the branch with it:
    - `clean` (dim italic): it would merge without conflicts, including when it has nothing to merge
    - `conflicts` (red): it would not
    - from 100 terminal columns, `clean` and `conflicts` are followed by `+N` (dim green, commits the branch has that the target lacks) and `-N` (dim red, commits the target has that the branch lacks); a zero side is left out, so equal tips read `clean` alone. Below 100 columns only the state word shows. The width is the terminal's; `-w` / `--width` sizes only the graph
    - `—`: the default branch's own row, and detached worktrees
    - `?`: git could not answer; `wt` never shows a guessed result
- **`-> parent`** -- the same comparison, counts included, against the recorded fork parent's local branch (never its `origin/*` copy). `—` when there is no non-default parent; `parent deleted` when the recorded parent is gone.

Comparison results are cached by the pair of commit SHAs, so a warm `wt list` runs no `rev-list` or `merge-base`; see [performance-testing.md](../performance-testing.md). Uncommitted-file status is always checked live.

The Worktree marker and the comparison cells answer different questions. The marker is about the checkout's **files** (`git status`); the comparison cells are about the branch's **commits**, read from refs. So a comparison cell saying `clean` means "merges without conflicts", never "the working files were checked", and a `✕` or `?` row still shows the ordinary comparison cells for its branch:

```text
│ ○ feat-ok      │ feat/ok            │ clean +2 -1 │ —   │   files checked, none uncommitted
│ ✕ feat-gone    │ feat/gone          │ clean +2 -1 │ —   │   files not checked; the branch still merges cleanly
│ ? feat-unknown │ feat/unknown       │ clean +2 -1 │ —   │   git status failed; the branch still merges cleanly
```

#### Worktrees Git can't read

A linked worktree can lose its connection to the repository while Git still records it; `git worktree list --porcelain` then marks it `prunable`. `wt list` never runs `git status` for such an entry (it could fail, or report an enclosing repository's files as this one's) and never repairs or removes anything. It looks at the recorded path instead and shows `✕` with one of three explanations:

| What `wt` finds on disk | Called | Note |
|---|---|---|
| The recorded directory does not exist | missing | `its directory is gone; wt remove <name> checks whether its remaining Git record can be removed safely.` |
| The directory exists but its `.git` file does not | unlinked | `its .git file is missing; wt remove <name> removes it.` |
| Anything else: a file or a link at the path, a path or `.git` that can't be inspected, or a `.git` that exists but Git can't use | other | `Git can't read this worktree: <what was found>; Git reports: <Git's reason>.` |

"Does not exist" means the operating system answered "not found". A permission or I/O error is never read as absence; it is the "other" case, with the system's message. See [`wt remove`](remove.md) for what removing each kind does.

#### A link in place of the worktree

A worktree directory moved away and replaced by a link to its new location (a symbolic link, or on Windows any reparse point such as a junction) is not marked `prunable`: Git reads the checkout through the link. `wt list` looks at every recorded path without following it, shows such a row as `✕`, and notes `<name>: <path> is a link, which may have replaced the original checkout.` `wt remove` refuses it ([why](remove.md#a-link-in-place-of-the-worktree)). Only the path itself counts; a link among its parent directories, such as macOS's `/tmp`, is not reported.

### PR badges

Every listing asks `origin` for its open pull requests, whatever the age of the last answer, and shows the answer that arrives within its wait. Open pull requests whose source is this repository show as a green `PR #n` badge:

- in the `-> {target}` cell when the PR targets the default branch, after any counts
- in the `-> parent` cell when it targets the branch's fork parent
- beside the branch name, as `PR #n → <target>`, otherwise

A PR from a fork with a same-named branch is never shown. In terminals that support OSC 8 hyperlinks the badge links to the PR; elsewhere it shows the number only, with no visible URL, so the table always fits.

The request is made only by the background process's PR half (see [Checking origin](#checking-origin)), never by `wt list` itself, with a 10 s deadline. It asks for the complete list, which can take more than one HTTP request on some providers, and stores the answer only when it is complete and `origin` has not changed meanwhile. A failure is never stored, so the last good answer survives it. Each answer is stored with the `origin` it came from; an answer for a different `origin` is never shown. An empty answer is an answer: it clears the badges.

What the listing shows depends on how this run's PR half ended:

| This run's PR half | Badges | Status item |
|---|---|---|
| published an answer within the wait (its own, or one another listing published) | the new answer | none |
| finished but published nothing, or its result could not be observed | the last stored answer | `- PRs as of <age> ago (couldn't refresh)`, at any age |
| the same, with nothing stored | none | `- couldn't get open PRs` |
| still running at 3 s | the last stored answer | `- PRs as of <age> ago` when that answer is 60 s old or more; the refresh hint follows either way |
| repository listed in `~/.wt.json` (see `--ignore-api`) | none | none |
| `origin` is a local path or a host with no supported provider | none | none |

For example, a refresh that fails right after a good answer from 10 s ago keeps that answer's badges and reads `- PRs as of less than 1 min ago (couldn't refresh)`. A local-path or unsupported `origin` is not a failure: it simply has no provider to ask, and its head check still runs through Git.

If `origin` is replaced or removed during the wait, this listing says nothing about the requests made for the old `origin`. Every request notice is dropped: the PR badges, the PR item and its failure reason, the credentials warning from the `origin` check or the PR request, the keyless notice, and the closing fallback notice. The caption reads as a check that could not be made, since the check it ran was for another repository; its date comes from the tracking ref's reflog, not from the old `origin`'s stored answer. No second refresh starts.

```text
main is 3 commits behind origin/main (couldn't check origin; tracking ref last changed less than 1 min ago)
```

### Git Graph

When stderr is a terminal and `TERM_PROGRAM` (or `KITTY_WINDOW_ID`) names an image-capable terminal (Kitty, iTerm2, Ghostty, WezTerm, Warp, Konsole), a branch graph is drawn below the table. It is not drawn for a detached current worktree.

- **A feature branch is checked out**: the current branch forked from the default branch (or from its recorded parent branch, which then gets a line of its own), and the default branch down to just below the oldest fork or merge.
- **The default branch is checked out**: the default branch's 10 newest commits, and a line for every worktree branch.

Every line is its branch's first-parent history, so a merged branch's commits never appear on the default branch's line:

- a branch merged with a merge commit keeps its own line, drawn merging into the branch it was merged into, at the merge commit;
- a branch that got new commits after being merged stays one connected line: it merges at the merge commit and continues after it, and a branch still at the merged commit is a tag on that line;
- a branch with no commits of its own (fast-forwarded, or created and not yet committed to) is a tag on its commit;
- forks and merges older than the drawn commits stay visible, with the commits between them folded into `+N` squares.

Each line shows its 5 newest commits; older ones fold into a `+N` square. Open PRs appear as `PR #n → target` tags. When `origin/<default>` has diverged from the local default branch it gets a line of its own.

When local history cannot establish a connection (in a shallow clone, for instance), or something the graph should show cannot be drawn at its own commit, the graph shows what it could verify, and the closing notes say so. In a shallow clone the note tells you how to fill in the history (`This clone is shallow, so the graph can't connect some older history; run git fetch --unshallow to fill it in.`); otherwise it says only `The graph leaves out a fork, merge, or tag it couldn't draw at its own commit; the table above is unaffected.` and there is nothing to run. `wt list` never fetches more history itself. A branch whose commits reached the default branch only through another branch's merge is not missing anything; a note says where they are, for example `feat/schema-enhancement's commits are all in origin/main (merged through fix/wt-skill).`

There is no minimum terminal width: the graph is sized from its natural width and the terminal's cell size, trims commits into `+N` squares to fit, and shrinks only when even the trimmed graph is too wide. In the base view a graph taller than half the terminal keeps the most recently active lines, and the closing notes say how many were left out (`3 worktrees aren't in the graph: it shows the most recently active ones that fit the terminal.`). See [git-graph.md](../git-graph.md) for the design.

`-w` / `--width` sets the graph's width directly (`70`, `70ch`, or `50%` of the terminal); the graph is then never trimmed to fit.

### Status list

A Markdown-style list of dim items follows the graph (or the legend when no graph is drawn), before the verbose section. It is left out when it has no items:

- this run's PR item, when its PR refresh failed or was still running with an answer at least 60 seconds old (see the table under [PR badges](#pr-badges))
- the refresh hint, when the wait ran out before both halves finished

A failed refresh adds the hint only when the wait also ran out (the head half was still running). A refresh still running at 3 s:

```text
- PRs as of 12 min ago
- running this command again will provide updated metrics; alternatively use the --refresh / -r flags to force refresh immediately
```

### Verbose Mode

With `-v` / `--verbose`, and whenever the current branch is not the default branch (including when it is checked out in the base repo), two sections follow the graph:

- **Default branch section** -- the commit the branch forked from, formatted with SHA, conventional commit type/scope, timestamp, and any refs
- **Feature branch section** -- every commit on the branch that neither the local default branch nor `origin/<default>` has, oldest first, using the same format

Commits are formatted as conventional commits when possible (e.g. `feat(scope): description`), with fallback to a truncated first line for non-conventional messages.

### Closing notes

After a blank line, the output can end with up to five kinds of note, in this order. A command a note tells you to run (`wt --ff`, `--ignore-api`) is shown in reverse video, so it stands out as something to type. The examples below are plain text.

- **Fast-forward suggestion.** When the local default branch is strictly behind `origin/<default>` (not diverged):

    ```text
    - main is 3 commits behind origin/main; run wt --ff to fast-forward it.
    ```

    It is not shown when the listing rendered while origin was still being checked or pulled, or under `--ff`, which reports its own result instead: nothing when it moved the branch or there was nothing to do, otherwise the reason it did not, for example (`main has diverged from origin/main, so it can't be fast-forwarded.`, `main wasn't fast-forwarded: the checkout has uncommitted changes to files the update touches.`, or `main wasn't fast-forwarded: origin/main doesn't exist.`). A failed check or fetch still shows it, since `--ff` moves to the local tracking ref; the caption keeps the failure reason, so the target may itself be out of date.
- **Fallback notice.** When no API key was set, the provider would not show the repository without one, and `ls-remote` answered instead:

    ```text
    - Git checked origin using `ls-remote`; this can take longer than the provider API.
    - Set GITHUB_TOKEN or GH_TOKEN to let wt try the provider API, or use --ignore-api to use Git directly for this repository.
    ```

    A rate-limit fallback never produces it, and neither does a check made for an `origin` that was replaced or removed during the wait.
- **About the graph.** Shown only when a graph was drawn; the graph image itself carries no text. Lanes left out, then history it couldn't connect (one of the two notes below, the shallow one when the clone is shallow), then one note per branch merged through another branch:

    ```text
    - 3 worktrees aren't in the graph: it shows the most recently active ones that fit the terminal.
    - This clone is shallow, so the graph can't connect some older history; run git fetch --unshallow to fill it in.
    - The graph leaves out a fork, merge, or tag it couldn't draw at its own commit; the table above is unaffected.
    - feat/schema-enhancement's commits are all in origin/main (merged through fix/wt-skill).
    ```

    The last names the lane by what you know it as (`origin/main` when it is ahead of `main`), and the carrying branch only when it is drawn; otherwise it says `merged through another branch`.
- **Unavailable worktrees.** One dim note per `✕` row, in table order (a `?` row gets none), as described under [Worktrees Git can't read](#worktrees-git-cant-read):

    ```text
    - feat-gone: its directory is gone; wt remove feat/gone checks whether its remaining Git record can be removed safely.
    - lhg-before: its .git file is missing; wt remove lhg-before removes it.
    ```

    The suggested name is one that `wt remove` resolves to exactly this worktree: its branch, else its directory name. When neither does (another worktree matches the same name, for example), the note says `wt remove` without a name and explains why: `No name selects it for wt remove: feat also matches /code/wts/other.` Names and paths that need it are single-quoted (`wt remove 'my work'`). A value that can't be spelled the same way in bash, zsh, fish, and PowerShell (a leading `-` or `~`, a single quote, two backslashes in a row or a trailing one, a control character) is never offered as something to type: the note names the command without it and says why instead, for example `wt remove removes it. No name selects it for wt remove: it's can't be typed the same way in every shell.` Nothing in a note is ever executed. Names, paths, Git's reasons, and error messages are shown exactly as recorded, whatever characters they hold: text that looks like styling, such as `<red>`, is printed as is.

    A path or command in these notes, and the path in the `--ff` refusal for a worktree Git can't read, is never broken by the wrap, so what you copy is what `wt` means. It moves to the next line whole, and one too long for any line gets a line of its own that your terminal wraps, without an added `-`. In a 60-column terminal:

    ```text
    - wt-gone: its directory is gone;
      wt remove feature/a-rather-long-branch-name  checks
      whether its remaining Git record can be removed safely.
    ```

## Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--width <WIDTH>` | `-w` | Set the graph width (e.g. `70`, `70ch`, `50%`) and turn off trimming |
| `--verbose` | `-v` | Show the commit history of the current branch |
| `--refresh` | `-r` | Wait for the full check, fetch, and PR refresh, up to 75 s instead of 3 s |
| `--ignore-api` | | Check `origin` with Git only, never the provider API, for this repository from now on |
| `--fast-forward` | `--ff` | Wait like `--refresh`, then fast-forward the local default branch to `origin/<default>` |
| `--perf` | | Print a per-stage timing report to stderr; work that overlaps the remote wait is one measured group ([details](../performance-testing.md#runtime---perf-flag)) |

`-r`, `--ignore-api`, and `--ff` are global, so `wt -r` and `wt list -r` are the same. They apply only to listing: `wt create`, `wt go`, and `wt remove` reject them with exit code 2. They can be combined; `wt --ff -r` makes one check, at most one fetch, and one fast-forward.

### `--refresh` / `-r`

Waits for both the `origin` check (with any fetch) and the PR refresh to finish, up to 75 s (the 10 s check and 60 s fetch caps plus a short allowance) instead of 3 s. Every listing asks both questions anyway; `-r` only waits longer for the answers. When a check is already running for the same `origin` and branch, `-r` follows it; one for another branch is waited out and then a fresh one starts. When another listing was already asking for PRs and published nothing, `-r` asks once more. A failure is reported in the caption or credentials line as usual and does not fail the listing.

### `--ignore-api`

Records the current repository in `~/.wt.json` before the check starts, so this run and every later one checks `origin` with `git ls-remote` only. Neither the check nor the PR refresh makes a provider request for that repository, so it shows no PR badges, no credentials line, and no fallback notice. It fails (exit code 1, nothing listed) when there is no `origin`, when `origin` is a local path, or when the home directory is unknown.

The file lives in the home directory (`%USERPROFILE%` on native Windows), separate from `~/.worktree.json`:

```json
{
  "format_version": 1,
  "ignore_api": [
    { "host": "github.com", "port": 443, "path": "owner/repo" }
  ]
}
```

A repository is identified by its lowercase host, effective port, and path, never by the raw URL, and user names and credentials are dropped. HTTPS and SSH remotes of the same provider repository share one entry (SSH to a known provider host counts as port 443); different ports or paths never do. The file is written atomically under a lock, so concurrent `wt` processes keep each other's entries. A missing file ignores nothing; an unreadable or corrupt one is treated as empty for listing, but `--ignore-api` reports an error rather than overwrite it. To undo the choice, remove the entry or the file.

### `--fast-forward` / `--ff`

Waits for the check like `--refresh`, then moves the local default branch to `origin/<default>` when that is a fast-forward, and lists the result:

- when no worktree has the default branch checked out, it updates the branch with a compare-and-swap (`git update-ref`), so a concurrent change makes it refuse
- when a worktree has it checked out, it runs `git merge --ff-only` there, which refuses rather than overwrite local changes to files the update touches
- a diverged branch, or a missing local branch or tracking ref, is refused with a note and nothing is created or changed; in sync or ahead does nothing and prints nothing
- when the worktree holding the default branch is one Git can't read (`✕`), the move is refused rather than made underneath it: `main wasn't fast-forwarded: Git can't read the worktree that has it checked out, /code/wts/main-copy.` `--ff` never repairs that worktree. When the branch is already in sync, there is nothing to refuse and no note

Both refs and their ancestry are read again immediately before the move. The move happens after the listing's local work is done, and a checkout whose files it moved gets its uncommitted-file status measured again; see [Local work during the wait](#local-work-during-the-wait). When the check or fetch failed, `--ff` still fast-forwards to the local `origin/<default>` if that is ahead, and the caption keeps the failure reason, so you know the target may itself be out of date. A refusal does not fail the listing.

## Examples

```bash
wt              # List worktrees (default command)
wt list         # Explicit list
wt -v           # List with verbose commit details
wt -w 100       # List with graph forced to 100 characters wide
wt -w 50%       # List with graph at 50% of terminal width
wt -v -w 120    # Verbose output with wider graph
wt -r           # List after a full update from origin
wt --ff         # Fast-forward the default branch to origin, then list
wt --ignore-api # Check this repository with git ls-remote only, from now on
```
