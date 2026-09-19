# Darkmatter-Owned Document Lifecycle

**Status:** Draft for review  
**Date:** 2026-09-19

The control-flow experiment is recorded in [spike-results.md](spike-results.md),
with the source-derived baseline in [transition-table.md](transition-table.md).
It favors typed engine requests and identifies recovery mode and failure stage
as additional requirements beyond event roles.

## Recommendation

Move the document lifecycle language and reusable execution engine into
Darkmatter. Darkmatter defines `initialize`, `blocked`, and `composed`; `start`
is an alias for `composed`. Claudine registers `success`, `failure`, `finalize`,
and `loop` as its continuation after the core composition phase.

The strongest benefit is one semantic authority for lifecycle parsing,
validation, deferred evaluation, state mutation, and event descriptors alongside
the expressions and effects these features already use. Other document hosts
can reuse that authority without depending on an agent wrapper. DMLS can consume
the same passive definitions as the runtime.

The strongest objection is also valid: the events form a control protocol, not
just a list of callbacks. Registration must preserve terminal selection,
finalization, recovery, and shell authorization. This specification makes those
obligations explicit instead of moving provider orchestration into Darkmatter.

Proceed in stages, beginning with a boundary prototype and recorded behavioral
baselines. This is a substantial extraction, not an enum relocation. The
25–45 engineer-day allowance in the impact assessment is an initial estimate,
not a delivery commitment; re-estimate after the prototype.

## Assessment of the supporting documents

- **Benefit statement:** convincing on language ownership, reusable execution,
  and passive authoring metadata. Neither faster builds nor automatic portability
  follows from the move, and neither is needed to justify it.
- **Against statement:** its annotations correctly identify several objections
  as solvable boundary requirements. Automatic lifecycle execution during ordinary
  composition is avoidable. However, ordering and re-entry remain substantive
  design problems. Position alone cannot authorize shells, and a passive editor
  consumer does not prove that the runtime works for a second execution host.
- **Impact statement:** its move/decouple/generalize/integrate/prove breakdown
  is useful. Its proposed six-event universal profile is not adopted here:
  success, failure, and finalization describe the caller's continuation and are
  registered by Claudine. The engine can enforce their generic roles without
  making their names Darkmatter core events.

The planned DMLS extension work is an integration opportunity, not a reason to
retain duplicate lifecycle knowledge. Its delivery must be verified before
claiming editor parity for registered events.

## Scope and ownership

Use `darkmatter::lifecycle` initially. Introduce another crate only if a concrete
dependency or build constraint requires it. Dependency direction remains
Claudine → Darkmatter; Darkmatter must not depend on Claudine, its CLI, provider
catalog, or rendezvous services.

| Darkmatter owns | Claudine owns |
|---|---|
| Core event identities and aliases | Its registered event profile |
| Stack AST, parser, validation, source maps | Provider hooks and their separate 16-event model |
| Registry, passive descriptors, event scope | Provider launch, streams, sessions, and cancellation |
| Ordered evaluation using existing expressions and effects | Provider-specific diagnostic selection and outcome facts |
| Run state and enforcement of registered transition roles | Completion verdict policy and inline document closure |
| Phase permissions and authorized effect dispatch | Approval interaction and configured host services |
| Generic recovery requests and transition accounting | Retry/resume execution, proxy preparation, budgets, sequences |

Do not introduce a second expression language, arbitrary transition graphs,
dynamic plugins, custom action languages, the `defer` scheduler, or a provider-hook
migration. Do not change binding time, completion criteria, or retry policy as
part of extraction.

Ordinary compose, render, schema, and editor APIs do not execute lifecycle stacks.
Execution requires an explicit run created and driven by a host. Adding lifecycle
execution to the `md` CLI is a separate product decision.

## Core event contract

| Event | Meaning | Cardinality and error context |
|---|---|---|
| `initialize` | The active document's shell-free bootstrap is available, before body dependency discovery and preflight | Once per adopted document; no `err` |
| `composed` (`start`) | Composition and required preflight have succeeded; the prepared document is ready for the host's next operation | Once per eligible attempt/iteration; no `err` |
| `blocked` | The run cannot proceed to the host operation | Terminal branch carrying the selected error; excludes a normal success outcome |

`composed` and `start` are two spellings of one identity, not two emitted events.
`composed` is the canonical public name; existing `start` documents remain valid.
Declaring both on the same document is an error naming both source locations;
the engine neither merges them nor runs the stack twice. Alias normalization
must apply to parsing, deferral, validation, registration collision checks,
schemas, and editor help. Diagnostics retain the authored spelling.

