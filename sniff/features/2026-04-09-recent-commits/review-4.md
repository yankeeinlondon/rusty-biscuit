# Review 4: recent-commits

## Findings

1. High: the walkers silently drop empty commits, which makes the core query APIs return the wrong commit set and can make hash boundaries non-inclusive.

   The implementation skips any commit whose diff produces no paths at [sniff/lib/src/filesystem/git/recent_commits.rs#L352](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/git/recent_commits.rs#L352) and [sniff/lib/src/filesystem/git/recent_commits.rs#L440](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/git/recent_commits.rs#L440). That is not part of the spec or the tech design: the feature is defined in terms of commits, not “commits that touched at least one path” ([spec.md](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/features/2026-04-09-recent-commits/spec.md), [tech-design.md#L180](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/features/2026-04-09-recent-commits/tech-design.md#L180)).

   This is observable today. A repo with:
   - `feat: initial` touching `src/main.rs`
   - `chore: empty marker` created via `git commit --allow-empty`

   returns only the first commit from `sniff repo recent-commits --json`; the empty head commit disappears entirely. The hash path is worse: if the requested boundary commit is empty, [sniff/lib/src/filesystem/git/recent_commits.rs#L481](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/git/recent_commits.rs#L481) breaks after excluding it, so `get_recent_commits_by_hash()` is no longer inclusive for that boundary.

   Recommendation: keep empty commits in `CommitDescSet` with `files: []`, and let the rendering/filtering layers decide whether an empty-file commit should be hidden for a specific view.

2. Medium: the time-based walker assumes the revwalk is monotonically ordered by commit timestamp and can miss qualifying commits in skewed history.

   The range path sets `TOPOLOGICAL | TIME` sorting at [sniff/lib/src/filesystem/git/recent_commits.rs#L319](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/git/recent_commits.rs#L319) and then terminates on the first out-of-range commit at [sniff/lib/src/filesystem/git/recent_commits.rs#L338](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/git/recent_commits.rs#L338). That break is only safe if the walk is strictly newest-to-oldest by commit time. Once `TOPOLOGICAL` is mixed in, that guarantee is gone.

   A simple linear repro already fails if the head commit has an older timestamp than its parent:
   - parent commit dated `2026-04-09T12:00:00Z`
   - head commit dated `2000-01-01T00:00:00Z`

   `sniff repo recent-commits 7d --json` currently returns `none found`, even though the parent commit is inside the requested window, because the walker sees the old HEAD first and breaks before visiting its newer parent.

   Recommendation: do not `break` based on timestamp unless the traversal is guaranteed to be time-monotonic. Either use a pure time sort for range queries, or keep walking and filter by `since..until` without early termination.

3. Medium: the tests are still too shallow for the failure modes above and for the spec’s “exact result set” behavior.

   The feature added many tests, but the important CLI cases are still mostly smoke tests. For example, the new CLI coverage around [sniff/cli/tests/cli.rs#L1781](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/tests/cli.rs#L1781) through [sniff/cli/tests/cli.rs#L2245](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/tests/cli.rs#L2245) usually asserts only success/non-empty output, not the exact commit set, ordering, or rendered grouping. The library integration coverage around [sniff/lib/tests/integration.rs#L1048](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/tests/integration.rs#L1048) through [sniff/lib/tests/integration.rs#L1643](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/tests/integration.rs#L1643) never creates an empty commit and never exercises skewed commit timestamps for the duration/date paths.

   Those are not hypothetical gaps: both bugs above survive the current green suite.

   Recommendation:
   - add a library test for `--allow-empty` commits, including `get_recent_commits_by_hash()` with an empty boundary commit;
   - add a range-query test where HEAD is older than its parent and assert the newer parent is still returned;
   - strengthen the CLI tests to assert exact JSON payloads and exact grouped Markdown/plain output for `recent-commits`, `source-code-changes`, and `documentation-changes`, rather than only exit status.

## Open Question

- `today` and `yesterday` are currently evaluated on UTC day boundaries at [sniff/cli/src/output/recent_commits.rs#L34](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/src/output/recent_commits.rs#L34). If the intended UX is local calendar days for the user running the CLI, that still needs an explicit decision and test coverage.

- DECISION: we should use local time!

## Verification

- `cargo test -p sniff recent_commits -- --nocapture`
- `cargo test -p sniff-cli recent_commits -- --nocapture`
- Manual repro: empty-head commit omitted from `target/debug/sniff --base <tmp> repo recent-commits --json`
- Manual repro: skewed-date HEAD causes `target/debug/sniff --base <tmp> repo recent-commits 7d --json` to print `none found`
