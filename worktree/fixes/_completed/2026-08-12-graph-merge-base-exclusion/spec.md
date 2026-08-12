---
status: draft
created: 2026-08-12
area: worktree
packages:
  - worktree-cli
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-08-12
review_iterations: 1
---

# Git Graph Lanes Must Exclude the Opposite Tip, Not the Merge-Base

## Summary

The `wt` git graph draws each lane (default branch and worktree branch) as
"commits reachable from that tip but not from one selected merge-base". That
exclusion is equivalent to "that side's unique commits" only when the selected
merge-base dominates all history common to the two tips. In a criss-cross
history there can be multiple incomparable best merge-bases, and plain
`git merge-base A B` returns only one of them. History reachable through the
other best base is then not excluded and leaks into **both** lanes:

- the same trunk commits are drawn as dots on both the default lane and the
  branch lane,
- the fork-adjacent `+N` elision squares report the entire trunk depth
  (four digits) instead of the handful of actually-elided commits,
- the graph wildly contradicts the `Commits` column of the `wt list` table,
  which computes ahead/behind symmetrically and is correct.

The verbose path (`wt list -v`) shares the same exclusion, so its per-branch
commit detail list is inflated identically.

## Reported Case

Observed in `rusty-biscuit` on 2026-08-12, running `wt` from the
`fix-ctx-launch-anchor` worktree. The table row for `fix/ctx-launch-anchor`
correctly reported `+4 -1`, while the rendered graph showed:

- main lane: `7c83083`, `ee8658f`, `+1257` elision, `f6a93d2`, `24c1f2e`,
  `cbe6c3c`, `22814e9`, `47dfded`
- branch lane: `+1260` elision, `24c1f2e`, `cbe6c3c`, `22814e9`, `0de04a0`,
  `e20c558`

`24c1f2e`, `cbe6c3c`, and `22814e9` are main-trunk commits drawn on **both**
lanes, and the elision squares claim ~1,260 commits per lane on a branch that
is 4 ahead / 1 behind.

### Topology that triggers it

```text
terminal:   7c83083 ── ee8658f ──────────────┐──────────────┐
                                             │              │
main:  ... 24c1f2e ── cbe6c3c ── 22814e9 ── 47dfded (merge terminal)
                                    │        second parent: ee8658f
research:              ... ── 0de04a0 (merge main)
                                    │
branch:                        e20c558 (merge terminal)
                                    parents: 0de04a0, ee8658f
```

Both `main` (via `47dfded`) and `fix/ctx-launch-anchor` (via `e20c558`) merged
the `terminal` branch tip `ee8658f`, so:

- `git merge-base main fix/ctx-launch-anchor` → `ee8658f` (the selected
  *terminal* tip on the reporting host)
- `git merge-base --all main fix/ctx-launch-anchor` → `ee8658f` and
  `22814e9`, two incomparable best bases
- main's trunk commits through `22814e9` are **not** ancestors of `ee8658f`

Verified against the live repo:

| Query | Result |
|---|---|
| `git log --max-count 5 fix/ctx-launch-anchor --not ee8658f` | `e20c558 0de04a0 22814e9 cbe6c3c 24c1f2e` (matches buggy yellow lane) |
| `git rev-list --count fix/ctx-launch-anchor --not ee8658f` | 1,265 → `+1260` elision (matches image) |
| `git log --max-count 5 main --not ee8658f` | `47dfded 22814e9 cbe6c3c 24c1f2e f6a93d2` (matches buggy blue lane) |
| `git rev-list --count main --not ee8658f` | 1,262 → `+1257` elision (matches image) |
| `git log fix/ctx-launch-anchor --not main` | `e20c558 0de04a0 bb5bd66 088bbda` (the true `+4`) |
| `git log main --not fix/ctx-launch-anchor` | `47dfded` (the true `-1`) |

Every artifact in the reported image is reproduced by the code's queries; the
repository itself is healthy.

## Root Cause

`gather_branch_impl` in `worktree/cli/src/commands/git_graph.rs` derives all
lane content and elision counts from the merge-base:

- `commits_since(default_branch, &merge_base_full, 5)` — default lane dots
- `commits_since(branch, &merge_base_full, 5)` — branch lane dots
- `hidden_since(<tip>, &merge_base_full, shown)` — both `+N` elision counts
- `commit_details_since(branch, &merge_base_full)` — verbose commit details