The alias preserves the existing `start` boundary and its state visibility. It
does not move the stack earlier merely because an intermediate composition call
returned. In Claudine it remains the pre-provider boundary after launch checks.
`composed` does not mean the provider ran, the output passed its completion
verdict, or later handlers cannot mutate document state.

`blocked` can also follow `composed` on an applicable pre-operation refusal.
However, an unstarted operation is not sufficient to select blocked: Claudine's
incompatible resume after start uses failure and finalize before spawn. The host
reports the failure stage and entry reason as well as whether the operation
started; it cannot freely emit core events. The engine chooses the valid
transition under the registered profile.

A core-only run returns a typed result: ready/composed, blocked, failed, or a
control request. An evaluation failure need not be disguised as `blocked` to
invent a universal `failure` event. A host profile can attach its catch routing
to these results. Failures before document lifecycle ownership exists are
returned without trying to execute an unavailable handler.

## Caller registration

### Bounded placement and explicit dispatch

Provide a builder that begins with the core registry and accepts ordered caller
event groups through `before_core(...)` and `after_core(...)`. Freeze and validate
the registry before parsing a document. This bounded model avoids arbitrary
before/after dependency graphs and sorting cycles.

Placement defines an allowed execution region and deterministic descriptor
order. It does not automatically fire every registered event. The host explicitly
requests ordinary event emissions and reports operation outcomes; the engine
checks placement, payload, repetition, and current run state before evaluation.

- Before-core events run after passive bootstrap/registry validation and before
  `initialize`. They cannot depend on initialization or body composition, and
  they are shell-free. A failure stops normal progression and enters applicable
  host catch/cleanup routing.
- After-core events run after the core phase resolves, either successfully or
  through a blocked/error path. Outcome and cleanup roles may therefore run
  without `composed` having fired. Such paths retain their actual permissions.
- Events within a group have explicit declaration order. Do not derive it from
  a hash map. Authored action order remains unchanged.
- Registration is immutable for the run. Recursive emission from a handler is
  rejected. Repeated emissions require a declared repeat policy and an eligible
  attempt or iteration; callers cannot reset counters themselves.

### Event descriptors

Each descriptor contains a stable namespaced identity, description, placement,
payload schema, available globals and their types, `err` availability, supported
controls, required host capabilities, repeat policy, and transition role.
Authored paths and aliases are declared once and feed both composition deferral
and passive tooling. Registration rejects duplicate identities, path collisions,
reserved core names/aliases, and inconsistent role combinations.

Use typed handles returned by registration for runtime dispatch. A minimal API
should feel like the following; names and Rust signatures are illustrative:

```rust,ignore
let mut registry = LifecycleRegistry::core();
let published = registry.after_core(
    EventDescriptor::new("publisher.published")
        .payload(published_schema)
        .once_per_attempt(),
)?;
let registry = registry.build()?;
let mut run = registry.prepare(document, context, services)?;
// The host drives core preparation and performs publication.
run.emit(published, published_payload)?;
```

Default custom events are ordinary notifications with ordered stacks. They
cannot replace a terminal result or cause re-entry merely by choosing a name.
Reject unsupported controls during passive validation where knowable, and again
at runtime when they require facts such as an available session.

### Authoring surface

Keep the existing top-level core keys and Claudine's top-level event spellings.
For new custom events, use a dedicated namespaced container:

```yaml
composed:
  info: "Document is ready"
lifecycle:
  events:
    publisher.published:
      stack:
        - action: { info: "Published to {{event.url}}" }
```

`event` is a read-only payload namespace, validated before any handler action.
Payload fields cannot overwrite `doc`, `ctx`, `env`, `err`, or run state. A
document field named `event` remains accessible through `doc.event`.

Unknown names inside `lifecycle.events` fail execution before effects; unrelated
top-level frontmatter remains ordinary document data. Claudine's shipped schema
and profile identify its legacy top-level event keys. A lifecycle execution host
that recognizes a required profile but lacks it reports an unsupported-profile
diagnostic rather than silently discarding those handlers. Passive tooling may
load a descriptor without enabling execution capabilities.

## Claudine's continuation profile

Claudine registers one after-core profile with these roles:

| Event | Generic role | Claudine supplies |
|---|---|---|
| `success` | Successful terminal outcome | Provider outcome plus passing completion verdict |
| `failure` | Failed terminal outcome and catch handler | Selected typed error and recovery facts |
| `finalize` | Once-per-attempt/iteration cleanup | Host cleanup services and current error, if any |
| `loop` | Post-finalization iteration gate | Existing loop condition, limits, and mutation policy |

The profile associates core `blocked` with the same terminal slot as its success
and failure events. The engine owns slot accounting and transition validation;
Claudine owns the meaning of the operation outcome. These are fixed reusable
roles, not user-authored state-machine edges.

