---
kind: fix
created: 2026-11-20
status: finalized-spec
area: claudine
packages:
    - claudine-cli
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
reviewed: true
reviewed_by: "codex/gpt-6.2-sol"
reviewed_on: "2026-11-20"
review_iterations: 0
needs_rulings: false
implemented: false
---

# Keep loop prompts and lifecycle context in sync

## The defect

A running Claudine prompt loop can fail after someone edits the prompt. In the
reported run, the first implementation phase succeeded. The second phase
loaded an edited lifecycle message that referred to `ctx.branch` and
`ctx.hostname`, then tried to render the message using a context snapshot taken
for the earlier text. The snapshot did not have the newly required values.
The `start` event failed before the second provider launch, and the edited
`failure` message could fail for the same reason.

```text
⤫ CompositionError: lifecycle evaluation error
┃ A late-binding expression raised while the `start` lifecycle event was firing,
┃ in `start[0].message` (…/prompts/_implement/implement-plan.md).
┃
┃ Reason: interpolation of `ctx.branch` failed: context variable `ctx.branch`
┃ belongs to the Git capture group, which this composition request did not
┃ capture; …
```

Commit `daa04d786` added the branch and hostname references to the implementation
prompt's lifecycle messages. The earlier prompt already referred to repository
and working-tree information. The error establishes which values were missing
from the snapshot used by that event.

### Reproduction

The reported reproduction used a temporary Git repository with copies of the
implementation router and target prompt, the local Claudine configuration, and a
spec and plan pair with `total_phases: 2`. The target started with the prompt
from before `daa04d786`, with shell actions replaced by informational actions
to avoid approval prompts. A fake `claude` replaced the target with the edited
version on its first launch and emitted a successful stream-JSON result. The
command was `claudine compose --claude prompts/implement.md spec=<spec>`. The
second phase failed in `start` and the process exited with code 1. The tests
below use a small fixture so changes to the implementation prompts cannot
affect the regression test.

## What causes the failure

