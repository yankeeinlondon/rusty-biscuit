---
status: draft
reviewed: false
created: 2026-07-15
inputs:
  - ../../../claudine/features/2026-07-14-type-system/spec.md
  - ../../../claudine/features/2026-07-14-type-system/spec-review.md
  - ../../lib/src/markdown/schemas/mod.rs
  - ../../lib/src/markdown/schemas/simplified/types.rs
  - ../../lib/src/markdown/compose/subtree.rs
  - ../../lib/src/markdown/compose/expression/mod.rs
  - ../../lib/src/markdown/compose/interpolation/rewrite.rs
  - ../../../claudine/lib/src/composition/preflight.rs
  - ../../../claudine/lib/src/composition/lifecycle/executor.rs
  - ../../../claudine/lib/src/composition/lifecycle/validate.rs
---

# Schema-Aware Expression Type System

## Status

Draft for adversarial review. This specification resolves the ownership and
semantic questions raised by the Claudine type-system review. It is not an
implementation plan and does not authorize changes while conflicting
Darkmatter expression or performance work is in flight.

## Summary

Darkmatter owns Markdown schema resolution, expression parsing and evaluation,
interpolation, and strict unknown-root detection. It must therefore own the
schema-to-expression boundary as well.

This feature introduces a value-free variable declaration layer derived from
the effective schema. Expression lookup combines that layer with live runtime
values, standard namespaces, and host-provided ambient variables. A declared
variable can consequently be known to the expression system without having a
runtime value and without being inserted into frontmatter.

The feature also centralizes strict root validation so string interpolation and
parsed expressions use the same rules. Hosts such as Claudine provide values,
ambient declarations, and per-surface strictness policy; they do not reproduce
Darkmatter's schema projection or unknown-root logic.

## Motivating Regression

A prompt accepts either a specification or a review:

```yaml
$schema:
  - spec: file(required;match(**/*spec*.md);eager)
    design: file(match(**/*design*.md))
    plan: file(required;match(**/*plan.md))
    area: string()
  - review: file(required;match(**/*review.md);eager)
    plan: file(required;match(**/*plan.md))
    area: string()

start:
  message: '{{ spec ? "creating a plan for `{{spec}}` specification" : review ? "creating a plan for `{{review}}` review" : "" }}'
```

The review form is valid:

```sh
claudine compose prompts/plan.md \
  review='reviews/2026-07-14-module-assessment/review.md' \
  -y --codex
```

Schema validation selects the review arm, but strict lifecycle interpolation
currently rejects the first condition:

```text
subtree-compose strict mode: unknown root 'spec'
```

The failure occurs because `LayeredLookup::is_known_variable_root` currently
equates membership in the runtime frontmatter map with declaration. The
effective schema is not part of its lookup. Adding `spec` as an optional
property to the review arm still fails when no runtime value is supplied.

There is a second issue in the same example. Mixed-text interpolation already
rescans replacement strings containing `{{ ... }}`, but whole-value
interpolation returns a string result without that rescan. Once the root error
is corrected, the branch can therefore leave the inner `{{spec}}` or
`{{review}}` unresolved. The acceptance scenario requires both paths to have
consistent bounded-rescan behavior.

## Ownership Boundary

### Darkmatter owns

Darkmatter is the sole authority for:

- deriving expression-visible declarations from an `EffectiveSchema`;
- projecting schema property types into expression types;
- joining types across property-level and root-level unions;
- deciding whether an expression root is known to a Darkmatter lookup;
- validating roots under Darkmatter's expression short-circuit semantics;
- evaluating Darkmatter expressions against declared symbols and live values;
- applying interpolation and recursive-rescan semantics.

These responsibilities must be exposed through reusable, presentation-neutral
library APIs. They must not be reimplemented in Claudine, DMLS, or another
consumer.

### Hosts own

A host such as Claudine remains responsible for:

