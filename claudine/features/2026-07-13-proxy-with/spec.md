---
created: 2026-07-13
status: draft
reviewed: false
depends_on:
    - ../_completed/2026-05-12-lifecycle/spec.md
    - ../_completed/2026-06-26-positional-and-key-value/spec.md
related:
    - ../2026-07-13-file-resolution/spec.md
    - ../2026-07-13-error-propogation/spec.md
---

# Canonical Document Handoffs and Transient Proxy Frontmatter (`with:`)

## Introduction

Claudine's lifecycle `proxy` action hands execution to another prompt document,
but the current handoff is not equivalent to invoking that target directly.
Proxy changes the source path inside the lower provider-attempt harness after
the outer composition pipeline has already made source-dependent decisions.
The target is re-composed, but it does not become the owner of a fresh document
execution.

The most visible failure is a routing prompt that proxies to a looping target:

```sh
claudine compose prompts/_implement/implement-plan.md \
  spec=reviews/2026-07-12-perf/spec.md -y --codex

claudine compose prompts/implement.md \
  spec=reviews/2026-07-12-perf/spec.md -y --codex
```

The direct command recognizes `implement-plan.md` as a looping document and
executes all phases. The routed command recognizes `implement.md` as a
single-run document before its `initialize` proxy fires, then runs only one
provider attempt for the target. The same split can cause target references
such as `ctx.area`, `ctx.agent`, and `ctx.model` to resolve from a different
snapshot than they do in a direct invocation.

The missing `proxy.with` payload and this broader execution drift have the same
architectural cause: proxy is modeled as partial harness rematerialization
instead of a typed document transition. Adding more fields to the existing
state would preserve that split and invite another series of
route-specific fixes.

This feature therefore has two inseparable parts:

1. make direct execution, proxy, retry, and resume use one canonical document
   preparation path with explicit state ownership; and
2. add an optional typed `with:` mapping to a proxy handoff so a router can
   pass transient frontmatter to the immediate target.

`with:` is payload on the canonical handoff. It is not the architecture that
owns the handoff.

## Equivalence Contract

Given the same immutable CLI invocation inputs, activating a proxy target must
be observationally equivalent to invoking that target directly.

Once the target becomes active, both routes must agree on:

- target source identity, repository root, workspace, child CWD, and file
  resolution context;
- composed prompt, effective frontmatter, schema behavior, warnings, and
  lifecycle configuration;
- `ctx.*` and `env.*` values used by body, frontmatter, lifecycle, and shell
  pre-flight composition;
- provider and model selection, interactivity, binary/profile selection,
  system prompt, MCP configuration, argv, and child environment;
- shell discovery, approval, and the exact bytes later executed;
- target lifecycle ordering, including its own `initialize`;
- loop ownership, loop iteration count, and loop mutations;
- typed failures and their terminal rendering.

Intentional differences are limited to:

- lifecycle actions already emitted by the routing document before handoff;
- proxy provenance, cycle detection, and hop accounting; and
- the immediate target overlay supplied by `proxy.with`.

Equivalence is semantic, not an assertion that internal allocation, tracing,
or performance timings are byte-identical.

## Goals

- Make the active document, rather than the provider-attempt harness, the unit
  of orchestration.
- Give every proxy target a canonical fresh-document bootstrap and preparation
  so it can own its provider setup, lifecycle, and loop.
- Use one preparation/materialization service for direct execution and all
  later re-entry paths; no private reduced composer may claim equivalence.
- Preserve immutable caller intent separately from document-scoped and
  attempt-scoped state.
- Capture one explicit document context and use it consistently across body,
  frontmatter, lifecycle, and pre-flight evaluation.
- Define retry and resume re-entry precisely, including what is refreshed and
  what is retained.
- Let a proxying document pass route-specific, typed frontmatter to its
  immediate target through `with:`.
- Keep the handoff transient: no Markdown writes and no hash changes.
- Apply `with:` before target composition and schema validation while
  preserving caller overrides as the highest-precedence input layer.
