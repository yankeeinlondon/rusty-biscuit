---
kind: fix
created: 2026-11-14
status: finalized-spec
area: sniff
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-14-edit-action
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
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: false
review_iterations: 0
---

# One race-free process tree, owned by Sniff

## Why this exists

Three packages each implement "a spawned child and everything it starts,
stopped as one unit":

| Implementation | Used by | Windows assignment | Notable capability |
|---|---|---|---|
| `darkmatter/lib/src/process_tree.rs` (`ProcessTree`, crate-private) | lifecycle `edit` launcher (`editor/launch.rs`), `md` shell expansion (`markdown/compose/shell_expansion/launcher.rs`) | spawned running, then assigned: **a descendant started before assignment escapes the Job** (accepted in its docs) | graceful stop (SIGTERM, grace period, SIGKILL; Windows immediate); `release` clears kill-on-close after a successful edit so a GUI editor's window survives |
| `claudine/lib/src/composition/sequence/task/shell.rs` (its own `ProcessTree`) | sequence `shell:` tasks | spawned `CREATE_SUSPENDED`, assigned, then resumed: race-free | reaps the whole tree on `Drop`, so every exit path ends it |
| `sniff/lib/src/process.rs` (its own `ProcessTree`, crate-private) | Sniff probe commands | spawned `CREATE_SUSPENDED`, assigned, then resumed: race-free | Unix: remembers observed descendants (with start times), so a `setsid` descendant reparented away from the child is still found |

The Windows window is a real gap in the `edit` action's cancellation
guarantee (an editor helper started in the first milliseconds survives
Ctrl-C or a sibling failure), and the duplication means each fix lands in
one copy only. Darkmatter's comment says closing the window would require
enumerating the child's threads; the other two copies already do that.

The review of `2026-11-14-edit-action` surfaced it.

## Author rulings (2026-11-14)

- **One implementation, race-free on Windows, in `sniff`.** `sniff` is the
  lowest crate in the chain (`sniff` ← `darkmatter` ← `claudine`), so all
  three consumers can share it without a new crate.
- **DRY:** the other two copies are deleted, not wrapped.
- **A canonical document exists at the conclusion of this fix** that states
  the rules (below). The `os` skill and consumer docs link to it rather than
  restating it.

## Requirements

1. A public process-tree API in `sniff` that is the **union** of the three
   capability sets, not the intersection:
   - configure a `Command` before spawn (Unix: own process group; Windows:
     `CREATE_SUSPENDED`), then attach to the spawned `Child`, assigning it to
     a kill-on-close Job and resuming its initial thread on Windows;
   - immediate terminate, and graceful terminate with a grace period (Unix
     SIGTERM then SIGKILL; Windows immediate);
   - release (Windows: clear kill-on-close so remaining members outlive the
     handle; Unix: stop owning the group) for callers whose success path
     must leave descendants running;
   - reap on `Drop` unless released;
   - Unix descendant tracking that survives `setsid` reparenting.
   An attach failure leaves the child suspended (Windows) for the caller to
   kill, as Claudine's does today.
2. `darkmatter` (`editor/launch.rs`, shell expansion) and `claudine`
   (sequence shell tasks) use it; their copies and Sniff's private one are
   removed. No behavior regression in any caller's existing tests.
3. The accepted Windows assignment window is gone: the `edit` launcher's and
   `md` shell expansion's descendants cannot escape the Job.
4. **Canonical document:** `sniff/docs/topics/process-trees.md`, written for a
   developer new to the repository, covering: what owning a process tree
   means and why; the per-OS mechanism (Unix process group and descendant
   tracking; Windows suspended start, Job assignment, resume); the lifecycle
   (configure → spawn → attach → terminate / graceful terminate / release /
   drop) as a Mermaid diagram; what is guaranteed and what still escapes (a
   Unix descendant that changes its own group or session after it was last
   observed; process-group ID reuse after the root is reaped); and how to use
   the API, with a compact example per operation.
5. Docs and skills point at that page instead of restating it:
   `.claude/skills/os/SKILL.md` and `os/windows.md` (the Job-assignment
   entry added during `2026-11-14-edit-action`), `.claude/skills/sniff/`,
   `.claude/skills/darkmatter/` and `.claude/skills/claudine/` where they
   describe process trees, and Darkmatter's `edit` cancellation docs and
   Claudine's lifecycle cancellation docs. No skill or doc may still describe
   the window as accepted. Update `docs/dependencies.md` files if dependency
   edges change.

## Verification

- Sniff unit and integration tests for each operation on Unix and Windows,
  including a child that spawns a descendant immediately at start (proves no
  escape on Windows) and a `setsid` descendant (Unix).
- The existing `edit` cancellation tests
  (`editor_launcher::cancellation_terminates_the_whole_tree_and_reaps_it`,
  the SIGTERM-ignoring and GUI-client cases) and Claudine's sequence shell
  tests pass unchanged.
- Windows evidence from the Windows rig or the nightly Windows CI cell (see
  the `os` skill); macOS cannot run the Windows path.
- A repository search finds one `CreateJobObjectW` call site and one
  `process_group(0)` configuration for tree ownership.
