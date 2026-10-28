---
kind: feature
created: 2026-10-27
status: draft-spec
area: darkmatter
implemented: true
implemented_by: "claude/sonnet"
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-27
review_iterations: 0
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
related:
    - 2026-11-01-map-and-filter-expression
    - 2026-11-01-map-and-filter-directive
---

# Declarative Narrowing Metadata in SimplifiedSchema

## Objective

Let a function's SimplifiedSchema declaration express what its boolean result
proves about an original argument. Compile that declaration into the existing
`NarrowingFact` machinery used by expression analysis, composition, and DMLS.
Do not introduce a second narrowing engine.

Today `function(parameters(...); returns(boolean))` describes accepted inputs
and the result type, but cannot express that a true result proves an argument
suitable for `file`. That fact lives separately in Rust registration metadata.
This feature makes the catalog the authority for that public typing contract.

This draft selects the syntax and artifact representation below. The semantics
reuse the existing fact kinds. Implementation has not started; the remaining
scope decision is listed under Open Questions.

## Existing Contracts and Scope

The following Darkmatter library surfaces define the behavior this feature
extends:

- [FunctionType](../../lib/src/markdown/schemas/simplified/types.rs) holds the
  parsed function declaration. Add the clause here so catalog readers,
  serializers, and editor tools all see the same contract.
- [Function registration and fact compilation](../../lib/src/markdown/compose/expression/call_binding/registration.rs)
  validate predicate facts and attach them to callable functions. Move the
  declaration's authority into the schema without duplicating these checks.
- [Predicate fact application](../../lib/src/markdown/compose/expression/scope/facts.rs)
  applies facts to stable subjects, using the existing type-scope operations.
  Preserve its conservative behavior when an exclusion cannot be represented.
- [Host artifact loading](../../lib/src/markdown/compose/expression/host/artifact.rs)
  supplies editor-only processes with operational registration data. Keep its
  strict trust checks while deriving narrowing from the catalog.

Function declarations remain description-only: they do not become runtime
function values or JSON Schema instance validators. Conversion of a schema
containing a function to JSON Schema must still fail at the function's property
path. Do not add a JSON Schema extension to carry this clause. Parsing a
function catalog does not register or execute its functions, and catalogs remain
separate from document schemas. These are existing boundaries, not standards
this feature intends to change.

## Proposed Grammar

Add an optional `narrows(...)` clause to `function(...)`:

```yaml
file_exists: |-
    function(
        parameters(value: unknown | null);
        returns(boolean);
        narrows(value: file; true-implies);
        category(Filesystem);
        fallible;
    ) -> Checks whether the value is a valid file reference naming an existing file.

is_file_type: |-
    function(
        parameters(value: unknown | null);
        returns(boolean);
        narrows(value: file; true-implies);
        category(Filesystem);
    ) -> Checks file-reference syntax without resolving the reference or checking existence.
```

The clause names an existing parameter, the proven type, and the implication:

```text
narrows(parameter: type; true-implies)
narrows(parameter: type; true-iff)
narrows(parameter: type; false-implies)
```

Presence-only facts omit the type and lower to the existing `FactType::NonNull`:

```text
narrows(value; false-implies)
```

Here, “present” means the supplied value is not `null`; it does not mean nonempty,
truthy, or that an omitted argument acquired a value. For example, the built-in
`is_empty` declares `narrows(x; false-implies)`: a false result excludes null,
but does not select a concrete type. There is no schema type named `non-null`.

The omitted-type form supports all three implication kinds. With `true-implies`,
only the true branch excludes null. With `true-iff`, the true branch excludes
null and the false branch keeps only null. With `false-implies`, only the false
branch excludes null. These are the existing `NonNull` semantics.

| Kind | True branch | False branch |
| --- | --- | --- |
| `true-implies` | Narrow to the declared type | No new fact |
| `true-iff` | Narrow to the declared type | Exclude that type where representable |
| `false-implies` | No new fact | Narrow to the declared type |

A function may declare at most one narrowing clause initially, matching the
current registration model. The parameter is passed normally; naming it in a
fact does not add a new argument or affect evaluation order.

`narrows` is valid only inside `function(...)`. Like the existing function
clauses, it may appear in any order, uses semicolons between clauses, and allows
the existing trailing semicolon before the function's closing parenthesis.
Inside `narrows`, exactly one parameter identifier, an optional `: type`, and
one `; implication` are required. Both operands are single declarations, not
lists. Serialize it after `returns(...)` with the spellings shown above; an
absent clause stays absent. Parameter names use the existing parameter grammar
and must match the declaration exactly. Aliases inherit the canonical function's
fact rather than declaring another one.

