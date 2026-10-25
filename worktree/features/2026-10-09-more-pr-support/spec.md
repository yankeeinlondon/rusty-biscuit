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
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-09
review_iterations: 0
clarified: false
implemented: false
human_review: false
message_to_agent: |-
    Phase 2 is done: OpenPullRequest::status (sniff's PrCheckStatus, re-exported
    as worktree::pull_requests::PrCheckStatus) is stored in format 7, with
    status_name() giving failing|running|passed|unknown. The store may hold two
    records with the same PR number, so Phase 3's coverage code must dedupe by
    number. Struct literals of OpenPullRequest now need a status field. Also
    still open from Phase 1: confirm the GraphQL count-field names against
    GitHub's reference before the Phase 4 docs.
related:
    - 2026-10-02-fresh-prs
---

# Account for open PRs in `wt list`, and show their check status

## Problem

`wt list` fetches the complete open pull request (PR) list for `origin`,
within the provider's existing page and time limits. It shows table badges
only when a worktree's branch has the same name as a PR's source branch in
this repository. Two cases lose information:

1. **A PR with no matching worktree has no table badge.** On 2026-10-09
   PR #121 (`fix/windows-mega-fix` → `main`) was open and stored, and
   `wt list` showed nothing for it. The branch had been pushed from the
   `feat-modern-config` worktree under a different remote name, so no
   worktree branch matched it. The same happens for a PR opened from another
   host or from a branch whose worktree was removed.
2. **A badge says a PR exists, not whether its checks need attention.**
   Every table badge has the same green background, so failing checks look
   like successful checks.

## Goals and existing contracts

- Account for every open **same-repository** PR in the accepted answer: in a
  table badge, an unmatched-PR item, or the explicit overflow count.
- Each table badge and unmatched-PR marker distinguishes failing, running,
  passed, and unknown checks. This describes reported checks, not permission
  to merge or overall PR readiness.
- Keep the existing detached worker, credential selection, complete-list
  requirement, origin binding, and wait budgets: 3 s normally and 75 s for
  `--refresh` or `--ff`. The PR half's request deadline remains 10 s.
- Preserve fork exclusion and existing graph behavior. Graph PR tags already
  exist; this feature neither adds graph lines nor changes those tags.
- Work on macOS, Linux, native Windows, and WSL2 through the existing
  provider and terminal abstractions.

> **Reader's note:** The original promise that every open PR appears
> individually conflicted with both fork exclusion and the ten-item limit.
> This draft retains those deliberate scope choices and promises explicit
> accounting instead. The Open questions section asks whether to remove
> the display limit if individual visibility is the stronger requirement.

## Design

### 1. Unmatched PRs as status-list items

An eligible PR that no worktree table row displays gets one item in the
status list beneath the graph (or beneath the table and legend when there
is no graph):

```text
 - ✗ PR #121 fix/windows-mega-fix → main  (no worktree row)
```

- **Eligibility.** Use the same source-repository comparison as existing
  table badges: both repository identities must be present and equal under
  the existing comparison. Fork PRs and PRs with unknown source identity
  remain excluded, even when a fork branch shares a local branch's name.
- **Displayed means placed on a worktree row.** Preserve all three badge
  locations: the default-target comparison cell, parent comparison cell,
  and beside the branch name. Collect the PR numbers those row rules place;
  do not treat a branch ref, a parent-only row, or a graph tag as a worktree
  badge. A PR with only a local branch ref still gets an unmatched item.
- **Multiple matches.** Preserve multiple PRs on one branch and their
  existing placement by target branch. Each PR number appears at most once
  in the unmatched list, even if duplicate records occur in the stored
  answer. A PR already placed on any worktree row is never repeated there.
- **Order and volume.** Sort unmatched PRs by ascending number, then show
  the first 10. The named limit applies only to unmatched items. If there
  are more, add `and N more open PRs without a worktree row`, where `N` is
  the number of distinct eligible unmatched PRs omitted. For example, 12
  unmatched PRs produce 10 items and an overflow count of 2. Table badges
  and excluded fork PRs never enter that count.
- **Status-list order.** The existing PR age/failure item comes first,
  followed by unmatched items and their overflow count, followed by the
  existing refresh hint. An unmatched PR makes this list nonempty even when
  neither existing item is needed.
