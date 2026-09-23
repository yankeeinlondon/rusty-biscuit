---
created: 2026-09-16
status: draft
for: spec.md
absorbs:
  - ../2026-07-15-type-system/spec.md
  - ../2026-07-22-explicit-null/spec.md
---

# Declarations Design Annex

This annex carries the Phase C/E design detail for the parent
[Expression Type System spec](spec.md): the schema-derived variable
declaration layer, strict root validation, and the host integration
contract. It is re-homed — with dated corrections, not copied wholesale —
from the superseded July draft
([2026-07-15-type-system](../2026-07-15-type-system/spec.md)); that draft
remains in place as the historical record, and this annex — not the draft —
is the design of record. Where this annex and the July draft disagree,
this annex wins. The parent specification's dated confirmed clarifications
also apply here; remaining compatibility implementation work is recorded
in the parent specification. The consolidation itself is recorded as
Resolved Decision 17 in the
chartering spec
([2026-09-15-dasherized-identifiers](../2026-09-15-dasherized-identifiers/spec.md)).

Phase mapping: the declaration layer, static types, effective-schema
projection, strict root validation, interpolation consistency, and the host
integration contract serve **Phase C** (type-aware parsing/evaluation) and
depend on Phase A's `null` vocabulary. The diagnostics distinctions serve
**Phase E**. Acceptance criteria are grouped by phase at the end.

## The Ratified Translation Model

Optionality is a **default constraint** (`optional` unless `required`),
exactly like `max-length: 5` refines `string`. A schema declaring
`string(max-length: 5; required)` gives the variable a runtime type that is
a string constrained to five characters — calling it merely "string" is
incomplete. Likewise a schema declaring `string` — optional by default —
gives the variable the effective runtime type `string | null`; calling it
merely "string" is incomplete in the same way. In general:

> An optional property with declared type A has effective runtime type
> `A | null`. `null` is YAML's representation of undefined.

Considered and rejected: **required-by-default**. It would make
`string(optional)` → `string | null` the more intuitive reading, but
optional-by-default was chosen because far fewer properties tend to be
required — the ergonomics favor the shorter spelling for the common case.

**Reversal, recorded 2026-09-16.** The July draft held that "schema
nullability and optional absence do not contribute a separate `null` type
to the union. They are validation/value-presence concerns," and its
verification matrix accordingly typed an absent optional `spec` as `file`.
That stance is consciously overturned by the ratified semantics above:
optionality contributes `null` to the effective runtime type, exactly as a
constraint refines a scalar family. Everything downstream in this annex —
the union algebra, the projection tables, the verification matrix —
reflects the overturned stance.

Declaration projection and expression evaluation must not synthesize or
replace properties merely to provide names or types. This component boundary
does not remove existing composition-stage schema normalization: eligible
absent optional, no-default top-level properties from document inline or
referenced SimplifiedSchema still become null before coercion. Baseline and
trigger properties, raw JSON Schema, root unions, nested properties,
required properties, and defaulted properties remain excluded. Present
values remain unchanged, repeated schema passes remain idempotent, and
validation-only APIs remain passive. No new insertion is authorized by
this declaration layer.

### Semantic invariants

Expression variables have three independent properties:

1. **Visibility** — whether a root is declared in the current environment.
2. **Static type** — the most precise safe type known before evaluation;
   for an optional declaration this is `A | null`.
3. **Runtime value** — the live value, if any, available during evaluation.

Normative:

- A visible variable may have type `unknown`.
- A visible variable of any type may have no runtime value.
- A missing runtime value does not make a declared variable unknown.
- `unknown` is the union of all types, including `null`; it is not itself a
  value or an alias for `null`. `any` is an equivalent schema alias for
  `unknown`, the canonical expression-facing name.
- `required` is a validation constraint; optionality is a default constraint
  whose type contribution is `| null`. The confirmed `null(required)` and
  `unknown(required)` declarations both accept null and omission; `required`
  must not be treated as universally forbidding them.
- Static type information never manufactures a runtime value — including
  `null`.
- Runtime frontmatter keys not declared by the schema are visible with type
  `unknown`.
- A root absent from both declarations and runtime state remains unknown in
  a strict surface.

## Ownership Seam

### Darkmatter owns

Darkmatter is the sole authority for:

