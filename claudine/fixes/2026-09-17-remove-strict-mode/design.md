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
integration and artifact layout are confirmed in D9. D6 fixes the descriptor
envelope; its remaining payload details are still open.

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
  guidance uses `schema-trigger`. D10 below resolves the canonical spelling and
  migration behavior.
- `claudine-gen` already depends on Darkmatter and forbids a dependency on the
  Claudine library or CLI. D9 selects it for generation. Darkmatter's current
  `include_str!` embedding is not code generation.

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
Propagation of restrictions through moved/generated values is settled by D5
below; exact API names remain open.

The [lifecycle investigation](lifecycle-investigation.md) records the reusable
scanner/parser primitives and whole-value versus mixed-string behavior. It also
identifies a variable fast path in the interpolation evaluator that currently
calls `get`/`get_string`; this must use D1's richer resolution contract. A
prepared artifact alone does not repair that bypass.

This decision settles the shared representation and behavior, not final Rust
names, error enums, or serialized artifact syntax. D5 supplies policy movement
semantics.

## D5 — Carry sparse policy and stage provenance with values

**Human decision:** confirmed 2026-09-18, option **B**. Use a Darkmatter-owned
value envelope containing ordinary JSON plus sparse subtree policy and stage
provenance. Claudine carries the complete envelope through overlays, runtime
mutation, sequence transfer, and retries whenever evaluation can resume.

**Recommendation presented:** option B, retaining metadata at the concrete
boundaries that need it without replacing every ordinary JSON node with a new
recursive value type.

| Material alternative | Benefit | Cost |
| --- | --- | --- |
| A: Recursive policy-bearing value tree | Each node naturally carries metadata through selection and movement. | Changes ordinary value operations and bridges throughout the pipeline, including consumers that need only plain data. |
| B: Ordinary JSON with sparse subtree provenance **(confirmed)** | Keeps ordinary data representation and records inherited restrictions and pending stages where needed. | Every boundary that can resume execution must carry the envelope atomically; bare-JSON conversion can otherwise lose policy. |

Deferred executable units retain their origin restrictions. At execution,
Darkmatter unions those restrictions with applicable destination/ancestor
restrictions and enclosing execution prohibitions; descendants and overrides
cannot relax the result. Generated executable descendants inherit the
generating operation's restrictions and their destination policy. This is
provenance for executable units and their remaining stages, not general taint
tracking of every value used as an operand.

The envelope records which existing composition stages remain. Successfully
completed output remains data, including expression-looking strings and escaped
templates; moving it does not add scanning or evaluation passes. A genuinely
pending ordinary stage still executes at its existing point, subject to the
combined restrictions. Policy checks occur before a prohibited feature runs.
The independent Claudine initialization shell backstop remains in force.

Darkmatter owns atomic selection, replacement, merge, and move operations for
the value and its metadata. Sparse locations must distinguish property names
from array indices without aliasing dotted keys. Selecting or moving a subtree
materializes inherited ancestor restriction and stage metadata at the selected
subtree's root before rebasing its sparse records. Copying only records located
inside the subtree would lose inherited policy. Replacing discarded content does not contaminate
unrelated replacement data with its provenance. Destination schema restrictions
still apply to the replacement. Claudine supplies domain identities and chooses
operations, but does not reconstruct policy propagation by traversing expression
trees.

The envelope survives while normal evaluation stages can resume, including
deferred lifecycle extraction, proxy refresh, batched `set`, mutation snapshots,
sequence layering, and retry preparation. It retains source restrictions without
retaining a lazy session. Re-preparation uses the new destination schema generation
and retained source restrictions; an old generation identifier is not permission
to bypass current policy. D2's cache lifetime remains unchanged.

The main integration risk is an executable transfer that silently strips the
envelope to plain JSON. Covered in-memory transfers must preserve it. If such a
transfer crosses serialization, provenance must accompany the value in an
internal representation, or the allowed stages must finish before plain-data
export. This decision introduces no new public persistence format and does not
authorize plain exported JSON to re-enter automatically as deferred source.

Verification must cover movement in both directions between restricted and
unrestricted locations, arrays and ambiguous-looking property names, sibling
isolation, descendant overrides, generated executable content, and initialization
recovery. Proxy, mutation, sequence, and retry transfers must retain provenance
and atomicity. Whole-value typing and all supported literal escapes must survive
the same transfers without extra evaluation. Any required serialized transfer
needs a value-plus-provenance round-trip check. The
[schema investigation](schema-investigation.md) records the transfer analysis
and these contracts.

