---
kind: fix
created: 2026-11-14
status: draft-spec
area: sniff
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-14-edit-action
depends-on:
    - 2026-11-14-shared-process-tree
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
review_iterations: 2
completed: true
implemented: true
implemented_by: "claude/opus"
---

# Own the tree of a command that holds the terminal

## Why this exists

`2026-11-14-shared-process-tree` gives every caller one race-free process
tree owner in `sniff`. One Claudine path is left unowned on Unix by an
author ruling in that spec: a lifecycle `shell` action that runs in the
host's terminal foreground, so it can read keyboard input. Owning its tree
means giving it its own process group, and a process group that is not the
terminal's foreground group cannot read from the terminal. Today that path
keeps direct-child-only cancellation, so a background process the command
starts survives Ctrl-C:

```yaml
initialize:
  - shell: "./scripts/setup.sh"   # prompts "continue? [y/n]", then starts a watcher in the background
```

Darkmatter's `edit` launcher already solves this for terminal editors by
handing the terminal's foreground to the editor's group and restoring it
afterward. That handoff lives in the editor launcher only.

## Author rulings (2026-11-14)

- **Close the gap in a follow-up to `2026-11-14-shared-process-tree`**,
  queued after it.
- **One handoff implementation:** the terminal foreground handoff and its
  restoration move into the shared process-tree API in `sniff`; the
  Darkmatter editor launcher and Claudine's foreground lifecycle commands
  both use it, and neither keeps its own copy.

## Requirements (draft)

1. The shared API can start a child in its own group and hand it the
   terminal's foreground for the duration of the command, restoring the
   host's foreground and terminal settings on every exit path (success,
   failure, cancellation, panic unwind).
2. Claudine's foreground lifecycle commands own their tree through it, so
   cancellation stops the command and everything it started; the documented
   exception in the canonical process-tree page is removed.
3. Darkmatter's editor launcher uses the same handoff; its existing terminal
   tests pass unchanged.
4. Native Windows and WSL2 behavior is unchanged (no terminal foreground
   concept on native Windows; WSL2 follows the Unix path).
5. The canonical page `sniff/docs/topics/process-trees.md` documents the
   handoff.

## Verification (draft)

- An L2 terminal test: a foreground lifecycle command reads a typed line,
  starts a background child, and is cancelled; both are stopped and the
  terminal is restored. No terminal or browser window takes focus.
- The existing `edit` L2 handoff tests pass unchanged.

## Open Questions

To be settled by review.