- resolving the effective schema at the correct point in its workflow;
- retaining and passing the resulting variable declarations;
- supplying live frontmatter values;
- declaring host-specific ambient variables and their scope;
- selecting strict or lenient evaluation for each surface;
- enforcing host policy, such as where `err` is permitted or which variables
  are unavailable during shell-command preflight;
- deciding what action follows an evaluation failure.

A host policy may restrict an otherwise known ambient variable. It must not
reclassify a schema-declared root by inspecting only the runtime map.

## Semantic Model

Expression variables have three independent properties:

1. **Visibility** — whether a root is declared in the current environment.
2. **Static type** — the most precise safe type known before evaluation.
3. **Runtime value** — the live value, if any, available during evaluation.

The following invariants are normative:

- A visible variable may have type `any`.
- A visible variable of any type may have no runtime value.
- A missing runtime value does not make a declared variable unknown.
- `any` is a static type, not a value and not an alias for `null`.
- Requiredness is a schema validation constraint, not a variable type.
- Static type information never manufactures a runtime value.
- Runtime frontmatter keys not declared by the schema are visible with type
  `any`.
- A root absent from both declarations and runtime state remains unknown in a
  strict surface.

The established schema convention in which `null` represents an absent
optional value is unchanged. This feature prohibits synthesizing `null`
properties merely to make names visible; it does not change validation or the
existing optional-value sentinel.

## Variable Environment

### Declaration layer

Darkmatter must provide an immutable, cloneable declaration artifact derived
from an `EffectiveSchema`. The exact Rust names may change during planning, but
the public roles are represented here as:

```rust
pub struct VariableDeclarations {
    // literal root name -> declaration
}

pub struct VariableDeclaration {
    pub ty: VariableType,
    pub sources: VariableSources,
}
```

The declaration artifact:

- contains literal top-level roots, not values;
- is safe to retain after schema validation;
- is deterministic and independent of mapping or validator-error order;
- can be shared across preparation, preflight, and event-time evaluation;
- preserves enough source information for diagnostics and inspection without
  making source origin part of type equality.

Schema declarations are one layer of the complete evaluation environment. At
evaluation time, `LayeredLookup` combines:

1. host-injected globals or ambient variables;
2. the standard `ctx`, `env`, and `doc` namespaces;
3. schema-derived declarations;
4. live runtime frontmatter keys and values.

The lookup's precedence for resolving actual values remains unchanged. The new
declaration layer participates in visibility and static typing only. Looking up
a schema-declared root with no live value returns the established no-value
result, which expression evaluation maps to `null`/falsy behavior. It does not
write to `EffectiveState::data()` and does not masquerade as an injected global.

### Public integration seam

Darkmatter must expose APIs equivalent to:

```rust
impl EffectiveSchema {
    pub fn variable_declarations(&self) -> Result<VariableDeclarations, SchemaError>;
}

impl<'a> LayeredLookup<'a> {
    pub fn with_variable_declarations(self, declarations: &'a VariableDeclarations) -> Self;
}

impl<'a> SubtreeCompose<'a> {
    pub fn with_variable_declarations(self, declarations: &'a VariableDeclarations) -> Self;
}
```

Equivalent builder-based APIs are acceptable. A bare `with_known_roots` set is
not sufficient as the primary abstraction because it discards the static type
model and invites a second projection elsewhere. A known-roots convenience API
may exist for hosts to declare untyped ambient variables.

Callers that do not provide declarations retain current behavior. This permits
incremental migration without changing lenient composition globally.

## Static Types

### Normalized expression types

The declaration layer uses a normalized expression-facing type. It does not
expose validation constraints as expression types.

At minimum, the model distinguishes:

- `any`;
- each supported scalar family, including Darkmatter refinements such as
  `file`, `date`, `url`, and `expression`;
- object;
- array of a normalized item type;
- a union of two or more distinct concrete types.

Constraints such as `required`, `generated`, `eager`, `match(...)`, defaults,
descriptions, and cardinality do not affect type identity. Inline object shapes
normalize to object for this feature; nested member typing is a non-goal.