Final Rust field names, internal serialization syntax where needed, and error
variants remain open; the carrier granularity and retention rules are confirmed.

## D6 — Put declarative bindings in the existing schema envelope

**Human decision:** confirmed 2026-09-18, option **A**. Add an optional generic
`bindings` section to the existing `kind: schema` envelope. Claudine owns one
authoritative YAML catalog; Darkmatter owns its shared format, parsing, and
binding mechanics.

**Recommendation presented:** option A, keeping declarations beside the schemas
that use them without a second document format and required companion-file
association.

| Material alternative | Benefit | Cost |
| --- | --- | --- |
| A: Optional `bindings` section on `kind: schema` **(confirmed)** | Keeps schema and binding policy together and gives runtime/editor consumers one authoritative source. | Extends the generic schema envelope and requires explicit scope/use-site semantics. |
| B: Referenced companion binding document | Separately reusable declarations and smaller schema documents. | Adds a document format, dependency edge, and association that loaders and authors must maintain. |

The catalog contains root names, static types, documentation, and default
availability. Opaque named scopes declare availability overrides and structured
unavailability reasons. Use-site schema locations select those scopes; runtime
callers can also select a named scope explicitly, such as for shell preflight.
Claudine gives scope names their lifecycle meaning. Darkmatter interprets the
generic declarations without knowing what an event means.

Importing a reusable type does not automatically select a lifecycle event scope.
That association belongs to the location using the type or to an explicit
runtime selection. This keeps common action/type definitions reusable across
events with different availability rules. The exact nested use-site syntax,
scope-conflict/merge rules, and complete lifecycle matrix still require design
agreement.

Declarations contain no value providers or executable predicates. Runtime
generation consumes this same YAML catalog, and Claudine associates the resulting
declarations with its eager/lazy/unavailable runtime entries through D2's checked
boundary. DMLS dynamically reads the original YAML through Darkmatter; it neither
depends on Claudine nor keeps a separate catalog. Serialization layout and the
generated artifact format remain open.

Malformed binding metadata and invalid reserved-root registrations produce
structured errors from the shared Darkmatter authority. Failure of a required
editor catalog marks dependent validation and assistance incomplete under the
specification's isolation/recovery rules. Mandatory runtime declarations and
restrictions remain independent of optional editor activation.

## D7 — Require complete runtime registrations

**Human decision:** confirmed 2026-09-18, option **A**. A runtime session must
have an explicit eager, lazy, or unavailable entry for every declared global.
Darkmatter checks completeness and configuration before evaluation without
invoking lazy providers. It must not infer a null-valued or unavailable entry
from an omitted registration.

**Recommendation presented:** option A, making lifecycle construction mistakes
visible at the binding boundary instead of letting expression branch selection
determine whether an incomplete session is detected.

| Material alternative | Benefit | Cost |
| --- | --- | --- |
| A: Complete, checked runtime entries **(confirmed)** | Deterministic configuration failures before evaluation; preserves available-null versus unavailable distinctions. | Callers explicitly construct entries even for declared globals an expression does not reference. |
| B: Sparse entries checked only on reference | Less runtime registration work for unused globals. | Missing-registration defects become branch-dependent and are easier to confuse with intentional unavailability. |

This completeness rule applies to sessions with a declared global catalog.
Ordinary lookups without a catalog retain D1's lightweight default. Editor and
other passive consumers need only declarations and never construct providers to
satisfy runtime completeness.

The confirmed `err` availability is:

| Lifecycle scope | Declaration and runtime entry |
| --- | --- |
| `initialize`, `start`, `success`, `loop`, task `setup` | Definitely unavailable with a structured scope-forbidden reason; explicit unavailable runtime entry. |
| `blocked`, `failure` | Available, with the required error value supplied. |
| `finalize`, task teardown | Available, with the error value when present or an explicit eager `null` when no error exists. |

An unavailable `err` never falls back to a same-named document property;
`doc.err` remains explicit document access. A missing required `err`, `timing`,
or `current` entry is a configuration error, not automatic null inference.
The caller must intentionally supply the available-null case where allowed.

