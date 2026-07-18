---
created: 2026-07-13
status: draft
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-07-16
review_iterations: 8
depends_on:
    - ../_completed/2026-05-12-lifecycle/spec.md
    - ../_completed/2026-06-26-positional-and-key-value/spec.md
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

The contract applies when a proxy is actually selected during live execution.
Existing dry-run behavior remains side-effect-free: dry run does not fire
lifecycle events and therefore does not traverse a dynamic proxy route.

A clean handoff is control transfer, not source completion or source failure.
Once `proxy` is selected, the coordinator does not synthesize any later source
lifecycle signal or apply an uncommitted source closure. Events that already
fired remain observable; a proxy selected by `success` or `failure` skips that
source attempt's ordinary `finalize`, while a proxy selected by `finalize`
obviously does not re-enter it. The target becomes the closure/output owner.

Equivalence is semantic, not an assertion that internal allocation, tracing,
or performance timings are byte-identical.

## Goals

- Make the active document, rather than the provider-attempt harness, the unit
  of orchestration.
- Give every proxy target a canonical fresh-document bootstrap and preparation
  so it can own its provider setup, lifecycle, and loop.
- Use one preparation/materialization service for direct execution and all
  later re-entry paths; no private reduced composer may claim equivalence.
- Preserve immutable caller intent separately from the invocation-run ledger,
  document-scoped preparation, and active-document execution state.
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
- Preserve the enclosing command's composition mode and sequence-step identity
  while transferring document identity and closure ownership to the target.

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
- Changing dry-run into lifecycle simulation or statically predicting which
  dynamic proxy branch would run.

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
file-resolution and error-propagation dependency specifications define the shared
resolver and diagnostic transport; this feature must use them rather than add
transition-local substitutes.

## Required Runtime Model

### R1 — One active-document coordinator

