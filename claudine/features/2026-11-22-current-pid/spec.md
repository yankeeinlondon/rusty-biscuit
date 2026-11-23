---
kind: feature
created: 2026-11-22
status: draft-spec
area: claudine
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-22-host-registration-v2
---

# Expose wrapper and agent PIDs through `current`

## Why this exists

A wrapped invocation has two processes: the Claudine wrapper and the immediate
agentic-CLI child it spawns. Claudine already captures both process IDs for
logs, reports, and hooks, but a composed document cannot read them through its
runtime context.

This feature exposes those existing observations as `current.claudine_pid` and
`current.agent_pid`. It also gives `current` a Claudine-owned namespace schema
so runtime evaluation, static expression checking, and editor tooling agree on
the expanded shape. The eager `ctx` namespace does not change: it is captured
while the prompt is being composed, before the provider child exists.

## Current behavior

The PID sources and names already exist and must be reused rather than
replaced with process-table discovery:

| Fact | Existing source and behavior |
| --- | --- |
| Wrapper PID | `std::process::id()` is injected into the provider environment as `CLAUDINE_PID` before spawn. |
| Agent PID | The immediate child's `child.id()` is captured after every successful spawn and carried as `agent_pid` on `ProcessResult` and `EventMeta`. |
| Logging and reporting | Wrapper records carry `env.claudine_pid`; post-spawn records carry `agent_pid`. JSONL omits an unavailable agent PID, while report and query projections expose it as nullable. |
| Provider environment | `CLAUDINE_PID` is present. `AGENT_PID` is deliberately **not** injected into the provider's initial environment; the spawn API does not return the child PID until after the child has been created. |
| Expression context | `ctx` and `current` currently share Darkmatter's context-variable shape. Neither declares `claudine_pid` or `agent_pid`. |

`current` is the appropriate surface because it is evaluated lazily at event
time. Adding the fields only to a schema is not sufficient: Darkmatter's lazy
resolver currently admits only members of its context-variable catalog, so the
runtime resolver must learn the two additional members as well.

## Goals

- Expose the wrapper PID as `current.claudine_pid` in every Claudine lifecycle
  event where `current` is available.
- After a successful provider spawn, expose that attempt's immediate child PID
  as `current.agent_pid`.
- Preserve the existing PID capture, environment, JSONL, report, and query
  behavior.
- Declare the expanded `current` shape once through Claudine's host
  `namespace-schema` registration, reusing Darkmatter's context type rather
  than copying its members.
- Make runtime evaluation, pre-launch expression analysis, DMLS hover and type
  checking, and bare-`current` enumeration agree on the two fields.
- Keep agent PID state scoped to one provider attempt so retries, loop passes,
  sequence steps, and parallel group legs cannot observe one another's PID.

## Non-goals

- Adding either PID to `ctx`, or changing when `ctx` is captured.
- Adding `AGENT_PID` to the provider's initial environment. Doing that would
  require a launcher/shim or provider cooperation and is separate from this
  feature. Existing `CLAUDINE_PID` injection remains unchanged.
- Discovering provider descendants, scanning the OS process table, checking
  whether a PID is still alive, or protecting against PID reuse. The values
  identify the processes observed for this run; they are not liveness tokens.
- Changing process-group, signal, timeout, or Job Object behavior.
- Changing the JSONL or reporting schemas.

## Public contract

### Values and identity

`current.claudine_pid` and `current.agent_pid` are numbers containing the
decimal PID values already captured by the wrapper:

- `claudine_pid` comes from the running wrapper's process ID, not from an
  environment-variable lookup. A caller cannot spoof it by supplying a
  conflicting `CLAUDINE_PID` in the launch environment.
- `agent_pid` is the immediate child ID returned by the successful provider
  spawn. It is not a descendant PID, process-group ID looked up later, session
  ID, or a value read back from the provider.
- A terminal lifecycle event retains the PID of the child that just ran even
  after that child exits. `current` reports run identity here, not current
  process liveness.

The provider environment continues to use the matching uppercase names:
`CLAUDINE_PID` for the wrapper and the existing internal/log field name
`AGENT_PID`/`agent_pid` for the child. No environment rename is needed.

### Namespace schema

Add a Claudine registration member conceptually equivalent to:

```yaml
kind: namespace-schema
namespace: current
description: "The live context for this composition run, including its wrapper and provider process IDs."
$schema: "{ ...context@darkmatter:context-variables, claudine_pid: number(required), agent_pid: number(required) }"
```

`...context@darkmatter:context-variables` is the concrete named-type spelling
for the requested `...ctx` composition: it inherits the existing context shape
without adding either PID to `ctx` itself. If the host-registration work
provides a direct `...ctx` global-spread spelling with identical semantics,
that spelling may be used instead; there must still be only one inherited
context definition.

The resulting `current` shape contains every existing `ctx` member plus:

| Member | Type | Meaning |
| --- | --- | --- |
| `claudine_pid` | `number(required)` | The Claudine wrapper's PID. |
| `agent_pid` | `number(required)` | The immediate provider child's PID once that attempt has spawned successfully. |

Darkmatter currently refuses a host namespace named `current` because it is a
reserved global. This feature includes the narrow registration/catalog support
needed for a host to **extend** `current` with this declared shape. It must not
open general replacement of `fm`, `ctx`, `env`, `err`, `timing`, or other
reserved roots. The built-in `current` definition remains the base for other
hosts and for `md`; Claudine's extension applies only where Claudine's host
catalog applies.