Normal Claudine progression is:

```text
initialize → preparation/preflight → composed (= start)
           → provider → inline closure when applicable → completion verdict
           → success | failure → finalize → loop gate
```

Early blocked progression is `initialize → blocked → finalize`, subject to the
existing initialization and catch-error rules. `success` and `failure` are
alternatives, not consecutive callbacks. A blocked run never needs a fabricated
`composed` event to reach cleanup.

Preserve these contracts:

1. One effective terminal slot per attempt/iteration. Explicit errors in a
   success or blocked stack may redesignate it to failure and execute the failure
   stack without undoing already-emitted communications.
2. Finalize eligibility, error precedence, and catch routing follow the current
   runtime. Finalize runs at most once per eligible attempt/iteration, including
   recovery paths; an error inside finalize never recursively finalizes.
   An admitted terminal-handler retry/resume skips that attempt's finalize;
   proxy skips source finalize. A recovery selected by finalize occurs after
   that finalize has already run. Do not introduce unconditional cleanup before
   every recovery as part of extraction.
3. Evaluation failures, dispatch failures, and explicit error controls remain
   distinct. In particular, a terminal-phase evaluation failure does not
   automatically replay the failure event. `no_error` suppresses only the
   dispatch failures it suppresses today.
   Normal finalize recovery and catch-only cleanup are explicit engine modes:
   cleanup following a fatal evaluation or control refusal cannot dispatch its
   own retry/resume/proxy. A later cleanup evaluation can supersede the error.
4. Loop concerns run after finalize, then the condition is checked, then eligible
   per-iteration mutations run. Another iteration re-enters at `composed` using
   the already-audited plan, without repeating initialization or preflight.
5. Retry/resume reread and audit without repeating initialization. Proxy adoption
   establishes a new active document and resets authorization before its own
   initialization. The host resolves targets and verifies session compatibility.
6. Recovery produces typed host requests. A host must acknowledge the resulting
   outcome before the engine advances. Budgets and provider handles remain in
   Claudine; registration cannot bypass their checks.
   Bind each request to its run and transition generation. Reject stale,
   duplicate, foreign, and wrong-kind replies without mutating state. Expose
   owned request data so the host need not borrow the run mutably across await.

Do not promise durable exactly-once effects across crashes or uncatchable process
termination. Preserve existing best-effort guard behavior without expanding it
into a persistence protocol.

## Execution, state, and services

One run object holds authoritative phase, active-document identity, attempt and
iteration identity, terminal/finalization state, and permission state. Hosts
drive it through checked operations rather than editing those fields.

Capture composition context once and pass it through preparation and event-time
evaluation. Preserve current lazy values, deferred spans, early-bound shell
commands, whole-value types, source provenance, and ordered mutation visibility.
Mapping `set` keeps its pre-write snapshot semantics. Sequence task/group state
remains isolated while using the same extracted stack machinery.

Initialization, before-core events, and bootstrap frontmatter remain shell-free,
including dead branches. Early catches and cleanup cannot open an alternate shell
route. Shell authorization requires both successful applicable preflight and the
existing `composed`/`start` permission transition. An after-core registration,
approval flag, cached approval, or `no_error` cannot independently authorize it.
Approved command text must remain byte-identical to executed text.

Inject neutral effect and communication services. Darkmatter must not import
`GlobalSettings`, provider summaries, messaging configuration, or concrete TTS
settings from Claudine. Missing capabilities produce typed errors, not silent
no-ops. Claudine adapters retain terminal rendering, messaging routes, audio
queue handoff, and process behavior. Terminal adapters use `TerminalRenderable`.

Keep a typed error envelope with code, category, disposition, origin, severity,
details, concise message, one-level cause, and source location. Host categories
remain typed on the host side and project into the shared envelope. Claudine's
effective-diagnostic selection must feed rendering, machine output, and `err`
consistently, including engine-originated failures routed through its adapter.

Use `FileReference` and the captured resolution context for authored file
references; use existing effect and remote policies. This migration grants no
additional filesystem or network authority.

Nested/transcluded documents do not create runs or fire lifecycle events.
Retries, loops, and proxy adoption use explicit run transitions; caches never
replay effects. Dry runs execute no lifecycle events or dynamic proxy handoffs.

## Passive tooling

The descriptor model and stack parser must work without runtime services.
Descriptors drive event paths, interpolation exclusions, action validation,
payload/global scope, schema/help output, and DMLS diagnostics. Avoid introducing
another hand-maintained list of event names or late-binding roots.

