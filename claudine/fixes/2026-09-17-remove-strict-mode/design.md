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
navigation attempts found stale and malformed GitNexus records. The requested
refresh timed out while waiting for an existing analyzer. Current graph callers,
process counts, and impact risk are therefore **unresolved**; neither historical
specification counts nor unusable graph results establish present impact.

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
  injected. Published guidance describes a different intended projection; shape
  and timing must be resolved explicitly.
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

D1 **does not settle** the resolver result type, passive descriptor API,
provider representation, lazy cache scope, error enums, or prepared-evaluation
representation. Those consequential contracts remain subject to agreement.
It authorizes no change to observation timing, short-circuit behavior, whole-value
typing, interpolation, escaping, or platform behavior.

## Remaining decisions and design completion work

The priority below reflects architectural dependencies, not implementation
sequencing.

| Priority | Open subject | Consequence to resolve before planning |
| --- | --- | --- |
| 1 | Binding declarations versus runtime values/cache | Decide between a combined declaration/provider object and immutable descriptors with a runtime session. The latter is the current recommendation, **not confirmed**. Define eager/lazy/unavailable values, structured reasons, and memoization lifetime. |
| 2 | Shared classification and prepared evaluation | Specify passive all-reference validation, runtime-dependent availability, context requirements, and typed results/errors without provider invocation during preparation. |
| 3 | Lifecycle catalog and live environment projection | Reconcile current implementation and intended `current`/`current_env` shape; preserve or explicitly agree any observable change to capture timing. |
| 4 | Evaluation and restriction retention | Preserve ordinary whole-value, mixed-string, and escape behavior; define restriction retention through deferred, generated, and moved values without extra evaluation passes. |
| 5 | Descriptors, generation, and distribution | Select schema-associated descriptor format, runtime artifact representation, cycle-free generator integration, source identity, and drift verification. |
| 6 | Generic activation and source precedence | Resolve trigger-kind mismatch, workspace-fact syntax/anchors, explicit source precedence and deduplication, and origin versus applicability scope. |
| 7 | Refresh, isolation, and recovery | Define current-generation dependency/failure states and shared suppression of dependent diagnostics, hover, and completion; cover missing dependencies and newly created sources. |
| 8 | Typed error transport and ownership audit | Specify owned causes through lifecycle/recovery/proxy paths and complete the state/context/binding/evaluation/validation/error DRY audit, including reasons for retained duplication. |

Before completion, the design also needs a current lookup-by-lookup migration
inventory, concrete preparation/evaluation/editor-refresh flows, and a mapping
from the agreed contracts to the specification's acceptance tests. Cross-platform
path and watcher behavior must cover macOS, Linux, native Windows, and WSL2.
Responsiveness verification remains qualitative; no numeric threshold is added.

Independent review of the completed design remains pending. Open rulings,
unresolved graph evidence, and the incomplete migration inventory prevent a
claim that this design is ready for implementation planning.