An unresolved imported type must not reach effective-schema projection. If it
does, projection returns a schema error rather than silently omitting its
property.

### Union normalization

Every property-level or root-level union is reduced with one deterministic
normalization operation:

```text
union(T, T)          = T
union(any, T)        = any
union(T, any)        = any
union(T, U)          = T | U, when T != U
union(T | U, V)      = normalize({T, U, V})
```

`any` is the absorbing type because it states that no concrete restriction can
be established. Distinct concrete types do not lose their information merely
because they differ: they remain alternatives in a union.

Normalization must flatten nested unions, remove duplicate members, collapse a
single-member union to that member, and use a stable canonical member ordering.
Equivalent unions therefore have the same representation regardless of arm or
mapping order.

Array shape is part of type identity. An array and a scalar remain distinct
union members, as do arrays with different item types. A union inside an array's
item type is also distinct from a union between whole array types.

Examples:

| Inputs | Result |
|---|---|
| `file`, `file` | `file` |
| `file`, `any` | `any` |
| `string`, `number` | `string | number` |
| `file`, `string` | `file | string` |
| `string[]`, `string[]` | `string[]` |
| `string[]`, `number[]` | `string[] | number[]` |
| `string`, `string[]` | `string | string[]` |
| `any`, `string | number` | `any` |

Schema nullability and optional absence do not contribute a separate `null`
type to the union. They are validation/value-presence concerns.

### Property-level unions

A `PropertyDef::Union` is projected before its containing object or root-union
arm is combined. Each atom becomes a normalized expression type, and the same
union normalization folds the atoms. Thus `[string, number]` projects to
`string | number`, while `[file, file]` projects to `file` and
`[file, any]` projects to `any`.

### Root-level unions

For every literal property name declared by at least one arm:

1. Project that property into every arm.
2. Use the arm's projected property type when the property is declared.
3. Use `any` when that arm has no property-specific declaration.
4. Fold all arm results with union normalization.

For the motivating schema:

| Property | Specification arm | Review arm | Union type |
|---|---|---|---|
| `spec` | `file` | `any` | `any` |
| `design` | `file` | `any` | `any` |
| `review` | `any` | `file` | `any` |
| `plan` | `file` | `file` | `file` |
| `area` | `string` | `string` | `string` |

Arm selection remains relevant to validation, coercion, and runtime values. It
does not remove declarations contributed by other arms.

### Single object shapes

An optional property in a single object shape is declared even when absent at
runtime:

```yaml
$schema:
  spec: file()
```

Here `spec` is visible with type `file`. Runtime absence makes it falsy; it does
not make it an unknown root.

### Open and pattern-based shapes

Open object shapes, `additionalProperties`, and pattern properties do not
create an infinite declaration set. They permit or type keys once those keys
exist, but cannot declare arbitrary bare identifiers in advance.

Therefore:

- a runtime-present `foo` is visible with type `any` when no literal schema
  property gives it a more specific type;
- a literal `foo` property declared in any effective schema arm is visible even
  when runtime-absent;
- a runtime-absent name matched only hypothetically by a pattern remains
  unknown in strict mode.

## Effective-Schema Projection

`EffectiveSchema` is the only schema authority for this feature. Projection
occurs after references, imports, baselines, triggers, and document layers have
been resolved and merged. Consumers must not reparse authored `$schema` YAML or
independently reproduce schema precedence.

Darkmatter may preserve the declaration projection while assembling the
effective schema or derive it from Darkmatter's final normalized schema. The
result must reflect all effective layers, not merely
`EffectiveSchema::simplified`, because that optional field is absent for raw
JSON Schema and does not by itself describe every mixed/merged form.

### SimplifiedSchema

Projection supports:

- inline single shapes;
- referenced schema files after resolution;
- property-level unions;
- root-level unions;
- imported named types after resolution;
- effective baseline, trigger, and document merges.

The expression type projection may reuse Darkmatter's typed
`SimplifiedSchema` representation where it fully represents the effective
shape. It must not ask a host to walk `SchemaArm` or `PropertyDef` itself.

