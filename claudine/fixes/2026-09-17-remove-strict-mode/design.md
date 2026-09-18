# Remove Strict Mode: Technical Design

Status: **technical design in progress; not ready for implementation planning**.
Last updated: 2026-09-18.

## Relationship to the specification

[spec.md](spec.md) is authoritative for behavior, scope, ownership, and acceptance
criteria. This document records technical decisions only after explicit human
agreement. Open decisions below must be resolved here before implementation
planning; they are not delegated to a future plan. No prototype has been
performed, and current investigations have not identified a question requiring
one.

The required result is one Darkmatter expression model: missing document
properties evaluate to `null`, while actual expression failures and unavailable
reserved globals remain errors. Schema-declared properties retain their static
types when absent. Bare document lookup never falls back to runtime context.
The specification also requires passive validation, preserved evaluation and
capture timing, non-relaxable shell restrictions, shared schema sources, and
editor recovery without stale dependent assistance.

These are existing requirements, not decisions reopened by this design.

## Ownership boundaries

| Owner | Responsibility fixed by the specification |
| --- | --- |
| `darkmatter` | Parsing, traversal, binding classification, namespace precedence, evaluation, passive validation, schema/result validation, and feature restriction enforcement. |
| `claudine` | Lifecycle policy, global names and availability declarations, value construction and capture timing, orchestration, mutation atomicity, shell approval policy, and contextual wrapping that preserves typed Darkmatter causes. |
| `claudine-cli` | Presentation of library errors and invocation integration. |
| `dmls` | Generic schema discovery and refresh integration, passive Darkmatter validation, and consistent diagnostic/hover/completion presentation. No dependency on Claudine or compiled-in lifecycle catalog. |

Authoritative lifecycle schemas belong in `claudine/schemas`; generated runtime
definitions and editor consumption must derive from that source. Generation
integration, artifact layout, and descriptor representation remain open.

## Current architecture and evidence quality

The supporting [lifecycle investigation](lifecycle-investigation.md) and
[schema/editor investigation](schema-investigation.md) record source locations,
current behavior, and explicitly unconfirmed recommendations. Their graph-first
navigation attempts found stale and malformed GitNexus records. The initial
refresh timed out while waiting for an existing analyzer. Subsequent refreshed
index results remained inconsistent: the repository identifier changed to
`rusty`, an expected lookup implementation set was returned, but malformed
symbol candidates remained. The [initial review](design-review-1.md) records
that follow-up evidence. Current graph callers, process counts, and impact risk
remain **unresolved**; neither historical specification counts nor partially
recovered graph results establish a clean current impact assessment.

Direct source inspection following those unsuccessful graph queries established:

- `EvaluationLookup` exposes `get` returning `Option<Value>`, which cannot
  distinguish an unavailable reserved global from ordinary missing data.
  `LayeredLookup` combines document state with injected globals and memoizes lazy
  values within one lookup instance.
- Claudine has four production subtree consumers and additional direct evaluator
  consumers. Lifecycle guards, messages, and typed values also apply bespoke
  checks. A subtree-only migration would miss direct evaluation.
- `LifecycleCurrent` currently captures context and environment at event time,
  exposes them through `current.ctx` and `current.env`, and delays serialization
  through a lazy closure. An independent `current_env` global is not currently
  injected. Published guidance describes a different intended projection;
  D3 below assigns that migration to the more-context feature.
- Lifecycle error transport loses original typed causes in diagnostic projection;
  proxy overlay evaluation has an additional earlier conversion to text.
- DMLS currently retains a last-good trigger registry after scan failure, and
  trigger scanning resolves payloads transactionally. Both conflict with the
  required failure isolation and no-stale-assistance behavior.
- Trigger parsing currently recognizes `trigger-schema`, while repository kind
  guidance uses `schema-trigger`. This needs an explicit compatibility/scope
  ruling before activation artifacts are designed.
- `claudine-gen` already depends on Darkmatter and forbids a dependency on the
  Claudine library or CLI. It is a candidate for generation, not yet a confirmed
  choice. Darkmatter's current `include_str!` embedding is not code generation.

The investigation documents contain the detailed source references. No current
impact result or completed migration inventory is claimed by this draft.

## D1 — Add a richer method to the existing lookup interface

**Human decision:** confirmed 2026-09-18, option **B**. Add a richer resolver
method to `EvaluationLookup` with an ordinary-lookup default. Do not replace the
existing method with a mandatory richer signature or require a separate adapter
at every evaluation boundary.

**Recommendation presented:** option B, using shared Darkmatter binding
classification for passive validation and runtime evaluation.

The existing `get` method remains suitable for lightweight data lookups. Its
ordinary default treats missing document data as valid missing data. Surfaces
that declare globals must preserve available-null, unavailable-global, and
missing-document distinctions through the richer evaluator contract. Expression
evaluation must use that contract; forwarding wrappers must not erase it.

