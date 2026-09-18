---
created: 2026-09-17
status: finalized-spec
clarified: true
reviewed: true
needs_rulings: false
clarified_by: codex/gpt-6-astra
reviewed_by: "review_refined_spec (codex/gpt-6-astra)"
reviewed_on: 2026-09-17
review_iterations: 0
implemented: false
review_note: the clarification process served as a review
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
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
area: claudine
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-cli
---

# Remove Strict Mode and Centralize Expression Binding

## Outcome

Darkmatter has one expression language and one variable-resolution model. A
bare identifier that is neither a reserved namespace nor an explicitly
registered global is a document
frontmatter property, equivalent to the same property beneath `doc`. Lookup of
an absent property is valid and evaluates to `null` at runtime. Its static type
comes from the schema when declared; otherwise its static type is unknown. It
is not an "unknown root" and does not require `||` to become legal.

Remove `SubtreeStrictness` and every runtime check that tries to distinguish a
declared frontmatter root from an undeclared one. Keep failures for actual
expression defects: malformed syntax, unknown functions, invalid function
arguments, failed file-backed operations, unavailable lifecycle-only globals,
and prohibited features at restricted schema locations. Preserve ordinary
Darkmatter literal and escape behavior for successfully evaluated data.
Claudine must
use those ordinary Darkmatter semantics for lifecycle actions, shell preflight,
sequence values, and every other `SubtreeCompose` consumer.

Darkmatter must also own the distinction between a missing document property
and an unavailable reserved global. Claudine supplies the lifecycle policy —
which names are reserved globals, which event makes each global available, and
the eager or lazy value of an available global — but it must not parse or walk
expression trees to enforce that policy. Darkmatter resolves and validates the
resulting binding environment through the same expression model used at
runtime.

Lifecycle properties such as `start`, `success`, and `failure` use ordinary
Darkmatter frontmatter semantics. Claudine supplies their schemas, additional
late-bound globals, and feature restrictions for selected schema locations;
Darkmatter performs all expression, schema, and result validation. The
`initialize` location must disable shell expansion because it executes before
preflight.

Static tooling may continue to warn that a document property is not declared
in authored frontmatter or the effective schema. That warning is advisory and
must describe a valid, unknown-typed document property rather than an invalid
identifier.

## Reported Failure

From the `claudine` package area:

```sh
compose prompts/implement.md \
  spec='fixes/2026-09-16-better-spec-syntax/spec.md' \
  -y --codex --model gpt-5.6-sol
```

The specification was already marked `implemented: true`, and no pending
review file existed. The implementation router therefore reached its final
diagnostic action, which contains:

```text
{{ plan ? '- a plan file was identified: ' + plan : '' }}
```

Only `spec` was supplied. `plan` is also declared by the prompt's `$schema`,
but no value was assigned for this invocation. Instead of evaluating the
condition as falsy, Claudine stopped during `initialize`:

```text
CompositionError: lifecycle evaluation error

Reason: Transform error: subtree-compose strict mode: unknown root 'plan' in
'{{ plan ? '- a plan file was identified: ' + plan : '' }}'
```

The expression is valid. `plan` means `doc.plan`; because that property is
absent, it evaluates to `null`, and the ternary must select the empty-string
branch. Rewriting it as `plan || false` would hide the engine defect and would
incorrectly teach authors that optional document properties require an escape
hatch.

## Root Cause

Darkmatter's ordinary evaluator already has the correct missing-value
behavior:

```rust
Expr::Variable(path) => Ok(lookup.get(path).unwrap_or(Value::Null))
```

`SubtreeCompose` bypasses that behavior in its `Strict` branch. Before the
evaluator runs, `validate_strict_roots` walks the expression and asks
`LayeredLookup::is_known_variable_root` whether each root is an injected
global, a reserved namespace, or a key currently materialized in
`EffectiveState`. An absent document property fails that closed-world test,
even though the expression evaluator can represent it as `null` and even when
the effective schema declares it.

Claudine duplicates the same policy. `first_undefined_stack_variable` and its
supporting walkers compare expression roots with the current frontmatter map.
The executor invokes that check before evaluating:

- `when:` conditions;
- lifecycle message expressions;
- typed lifecycle values, including `proxy.with` and mapping-based `set`
  values.

Consequently, removing Darkmatter's pre-pass alone would leave several
Claudine paths with the same invalid behavior.

The duplication is encouraged by a weaker seam below strict mode:
`EvaluationLookup::get` returns `Option<Value>`. That shape cannot distinguish
these semantically different outcomes:

- the path is an absent document property and therefore valid `null`;
- the root is a reserved global that is unavailable in the current scope;
- the root is an available global whose value is actually `null`.

`lifecycle_injected_globals` currently represents an unavailable global by
omitting it from the injected map. `LayeredLookup` then falls through to the
document lookup. Claudine has to prevent that fallback with its own AST scans,
including the separate `err` availability validator. This is the wrong
abstraction boundary: Darkmatter performs the binding, so Darkmatter must be
able to represent all three outcomes. Claudine should only describe the
lifecycle scope.

The mode itself is not a user-facing Claudine concept. It was added as an
internal Darkmatter API for late-bound lifecycle interpolation. Every
production `SubtreeCompose` caller selects `.strict()`; the lenient branch is
retained only by tests and the public API. The enum therefore does not express
a real production choice. It couples two unrelated questions:

1. whether genuine parse/evaluation failures are returned; and
2. whether an absent bare document property is legal.

The first is required for executable expressions. The second is part of the
language's namespace and null semantics and must not vary by caller.

## Language Contract

### Bare identifiers are document properties

Resolution follows this order:

1. The reserved `doc`, `ctx`, and `env` namespaces resolve through their
   respective lookup surfaces. Caller globals cannot replace these namespaces.
2. An explicitly registered global owns its exact root. Claudine's
   lifecycle globals such as `err`, `timing`, `current`, and `group`
   are examples.
3. Every other bare root resolves as a property of the current document.

Darkmatter rejects caller global registrations named exactly `doc`, `ctx`, or
`env` with a structured configuration error before evaluation or lazy-provider
invocation. DMLS reports the same invalid registration through shared
Darkmatter validation. Ordinary globals retain their precedence over
same-named document properties.

There is no implicit fallback from a missing bare document property to a
same-named `ctx` property. Authors use `ctx.<name>` for runtime context and
`env.<name>` for environment variables. A document property literally named
`ctx`, `env`, or `doc` remains reachable through the existing explicit
`doc.ctx`, `doc.env`, or `doc.doc` spelling.