For example, the existing primitive and presence predicates migrate as follows:

```text
is_string: narrows(x: string; true-iff)
is_array:  narrows(x: unknown[]; true-iff)
is_null:   narrows(x: null; true-iff)
is_empty:  narrows(x; false-implies)
```

These lines show only the new clause, not complete schema declarations.

## Semantic Contract

Narrowing describes the original checked argument, not a coerced replacement,
return value, or later reevaluation of the same expression text. Analysis reads
the declaration without invoking the predicate. Runtime behavior must uphold
the declared implication; a catalog declaration does not make an arbitrary
handler truthful. A predicate that converts `"4"` to a number before checking it
cannot declare that the original value is a number merely because its converted
argument is one. Authors must test the original-value implication, retaining any
source exclusions needed by the existing binder.

Facts apply only after a call is accepted by the normal call checker. A binding
failure or a thrown error proves neither branch; `fallible` boolean predicates
may narrow on successful boolean returns only. An optional parameter may be the
subject, but a call omitting that argument produces no fact. Argument defaults,
conversions, and recovery do not create a new subject to narrow.

Both `file_exists(value)` and `is_file_type(value)` declare true-implies `file`.
The former additionally observes existence at evaluation time. A valid path to
a missing file remains a possible file value on its false side, so `file_exists`
must not declare true-iff `file`. Existence does not guarantee future reads will
succeed. Preserve existing resolution and remote-access policy.

For an unknown bound value `candidate`, either successful predicate can establish
file suitability. Facts attach to its binding identity, including the same
binding spelled as a bare local or `local.candidate`. They do not transfer to a
separate `frontmatter(...)` evaluation merely because the call text matches.
No implicit memoization or callback execution is introduced by this metadata.
Preserve existing fact lifetimes across short-circuit operators, negation,
conditional blocks, and lifecycle state changes. Facts refine the current type
scope; they never rewrite the argument, stored frontmatter, or its declared type.
Contradictory facts continue to make a branch impossible rather than replacing
its type with an unrelated one.

For instance, with `source: unknown` in the document schema,
`is_file_type(source) && frontmatter(source, 'title')` lets the call checker accept
`source` as a file argument on the right side. The guard does not establish
existence: the later read can still fail. On a false branch, `true-iff` removes
an exact family from a known union; an unknown value may remain unknown because
the analyzer cannot represent every negative type. A declaration does not
promise a more expressive complement system.

Collection filtering can consume the existing true-side refinements to retain
more precise element types. This spec supplies the fact declaration; the shared
map/filter work owns collection type propagation and callback scopes. It does
not add generic function types or dependent return signatures.

## Validation

Validate through one shared fact compiler used by catalogs, runtime registration,
and DMLS. Preserve authored spans for the clause, parameter, kind, and type.

- Every overload must return a non-null boolean on success; `boolean | null`,
  `unknown`, and truthy non-boolean results do not qualify. `fallible` is allowed.
- The named parameter must exist at the same position in every overload and
  must not be a rest parameter. Check every overload explicitly; omitting one
  cannot be hidden by filtering unsuccessful parameter lookups. The existing
  fact compiler uses such filtering: replace it with an all-overload check as
  part of this change, with a regression where only the later overload omits
  the parameter.
- Overload declarations must agree on their effective fact, including absence.
  Mixed or contradictory facts are rejected rather than selecting the first.
- Reject duplicate clauses, unknown kinds, unknown parameters, malformed types,
  extra operands, and unsupported fact targets with located errors.
- Reuse current target restrictions: a type fact names one supported type, not
  a union. `true-iff` requires null or an exact runtime family with a supported
  complement; refinements such as `file` cannot be used as true-iff targets.
- Resolve and validate target types through the owning type/signature machinery;
  reject unsupported named or constrained targets rather than accepting an
  annotation the analyzer cannot honor. Preserve the established one-way
  targets `file`, `string(min(3))`, and `enum(a, b)`; the latter two must still
  be rejected for `true-iff`. Bare exact-family targets are `string`, `number`,
  `boolean`, `object`, and `unknown[]`, with `null` additionally permitted.
  Do not narrow the existing supported constrained-target behavior to primitives
  alone. Richer named-target support is a separate scope choice under Open
  Questions; do not broaden it incidentally while adding syntax.
- Presence-only clauses remain presence facts. Support the existing valid
  `NonNull` behavior without silently translating it into a concrete type.