- Preserve typed errors and source context through every transition route.
- Make direct-versus-proxy equivalence mechanically testable.

## Non-goals

- Changing positional `proxy: target.md` syntax.
- Turning `set_frontmatter` or `merge_frontmatter` into in-memory operations.
- Persisting proxy inputs into the target document.
- Making a source document's complete effective frontmatter implicitly inherit
  into every target. Handoff data remains explicit.
- Adding a new expression evaluator or interpolation grammar.
- Changing the author-facing meanings of `retry`, `resume`, or the document
  `loop:` construct beyond making their current re-entry contracts coherent.
- Changing proxy cycle detection or the hop limit.
- Implementing `defer` or serializing a handoff for deferred execution.
- Making provider session state transferable across a proxy. A proxy always
  starts a fresh target session.
- Adding general nested-map parameters to every lifecycle action. `with:` is a
  typed field owned specifically by `proxy`.

## Current State Transfer and Failure Modes

The current implementation distributes state across several types and two
orchestration levels:

| State | Current responsibility | Lifetime problem |
|---|---|---|
| `CompositionPrepContext` | launch CWD/workspace and source repository discovery | source-specific values are established before the target is known |
| `PreparedComposition` | composed prompt, effective frontmatter, lifecycle, warnings, and reduced rematerialization inputs | the initial source has a richer representation than later materializations |
| `CompositionExecutionRequest` | prepared source plus invocation configuration and caches | combines immutable caller intent with source-specific preparation |
| `HarnessPromptState` | source path, base prompt, overlay, prompt tail, resume session, and rematerialization inputs | proxy mutates document identity inside an attempt-level state object |
| `MaterializedHarnessPrompt` | prompt, frontmatter, lifecycle closure, and environment overrides | does not retain the exact composition context that produced those values |
| `HarnessLoopState` | provider/profile/CWD/argv/env, lifecycle guard, approvals, budgets, attempts, and proxy chain | target-independent and target-dependent state are frozen together |
| `LifecycleRunGuard` | lifecycle configuration and an original prepared context | proxy invalidates the original context without installing a complete target replacement |

### Direct execution

The direct first attempt consumes the full `PreparedComposition`. The outer
compose layer inspects that source before entering either the single-document
path or the document-loop path. A looping direct source therefore acquires loop
ownership before the provider harness starts.

### Retry

Retry consumes the initial prepared seed, then re-reads and re-composes the
same file through a reduced `RematerializeInputs` set. Provider launch state and
the lifecycle guard remain outside that reduced materialization. The body may
refresh while source context, lifecycle configuration, or target-dependent
launch decisions remain stale.

### Resume

Resume follows the same reduced re-composition path, then replaces the provider
prompt with a follow-up and carries a provider session identifier. This is a
hybrid: some document state is refreshed, some is retained, and the prompt no
longer comes from the refreshed document.

### Proxy

Proxy swaps the harness source path, clears the prompt/session override, and
re-composes the target. The lower harness can parse the target lifecycle and run
the target once, but it cannot revisit outer decisions already made for the
router, including:

- single execution versus document loop;
- provider/model and interactivity selection;
- MCP tags and runtime injection;
- repository/workspace and child CWD;
- profile, binary, structured mode, argv, environment, and system prompt;
- complete shell pre-flight and lifecycle shell resolution;
- the exact `ComposeContext` shared by body and lifecycle evaluation.

Changing process CWD before reduced re-composition can also change what a fresh
`ComposeContext::capture()` observes. In a monorepo this can turn a launch-area
`ctx.area` into an empty/root area. Injecting `AGENT` or `MODEL` into one body
composition path does not solve lifecycle evaluation when the lifecycle guard
uses a different context fallback.

### Parallel proxy exits

Initialize-time routing, provider recovery, and the library document-loop
engine do not converge on one consumed transition. In particular, a library
loop result can expose an initialize proxy target to its caller without the CLI
necessarily re-entering the target as a new document. A supported proxy path
must never return a target that the owning coordinator silently ignores.