Tests must distinguish complete unavailable entries from omitted entries,
exercise each confirmed `err` scope through passive validation and runtime
resolution, and prove invalid configuration fails before any provider invocation
even when the missing global would lie in an inactive branch. Finalize and task
teardown must accept explicit null while same-named document properties cannot
hide unavailability or missing registration. Ordinary no-catalog lookups and
provider-free editor validation retain their separate contracts.

This decision does not by itself settle group lexical scope or shell-preflight
binding policy; D8 supplies the sequence-wide approval ruling.

## D8 — Keep lexical group unavailable at sequence-wide shell approval

**Human decision:** confirmed 2026-09-18, option **A**. Lexical `group` is
unavailable while resolving shell bytes for sequence-wide approval, including
member primary commands, task setup/teardown commands, and referenced-prompt
commands covered by that earlier approval.

**Recommendation presented:** option A, preserving group observation timing and
the existing approval boundary without introducing early evaluation or another
approval phase.

| Material alternative | Benefit | Cost |
| --- | --- | --- |
| A: Make lexical group unavailable at sequence-wide approval **(confirmed)** | Preserves execution-entry group evaluation and one approval of fixed command bytes. | Group-dependent shell commands cannot use lexical group values at that early boundary. |
| B: Permit a bounded early-stable group subset | Could allow commands using provably fixed group values. | Requires new stability/dependency rules and an immutable projection shared by approval and execution; expands behavior beyond this fix. |

This restriction belongs to the approval phase. Group-variable definitions
continue to evaluate at execution entry; approval neither evaluates them early
nor adds reapproval. `doc.group` remains explicit document data under ordinary
early-state rules. Non-shell runtime group use retains existing scope and
timing. Reusable documents whose eventual group membership is unknown declare
that availability execution-dependent during passive validation.

Only callers installing the Claudine lifecycle/sequence catalog reserve `group`.
It does not become a Darkmatter built-in or a reserved name in unrelated loop
expression or dispatch lookups. An already-established group later in execution
does not override an earlier sequence-wide shell-approval restriction.

Resolved approved command artifacts must reach setup/teardown execution as well
as primary command execution. Re-parsing an authored stack and re-evaluating its
shell string against later group state cannot replace the approved bytes. This
preserves the specification's existing byte-parity requirement; no dynamic
reapproval mechanism is introduced. The
[lifecycle investigation](lifecycle-investigation.md) identifies the current
setup/teardown artifact-transfer gap.

The human authorized the narrow corresponding clarification in R6 of the
specification. Remaining scope composition, binding-association details, and
concrete command artifact interfaces still require completion.

## D9 — Generate checked-in Rust schema constructors

**Human decision:** confirmed 2026-09-18, option **A**. Extend the existing
`claudine-gen` generation/check workflow with `schemas` generation, producing
checked-in Rust constructors at
`claudine/lib/src/composition/schema/generated.rs`. The constructors build a
shared Darkmatter semantic schema definition bundle from authoritative
`claudine/schemas` YAML.

**Recommendation presented:** option A, using the existing independent generator
and Rust's type checking without creating a serialized compiled-schema format
or build-time generation step.

| Material alternative | Benefit | Cost |
| --- | --- | --- |
| A: Checked-in Rust constructors **(confirmed)** | Reviewable generated output, compile-time checking, and reuse of the existing generation/check workflow. | Generator must emit all semantic fields deterministically and adapt with shared Rust types. |
| B: Compiled JSON bundle | Compact data artifact and potential use outside Rust. | Introduces a serialization/versioning contract and loader validation for a representation not otherwise needed by this fix. |

The generated bundle preserves resolved types, binding declarations, named
scopes, feature restrictions, and portable source origins. It does not contain
compiled validator instances, live value providers, or developer-machine
absolute paths. Darkmatter constructs process-local validators from the shared
definitions. Mandatory Claudine runtime validation needs no repository schema
files on disk.

Dependency direction remains `claudine-gen` → Darkmatter and Claudine runtime →
Darkmatter. The generator must not depend on the Claudine library or CLI, and
normal builds do not run a schema-generation build script. DMLS continues to
load original YAML dynamically through the same Darkmatter schema engine,
without embedding Claudine's generated artifact or depending on Claudine.