| Material alternative | Benefit | Cost |
| --- | --- | --- |
| A: Replace `get` with a fallible classified result | One mandatory interface; compiler forces migration. | Broad trait change and mechanical work for simple lookups. |
| B: Add a richer method with a default **(confirmed)** | Reuses the existing boundary and keeps ordinary implementations small. | Two methods remain; evaluator routing and wrapper forwarding require an explicit audit. |
| C: Keep the trait unchanged and require a binding adapter at evaluation boundaries | Separates raw data access from binding policy. | More caller wiring and potential parallel wrapped/unwrapped evaluator paths. |

Simplicity strongly favored B: richer semantics are required where globals are
declared, without requiring each fixture or unrelated lookup to recreate
lifecycle concepts. This is not a compatibility mode for obsolete strictness
behavior; the specification still requires its direct removal and coordinated
consumer migration.

D1 alone does not settle the resolver result type, passive descriptor API,
provider representation, lazy cache scope, error enums, or prepared-evaluation
representation. D2 below settles the declaration/session separation and cache
lifetime; the remaining contracts still require agreement.
It authorizes no change to observation timing, short-circuit behavior, whole-value
typing, interpolation, escaping, or platform behavior.

## D2 — Separate binding declarations from runtime values and caches

**Human decision:** confirmed 2026-09-18, option **B**. Keep immutable binding
declarations separate from a runtime evaluation session. Give each existing
lookup operation a fresh lazy-value cache, preserving its current lifetime.
One `SubtreeCompose::compose` operation creates one lookup and shares that
session across recursive leaves, including multiple expression-bearing values.
This is not a new per-expression or per-key cache boundary.

**Recommendation presented:** option B, so preparation and DMLS can consume
the complete binding declarations without carrying or invoking runtime providers.

The agreed semantic contract is:

| Surface | States and responsibilities |
| --- | --- |
| Immutable declarations | Classify a global as definitely available, definitely unavailable with a structured reason, or execution-dependent. These declarations support shared passive and runtime classification. |
| Runtime entries | Supply an eager value, a lazy provider, or an unavailable state. A real `null` value is available data and must never be treated as unavailability. |
| Checked association | Associate declarations and runtime entries through a checked boundary. Invalid configuration produces structured errors; association and passive validation do not invoke lazy providers. |
| Runtime session | Own one existing lookup operation's memoized lazy values, including all recursive leaves of one subtree compose. Reusable declarations do not extend that cache across separately constructed lookups, actions, or events. |

Passive validation can reject definitely forbidden references in inactive
branches using declarations alone. Execution-dependent availability remains a
runtime check. The existing specification requirement to reject registrations
named exactly `doc`, `ctx`, or `env` before evaluation or provider invocation
applies to this checked boundary as well.

| Material alternative | Benefit | Cost |
| --- | --- | --- |
| A: Combine declarations, providers, and cache in one object | Fewer distinct objects and a direct runtime construction path. | Static consumers must avoid runtime-bearing fields; reuse can accidentally extend provider/cache lifetime. |
| B: Immutable declarations plus a runtime session **(confirmed)** | Keeps passive consumers provider-free and makes cache lifetime explicit. | Requires checked association between the declarations and runtime entries. |

Simplicity here means separating data with different lifetimes and consumers,
not introducing a second policy catalog. Darkmatter owns the shared contracts
and validation mechanics; Claudine supplies its lifecycle declarations and
runtime values. DMLS consumes the declarative form without depending on Claudine.
The declaration serialization/distribution format remains open.

This decision does not choose exact Rust type or method names, final error
variants, or the prepared-expression API. It also does not settle the
`current`/`current_env` projection or change when context and environment are
observed. D3 supplies that scope ruling. Lazy materialization and observation
timing remain distinct; the more-context feature's future per-key freshness
and memoization contracts are not decided by D2.

## D3 — Keep the coherent live-global migration in more-context

**Human decision:** confirmed 2026-09-18, option **A**. This fix retains the
existing event-captured `current.ctx.*` and `current.env.*` representation and
introduces no independent `current_env` global. The
[more-context specification](../../../../darkmatter/features/2026-09-09-more-context/spec.md)
owns the coherent future migration to Darkmatter built-ins, direct mirrors
across expression surfaces, and reference-time freshness.

**Recommendation presented:** option A, preserving this fix's scope and capture
timing without reversing the separately agreed more-context destination.

| Material alternative | Benefit | Cost |
| --- | --- | --- |
| A: Retain current snapshots here; more-context owns the migration **(confirmed)** | Keeps the binding fix focused and preserves current observation timing. | This fix continues to expose the existing nested lifecycle representation until the separate migration. |
| B: Bring the coherent live-global migration into this fix | Delivers the future public representation together with the binding changes. | Broadens scope to every expression surface and requires new provider, invocation-evidence, per-key freshness, and memoization rulings. |

The existing lazy `current` provider materializes an already-captured event
snapshot; it does not observe fresh environment or context on reference. This
fix must keep that distinction across its consumers and descriptors. No interim
direct-mirror alias or lifecycle-only `current_env` is introduced. Future
built-ins will supersede the lifecycle injection under the separate feature.

The human also authorized the narrow, dated clarification in this fix's
specification: remove `current_env` from its catalog examples, record retained
snapshot behavior and future ownership, and align the technical checkpoint with
that boundary. No other functional requirements or review metadata change.