### Shell and error drift

Target lifecycle shell commands can be discovered after a target-only preflight
that did not include target lifecycle configuration. Approval caching can then
freeze before a fresh target has had a chance to request approval. Parallel
proxy/retry routes also wrap some typed failures into generic messages. The
related file-resolution and error-propagation specifications define the shared
resolver and diagnostic transport; this feature must use them rather than add
transition-local substitutes.

## Required Runtime Model

### R1 — One active-document coordinator

The composition command owns one coordinator above both the document-loop
engine and provider-attempt harness. Only this coordinator may change active
document identity.

All lifecycle control paths return a shared typed transition to it. Names are
illustrative, but the semantic surface is required:

```rust
pub enum DocumentTransition {
    Continue,
    Retry,
    Resume {
        session: SessionId,
        message: String,
    },
    Proxy(ProxyHandoff),
    Complete,
    Abort(CompositionError),
}
```

The provider-attempt harness may request `Proxy`, but it must not replace its
own source path and continue as though the target were another attempt. The
coordinator consumes the transition, ends the source document execution, and
boots the target through the same route used for a direct source.

Initialize routing, terminal recovery, target-initialize chaining, and library
loop routing must all converge on this transition. There must be no parallel
target-only return value whose consumption is optional.

### R2 — State has four explicit ownership layers

The implementation must keep these concerns distinct. Exact Rust names may
differ.

#### Invocation state

Immutable for the whole command:

- authored CLI reference and caller `key=value` / `--set` overrides;
- explicit provider/model/interactivity flags and provider arguments;
- approval/yolo policy and the exact-command approval cache;
- launch CWD and launch-workspace discovery inputs;
- timeout, display, system-prompt, and MCP CLI intent;
- environment inputs and installation/configuration snapshot.

Proxy cannot mutate invocation state. Caller overrides remain authoritative at
every document.

#### Handoff state

Created only by a proxy transition:

```rust
pub struct ProxyHandoff {
    pub target: String,
    pub with: IndexMap<String, serde_json::Value>,
    pub provenance: ProxyProvenance,
}
```

It contains the unevaluated target only until lifecycle evaluation resolves it;
the coordinator receives an atomic resolved handoff. Provenance carries the
source path/event/action location needed for cycle checks and diagnostics.

#### Prepared document

The complete canonical output for one active document:

- resolved source and source-specific repository/file-resolution context;
- exact `ComposeContext` and environment layers;
- authored and effective frontmatter, prompt, schema result, and warnings;
- selection hints and the resolved provider/model/interactivity;
- lifecycle configuration and lifecycle lookup context;
- loop definition and loop state adapter;
- workspace, child CWD, MCP plan, system prompt, argv, child environment,
  structured-output mode, and dispatch configuration;
- shell discovery/approval plan and exact approved commands;
- immediate proxy overlay and immutable caller overrides;
- metadata needed to refresh the same document canonically.

An optimization may move or borrow fields, but direct and transitioned
documents must have the same semantic representation before provider launch.

#### Attempt state

Mutable only within the current active document iteration:

- provider-attempt number and last outcome;
- retry and resume budgets;
- optional live provider session identifier;
- a resume follow-up override;
- per-attempt timing and performance records.

Proxy discards attempt state and creates fresh target attempt state. Retry
starts a new session attempt. Resume alone retains the compatible live session.
The global proxy hop chain remains invocation-run state and is not reset by a
handoff.

### R3 — One canonical preparation service

Direct execution, proxy targets, retries, resumes, and document-loop refreshes
must call the same canonical preparation service. The service may expose
explicit stages, but it must not have a rich direct path and a reduced harness
path with overlapping responsibilities.

Canonical preparation owns, or delegates once to shared authorities for:

1. file-reference resolution and source repository discovery;
2. input-layer assembly, including caller overrides and `proxy.with`;
3. context/environment construction;
4. Darkmatter composition and report collection;
5. schema behavior and file-valued property handling;
6. warnings and interpolation/leak validation;
7. provider/model/interactivity selection;
8. lifecycle parsing and lifecycle shell resolution;
9. workspace/MCP/system-prompt/argv/environment launch planning;
10. loop recognition; and
11. typed source-aware errors.

The harness must not manually call Darkmatter `compose_with` as a substitute
for canonical preparation. `RematerializeInputs` must either become an internal
input to the canonical service with an honest limited name, or be replaced by
the invocation/document state above. Adding more source-specific fields to it
is not the strategic solution.

### R4 — Bootstrap initialize before full preparation

The documented lifecycle order requires a fresh target to reach its own
`initialize` after source/frontmatter resolution but before schema validation
and full shell pre-flight. A proxy may itself originate from `initialize`, so
the coordinator must stabilize an initialize proxy chain before committing to
provider launch or document-loop execution.

A staged canonical boot is expected:

1. resolve/read the candidate document and apply its input layers;
2. derive the target-specific repository, selection hints, provider/model
   environment, and exact early-binding context needed by `initialize`;
3. parse and run the target's `initialize` through the normal lifecycle
   evaluator;
4. consume `skip`, `error`, or another `Proxy` transition atomically;
5. once document identity stabilizes, perform full canonical preparation,
   validation, shell approval, loop recognition, and launch planning.

This staging must not create two composition implementations. Shared work is a
shared service with explicit stage boundaries. Any executable initialize action
must still satisfy the existing approval and security policy before dispatch;
"initialize before full pre-flight" never means "execute unapproved shell."

### R5 — Context is prepared data, not ambient process state

Each prepared document stores the exact early-binding `ComposeContext` used to
compose it. Body interpolation, effective frontmatter, lifecycle DM2 lookup,
schema/file evaluation, and shell pre-flight all consume that same snapshot and
the same explicit environment override layer.

The snapshot is derived from immutable launch inputs plus target-specific
source/repository/workspace and resolved provider/model identity. It must not be
recaptured from `std::env::current_dir()` after the wrapper changes child CWD.

This contract covers at least:

- `ctx.area` and other repository/launch-area values;
- `ctx.agent` and `ctx.model`;
- the equivalent `env.AGENT` and `env.MODEL` values;
- lifecycle fields/actions as well as the prompt body.

The late-binding `current.ctx.*` surface remains intentionally live and may
capture current state at event time. It must not be used as a fallback for a
missing prepared `ctx.*` snapshot.

### R6 — Target-dependent launch state is rebuilt per document

When proxy changes the active document, the coordinator recalculates every
document-dependent launch decision from the target plus immutable invocation
state. This includes provider/model selection, interactivity, MCP tags and
runtime injection, workspace/repository behavior, profile/binary, structured
mode, system prompt, argv, environment, child CWD, and dispatch context.

Normal precedence still applies: explicit CLI intent remains authoritative,
while target frontmatter can affect values that were not fixed explicitly.
Proxying to a prompt pinned to another provider must behave like directly
invoking that prompt under the same CLI arguments.

### R7 — Loop ownership follows active document identity

Loop recognition happens after initialize routing stabilizes and before the
first provider attempt for the active target. A target reached through proxy
therefore acquires the same document-loop coordinator it would receive when
invoked directly.

Later iterations retain the existing loop lifecycle contract: initialize runs
once for that active document, and the loop gate/mutations determine whether to
continue. Any per-iteration refresh uses canonical document preparation inputs
and the stored document context; it cannot fall back to the reduced harness
composer.

A proxy emitted by the loop lifecycle ends the source document and returns a
handoff to the active-document coordinator. It does not make the target an
extra iteration of the source loop.

### R8 — Retry and resume have explicit re-entry semantics

