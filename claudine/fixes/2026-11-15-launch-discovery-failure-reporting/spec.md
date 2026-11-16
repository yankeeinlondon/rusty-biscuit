---
kind: fix
created: 2026-11-15
status: abandoned
area: claudine
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-14-lifecycle-mutation-root
---

# Report a failed launch-repository discovery before lifecycle writes

> **Abandoned.** The requirement was completed by
> `2026-11-14-lifecycle-mutation-root` review iteration 1 (finding R1-F2):
> every lifecycle host now refuses a run whose launch-repository discovery
> failed, through one shared check, before any file action. The first
> option below was taken; `InvocationContext::capture` is unchanged.

## Problem

The lifecycle mutation root is the caller's Git working-tree top or, only
when a successful observation finds no working tree, the launch directory.
`2026-11-14-lifecycle-mutation-root` requires that "a discovery failure must
be reported before file actions run" and that a discovery failure never
becomes permission to write under a guessed root.

Claudine does not report it. `InvocationContext` records a failed
launch-repository discovery as `RepositoryEntry::failed`
(`claudine/lib/src/invocation_context.rs`, `observe_repository`) and
`capture` still returns `Ok`. The launch scope then has no repository root,
so every lifecycle host bounds writes by the launch directory, with no
diagnostic. Example: `git init` a repository, corrupt its `.git/config`, and
run `claudine compose` from it with `initialize: [ensure_file: notes.md]`;
the run exits 0 and writes `notes.md` in the launch directory.

The fallback only narrows the boundary, so nothing is written outside the
caller's tree (pinned by
`lifecycle_mutation_root::a_failed_launch_discovery_never_widens_the_boundary_to_the_repository_top`),
but the author is never told that the repository could not be read.

## Decision needed

- Fail lifecycle file actions (or the whole run) when
  `InvocationContext::launch_repository().failure()` is set, at the seven
  lifecycle `EffectEngine` constructions (compose loop route, staged boot,
  wrapper pipeline, sequence container, task stack, step guard, loop
  harness), through one shared helper; or
- fail `InvocationContext::capture` itself, which affects every Claudine
  route, including runs with no lifecycle actions.

Darkmatter maps a discovery error to `ContextBuildError::Discovery`, and
`md compose` fails before any file action for a repository with a corrupt
`HEAD` and for a broken `.git` file (sniff now reports that instead of
skipping it). Claudine is the host that still continues.