Expose a versioned passive representation of caller profiles through the planned
DMLS schema-extension mechanism. Generate it from the same definitions used by
runtime registration and check parity. Loading a profile must not launch Claudine,
load executable plugins, or dispatch effects. Unsupported descriptor versions or
missing profiles receive clear diagnostics.

Core event support ships with Darkmatter. Claudine's profile supplies its four
events, legacy top-level paths, and loop-specific authoring rules. If the separate
DMLS extension work is not ready, editor parity remains an explicit incomplete
deliverable; do not create a second extension transport to bypass that dependency.

## Migration and acceptance

### 1. Prove the boundary

Record current traces for normal execution, blocked preparation, catch failures,
terminal downgrade, finalization, retry, resume, proxy, loop, and sequence tasks.
Include event/action ordering, state snapshots, error selection, and permission
transitions. Inspect source and tests when graph analysis cannot resolve callers.

Build a small deterministic non-agent document host using core events and both
before-core and after-core registrations. Exercise a mutation, a typed failure,
payload validation, and refused shell execution with fake services. It must need
no Claudine types. DMLS is useful passive reuse but does not replace this proof.

**Gate:** the event/profile API is ergonomic, core permissions have one owner,
and Claudine's terminal and re-entry rules fit without arbitrary callbacks that
mutate engine state. Revisit the design if that requires moving its coordinator
or provider harness wholesale.

### 2. Extract shared semantics

Move AST, parser, source maps, validation, evaluator, and reusable transition
machinery with their tests. Decouple host errors and services. Temporary Claudine
re-exports/adapters are acceptable; two independently evolving parsers are not.
Preserve behavior before adding new custom-event behavior. Compare engines only
using fake effects so parity testing cannot duplicate real side effects.

### 3. Adopt the registry and Claudine profile

Replace fixed slots and seven-key assumptions with descriptors. Normalize
`start` to `composed`, preserving existing authored paths in diagnostics. Wire
direct composition, inline composition, sequences, tasks, and loops through the
same engine. Remove superseded implementations after parity is established.

### 4. Complete authoring and public documentation

Wire core and caller descriptors into schemas and DMLS. Update package READMEs,
lifecycle documentation, public API examples, and relevant skills alongside
implementation. Update dependency inventories only when dependencies change.
Document which profiles and capabilities a host supports.

### Required evidence

- Existing shipped Claudine documents retain action order, outcomes, diagnostics,
  recovery behavior, and audio handoff semantics.
- `start` and `composed` independently produce equivalent traces; using both is
  rejected before effects. Core-name and alias collisions are rejected.
- Custom registration covers duplicate/unknown events, invalid payloads, illegal
  placement, unavailable capabilities, invalid controls, repeat violations, and
  recursive emission with source-located errors.
- Early shell prohibitions survive all custom-event and catch paths, including
  dead branches and proxy resets; approved commands retain exact text.
- Terminal downgrade, evaluation failures, and errors in finalize do not duplicate
  terminal accounting or cleanup. Loop and recovery preserve their distinct
  re-entry rules.
- Passive shipped-artifact corpus checks and DMLS tests verify event-aware
  diagnostics, alias support, and caller-profile parity without runtime effects.
- Normal invocation tests cover the Claudine CLI, not just isolated parser tests.
  Audio tests stay silent; terminal/browser tests never take focus.

Use package-scoped `just test` and `just lint` for affected areas, with
`just test-l2` only where actual terminal integration needs proof. Validate
portable behavior on macOS, Linux, native Windows, and WSL2 under the existing
repository coverage policy; add no speculative CI cells for this proposal.

## Review decisions remaining

The ownership split, `start` alias, bounded before/after placement, and preservation
of Claudine behavior are the proposed contract. Before implementation approval,
the boundary prototype should settle concrete Rust service lifetimes and error
types, profile role validation, and the passive descriptor representation shared
with the forthcoming DMLS extension work. The custom `lifecycle.events` syntax
is proposed here and requires a shipped-document collision check.

## Evidence and review limits

This draft reviews the against, benefit, and impact statements for
`2026-09-22-lifecycle-events`, plus the current lifecycle contract, event model,
runtime routing, and DMLS schema-support documentation. GitNexus was bound to
this worktree (`rusty`), with indexed/current commit `a0eedb5` and matching covered
files at inspection. Its concept query returned no execution processes and some
malformed records; exact `LifecycleSignal` impact returned `UNKNOWN` with no
resolved callers. Those results do not establish low risk. Source inspection
confirms integration in preparation, catch routing, and terminal orchestration;
the supporting assessment identifies loops and sequences as additional callers.

Production migration is not implemented. The test-only control-flow prototype
and its bounded verification are documented separately in the spike report;
they do not establish full CLI orchestration parity. No existing supporting
statement is rewritten by this proposal.