### Raw JSON Schema

Raw JSON Schema is in scope with conservative typing:

- every explicit top-level property reachable through supported object,
  `anyOf`, `oneOf`, or `allOf` shapes becomes visible;
- ordinary scalar, object, array, `const`, and Darkmatter-recognized format
  shapes may project to their corresponding normalized type;
- property alternatives use the same union normalization;
- recognized distinct concrete alternatives remain a concrete union;
- an unrecognized or indeterminate schema alternative contributes `any`, which
  absorbs the other alternatives rather than narrowing them unsafely;
- open/pattern shapes do not manufacture literal root declarations.

This is an owner-maintained projection from Darkmatter's resolved schema, not a
second Claudine JSON Schema parser. Full JSON Schema static inference is not
required. A raw construct whose possible literal property names cannot be
enumerated provides no absent-runtime declarations; keys actually present at
runtime remain visible as `any`.

Equivalent supported inline and referenced schemas must produce the same
declarations. Equivalent SimplifiedSchema and raw JSON Schema forms must expose
the same literal roots; raw forms may conservatively produce `any` where a
SimplifiedSchema form retains a Darkmatter-specific refinement.

## Strict Root Validation

### One Darkmatter validator

Darkmatter must expose one reusable strict-root validation path over a parsed
`Expr` and an `EvaluationLookup`. String interpolation and direct parsed
expression evaluation must use it instead of maintaining separate root walks.

An API equivalent to the following is required:

```rust
pub fn validate_expression_roots(
    expr: &Expr,
    lookup: &impl EvaluationLookup,
) -> Result<(), ExpressionError>;

pub fn evaluate_strict(
    expr: &Expr,
    lookup: &impl EvaluationLookup,
) -> Result<Value, ExpressionError>;
```

The existing lenient `evaluate` entry point remains available. Strictness is a
surface policy, not a property of the shared declarations:

- lenient body/frontmatter surfaces may continue mapping missing values to
  `null`;
- strict lifecycle or side-effect surfaces reject genuinely unknown roots;
- both surfaces consult the same visibility environment.

### Known-root rule

A root is known when any current layer declares it:

- a live runtime frontmatter key;
- a literal property in the effective-schema declarations;
- `ctx`, `env`, or `doc`;
- a host ambient/global declared for the current scope.

Host ambients include Claudine's lifecycle globals where applicable:

- `err`;
- `timing`;
- `current`;
- `_loop_count`;
- `_loop_is_first`;
- `_loop_is_last`;
- `_loop_last_output`;
- `_loop_last_exit_code`.

Their availability remains scope-specific. Listing them here does not make
`err` valid in events where Claudine policy forbids it or loop values valid
outside loop scope.

A strict unknown-root error is valid only when no layer declares the root. A
typo such as `specc` therefore still fails when it is absent from runtime state,
the effective schema, namespaces, and host ambients.

### Short-circuit compatibility

This feature does not change Darkmatter's established tolerance for fallback
and unchosen ternary paths. The shared validator must preserve the same
reachable-root policy currently used by strict subtree composition. Refactoring
the root walker must not broaden or narrow those semantics accidentally.

### Diagnostics

Strict diagnostics must continue to identify the unknown root and authored
expression. When available, diagnostics should distinguish:

- unknown root;
- known root with no runtime value;
- known root rejected by host scope policy.

The second case is not an error by itself.

## Interpolation Consistency

Darkmatter currently rescans replacement strings on the mixed-text path, with
a bounded interpolation depth, but not when a whole-value expression evaluates
to a string. Both paths must use the same rule:

- a whole-value expression yielding a non-string JSON value preserves that
  typed value;
- a whole-value expression yielding a string containing recognized
  `{{ ... }}` spans continues through the normal bounded string-rescan path;
- each generated span is parsed, root-validated, and evaluated with the same
  lookup and strictness as its parent;
- interpolation literals keep their existing escape semantics;
- the existing depth bound prevents unbounded self-expansion;
- exhausting the bound on a strict surface is an error rather than successful
  output containing a live interpolation span.