All four use `--not <merge-base>`. The intended semantics — per the lane model
of a two-lane Mermaid gitGraph and the `Commits` column — is *each side's
unique commits*, i.e. the same sets counted by
`git rev-list --left-right --count <default>...<branch>`. The formulations
agree when one merge-base dominates all common history. They diverge when
there are multiple incomparable best bases and only one is excluded, as in
the reported criss-cross history.

## Proposed Fix

Exclude the **opposite branch tip** instead of the merge-base for lane
content, elision counts, and verbose details:

| Data | Current query | Fixed query |
|---|---|---|
| branch lane dots | `log <branch> --not <merge-base>` | `log <branch> --not <default>` |
| branch elision count | `rev-list --count <branch> --not <merge-base>` | `rev-list --count <branch> --not <default>` |
| default lane dots | `log <default> --not <merge-base>` | `log <default> --not <branch>` |
| default elision count | `rev-list --count <default> --not <merge-base>` | `rev-list --count <default> --not <branch>` |
| verbose branch details | `log <branch> --not <merge-base>` | `log <branch> --not <default>` |

This makes the graph agree with the table by construction: both now derive
from the symmetric difference of the two tips.

Reader's note: this deliberately changes the data contract from "commits since
one merge-base" to "commits unique to one tip." The latter is the stable user
concept already presented by the table and remains well-defined when Git can
choose among multiple best merge-bases. The implementation should rename
merge-base-oriented lane fields rather than preserving names that would encode
the old, incorrect contract.

The merge-base keeps its remaining roles unchanged:

- **fork anchor** — `merge_base_idx` placement in `base_graph` still looks the
  single merge-base selected by Git up in the recent-main-commits list;
  `ee8658f` is reachable from `main`, so placement still resolves in the
  reported topology. In a multiple-base history this remains a deliberately
  simplified visual anchor, not a claim that the selected commit is the sole
  point of divergence.
- **context commits** — `default_context` (up to 2 ancestors ending at the
  merge-base) is genuinely shared history of both tips and stays as-is. In the
  reported case it renders `7c83083, ee8658f`, which reads correctly once the
  duplicated trunk commits are gone.
- **merge-base verbose detail** — unchanged.

Applies to both gather scopes (`Full` and `BaseOverview`), so the single-
worktree graph (`worktree_graph`) and the base overview graph (`base_graph`)
are fixed together.

### Expected result for the reported case

- branch lane: `088bbda`, `bb5bd66`, `0de04a0`, `e20c558` (4 dots, no elision)
- main lane: `47dfded` (1 dot, no elision)
- context: `7c83083`, `ee8658f` before the fork
- graph and table both say +4 / −1

### Options considered

- **A (chosen): exclude the opposite tip.** Matches the
  already-correct table semantics, robust to shared-side-branch merge-bases
  and to multiple merge-bases (criss-cross merges). The lane sets are
  independent of which base `git merge-base` happens to select; only the
  simplified fork anchor and its context can vary.
- **B: first-parent linearization of each lane.** Would hide back-merge
  commits (`0de04a0`, `e20c558`) that are real branch work and would still
  disagree with the table's counts. Rejected.
- **C: teach the renderer real merge edges (Mermaid `merge` statements).**
  Mermaid gitGraph cannot faithfully express back-merges from an elided trunk;
  substantially larger change for cosmetic gain. Out of scope (noted below).

## Affected Code

Implementation changes in `worktree/cli/src/commands/git_graph.rs`:

- `gather_branch_impl` — swap the exclusion rev per the table above (both
  `GatherScope` arms and the verbose block)
- rename `BranchGraphData::{default_after_base, branch_after_base}` to names
  that express unique reachability (for example, `default_unique` and
  `branch_unique`); likewise make the hidden-count names unambiguous rather
  than leaving "after base" as an implied invariant
- rename the `hidden_since` exclusion parameter from `merge_base` to
  `exclude`, and update the helper, `GatherScope`, gather-function, accessor,
  and `BranchGraphData` docs from "since the merge-base/divergence" to "not
  reachable from the opposite tip" (comment-drift rule: fix docs in the same
  change)
- tests `worktree_graph_uses_in_process_display_ids` and
  `worktree_graph_marks_elided_commits` build their expected values from the
  same `--not <merge-base>` queries and must be updated to the new exclusion

