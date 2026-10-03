# Strict Mode Integration Design

Status: proposed integration design. This document identifies ideas to incorporate
into `2026-09-21-schema-enhancements`; it is not an implementation plan or a
claim that the changes have landed.

## Intent and authority

Make `2026-09-21-schema-enhancements` the primary design. Reconcile
`2026-09-17-remove-strict-mode` against it rather than implementing the older
specification as a separate prerequisite. Preserve useful behavioral contracts
without importing its entire architecture or duplicating work already specified
by the newer design.

[spec.md](./spec.md) remains normative for schema enhancements. This document
proposes additions and records their disposition; it does not silently amend
that specification. The older strict-mode implementation plan is superseded
and must not be used as the integration plan.

The author's clarification for this integration is explicit: `err` is a global
object made available in lifecycle events, already modeled by the schema work.
There is no `doc.err` model to introduce or preserve. The older specification's
same-named document-property examples are not carried forward. Existing global
schemas are the starting point, not a reason to create another error schema or
another catalog of lifecycle globals.

Lifecycle-based specifications will be discussed separately. This document
identifies the shared expression/schema boundary without deciding lifecycle
ownership, orchestration, or their implementation sequence.

## What remove-strict-mode contributes

The original failure was an optional document property rejected before ordinary
expression evaluation:

```text
{{ plan ? 'Plan: ' + plan : '' }}
```

With `plan` absent, evaluation should select the empty string. An internal
strict-root check instead treated absence as an invalid identifier. Claudine
also performed its own checks, so removing Darkmatter's check alone would not
fix every caller.

The older specification expanded from this defect into several subjects:

| Subject | Contribution |
| --- | --- |
| Document lookup | Bare document properties resolve through `doc`; absence produces `null`, not an invalid-root error. |
| Global bindings | A global's declared shape, runtime availability, and actual value are distinct facts. |
| Shared evaluation | Darkmatter owns expression parsing, resolution, and validation; consumers supply policy and values. |
| Consumer cleanup | Remove strict-root checks, duplicated expression walkers, and obsolete strictness APIs. |
| Restricted evaluation | Schema locations can prohibit shell expansion, including through deferred evaluation. |
| Schema authority | Runtime and editor derive their understanding from shared schema sources. |
| Editor infrastructure | Schema discovery, activation, layering, generation, and recovery use generic mechanisms. |

These subjects do not all need to become new schema-enhancement requirements.
Some are already covered, some are small additions, and some need a separate
scope decision.

## Already covered by the primary design

Schema enhancements already specifies the richer grammar, function signatures,
one function catalog, one coercion engine for frontmatter and function
arguments, and the shared compatibility API used by DMLS. It also requires the
complete function migration in one pass. These are not separate strict-mode
deliverables.

Use the final catalog migration directly. Do not first relocate the legacy
`expression-functions.yaml` as the older specification proposed and then
replace it in a second migration.

The global error object is already modeled. The remaining integration question
is how evaluation and passive validation consume the existing declaration and
the host's availability information, not how to invent an `err` type.

The parent `2026-09-16-expression-type-system` supplies the `unknown` and `null`
vocabulary. Reuse that vocabulary and the primary design's optional-property
typing; do not establish an independent declaration/type system for strict-mode
removal.

## Ideas to incorporate

### 1. Missing document data is a valid lookup

For roots that are document properties, `plan` and `doc.plan` have the same
meaning. An absent property evaluates to `null`. A schema declaration supplies
its static type but is not permission to use its name. An undeclared property
has unknown static type and remains a valid lookup.

Do not replace the current strict-root check with a schema-aware allowlist.
An optional property does not need `|| false` to become legal. A required
property may still fail document validation: valid lookup and valid document
shape are separate questions.

Bare document lookup must not fall back to a same-named context property.
Context access uses `ctx.*`.

This is a necessary addition because coercion alone cannot fix the motivating
ternary: it contains no function call, and a root check can reject it before
evaluation reaches the new engine.

### 2. Consume global declarations without confusing absence and availability

Integrate the existing global schemas with a shared binding contract that
distinguishes:

- an absent document property;
- an available global, including one whose value is legitimately `null`;
- a global unavailable in the current scope.

Schema knowledge does not make a global available at runtime. The host supplies
availability and values; Darkmatter interprets that information. An unavailable
`err` must produce a structured availability error, not become missing document
data. Authors cannot make a lifecycle global available by adding frontmatter.

Passive validation must use the same declarations without invoking lazy value
providers or executing effects. Definitely forbidden references can be rejected
before execution, including in inactive branches; availability dependent on
runtime state remains a runtime check.