These pairs are semantically equivalent:

```text
plan                 doc.plan
plan.path            doc.plan.path
items[0]             doc.items[0]
```

Injected globals retain their documented precedence over document properties
with the same root. An out-of-scope reserved lifecycle global remains an error;
it must not silently fall back to a document property. `doc.err` remains the
explicit way to read a document property named `err`.

### Missing properties are valid lookups

An absent document property evaluates to JSON `null`. Its static type comes
from the effective schema when declared; otherwise its static type is unknown.
Existing null propagation, truthiness, interpolation, comparison,
member access, and indexing rules remain authoritative. In particular:

```text
{{ missing }}                         null as a whole value
prefix {{ missing }} suffix           "prefix  suffix"
{{ missing ? 'yes' : 'no' }}          "no"
{{ missing || 'fallback' }}           "fallback"
```

The fallback operator remains useful for selecting a default value. It is not
a declaration, permission, or guard that makes the primary identifier valid.

A property declared by `$schema` but omitted from the current document obeys
the same runtime rule. Its static type comes from the schema where available;
otherwise its static type is unknown. Runtime state supplied by CLI values,
proxy overlays, lifecycle mutation, or sequence state may provide the property
later without changing the expression's validity. Valid lookup does not imply
that the complete document passes schema validation: an absent property that
the effective schema requires still produces a blocking schema error.

### Actual expression defects still fail

Removing strict mode must not make executable expressions best-effort. A
`SubtreeCompose` operation returns an error for failures the parser or evaluator
actually raises, including:

- malformed expression syntax;
- an unknown function name;
- wrong function arity or argument type when the function contract rejects it;
- a failed read-side operation, including invalid or missing required file
  references under its existing error policy;
- a lifecycle-only global used outside an event where it is available;
- use of a feature disabled for the applicable schema location.

An absent document property is not on this list because evaluating it
successfully produces `null`.

### Successful literal output retains ordinary Darkmatter semantics

Lifecycle frontmatter uses ordinary Darkmatter interpolation behavior,
including escaping and literal output. Triple-brace escaping intentionally
produces literal double-brace template text in the composed output so that,
for example, documentation can show template syntax without executing it.
That output is complete literal data, not unfinished or unresolved evaluation
merely because it contains braces. The supported escape forms include triple
braces, `\{{ some_variable }}`, and `\{\{ some_variable }}`. All three express
the same intentional literal template text without interpolating that text.
Preserve Darkmatter's established output and escape rules for each form; none
may be rejected by Claudine merely for producing double-brace template text.

Preserve existing whole-value return behavior and mixed-string interpolation
passes. Remove Claudine's additional post-evaluation rejection based merely on
expression-looking spans remaining in a successful result. Do not add an
extra evaluation pass, change escaping, make genuine parser or evaluator
failures best-effort, or bypass prohibited-feature checks. Darkmatter remains
the owner of all validation.

This ordinary-semantics clarification supersedes the earlier question about
blanket remaining-span rejection; no separate literal-output restriction is
part of this change.

## Ownership Seam

### Darkmatter owns expression and binding mechanics

Darkmatter owns:

- parsing and expression-tree traversal;
- binding precedence and namespace resolution;
- the distinction between document properties, reserved namespaces, available
  globals, and unavailable globals;
- missing-document-property-to-`null` behavior;
- eager and lazy global evaluation;
- typed errors for an unavailable reserved global;
- fail-fast executable subtree interpolation;
- validation of caller-supplied frontmatter schemas and evaluated results;
- schema-location-specific feature restrictions supplied by callers; and
- the reusable validation operation that checks an expression against a
  supplied binding environment, including references in inactive branches.

The binding model must be able to express, by behavior rather than necessarily
by these exact Rust names:

```text
DocumentProperty
AvailableGlobal(value or lazy provider)
UnavailableGlobal(reason)
ReservedNamespace
```

An implementation may evolve `EvaluationLookup`, add a richer resolver beside
it, or place the distinction in the layered document lookup. The public design
must nevertheless preserve the semantic distinction all the way into the
evaluator; converting every case to `Option<Value>` before evaluation is not
sufficient.

### Claudine owns lifecycle policy and orchestration

Claudine owns:

- the catalog of lifecycle globals such as `err`, `timing`, `current`, and
  `group`;
- the event or execution scope in which each global is available;
- construction and capture timing of their eager or lazy values;
- lifecycle flow, action dispatch, and mutation atomicity;
- lifecycle schemas and schema-location-specific feature policy;
- shell approval and byte-parity policy; and
- contextual projection of Darkmatter errors with event, action, property,
  task, and source-path identity.

Claudine provides those declarations to Darkmatter. It must not independently
decide identifier validity, traverse Darkmatter ASTs to find unavailable
bindings, or recreate evaluator short-circuit rules.

Scope clarification agreed 2026-09-18: this fix retains the existing
event-captured `current.ctx.*` and `current.env.*` snapshots and introduces no
independent `current_env` global. Laziness continues to defer snapshot
materialization, not observation of context or environment. The separate
[more-context specification](../../../../darkmatter/features/2026-09-09-more-context/spec.md)
owns the coherent future migration to Darkmatter built-ins across all expression
surfaces, the direct `current.*` and `current_env.*` mirrors, and reference-time
freshness. This clarification does not reverse that feature's agreed destination
or introduce interim aliases.

During preparation, Darkmatter must reject every reference that is definitely
forbidden in the declared event or scope, including references in inactive
branches. Availability that depends on execution state is checked at runtime.
Preparation must be passive: validation cannot invoke lazy value providers or
perform effects merely to determine availability. Both phases use the same
binding declarations and variable classification.

Passive validation is blocking, not advisory: a required schema or expression
validation failure stops normal execution at that boundary. The operation
that failed validation cannot proceed to its lifecycle actions, provider
launch, or prohibited expansion. Passive means that validation itself performs
no effects; it does not mean that the failed operation continues. Existing
error reporting and recovery policy remains in force, including applicable
failure, catch, and finalize routes; those routes must obey all feature
restrictions, including the preflight shell prohibition. This requirement
does not introduce rollback of effects that completed before a later validation
boundary.

Darkmatter returns structured errors to its library callers. The `claudine`
library adds lifecycle context while preserving the original typed Darkmatter
cause, so a library caller can inspect the failure and choose its own
presentation. `claudine-cli` only renders those errors; it does not own
validation. The existing lifecycle evaluation wrapper retains context and a
reason classification but does not retain the original typed Darkmatter
cause. Preserving that cause is a required change, not an existing guarantee.

## Required Behavior

