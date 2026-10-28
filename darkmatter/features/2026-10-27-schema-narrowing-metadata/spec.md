---
kind: feature
created: 2026-10-27
status: draft-spec
area: darkmatter
implemented: false
reviewed: false
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

This is a quick draft. The syntax below is proposed for review; the semantics
reuse the existing fact kinds. Implementation has not started.

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

This expresses a predicate whose false result proves the argument present. It
does not introduce a schema type named `non-null` or pretend that presence
proves suitability for a concrete type.

| Kind | True branch | False branch |
| --- | --- | --- |
| `true-implies` | Narrow to the declared type | No new fact |
| `true-iff` | Narrow to the declared type | Exclude that type |
| `false-implies` | No new fact | Narrow to the declared type |

A function may declare at most one narrowing clause initially, matching the
current registration model. The parameter is passed normally; naming it in a
fact does not add a new argument or affect evaluation order.

## Semantic Contract

Narrowing describes the original checked argument, not a coerced replacement,
return value, or later reevaluation of the same expression text. Analysis reads
the declaration without invoking the predicate. Runtime behavior must uphold
the declared implication; a catalog declaration does not make an arbitrary
handler truthful.

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

Collection filtering can consume the existing true-side refinements to retain
more precise element types. This spec supplies the fact declaration; the shared
map/filter work owns collection type propagation and callback scopes. It does
not add generic function types or dependent return signatures.

## Validation

Validate through one shared fact compiler used by catalogs, runtime registration,
and DMLS. Preserve authored spans for the clause, parameter, kind, and type.

- Only functions returning boolean in every applicable overload may narrow.
- The named parameter must exist at the same position in every overload and
  must not be a rest parameter. Check every overload explicitly; omitting one
  cannot be hidden by filtering unsuccessful parameter lookups.
- Overload declarations must agree on their effective fact, including absence.
  Mixed or contradictory facts are rejected rather than selecting the first.
- Reject duplicate clauses, unknown kinds, unknown parameters, malformed types,
  extra operands, and unsupported fact targets with located errors.
- Reuse current target restrictions: a type fact names one supported type, not
  a union. `true-iff` requires null or an exact runtime family with a supported
  complement; refinements such as `file` cannot be used as true-iff targets.
- Resolve and validate target types through the owning type/signature machinery;
  reject unsupported named or constrained targets rather than accepting an
  annotation the analyzer cannot honor. Do not broaden existing fact semantics
  incidentally while adding their syntax.
- Presence-only clauses remain presence facts. Support the existing valid
  `NonNull` behavior without silently translating it into a concrete type.

An invalid catalog fails passive loading/registration before any handler runs.
Schemas must not execute predicates to validate their declarations.

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
4. Remove or constrain the old registration setter so it cannot override a
   conflicting catalog fact. A temporary consistency check may require exact
   agreement during migration, but there must be one final authority, not a
   documented precedence contest between catalog and Rust metadata.
5. Preserve generated host fact artifacts for their remaining behavioral data
   and existing trust checks. If narrowing is retained as a derived artifact
   field, require agreement with the catalog and reject drift. Alternatively,
   remove the redundant field through an explicit format-version migration.
   Select and document that representation before implementing artifact changes.
6. DMLS derives narrowing from the same validated catalog contract. Do not
   weaken its existing incomplete-analysis behavior when other required host
   registration facts are missing or invalid; catalog narrowing alone does not
   establish those unrelated behaviors.

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
- Predicate behavior tests establish that successful results satisfy the fact
  for representative inputs without inferring the contract from runtime samples.

Use nextest-backed package recipes and the repository Test Toolkit. These are
L1 grammar, registration, scope, and hermetic composition tests. Use the existing
CLI fixture for a minimal end-to-end invocation, and add a passive shipped
catalog corpus check. Do not add real filesystem/network discovery requirements
or speculative CI cells. No terminal or browser window should gain focus.

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

## Draft Decisions to Finalize

- Approve the `narrows(parameter: type; implication)` spelling and the omitted-type
  presence form, or choose an equally explicit mapping to the existing facts.
- Choose whether generated host artifacts retain a checked, derived narrowing
  field or remove it under a new artifact version. Neither choice changes the
  catalog's authority or relaxes remaining host-analysis trust requirements.