`Retry` retains active document identity, caller overrides, immediate proxy
overlay, prepared context, and proxy provenance. It creates a fresh provider
attempt/session and refreshes mutable document material through the canonical
preparation service. Whether it re-enters before or after a particular
lifecycle/pre-flight stage continues to derive from `provider_launched`, but
the refreshed body, lifecycle, and launch plan must come from one coherent
prepared document.

`Resume` retains the active document and compatible live provider session. It
refreshes mutable document/lifecycle material through the same canonical
service, then deliberately substitutes the evaluated follow-up message as the
provider input. It does not rerun `initialize`, change active document, or
silently select a different provider. If canonical refresh would make the live
session incompatible with the resolved provider/model, resume fails with a
typed error instead of mixing configurations.

Retry and resume budgets are scoped to the active document iteration. A proxy
target receives fresh document-attempt budgets, while the invocation-wide
proxy hop limit and cycle chain continue. Retry, resume, proxy, and loop counts
must not share an unlabeled global counter.

### R9 — Shell approval follows the prepared target

Every fresh proxy target receives the same shell discovery and approval
opportunity as a direct invocation. The command approval cache may be shared
across the invocation, but only an exact already-approved command may bypass a
new prompt. Freezing the cache for the source must not prevent a target from
requesting approval for newly discovered commands.

Target body/frontmatter/lifecycle shell surfaces are resolved from the same
prepared document that will execute. A `with:` value that influences a command
must be present both when the command is approved and when it runs.

### R10 — Errors remain typed across transitions

Resolution, initialization, preparation, schema, shell, selection, retry,
resume, and proxy failures retain their concrete error and source/provenance
context. The active-document coordinator does not flatten a transition failure
into an `eyre!` string.

The related error-propagation specification owns registry and rendering
mechanics. The equivalence requirement here is that the same target failure has
the same typed identity and actionable rendering whether the target was direct,
proxied from initialize, or proxied from terminal recovery.

## `proxy.with` Authoring Contract

### Syntax

`with:` is accepted only on key/value `proxy` actions:

```yaml
- action: proxy
  target: "{{ next_prompt }}"
  with:
      spec: "{{ spec }}"
      iteration: "{{ iteration }}"
      dry_run: "{{ false }}"
```

| Field | Required | Type | Meaning |
|---|---:|---|---|
| `action` | yes | literal `proxy` | Selects key/value lifecycle action form |
| `target` | yes | lifecycle string | Existing proxy target reference |
| `with` | no | mapping | Transient top-level frontmatter overlay for the immediate target |
| `no_error` | no | boolean | Existing dispatch-error behavior; evaluation errors remain unsuppressed |

`with: {}` is valid and equivalent to omitting `with:`. A non-mapping value is
a typed frontmatter error. In v1, an entire mapping cannot be supplied as
`with: "{{ payload }}"`; authors write the mapping explicitly and may inject
typed object or array values at individual keys.

Positional form remains intentionally compact and unchanged:

```yaml
- proxy: prompts/next.md
```

An author who needs `with:` opts into key/value form. A multi-key object without
`action: proxy` remains ambiguous and continues to fail with the existing
positional-versus-key/value guidance.

### Value semantics

The lifecycle action rule remains authoritative:

> Values are literal by default. Use `{{ ... }}` to inject a variable or
> expression.

`with:` keys are static YAML strings and are never interpolated. Values resolve
recursively at event time:

- a mixed string such as `"phase-{{ phase }}"` resolves to a string;
- a string consisting of exactly one interpolation span preserves the resolved
  type (`bool`, number, string, array, object, or null);
- YAML numbers, booleans, arrays, objects, and nulls retain their authored
  types;
- strings inside nested arrays and objects follow the same interpolation rule;
- arrays and objects are data values, not positional action arguments.

```yaml
- action: proxy
  target: prompts/next.md
  with:
      attempt: "{{ iteration }}"
      ready: "{{ true }}"
      label: "phase-{{ iteration }}"
      files: "{{ changed_files }}"
      metadata:
          source: router
          area: "{{ ctx.area }}"
```

