# Control-flow spike (throwaway, run once)

This is the one bounded in-process experiment from the plan's Wave 2. It does
not touch production code. `loop_flow_spike.rs` was installed temporarily as
`claudine/lib/src/composition/lifecycle/tests/loop_flow_spike.rs`, registered
in that directory's `mod.rs`, and run with:

```sh
cargo nextest run -p claudine --lib -E 'test(loop_flow_spike)'
```

It passed 6/6 on 2026-10-16 (macOS) and was then removed from the crate. Do
not reinstall it. Later phases build the production seams below and test them
through public results.

The spike drives the **real** `ContainerRunner` and `LifecycleRunGuard` with
lifecycle stacks parsed by `parse_lifecycle_config`, a `RecordingEmitter`,
and fake primitive bodies. `break` and `prep` are not parsed yet, so the
spike stands in for them: a body returns a typed `Broken` outcome, and an
injected `ShellRunner` plays the part of a returning target call.

## Results and the interface contracts they support

| # | Test | Result | Contract for Phases 3–4 |
|---|---|---|---|
| 1 | `spike_break_crosses_a_container_with_exactly_once_cleanup` | Events: `initialize`, `start`, `finalize`. No terminal event, and `finalize` ran once. A second `finalize` request is ignored. Dropping the guard synthesized nothing | A break settles a container through the **existing** guard. The adapter calls `guard_mut().defuse()` (it marks the terminal slot settled without emitting), then `execute_event(Finalize)`. No new guard state is needed for "no event but `finalize`". The adapter returns a typed `Settled::Broken { target: LoopId, reason, code, partial }` to its parent, which repeats the same steps until the frame whose id matches `target` consumes it |
| 2 | `spike_failed_cleanup_defeats_a_pending_break` | A `finalize` `error` gives `Finished { failed: true }` with `primary_error` = "cleanup refused" | The break is discarded when `Finished { failed: true }` carries a primary error. A failure always wins, and no new recovery event runs after cleanup |
| 3 | `spike_pending_concurrent_break_waits_and_picks_declaration_order` | Legs finish c, b, a; the group breaks with b's reason and code. A handled leg failure (`fail_fast: false`) keeps the break; `fail_fast: true` turns it into a failure | Group pass settlement is a **pure fold over legs in declaration order**, run only after every leg (including queued `max_parallel` legs) has settled. A pending break never cancels or stops admission. The pass dictionary keeps `null` for legs without output |
| 4 | `spike_returning_action_continues_the_list_and_after_the_branch` | `start: [shell, stderr, {when, then: [shell]}, stderr]` called the injected runner twice inline. Both `stderr` items ran, including the one after the conditional branch | `prep` is an **action**, not a `StackControl`. It calls an injected runner and continues (`ActionStep::Continue`) on success, or errors (`ActionStep::Errored` carrying the child cause) on failure. The executor already resumes the enclosing list after a branch. Proposed seam: a `target_runner: &dyn TargetRunner` field on `StackExecutionContext`, beside `shell_runner`, with `fn call(&self, PrepRequest) -> Result<PrepReturn, LifecycleErrorInfo>`. `PrepRequest` carries the evaluated target, the overlay with origins, the `ActionLocation`, and the `CallDeadline`. Validators drop `prep` from the last-item rule; `break` keeps it |
| 5 | `spike_retry_restores_only_the_current_pass` | `replace_sequence_scope` restored outputs to `["A","B"]`, but the failed attempt's `k = 2` survived. `RuntimeState::from_snapshot(checkpoint)` restored both | **New seam needed**: `RuntimeState::restore_pass(&RuntimeSnapshot)` (name to be chosen by the coordinator). It restores `mutations` and `outputs` from the pass-entry checkpoint and keeps `proxy_chain`. It replaces `clear_outputs()` on looping retry; a non-looping container keeps today's behavior, because pass entry is invocation entry |
| 6 | `spike_call_ancestry_releases_and_limits_pass_through` | `enter_container_proxy` gave a legal repeated call three times, because the guard truncates on drop. While active, the callee could not call the caller. A branch built from a snapshot had its own chain. A child `CancellationToken` saw the parent's cancel. A child deadline kept the caller's earlier instant | Call ancestry reuses `RuntimeState::enter_container_proxy` and its RAII guard. That gives push/pop over one active chain shared with `proxy`, the 16-hop limit, and branch-local chains for parallel legs. The document-level `RunLedger` chain is append-only today. The Prep target runner adds the same guard there, and `proxy` keeps its append semantics. Cancellation: enter `cancellation::current().child()` for the call. Deadline: **no carrier exists**. Add `CallDeadline(Option<Instant>)`, an absolute instant computed from the command's start and `timeout`, and pass it in `PrepRequest`. The child's effective deadline is the earlier of the caller's instant and the child's own limit, so entering a call never resets the clock |

## Trace expectations for later tests

- A break from a container leg: `leg.start`, `leg.finalize`,
  `group.finalize`, then `owner.finalize`, where the owner is the loop
  frame. No `success`/`failure` event is emitted and no gate runs. The
  owner's summary records `broken` with its reason and code.
- A pending break that `finalize` fails: the primary error is the cleanup
  error, and the break appears nowhere in the summary.
- `prep` from `start`: the items before and after the call both run, and the
  child's events are reported under the child invocation only.
- Retry of pass 2: `seq.outputs` equals the pass 1 entries at the retried
  attempt's `start`. The failed attempt's mutations are absent and the
  retry's `with` overlay is applied after the restore.

## Not decided by the spike (kept for the owning tasks)

- `LoopAmbient.last_output` must become a `serde_json::Value` (string,
  array, or dictionary). This is a coordinator-owned shared type change.
- Where the loop frame's `LoopId` lives: a field on `LoopAmbient`, or a
  separate `LoopFrame` passed beside it. Either works with contract 1.
- The production name of `Settled`, which becomes a typed pass result
  (`PassOutcome`) owned by the Loop state machine task.