This is not general evaluation of arbitrary strings as code. Only strings
produced inside an already active interpolation operation and containing the
existing recognized delimiter syntax are rescanned, matching established
mixed-text behavior.

## Host Integration Contract

The declarations must reach every site that evaluates or pre-validates a
Darkmatter expression. For Claudine, this includes at least:

1. strict lifecycle communication/action string interpolation through
   `SubtreeCompose`;
2. parsed `when:` guards and other parsed lifecycle expression surfaces;
3. lifecycle shell-command interpolation during early-binding preflight;
4. any prepare-time unknown-root scan retained for non-lifecycle expression
   surfaces;
5. loop, retry, resume, proxy, sequence, inline, and re-composition paths.

Claudine currently validates with an `EffectiveSchema` and then discards it.
Integration must instead retain the derived declarations—or an equivalent
immutable environment—on `PreparedComposition` and on any rematerialization
inputs that rebuild it. Re-resolving the schema solely for each lifecycle event
is not acceptable.

Schema resolution and schema validation are separate operations. Claudine may
derive declarations after effective-schema resolution and before validation,
so the environment is available to `initialize` even though that event runs
before schema validation.

Preflight and event-time lookup use the same schema declarations but different
ambient scopes. For example, a schema-declared `spec` is known during shell
preflight even when absent at runtime, while `err` remains unavailable because
its value cannot exist before an event failure. The shared environment does not
erase this phase distinction.

Host code that currently determines unknown roots by inspecting only a
frontmatter map must migrate to Darkmatter's validator or query the complete
Darkmatter environment. Claudine may retain scans for Claudine-specific policy,
such as prohibiting `err` in no-error events; it must not retain an independent
definition of schema-visible variables.

## Required Design Properties

### D1 — Value-free declarations

Schema visibility must be represented independently of frontmatter values and
injected globals. No schema-declared-but-absent property is inserted into
authored frontmatter, effective frontmatter, closure output, or
`EffectiveState::data()`.

### D2 — Darkmatter-owned projection

All schema-to-expression projection is implemented and tested in Darkmatter.
Consumers receive a resolved declaration artifact and do not walk authored
schema syntax.

### D3 — Deterministic union normalization

Projection is independent of mapping order, selected validation arm, closest
arm diagnostics, and validator error order. `any` absorbs other types, while
distinct concrete types remain members of a flattened, deduplicated,
canonically ordered union.

### D4 — Shared symbols, per-surface policy

All expression surfaces can consume the same declarations. Strictness and host
ambient scope remain explicit per-surface policies.

### D5 — Effective-schema completeness

Declarations reflect the final schema after supported resolution and merging.
Inline, referenced, baseline, trigger, and document sources cannot silently
disappear from the environment.

### D6 — Multi-site consistency

String interpolation, parsed expression guards, lifecycle preflight, and any
remaining host validation path must agree on whether a root is known.

### D7 — Compatibility

Existing evaluation precedence, runtime function contracts, arm selection,
coercion, optional-null handling, and short-circuit rules remain unchanged
except where this specification explicitly adds declaration visibility or
whole-value string rescanning.

## Non-goals

- Creating a complete flow-sensitive or nested-object type checker.
- Making every name visible because an object schema is open.
- Treating `any` as `null`, undefined, present, or valid for every runtime
  operation.
- Materializing absent schema properties in document state.
- Changing schema validation, arm selection, requiredness, coercion, eager file
  rewriting, or optional-value dropping.
- Changing function runtime contracts such as `file_exists`; this feature only
  allows a valid declared input to reach those contracts.
- Making all lifecycle globals available in all events or during preflight.
- Moving Claudine lifecycle scheduling or side-effect policy into Darkmatter.
- Inferring arbitrary literal roots from `patternProperties`,
  `additionalProperties`, or non-enumerable raw JSON Schema constructs.
- Replacing Darkmatter's validator with the conservative expression-type
  projection.
