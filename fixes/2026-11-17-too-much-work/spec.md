---
kind: fix
name: too-much-work
date: 2026-11-17
status: draft-spec
related:
  - 2026-10-06-archive-guard-full-tree
reviewed: false
implemented: false
---

# Select CI work by the behavior a change can affect

## Problem

A maintenance-script change can trigger Rust linting, broad infrastructure test
suites, and substantial compilation even when no Rust source changed. The
planner treats directory membership as evidence that all of a package's gates
are relevant. That produces a passing result at considerable cost without
directly testing the changed behavior.

The problem becomes more expensive as coverage extends across macOS, Linux,
native Windows, and WSL2. Each environment should prove the behavior that needs
platform evidence, rather than repeat unrelated work selected by directory
ownership. This is a scope-selection problem, separate from whether existing
passing evidence can be reused.

## Observed example

[PR #154](https://github.com/yankeeinlondon/rusty-biscuit/pull/154) changes
`scripts/sweep.sh`, its documentation, the Kache skill, and a `justfile` help
comment. The script adds an unconditional 200 GB target limit, retains a tighter
120 GB limit under disk pressure, and includes Kache's managed incremental
directories in the existing combined incremental budget. No Rust source changes.

The resolved pull-request plan selects:

| Changed input | Work selected | Why this is disproportionate |
| --- | --- | --- |
| `scripts/sweep.sh` | `repo-deps` Rust lint and its broad L1/companion suites | The shell script sits inside the Rust package's directory; the selected suites mostly exercise CI infrastructure rather than sweep policy. |
| `.claude/skills/kache/SKILL.md` | Two Darkmatter document-corpus tests | These read the edited document and provide relevant coverage, but require compiling a large Rust dependency graph. |
| `justfile` comment | Test-toolkit input tests; the local run executes 109 tests | A help-comment edit selects contracts covering substantially more than the edited text. The planner initially identifies ten test units; execution expands to 109 tests. |

Strict local validation takes **21m21s** and passes 15 gates, including the CI
Python companion suites, Rust linting, 919 `repo-deps` Rust tests, two Darkmatter
tests, and 109 test-toolkit tests. Hosted CI then builds a Linux archive and runs
the Linux `repo-deps` gates while reusing qualifying local evidence. These are
observations from this run, not a general timing baseline.

There is no checked-in regression test that exercises the changed sweep script.
Its new behavior was checked with temporary controlled fixtures. The existing
`tools/test-toolkit/tests/real_kache_worktree_restore.rs` does test real Kache
cache reuse between worktrees, but it belongs to the opt-in `test-real` suite
and is not selected for this PR. Kache setup/configuration contract tests are
different from that real-cache test and from sweep-policy tests.

## Intended outcome

Every selected gate should answer a concrete question about the changed behavior.
A shell-only maintenance change should run focused shell checks and its declared
behavior tests. It should not select Rust Clippy, unrelated Rust tests, or the
CI-planner companion suites solely because the script shares a directory with
`repo-deps`.

Documentation readers remain valid consumers. Narrow their work where possible;
do not remove meaningful schema or contract coverage merely to lower a count.
Likewise, changes to actual Rust source or CI-planner logic must retain the gates
that protect those surfaces.

## Proposed requirements

1. Distinguish package source from maintenance-script inputs. Express the
   script's real consumers and checks explicitly; do not introduce a blanket
   exclusion for shell or Python files, some of which implement CI itself.
2. Select lint tools by the changed language and contract. A shell-only change
   can justify shell syntax checks or an appropriate shell linter, but not Rust
   linting without an affected Rust consumer.
3. Add durable, focused sweep fixtures covering both target caps, the free-space
   boundary and unreadable-space fallback, the combined incremental budget,
   single-cache and empty-cache cases, and preservation of unrelated artifacts.
   Simulate sizes and external commands; never allocate hundreds of gigabytes,
   invoke a host-wide sweep, or delete a developer's live build outputs.
4. Keep real Kache cache-hit tests separate. Run them when wrapper, configuration,
   restore, or compatibility behavior warrants them; changing eviction thresholds
   alone does not establish that need.
5. Preserve one canonical resolved plan and existing package/environment/gate
   identities. Local hooks and hosted jobs consume that plan; neither may
   independently rediscover scope. This fix must improve selection before
   evidence reuse, not disguise unnecessary cells as reused or skipped.
6. Prove behavior on all supported environments where it executes. Keep shared
   policy tests cheap and isolate platform-specific shell/filesystem checks.
   Document which implementation each environment exercises, including Git Bash
   versus the separate native Windows sweep policy. Do not change event-based
   OS scheduling as a shortcut.

## Acceptance criteria

- A sweep-only diff selects its focused checks and selects no unrelated
  `repo-deps` Rust lint, Rust test, or CI-planner companion work.
- Rust-source and CI-implementation counterexamples still select their required
  broad gates; scope tests demonstrate both inclusion and exclusion.
- A comment-only `justfile` diff selects only justified consumers. Any remaining
  expansion from a few selected units to a larger executable suite is explained
  and measured.
- Mixed changes union the required checks without losing coverage or duplicating
  suites. Local and hosted planning agree.
- Before/after plans for the example change list cell counts, test counts,
  compilation requirements, and measured runtime. Improvements are demonstrated
  by removed work, not solely by warm-cache timing.
- Current CI documentation and repo skills describe the resulting ownership
  rules and how a developer registers a maintenance script's checks.

## Coordination with `2026-10-06-archive-guard-full-tree`

That fix changes what the archive-path guard scans once it is selected (always
the full eligible tree) and leaves its selection rules alone. This spec changes
selection. The two are independent and can land in either order.

- Narrowing when an ordinary `test-toolkit` lint cell exists will also reduce
  how often that cell carries the `archive-path-guard` companion. That is
  acceptable. The guard's explicit triggers must stay covered: an
  eligible-source change or a change to a guard-owned input (deletions
  included) still selects the guard, through the guard-only cell if no
  ordinary owner lint cell exists, whenever the event schedules
  `ubuntu-latest`.
- Selecting the guard does not count as disproportionate work because it scans
  the whole tree. That repository-wide scope answers the guard's question
  directly; this spec decides only whether a change selects it.
- Contract tests are deleted only when their contract disappears, as that fix
  does for the guard's cross-language scope tests, and never only to lower a
  count.

## Decisions still needed

- Choose the smallest maintainable ownership mechanism: explicit script/input
  contracts, package layout changes, or another approach consistent with the
  existing test-input model. Do not grow a second scheduler or ad hoc per-file
  workflow exceptions.
- Decide how focused shell fixtures integrate with the existing canonical gates
  without requiring a large unrelated Rust build just to execute them.
- Determine which documentation checks can validate edited content without
  repeatedly compiling the full consuming package.

This is a draft describing follow-up work. It does not change the checks or
required coverage of the sweep-policy PR already running.
