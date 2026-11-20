---
kind: fix
created: 2026-11-20
status: draft-spec
area: claudine
packages:
    - claudine-cli
reviewed: false
implemented: false
---

# Loop Iterations Reread Their Prompt and Fire Lifecycle Events Against a Stale `ctx`

## The defect

Editing a looping prompt while it runs crashes the next iteration's `start`
event when the edit references a `ctx.*` variable from a context group the
original text never named:

```text
⤫ CompositionError: lifecycle evaluation error
┃ A late-binding expression raised while the `start` lifecycle event was firing,
┃ in `start[0].message` (…/prompts/_implement/implement-plan.md).
┃
┃ Reason: interpolation of `ctx.branch` failed: context variable `ctx.branch`
┃ belongs to the Git capture group, which this composition request did not
┃ capture; …
```

Observed while `prompts/_implement/implement-plan.md` ran phase by phase and
commit `daa04d786` added `{{ctx.branch}}` and `{{ctx.hostname}}` to its
lifecycle messages. Before that commit the prompt named no Git or Os group
variable, so neither group was captured for the run.

The crash is the visible symptom of two defects:

1. **Later loop iterations reread the document from disk.** The documented
   contract is that "later loop iterations reuse the audited structural plan"
   (`claudine/docs/pipeline.md`), and the entry matrix gives
   `DocumentEntryReason::LoopIteration` the `StampedStructuralPlan` basis with
   no full validation (`claudine/lib/src/composition/prepare/entry.rs`). In
   practice every iteration after the first runs the harness's stabilized
   reread, re-composing and re-validating the file as it now stands on disk.
2. **A stabilized reread leaves the lifecycle guard on the earlier read's
   context.** The reread installs its own lifecycle on the guard but not the
   `ctx` snapshot it composed with, so the guard pairs new lifecycle text with a
   context captured for different text. Any lifecycle expression that names a
   group only the reread text requires raises `ContextNotCaptured`.

### Reproduction

Reproduced without code changes in a throwaway Git repository holding a copy of
`prompts/`, `.claudine/`, and a spec/plan pair with `total_phases: 2`:

1. Check out the pre-`daa04d786` `implement-plan.md` (shell actions swapped for
   `info:` so the run needs no approvals).
2. Put a stub `claude` on `PATH` that, on its first launch only, overwrites
   `implement-plan.md` with the current version, then prints a successful
   stream-json `result` line.
3. Run `claudine compose --claude prompts/implement.md spec=<spec>`.

Phase 1 succeeds. Phase 2's `start[0].message` raises the error above (for
`ctx.hostname`, whose Os group was also never captured; which missing variable
is reported first depends on evaluation order), followed by the same error
from `failure.message`. Exit code 1.

## Why it happens

```mermaid
sequenceDiagram
    participant Loop as compose loop (prep.rs)
    participant Guard as LifecycleRunGuard
    participant Harness as harness attempt (loop_control.rs)
    participant Disk as prompt on disk

    Note over Loop: iteration ≥ 2
    Loop->>Loop: capture ctx for in-memory source (old text)
    Loop->>Guard: set_run_prepared_context(old ctx)
    Loop->>Harness: request (schema_verdict_deferred = true, inherited)
    Harness->>Disk: stabilized reread (new text)
    Harness->>Harness: compose with epoch ctx extended for new text
    Harness->>Guard: set_config(new lifecycle)
    Note over Guard: context stays old ctx
    Harness->>Guard: fire start
    Guard-->>Harness: ContextNotCaptured (ctx.branch)
```

- **The deferral flag leaks into later iterations.**
  `claudine/cli/src/commands/compose/prep.rs` builds the loop's shared
  `PrepareOptions` with
  `defer_schema_verdict: schema_stage == SchemaStage::DeferToStabilizedReread`
  (around line 1627). That is correct for iteration 1. Later iterations clone
  the same options and call `prepare_without_schema`, which copies the flag
  onto the prepared composition as `schema_verdict_deferred: true`
  (`claudine/lib/src/composition/prepare.rs`, `prepare_direct_on`). The runner
  then computes `stabilize_after_initialize = schema_verdict_deferred && …`
  (`claudine/cli/src/commands/wrap/composition/runner.rs`, around line 165),
  so the harness arms `BootstrapStage::StabilizeOnly` for every iteration and
  `bootstrap_adopted_document_phase` rereads the file
  (`claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`, stage 4).
- **The guard keeps the earlier snapshot.** In `bootstrap_adopted_document_phase`
  the reread's lifecycle is installed with `set_config(stabilized_lifecycle)`,
  but the reread's `materialized.compose_context` is installed on the guard
  only for a proxied target (`if stage != BootstrapStage::StabilizeOnly`). For a
  `StabilizeOnly` reread the guard keeps the context installed earlier: for
  iteration ≥ 2, the one `prep.rs` captured from the in-memory source and set
  with `guard.set_run_prepared_context` (around line 1233); for iteration 1,
  the coordinator's prepared snapshot. `LifecycleRunGuard::effective_prepared_context`
  (`darkmatter/lib/src/lifecycle/run_guard.rs`) returns that snapshot.
  `install_reentry_lifecycle_context` swaps the snapshot only for `Retry` and
  `Resume`.