- Adding or changing SimplifiedSchema authoring syntax solely for this feature.

## Acceptance Criteria

### Darkmatter library

1. `EffectiveSchema` exposes a reusable declaration artifact containing every
   supported literal top-level property in the final effective schema.
2. An optional property in a single shape is a known root when absent at
   runtime and retains its declared type.
3. A root declared by only one root-union arm is known for all invocations of
   that union and projects to `any` in arms where it is undeclared.
4. Property-level unions and root-level unions use the same deterministic
   normalization.
5. `any | T` produces `any`; distinct concrete alternatives such as
   `string | number` remain a union; identical concrete types collapse to that
   type.
6. Runtime-present, schema-untyped keys are known with type `any`.
7. Open or pattern-based object acceptance does not make an arbitrary
   runtime-absent identifier known.
8. Raw JSON Schema object/union forms expose enumerable literal roots without
   requiring a consumer-side schema parser.
9. Strict `SubtreeCompose` accepts a schema-declared root with no runtime value
   and preserves established falsy/no-value evaluation.
10. Strict parsed-expression evaluation makes the same decision as strict
    interpolation for the same lookup and expression.
11. A root absent from all declaration/value/namespace/ambient layers still
    produces an unknown-root error.
12. Neither projection nor evaluation inserts synthetic properties or values
    into frontmatter.
13. A whole-value ternary whose selected branch contains a nested interpolation
    span resolves that span with the same lookup and strictness.
14. Whole-value interpolation continues to preserve non-string JSON values.
15. Strict recursive interpolation fails when the depth bound is exhausted;
    it does not return a live recognized span as successful output.

### Claudine integration

16. The motivating `review=...` command reaches provider execution without an
    unknown-root error for `spec` and renders the review message with the review
    path substituted.
17. The corresponding `spec=...` invocation does not fail on `review` and
    renders the specification message with the specification path substituted.
18. `spec || review`, `file_exists(spec)`, and truthiness tests accept both
    invocation shapes and apply their existing runtime behavior.
19. A typo such as `specc` still fails on strict lifecycle surfaces.
20. The same declarations are used by lifecycle messages, parsed `when:`
    guards, and shell-command preflight.
21. Preflight accepts a schema-declared-but-absent early-binding root but still
    rejects late-binding lifecycle globals unavailable in that phase.
22. Loop ambients remain valid in loop scope and invalid outside their defined
    scope.
23. Direct, inline, sequence, retry, resume, proxy, loop, and re-composition
    paths retain the declarations.
24. No invocation requires `spec=null`, `review=null`, placeholder properties,
    or duplicated union-arm fields.

## Verification Matrix

| Scenario | Declaration type | Runtime result |
|---|---|---|
| Present untyped `foo` | `any` | supplied value is used |
| Optional typed `spec`, present | `file` | supplied value is used |
| Optional typed `spec`, absent | `file` | known root; no-value/falsy semantics |
| `spec` absent from selected arm but present in another arm | `any` | known root; no-value/falsy semantics |
| `file | any` | `any` | runtime operation decides from actual value |
| `string | number` | `string | number` | runtime operation decides from actual value |
| `[string, number]` property union | `string | number` | runtime operation decides from actual value |
| Raw JSON explicit property with indeterminate type | `any` | known root; runtime value if supplied |
| Runtime-absent key matched only by an open/pattern shape | no declaration | strict unknown-root error |
| `specc` absent everywhere | no declaration | strict unknown-root error |
| Whole-value ternary returns `"{{review}}"` | inherited from `review` lookup | nested span resolves within bound |

## Implementation Sequencing Constraint

The API and semantics in this document should be ratified before implementation
planning. When implementation begins, the declaration and shared-validator
work must land in Darkmatter before Claudine migrates its enforcement sites.
Temporary insertion of `null` values or misuse of injected globals is not an
acceptable bridge. Coordination with active Darkmatter expression/performance
work is required to avoid parallel edits to the lookup and interpolation hot
paths.