## D4 — Share an immutable prepared representation

**Human decision:** confirmed 2026-09-18, option **B**. Darkmatter owns an
immutable prepared representation used by both passive validation and runtime
execution. Runtime values and observations remain outside that representation.

**Recommendation presented:** option B, preserving authored source form and
centralizing mechanics that Claudine currently repeats. Reuse Darkmatter's
existing parser, interpolation scanners, and `SpannedExpr` (the source-aware
expression tree); do not introduce a second evaluator.

| Material alternative | Benefit | Cost |
| --- | --- | --- |
| A: Share classification and validation but let callers parse independently | Smaller immediate API addition. | Repeated parsing and source-form handling remain; callers can still confuse authored templates with successful result text. |
| B: Share immutable prepared source plus a fresh runtime session **(confirmed)** | Preserves source identity and supplies one reference model to validation and execution. | Lifecycle operands must retain more than a bare AST; generated mixed-string spans still require runtime parsing. |

The prepared representation retains authored source form, source locations,
parsed expression trees, reference metadata, and context requirements. Its
source forms distinguish direct expressions and their parse mode, whole-value
expressions, mixed templates, literal data, and subtree containers. Context
requirements describe what may be needed; they do not capture values, observe
the host, determine runtime property existence, or select branches. Claudine
retains authority over observation timing and invocation/source context.

Passive validation uses the shared declarations and effective schema to classify
every authored executable reference, including inactive expression branches.
It preserves schema-provided static types for absent document properties and
defers execution-dependent availability. It does not invoke providers or treat
quoted strings inside expressions as additional expression programs.

Runtime execution binds the prepared input to current document values and a
fresh session at the existing lookup-operation boundary defined in D2. Reusing
prepared input must not widen the cache to an event or request. Runtime
short-circuit behavior remains authoritative for evaluated branches.

Whole-value expressions evaluate once and return their typed result, including
objects, arrays, and strings containing braces. Mixed strings retain only the
existing Darkmatter interpolation passes and stopping policy. New executable
content introduced by those existing passes receives runtime parsing and
validation; preparation does not claim to discover it by evaluating authored
expressions. Successful literal output is never subjected to an arbitrary
result rescan. Existing escape handling remains the authority for triple braces
and escaped openers.

Schema-policy identifiers retained by prepared input must refer to an immutable
schema generation or policy snapshot. A mutable editor-registry index alone is
insufficient: refresh must not silently reinterpret previously prepared input.
Exact propagation of restrictions through moved/generated values remains a
separate decision.

The [lifecycle investigation](lifecycle-investigation.md) records the reusable
scanner/parser primitives and whole-value versus mixed-string behavior. It also
identifies a variable fast path in the interpolation evaluator that currently
calls `get`/`get_string`; this must use D1's richer resolution contract. A
prepared artifact alone does not repair that bypass.

This decision settles the shared representation and behavior, not final Rust
names, error enums, serialized artifact syntax, or policy movement semantics.

## Remaining decisions and design completion work

The priority below reflects architectural dependencies, not implementation
sequencing.

| Priority | Open subject | Consequence to resolve before planning |
| --- | --- | --- |
| 1 | Binding interface details | Specify the richer result, passive descriptor API, structured reason/error shapes, and checked-association invariants within D1 and D2. |
| 2 | Prepared interface details | Make D4's inputs, outputs, source provenance, schema-generation association, and typed error boundaries concrete without adding runtime observations to preparation. |
| 3 | Lifecycle catalog details | Specify event/scope declarations for the retained globals under D3; do not introduce future built-ins or change event capture timing. |
| 4 | Restriction retention | Define restriction retention through deferred, generated, and moved values within D4's confirmed evaluation and literal-output behavior. |
| 5 | Descriptors, generation, and distribution | Select schema-associated descriptor format, runtime artifact representation, cycle-free generator integration, source identity, and drift verification. |
| 6 | Generic activation and source precedence | Resolve trigger-kind mismatch, workspace-fact syntax/anchors, explicit source precedence and deduplication, and origin versus applicability scope. |
| 7 | Refresh, isolation, and recovery | Define current-generation dependency/failure states and shared suppression of dependent diagnostics, hover, and completion; cover missing dependencies and newly created sources. |
| 8 | Typed error transport and ownership audit | Specify owned causes through lifecycle/recovery/proxy paths and complete the state/context/binding/evaluation/validation/error DRY audit, including reasons for retained duplication. |

Before completion, the design also needs a current lookup-by-lookup migration
inventory, concrete preparation/evaluation/editor-refresh flows, and a mapping
from the agreed contracts to the specification's acceptance tests. Cross-platform
path and watcher behavior must cover macOS, Linux, native Windows, and WSL2.
Responsiveness verification remains qualitative; no numeric threshold is added.

The [initial independent review](design-review-1.md) found D1 and D2 faithful to
the confirmed choices and requested the cache-boundary and graph-evidence
clarifications incorporated above. It was not approval for planning. Independent
review of the completed design remains pending. Open rulings,
unresolved graph evidence, and the incomplete migration inventory prevent a
claim that this design is ready for implementation planning.