The existing generator Level 1 verification includes deterministic generation
drift checking and semantic equivalence between the original YAML and generated
bundle. Verification must cover binding/scope and restriction metadata as well
as ordinary schema types; byte equality alone cannot establish runtime/editor
agreement. This joins existing package verification without a new CI matrix
cell.

D9 settles generation integration and artifact layout. It does not settle the
remaining binding syntax, scope-conflict rules, activation grammar, or source
precedence.

## D10 — Use the canonical `schema-trigger` kind directly

**Human decision:** confirmed 2026-09-18, option **A**. Adopt
`kind: schema-trigger` as the canonical trigger envelope spelling and migrate
active parser, schema, fixture, test, documentation, and skill usage directly.
Do not accept `trigger-schema` as an alias. Historical specifications remain
unchanged.

**Recommendation presented:** option A, aligning the implemented grammar with
the repository kind catalog without maintaining two names for one document kind.

| Material alternative | Benefit | Cost |
| --- | --- | --- |
| A: Direct migration to `schema-trigger` **(confirmed)** | One canonical spelling across parsing, schemas, and active guidance. | Existing legacy files need correction before they can activate. |
| B: Accept `trigger-schema` as an alias | Existing legacy files continue to load. | Preserves competing spellings and adds compatibility behavior without an established external-user requirement. |

The shared Darkmatter parser recognizes the old spelling as invalid and returns
a structured diagnostic naming `schema-trigger` as the replacement. Discovery
must not silently ignore a legacy trigger as an unrelated document. In DMLS a
broken legacy rule produces incomplete validation within its possible discovery
scope, using the same dependency suppression and recovery contract as other
broken activation definitions.

Tests must cover successful canonical parsing, the typed legacy-spelling error
and replacement guidance, active-artifact migration, and consumer-facing
incomplete validation with automatic sibling isolation and explicit-source
scope. No compatibility alias or silent loss of the rule is acceptable.

The human authorized the corresponding narrow compatibility clarification in
this fix's specification. Activation condition syntax and source precedence
remain separate pending decisions.

## D11 — Use AND at the top level and OR within condition groups

**Human decision:** confirmed 2026-09-18, option **A**. Adopt the desired
documentation's Boolean trigger grammar: a top-level `match` list combines its
entries with **AND**, while a group's conditions combine with **OR**. D10's
canonical `kind: schema-trigger` remains unchanged.

**Recommendation presented:** option A, adopting the explicitly requested
target semantics while migrating existing rules deliberately.

| Material alternative | Benefit | Cost |
| --- | --- | --- |
| A: Adopt documented top-level AND and grouped OR **(confirmed)** | Matches the requested authoring model. | Existing rules using implemented top-level OR require logic-preserving migration. |
| B: Preserve the existing Boolean grammar | Avoids changing the current parser's list semantics. | Does not deliver the requested Boolean authoring model. |

The existing implementation treats the top-level list as OR. Active rules must
be migrated to preserve their intended logic, not silently interpreted as AND
under the new grammar. Exact parser shape and legacy-form detection remain
pending; this decision does not select aliases, versioning, or a compatibility
mechanism.

The user identified
[authoring-schemas.md](../../../../darkmatter/docs/topics/schemas/authoring-schemas.md),
[schema-targeting.md](../../../../darkmatter/docs/topics/schemas/schema-targeting.md),
and [schema-target.yaml](../../../../darkmatter/docs/schemas/schema-target.yaml)
as target intent, not evidence of implemented support. Where those documents
use `schema-target`, the separately confirmed `schema-trigger` name takes
precedence. Their other proposals are not confirmed by D11. D12 separately
settles automatic loading of standalone schemas; additional predicates and
envelope details remain open.

The Boolean grammar is a technical choice delegated by R12; no functional
specification amendment is required for this selection. Verification must
distinguish top-level AND from grouped OR and prove migrated rules retain their
intended matching behavior. Predicate semantics and source precedence remain
pending; D12 settles automatic application within discovery scope.

## D12 — Apply discovered standalone schemas within their scope

**Human decision:** confirmed 2026-09-18, option **A**. Recognized standalone
schemas discovered in supported `schemas/` directories automatically augment
the Darkmatter baseline within their discovery scope. `schema-trigger`
definitions remain conditional on matching. This does not merge arbitrary YAML.

**Recommendation presented:** option A, making directory placement declare
applicability and reducing separate activation settings for always-on schemas.

