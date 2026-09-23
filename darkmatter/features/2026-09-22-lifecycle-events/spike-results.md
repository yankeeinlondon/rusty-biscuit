# Control-flow spike: decision and evidence

**Status:** Control-flow experiment complete; validation results below  
**Date:** 2026-09-19

## Decision

Prefer a **host-driven engine that returns typed work requests**. Darkmatter
should own the pending transition and accounting; the host performs requested
work and reports facts. The engine is synchronous and inert between calls. It
needs no provider process handles, async runtime, shell runner, or background
worker to make control decisions.

Keep ordinary custom-event emission as a checked API, but do not use arbitrary
`emit(event)` calls as the way hosts advance terminal, cleanup, or recovery
state. The same state owner should validate both surfaces.

The experiment supports the ownership direction. It also shows that the draft's
simple event-role vocabulary needs **recovery mode** and **failure-stage facts**.
A full migration is not yet proven safe.

## What was built

- `spike/model.rs`: a dependency-free Rust prototype with registered profile
  names, fixed semantic roles, a catch router, retry/resume admission, a request/
  reply driver, and a contrasting host-driven emission validator.
- `spike/scenarios.rs`: deterministic two-host traces and protocol misuse tests.
  A publisher has its own success/failure/cleanup names plus before/after events;
  it does not use Claudine types. Fake work and mutations are in-memory only.
- `darkmatter/lib/tests/lifecycle_control_flow_spike.rs`: compiles and tests the
  neutral prototype in the Darkmatter package.
- `claudine/lib/tests/lifecycle_control_flow_spike.rs`: compares the prototype
  against the **actual** public Claudine catch and control APIs, and records
  production guard behavior for downgrade and re-entry.
- `transition-table.md`: source-derived baseline, including CLI behavior outside
  those public APIs.

No production module, dependency, Cargo manifest, effect adapter, or CLI behavior
was changed. The model is intentionally not exported from Darkmatter. Tests use
normal Cargo-discovered test targets and nextest rather than a new crate or CI
workflow.

## Findings

### Finalize is not an unconditional finally block

`drive_terminal_recovery` dispatches a terminal handler's recovery **before**
finalize. An admitted retry/resume skips finalize for that attempt; a proxy skips
source finalize. A recovery requested inside normal finalize occurs after that
finalize has run. The prototype exercises both retry positions and proxy adoption.

Moving finalize ahead of all recovery would be a behavior change, even if it
looked like cleaner resource management. Preserve this policy during extraction;
any redesign belongs in a separate decision.

### The same event has two recovery modes

Normal finalize is a recovery surface. Finalize running as cleanup after an
expression failure is not: its control is ignored, while a later evaluation
error can become the reported failure. The catch protocol also keeps setup error
and evaluation error separate.

The differential experiment enumerates origin event, terminal slot, finalize
state, prior error, and three independently supplied handler outcomes. It checks
request order, error payloads, blocked-slot redesignation, winning evaluation,
and returned control against production. Some combinations are deliberately
unreachable through a well-formed host; they probe the public protocol's complete
input matrix rather than claiming 24,192 distinct real-world workflows.

### One pure function is not the complete lifecycle

`decide_lifecycle_transition`, `LifecycleCatchProtocol`, the guard, terminal
explicit-error downgrade, and CLI recovery each own part of today's behavior.
The prototype's full request driver is checked with hand-authored traces grounded
in those sources; only the catch and admission subsets have differential proof.

This distinction matters particularly for blocked: normal terminal downgrade can
use the failure handler's recovery, while the setup catch protocol does not
return that handler's control. A production engine must select the mode from the
entry route, not simply the event name.

### Failure classification needs more than a launched flag

An incompatible resume after start but before spawn routes through failure and
finalize. It is not equivalent to a preparation denial. The production adapter
must report entry reason and failure stage, with the selected diagnostic.

The prototype does not reproduce every stage of refused resume or handoff. Its
recovery acknowledgement tests establish suspension, refusal, and no premature
identity reset; they are not equivalence evidence for all refusal routes.

### The useful extension is a profile, not just a name

The publisher runs through the same engine with `publisher.published`,
`publisher.rejected`, and `publisher.closed` mapped to outcome/cleanup roles.
Both successful publication and a typed publication failure reach the registered
outcome and cleanup roles. Before/after ordinary events do not acquire terminal
privileges from their names. This demonstrates a non-agent use of the protocol, though not yet a
complete publishing integration or reusable parser/effect boundary.

The aliases `start` and `composed` resolve to the same role. Registered names
cannot shadow either spelling. Duplicate authored YAML declarations are outside
this spike because it has no YAML parser.

## Comparison of the two designs

| Concern | Host chooses next event; library validates | Engine returns next work request |
|---|---|---|
| Missing cleanup | Thin validator detects omission at finish; host must repair/drive cleanup | Next pending request is cleanup; no normal completion response can skip it |
| Error precedence | Host must select protocol and thread errors, or validator must implement the state machine too | Engine retains active error and catch mode |
| Retry/proxy | Host sequences resets and effects | Request suspends state; acknowledgement commits the transition |
| Duplicate/stale responses | Requires additional per-operation accounting | Run identity and monotonically increasing request generation reject them |
| Provider knowledge | Can remain outside library | Can remain outside library |
| Extensibility | Simple for ordinary notifications | Profiles map custom names to bounded roles; ordinary emissions still need a checked seam |