The namespace file participates in the normal registration inventory and
lifecycle catalog hash. The generated lifecycle artifact and any shipped host
catalog artifact must be regenerated through their existing generators, not
hand-edited.

### Availability around spawn

The schema declares the member for static analysis; it cannot make a child PID
exist before the OS creates the child. No implementation may fabricate `0`,
reuse the wrapper PID, or leak a previous attempt's PID to satisfy the schema.

| Moment | `current.claudine_pid` | `current.agent_pid` |
| --- | --- | --- |
| `initialize` | Available | No provider child exists yet. |
| `start` | Available | No provider child exists yet; `start` fires before spawn. |
| Pre-launch `blocked`, dry run, or failed spawn | Available | Unavailable. |
| Post-spawn `success`, `failure`, and `finalize` | Available | The just-completed attempt's PID. |
| Loop gate after a provider pass | Available | That pass's PID. |

A read of `current.agent_pid` when no child has been spawned follows the
existing lazy-`current` unavailable-value behavior: it fails closed with the
established diagnostic rather than probing the host or returning a stale
value. The `required` declaration must not cause otherwise valid
`initialize`, `start`, or pre-launch `blocked` stacks to fail merely because
no agent PID exists yet. Passive preflight and editor analysis perform no
process lookup and no spawn.

## Runtime ownership and scoping

### One observation path

Reuse the wrapper's existing PID observations. Do not create a second process
identity mechanism in the expression layer.

Publish the successful spawn's `child.id()` immediately to run-scoped state
that the lazy `current` provider can read. Waiting until `ProcessResult`
returns would be too late for any event or runtime consumer that runs while
the child is active. All spawn paths — structured streaming, captured output,
inherited/interactive output, and wire/RPC execution — must publish through
the same contract.

### Attempt-local agent PID

`claudine_pid` is stable for the wrapper process, but `agent_pid` belongs to
one provider attempt:

- Clear or initialize the attempt's agent PID to unavailable before every new
  provider attempt.
- Set it only after that attempt's spawn succeeds.
- Preserve it for that attempt's terminal and finalize events.
- Never carry it into the next retry, resume, loop pass, proxy attempt, or
  sequence step.
- Give concurrent group legs independent state. A process-global singleton or
  one mutable slot on the invocation-wide refresh provider is not sufficient.

Existing `ProcessResult.agent_pid`, dispatch metadata, and log records continue
to receive the same value. The new runtime read must not reset or otherwise
change those surfaces.

### Resolver and tooling parity

Extend the lazy `current` resolver so the two Claudine-owned keys reach the
run-scoped provider instead of being rejected by Darkmatter's built-in
`ContextGroup` membership check. Existing context keys keep their current
capture and refresh behavior. Bare `current` enumeration includes both new
keys without observing processes or forcing a spawn.

Claudine's expression binding and global-catalog consumers must use the
extended shape consistently. `ctx.claudine_pid` and `ctx.agent_pid` remain
unknown paths, as do the removed nested spellings `current.ctx.*` and
`current.env.*`.

## Acceptance criteria

1. A real wrapped run can read `current.claudine_pid` and
   `current.agent_pid` from a post-spawn lifecycle event, and the values equal
   the wrapper and immediate child PIDs already recorded for that run.
2. `current.claudine_pid` is available before spawn. A pre-spawn or
   no-provider run never fabricates an agent PID and never borrows one from an
   earlier attempt.
3. Structured, captured, inherited/interactive, and wire/RPC spawn paths all
   expose the correct post-spawn agent PID.
4. A failed spawn leaves `agent_pid` unavailable. A later successful retry
   exposes only the new child PID.
5. Consecutive loop passes and sequence steps observe their own provider PID;
   parallel group legs cannot read a sibling's PID.
6. The namespace catalog and generated artifacts type both new members as
   required numbers. Static analysis accepts numeric uses and diagnoses a
   string argument where a number is required, before launch and in DMLS.
7. Static analysis and runtime still reject `ctx.claudine_pid`,
   `ctx.agent_pid`, and unknown `current` members.
8. Bare `current` includes `claudine_pid` and `agent_pid` without triggering
   host discovery; bare `ctx` and the `claudine context` output do not acquire
   the new fields.
9. Existing tests prove that the provider still receives `CLAUDINE_PID`, that
   no new initial `AGENT_PID` injection is claimed, and that existing JSONL,
   report, and query PID fields are unchanged.
10. Behavior and tests pass on macOS, Linux, native Windows, and WSL2 using
    process IDs as OS-assigned integers, with no Unix-only signal or process
    table dependency.

## Documentation drift

The implementation must update the canonical documentation for:

- `docs/topics/state-management/context-variables.md`: `current` now extends
  the `ctx` shape with the two runtime-only PID members; document pre-spawn
  availability.
- `docs/topics/flow-control/lifecycle.md` and
  `docs/topics/composition.md`: event-time PID reads and per-attempt scoping.
- `docs/topics/schemas.md`: Claudine's `current` namespace registration and
  generated-artifact behavior.
- `docs/topics/wrapped-execution-switches.md` and
  `docs/topics/traces-and-logging.md`: distinguish the provider's
  `CLAUDINE_PID` environment variable, the internally tracked `AGENT_PID`, and
  the two `current` expression members without implying that `AGENT_PID` is
  injected into the provider.
- Applicable caller-skill source under `docs/use-skills/`, followed by the
  normal skill publication flow.