An invalid catalog fails passive loading/registration before any handler runs.
Schemas must not execute predicates to validate their declarations. Keep grammar
and declaration checks in schema-owned code so the SimplifiedSchema parser does
not initialize the runtime registry to validate a function: that registry itself
loads the built-in catalog. Runtime registration and editor loading consume the
same validated declaration and existing fact lowering, without a dependency
cycle or a second implementation of validity rules.

A narrowing declaration is part of a function's typing contract, but does not
change which calls bind. Overloads with identical parameter binding contracts
remain duplicates even when their narrowing clauses differ. Preserve existing
parameter-name and annotation insensitivity for duplicate-overload detection;
naming a parameter in `narrows` makes it a checked reference within its own
declaration, not a new overload discriminator. Compare effective facts by
parameter name and position, implication, and canonical parsed target (or
presence-only), not raw type text or source spans.

Located errors must follow the existing schema provenance path through imports,
quoted strings, and YAML block scalars into DMLS. Duplicate clauses point to the
second clause with related information for the first; mismatched overloads name
both declarations. Source-free programmatic declarations still receive the same
typed error, without invented file ranges. Serialization preserves meaning,
not old byte offsets; reparsing serialized text builds new spans.

## Ownership and Migration

SimplifiedSchema becomes the authority for narrowing facts. Runtime handlers
still own their behavior, and Rust registration still owns unrelated operational
metadata such as evaluation mode, source exclusions, and numeric recovery.

1. Extend the function type representation, parser, serializer, and signature
   compilation path with optional narrowing metadata and source provenance.
2. Lower declarations into the existing `FactKind`, `FactType`, and compiled
   `NarrowingFact` representation. Reuse TypeScope and predicate fact consumers.
3. Migrate built-in predicate declarations and registered host catalogs to the
   new clause; remove duplicate Rust-side narrowing declarations. In particular,
   change `file_exists` from string-only narrowing to file suitability.
4. Remove the Rust-side narrowing setter from the public registration builder
   and the private built-in binding declarations after migrating all callers,
   including test-function fixtures. Programmatic users author a function
   declaration containing `narrows`, through schema text or the typed function
   representation. Do not leave a second setter that can introduce a fact absent
   from the catalog. This intentional API change needs no deprecation period in
   this repository.
5. Retain host artifact `format_version: 1` and its required `facts` array as
   derived output. Generate zero entries when the catalog has no clause and
   exactly one when it does; preserve the current underscore kind spellings,
   parameter position, and `<non-null>` encoding. On loading, compare the entire
   decoded fact with the catalog's compiled fact, including absence. Neither
   `facts: []` for a declared fact nor an artifact-only fact is acceptable.
   The artifact must never call a setter to supply or override catalog narrowing.
   Keep deterministic generation and the existing catalog-byte hash check;
   regenerate affected committed companions after changing their catalogs.
6. DMLS derives narrowing from the same validated catalog contract. Do not
   weaken its existing incomplete-analysis behavior when other required host
   registration facts are missing or invalid; catalog narrowing alone does not
   establish those unrelated behaviors. A fact mismatch makes that function
   incomplete, suppresses all its narrowing, and reports the existing
   `dm.expression.incomplete_analysis` diagnostic. Whole-artifact failures still
   affect the whole catalog; per-function failures remain per-function. An
   invalid catalog declaration fails catalog loading under the existing
   `dm.schema.host_catalog` behavior rather than becoming a partially trusted
   function.

**Reader's note:** keeping the derived artifact field avoids changing the wire
format while this feature changes declaration ownership. The new catalog syntax
already requires an updated parser, and the catalog hash invalidates old
companions. Retaining the field makes drift detectable without giving it
independent authority. Removing it would add a format migration without improving
this feature's typing behavior.

The schema/registration compiler must use the same validation for built-ins,
hosts, and passive editor loading. No function-name-specific narrowing branch
is permitted. No new persistent store or parallel function catalog is needed.

## Acceptance Criteria

- Parse and serialize all three implication kinds and presence-only clauses
  without losing semantics; repeated parse/serialize/parse round trips agree.
- Reject each invalid clause/overload case through public schema and registration
  entry points, with precise source ranges and no callback/handler execution.
- Both file predicates narrow an unknown stable subject to `file` on true;
  a valid-but-missing path does not acquire an incorrect false-side exclusion.
- Existing true-iff primitive predicates and presence predicates retain their
  current positive and negative branch behavior after migration.