Evaluation uses the same event-time Darkmatter subtree composition used by the
rest of the lifecycle surface. It must not introduce a bespoke Claudine
interpolator. The overlay resolves against the source document's live
frontmatter, its prepared `ctx.*`/`env.*` layers, and the globals valid for the
event (`err`, `timing`, and `current`, where applicable).

A preceding action in the same stack that changes the source document's live
frontmatter is visible to `with:`. Unknown roots, malformed expressions,
unknown functions, or out-of-scope late-binding globals fail closed as
lifecycle evaluation errors.

### Atomic handoff

The target and complete `with:` mapping are evaluated before active-document
state changes. Target resolution, cycle/hop validation, and overlay evaluation
must all succeed before the coordinator begins target bootstrap.

If any step fails:

- no partial overlay is installed;
- the source remains the active document for failure/finalize handling;
- the target is not initialized or composed;
- the concrete failure follows the existing lifecycle evaluation/setup-error
  route; and
- `no_error` does not suppress expression evaluation failures.

Once target bootstrap begins, any target-origin failure is attributed to the
target while retaining proxy provenance for diagnostics.

## Target Composition and Precedence

The resolved `with:` mapping is merged into the target's authored frontmatter
before target initialize composition, full Darkmatter composition, and schema
validation. It participates in:

- target frontmatter interpolation and computed properties;
- target selection hints and initialize conditions;
- SimplifiedSchema validation, coercion, defaults, and eager file handling;
- target lifecycle parsing and shell discovery;
- loop configuration; and
- the prompt body delivered to the provider.

Precedence, from lowest to highest, is:

1. target-authored frontmatter;
2. the immediate proxy's `with:` mapping;
3. caller-supplied compose overrides (`key=value` / `--set`).

The original caller remains authoritative. A router cannot silently replace an
explicit caller value with `with:`. Target computed properties and schema
normalization run after these input layers are assembled.

The overlay is shallow at the top level:

- a scalar or array replaces the target-authored value;
- an object replaces the target-authored object at that key rather than deep
  merging it; and
- null removes that target-authored top-level property before composition.

Caller overrides may subsequently restore or replace any key. File-valued
properties use the target's canonical file-resolution context from the related
file-resolution specification; `with:` does not add a second path resolver.

## Lifetime and Proxy Chains

The overlay is scoped to the immediate target document:

- it survives canonical refresh for retry, resume, and loop iterations of that
  target;
- it is available to every lifecycle event and body composition for that
  target;
- it is never written to disk; and
- it is discarded when that target proxies to another document.

A downstream proxy receives only its own `with:` mapping plus immutable caller
overrides. It does not implicitly inherit the previous hop's overlay. Forwarding
is explicit:

```yaml
- action: proxy
  target: prompts/final.md
  with:
      spec: "{{ spec }}"
```

Omitting `with:` on a downstream proxy therefore installs an empty overlay for
the new target. Cycle detection and `MAX_PROXY_HOPS` remain based on resolved
document paths; an overlay does not create a distinct document identity.

## Security and Side Effects

`with:` is data-only. It does not invoke Darkmatter's effect engine, create a
temporary Markdown file, or trigger frontmatter hashing.

Because the overlay participates in target preparation, executable target
configuration influenced by it passes the same schema, permission, and shell
approval controls as target-authored configuration. Approval and execution
consume one coherent prepared target.

Status output may report that a handoff includes an overlay, but must not print
overlay values. Tracing may record property names and counts; values can contain
secrets and follow existing redaction policy.

## Errors and Diagnostics

Add typed, source-aware diagnostics for:

- `with:` being anything other than a mapping;
- interpolation failure at a specific `with.<key>` path;
- a proxy-only `with:` parameter used on another action;
- target bootstrap/preparation failure with source and proxy provenance;
- resume incompatibility after canonical refresh; and
- any supported transition returned without an owning coordinator able to
  consume it.