- deriving expression-visible declarations from an `EffectiveSchema`;
- projecting schema property types into expression types, optionality
  included;
- joining types across property-level and root-level unions;
- deciding whether an expression root is known to a Darkmatter lookup;
- validating roots under Darkmatter's expression short-circuit semantics;
- evaluating Darkmatter expressions against declared symbols and live values;
- applying interpolation and recursive-rescan semantics.

These responsibilities must be exposed through reusable,
presentation-neutral library APIs. They must not be reimplemented in
Claudine, DMLS, or another consumer.

### Hosts own

A host such as Claudine remains responsible for:

- resolving the effective schema at the correct point in its workflow;
- retaining and passing the resulting variable declarations;
- supplying live frontmatter values;
- declaring host-specific ambient variables and their scope;
- selecting strict or lenient evaluation for each surface;
- enforcing host policy, such as where `err` is permitted or which
  variables are unavailable during shell-command preflight;
- deciding what action follows an evaluation failure.

A host policy may restrict an otherwise known ambient variable. It must not
reclassify a schema-declared root by inspecting only the runtime map.

## Variable Environment

### Declaration layer

Darkmatter must provide an immutable, cloneable declaration artifact derived
from an `EffectiveSchema`. The exact Rust names may change during planning,
but the public roles are represented here as:

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

The lookup's precedence for resolving actual values remains unchanged. The
declaration layer participates in visibility and static typing only. Looking
up a schema-declared root with no live value returns the established
no-value result, which expression evaluation maps to `null`/falsy behavior —
consistent with the translation model, since `null` is already in the
variable's effective type. It does not write to `EffectiveState::data()` and
does not masquerade as an injected global.

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

Equivalent builder-based APIs are acceptable. A bare `with_known_roots` set
is not sufficient as the primary abstraction because it discards the static
type model and invites a second projection elsewhere. A known-roots
convenience API may exist for hosts to declare untyped ambient variables.

Callers that do not provide declarations retain current behavior. This
permits incremental migration without changing lenient composition
globally.

## Static Types

### Normalized expression types

The declaration layer uses a normalized expression-facing type. It does not
expose validation constraints other than optionality as expression types —
and optionality's contribution is exactly `| null`, per the translation
model.

At minimum, the model distinguishes:

- `unknown`;
- `null` (Phase A vocabulary);
- each supported scalar family, including Darkmatter refinements such as
  `file`, `date`, `url`, and `expression`;
- object;
- array of a normalized item type;
- a union of two or more distinct concrete types.

Constraints such as `required`, `generated`, `eager`, `match(...)`,
defaults, descriptions, and cardinality do not affect type identity beyond
optionality's `| null`. Inline object shapes normalize to object for this
feature; nested member typing is a non-goal.

An unresolved imported type must not reach effective-schema projection. If
it does, projection returns a schema error rather than silently omitting
its property.

### Union normalization

Every property-level or root-level union is reduced with one deterministic
normalization operation:

```text
union(T, T)          = T
union(unknown, T)        = unknown
union(T, unknown)        = unknown
union(null, null)    = null
union(T, null)       = T | null, when T != null and T != unknown
union(T, U)          = T | U, when T != U
union(T | U, V)      = normalize({T, U, V})
```

`unknown` is the absorbing type because it includes all types — it absorbs
`null` like everything else. `null` is otherwise an ordinary union member: it does not absorb, and nothing but
`unknown` absorbs it. Distinct concrete types do not lose their information
merely because they differ: they remain alternatives in a union.

Normalization must flatten nested unions, remove duplicate members,
collapse a single-member union to that member, and use a stable canonical
member ordering. Equivalent unions therefore have the same representation
regardless of arm or mapping order.

Array shape is part of type identity. An array and a scalar remain distinct
union members, as do arrays with different item types. A union inside an
array's item type is also distinct from a union between whole array types.

Examples:

| Inputs | Result |
|---|---|
| `file`, `file` | `file` |
| `file`, `unknown` | `unknown` |
| `string`, `number` | `string \| number` |
| `file`, `string` | `file \| string` |
| `string`, `null` | `string \| null` |
| `file`, `null`, `null` | `file \| null` |
| `string[]`, `string[]` | `string[]` |
| `string[]`, `number[]` | `string[] \| number[]` |
| `string`, `string[]` | `string \| string[]` |
| `unknown`, `string \| number` | `unknown` |
| `unknown`, `null` | `unknown` |