- Explicit and implicit local reads share facts; frontmatter and shadowed locals
  remain separate subjects. Repeated read-side calls do not gain identity from
  matching text.
- Runtime registration, composition's call checker, and DMLS agree on facts and
  diagnostics. Host artifact drift remains detectable without invoking a host.
- Catalog examples and a normal `md compose` case demonstrate an unknown value
  accepted at a file-typed call after a suitable guard.
- Predicate behavior tests establish that the declared branch satisfies the
  original-value implication for representative inputs, including values that
  could be converted, without inferring the contract from runtime samples.
- Omitted optional arguments, invalid calls, and errors add no fact; aliases,
  negation, compound guards, and scope invalidation retain existing behavior.
- Duplicate binding contracts remain errors despite differing facts; equivalent
  target spellings compare consistently without including source locations.
- Constrained one-way targets retain existing behavior. Unsupported targets fail
  during passive loading, never by resolving a file or executing a predicate.
- A matching version-1 host artifact agrees with runtime registration. Test empty,
  extra, mismatched, malformed, stale-hash, and missing fact data, preserving the
  existing per-function versus whole-catalog failure boundary. A missing artifact
  grants no narrowing even when the catalog declares it.
- A function declaration with `narrows` remains description-only and is rejected
  by document-schema JSON Schema conversion; passive meta-type validation and
  DMLS schema authoring use the same declaration checks and located errors.

Use nextest-backed package recipes and the repository Test Toolkit. These are
L1 grammar, registration, scope, and hermetic composition tests. Use the existing
CLI fixture for a minimal end-to-end invocation, and add a passive shipped
catalog corpus check. Do not add real filesystem/network discovery requirements
or speculative CI cells. Temporary local files for representative existing and
missing paths are appropriate; do not depend on the host's repositories, home
contents, or live remote services. Use handler-entry counters to prove passive
paths do not invoke handlers. No terminal or browser window should gain focus.

This adds parsing and registration-time metadata, not per-element execution or
I/O. No performance spike or new benchmark campaign is required. Preserve the
existing compiled-registration reuse; do not parse or compile the clause on
each predicate call or collection element.

## Documentation and Delivery

Update the SimplifiedSchema function-definition topic, expression guard/narrowing
reference, host-function catalog/registration/artifact documentation, and relevant
Darkmatter skill topics when the behavior lands. Update their examples and the
current `file_exists` string-only claims in the same implementation change.
Explain the original-argument implication and the limits of negative narrowing.

This metadata feature is **not a prerequisite** for
`2026-11-01-map-and-filter-expression` or
`2026-11-01-map-and-filter-directive`. The existing Rust-registered
`NarrowingFact` machinery already supplies the semantics those features need.
They may change `file_exists` to true-implies `file` and consume shared facts
without waiting for schema syntax or host-artifact migration. That uses the
existing authority; it is not a temporary parallel registry.

When this feature lands, migrate those existing registrations into catalog
metadata and preserve their behavior. Coordinate the shared consumers, but do
not add a dependency gate from map/filter to this draft. Syntax and artifact
choices here do not block planning or implementation of map/filter.

## Open Questions

### Should named and recursive narrowing targets be included in this feature?

Function signatures can use named types, but the existing fact compiler parses
its target as a standalone type. Supporting names would require carrying the
owning definition table into fact compilation and application. Without that
context, an accepted name could produce a fact the analyzer cannot honor.

1. **Keep this feature to existing self-contained targets (recommended).** Reject
   named/imported references and targets that require a definition table, with
   a located unsupported-target error. Preserve existing supported primitive,
   constrained, and array targets. **Pros:** retains the already-tested fact
   behavior and keeps this change focused on declaration ownership. **Cons:** a
   host cannot yet express a fact about its own named object type.
2. **Include resolved, nonrecursive named targets.** Resolve the target through
   the catalog's existing type resolver before compiling the fact. **Pros:**
   makes reusable host types available without exposing recursive facts.
   **Cons:** adds definition ownership, canonical equality, and imported-source
   error cases to this feature.
3. **Include recursive targets and retain their definitions.** Carry the owning
   definition table with the fact and reuse reference-aware scope operations.
   **Pros:** supports the full named-type model. **Cons:** requires a broader
   lifetime and compatibility design, with no current built-in predicate needing
   it.

Recommend the first option because every existing built-in fact can migrate
without expanding the analyzer's type model. The implementation scope above
assumes that option; the author should decide before adding richer targets.
This question does not block the separate map/filter features.