| Material alternative | Benefit | Cost |
| --- | --- | --- |
| A: Automatically apply discovered standalone schemas **(confirmed)** | Predictable directory-based applicability with fewer activation settings. | Discovery must distinguish applicable standalone schemas from reusable helper definitions. |
| B: Require explicit selection for every always-on schema | Makes each always-on activation separately explicit. | Additional consumer configuration is required even after a schema is placed in its intended scope. |

Automatically discovered repository/opened-root, package-area, and package
schemas apply only within their associated scope; nested package definitions
cannot affect siblings. A valid explicit `SCHEMA_DIR` supplies workspace-wide
standalone schemas and conditional triggers, with relative imports retaining
their source origin. These sources augment rather than replace the base schema
and other applicable sources.

Reusable imported definitions must not accidentally become always-on document
schemas simply because they reside in a discovered directory. D13 supplies the
standalone-schema versus helper envelope/classification. Consumers also retain
the explicit direct-schema capability required by R12.

The human authorized a narrow R12 clarification of automatic standalone-schema
application. Verification must cover baseline participation, conditional
triggers, nested sibling isolation, explicit-source workspace scope, and the
exclusion of arbitrary YAML and reusable helpers from automatic application.
Source precedence is not settled here; D13 resolves the export distinction.

## D13 — Separate exported document schemas from reusable types

**Human decision:** confirmed 2026-09-18, option **A**. A `kind: schema`
envelope separates an optional exported `$schema` from reusable `types`.
A types-only file is importable but is not automatically applied to documents.

**Recommendation presented:** option A, making a file's applicability explicit
in its representation rather than relying on a helper-directory convention.

| Material alternative | Benefit | Cost |
| --- | --- | --- |
| A: Explicit exported `$schema` and reusable `types` **(confirmed)** | Co-locates imports and applicable schemas while making their different roles machine-checkable. | Existing helper files require explicit migration; whole-file callers must select an actual export. |
| B: Keep helper libraries outside discovered directories | Avoids extending the envelope distinction. | Directory placement becomes an implicit applicability contract and makes reusable-source organization fragile. |

Automatic discovery applies the exported document schema when present, within
D12's scope. A file containing only reusable types contributes no automatically
applied document schema. Whole-file schema references to a types-only file
produce a structured no-export error instead of guessing a default type or
merging every definition. Named type imports remain supported through explicit
helper-file migration.

D6's optional generic `bindings` section remains part of the schema envelope.
Importing a reusable type library does not automatically activate its bindings
or event scopes. Binding association syntax and scope-selection details remain
open; D13 does not infer them from the presence of a named type.

Verification must distinguish discovered exported schemas, discovered types-only
libraries, valid named imports, and invalid whole-file references with no
export. Importing a helper must not activate binding/event policy as a side
effect. The user also authorized documentation updates to describe this
distinction; that authorization does not implement the parser or migrate runtime
schema data during this design checkpoint.

Automatic assembly of exported unions remains unresolved. The existing
`schemas/feature-review.yaml` union must not be silently flattened or discarded
when implementing automatic application. The populated documentation draft
`darkmatter/docs/schemas/schema-definition.yaml` describes intended envelope
behavior, not implemented support; its detailed `bindings` grammar still awaits
agreement.

## D14 — Include the full desired activation predicate family

**Human decision:** confirmed 2026-09-18, option **A**. This fix includes
document expressions; file presence, absence, and content; executable
availability (found/absent); repository membership; OS; timezone; and local/UTC
time predicates.

**Recommendation presented:** option **B**, limiting the initial family to
document predicates and file presence/absence for the original scope and
simplicity. The human selected the broader option A explicitly.

| Material alternative | Benefit | Cost |
| --- | --- | --- |
| A: Full desired predicate family **(confirmed)** | Delivers the requested range of document, host, filesystem, and time activation conditions together. | Requires concrete passive observation, refresh, error, and cross-platform contracts for every family. |
| B: Document predicates plus file presence/absence **(recommended, not selected)** | Smaller extension around the original workspace-marker requirement. | Leaves the other desired predicates outside this fix. |

The expanded family does not authorize execution during validation. Activation
must not execute actions, shell expansion, lazy providers, or discovered
binaries. Availability checks observe whether an executable can be found; they
do not run it. Predicate observations and their refresh rules must preserve the
shared passive-validation boundary.