### R1. Darkmatter exposes one `SubtreeCompose` behavior

Remove the public `SubtreeStrictness` enum, `SubtreeCompose::strict`,
`SubtreeCompose::with_strictness`, and the strictness argument from the
`compose_subtree` convenience function. `SubtreeCompose` must have one
production behavior: evaluate the supplied subtree and return genuine
parse/evaluation failures.

Remove `validate_strict_roots`, `collect_variable_roots`, and
`EvaluationLookup::is_known_variable_root`. Do not replace them with a
schema-aware allowlist or another closed-world root inventory. Schema metadata
improves static type information; it does not determine whether a document
property name is legal.

The subtree interpolator may invoke the existing interpolation machinery in
fail-fast form so malformed mixed strings do not leak into lifecycle messages,
shell commands, or sequence values. This is ordinary executable-expression
error propagation, not a named mode and not a different namespace policy.

### R2. Darkmatter exposes a first-class binding environment

Introduce a Darkmatter-owned representation that distinguishes document
properties, reserved namespaces, available globals, and unavailable globals.
It must preserve a real `null` global value as distinct from an unavailable
global and an absent document property.

Reject caller global registrations named exactly `doc`, `ctx`, or `env` with a
structured configuration error before evaluation or lazy-provider invocation.
The shared passive validation operation must expose this error to DMLS as
well. Explicit document access through `doc.err`, `doc.doc`, `doc.ctx`, and
`doc.env` remains valid regardless of same-named document properties or ordinary
global declarations.

An unavailable global is still reserved. It must not fall through to a
same-named document property. For example, if `err` is unavailable during
`initialize`, bare `err` raises the typed unavailable-global error even when
the document contains an `err` property. `doc.err` remains the explicit access
to that document property.

Provide both runtime resolution and reusable parser-aware validation against
this environment. Darkmatter owns expression traversal in both cases. The
validation API must enforce definite scope restrictions independently of
runtime branch selection without reviving a closed-world inventory of document
property names. Execution-dependent availability remains a runtime check;
validation must not invoke lazy providers or effects.

Do not require every lightweight `EvaluationLookup` implementor to reproduce
the classification manually. Supply defaults or adapters where ordinary
`None` means a missing document property, and require richer behavior only for
surfaces that declare reserved globals. The technical design must inventory
all lookup implementations and record their migration or compatibility
disposition before planning the coordinated change to the public trait.

### R3. Bare lookup never falls through to `ctx`

Update Darkmatter's effective-state lookup so a bare name reads document state
only. Remove the compatibility fallback that currently attempts
`get_context_value(path)` after a missing frontmatter lookup. Context remains
available exclusively through `ctx.*`.

Apply the same rule to every `EvaluationLookup` implementation. A consumer may
inject a true global explicitly, but it must not infer one from a missing
document property or from a matching context tail. Add a parity inventory so
new lookup implementations cannot silently restore the fallback.

### R4. Claudine declares lifecycle bindings but does not inspect expressions

Claudine must describe the complete lifecycle-global catalog and the
availability of each global for the current event or execution scope. Do not
represent unavailability by omitting a known global from the injected map.
Pass an explicit unavailable binding with enough structured reason data for
Darkmatter to return a typed error.

The declaration is domain policy and remains in Claudine. Binding precedence,
path resolution, expression walking, and availability enforcement belong to
Darkmatter. Prepare-time and event-time checks must consume the same binding
declarations so they cannot disagree about whether a name is a document
property or lifecycle global.

### R5. Claudine removes duplicate expression traversal

Remove the event-time calls to `first_undefined_stack_variable` from
`when_matches`, `render_message`, and `resolve_typed_value`. Remove the helper
and the root-inventory walkers when no valid producer remains.

Audit `validate_no_undefined_lifecycle_variables`. It is not a valid runtime
or prepare-time rejection under this contract. Remove the validator,
`CompositionError::LifecycleUndefinedVariable`, its renderer branches, and
tests that exist only to enforce closed-world document keys unless another
independent contract still requires a narrower diagnostic. Do not retain a
dead public or crate-visible validator solely for compatibility.

Replace the custom AST traversal inside `validate_no_err_in_no_error_events`
with Darkmatter's generic binding validation. The Claudine policy remains:
bare `err` in `initialize` is invalid because `err` is a reserved but
unavailable lifecycle global there, while `doc.err` remains valid document
access. If the function remains as a lifecycle-oriented adapter, it must only
construct the event-specific binding environment, invoke Darkmatter, and map
the typed result into Claudine's diagnostic context.

Remove other Claudine walkers that exist only to recognize variable roots,
reproduce short-circuit behavior, or detect surviving spans. Darkmatter owns
all expression, schema, and result validation, including enforcement of
caller-supplied policy. Remove extra post-evaluation checks that reject
successful literal output merely because it contains expression-looking
braces; preserve ordinary Darkmatter evaluation and error behavior.

### R6. All Claudine subtree consumers migrate together

Migrate every production `SubtreeCompose` consumer to the single Darkmatter
API in the same change:

- lifecycle event-time interpolation;
- lifecycle shell-command preflight;
- sequence shell preflight;
- sequence task value resolution.

Preserve each caller's contextual error wrapper, source path, and property or
task identity, and retain the original typed Darkmatter error through the
Claudine library boundary. Where the current wrapper loses the typed cause,
extend it rather than preserving that loss. Removing `.strict()` must not
flatten diagnostics or weaken shell approval's byte-parity guarantee.

Lifecycle communication, proxy overlays, and batched `set` evaluation remain
atomic: resolve the complete value before dispatch or mutation. A genuine
evaluation error still prevents partial effects. A missing document property
is a successfully resolved `null`, not a partial failure.

### R7. Static diagnostics consume the shared binding model

DMLS may warn when a bare document property is absent from both authored
frontmatter and the effective schema. The diagnostic must be advisory and make
the runtime contract clear: the property is valid, currently undeclared, has
unknown static type, and resolves to `null` unless supplied at runtime.

Replace terminology such as "unknown identifier" where it implies the name is
not part of the language. Prefer "undeclared document property" or equivalent
wording. If the existing diagnostic code is renamed, migrate all tests and
documentation atomically; do not retain two diagnostics for the same condition.

Unknown functions remain errors because function names occupy a closed catalog.
Unknown `ctx.*` tails may retain their existing schema/catalog-aware advisory,
but runtime lookup still resolves a missing tail to `null` unless a more
specific function contract rejects that value.