The reread's own context is not a second capture: `harness_prepare_options`
(`claudine/cli/src/commands/wrap/harness_orch/prompt.rs`) extends the
retained epoch snapshot with exactly the groups the reread text requires. Body
and frontmatter composition already use it; only the lifecycle guard does not.

## What should happen

- **R1 — Later loop iterations do not reread their document.** An iteration
  after the first composes from the stabilized source the loop already holds
  (the reread iteration 1 took, or the caller-resolved source when there was
  none), exactly as `pipeline.md` documents. An edit to the prompt file made
  while the loop runs takes effect on the next run, not the next iteration,
  matching how a mid-run `$schema` edit already behaves
  (`claudine/docs/topics/composition.md`). Only iteration 1 may carry a
  deferred schema verdict.
- **R2 — A stabilized reread's lifecycle and context come from the same read.**
  Whenever the harness replaces the active document with a reread (stage 4 of
  the staged boot, for a direct or adopted document), the lifecycle guard
  receives that read's context alongside its lifecycle. It stays the same
  document epoch's snapshot, extended for any newly required groups, so
  launch-facing values do not change and nothing is captured twice. A direct
  document's `initialize` that rewrites the file to name a new `ctx.*` group in
  a lifecycle stack therefore evaluates that stack without error.
- **R3 — Retry and resume are unchanged.** They already reread and install the
  reread's context; a prompt edited before a retry is still honored by that
  retry, as documented in `flow-control-reference.md`.

## Design decisions

- **Fix both defects, not only the guard.** R2 alone would stop the crash, but
  iterations would keep rereading and re-validating, against the documented
  contract and the entry matrix, and a prompt could change underneath a running
  loop. The reread also costs a full composition per iteration. R1 alone would
  leave iteration 1 exposed when `initialize` rewrites the prompt.
- **Mid-run prompt edits are not picked up.** This follows the existing
  documentation rather than introducing a new behavior. Picking up edits would
  require a deliberate design (re-auditing new shell commands each iteration,
  deciding which loop state survives a structural change) and is out of scope.
- **Where R1 is enforced.** Clear `defer_schema_verdict` on the options for
  iterations after the first in the loop closure in `prep.rs`, next to the
  existing `ctx.iteration > 1` context recapture, rather than special-casing
  the runner. The runner's `stabilize_after_initialize` rule stays as is: it is
  correct for any composition that genuinely deferred.
- **Where R2 is enforced.** In `bootstrap_adopted_document_phase`, install
  `materialized.compose_context` on the guard for `StabilizeOnly` as well, and
  record the `Lifecycle` prepared-context consumer on the document epoch as
  `install_reentry_lifecycle_context` does. Update the comment above the
  `stage != BootstrapStage::StabilizeOnly` branch, which currently states that
  replacing a direct document's lifecycle context "would substitute a second
  capture"; that holds for the launch identity rebuild but not for the epoch
  snapshot.

## Implementation and verification

### Phase 1 — Failing tests

- L1 lifecycle trace (`claudine/cli/tests/l1/lifecycle_traces.rs`): a looping
  document (`loop:` with two iterations) whose `start` stack names only
  launch-safe variables, driven by a provider stub that, on its first launch,
  rewrites the document so `start` also interpolates `ctx.branch` and
  `ctx.hostname`. Expect both iterations to run, `start` to render with the
  **original** text on iteration 2, and exit 0. This fails today.
- L1 test for R2: a directly invoked, non-looping document whose `initialize`
  rewrites its own file so a lifecycle stack names a context group the
  bootstrap text did not. Expect the stack to evaluate. Confirm whether this
  fails today; if it does not, record why in the implementation log and keep
  the test as a guard.
- Unit test in the loop closure (or an existing loop preparation test) that
  iteration ≥ 2's prepared composition has `schema_verdict_deferred == false`.

### Phase 2 — Fix

- Apply R1 in `claudine/cli/src/commands/compose/prep.rs`.
- Apply R2 in `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`.
- Check the sequence step loop path (`claudine/cli/src/commands/wrap/sequence/`)
  for the same deferral inheritance and the same guard pairing; fix it if it
  exhibits either.
- Fix comments that drifted with the behavior (the `StabilizeOnly` branch
  comment; any comment in `prep.rs` claiming later iterations re-read).

### Phase 3 — Docs

- `claudine/docs/topics/flow-control/looping.md`: state that a document's text
  is fixed for the whole loop run once iteration 1 stabilizes, that edits take
  effect on the next run, and that each iteration still observes fresh Git
  working state through `ctx.*`.
- `claudine/docs/pipeline.md`: keep "Later loop iterations reuse the audited
  structural plan"; add that a stabilized reread hands its context to the
  lifecycle events along with its lifecycle.

## Acceptance

- The reproduction above exits 0, and phase 2's `start` renders the original
  message.
- No loop iteration after the first triggers a stabilized reread (verifiable
  in `--debug debug` output: no `harness_materialize_prompt` span on
  iterations ≥ 2 unless a retry or resume occurred).
- The new tests pass; the existing `lifecycle_traces`, loop, retry, resume, and
  proxy tests pass unchanged.
- The looping and pipeline docs describe the behavior above.

## Open Questions

- None blocking. If the author prefers loops to pick up mid-run prompt edits,
  R1 is replaced by a design for per-iteration rereads (re-audit, loop state
  carry-over); R2 is needed either way.
