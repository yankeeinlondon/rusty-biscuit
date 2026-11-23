---
kind: feature
created: 2026-11-22
status: finalized-spec
reviewed: true
reviewed-by: codex/gpt-6.2-sol
reviewed-on: "2026-11-23"
review-iterations: 0
needs-rulings: false
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
    reviewed-by: string -> the agent and model used in the spec review
    reviewed-on: date -> the date the spec was reviewed
    review-iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented-by: string -> the agent who implemented the plan
area: claudine
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-22-host-registration-v2
---

# Expose wrapper and agent PIDs through `current`

## Why this exists

Claudine runs as a wrapper and launches the selected agent CLI as an immediate
child process. The operating system assigns each process an ID (PID).
Claudine already captures both IDs for logs, reports, and hooks. A composed
document needs access to the same values in expressions.

The paths are `current.claudine_pid` and `current.agent_pid`, registered as
an additive extension of Darkmatter's reserved `current` root (see
[Decisions](#decisions)). One declaration gives runtime evaluation, expression checking, and the
editor the same field definitions. Claudine captures the prompt's `ctx`
snapshot before the provider child exists.

## Current behavior

Claudine already observes both PIDs during launch.

| Fact | Existing source and behavior |
| --- | --- |
| Wrapper PID | `std::process::id()` is injected into the provider environment as `CLAUDINE_PID` before spawn. |
| Agent PID | The wrapper reads the immediate child's ID at launch and carries it as `agent_pid` on process results and event metadata where available. |
| Logging and reporting | Wrapper records carry `env.claudine_pid`; post-spawn records carry `agent_pid`. JSONL omits an unavailable agent PID, while report and query projections expose it as nullable. |
| Provider environment | `CLAUDINE_PID` is present. `AGENT_PID` is deliberately **not** injected into the provider's initial environment; the spawn API does not return the child PID until after the child has been created. |
| Expression context | Darkmatter's `ctx` snapshot and its event-time `current` namespace use the same [context schema](../../../darkmatter/schemas/partials/context-variables.yaml), which declares neither PID. |

`current` can read values at event time, including after spawn. Darkmatter
currently admits only members of its context-variable catalog at runtime,
so the resolver needs the same additions as the schema.

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

`current.claudine_pid` contains the wrapper PID. When available,
`current.agent_pid` contains the child PID. Both IDs are positive integers.

- `claudine_pid` comes from the running wrapper's process ID, not from an
  environment-variable lookup. A caller cannot spoof it by supplying a
  conflicting `CLAUDINE_PID` in the launch environment.
- `agent_pid` is the immediate child ID returned by the successful provider
  spawn. It is not a descendant PID, process-group ID looked up later, session
  ID, or a value read back from the provider.
- A terminal lifecycle event retains the PID of the child that just ran even
  after that child exits. The value records which process ran.

Claudine injects `CLAUDINE_PID` into the provider environment. It records the
child ID as `agent_pid` in event metadata. An inherited variable named
`AGENT_PID` supplies no evidence of the child PID.

### Namespace schema

A mapping-valued `namespace-schema` declaration, written in the explicit
extension form described in [Decisions](#decisions), adds the following
members to Darkmatter's existing `current` shape.

| Member | Type | Meaning |
| --- | --- | --- |
| `claudine_pid` | `number(integer; min(1); required)` | The Claudine process running the workflow. |
| `agent_pid` | `number(integer; min(1)) \| null` | The attempt's immediate provider child, if spawned. |

Darkmatter supplies the base members from its shared context type. Resolve and
preserve those members' types, documentation, and definition locations.
Reject collisions with any base member or second extension of the same root
within a host, with diagnostics at the offending declarations. Keep the
extension tied to the selected host. A document analyzed under Darkmatter's
default profile or under another host uses its own catalog. When `md`
analyzes a document that selects the Claudine registration, its checker can
use the declarations. Evaluating the values still requires Claudine's
runtime evidence.

**Reader note.** The draft used required numbers for both fields, but a
missing child is normal in several events, including some failures. The
nullable type lets authors handle absence explicitly and gives the editor
the same contract as the runtime. In a Claudine run, an empty attempt returns
`null` as an observed value without a warning. Darkmatter's existing
`PartialRuntimeCapture` diagnostic still applies when an evaluator has the
declaration but has no provider capable of answering the read.

The verified registration bundle, the embedded catalog, and the generated
lifecycle artifact must use the same declaration. Regenerate the Claudine
artifacts with `just shipped-schemas` and publish them through the existing
host-registration flow. Include the new declaration and any bundled imports
in the catalog digest through the existing generator. Preserve member
descriptions and locations for Darkmatter's language server (DMLS).

### Availability around spawn

| Moment or scope | `current.claudine_pid` | `current.agent_pid` |
| --- | --- | --- |
| Prompt composition, including dry-run rendering | Claudine's PID when evaluated at runtime | `null`. Dry run fires no lifecycle events. |
| Document `initialize`, `start`, and pre-launch `blocked` | Claudine's PID | `null`. |
| Failure or cancellation before a successful spawn, including spawn errors | Claudine's PID | `null`, including during cleanup. |
| After a successful spawn, including `success`, `failure`, `cancelled`, and `finalize` for that attempt | Claudine's PID | That attempt's child ID, including after the child exits. |
| A prompt's loop gate after a provider pass | Claudine's PID | The child ID for the pass that just ran. |
| Events or gates owned by a sequence or group, or by work that launches no provider | Claudine's PID | `null`. These scopes own no provider child. |

The event name alone does not establish whether a child ran. For example,
`failure` can fire after `start` even if the spawn failed. To use the PID only
when present, a lifecycle stack can branch on
`when: "current.agent_pid != null"` and read the PID in its `then:` body.
Passive expression checking and editor requests use the declaration and
perform no PID reads or provider launches.

Preserve [Claudine's shell approval rules](../../docs/topics/pre-flight-checks.md).
Naming either PID does not make a lazy `current` read eligible for exact
pre-launch approval or for a sequence shell command. Apply the existing policy
for each command surface, including typed template approval where supported.
A lifecycle guard can check a PID at event time. This does not authorize
command interpolation or permit a new approval prompt after launch.

## Runtime ownership and scoping

### One observation path

Reuse the wrapper's existing PID observations. Do not create a second process
identity mechanism in the expression layer.

Publish each successful spawn's `child.id()` at the spawn boundary, before
protocol setup or other operations that can fail. Carry the observation
separately from the final process result so failure and cleanup events can read
it even if execution returns an error. Structured streaming, captured output,
inherited/interactive output, wire/RPC, and the retained steering launch must
share this contract. Use the same captured value for the existing result,
dispatch, and log fields wherever those paths already report one.

A provider launch with live steering can start a child that never becomes ready to
accept the prompt. If Claudine launches a fallback, that spawn starts with
an empty PID, then publishes its own ID on success. If the fallback fails
to spawn, terminal events see `null`. If Claudine ends the attempt without
launching a fallback, terminal events retain the steering child's ID. Keep
the existing record of whether any child started for lifecycle error routing,
even when a failed fallback leaves no PID to expose.

### Attempt-local agent PID

Allocate fresh agent PID state before the expressions of each attempt can
run, including before `initialize` or `start` when they fire. Associate the
state with the attempt whose provider will be launched. Retain it through
that attempt's cleanup and loop gate, then drop it before retry, resume,
the next loop pass, or proxy execution. An outgoing document's cleanup
reads its own state. An adopted or nested document starts with its own
state when it runs, including a target invoked by `prep`.

Give each prompt step and each parallel group leg independent state, even
when they share the same initial context snapshot. Sequence and group
events use their own provider-free scope. When nested work returns to a
parent action, subsequent parent reads use the parent's original state.

### Resolver and tooling parity

In Darkmatter, the lazy [`CurrentScope` resolver](../../../darkmatter/lib/src/markdown/compose/context/current.rs)
checks membership before it asks for a value. Admit additional members only
from the selected host's prepared declaration. Claudine supplies their values
through its existing refresh capability, scoped to the owning attempt.
Preserve the existing behavior for built-in members, including per-event
memoization and request-owned values. A `null` read before launch cannot
be cached into a later event.

Use the same catalog for expression binding, member and argument checking,
DMLS completion and hover, and runtime membership. Keep `current` reserved
in expression precedence. Bare `current` remains a key listing with null
placeholders for every member, even after a spawn. Enumerating keys performs
no refresh and emits no unavailable-value warning. Keep unknown members
unknown, including `current.ctx.*`, `current.env.*`, and either PID under
`ctx`. Do not widen the narrowing rules for other `current` members.

## Acceptance criteria

1. A wrapped fixture reports its real child PID. A later lifecycle event
   reads that same value and the actual wrapper PID, even with a conflicting
   launch-environment value. Existing logs, reports, and queries continue to
   record their existing PID fields, and Claudine continues to inject only
   `CLAUDINE_PID` as a PID environment variable.
2. Before spawn, after a failed spawn, and in a provider-free scope, reading
   the agent PID returns `null` without a missing-capability diagnostic.
   Dry run fires no events and does not spawn a provider. After a successful
   spawn, both normal completion and post-spawn errors retain the child's
   PID through terminal and cleanup events, including cancellation.
3. Cover structured streaming, captured output, inherited output, wire/RPC,
   and retained steering. Cover retained-child failure with no fallback,
   successful fallback, and failed fallback. Use the existing spawn fixtures
   and keep terminal and browser windows from gaining focus.
4. Cover retries, resumes, loop passes, proxy and `prep` targets, sibling
   steps, and overlapping group legs. Compare each observation to the PID
   captured for its own launch, and verify that a fresh attempt starts empty.
   Container events see no child PID, even after a child step finishes.
5. Claudine launch checking and DMLS agree on both members and their types.
   Cover completion and hover, a numeric read such as
   `min(current.claudine_pid, 5)`, and rejection of a numeric PID passed to
   a string-only parameter. Check nullable use with a fallback or another
   expression that handles `null`. A standalone Darkmatter catalog and
   other hosts gain neither member. Bare `current` lists both keys without
   calling the refresh provider, while bare `ctx` and `claudine context`
   exclude them.
6. Verify that the installed registration and the compiled catalog agree and
   that a mismatched lifecycle artifact follows existing validation. Cover
   the extension rules: an extension of `current` is accepted, an ordinary
   (non-extension) declaration of `current` and an extension of any other
   reserved root are refused, and a member colliding with a built-in
   `current` member or with a second extension in the same host is refused
   at the offending declaration.
7. Verify the existing shell approval boundary for direct and sequence-based
   composition. Neither new member can turn an unavailable runtime value into
   approved command bytes.
8. Use portable process APIs and fixtures for macOS, Linux, native Windows,
   and WSL2. Verify that reading or enumerating the fields adds no process
   discovery. A separate performance spike is unnecessary for reusing the
   existing PID observations.

## Decisions

### Claudine registers the PID fields as an additive extension of `current`

*Ruled by the author, 2026-11-23.*

Darkmatter's [host registration contract](../../../darkmatter/docs/topics/host-registration.md#namespace-schemas)
refuses a host namespace named after a reserved root, including `current`.
This feature changes that contract: a verified host registration may extend
`current`, and only `current`, through an explicit extension form in the
namespace envelope.

- The extension's mapping declares the added members only. Darkmatter
  supplies the built-in base from its shared context type.
- An extension never replaces, removes, or retypes a built-in member. A
  member that collides with a built-in, or with a second extension of
  `current` in the same host, is refused at the offending declaration.
- An ordinary (non-extension) declaration of `current`, and any extension of
  another reserved root (`fm`, `ctx`, `env`, `err`, `timing`), stay refused.
- Under a host that extends it, `current` becomes a superset of `ctx`: every
  `ctx` key, observed at event time, plus the host's members. `ctx` itself is
  never extended.

The change touches the host-registration reader, the Simplified Schema
namespace envelope, and the way Darkmatter types the reserved root for the
checker, DMLS, and the runtime membership test in `CurrentScope`.

Neither Darkmatter nor Claudine has been released, so no older reader has to
be supported. Darkmatter, DMLS, and the Claudine bundle change together, with
no upgrade order or compatibility shim.

**Rejected alternative: a Claudine-owned root such as `process`.** It would
need no Darkmatter change. It was rejected because it moves the requested
paths and splits event-time state across two roots. It would also make
Claudine define a second set of evaluation scopes that `current` already
has.

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
  `CLAUDINE_PID` environment variable, the `agent_pid` event metadata field,
  and the two expression members. Correct wording that makes `AGENT_PID`
  look like a second injected variable.
- Darkmatter's `docs/topics/host-registration.md` (the `current` extension
  form and its refusal rules), Darkmatter's context-variable documentation
  (`current` is a superset of `ctx` under a host that extends it), the
  Simplified Schema `namespace-schema` kind documentation, and the repository
  skills that describe the catalog (`darkmatter`, `simplified-schema`,
  `claudine`).
- Claudine's README overview and applicable caller-skill source under
  `docs/use-skills/`, followed by the normal skill publication flow.