DMLS must consume Darkmatter's binding classification or a descriptor catalog
derived from it. It must not maintain another hardcoded list that can drift
from runtime global precedence or availability. Claudine-specific lifecycle
descriptors may be supplied as an extension, but the classification and
validation machinery remain Darkmatter-owned.

DMLS must also show authors error diagnostics for schema feature violations
under the applicable schemas and binding descriptors,
such as shell expansion prohibited in `initialize`, and references to lifecycle
globals definitely unavailable in the declared event or scope. These are
actual defects, distinct from advisory reports for valid but undeclared
document properties. DMLS must use the same Darkmatter schema and binding
validation as preparation, including definite scope violations in inactive
branches.

Editor validation is passive: it must not execute actions or shell expansion,
perform file side effects, or invoke lazy value providers. Availability that
depends on execution state remains deferred; DMLS must not guess runtime facts
and report them as definite errors. The authoritative Claudine schemas are the
repository YAML files specified in R11; editor and runtime validation must
consume that same source. DMLS dynamically activates generic schema data as
specified in R12. Distribution or registration details and the format and
availability of lifecycle-global descriptors remain design decisions; no
Claudine dependency, hardcoded detection, or separate catalog is authorized.

When the optional lifecycle extension is absent or inactive, DMLS validates
against the Darkmatter baseline and other applicable schemas and descriptors;
it does not promise Claudine-specific lifecycle checks. A missing required
dependency of an active extension instead follows the incomplete-validation
policy in R12. Optional editor activation does not control Claudine's mandatory
embedded runtime policies.

### R8. Do not patch authored prompts

Do not add `|| false`, synthetic null frontmatter keys, schema materialization,
or prompt-specific guards to make valid expressions pass. In particular,
`prompts/implement.md` must retain the ordinary ternary form from the reported
case as a regression fixture.

Shipped prompts that previously added `|| false` solely to appease
closed-world root validation should be audited. Remove those workarounds when
their only purpose was variable legality; retain a fallback when it expresses
an actual desired default value.

### R9. Documentation and skill snapshots use one vocabulary

Update Darkmatter expression and interpolation documentation, Claudine's
lifecycle and composition topics, and the mirrored Claudine skill references.
Remove claims that an "unknown root" is a runtime expression failure, that
optional names need `||` to become legal, or that lifecycle interpolation has a
strict mode.

Keep the useful fail-closed statement, but state it precisely: executable
expressions fail on parser/evaluator errors and prohibited features; missing
document data evaluates normally as `null`. Describe expression-looking result
text according to ordinary Darkmatter literal and escape semantics.

The historical late-binding specification and completed implementation logs
remain historical records. Current topic docs, code documentation, timeline
entries, and skill snapshots must describe the corrected contract.

### R10. Darkmatter enforces feature restrictions by schema location

Darkmatter must accept caller-supplied schemas for lifecycle properties and
allow callers to disable features for selected portions of a frontmatter
schema. Claudine declares the policy; Darkmatter validates and enforces it
through preparation and deferred evaluation. Deferred values must retain the
restrictions of their applicable schema location. A feature-policy violation
is a schema validation failure and blocks execution before the prohibited
feature runs. Evaluation-time enforcement covers content unavailable during
earlier validation; it is not permission to continue after an earlier failure.

Claudine disables shell expansion for `initialize`, which executes before
preflight. Preserve the existing initialization boundary: shell actions are
forbidden even in dead branches, bootstrap frontmatter shell expansion is
forbidden, and approval flags, whitelists, caches, handlers, and `no_error`
cannot override the prohibition. Early blocked/failure/finalize and catch
routes remain shell-free until the existing post-preflight boundary. Keep the
runtime execution backstop alongside Darkmatter validation.

Use the dedicated `SimplifiedSchema` constraint `no-shell-expansion` to carry
this restriction. When assigned to an array or key/value container, it applies
to every descendant child node and cannot be relaxed by a descendant schema or
an override. The `initialize` schema must describe its key/value structure
narrowly rather than accepting a broad `object` type.

Narrow object schemas already support postfix constraints, native mapping
`$constraints`, and constrained named-type imports. Reusing the existing narrow
lifecycle type is the recommended representation. For example, the target
syntax after implementation is:

```yaml
initialize: "lifecycle-event(no-shell-expansion)@./claudine-types.yaml"
```

The named-import syntax already exists; the `no-shell-expansion` keyword does
not. Implement its parsing, shared metadata, validation, composition enforcement,
and DMLS support. This example preserves the named type's explicit lifecycle
properties rather than substituting a generic object. Correct the schema
documentation that currently claims native mapping schemas are unsupported;
the parser already supports them.

This change requires `no-shell-expansion`; it does not require a catalog of
additional feature restrictions. Handling generated or moved values remains a
technical design choice subject to the confirmed inheritance and non-relaxation
rules, narrow initialization typing, and initialization prohibition.

### R11. Repository YAML owns schemas and generates runtime embedding

Claudine schema definitions must be developer-editable YAML files in
`claudine/schemas`. Those files are authoritative; a code generation step must
bring their definitions into the Claudine binary for runtime use. Do not
maintain a separate hand-authored runtime schema that can diverge from the
YAML. DMLS must validate against schemas derived from the same authoritative
files.

Move Darkmatter's schema directory from `darkmatter/docs/schemas` to
`darkmatter/schemas`, separating Claudine-owned definitions by their contents.
The verified migration inventory is:

| Current files beneath `darkmatter/docs/schemas` | Destination or disposition |
| --- | --- |
| `claudine.yaml`, `claudine-types.yaml`, `err.yaml`, `env.yaml` | `claudine/schemas` |
| `darkmatter.yaml`, `expression-functions.yaml` | `darkmatter/schemas` |
| `darkmatter-schema.md` | Keep as documentation and update its schema transclusion |
| `schema-definition.yaml` | Empty placeholder; inspect references before deciding its disposition |

`err.yaml` references the empty `schema-definition.yaml` placeholder; it is
not a functioning schema dependency. Preserve the existing
`claudine/schemas/review.yaml`. The five empty files and draft `action.yaml`
under `claudine/docs/schemas` must not overwrite populated source schemas.
Avoid broad path replacement: inspect references individually so unrelated
fixture paths are not rewritten. Do not create competing schema copies.

Update affected schema imports, runtime loading or embedding, generation
inputs, fixtures, documentation links, and skill references as part of the
migration. Verify that generation preserves the YAML schema definitions and
that the runtime and editor use equivalent schemas. Add a drift check through
the existing package verification workflow, without introducing a CI matrix
cell.