- **Link.** Link the PR marker only when the stored provider URL is present
  and OSC 8 is supported. Otherwise show the marker without a visible URL,
  as table badges do today. Branch names remain visible in both cases.
- **Rendering.** Use biscuit-terminal's existing `Prose`, `UnorderedList`,
  and `TerminalRenderable` path, including escaping branch names and other
  provider text before treating them as markup. Account for the wrapped
  items when reserving height for the graph. Keep all listing output on
  stderr and preserve the shell wrapper's stdout contract.
- **Empty or unavailable answer.** An empty successful answer clears these
  items and the badges. An ignored API, unsupported origin, or cache miss
  shows neither. A failed or pending refresh with a usable old answer keeps
  both, with the existing age/failure item and refresh hint. Changing or
  removing origin during the wait drops the old answer and its notices.
- **Label.** `(no worktree row)` describes the display, including a branch
  that still exists without a usable worktree row. It suggests no action;
  removing a worktree is a normal way to reach this state.

Matching remains by source repository and exact branch name, not commit
ancestry. A PR whose differently named branch contains a worktree's `HEAD`
(the #121 case) remains unmatched. The provider may use a head SHA internally
to validate status evidence; this does not introduce local ancestry checks.

No new table row or graph line is created. Existing graph tags do not count
as table coverage and remain unchanged, including tags on a drawn local
branch without a worktree.

### 2. Check status on the marker

Use distinct glyphs **with and without color**, so color is supplementary:

| Marker prefix | Optional color | Stored status | Meaning |
|---|---|---|---|
| `✗` | red | `failing` | at least one reported check needs attention |
| `…` | yellow | `running` | checks are pending or running, with no reported failure |
| `✓` | green | `passed` | at least one check reported, all results acceptable |
| none | neutral | `unknown` | no checks reported, or usable status evidence unavailable |

Use the same marker helper for table badges and unmatched items. For
example, `✗ PR #121` retains its meaning with `NO_COLOR`, when stderr is
captured, and for readers who cannot distinguish the colors. Extend the
existing legend with these meanings when visible PR markers are present;
unknown means a bare `PR #n`. Keep the glyphs distinct from the circles that
already indicate the current worktree.

> **Note (2026-10-23).** The Worktree column's legend now also has `✕`
> (red: Git can't read the worktree) and `⊘` (yellow: being removed). In
> many fonts, and always without color, `✗` is hard to tell apart from
> `✕`. Phase 3 should confirm the failing glyph is distinguishable from both
> before it lands, or choose another (R8).

Replace the green table-badge background with one shared neutral badge
style in the worktree-cli renderer. Give the style a name beside the shared
marker helper rather than introducing a theme system. Its foreground and
background must remain legible on light and dark terminals and degrade
through biscuit-terminal's capability handling. The exact neutral fill is
an author decision below.

#### GitHub mapping

Status covers GitHub's reported checks and legacy commit statuses on the
PR's latest source commit. It does not inspect branch protection, reviews,
merge conflicts, or the repository's merge gate. Checks that run only on a
synthetic merge commit can therefore be absent; see Open questions.

Aggregate the count-by-state fields of the commit's `statusCheckRollup`,
rather than fetching individual check records or trusting only its single
combined `state`. Those fields distinguish canceled checks and include both
check runs and commit statuses without walking pages of individual checks.
GitHub documents these fields in its [commit rollup reference](https://docs.github.com/en/graphql/reference/commits#statuscheckrollupcontextconnection)
and the states in its [checks reference](https://docs.github.com/en/graphql/reference/checks#checkrunstate).

| Evidence type | Counts as failing | Counts as running | Acceptable completed result |
|---|---|---|---|
| Check run | `FAILURE`, `TIMED_OUT`, `CANCELLED`, `ACTION_REQUIRED`, `STARTUP_FAILURE`, `STALE` | `QUEUED`, `IN_PROGRESS`, `PENDING`, `WAITING` | `SUCCESS`, `SKIPPED`, `NEUTRAL` |
| Commit status | `ERROR`, `FAILURE` | `EXPECTED`, `PENDING` | `SUCCESS` |

Only positive counts contribute. Failure wins over running. Running wins
when no failure is known. Green requires a nonzero total, complete and
consistent counts, and only acceptable results. A new/unrecognized state,
an ambiguous `COMPLETED` without a conclusion, missing counts, or malformed
counts prevents green; with no known failure or running state it is unknown.
A null rollup or zero contexts is unknown. Validate totals and counts before
using them; unusable evidence is unknown, never an invented failure.

> **Reader's note:** Cancellation is no longer treated as success. A canceled
> run did not prove the checks passed, and this repository also treats
> canceled blocking jobs as failures to satisfy its merge gate. Skipped and
> neutral results remain acceptable, but green means only that the reported
> results are acceptable. It does not prove that required coverage exists or
> that a neutral check permits merging.

Draft PRs use the same rules and remain eligible. A draft with no checks is
unknown. This feature adds no draft marker or readiness inference.

### 3. Fetching and publishing status

Status belongs to the PR half's answer. Only the detached worker writes it,
under the existing PR lock, request deadline, atomic-publication rules, and
origin recheck. The foreground listing makes no provider requests.

#### Integration and request cost

The sniff library's [open_pull_requests lookup](../../../sniff/lib/src/remote/blocking.rs)
provides complete PR summaries and evidence of which credentials its
requests actually sent. Add a typed `status` field to the sniff library's
[PrSummary](../../../sniff/lib/src/remote/blocking.rs), defaulting to unknown
for providers without support. Implement the enrichment in sniff's
[FocusedProviderClient](../../../sniff/lib/src/remote/focused.rs), the client
this blocking lookup actually uses, rather than only in the separate
schematic-backed GitHub provider adapter. Carry the normalized status into
the worktree library's [OpenPullRequest](../../lib/src/pull_requests.rs),
which is the stored record used for matching and rendering. Keep ordinary
PR queries from acquiring status requests they did not ask for.

1. Fetch the complete PR list through the existing REST listing path and
   existing page bound. An incomplete list is still a failed PR refresh,
   never a publishable partial or empty answer.
2. For GitHub with a token selected by sniff and a nonempty list, enrich the
   summaries with batched GraphQL requests. Each batch looks up at most 100
   PR numbers using aliases for `pullRequest(number: ...)`, requesting the
   latest commit and rollup counts. Use variables for repository identity
   and PR numbers; never interpolate provider text into a query.
3. Compare the returned commit SHA with the head SHA from the REST record
   before accepting status. Missing or changed heads, closed PRs, absent
   records, or unusable status fields leave that PR unknown. No local Git
   request is needed, and the SHA need not become a worktree-store field.

This intentionally preserves REST as the source of the complete list, so
status failures cannot hide PRs. For `N` open PRs the successful keyed path
adds at most `ceil(N / 100)` GraphQL requests, not one HTTP request per PR;
the first batch serves a small repository in one additional request. The
anonymous path adds none. Do not enrich an empty list. A page-based GraphQL
list could instead need multiple requests: GitHub's [pagination guide](https://docs.github.com/en/graphql/guides/using-pagination-in-the-graphql-api)
limits each connection page to 100 items. There is no existing universal
one-request-per-repository contract to preserve.

#### Failure, deadline, and credential behavior

- **Optional enrichment.** No token, another provider, or unavailable
  GraphQL support yields unknown status without failing the PR list.
  Retain the complete REST answer on an enrichment timeout, HTTP error,
  rate limit, denied permission, or malformed GraphQL response. A GraphQL
  HTTP 200 response can still contain errors: affected PRs become unknown;
  independently valid records may retain their status. On a request-wide
  failure, stop further enrichment rather than retrying each PR.
- **One deadline.** REST pages and enrichment share the PR half's existing
  10 s deadline. Enrichment must not reset it or cause the complete REST
  answer to be discarded by the outer timeout. Cancel unfinished enrichment
  and return that list with unresolved statuses unknown within the budget.
  Preserve successful batches; never borrow statuses from an older answer.
- **Credential ownership.** Worktree reads no tokens. GraphQL uses sniff's
  same selection, host policy, approved endpoint, redirect restrictions,
  and per-request credential evidence as REST. Tokens and raw credential
  values never enter stores or output. Test with an injected API endpoint.
  For a self-hosted GitHub origin, use its validated GraphQL endpoint or
  return unknown; never send its token to github.com as a fallback.
- **Notices.** Optional status failure does not become a whole-PR refresh
  failure, add a credentials warning, or replace the existing warning
  precedence. A bare marker states that its checks are unknown. The GitHub
  keyless notice adds `An API key is also required to fetch PR check
  statuses.` only when an anonymous PR answer from this run was accepted,
  using worker-recorded request evidence. An anonymous head check alone
  does not justify that clause. Other providers keep their existing text.
  A token enables a status attempt; it does not guarantee permission to
  read checks.

#### Store compatibility and freshness

Bump the PR store from format 6 to 7; each PR has a string `status` member:
`failing`, `running`, `passed`, or `unknown`. Write all fields explicitly.
Read formats 5 **and** 6 as before, assigning unknown status even if an
unexpected status field is present. Format 5 still has unknown credential
evidence; format 6 still requires valid credentials. Formats 4 and older,
and 8 and newer, remain misses.

For format 7 only, a missing, null, wrongly typed, or unrecognized status
value becomes unknown for that PR. This is a deliberate exception for an
optional annotation, not a relaxation of the rest of the store: malformed
JSON, duplicate member keys (including duplicate `status`), invalid PR
identity fields, publication, origin digest, timestamp, or credentials
still make the publication unusable. Preserve the current handling of
unknown extra fields. A status error must not drop a valid PR identity.

Publish the complete list and its statuses in one atomic write, with the
existing publication ID and timestamp semantics. A successful empty list
still publishes. Enrichment failure may publish a complete list with unknown
statuses; failure to fetch the complete list publishes nothing. A later
successful answer replaces old statuses, even when the PR numbers are the
same, and origin changes suppress the whole old answer.

There is no separate status cadence or timestamp: each ordinary listing
already launches the worker. Markers reflect the accepted publication,
which may be old while refresh fails or runs beyond the wait. The existing
PR age/failure item applies to checks as well as PR existence; document that
relationship explicitly. Do not claim that cached green is a current check.

## Out of scope

- New graph lines or changes to existing graph PR tags.
- Showing fork PRs or matching by commit ancestry.
- Individual check details, draft markers, reviews, branch-protection rules,
  merge queues, synthetic merge-commit checks, or deciding merge readiness.
- Check status for providers other than GitHub.
- A new refresh cadence, foreground request, or longer wait budget.

## Documentation to update during implementation

- Worktree `docs/cli/list.md`: scope, markers, legend, neutral style, exact
  status rules, unmatched items and overflow, keyless clause, and cached
  status age. Clarify that the graph retains its existing tags.
- Worktree `README.md`: PR summary and displayed check status.
- Sniff `docs/cli/repo_remote.md`: the open-PR summary's optional status
  enrichment, head-commit scope, cost, and failure behavior. Update public
  API comments and the affected sniff topic documentation too.
- The worktree skill's `list.md` and `list-remote.md`: displayed-PR
  accounting, marker rendering, format 7, legacy readers, and optional
  enrichment. The sniff skill's `remote-and-repository.md`: the focused
  lookup's status and request-evidence contract.
- Each affected area's `docs/dependencies.md` and the root dependency page
  only if crates are added or removed. No new crate is required by design.

## Acceptance and validation

Use the existing stand-in servers and saved response fixtures; tests never
contact a live provider. Run affected L1 tests through `just test` in the
worktree and sniff package areas and lint through their existing recipes.
Use existing integration coverage where it exercises the worker/store path;
any terminal coverage must keep windows from gaining focus.

- A #121-style same-repository PR has an unmatched item, no new table row,
  and no new graph line. Parent-only refs, detached worktrees, and worktrees
  without a usable branch row do not silently count as table badges.
- Exercise all three table placements, several PRs on one branch with
  different targets, fork/unknown-source exclusion, duplicate PR numbers,
  and a PR already tagged on an existing graph line. Unmatched items follow
  the actual table coverage and remain in stable number order.
- Cover 0, 1, 10, and 11 unmatched PRs, including an unrelated table badge
  and an excluded fork, to verify the exact overflow count and list order.
  Long and markup-like branch names wrap and escape safely. Test OSC 8 on
  and off, missing URLs, graph height reservation, light/dark rendering,
  and color/no-color markers. Shared-module snapshots belong in integration
  tests because worktree-cli compiles its shared modules twice.
- Map failure plus running, cancellation, timeout, action required,
  startup failure, stale, queued/waiting, legacy expected/pending, success,
  neutral/skipped only, mixed acceptable results, zero checks, null rollup,
  unrecognized state, ambiguous completion, and malformed counts. None of
  the unknown cases may become green.
- Provider fixtures cover changed/missing head SHA, partial GraphQL errors,
  HTTP 200 with request-wide errors, permission failure, rate limit,
  missing/empty tokens, empty PR lists, and a deadline reached after a
  complete REST list. Status failure keeps the list; incomplete REST does
  not publish. Preserve the same credential selection and host restrictions.
- Verify one enrichment batch for 1–100 PRs and two for 101, with no
  per-PR requests and no request for anonymous, unsupported, or empty
  answers. Request-wide failures stop subsequent batches.
- Formats 5 and 6 keep their existing credential rules and show unknown
  statuses; format 7 round-trips. Cover absent/null/invalid status values,
  duplicate member keys, malformed identity fields, and unsupported formats.
- Through the worker/store path, test successful status replacement,
  successful empty publication, stale-answer fallback, API suppression,
  origin changes, and publication-ID recognition without waiting for a
  receipt. The keyless status clause requires accepted anonymous PR
  evidence and respects existing warning precedence.
- Preserve the 3 s ordinary wait, 75 s forced wait, 10 s PR deadline, lock
  contention behavior, and independent live-head half. Reuse existing
  request counters and foreground Git-call assertions to verify that status
  adds no foreground request or Git process. No new performance spike,
  host matrix, or statistical benchmark is required for this feature.

## Open questions

### Should every unmatched PR be named individually?

The retained ten-item cap accounts for all eligible PRs but hides the branch
names and statuses of those represented by the overflow count.

- **Keep 10 plus an exact overflow count (recommended).** Pros: preserves
  the draft's compact listing and predictable extra height. Cons: the
  eleventh PR's identity and status require opening the provider's PR list.
  Recommended because this is the existing draft's explicit volume policy
  and the graph already shares a limited terminal-height budget.
- **Show every unmatched PR.** Pros: fully satisfies individual visibility
  and exposes every status. Cons: repositories with many PRs scroll the
  listing and reduce the graph's available height. Choose this if the
  original individual-visibility goal outweighs compact output.
- **Keep the cap and add an explicit show-all option.** Pros: compact default
  with complete detail on demand. Cons: adds a CLI option and documentation
  for a need not yet established. This would expand the current scope.

### Which neutral badge style should be used?

The current table badge has a green fill and white text. Its replacement
must not imply that checks passed or make colored glyphs hard to read.

- **A neutral fill with contrasting text (recommended).** Pros: retains the
  familiar badge shape and requires only a shared renderer style. Cons:
  foregrounds need checking against the fill in light and dark terminals.
  Recommended for the smallest change to the existing component usage.
- **Default background with a simple neutral text outline.** Pros: follows
  the terminal's background and does not paint a band. Cons: adds characters
  to table cells and changes their visual density.

### Is latest-source-commit status sufficient?

The design reports checks on the PR's latest source commit. Some workflows
run checks on a synthetic merge commit instead, so this can legitimately
show unknown while the provider's PR page has check results.

- **Keep latest source commit only (recommended for this feature).** Pros:
  simple, bounded evidence tied to an exact commit; avoids claiming merge
  readiness. Cons: misses checks attached only to synthetic merge commits.
  Recommended because this feature is an informational summary, and the
  marker's documented scope prevents a green badge from becoming a gate.
- **Include source and synthetic merge commits.** Pros: closer to the PR
  page for merge-based workflows. Cons: needs rules for changing merge
  commits, duplicate checks, and conflicting results; broadens fetching and
  stored evidence. Decide those rules before expanding the scope.
- **Report only checks required by branch protection.** Pros: focuses on
  checks that matter to merging. Cons: requires protection/ruleset data and
  additional permissions, and omits useful nonblocking failures. This
  changes the goal from check summary to merge-readiness evaluation.