Errors rooted in authored frontmatter use the existing `FrontmatterExcerpt`
rendering path and highlight the most specific locatable line. Diagnostics name
the lifecycle event, proxy action, target, and failing `with` key without
dumping unrelated overlay values.

The existing ambiguous-action diagnostic remains correct for:

```yaml
- proxy: prompts/next.md
  with:
      spec: "{{ spec }}"
```

Its actionable rewrite points to key/value form.

## Backward Compatibility

The authoring surface is additive:

- positional `proxy: target.md` is unchanged;
- key/value `{ action: proxy, target: target.md }` is unchanged;
- caller overrides continue to survive every handoff;
- proxy cycle protection and hop limits are unchanged; and
- action parameters other than `proxy.with` continue to reject direct mapping
  values.

The runtime refactor intentionally corrects behavior that depended on the
router path. A proxied target may now execute additional loop iterations,
select its authored provider/model, request approval for its own shell actions,
or surface the typed error already produced by direct invocation. Those are
compatibility fixes required by the equivalence contract, not preserved route
quirks.

## Documentation

Update the lifecycle topic, composition topic, and Claudine skill to cover:

- the direct-versus-proxy equivalence contract;
- active-document ownership and the retry/resume re-entry contract;
- target-specific context/provider/MCP/workspace/loop behavior;
- key/value `proxy.with` syntax and typed interpolation;
- precedence and immediate-target overlay lifetime; and
- transient `with:` versus persistent `set_frontmatter`/`merge_frontmatter`.

Correct stale documentation that describes retry/resume/proxy behavior in
terms of a reduced harness path or implies that recovery is limited to failure
when the universal lifecycle contract supports other runtime events.

## Acceptance Criteria

1. A proxied target is bootstrapped and fully prepared by the same canonical
   service as a directly invoked target.
2. Only the active-document coordinator changes document identity; the
   provider harness returns `Proxy` instead of swapping its own source path.
3. Initialize, terminal recovery, target-initialize chaining, and library loop
   proxy routes return one typed handoff that is always consumed or rejected
   explicitly.
4. A target reached through proxy acquires its own document loop before its
   first provider attempt and executes the same number of iterations as direct
   invocation under the same inputs.
5. Body, effective frontmatter, lifecycle, schema/file evaluation, and shell
   pre-flight use the same stored target `ComposeContext` and environment
   layers.
6. `ctx.area`, `ctx.agent`, `ctx.model`, `env.AGENT`, and `env.MODEL` match
   direct execution in body, frontmatter, and lifecycle surfaces.
7. Provider/model/interactivity, MCP, workspace/repository, profile/binary,
   structured mode, system prompt, argv, environment, child CWD, and dispatch
   configuration are recalculated for the active target subject to immutable
   CLI precedence.
8. Target `initialize` runs at its documented stage before target schema
   validation/full pre-flight and may chain another atomic proxy.
9. Retry canonically refreshes the current document and starts a fresh provider
   attempt without losing its overlay or prepared context.
10. Resume canonically refreshes mutable document/lifecycle state, retains only
    a compatible live session, and deliberately uses its follow-up message.
11. Retry/resume budgets are scoped to the active document iteration; proxy
    resets them while preserving invocation-wide hop/cycle accounting.
12. Every fresh target receives complete shell discovery/approval, including
    lifecycle shell actions, and approved bytes equal executed bytes.
13. Key/value `proxy` accepts an optional mapping-valued `with:` field; omitted
    and empty mappings preserve existing authoring behavior.
14. Positional proxy remains valid; positional proxy plus sibling `with:`
    remains an ambiguous-action error with an actionable rewrite.
15. `with:` recursively resolves through lifecycle DM2; whole-value
    interpolation preserves bool, number, null, array, and object values.
16. Malformed/unknown/illegal interpolation aborts the handoff atomically
    before active-document state changes.
17. Target-authored frontmatter < `proxy.with` < caller overrides, with shallow
    replacement and null removal at the proxy layer.