Darkmatter currently embeds schema text with `include_str!` and parses it at
runtime; that is not an existing schema code generation pipeline. The existing
`claudine-gen` package generates provider catalogs, so schema generation needs
new integration. The technical design must avoid dependencies that create a
cycle between the generator, Claudine library, and CLI.

Generation integration and artifact layout are technical design choices. DMLS
already supports generic runtime schema triggers: it discovers schema files in ancestor
`schemas` directories within the nearest workspace and passes its trigger
registry to Darkmatter's effective-schema resolution. Document changes,
configuration changes, and watched schema-file changes refresh the applicable
state. Schema sources are disk-backed; unsaved schema edits are not guaranteed
to affect other documents. Files outside a workspace have no such discovery.

Dynamic trigger integration is required as specified in R12. Reuse the
existing generic mechanism where it satisfies that contract; extend it where
needed for generic workspace marker facts. New feature restrictions and global
descriptors require shared payload support, and normal schema precedence must
not relax inherited `no-shell-expansion` constraints. Mandatory Claudine runtime
restrictions must not depend on optional editor trigger matching.

Repository schema ownership, YAML source authority, and discovery directories
in R12 are settled. A concrete workspace marker filename or trigger matching
syntax is not implied.

### R12. Shared schemas support dynamic, neutral editor activation

DMLS always begins with the base Darkmatter schema. Additional schemas augment
that baseline rather than bypassing it. Darkmatter consumers must be able to
supply direct, always-on schema files as well as conditional trigger files;
the same Darkmatter schema engine must apply these inputs consistently during
composition and validation.

The Darkmatter grammar owns generic constraints such as `no-shell-expansion`
and their subtree semantics even when the base Darkmatter schema does not use
that constraint. Consumer schema data selects constraints; it does not supply
a separate grammar or validator.

DMLS must not compile Claudine schemas or triggers into itself or depend on
Claudine to recognize them. Its relationship to those schemas is neutral and
dynamic. Claudine's separately required code-generated runtime embedding does
not authorize embedding that data in DMLS.

DMLS discovers schema files and trigger definitions under `schemas/` at each
repository root, or at the opened tree root when the tree is not a repository.
Monorepos also support `schemas/` at package-area and package roots. Automatically discovered definitions
associated with one of these scopes may only activate for documents inside
that scope: automatic package or package-area discovery must never affect
sibling scopes. Discovering a directory does not mean merging every YAML file into the
schema; conditional trigger definitions remain conditional, and direct
always-on schema inputs remain an explicit Darkmatter consumer capability.

When `SCHEMA_DIR` is set and names a valid directory, DMLS must also search that
directory. The directory may be anywhere, including another package's schema
directory. Its definitions are available workspace-wide, subject to their
trigger conditions. This explicit source is an intentional exception to the
scope limits of automatic discovery; do not impose additional source-location
classifications. Relative imports retain the schema file's source origin.
This is additive discovery, not replacement of root and nested `schemas/`
locations. Reuse existing schema merge precedence where compatible with the
confirmed non-relaxable restrictions. Invalid environment-value diagnostics
remain an implementation detail.

DMLS must evaluate applicable definitions at startup and refresh the effective
schema when relevant filesystem facts change. Create, update, and delete
changes must be reflected, including a newly introduced `schemas/` directory
at any supported root. Generic activation must support workspace marker facts:
the presence of a Claudine configuration file at a workspace root is an
illustrative use case, not a selected filename or hardcoded Claudine rule.
The loader, environment read timing, and change-monitoring strategy are
implementation choices; this specification does not require eager scanning of
every subtree rather than discovery as documents become relevant.

If an applicable schema, trigger, import, or global descriptor fails to load
or refresh, DMLS must report that validation is incomplete and identify the
failing source and cause. Suspend checks that depend on the failed definition
and continue independent checks. Do not present dependent diagnostics from an
earlier successful load as current, report partial validation as complete, or
fall back to stale definitions. Withhold hover type information and completion
suggestions that depend on the failed definitions, while retaining independent
editor assistance. A successful refresh restores dependent checks and editor
assistance and clears the corresponding failure diagnostic; stale definitions
must not supply a fallback for hover or completion.

When a failed activation rule prevents DMLS from determining which documents
it applies to, report incomplete validation for documents being checked within
the rule's possible discovery scope. Continue checks demonstrably independent
of that rule. An automatically discovered nested rule cannot affect sibling
scopes; a rule supplied through `SCHEMA_DIR` has a possible scope covering the
workspace. Do not assume that an unreadable rule is inactive.

For example, consider a hypothetical `payments/schemas/` directory with an
activation rule intended to select documents whose frontmatter declares
`kind: task`. If that rule becomes unreadable, DMLS can no longer establish
which documents it selects. It reports incomplete validation for documents
being checked in the `payments` scope while continuing independent Markdown
checks. Documents in a sibling `shipping` scope remain unaffected by this
automatically discovered rule. If the same directory was explicitly selected
through `SCHEMA_DIR`, its possible scope would instead cover the workspace.
This example does not prescribe trigger syntax or add repository files.

Intentional removal or nonactivation of an optional schema or extension is
distinct from failure to load a required dependency. In the former case,
validate using the baseline and remaining applicable definitions; in the
latter, report incomplete validation for the active definitions that depend on
it. Claudine's embedded runtime validation remains mandatory in either case.

Activation and refresh must remain responsive during editing. The human has
chosen a loose initial performance requirement, without numeric latency or
scale targets. Record representative responsiveness observations during
verification; do not invent a performance budget. This supports qualitative
review rather than a numeric pass/fail threshold. Concrete trigger matching
syntax remains a design choice.

## Compatibility and API Disposition

This repository has no established external users, so remove the obsolete
public API directly rather than deprecating it. Do not retain aliases for
`SubtreeStrictness`, `.strict()`, or `with_strictness(...)`.

The change is intentionally behavior-affecting for callers that depended on
either of these accidental semantics:

- rejecting an absent bare document property;
- resolving a missing bare document property from a same-named `ctx` value.

Both behaviors conflict with the namespace contract and must be removed rather
than preserved behind compatibility flags.

`EvaluationLookup` has a materially larger blast radius than strict mode
itself. If its public contract changes, migrate every implementation in one
coordinated change and retain parity tests for lookup surfaces that do not use
lifecycle globals. Do not force an invasive trait break when a Darkmatter-owned
adapter or additive resolution method can provide the richer binding result
without semantic duplication.

## Technical Design Checkpoint Before Planning