### Optionality in projection

Optionality is applied after a property's declared type — including any
property-level union fold — is projected:

- an `optional` property with projected type A contributes `A | null`;
- a `required` property contributes A unchanged.

`string(max-length: 5)` therefore projects to `string | null` for the same
reason it is a five-constrained string: the default constraint participates
in the effective runtime type, and constraints other than optionality are
deliberately not reified beyond their scalar family.

### Property-level unions

A `PropertyDef::Union` is projected before its containing object or
root-union arm is combined. Each atom becomes a normalized expression type,
and the same union normalization folds the atoms. Thus `[string, number]`
projects to `string | number`, while `[file, file]` projects to `file` and
`[file, unknown]` projects to `unknown`. An optional property declared
`[string, number]` projects to `string | number | null`.

### Root-level unions

For every literal property name declared by at least one arm:

1. Project that property into every arm, optionality applied per arm.
2. Use the arm's projected property type when the property is declared.
3. Use `unknown` when that arm has no property-specific declaration.
4. Fold all arm results with union normalization.

For the motivating schema (see the acceptance scenario below):

| Property | Specification arm | Review arm | Union type |
|---|---|---|---|
| `spec` | `file` (required) | `unknown` (undeclared) | `unknown` |
| `design` | `file \| null` (optional) | `unknown` (undeclared) | `unknown` |
| `review` | `unknown` (undeclared) | `file` (required) | `unknown` |
| `plan` | `file` (required) | `file` (required) | `file` |
| `area` | `string \| null` | `string \| null` | `string \| null` |

More than one arm may match, and validation succeeds without a unique match.
Expressions do not require a unique arm identity. Existing arm-selection
policies for coercion and runtime values are not changed by this ruling.
They do not remove declarations contributed by other arms.

### Single object shapes

An optional property in a single object shape is declared even when absent
at runtime:

```yaml
$schema:
  spec: file()
```

Here `spec` is visible with effective runtime type `file | null`. Runtime
absence makes it falsy; it does not make it an unknown root.

### Open and pattern-based shapes

Open object shapes, `additionalProperties`, and pattern properties do not
create an infinite declaration set. They permit or type keys once those keys
exist, but cannot declare arbitrary bare identifiers in advance.

Therefore:

- a runtime-present `foo` is visible with type `unknown` when no literal schema
  property gives it a more specific type;
- a literal `foo` property declared in any effective schema arm is visible
  even when runtime-absent;
- a runtime-absent name matched only hypothetically by a pattern remains
  unknown in strict mode.

## Effective-Schema Projection

`EffectiveSchema` is the only schema authority for this feature. Projection
occurs after references, imports, baselines, triggers, and document layers
have been resolved and merged. Consumers must not reparse authored
`$schema` YAML or independently reproduce schema precedence.

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
- property alternatives use the same union normalization, with JSON Schema
  `"type": ["string", "null"]`-style nullability folding through the same
  `null` member;
- recognized distinct concrete alternatives remain a concrete union;
- `allOf` applies simultaneous constraints, not alternative union arms;
  where their safe intersection is not representable, conservatively project
  `unknown` while preserving enumerable literal root declarations;
- an unrecognized or indeterminate schema alternative contributes `unknown`,
  which absorbs the other alternatives rather than narrowing them
  unsafely;
- open/pattern shapes do not manufacture literal root declarations.

This is an owner-maintained projection from Darkmatter's resolved schema,
not a second Claudine JSON Schema parser. Full JSON Schema static inference
is not required. A raw construct whose possible literal property names
cannot be enumerated provides no absent-runtime declarations; keys actually
present at runtime remain visible as `unknown`.

Equivalent supported inline and referenced schemas must produce the same
declarations. Equivalent SimplifiedSchema and raw JSON Schema forms must
expose the same literal roots; raw forms may conservatively produce `unknown`
where a SimplifiedSchema form retains a Darkmatter-specific refinement.

**Phase note.** This conservative raw-JSON-Schema projection is Phase C
scope by default. The per-increment plan may consciously defer it to a
later increment; if it is deferred, the deferral and the resulting
declaration gap must be recorded in that increment's plan rather than
silently dropped.

## Strict Root Validation

### One Darkmatter validator