Each composition command run owns one coordinator above both the document-loop
engine and provider-attempt harness. Only this coordinator may commit a change
to active document identity. The provider-neutral transition types and pure
state decisions live in `claudine::composition`; the CLI driver owns process,
terminal, filesystem, and provider adapters. Library loop code therefore
returns the same transition rather than exposing a second optional proxy-target
channel.

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
    Proxy(EvaluatedProxyRequest),
    Complete,
    Abort(TypedTransitionError),
}
```

`TypedTransitionError` is illustrative: the implementation may use a generic
payload, a source-preserving envelope, or a coordinator outcome alongside the
provider-neutral enum. It must not force CLI-only resolution, launch, shell, or
provider-adapter failures into `CompositionError`, erase them into `String`, or
create a library-to-CLI dependency.

The provider-attempt harness may request `Proxy`, but it must not replace its
own source path and continue as though the target were another attempt. The
coordinator consumes the transition, ends the source document execution, and
boots the target through the same route used for a direct source.

Initialize routing, terminal recovery, target-initialize chaining, and library
loop routing must all converge on this transition. There must be no parallel
target-only return value whose consumption is optional.

The coordinator is nested inside, and cannot mutate, command-level ownership:

- `inline-compose` remains inline mode, but a committed proxy makes the target
  the only document eligible for the eventual inline closure; the router is
  not rewritten with target output;
- a sequence proxy remains inside the current sequence step until the target
  completes, fails, or proxies again; it neither advances nor restarts the
  sequence and it retains the step's scoped inputs and timing identity; and
- direct `compose` continues to route the final active document's output to
  stdout.

### R2 — State has four explicit ownership layers

The implementation must keep these concerns distinct. Exact Rust names may
differ.

#### Invocation state

This command-lifetime layer contains immutable invocation inputs and a
coordinator-owned run ledger. Keeping them in one lifetime layer does not make
the ledger immutable.

Immutable invocation inputs are:

- authored CLI reference and caller `key=value` / `--set` overrides;
- command mode (`compose`, `inline-compose`, or a particular sequence step),
  step-scoped overrides, and command-level output policy;
- explicit provider/model/interactivity flags and provider arguments;
- approval/yolo policy;
- launch CWD and launch-workspace discovery inputs;
- timeout, display, system-prompt, and MCP CLI intent;
- environment inputs and installation/configuration snapshot.

The mutable invocation-run ledger contains:

- the exact-command approval cache;
- the proxy chain and hop accounting;
- command/sequence timing anchors and command-wide performance accumulation;
  and
- transition provenance needed for final output and diagnostics.

Only the coordinator, or an adapter holding a narrow capability supplied by the
coordinator, may mutate the ledger. A proxy cannot mutate invocation inputs or
reset the ledger. Caller overrides remain authoritative at every document.

#### Handoff state

Handoff construction has two typed stages. Lifecycle evaluation produces a
provider-neutral request without consulting the filesystem:

```rust
pub struct EvaluatedProxyRequest {
    pub target: String,
    pub overlay: IndexMap<String, serde_json::Value>,
    pub provenance: ProxyProvenance,
}
```

The coordinator resolves that target through the file-resolution dependency,
performs hop/cycle validation, and only then creates a committable handoff:

```rust
pub struct ProxyHandoff {
    pub authored_target: String,
    pub resolved_target: PathBuf,
    pub overlay: IndexMap<String, serde_json::Value>,
    pub provenance: ProxyProvenance,
}
```

Exact Rust names and the related spec's resolved-path wrapper may differ. The
required distinction is evaluated request versus resolved handoff; no string
target is resolved again downstream. Provenance carries the source path,
event, action/property location, and proxy chain needed for diagnostics.

#### Prepared document

The complete canonical output for one active document:

- resolved source and source-specific repository/file-resolution context;
- exact `ComposeContext` and environment layers;
- authored and effective frontmatter, prompt, schema result, and warnings;
- composition mode, target-owned closure plan, and command output routing;
- selection hints and the resolved provider/model/interactivity;
- lifecycle configuration and lifecycle lookup context;
- loop definition and immutable loop-state plan/adapter;
- workspace, child CWD, MCP plan, system prompt, argv, child environment,
  structured-output mode, and dispatch configuration;
- shell discovery/approval plan and exact approved commands;
- immediate proxy overlay and immutable caller overrides;
- metadata needed to refresh the same document canonically.

An optimization may move or borrow fields, but direct and transitioned
documents must have the same semantic representation before provider launch.

#### Active-document execution state

Mutable only within the current active document and loop iteration:

- provider-attempt number and last outcome;
- document-loop iteration number and in-memory loop mutations;
- retry and resume budgets;
- optional live provider session identifier;
- a resume follow-up override;
- per-attempt timing and performance records.

This layer contains a replaceable provider-attempt slice inside a longer-lived
document-iteration slice. Retry and resume replace the provider-attempt slice,
but they retain and decrement the enclosing retry/resume budgets; otherwise a
retry could reset its own limit indefinitely. Retry starts a fresh provider
session, while resume alone retains a compatible live session. A proxy discards
the complete active-document execution state and creates fresh state for the
target. Advancing the target's document loop starts a fresh iteration budget.
The proxy chain, hop accounting, exact-command approval cache, and command-wide
timing remain in the invocation-run ledger and are not reset by a handoff.

The immediate overlay stored at document scope is the immutable, evaluated,
pre-schema handoff input. Schema defaults, coercion, and invalid-optional
drops affect prepared effective frontmatter, never the stored overlay; a later
refresh reapplies the same overlay and reruns the stage required by its entry
policy.

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

"One service" does not mean every re-entry repeats every lifecycle stage. The
service accepts an explicit entry reason and enforces this policy:

| Entry reason | Source/input basis | `initialize` | Schema and complete shell audit | Loop ownership |
|---|---|---:|---:|---|
| Direct document | Fresh read plus caller overrides | once | run | establish |
| Proxy target | Fresh read plus handoff overlay plus caller overrides | once | run | establish |
| Retry | Fresh read of the same document plus retained inputs | no | rerun | refresh current document definition |
| Resume | Fresh read of the same document plus retained inputs | no | rerun | refresh current document definition |
| Next loop iteration | Prepared source snapshot plus in-memory loop state | no | reuse | retain |

This table preserves the ratified loop contract: later loop iterations do not
reread an externally changed document, rerun schema validation, or prompt for
shell approval. They re-materialize prompt/data state through the canonical
service while reusing the validated structural plan and the exact stamped
commands established for the active document. Retry or resume is the explicit
fresh-read boundary. No entry reason may silently fall through to a different
policy.

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
3. parse the target's bootstrap lifecycle surface and run a narrow safety gate
   for actions that could execute during `initialize`;
4. run the target's `initialize` through the normal lifecycle evaluator;
5. consume `skip`, `error`, or another `Proxy` transition atomically;
6. once document identity stabilizes, reread the target so successful
   initialize-time mutations are visible, then perform full canonical preparation,
   validation, shell approval, loop recognition, and launch planning.

This staging must not create two composition implementations. Shared work is a
shared service with explicit stage boundaries. Any executable initialize action
must still satisfy the existing approval and security policy before dispatch;
"initialize before full pre-flight" never means "execute unapproved shell."

The narrow safety gate parses and approves every potentially selected
`initialize` shell command against the same early-binding snapshot, and routes
all other initialize actions through the existing effect/permission engine. It
does not run target schema validation or audit later-event commands. The full
audit after identity stabilizes covers every remaining lifecycle and template
shell surface and reuses exact-command approvals already granted by the narrow
gate.

Malformed frontmatter or a failure too early to construct the target's
lifecycle configuration cannot fire target catch events; it returns the normal
typed parse/bootstrap diagnostic. After the target lifecycle exists, later
bootstrap/preparation failures follow the normal `failure`/`finalize` routing
without emitting either event more than once.

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
mode, system prompt, argv, environment, child CWD, dispatch context, and the
target-owned closure plan. The enclosing command/sequence output policy remains
invocation state.

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
overlay, context-derivation inputs, and proxy provenance. It creates a fresh
provider attempt/session and refreshes mutable document material through the
canonical preparation service. The service derives a new coherent prepared
context; it does not reuse a stale context snapshot merely because document
identity is unchanged. Whether retry re-enters before or after a particular
lifecycle/pre-flight stage continues to derive from `provider_launched`, but
the refreshed body, lifecycle, context, and launch plan must come from one
coherent prepared document.

`Resume` retains the active document and compatible live provider session. It
refreshes mutable document/lifecycle material through the same canonical
service, then deliberately substitutes the evaluated follow-up message as the
provider input. It does not rerun `initialize`, change active document, or
silently select a different session contract.

Compatibility is not merely provider/model equality. The prepared document
computes a session-compatibility key containing every launch property that the
provider cannot renegotiate on resume, including at least provider, model,
profile/binary and resume protocol, workspace/child CWD, permission/tool mode,
structured-output mode, system-prompt delivery/content, and effective MCP
server set. Provider adapters may add provider-specific identity fields. If a
canonical refresh changes that key, resume fails with a typed diagnostic that
names the incompatible facets and recommends retry; it never mixes a live
session with a newly prepared launch plan.

Two of those facets are **immutable invocation inputs**: they belong to the key
for completeness, but no same-document resume can move them, so they are proven
where they are computed rather than by an end-to-end refusal.

- **Workspace/child CWD.** The child CWD is resolved once, from the process's
  own launch directory (its enclosing git root, else the directory itself),
  before any document is read. The complete set of document surfaces over launch
  identity is `agent:`, `model:`, and `interactive:`; none of them names a
  directory, and the resolver deliberately ignores a document's own repository
  in favour of the launch repository, precisely so that composing or proxying to
  a document in a sibling clone cannot move the provider into that clone. `--repo`
  moves the metadata repo root, not the child CWD, and is invocation intent in
  any case.
- **System-prompt content.** Resolution — discovering the file, composing its
  body — is provider-independent and runs once, at invocation; the composed text
  is captured and then delivered either inline on argv or through a
  Claudine-owned temp file written from that captured text. A document has no
  `system_prompt:` surface, and a lifecycle stack that rewrites the discovered
  `system-prompt.md` between attempts changes nothing the child receives, so a
  resume would be refused for a difference that does not exist.

System-prompt **delivery** is not immutable and is not projection-only: it is
provider-shaped, so a rebuild that lands on a different provider re-applies it in
that provider's form. Delivery therefore moves only when the provider facet
moves, and the provider facet already refuses end-to-end; it needs no separate
refusal of its own.

Every other facet in the key is document-reachable and must drive a real
refusal, not merely a projection.

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

Retry and resume fresh-read preparation also rerun discovery and approval;
unchanged exact commands hit the invocation cache, while newly introduced or
changed commands receive normal review. Loop iteration materialization reuses
the already stamped structural plan under R3 and cannot introduce new command
bytes.

Target body/frontmatter/lifecycle shell surfaces are resolved from the same
prepared document that will execute. A `with:` value that influences a command
must be present both when the command is approved and when it runs.

### R10 — Errors remain typed across transitions

Resolution, initialization, preparation, schema, shell, selection, retry,
resume, and proxy failures retain their concrete error and source/provenance
context. The active-document coordinator does not flatten a transition failure
into an `eyre!` string.

The error-propagation dependency owns registry and rendering mechanics. The
equivalence requirement here is that the same target failure has the same typed
identity and actionable rendering whether the target was direct, proxied from
initialize, or proxied from terminal recovery.

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
| `no_error` | no | boolean | Universal action flag; proxy evaluation/transition failures are not suppressible |

`with: {}` is valid and equivalent to omitting `with:`. A non-mapping value is
a typed frontmatter error. Mapping keys must be static YAML strings; dynamic or
non-string keys are rejected with a property-path diagnostic. In v1, an entire
mapping cannot be supplied as `with: "{{ payload }}"`; authors write the
mapping explicitly and may inject typed object or array values at individual
keys.

The action parser recognizes this field through the typed proxy descriptor
before applying the generic "direct parameter maps are unsupported" rule. That
exception is exact: it does not make nested maps valid for another proxy field
or any other lifecycle action.

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

`no_error` remains accepted because it is a universal key/value-action field,
but proxy has no side-effect-dispatch phase to suppress. Overlay/target
evaluation, file resolution, cycle/hop rejection, and target bootstrap are
expression or control-transition failures and remain fatal to the handoff.

The entire mapping is evaluated at the source handoff and then transported as
typed JSON-compatible data. It is not a template passed to the target. A
string installed under a target lifecycle/configuration key is therefore
literal target data after handoff; raw `{{ ... }}` syntax may not survive DM2
and become a second target-time evaluation. This rule prevents ambiguous
source-versus-target binding.

`with:` may intentionally set any top-level frontmatter key, including
selection, lifecycle, loop, schema, timeout, MCP, or other control-plane keys.
This is not an authority escalation: the source prompt could already select
those behaviors or execute equivalent lifecycle actions. It is nevertheless
executable configuration, not inert data. The target reparses and validates
all resulting structural configuration, and every shell, filesystem, network,
messaging, and provider effect remains subject to its normal target-side
policy. Documentation must recommend schema-declared data properties for
ordinary parameter passing and call out control-plane overlays as an advanced,
trusted-prompt capability.

### Atomic handoff

The target and complete `with:` mapping are evaluated before active-document
state changes. Target resolution, cycle/hop validation, and overlay evaluation
must all succeed before the coordinator begins target bootstrap.

If any step fails:

- no partial overlay is installed;
- the source remains the active document for diagnostic attribution and any
  catch routing still legal for the event that requested the proxy;
- the target is not initialized or composed;
- the concrete failure follows the existing lifecycle evaluation/setup-error
  route; and
- `no_error` does not suppress expression evaluation failures.

Failure routing remains event-aware even though proxy dispatch is shared. A
handoff failure must not synthesize a duplicate terminal/finalize event: before
finalize it follows the existing failure/finalize transition; after finalize
has fired it surfaces directly. A failure inside `finalize` never re-enters
`finalize`. The source stays active only until this routing completes; a failed
handoff never half-activates the target.

Once target bootstrap begins, any target-origin failure is attributed to the
target while retaining proxy provenance for diagnostics.

## Target Composition and Precedence

The resolved `with:` mapping is merged into every read of the target's authored
frontmatter: the bootstrap read before target `initialize` and the fresh read
after initialize-time mutations. Caller overrides are then reapplied. This
occurs before full Darkmatter composition and schema validation. The overlay
participates in:

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
properties use the target's canonical file-resolution context from the
file-resolution dependency; `with:` does not add a second path resolver.

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

Evaluating and transporting `with:` is data-only: that step does not invoke
Darkmatter's effect engine, create a temporary Markdown file, or trigger
frontmatter hashing. The installed values are not necessarily inert because
the target may interpret control-plane frontmatter as executable
configuration, as defined above.

Because the overlay participates in target preparation, executable target
configuration influenced by it passes the same schema, permission, and shell
approval controls as target-authored configuration. Approval and execution
consume one coherent prepared target.

Status output may report that a handoff includes an overlay, but must not print
overlay values. Tracing may record property names and counts; values can contain
secrets and follow existing redaction policy.

Any new terminal status or diagnostic renders through existing
`TerminalRenderable` components (`StatusBlock`, `Prose`, lists, or tables as
appropriate), preserving TTY/color/link behavior. No transition path writes
ad hoc formatted status with raw `println!`/`eprintln!`.

## Errors and Diagnostics

Add typed, source-aware diagnostics for:

- `with:` being anything other than a mapping;
- a dynamic/non-string `with:` key;
- interpolation failure at the most specific representable path inside `with:`
  (including nested object keys and array indices);
- a proxy-only `with:` parameter used on another action;
- target bootstrap/preparation failure with source and proxy provenance;
- resume incompatibility after canonical refresh; and
- any supported transition returned without an owning coordinator able to
  consume it.

Errors rooted in authored frontmatter use the existing `FrontmatterExcerpt`
rendering path and highlight the most specific locatable line. Diagnostics name
the lifecycle event, proxy action, target, and failing nested `with` path
without dumping unrelated overlay values.

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

> Reader note: the runtime refactor intentionally corrects behavior that
> depended on the router path. A proxied target may now execute additional loop
> iterations, select its authored provider/model, request approval for its own
> shell actions, or surface the typed error already produced by direct
> invocation. Those are compatibility fixes required by the equivalence
> contract, not preserved route quirks.

## Documentation

Update the lifecycle topic, composition topic, and Claudine skill to cover:

- the direct-versus-proxy equivalence contract;
- active-document ownership and the retry/resume re-entry contract;
- target-specific context/provider/MCP/workspace/loop behavior;
- key/value `proxy.with` syntax and typed interpolation;
- precedence and immediate-target overlay lifetime;
- transient `with:` versus persistent `set_frontmatter`/`merge_frontmatter`;
- source-time evaluation and the advanced control-plane-overlay trust model;
- retry/resume/loop entry policies and the resume compatibility key; and
- inline closure ownership, sequence-step containment, and dry-run behavior.

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
4. Lifecycle evaluation produces an evaluated proxy request, while the
   coordinator alone resolves it, checks hop/cycle state, and atomically commits
   a resolved handoff; no downstream layer resolves the target again.
5. A clean proxy emits no synthetic source terminal/finalize/loop event and
   applies no uncommitted source closure; the target owns the eventual closure
   and output.
6. `compose`, `inline-compose`, and sequence-step modes remain command state;
   an inline proxy can rewrite only its final target, and a sequence proxy
   cannot advance or restart its step.
7. A target reached through proxy acquires its own document loop before its
   first provider attempt and executes the same number of iterations as direct
   invocation under the same inputs.
8. Body, effective frontmatter, lifecycle, schema/file evaluation, and shell
   pre-flight use the same stored target `ComposeContext` and environment
   layers.
9. `ctx.area`, `ctx.agent`, `ctx.model`, `env.AGENT`, and `env.MODEL` match
   direct execution in body, frontmatter, and lifecycle surfaces.
10. Provider/model/interactivity, MCP, workspace/repository, profile/binary,
   structured mode, system prompt, argv, environment, child CWD, and dispatch
   configuration are recalculated for the active target subject to immutable
   CLI precedence.
11. Target `initialize` runs at its documented stage after the narrow
   initialize-action safety gate but before target schema validation/full
   pre-flight, and it may chain another atomic proxy.
12. Full preparation rereads the stabilized target after `initialize`, reapplies
   the immutable overlay/caller layers, and observes successful initialize-time
   file/frontmatter mutations without firing `initialize` twice.
13. Direct, proxy, retry, resume, and loop entry reasons obey the locked stage
   matrix; in particular, loop iterations reuse the prepared source/validated
   structural plan while retry and resume are fresh-read boundaries.
14. Retry canonically refreshes the current document and starts a fresh provider
   attempt without losing its overlay, context-derivation inputs, or proxy
   provenance; it derives one fresh coherent prepared context rather than
   reusing a stale snapshot.
15. Resume canonically refreshes mutable document/lifecycle state, retains only
    a live session whose complete compatibility key still matches, and
    deliberately uses its follow-up message; incompatibility identifies changed
    facets and recommends retry. Every document-reachable facet drives that
    refusal end-to-end; the key's two immutable invocation inputs
    (workspace/child CWD, system-prompt content) are proven at the layer that
    computes them, including that the one mutation a document could attempt —
    rewriting the discovered `system-prompt.md` — moves nothing.
16. Retry/resume budgets persist across provider attempts within the active
    document iteration; proxy or the next loop iteration resets them while
    preserving invocation-wide hop/cycle accounting.
17. Every fresh target receives complete shell discovery/approval, including
    lifecycle shell actions, and approved bytes equal executed bytes.
18. Key/value `proxy` accepts an optional mapping-valued `with:` field with
    static string keys; omitted and empty mappings preserve existing authoring
    behavior.
19. Positional proxy remains valid; positional proxy plus sibling `with:`
    remains an ambiguous-action error with an actionable rewrite.
20. `with:` recursively resolves once through source lifecycle DM2;
    whole-value interpolation preserves bool, number, null, array, and object
    values, and no raw span is deferred into target-time evaluation.
21. Malformed/unknown/illegal interpolation aborts the handoff atomically
    before active-document state changes.
22. Target-authored frontmatter < `proxy.with` < caller overrides, with shallow
    replacement and null removal at the proxy layer.
23. The stored document overlay remains the immutable pre-schema handoff input;
    defaults, coercion, and invalid-optional drops affect only prepared effective
    frontmatter and are deterministically reapplied on fresh preparation.
24. A target schema requirement can be satisfied by `with:`, and an invalid
    overlay produces the normal typed target schema error without invoking the
    provider.
25. Control-plane overlay values are reparsed and validated as target
    configuration and cannot bypass effect, permission, shell, filesystem,
    network, messaging, or provider policy.
26. The immediate overlay survives retry, resume, and loop refresh, but a
    downstream proxy replaces it unless forwarding is explicit.
27. Neither source nor target Markdown bytes or Darkmatter hashes change solely
    because `with:` is used.
28. The same target failure retains the same typed diagnostic identity across
    direct, initialize-proxy, and terminal-recovery-proxy routes.
29. A failed handoff follows existing event-aware catch/finalize routing without
    duplicate lifecycle emissions or half-activating the target.
30. User-facing status, tracing, and errors do not disclose overlay values, and
    new terminal output uses `TerminalRenderable` components.

## Test Strategy

### L1 — state and transition contracts

- Model invocation, handoff, prepared-document, and active-document execution
  state separately;
  assert which fields survive initial, retry, resume, proxy, and loop
  transitions.
- Assert retry/resume replace only their provider-attempt slice, retain and
  decrement the current iteration's budgets, and cannot reset their own limit;
  assert proxy and the next document-loop iteration receive fresh budgets.
- Assert the provider harness cannot mutate active document identity.
- Assert every supported proxy producer returns the shared typed handoff and
  every coordinator outcome consumes or explicitly rejects it.
- Assert evaluation cannot resolve/commit document identity and that a resolved
  handoff is never sent back through file resolution.
- Prove canonical preparation returns semantically equivalent prepared targets
  for direct and proxy entry.
- Lock the per-entry stage matrix, including one initialize emission, target
  reread after initialize mutation, full retry/resume validation, and loop
  structural-plan reuse.
- Prove context construction is independent of later process-CWD changes.
- Prove target-dependent provider/MCP/workspace/launch decisions refresh on
  proxy but immutable CLI inputs do not.
- Prove retry starts a fresh session, resume retains only a session with an
  identical compatibility key, and proxy clears active-document execution
  state.
- Prove retry/resume budgets persist and decrement across provider attempts,
  then reset for a proxy target or the next document-loop iteration while proxy
  hop/cycle state continues.
- Parse key/value proxy with omitted, empty, scalar-valued, and nested `with:`;
  reject non-mapping `with:` and proxy-only fields on other actions.
- Prove literal, mixed, whole-value, and nested interpolation semantics.
- Test shallow replacement, null removal, precedence, atomic failure, and
  immediate-target overlay replacement independently.
- Prove schema normalization never mutates the stored handoff overlay and
  control-plane values are source-resolved once, reparsed by the target, and
  subject to normal policy.
- Lock closure/output ownership for direct, inline, and sequence-step command
  modes, including the absence of synthetic source finalize after clean proxy.

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
- closure target, sequence-step identity, and stdout/stderr routing;
- shell approval/execution bytes;
- typed failure identity and rendered diagnostic; and
- overlay-value non-disclosure: a hand-off whose `with:` carries a
  secret-shaped value renders its status through `TerminalRenderable`
  components without the value reaching the pane, while the target lifecycle
  still consumes it (acceptance criterion 30, asserted on captured pane text —
  not a Level 1 `Debug`-redaction proxy).

Use a fake provider and self-contained temporary paths. The equivalence matrix
is a real-terminal (tmux) suite and therefore runs under the repository's
ratified Level 2 platform policy (`docs/testing-strategy.md` → "Platform
Coverage (CI)"): Linux (tmux) and macOS (opt-in). The whole
`level2_lifecycle_control.rs` file is `#![cfg(unix)]` by construction — its fake
providers are `#!/bin/sh` scripts and it drives a Unix PTY — so Windows does not
run the L2 matrix (the harness is absent there by policy); Windows proxy
coverage is the Level 1 suite, which is platform-neutral. Do not claim a
cross-platform L2 leg the ratified policy does not provide.

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
- resume incompatibility for each compatibility-key facet;
- a three-document chain with explicit and omitted forwarding;
- cross-repository proxy context and file resolution;
- target-authored provider/model and target-specific MCP tags;
- cycle, hop-limit, missing target, invalid overlay, schema failure, shell
  denial;
- initialize proxy returned from the library loop route, proving it cannot be
  dropped silently;
- initialize-time mutation followed by the stabilized target reread;
- control-plane overlays that add target lifecycle/shell configuration and
  prove target-side validation and approval still run;
- `inline-compose` proxy closure ownership and a proxy inside a sequence step;
  and
- dry-run proving no lifecycle side effect or dynamic proxy traversal occurs.

### Regression and drift guards

- Existing proxy parser, lifecycle placement, cycle/hop, retry, resume, loop,
  and caller-override tests remain green unless they encode the route drift
  intentionally corrected here.
- Add a narrow allowlist guard for production Darkmatter composition call sites
  so a second harness composer or target-only proxy carrier cannot appear
  unnoticed; the guard reports semantic owners rather than relying on a broad
  substring ban.
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