The functional requirements are settled and no functional rulings remain.
Resolve consequential technical choices in a companion `design.md` beside this
specification and review those decisions with the human before creating the
implementation plan. The later `plan.md` organizes agreed work through
sequencing, dependencies, checkpoints, and verification; it is not the place
to resolve outstanding technical rulings. If design work exposes a change to
observable behavior or the agreed scope, bring that decision back to the human
and update this specification before proceeding. Routine implementation
details can remain with the implementer.

The design must record the following technical choices and findings within
the confirmed requirements:

1. Whether the richer result belongs directly on `EvaluationLookup`, on an
   additive resolver method with a compatibility default, or in a composed
   binding adapter above the existing trait.
2. How available eager globals, available lazy globals, unavailable globals,
   and their structured error reasons are represented without losing lazy
   memoization.
3. How Darkmatter's passive preparation validator expresses the confirmed
   all-reference scope check while sharing binding classification with runtime
   evaluation and deferring execution-dependent availability.
4. Which existing literal, escape, whole-value, and mixed-string fixtures
   establish parity after removing Claudine's extra result walkers. Preserve
   the confirmed ordinary Darkmatter semantics without adding evaluation
   passes or weakening genuine error propagation.
5. The lifecycle-global descriptor format and how DMLS obtains it without
   depending on Claudine or duplicating its lifecycle catalog. Schema source
   ownership is settled by R11; generation integration and editor discovery or
   distribution still need concrete designs.
6. Verify the retained event-captured `current.ctx.*` and `current.env.*`
   representation across relevant consumers and keep this fix's catalog and
   documentation consistent with it. This fix introduces no independent
   `current_env`; the future built-in/direct-mirror/freshness migration belongs
   to more-context as clarified above.
7. Handling generated or moved values under `no-shell-expansion`. Existing
   constrained named-type syntax can preserve narrow key/value typing; the
   keyword requires new support. Descendant inheritance and non-relaxation are
   decided; deferred evaluation must retain restrictions.
8. The schema-generation integration and artifact layout, generic activation
   syntax, and refresh mechanisms that satisfy R11 and R12, including passive
   failure recovery and the confirmed limits on discovery scope.

### Risk assessment outcome

Independent review found no warranted separate spike. No prototype or spike
was performed. The existing lookup and schema-loader inventories, coordinated
migration planning, and lasting contract tests are sufficient to address the
identified risks. The implementation plan must cover lifecycle event-time
evaluation, shell preflight, sequence preflight, and a non-Claudine lookup
implementation without expanding the confirmed scope.

### DRY seam audit

Use this design pause to inspect the complete Darkmatter–Claudine expression
boundary for repeated responsibilities that indicate a missing shared
primitive. At minimum, compare:

- effective-state construction from authored, external, overlay, and live
  frontmatter;
- early and late resolution-context construction;
- reserved-global catalogs, availability declarations, eager/lazy value
  providers, precedence, and memoization;
- expression parsing, binding validation, runtime evaluation, whole-value
  typing, mixed-string interpolation, and surviving-span checks;
- the four production `SubtreeCompose` call sites and direct `evaluate` call
  sites;
- prepare-time, shell-preflight, sequence-preflight, and event-time behavior;
- DMLS binding/type descriptors versus runtime descriptors; and
- Darkmatter errors versus Claudine's repeated contextual error wrapping.

Record where the same policy or mechanics are expressed more than once and
decide whether Darkmatter should expose a smaller shared operation, binding
plan, prepared evaluator, or descriptor catalog. Prefer a single prepared
expression/binding object when it can serve static validation and runtime
evaluation without erasing lifecycle context or capture timing.

DRYness is not permission to move Claudine domain policy into Darkmatter or to
create a broad abstraction that merely hides different behavior. Keep a seam
duplicated when the two sides own genuinely different policy, and document why
the duplication is intentional. Any proposed consolidation must identify its
owner, inputs, outputs, error boundary, snapshot timing, and affected callers
at the design checkpoint before planning begins.

## Verification

### Darkmatter Level 1

Add focused tests proving:

- an absent bare property and `doc.<property>` both evaluate to `null` as whole
  values;
- both render empty inside mixed strings;
- ternary conditions select the falsy branch without an error;
- `||` selects its fallback as value semantics, not as a validity escape;
- a schema-declared but unset property and an undeclared property are both
  valid expression lookups; a separate required-property schema violation
  remains a blocking error;
- an explicitly injected global shadows a same-named document property;
- registrations named exactly `doc`, `ctx`, and `env` each fail with a
  structured configuration error before evaluation or lazy-provider invocation;
- `doc.err`, `doc.doc`, `doc.ctx`, and `doc.env` read the corresponding
  document properties while built-in namespaces remain reserved;
- an unavailable reserved global raises a typed error and never falls through
  to a same-named document property;
- an available global whose value is `null` remains distinguishable from an
  unavailable global;
- a missing bare property does not resolve from `ctx.<property>`;
- malformed syntax, unknown functions, rejected arguments, and applicable
  file-operation failures remain errors;
- the public subtree API has no strictness selector;
- all three supported literal escape forms (triple braces,
  `\{{ some_variable }}`, and `\{\{ some_variable }}`), whole-value results,
  and mixed-string interpolation retain ordinary Darkmatter behavior without
  extra evaluation passes;
- definite scope violations fail preparation even in inactive branches;
- preparation invokes no lazy providers and produces no effects;
- failed required schema validation blocks execution, including prohibited
  expansion, rather than producing an advisory and continuing;
- execution-dependent availability is deferred and checked at runtime;
- schema-location feature restrictions survive deferred evaluation, with
  shell expansion rejected for the restricted initialization location;
- `no-shell-expansion` on an array or key/value container reaches every
  descendant and cannot be relaxed by descendant declarations or overrides;
- the initialization schema validates its narrow key/value structure rather
  than accepting any object.

Add contract tests for the binding environment and its ordinary-lookup
adapter. Every `EvaluationLookup` implementation must retain its intended
resolution behavior after the migration; lifecycle-only global semantics must
not leak into unrelated loop, dispatch, catalog, or fixture lookups.

Retain or replace the existing fatality characterization matrix so it tests
actual expression failures rather than root membership.

### DMLS Level 1

Add or update tests proving:

- an undeclared bare property produces at most one advisory diagnostic;
- the diagnostic describes a valid unknown-typed document property;
- a schema-declared property receives schema type information even when unset;
- a runtime-supplied property is not treated as a parser error;
- unknown functions remain distinct hard expression defects;
- with the lifecycle extension active and valid, prohibited shell expansion
  in `initialize` produces a schema feature error;