Darkmatter must expose one reusable strict-root validation path over a
parsed `Expr` and an `EvaluationLookup`. String interpolation and direct
parsed expression evaluation must use it instead of maintaining separate
root walks.

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

The existing lenient `evaluate` entry point remains available. Strictness is
a surface policy, not a property of the shared declarations:

- lenient body/frontmatter surfaces may continue mapping missing values to
  `null`;
- strict lifecycle or side-effect surfaces reject genuinely unknown roots;
- both surfaces consult the same visibility environment.

### Known-root rule and layered precedence

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
typo such as `specc` therefore still fails when it is absent from runtime
state, the effective schema, namespaces, and host ambients.

### Short-circuit compatibility

This design does not change Darkmatter's established tolerance for fallback
and unchosen ternary paths. The shared validator must preserve the same
reachable-root policy currently used by strict subtree composition.
Refactoring the root walker must not broaden or narrow those semantics
accidentally. The parent specification separately requires type checking
both conditional branches, including skipped branches. That diagnostic pass
does not evaluate skipped expressions or change this root-visibility policy.

### Diagnostics (Phase E)

Strict diagnostics must continue to identify the unknown root and authored
expression. When available, diagnostics should distinguish:

- unknown root;
- known root with no runtime value;
- known root rejected by host scope policy.

The second case is not an error by itself — under the translation model it
is ordinary `A | null` evaluation.

Type diagnostics must follow the parent specification's confirmed
conditional policy: report invalid calls even in skipped branches, but by
default stop execution only upon reaching an invalid call, before invocation.
The explicit CLI blocking policy below can stop execution earlier.
DMLS performs this analysis without expression execution or target-file
reads. Darkmatter's `file_exists` existence predicate is an explicit
exception: a known argument type outside `string | file | null` produces
a DMLS warning explaining its always-false result, and execution returns
`false` without a type error. Warning behavior for `unknown` inputs is
opt-in and disabled by default, with no execution change. The known-type
warning remains enabled by default. A true `is_file_type` guard establishes
suitability for a file parameter and removes this uncertainty within the
guarded branch, without rewriting stored values or establishing existence
or access permission.

### CLI diagnostic policy boundary

The parent specification's CLI consolidation uses `--deny <id|all>` for
selected blocking diagnostics, `--allow <id|all>` for recovery or explicitly
addressed access, and `--disable <id>` for stopping features. It supersedes
a separate `--deny-warnings` proposal. Specific recovery/diagnostic rules
override broad rules independent of order; equally specific opposing rules
are usage errors detected before composition/rendering effects.

This does not change DMLS's opt-in unknown-input warning policy or silently
enable that warning. `--deny all` blocks success for every reported, enabled
warning or error, including a type error in a skipped branch; informational
notices do not block. A specific allowance overrides the broad denial.
The earlier conditional execution examples describe default behavior
without an applicable CLI denial. No denial executes skipped expressions.

The CLI stops subsequent work and effects when a blocking diagnostic is
known, using available passive analysis before effects when possible. It
neither reorders the pipeline nor rolls back earlier effects. Failure
withholds the composed document from standard output, while an explicitly
selected report format may emit a JSON failure outcome and diagnostics
without document content. Existing document-only `--output json` remains
unchanged. These CLI rules do not replace Claudine's workflow policy.
Migration details and acceptance cases are owned by the parent specification.

### Diagnostic data and passive file checks

The library owns diagnostic codes, severity, messages, source positions, and
relevant expected/actual types, shared by CLI and DMLS. Do not add evaluated
arguments, environment values, or file contents, and do not perform extra
evaluation or I/O for reporting. Existing authored source or messages can
contain sensitive text; no general redaction guarantee is made.

`file_exists` rejects structurally malformed strings before filesystem or
network access using the same passive `biscuit-file` rules as `is_file_type`.
DMLS issues the always-false warning for statically provable malformed
contents, not merely for an ordinary string type. Existing valid-reference
resolution and remote-runtime behavior remains as specified in the parent.

## Interpolation Consistency

Darkmatter currently rescans replacement strings on the mixed-text path,
with a bounded interpolation depth, but not when a whole-value expression
evaluates to a string. Both paths must use the same rule:

- a whole-value expression yielding a non-string JSON value preserves that
  typed value;
- a whole-value expression yielding a string containing recognized
  `{{ ... }}` spans continues through the normal bounded string-rescan path;