Choose the implementation through the primary design's shared representation.
This proposal does not prescribe a second binding engine or a particular trait
redesign from the older design documents.

### 3. One executable-expression behavior across consumers

Remove the obsolete `SubtreeStrictness`, `.strict()`, and related mode selection
when the shared behavior is ready. Missing document data succeeds as `null`;
actual parse errors, unknown functions, rejected arguments, failed operations,
and prohibited features remain errors under their respective contracts.

Migrate Claudine's lifecycle interpolation, shell preflight, sequence values,
and direct evaluator consumers together where they rely on the changed API.
Replace consumer checks for identifier legality and global availability with
the shared Darkmatter operations. Retain walks that serve independent purposes,
such as dependency ordering or context-requirement discovery.

Preserve structured causes and source locations through consumer error wrappers.
Evaluation must complete before a batched mutation or action is committed;
removing strict mode must not permit partial effects after a genuine error.

### 4. Preserve completed literal output

Successful escaped output containing template syntax is data. Consumers must
not reject it merely because braces remain, or run an extra evaluation pass to
remove them. Preserve whole-value results, mixed-string interpolation, and
Darkmatter's supported escape forms.

This requirement protects the shared evaluator's output contract. It does not
absorb separately reported interpolation implementation defects into this work.

### 5. Align binding diagnostics with argument diagnostics

Use Darkmatter's binding classification for missing-property and unavailable-
global diagnostics, alongside the primary design's function compatibility API.
DMLS must not recreate either set of semantic rules.

An undeclared document property may receive an advisory; it must not be described
as an illegal identifier. An unavailable global or an impossible function call
is a different diagnostic. Preserve the primary design's five argument categories
and their severity rules.

The two uses of "strict" remain distinct:

| Surface | Disposition |
| --- | --- |
| Runtime subtree strictness | Remove the mode and its invalid-root gate. |
| DMLS `expressions.strict` | Keep the primary design's optional extra argument warnings, defaulting to `false`. |

The editor setting must not change runtime lookup, coercion, or global
availability. The primary spec currently says existing identifier diagnostics
are unaffected; incorporating this proposal requires an explicit reconciliation
of their wording/meaning, not silently overriding that statement.

## Ideas needing a separate scope decision

The older specification also proposes `no-shell-expansion` as an inherited,
non-relaxable schema constraint, authoritative generated lifecycle/global schema
artifacts, and extensive schema discovery, trigger activation, and editor
recovery behavior.

These ideas should be checked against the primary design and existing schema
work before assigning implementation. They are not established as covered merely
because both specifications discuss schemas, and they are not automatically
prerequisites for the coercion engine.

For each uncovered requirement, choose explicitly between an addition to schema
enhancements and a separately owned follow-up. Preserve existing shell-free
initialization and approval boundaries throughout integration. Do not use this
reconciliation to weaken them or to pre-decide the lifecycle design discussion.

Context refresh timing, proxy execution, loop behavior, process outcomes, shell
result suffixes, and moving lifecycle execution between packages remain outside
this document's proposed additions.

## Acceptance outline for the incorporated behavior

These checks supplement, rather than replace, the primary specification's
catalog-wide coercion and editor acceptance criteria:

1. The motivating ternary succeeds with `plan` absent through ordinary
   composition and Claudine lifecycle evaluation, without an authored guard.
2. An optional schema-declared property can be absent; an absent required
   property still fails schema validation. Undeclared document lookup remains
   valid and does not fall back to `ctx`.
3. An available lifecycle `err` resolves through its global declaration; an
   unavailable `err` fails with a structured cause. Passive checking invokes
   no value provider or effect.
4. Actual expression defects still fail, while escaped template output survives
   unchanged under the ordinary escape rules.
5. Consumers agree on resolution and diagnostics, retain typed error causes,
   and do not partially commit failed batched evaluations.
6. DMLS advisories describe missing document data accurately. Toggling
   `expressions.strict` changes only the specified argument warnings.
7. Obsolete strictness APIs and duplicate legality checks are removed; useful
   dependency/context analysis remains. The final function catalog is the only
   catalog, as already required by the primary specification.

## Integration outcome

The intended result is one primary schema design with the useful missing-data,
global-availability, evaluation, and diagnostic contracts incorporated into it.
There should be no separate large strict-mode implementation prerequisite.

Before retiring `2026-09-17-remove-strict-mode` as an independent workstream,
account for each requirement as covered, incorporated, explicitly superseded,
or assigned to follow-up. This document does not change that specification's
status, approve implementation, or settle the lifecycle-based specifications.