- with the lifecycle extension active and valid, definitely unavailable
  lifecycle globals produce error diagnostics,
  including references in inactive branches;
- those diagnostics agree with Darkmatter preparation validation and remain
  distinct from undeclared-document-property advisories;
- invalid global registrations named exactly `doc`, `ctx`, and `env` produce
  the shared structured configuration error;
- editor validation executes no actions or shell expansion, causes no file
  side effects, and invokes no lazy providers;
- execution-dependent availability is deferred rather than guessed;
- completion and hover continue to classify bare identifiers as document
  properties rather than context variables;
- the base Darkmatter schema always participates alongside direct always-on
  schema files and conditionally activated schemas;
- generic filesystem activation works at startup and refreshes when relevant
  workspace marker facts change, without compiled-in Claudine schema data;
- composition and editor validation agree on the supplied schemas and feature
  restrictions;
- repository-root and non-repository opened-root `schemas/` directories
  participate, as do monorepo package-area and package directories;
- automatically discovered nested definitions never activate in sibling
  scopes;
- a valid `SCHEMA_DIR` participates alongside ordinary discovery and may point
  outside the workspace or at another package's schema directory;
- a source scoped by automatic discovery can apply in another scope when
  explicitly selected through `SCHEMA_DIR`, but only when its triggers match;
- relative imports from an explicit source retain that source's origin;
- startup and create/update/delete changes refresh applicable schemas,
  including newly created `schemas/` directories at supported roots;
- startup and refresh failures for applicable schemas, triggers, imports, and
  global descriptors identify the failing source and cause, mark validation
  incomplete, suspend dependent checks, and continue independent checks;
- stale dependent diagnostics are not presented as current and stale
  definitions are not used as a fallback; successful refresh restores checks
  and clears the corresponding failure diagnostic without evaluating authored
  expressions or invoking lazy providers;
- hover type information and completion suggestions dependent on failed
  definitions are withheld without a stale fallback, independent assistance
  remains available, and successful refresh restores dependent assistance;
- when an activation-rule failure leaves applicability uncertain, documents
  checked within its possible discovery scope report incomplete validation
  while demonstrably independent checks continue; automatically discovered
  nested rules leave sibling scopes unaffected, while explicit `SCHEMA_DIR`
  rules have a possible scope covering the workspace;
- an absent or inactive optional lifecycle extension leaves baseline and other
  applicable validation available without promising Claudine-specific checks;
  a missing required dependency of an active extension instead reports
  incomplete validation. Neither case disables Claudine runtime policies.

### Claudine Level 1

Cover every event-time shape that previously called
`first_undefined_stack_variable`:

- an absent property in `when:` evaluates falsy and skips the action cleanly;
- an absent property in a lifecycle message renders through ordinary null
  semantics;
- an absent property in a typed action value resolves to `null` without
  causing a lifecycle evaluation error;
- `proxy.with` and mapping-based `set` preserve atomicity with unknown/null
  values;
- an unavailable reserved lifecycle global still fails with its specific
  diagnostic;
- a document property sharing the name of an unavailable lifecycle global is
  reachable only through `doc.*`;
- malformed expressions and unknown functions still halt before side effects.

Add a lifecycle binding matrix covering each reserved global against every
event/scope. Exercise the matrix through Darkmatter's validation and runtime
resolution APIs rather than a Claudine AST walker.

Add a library-only test proving a caller can inspect the original typed
Darkmatter cause and Claudine lifecycle context without invoking the CLI or
parsing rendered error text. Verify that a required schema validation failure
prevents the failed operation from proceeding to lifecycle actions or provider
launch, while preserving existing error reporting and recovery policy; verify
that prohibited shell expansion is never executed, including on recovery
routes. Retain initialization regressions for prohibited
shell actions, bootstrap expansion, and early error routes, including inactive
branches and attempts to override the prohibition.

Update the interpolation conformance matrix: remove strict-versus-lenient
unknown-root divergence and assert shared missing-property semantics instead.
Use the same fixtures for all three literal escape forms, whole-value results,
and mixed strings in Darkmatter and lifecycle consumers to prove parity. Successful expression-looking output
must not be rejected solely for its braces; genuine evaluation errors and
feature restrictions must still fail.

### Claudine CLI Level 1

Add a literal shipped-prompt regression for the reported implementation-router
case. It must prove that the final diagnostic action evaluates all absent
`plan`/`review` references without a lifecycle evaluation error and then reaches
the router's authored `error` action. Assert the output contains the identified
spec line and the intended routing error, and does not contain `unknown root`,
`undefined variable`, or advice to add a fallback.

Use `CliProcessFixture` with child-local `PLAYA_DRY_RUN=1` and its private audio
spool. Assert that no provider starts and no audio is published. Do not gain
focus in terminal or browser windows.

### Package gates

From the affected package areas, run:

```sh
cd darkmatter
just test
just lint

cd ../claudine
just test
just lint
```

Run `just test-l2` only where an existing L2 suite covers the changed
composition boundary or where the implementation adds an L2 regression. Do
not add a new CI matrix cell; this fix changes existing expression semantics
and is covered by the affected packages' existing gates.

## Acceptance Criteria

1. `SubtreeStrictness`, its builder methods, its convenience-function
   argument, strict-root validation, and the lookup root-membership hook no
   longer exist.
2. Every bare identifier other than a reserved namespace or registered global
   resolves as a document property. Caller registrations named exactly `doc`,
   `ctx`, or `env` fail with a structured configuration error before evaluation
   or provider invocation, and DMLS reports that shared validation error.
   Ordinary globals shadow document properties; explicit `doc.err`, `doc.doc`,
   `doc.ctx`, and `doc.env` access remains valid.
3. An absent document property evaluates to `null` and never raises an
   unknown-root or undefined-variable runtime error.
4. Bare document lookup never falls through to `ctx`; context access requires
   `ctx.*`.
5. Claudine has no duplicate runtime or prepare-time closed-world check for
   document-property names.
6. Darkmatter has a first-class binding model that distinguishes a missing
   document property, an available null-valued global, and an unavailable
   reserved global.
7. Claudine declares lifecycle-global availability but performs no custom
   expression-tree traversal to enforce it.
8. Malformed expressions, unknown functions, invalid evaluator operations,
   unavailable reserved lifecycle globals, and prohibited features fail before
   the affected effect. Definite scope violations fail passive preparation,
   including in inactive branches; execution-dependent availability fails at
   runtime. Successful literal and escaped output retains ordinary Darkmatter
   behavior without additional Claudine rejection or evaluation passes.