- each generated span is parsed, root-validated, and evaluated with the same
  lookup and strictness as its parent;
- interpolation literals keep their existing escape semantics;
- the existing depth bound prevents unbounded self-expansion;
- exhausting the bound on a strict surface is an error rather than
  successful output containing a live interpolation span.

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
Integration must instead retain the derived declarations — or an equivalent
immutable environment — on `PreparedComposition` and on any
rematerialization inputs that rebuild it. Re-resolving the schema solely for
each lifecycle event is not acceptable.

Schema resolution and schema validation are separate operations. Claudine
may derive declarations after effective-schema resolution and before
validation, so the environment is available to `initialize` even though
that event runs before schema validation.

Preflight and event-time lookup use the same schema declarations but
different ambient scopes. For example, a schema-declared `spec` is known
during shell preflight even when absent at runtime, while `err` remains
unavailable because its value cannot exist before an event failure. The
shared environment does not erase this phase distinction.

Host code that currently determines unknown roots by inspecting only a
frontmatter map must migrate to Darkmatter's validator or query the complete
Darkmatter environment. Claudine may retain scans for Claudine-specific
policy, such as prohibiting `err` in no-error events; it must not retain an
independent definition of schema-visible variables.

### Function extensions

Hosts supply function implementations and matching declarations through the
same Darkmatter contract as built-ins. That contract covers parameter and
result types, supported conversions, advisory unsupported-input behavior,
and optional true-result facts. Hosts must honor their declarations;
Darkmatter performs the analysis, and DMLS consumes the declarations
without executing functions. A declared file-suitability fact from a host
predicate must support the same guarded file call as `is_file_type`.
Custom host analysis callbacks are not required or authorized.

### Representation evidence and call presence

The completed [representation prototype](spikes/function-declarations-findings.md)
supports separating call presence and argument order from schema value types
and shared behavior metadata. A required function argument cannot be omitted
merely because its value type is `unknown` or `null`; explicit null remains a
supplied argument checked against that value type. This does not alter
SimplifiedSchema property-omission semantics.

The candidate is a proposed representation, not a validated generic checker.
Its passive probes used an installed binary not proven to match this
worktree; accepted metadata did not prove every nested definition or
behavior. Planning must preserve catalog refinements, including `ip-address`,
and audit variadic minimum counts, same-count overload dispatch, conversions,
and fallback behavior. Exact candidate field names remain implementation
choices. The parent specification records the evidence and completion limits.

## Design Properties

### D1 — Value-free declarations (Phase C)

Schema visibility must be represented independently of frontmatter values
and injected globals. Declaration projection, root validation, type
checking, and expression evaluation must not insert or replace properties
in authored frontmatter, effective frontmatter, closure output, or
`EffectiveState::data()` merely to supply names or types. Existing
composition-stage schema normalization remains unchanged.

### D2 — Darkmatter-owned projection (Phase C)

All schema-to-expression projection is implemented and tested in Darkmatter.
Consumers receive a resolved declaration artifact and do not walk authored
schema syntax.

### D3 — Deterministic union normalization (Phases A and C)

Projection is independent of mapping order, selected validation arm,
closest arm diagnostics, and validator error order. `unknown` absorbs other
types including `null`; distinct concrete types remain members of a
flattened, deduplicated, canonically ordered union. Depends on Phase A's
`null` vocabulary.

### D4 — Shared symbols, per-surface policy (Phases C and E)

All expression surfaces can consume the same declarations. Strictness and
host ambient scope remain explicit per-surface policies.

### D5 — Effective-schema completeness (Phase C)

Declarations reflect the final schema after supported resolution and
merging. Inline, referenced, baseline, trigger, and document sources cannot
silently disappear from the environment.

### D6 — Multi-site consistency (Phase C)

String interpolation, parsed expression guards, lifecycle preflight, and any
remaining host validation path must agree on whether a root is known.

### D7 — Compatibility (cross-phase)

Existing evaluation precedence, runtime function contracts, arm selection,
coercion, eager file rewriting, optional-value dropping, and short-circuit
rules remain unchanged except where this design explicitly adds declaration
visibility or whole-value string rescanning, or the parent specification's
confirmed conditional and file-predicate contracts. The type-vocabulary changes
(`unknown`, `null`) are owned by the parent spec's Phases A and C, not by
this annex.