This is not proof that every host-driven API is inferior. A sufficiently complete
validator can enforce the same rules; it then contains essentially the same
state machine, with the host still guessing what transition to request. The
request API makes legal next work discoverable and removes that duplication.
Neither API can force a host to keep polling after it abandons a run, undo an
external side effect, or guarantee cleanup after a process crash.

## Proposed production API shape

Use the prototype's `pending()` / `reply(ticket, response)` shape, but strengthen
the experimental enums before production:

```rust,ignore
let mut run = profile.begin(document_identity, captured_context)?;
while let Some(work) = run.next_request() {
    let response = match work.kind() {
        Request::EvaluateEvent(event) => evaluator.evaluate(event, run.view()),
        Request::Prepare(entry) => host.prepare(entry),
        Request::PerformOperation(operation) => host.perform(operation),
        Request::Recover(recovery) => host.recover(recovery),
    };
    run.complete(work.ticket(), response)?;
}
let outcome = run.finish()?;
```

The concrete API must permit releasing borrows before awaiting host work. Return
owned request data or stable handles, not a mutable borrow of the run across an
await. Use opaque tickets tied to run/document/attempt identity. Wrong-kind,
stale, duplicate, and foreign responses must fail without changing state.

Production responses need typed diagnostics and operation-specific facts:
preparation success, preflight authorization tied to an epoch, operation start
and completion, completion verdict, session availability/compatibility, budget
admission, and proxy adoption/refusal. `bool approved` is sufficient only for the
in-memory experiment. Caller assertions of approval are not a security boundary;
actual effect dispatch must consult the existing approved-command authority.

Return recovery intents with their evaluated arguments and source provenance.
Keep delay/backoff, distinct attempt ceilings, and cleanup mode explicit. The
host owns provider/session compatibility, budget ledger admission, path resolution,
and proxy-chain facts. The engine owns when those facts can advance the run.

Prefer named error/exit modes such as normal recovery, fatal-evaluation cleanup,
and refusal cleanup over a collection of booleans. The prototype uses booleans
for brevity; the discovered distinctions justify stronger production types.

## Verification

Executed on macOS using nextest through package-area recipes:

| Working directory | Command | Result |
|---|---|---|
| `claudine/` | `just test-library --test lifecycle_control_flow_spike` | 4 passed, 0 skipped |
| `darkmatter/` | `just test --test lifecycle_control_flow_spike` | 15 passed, 0 skipped |
| `claudine/` | `just test-cli --bin claudine harness_orch::loop_control::tests` | 118 passed; 1,663 unrelated tests filtered out |
| `darkmatter/` | `just lint` | Four Rust package lint checks passed; Zed WASM check blocked by missing `wasm32-wasip2` target |
| `darkmatter/` | `just _lint darkmatter` | Passed after the final publisher failure scenario was added |
| `claudine/` | `just lint` | Passed, including 8 error-transport guards |

The catch comparison checks **24,192 input combinations**; control admission
checks **160 combinations**. These are assertions within tests, not thousands
of independently scheduled nextest cases. The existing 118 CLI tests establish
that the inspected downgrade, recovery, refusal, budget, and adoption behavior
passes in this checkout; they do not run against the prototype.

All new scenarios are silent L1 tests: no shell, network, audio, provider,
terminal-window, or browser work. The existing CLI test binary emitted a macOS
linker warning about the size of its unwind table; its tests passed. No other
OS execution evidence was collected in this spike. The Zed extension target is
unrelated to the changed test-only protocol; it was not installed for this work.

## Limits and next implementation gate

This is a bounded control-flow spike, not the complete extraction prototype:

- No lifecycle parser, payload schema, DMLS descriptor transport, real stack
  evaluation, shell audit, file mutation, or terminal rendering is moved.
- Fake state mutation verifies host scheduling only; existing executor semantics
  such as ordered `set` snapshots are not re-proven here.
- The driver supports one before and one after event, fixed outcome roles, and
  one additional retry/resume. Generic event groups, optional core-only profiles,
  payload validation, backoff, skip/stop/defer, and cancellation are not modeled.
- Proxy acknowledgements use opaque identities. Cycle/hop limits, overlays,
  target bootstrap failure, and coordinator commit semantics stay unmodeled.
- The shell flag tests permission propagation only, not production shell safety.
- Refusal sub-stages, sequence task/group coordination, and loop expression/
  mutation ordering require integration traces before replacement of production
  orchestration. The modeled loop checks re-entry and post-finalize failure only.

The next gate is an adapter that records real CLI orchestration traces with fake
services, especially both blocked routes, refused resume after re-entry, failed
proxy bootstrap, and sequence task cleanup. Then extract the existing evaluator
behind the selected protocol. Do not take the passing subset as authorization to
replace all lifecycle orchestration in one step.

## Investigation quality

GitNexus was bound to the current worktree (`rusty`, index commit `a0eedb5`). Its
`decide_lifecycle_transition` impact was `UNKNOWN`, with no resolved callers,
and its context result carried an inconsistent source path. Source search
confirmed the CLI control dispatcher caller and catch-protocol use in CLI and
looping code. No production symbols were edited; the uncertainty is recorded
rather than interpreted as low impact. The source table documents the actual
inspected boundaries.
