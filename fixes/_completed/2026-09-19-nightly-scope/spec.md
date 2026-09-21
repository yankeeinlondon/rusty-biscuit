---
kind: fix
name: nightly-scope
date: 2026-09-19
status: complete
related:
  - 2026-09-18-ci-cadence
  - 2026-09-19-hosted-evidence-reuse
---

# The nightly runs WSL2 over what changed, and nothing else

## Problem

The cadence spec's stated goal for the `schedule` trigger is one line: "WSL2
is nightly and manual only." Planned locally as the workflow would have run
it on 2026-09-19 (`--all --event schedule`), the first nightly was 361 jobs:

| environment | executing cells | producer |
|---|---|---|
| ubuntu-latest | 161 (68 lint, 12 check, 68 L1, 11 L2, 2 browser) | 68 keys |
| windows-latest | 68 L1 | 68 keys |
| wsl2-ubuntu | 66 L1 | none (consumes the Ubuntu archives) |

Three reasons, none of them WSL2:

- `ubuntu-latest` named `schedule` in its `events` because the WSL2 guest
  runs the archive Linux built, and the planner drops an environment's build
  records along with its cells. Keeping the archives meant keeping 161 cells
  every pull request had already proved.
- `windows-latest` named `schedule` as well as `push`, so the nightly
  re-proved what every push to `main` proves.
- The schedule passed `--all`. A nightly that planned only what changed since
  the last nightly with WSL2 evidence is usually a handful of packages.

## Decisions

1. **Producers by demand.** `calculate_scope` keeps an unscheduled
   environment in the plan's table when a scheduled archive-only guest is in
   its `build.executes` (`producing_environments`). It bears no cell: the
   cell-bearing table is the event's, the build and preflight table adds the
   producers. The plan records it in a new optional `producing_environments`
   field (`{name, for}`) rather than in `deferred_environments`, so a reader
   can see why an owner job exists for an environment with no cells. The
   schema version stays 4. `classify_preflight` adds the producer's runner
   for every executing archive-consuming cell, since the owner job runs
   there.
2. **`schedule` leaves `ubuntu-latest` and `windows-latest`.** WSL2 alone
   names it. Windows is proven by every push to `main`; Linux joins the
   nightly through decision 1. `workflow_dispatch` still plans everything.
3. **The nightly plans the diff since the last successful nightly.**
   `reuse_validation.py nightly-base` prints the head of the newest
   completed, successful `schedule` run of `ci.yml` on `main` that is an
   ancestor of the current head, and `ci.yml` diffs from it; no such run
   plans the full workspace with the all-zero base, as a manual run does. A
   failed nightly is never a base, so the packages it failed stay in the next
   night's diff.

Resulting nightly: one Ubuntu producer for the changed packages, their WSL2
L1 cells, and the two preflight runners. An unchanged workspace plans
nothing.

## Completion (2026-09-19)

Merged to `main` with PR #87 (353aaf6ee) the same day, one commit
(09be36560), ahead of the first nightly. Planned locally against the
shipped table, that first nightly is 66 WSL2 cells and one 66-key Ubuntu
producer (job estimate 132, was 361); it plans the full workspace once
because no nightly has yet succeeded to diff from. Its post-merge run on
`main` was cancelled by the shared concurrency group when the next merge
followed within a minute, so its Windows proof came from the merge that
followed rather than its own run. The first hosted nightly's numbers, once
it has run, belong here.

## Not done

- Cell-level reuse of CI's own results across trees
  (2026-09-19-hosted-evidence-reuse), which would make the diff exact per
  cell rather than per package.
- Sharing the push run's Ubuntu archives with the nightly. Artifacts are
  retained one day and the trees rarely match; the diff scope makes the
  producer small instead.