The [Claudine preparation pipeline](../../docs/pipeline.md#b4-staged-initialization-and-shell-preflight)
and [entry rules](../../docs/topics/composition.md#entry-reasons-and-the-stage-matrix)
say a later iteration uses the document prepared and audited when the loop
started. Claudine's command code selects that source in memory, but its later
iterations inherit the flag that tells the provider harness it still owes a
read and a schema verdict. This can happen even when the first iteration
already validated the source after `initialize`.

```mermaid
sequenceDiagram
    participant Loop as Claudine loop
    participant Harness as Provider harness
    participant Disk as Prompt file
    participant Events as Lifecycle events
    Loop->>Loop: Prepare iteration 2 from retained prompt
    Loop->>Events: Install context for retained prompt
    Loop->>Harness: Run iteration with schema verdict marked pending
    Harness->>Disk: Read edited prompt and extend its context
    Harness->>Events: Install edited lifecycle actions
    Events->>Events: Render start with earlier context
    Events-->>Harness: Required context value is missing
```

The context problem also exists on a legitimate read after `initialize`. The
Claudine CLI's [provider harness startup](../../cli/src/commands/wrap/harness_orch/loop_control.rs)
installs the new lifecycle actions on the guard that fires events. A direct
document can keep the earlier context, and an adopted document installs its
identity-adjusted context only after later checks that can fail. The Claudine
CLI's [command-level startup](../../cli/src/commands/compose/prep.rs) has a
separate read after `initialize` for both single runs and the first loop
iteration. It installs the new actions too, while the loop also saves its
preparation context and frontmatter for the post-iteration condition. A
harness-only fix leaves those command-level paths exposed.

## Expected behavior

### Retain the prompt prepared at startup

- Keep the root document selected and validated for the first iteration as the
  source of subsequent ordinary iterations. If startup reads the file after
  `initialize`, retain that read, including its caller and proxy inputs and
  the approvals it earned. On the eager route where the first preparation
  owns the verdict, retain the source the caller or handoff resolved.
- Every later iteration composes from that source with its own loop values,
  runtime state, and required context. It observes Git working state as the
  preceding iteration left it and audits the command text the new values
  produce using existing approvals. A command that changes beyond those
  approvals is refused through the existing shell checks. Each later
  iteration skips the launch schema verdict and carries no pending verdict.
- Edits to the root prompt on disk after startup take effect on a new run.
  The rule covers the prompt body, authored frontmatter, and lifecycle
  actions, including disk writes to the root made by lifecycle actions.
  Use `set` or `loop.action` for values that should carry between iterations.
  Includes and explicit reads such as `frontmatter("progress.md", "done")`
  follow their existing resolution and approval rules.
- Retry and resume continue to read and validate the active document from
  disk for that attempt. A fresh read during recovery leaves the loop's
  retained source and loop configuration in place for the next ordinary
  iteration. A proxy handoff ends the source loop under the existing rules
  and prepares the target under its own startup rules.
- Apply the same document-loop rules inside a sequence. Sequence and group
  containers still reread the inputs they are documented to read when a
  step, task, or container starts. A new invocation of a prompt task sees
  those edits before its own document loop begins.

### Use the context that belongs to the active lifecycle actions

- After a successful read and preparation, hand off the lifecycle actions,
  prepared `ctx.*` and `env.*` values, and their recorded document run
  together. A read after `initialize` extends that run's snapshot on demand.
  Values already observed retain their earlier values, and existing provider
  and model selection rules apply. Install the matching values before any
  event uses the new actions, including `blocked` and `finalize` where the
  existing failure route selects those actions. If the read fails before a
  new lifecycle is available, keep the previous actions and their context
  for the existing failure route.
- The loop condition and actions after an iteration use the preparation for
  that iteration. When startup replaces the first iteration's preparation,
  pass its resulting context and effective frontmatter to the loop engine.
  Keep the existing event order and dispatch `initialize` only once.
  `current.*` remains an event-time read.

> **Reader’s note.** An `inline-compose` loop has two uses for the same file.
> Its prompt instructions follow the retained-source rule. The file on disk
> is also the deliverable, so Claudine still takes a baseline for the attempt
> and reads the result to check what the provider changed. Use the live
> baseline for change detection, repair, and rollback. An edit from iteration
> 1 must never count as an edit from iteration 2. Reading those bytes for
> reconciliation leaves prompt preparation and launch validation on the
> retained source.

## Implementation and verification

### Phase 1 — Reproduce with isolated tests

Use the Claudine CLI's [staged startup tests](../../cli/tests/l1/compose_initialize_staged_boot.rs),
[lifecycle traces](../../cli/tests/l1/lifecycle_traces.rs), and
[inline loop tests](../../cli/tests/l1/loop_cli.rs) as fixture patterns. Use a
temporary repository and the shared CLI fixture with a fake provider. Drive
file changes from the fixture or provider so the tests need neither a real
provider nor timing delays.

- Exercise a loop-owning proxy target with `initialize`, matching the
  reported route. On the first launch, replace its body, lifecycle text, and
  root schema declaration on disk. The edited lifecycle references a context
  group absent from the fixture's original prompt. Assert two launches, one
  initialization, the original instructions and lifecycle text rendered with
  the second iteration's values, and exit code 0. Start a new run or use an
  existing fresh-read test to show that the fixture's edit is observable at
  an intended read boundary. An eager direct loop alone can miss the bug
  because it starts with no pending schema verdict.
- Exercise a loop-owning proxy target whose first iteration delegates its
  read to the harness. Change a marker before that read and again during the
  first provider launch. Assert that both ordinary iterations use the
  marker the harness actually validated.
- Exercise a read after `initialize` that introduces a new context group in
  `start`, with an existing group present from the first read. Assert that
  the event renders the new value using the context used by composition
  and retains the earlier observation. Cover both command and harness
  startup where each installs the actions. At least one assertion must
  exercise an event through the actual guard. If a test already passes,
  record which route it covers and verify the separate affected route.
- For the staged first loop iteration, check that the condition or actions
  see a context value or frontmatter field supplied by the stabilized
  preparation. Cover a later ordinary iteration with zero deferred schema
  work and fresh working-tree values when the fixture changes them.
- For inline composition, let iteration 1 change the body and let iteration
  2 report success without editing it. Assert the existing unchanged-body
  verdict and preservation of the first iteration's deliverable. Keep the
  existing positive case where both iterations make distinct edits.

### Phase 2 — Correct the preparation paths

- In the Claudine CLI's [loop preparation](../../cli/src/commands/compose/prep.rs),
  make the startup source and the later-iteration source agree. Clear the
  pending-verdict flag in the later-iteration preparation path independently
  of whether the invocation can provide a context. Keep the first-iteration
  verdict at its existing boundary. If the first iteration delegates its
  read to the provider harness, make that same resolved source and approval
  basis available to the loop. Loading the file again after the provider
  returns could adopt an edit made during that launch. Prepare later passes
  under the existing loop-iteration policy and keep their per-iteration
  template and lifecycle command checks.
- In the Claudine CLI's [command startup](../../cli/src/commands/compose/prep.rs)
  and [provider harness startup](../../cli/src/commands/wrap/harness_orch/loop_control.rs),
  install the completed preparation's context on the lifecycle guard when
  the new actions take effect. The Claudine library's
  [prepared composition](../../lib/src/composition/types.rs) already carries
  the context and the document run that owns it. Record lifecycle use on
  that run through the existing accounting and reuse the installed context
  for post-iteration evaluation. Preserve target identity overrides on
  adopted routes and the existing shell gate for early handlers. Correct the
  comment that treats handing off the prepared context as a second capture.
- In the Claudine library's [inline preparation](../../lib/src/composition/prepare.rs),
  the closure plan currently derives its baseline from the same source as
  the prompt. Give inline reconciliation the actual pre-attempt bytes and
  body hash without feeding those bytes back into preparation. Preserve the
  existing [inline reconciliation](../../lib/src/composition/closure.rs)
  checks, including the basis used for rollback and the completion schema
  retained at launch.
- Check the Claudine CLI's [sequence prompt task runner](../../cli/src/commands/wrap/sequence/task_run.rs)
  for any document-loop entry that bypasses the shared preparation. Preserve
  [sequence step input reads](../../cli/src/commands/wrap/sequence/iterate.rs)
  when a new step begins.

### Phase 3 — Update the current documentation

In the [looping guide](../../docs/topics/flow-control/looping.md) and
[composition pipeline](../../docs/pipeline.md), show a root edit that a new
run sees and a `set` or explicit file read that a later iteration can see.
Explain how startup passes the same prepared context to composition and
lifecycle evaluation. Qualify the
[lifecycle guide's pre-warming example](../../docs/topics/flow-control/lifecycle.md#limits)
so a disk write becomes visible at a boundary that actually reads it. Document
the inline deliverable read alongside the retained instructions in the
[composition guide](../../docs/topics/composition.md).

## Acceptance

- The reproduction exits 0 and the second phase uses the original prompt and
  lifecycle text. A change made by `initialize` before the first iteration
  takes effect on that iteration and is the basis of later iterations.
- Later ordinary iterations run without a startup reread or full launch
  schema verdict. Verify through a focused assertion at the preparation
  boundary or a trace identifying the iteration and read reason. The
  existing harness materialization span may also represent retry or resume,
  so a raw count of that span is insufficient.
- The lifecycle and post-iteration checks use the matching prepared context
  without a second observation of a group already captured in the same run.
  A behavioral test and the existing context work counters are sufficient
  evidence. This fix requires no timing benchmark.
- The new cases and the existing Claudine loop, lifecycle, retry, resume,
  proxy, sequence, and inline reconciliation tests pass. Run the area's
  Level 1 suite with `just test` in `claudine/`.

## Open Questions

None.