The human authorized the corresponding narrow scope amendment to R12. Exact
syntax, observation ownership and snapshots, time semantics, refresh/invalidation,
and typed failure behavior remain technical decisions. Responsiveness remains
qualitative, without numeric acceptance thresholds. Selecting the families does
not implicitly approve details in draft authoring documents.

## D15 — Observe requested activation facts before pure matching

**Human decision:** confirmed 2026-09-18, option **A**. Capture requested host
facts before matching into an immutable activation snapshot, with one clock
instant for the activation pass. Darkmatter identifies observation requests;
shared host integration gathers them using focused Sniff host/repository APIs.
CLI and DMLS use the same observation and matching semantics.

**Recommendation presented:** option A, making the observation boundary explicit
and matching deterministic without a request/resume protocol inside evaluation.

| Material alternative | Benefit | Cost |
| --- | --- | --- |
| A: Collect requested facts, then match a snapshot **(confirmed)** | Clear passive boundary, deterministic tests, and consistent clock/fact views within one pass. | Can observe requested facts in branches that matching later does not use. |
| B: Request host facts on demand and resume matching with a cache | Avoids observing branches not reached by matching. | Adds suspended evaluation and request/cache coordination to the shared matching contract. |

Capture only requested facts rather than a full ambient host snapshot. Absence
and failed observation remain distinct outcomes. Matching performs no I/O and
invokes no providers; tests can supply snapshots directly. All time predicates
in one pass use the captured instant rather than consulting the clock again.
This activation snapshot does not change Claudine lifecycle capture timing or
D2's lazy-value cache lifetime.

Existing APIs do not by themselves establish this boundary. The current
`EvaluationMode::Pure` selects dispatch behavior, not actual purity:
`date()`/`is_today` can consult the live clock. `CtxLookup` can capture lazily,
and ordinary `ResolutionContext` dispatch can perform remote reads. The matcher
must not receive those unrestricted contexts directly; snapshot-backed adapters
must enforce the agreed observation boundary.

Dynamic expression-derived observation requests and the allowed activation
expression functions remain the next consequential ruling. D15 does not assume
arbitrary read-side expressions can be precollected, nor authorize their
execution during matching. Concrete snapshot/request types, observation errors,
and refresh dependencies remain to be specified within this contract.

## Remaining decisions and design completion work

The priority below reflects architectural dependencies, not implementation
sequencing.

| Priority | Open subject | Consequence to resolve before planning |
| --- | --- | --- |
| 1 | Binding interface details | Specify the richer result, passive descriptor API, structured reason/error shapes, and remaining checked-association invariants within D1, D2, and D7. |
| 2 | Prepared interface details | Make D4's inputs, outputs, source provenance, schema-generation association, and typed error boundaries concrete without adding runtime observations to preparation. |
| 3 | Lifecycle catalog details | Complete D6's event/scope matrix and nested use-site syntax/conflict rules within D3's capture timing, D7's `err` mapping, and D8's sequence-approval restriction. |
| 4 | Provenance interface and transfer inventory | Specify D5's envelope operations and enumerate every executable transfer, including any required internal serialization boundary. |
| 5 | Shared semantic bundle details | Complete the shared definition fields and portable origin representation needed by D9, alongside D6's still-open binding payload details. |
| 6 | Generic activation and source precedence | Complete D11's parser shape and legacy-rule migration and D13's import/export/union contracts; define D14 predicate syntax/functions and dynamic requests under D15, detailed time/refresh/failures, plus source precedence/deduplication and origin versus applicability scope using D10's canonical trigger kind. |
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
review of the completed design remains pending. The
[second review](design-review-2.md) prompted two incorporated clarifications:
D5 selection/movement materializes inherited ancestor metadata, and D7 names
the `loop` event distinctly from task `setup`. Concrete envelope operations,
scope composition, and remaining checked-association contracts are still open.
An independent schema-agent review of D11–D13, R12, and the corresponding
documentation changes found no blockers in that checkpoint. The documentation
marks the desired contract as not implemented. That review does not resolve
union-export assembly or the pending binding grammar and is not approval of the
unfinished overall design. Open rulings,
unresolved graph evidence, and the incomplete migration inventory prevent a
claim that this design is ready for implementation planning.