9. Lifecycle, shell-preflight, and sequence consumers preserve contextual
   diagnostics, atomicity, and shell approval byte parity.
10. DMLS treats an undeclared bare property as a valid unknown-typed document
    property, limits any report to an advisory diagnostic, and consumes shared
    Darkmatter binding descriptors rather than a separate root catalog. It
    also reports schema feature violations and definitely unavailable lifecycle
    globals under applicable schemas and descriptors as errors through the
    same passive Darkmatter validation used in preparation, without guessing
    runtime-dependent availability. An absent or inactive optional lifecycle
    extension leaves baseline and other applicable checks available without
    promising Claudine-specific checks; mandatory runtime policies remain
    independent of editor activation.
11. The reported `prompts/implement.md` route reaches its authored routing error
   without a lifecycle evaluation crash and without changing the prompt to add
   fallback guards.
12. Current documentation and Claudine skill snapshots contain no active
    guidance describing strict mode or requiring fallbacks to legalize optional
    document properties.
13. Consequential technical design decisions, including the binding API, are
    recorded in the companion `design.md` and reviewed with the human before
    `plan.md` organizes implementation. The 17 direct `EvaluationLookup`
    implementations are inventoried with a migration or compatibility
    disposition. Any resulting change to observable behavior or scope is
    explicitly agreed with the human and reflected in this specification.
14. The Darkmatter–Claudine expression seam has a documented DRY audit covering
    state, context, bindings, evaluation, validation, descriptors, and error
    projection; every retained duplication has an explicit ownership reason.
15. Darkmatter and Claudine Level 1 tests and lint pass; applicable existing L2
    suites pass.
16. Library callers can inspect the original typed Darkmatter error and added
    Claudine context independently of CLI presentation.
17. Lifecycle properties use ordinary Darkmatter frontmatter semantics with
    caller-supplied schemas and late-bound globals. Darkmatter owns all
    expression, schema, and result validation.
18. Darkmatter enforces caller-supplied feature restrictions by schema location
    through deferred evaluation. `initialize` forbids shell expansion, and the
    existing initialization shell-action and bootstrap prohibitions remain
    enforced. Its schema uses narrow key/value typing. The
    `no-shell-expansion` constraint applies to every descendant of a restricted
    array or key/value container and cannot be relaxed.
19. Required schema and expression validation failures stop normal execution
    of the failed operation at its validation boundary. Validation itself
    remains free of effects. Existing error reporting and recovery policy is
    preserved, and recovery routes obey the same applicable feature
    restrictions. Missing required properties still fail schema validation
    even though expression lookup of missing properties returns `null`.
20. Authoritative Claudine schemas are editable YAML in `claudine/schemas` and
    are embedded into the Claudine binary through code generation. Darkmatter's
    schema directory moves to `darkmatter/schemas`; affected imports, loaders,
    fixtures, documentation, and skill references are updated. Runtime and DMLS
    schemas derive from the same source, with generation drift checked in the
    existing verification workflow.
21. DMLS always includes the Darkmatter base schema and dynamically consumes
    generic direct schemas and conditional triggers without embedding Claudine
    schemas or depending on Claudine. Startup and filesystem changes activate
    applicable schemas, including generic workspace marker conditions.
    Automatic root, package-area, and package `schemas/` discovery respects
    document-scope isolation. A valid `SCHEMA_DIR` may point anywhere and adds
    workspace-wide definitions subject to their trigger conditions, with
    relative imports retaining their source origin.
    Creation, modification, and deletion refresh the applicable effective
    schema, including newly introduced schema directories. Composition and
    validation use the same Darkmatter schema engine. Representative
    responsiveness observations are recorded without an invented numeric
    threshold.
22. Failed loading or refresh of an applicable schema, trigger, import, or
    global descriptor reports incomplete validation with the failing source
    and cause. Dependent checks are suspended, independent checks continue,
    and stale definitions or dependent diagnostics are not presented as
    current. Successful refresh restores dependent checks and clears the
    corresponding failure. Dependent hover type information and completion
    suggestions are withheld without a stale fallback; independent assistance
    remains available and successful refresh restores dependent assistance.
    When a failed activation rule leaves applicability uncertain, documents
    checked within its possible discovery scope report incomplete validation
    while demonstrably independent checks continue. Automatic nested discovery
    cannot affect sibling scopes; explicit `SCHEMA_DIR` rules have a possible
    scope covering the workspace. Intentional optional-schema removal or
    nonactivation is distinct from a missing required dependency of an active
    definition. Editor recovery remains passive and does not weaken runtime
    validation requirements.

## Non-Goals

- Changing JSON null truthiness, comparison, member-access, indexing, or mixed
  interpolation rules beyond removing the invalid root gate.
- Making malformed expressions, unknown functions, or failed read-side
  operations silently recover.
- Removing schema validation or changing required-property enforcement.
- Treating arbitrary function names as document properties.
- Adding a user-facing strictness switch, compatibility flag, or deprecation
  period.
- Moving lifecycle event policy or lifecycle-global value construction into
  Darkmatter.
- Making Darkmatter depend on Claudine or teaching it the meaning of a
  particular event such as `initialize` or `failure`.
- Reworking loop action JSON re-parsing; only its variable namespace and
  missing-property semantics must remain consistent with the shared evaluator.

## Evidence and Impact

GitNexus was bound to the `better-static-analysis` worktree at commit
`4399b3a4b53553ad3c89e1f4bc1dbec1aa02e6e6`, matching `HEAD` when this
specification was authored. Its upstream walk rated `validate_strict_roots`
LOW: one direct caller (`compose_string`) and three in-module transitive
callers. A later upstream walk over the proposed seam rated
`EvaluationLookup` HIGH: 17 direct implementations, 71 affected symbols, four
modules, and an indexed process. This makes a direct trait contract change a
coordinated migration rather than a local refactor. Symbol walks for
`InjectedGlobal`, `SubtreeCompose`, and the Claudine availability helper were
UNKNOWN because the index does not model all enum, builder, import, and module
re-export references. Text inspection resolved the immediate uncertainty:
four production Claudine `SubtreeCompose` sites, lifecycle global construction,
three lifecycle executor checks, and the separate `err` validator participate,
with additional Darkmatter, Claudine, CLI, DMLS, documentation, and skill tests
encoding the old contract.

This is therefore a coordinated Darkmatter and Claudine change. Darkmatter
owns expression semantics, binding classification, expression traversal,
availability enforcement, and the public evaluation APIs. Claudine owns the
lifecycle-global catalog and per-event availability policy, global values,
integration, orchestration, and contextual error projection.