## Non-Goals

- Creating a complete flow-sensitive or nested-object type checker (parent
  Phase D owns the narrowing that is in scope).
- Making every name visible because an object schema is open.
- Treating `unknown` as `null`, undefined, present, or valid for every runtime
  operation.
- Inserting or replacing document properties through the declaration layer
  merely to provide names or types; existing schema-stage normalization
  remains in scope only for compatibility verification.
- Changing schema validation, arm selection, coercion, eager file rewriting,
  or optional-value dropping beyond the parent specification's confirmed
  new-type behavior. The `null(required)` and `unknown(required)` cases are
  explicit requirements, not contradictions to reject.
- Changing function runtime contracts beyond the parent specification's
  confirmed file-predicate behavior. `is_file_type` checks structure without
  resolution; `file_exists` accepts null and retains advisory false results
  for known unsupported types.
- Making all lifecycle globals available in all events or during preflight.
- Moving Claudine lifecycle scheduling or side-effect policy into Darkmatter.
- Inferring arbitrary literal roots from `patternProperties`,
  `additionalProperties`, or non-enumerable raw JSON Schema constructs.
- Replacing Darkmatter's validator with the conservative expression-type
  projection.
- Adding or changing SimplifiedSchema authoring syntax solely for this
  feature.

## Acceptance Criteria

Phase A criteria — `unknown`/`any`, `null`, their `required` behavior, and
inclusive/exclusive unions — live in the parent spec's confirmed
clarifications. The original null-type cases were absorbed from the superseded
[2026-07-22-explicit-null](../2026-07-22-explicit-null/spec.md) draft.

### Phase C — Darkmatter library

1. `EffectiveSchema` exposes a reusable declaration artifact containing
   every supported literal top-level property in the final effective
   schema.
2. An optional property in a single shape is a known root when absent at
   runtime; its effective runtime type is `A | null` for declared type A.
3. A root declared by only one root-union arm is known for all invocations
   of that union and projects to `unknown` in arms where it is undeclared.
4. Property-level unions and root-level unions use the same deterministic
   normalization, with optionality applied after the declared-type fold.
5. `unknown | T` produces `unknown`; distinct concrete alternatives such as
   `string | number` remain a union; identical concrete types collapse to
   that type; `T | null` is an ordinary two-member union.
6. Runtime-present, schema-untyped keys are known with type `unknown`.
7. Open or pattern-based object acceptance does not make an arbitrary
   runtime-absent identifier known.
