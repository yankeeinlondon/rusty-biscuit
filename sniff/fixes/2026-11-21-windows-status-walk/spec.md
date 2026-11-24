---
kind: fix
created: 2026-11-21
status: draft-spec
area: sniff
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-20-commit-link-walks
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
reviewed: false
implemented: false
---

# Bring Sniff's working-tree status walk on Windows close to `git status`

## Why this exists

Every Sniff command that reports working-tree state runs one status walk:
`sniff repo git-status`, `sniff repo state`, the aggregate `sniff repo`, and
the `changed_paths` consumers. On native Windows that walk is the largest
fixed cost in those commands. It was found while measuring
`2026-11-20-commit-link-walks` on the build rigs.

*Measured 2026-11-21 in the same checkout of this monorepo (25,003 tracked
files, no untracked non-ignored files, no `core.fsmonitor` or
`core.untrackedCache`), with this branch's build (`387cce9a4`):*

| Host | `sniff repo state --json` | `git status` | Ratio |
|---|---|---|---|
| Native Windows (`$BUILD_WIN`, PowerShell, `EXEPATH` unset) | 423–455 ms | 126–131 ms | about 3.4× |
| Native Windows, `EXEPATH` set | 318–403 ms | | about 2.8× |
| Linux build host (20,878 tracked files) | 100–122 ms | 59–66 ms | about 1.8× |

A Windows build from an earlier, unidentified commit took 1.65 s for the
same command in the same checkout. The improvement since then has not been
attributed to a specific change.

The `EXEPATH` part of the Windows cost (about 50–120 ms) is handled by
`2026-11-20-commit-link-walks`. This fix covers what remains: roughly
0.2–0.3 s more than `git status` on every run.

## Scope

- In scope: the status walk in `sniff/lib/src/filesystem/git/status.rs`
  (`status_platform` and `status_iter`) and anything it configures in gix,
  as measured on native Windows. Changes that also speed up Linux, macOS, or
  WSL2 are welcome but must not slow them down.
- Out of scope: changing what status reports. Untracked files stay individual
  paths, and renames and rewrites stay untracked. Also out of scope: writing
  the index or any other repository state (Sniff only reads repositories),
  and enabling `core.fsmonitor` or `core.untrackedCache` for the user.

## Phase 1: find where the time goes

The cause is not yet known, so the first phase measures instead of changing
code. Record every result in the implementation log.

1. **Split the walk into stages.** Add `--perf` stages, gated by
   `performance::is_collecting()`, for:
   - opening and decoding the index
   - the index-to-worktree comparison (one metadata read per tracked file)
   - the untracked-file directory walk
   - the tree-to-index (staged) comparison

   Report them on Windows, Linux, and macOS for the same checkout state.
2. **Test the leading hypotheses, each against the stage it predicts:**
   - *Per-file metadata cost on NTFS.* Each metadata read opens a file
     handle, and Windows Defender may scan on open. Git for Windows avoids
     this with its file-system cache (`core.fscache`), which reads a whole
     directory's metadata in one call. If the index-to-worktree stage
     dominates, compare against `git -c core.fscache=false status`.
   - *Worker count.* gix parallelizes the comparison according to its
     configuration. Record the thread count it actually uses on the
     rig and compare a forced single thread against several.
   - *Directory walk over ignored trees.* If the untracked stage dominates,
     confirm that ignored directories such as `target/` are pruned rather
     than descended.
3. **Stop and report** if the stages show the cost lives somewhere that
   needs a decision: an upstream gix change, a Windows-only metadata reader,
   or a change in what status reports.

## Phase 2: fix the measured cause

Phase 2 is written once Phase 1 has identified the cause. Whatever the
change, it keeps one shared status walk per request (`git.status_walks`
does not rise). It keeps the existing classification in
`classify_worktree_status`, and it keeps index reads behind
`open::guard_index_read`.

## Acceptance

- On the Windows rig, from PowerShell with `EXEPATH` set, in the measured
  checkout state, `sniff repo state --json` finishes within 2× `git status`
  on the same checkout (today it is about 2.8×).
- On Linux, macOS, and WSL2, `sniff repo state --json` is no slower than
  today in the same checkout state.
- Status output is byte-identical before and after on the existing fixtures
  and on the measured checkout.

## Verification

- The existing status, `changed_paths`, and `working_state` test suites pass
  unchanged on every OS. Run `just cross-check sniff --os windows` and
  `just cross-check sniff --os wsl` before handoff, plus `just test` and
  `just lint` in `sniff/` and the clippy gates from the `sniff` skill.
- Record the stage timings for each OS before and after in the
  implementation log.
- Wall-clock checks are manual. Tests assert counters and outcomes only.