18. A target schema requirement can be satisfied by `with:`, and an invalid
    overlay produces the normal typed target schema error without invoking the
    provider.
19. The immediate overlay survives retry, resume, and loop refresh, but a
    downstream proxy replaces it unless forwarding is explicit.
20. Neither source nor target Markdown bytes or Darkmatter hashes change solely
    because `with:` is used.
21. The same target failure retains the same typed diagnostic identity across
    direct, initialize-proxy, and terminal-recovery-proxy routes.
22. User-facing status, tracing, and errors do not disclose overlay values.

## Test Strategy

### L1 — state and transition contracts

- Model invocation, handoff, prepared-document, and attempt state separately;
  assert which fields survive initial, retry, resume, proxy, and loop
  transitions.
- Assert the provider harness cannot mutate active document identity.
- Assert every supported proxy producer returns the shared typed handoff and
  every coordinator outcome consumes or explicitly rejects it.
- Prove canonical preparation returns semantically equivalent prepared targets
  for direct and proxy entry.
- Prove context construction is independent of later process-CWD changes.
- Prove target-dependent provider/MCP/workspace/launch decisions refresh on
  proxy but immutable CLI inputs do not.
- Prove retry starts a fresh session, resume retains only a compatible session,
  and proxy clears session/attempt state.
- Prove retry/resume budgets reset at the documented document boundary while
  proxy hop/cycle state continues.
- Parse key/value proxy with omitted, empty, scalar-valued, and nested `with:`;
  reject non-mapping `with:` and proxy-only fields on other actions.
- Prove literal, mixed, whole-value, and nested interpolation semantics.
- Test shallow replacement, null removal, precedence, atomic failure, and
  immediate-target overlay replacement independently.

### L2 — direct/proxy equivalence matrix

For each fixture, invoke the target directly and through an initialize router,
then compare:

- prompt and effective non-lifecycle frontmatter;
- `ctx.area`/`ctx.agent`/`ctx.model` and corresponding environment values in
  body, frontmatter, and lifecycle events;
- provider/model/interactivity, MCP, workspace/CWD, system prompt, argv, and
  child environment;
- lifecycle event order and target initialize count;
- loop iteration count and mutations;
- shell approval/execution bytes; and
- typed failure identity and rendered diagnostic.

Use a fake provider and platform-neutral temporary paths so the matrix runs on
macOS, Windows, and Linux.

Include the shipped behavior that motivated the feature: route
`prompts/implement.md` to `prompts/_implement/implement-plan.md` with a
multi-phase plan and assert that all phases execute exactly as direct
invocation does.

Additional L2 cases cover:

- caller override precedence over `with:`;
- target schema/computed property/initialize/body observation of typed overlay
  values;
- failure/finalize proxy using `err.*` inside `with:`;
- retry, resume, and loop refresh retaining the immediate overlay;
- a three-document chain with explicit and omitted forwarding;
- cross-repository proxy context and file resolution;
- target-authored provider/model and target-specific MCP tags;
- cycle, hop-limit, missing target, invalid overlay, schema failure, shell
  denial, and resume incompatibility; and
- initialize proxy returned from the library loop route, proving it cannot be
  dropped silently.

### Regression and drift guards

- Existing proxy parser, lifecycle placement, cycle/hop, retry, resume, loop,
  and caller-override tests remain green unless they encode the route drift
  intentionally corrected here.
- Add a structural guard against a second production Darkmatter composition
  path in the harness or a new target-only proxy carrier.
- Add passive corpus tests proving every production proxy route carries the
  complete handoff and every canonical preparation caller supplies explicit
  context.
- Run `just test`, `just test-l2`, and `just lint` in the Claudine package area.

## Out of Scope Follow-ups

- A compact positional payload syntax such as `proxy: [target, payload]`.
- Passing a complete mapping via `with: "{{ payload }}"`.
- Deep-merge controls or per-key merge strategies.
- Persisting or serializing proxy overlays for deferred execution.
- A general call/return value model between prompt documents.