8. Raw JSON Schema object/union forms expose enumerable literal roots
   without requiring a consumer-side schema parser (subject to the
   projection section's deferral note).
9. Strict `SubtreeCompose` accepts a schema-declared root with no runtime
   value and preserves established falsy/no-value evaluation.
10. Strict parsed-expression evaluation makes the same decision as strict
    interpolation for the same lookup and expression.
11. A root absent from all declaration/value/namespace/ambient layers still
    produces an unknown-root error.
12. Declaration projection, root validation, type checking, and expression
    evaluation preserve input property membership and values when supplying
    names or types. Existing schema-stage null insertion retains its
    eligibility, exclusions, idempotence, and preservation of present values;
    validation-only APIs remain non-mutating.
13. A whole-value ternary whose selected branch contains a nested
    interpolation span resolves that span with the same lookup and
    strictness.
14. Whole-value interpolation continues to preserve non-string JSON values.
15. Strict recursive interpolation fails when the depth bound is exhausted;
    it does not return a live recognized span as successful output.

### Phase C — Claudine integration

16. The motivating `review=...` command reaches provider execution without
    an unknown-root error for `spec` and renders the review message with the
    review path substituted.
17. The corresponding `spec=...` invocation does not fail on `review` and
    renders the specification message with the specification path
    substituted.
18. `spec || review`, `file_exists(spec)`, and truthiness tests accept both
    invocation shapes and apply their existing runtime behavior.
19. A typo such as `specc` still fails on strict lifecycle surfaces.
20. The same declarations are used by lifecycle messages, parsed `when:`
    guards, and shell-command preflight.
21. Preflight accepts a schema-declared-but-absent early-binding root but
    still rejects late-binding lifecycle globals unavailable in that phase.
22. Loop ambients remain valid in loop scope and invalid outside their
    defined scope.
23. Direct, inline, sequence, retry, resume, proxy, loop, and
    re-composition paths retain the declarations.
24. No invocation requires `spec=null`, `review=null`, placeholder
    properties, or duplicated union-arm fields. (After Phase A an author may
    *choose* the explicit `null` XOR idiom; invocation never *requires*
    placeholders.)

### Phase E — diagnostics

25. Strict diagnostics distinguish an unknown root, a known root with no
    runtime value, and a known root rejected by host scope policy. The
    second case is not an error by itself.
26. Both conditional branches receive type checking, with diagnostics for
    invalid calls. A skipped invalid
    call does not block a valid selected branch; a reached invalid call
    fails before invocation, except for explicitly advisory contracts such
    as `file_exists` on known unsupported types.
27. DMLS produces these diagnostics without executing functions or reading
    target files. The parent specification supplies concrete conditional
    and file-predicate acceptance cases.

### Shared function contract and guarded calls

- A true `is_file_type` result permits a guarded file-parameter call for an
  originally `unknown` or `string` argument without mutating its stored
  value or claiming existence or access permission.
- A host predicate with an equivalent declared true-result fact receives
  the same analysis. DMLS uses declarations without invoking host functions.
- `file_exists` on an unknown-typed argument produces no uncertainty warning
  by default and does so when opted in; execution is unchanged. Its warning
  for a known unsupported type remains enabled by default.

### The motivating regression (acceptance scenario)

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

There is a second issue in the same example. Mixed-text interpolation
already rescans replacement strings containing `{{ ... }}`, but whole-value
interpolation returns a string result without that rescan. Once the root
error is corrected, the branch can therefore leave the inner `{{spec}}` or
`{{review}}` unresolved. The acceptance scenario requires both paths to have
consistent bounded-rescan behavior.

## Verification Matrix

| Scenario | Declaration type | Runtime result |
|---|---|---|
| Present untyped `foo` | `unknown` | supplied value is used |
| Optional typed `spec`, present | `file \| null` | supplied value is used |
| Optional typed `spec`, absent | `file \| null` | known root; no-value/falsy semantics |
| `spec` absent from selected arm but present in another arm | `unknown` | known root; no-value/falsy semantics |
| `file \| unknown` | `unknown` | runtime operation decides from actual value |
| `string \| number` | `string \| number` | runtime operation decides from actual value |
| `[string, number]` property union (required) | `string \| number` | runtime operation decides from actual value |
| `[string, number]` property union (optional) | `string \| number \| null` | runtime operation decides from actual value |
| Raw JSON explicit property with indeterminate type | `unknown` | known root; runtime value if supplied |
| Runtime-absent key matched only by an open/pattern shape | no declaration | strict unknown-root error |
| `specc` absent everywhere | no declaration | strict unknown-root error |
| Whole-value ternary returns `"{{review}}"` | inherited from `review` lookup | nested span resolves within bound |

## Completion Evidence

The parent specification's phase-completion and performance criteria apply
to this annex. Verify before/after composition and editor-analysis behavior
on representative documents and controlled declaration, union, and depth
growth, preserving existing resource bounds. Record and explain regressions
for explicit review; no numeric threshold or new CI job is implied.

The conservative raw JSON Schema projection includes simultaneous `allOf`
constraints, falling back to `unknown` when a safe intersection cannot be
represented. A documented increment deferral does not mark the full charter
complete. Grammar/schema changes require passive shipped-artifact and normal
invocation coverage, and correctness covers macOS, Linux, native Windows,
and WSL2. Terminal/browser verification must not take window focus.

## Sequencing Constraints

The inherited design in this annex is ratified, with subsequent confirmed
clarifications applied above. Clarification and risk review are complete;
the parent specification records the remaining implementation-planning work. The declaration layer's
non-mutation boundary preserves existing schema-stage normalization.
Per-increment planning happens when Phase C is scheduled. When
implementation begins:

- Phase A must land first — the `| null` projection has no vocabulary to
  draw on until the `null` type exists.
- The declaration and shared-validator work must land in Darkmatter before
  Claudine migrates its enforcement sites.
- Temporary insertion of `null` values or misuse of injected globals is not
  an acceptable bridge.
- Coordination with active Darkmatter expression/performance work is
  required to avoid parallel edits to the lookup and interpolation hot
  paths.