No changes to `worktree` (lib): the ahead/behind cache in
`worktree::worktree::list_worktrees` is a separate, already-correct path.

Public and forward-looking documentation must change with the behavior:

- `worktree/docs/cli/list.md` — define focused graph lanes and verbose branch
  details as commits unique to that tip, not generically "since divergence"
- `worktree/docs/performance-testing.md` — update graph-gather descriptions
  that still encode post-divergence or `default_after_base` terminology
- `worktree/docs/git-graph.md` — update the proposed extracted `GitGraph`
  component's reference behavior and data model so that later extraction does
  not restore the merge-base exclusion bug

## Acceptance Criteria

1. **AC1 — criss-cross fixture renders correctly.** A new temp-repo fixture
   reproduces the reported topology (a side branch merged into both `main` and
   the feature branch, plus a back-merge of `main` into the feature branch's
   line). The fixture asserts that `git merge-base --all` returns two
   incomparable best bases and that common history exists outside whichever
   single base plain `git merge-base` selects. `worktree_graph` output for it:
   - contains no commit id on both lanes,
   - contains no elision marker when each side's unique count ≤ 5,
   - branch lane dot count equals `git rev-list --count <branch> --not main`,
     default lane dot count equals `git rev-list --count main --not <branch>`.
2. **AC2 — elision counts match ahead/behind.** On a fixture exceeding the
   5-commit window, each lane's `+N` equals that side's
   `rev-list --count <tip> --not <other-tip>` minus the 5 shown.
3. **AC3 — verbose details are the unique sequence.** `branch_details` on the
   AC1 fixture equals `git log --reverse <branch> --not <default>` (including
   order), with no commits reachable from the default tip.
4. **AC4 — base graph consistent.** `base_graph` over the AC1 fixture draws
   the feature branch with only its unique commits and, when the selected
   merge-base is in the displayed main window, anchors it at that selected
   base's position. The test derives the selected base instead of assuming
   which best base Git returns.
5. **AC5 — linear topologies unchanged.** Existing fixtures
   (`temp_repo_with_branches`, `temp_repo_over_window`) produce byte-identical
   graphs before and after the change (on linear topology the two exclusions
   select the same sets).
6. **AC6 — subprocess budget unchanged.** The gather paths issue the same
   number of git subprocesses as today (`gather_branch_uses_one_merge_base_…`
   and `base_graph_subprocess_count_is_bounded` still pass unmodified counts).
7. **AC7 — docs updated.** The `BranchGraphData` field docs describe the new
   exclusion semantics; affected source comments and user/forward-looking docs
   no longer describe unique lane, elision, or verbose data as "since the
   merge-base," "post-divergence," or equivalent.
8. **AC8 — fixture is selection-independent.** Tests pass whether plain
   `git merge-base` selects the side-branch base or the main-trunk base and do
   not depend on commit hashes, merge-base output order, timestamps, or the
   host operating system.

## Test Plan

- New fixture builder `temp_repo_with_criss_cross_merge` alongside the existing
  builders in `git_graph.rs` tests, constructing two incomparable histories
  that are each merged into both tips. Validate the fixture with
  `git merge-base --all`, treating its output as an unordered set. Do not
  require plain `git merge-base` to select a particular base: Git does not
  promise that cross-version or cross-platform contract.
- Assert that the selected base leaves at least one commit reachable from both
  tips but not from that base. This is the precondition that makes the old
  implementation fail and prevents the fixture from becoming vacuous.
- New tests covering AC1–AC4 against that fixture; prove non-vacuous by
  confirming they fail on the current implementation before the fix.
- Unit tests should compare full SHAs and parse the branch/default lane
  segments rather than relying on a display-ID substring search.
- Run the package-area unit suite with `just test`, then run `just lint`.

## Out of Scope

- Rendering real merge edges (back-merges of `main` into the branch, or the
  side-branch merge itself) in the Mermaid gitGraph — the two-lane model
  remains a deliberate simplification.
- Multiple-merge-base (criss-cross) anchor selection: plain `git merge-base`
  returns one best base and the simplified fork anchor follows it; lane
  *content* is now independent of that selection, which is the correctness
  boundary this fix draws.
- The ahead/behind cache and `wt list` table computation (already correct).
