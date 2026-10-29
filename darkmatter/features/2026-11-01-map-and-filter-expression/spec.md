---
kind: feature
created: 2026-10-20
status: finalized-spec
area: darkmatter
clarified: false
reviewed: false
implemented: true
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
    - 2026-11-01-map-and-filter-directive
    - 2026-10-28-schema-enhancements
    - 2026-10-31-type-aware-evaluation
    - 2026-10-16-type-narrowing
implemented_by: "codex/gpt-6.1-sol"
---

# Map and Filter

Darkmatter expressions can read a list but cannot do anything _per item_ of
one. There is no `map`, no `filter`, and no way to write the per-item
expression they need. This spec adds both functions and the inline function
(lambda) argument that drives them.

## Motivation

A motivating use case is checking a specification's `depends-on` list before
starting work. An expression could select dependencies whose specification
does not declare `implemented: true`, then use that list in a lifecycle
condition and an error message. Today, an expression can neither iterate over
a list of arbitrary length nor carry the per-item expression this requires.

This is an illustrative lifecycle rule, not a claim that a dependency gate
or a supporting shell script currently exists in the implementation prompt.
Whether production prompts should enforce implementation completion, review
completion, or another dependency policy is outside this feature's scope.

The same gap shows up wherever frontmatter holds a list: picking the open
items of a checklist, the packages of a plan that touch one area, the paths of
a glob that still exist.

## What the Author Writes

```yaml
# every dependency whose spec is not implemented
unmet: "{{ filter(depends-on, dep => !frontmatter(first(find_files('**/' + dep + '/spec.md')), 'implemented')) }}"

# the spec paths of every dependency
dep_specs: "{{ map(depends-on, dep => first(find_files('**/' + dep + '/spec.md'))) }}"
```

```yaml
initialize:
    - when: "unmet"
      then:
          - error: "cannot start: {{ as_csv(unmet) }} not implemented"
```

An empty list is falsy (see Truthiness in the expressions topic), so `unmet`
works directly as a condition and no `length(...) > 0` is needed.

### Notes on the motivating example

The example in the request is the intent; four details change when it is
written in the grammar this spec defines:

1. **`find_file` does not exist, and this spec does not add it.** `find_files`
   returns a list; the example above uses `first(find_files(...))`. A
   single-result function may be added later, and if it is, it will be named
   `find_first_file`.
2. **`cond ? false : true` is `!cond`.** Both are valid; the negation is
   shorter and does not invert the reader's sense of the branch.
3. **`filter` returns a list, not a boolean.** `has-unmet-deps` would hold the
   unmet entries. That is more useful (the error can name them) and is still
   usable as a condition through truthiness.
4. **Implementation completion and review completion are different policies.**
   Checking `implemented: true` does not establish that the review cycle has
   closed. An illustrative check for a dependency under `_completed` is
   `filter(depends-on, dep => !first(find_files('**/_completed/' + dep + '/spec.md')))`.
   Which rule the prompt uses is a prompt decision, not part of this spec.

`depends-on` already reads as one identifier: a dash followed by an identifier
character continues a name, so `depends-on` is the property and `a - b` (with
spaces) is subtraction.

**Dependency boundary:** `2026-10-27-schema-narrowing-metadata` is a separate,
non-blocking metadata migration. Implement this feature against the existing
registered `NarrowingFact` API, including true-implies `file` for both file
predicates. Schema-authored narrowing syntax and host-artifact changes are not
prerequisites and do not require a temporary or duplicate fact registry.

## Design

### Relationship to the directives

This expression form and `2026-11-01-map-and-filter-directive` share callback
syntax, binding patterns, lexical scopes, iterable adaptation, type propagation,
truthiness, order, error provenance, and iteration accounting. Build these once.

Intentional differences remain:

- Expressions return arrays; directives render repeated Markdown bodies.
- Both expression functions require two arguments: an iterable expression and
  a callback. Directive-only omitted-callback defaults do not apply.
- The iterable argument may be any expression, including a literal or function
  result. Unlike a directive source, it does not require a named, declared
  collection. Known types are checked statically; uncertain values are checked
  by the shared binder at runtime.
- Block gating, sorting, stopping, and Markdown boundaries belong to directives.
- An expression callback binds its input only. It does not rebind its result
  to the parameter pattern; that second binding is a directive-body operation.

### The lambda argument

A lambda is written `pattern => body` or `pattern -> body`. Both arrows have
identical semantics and bind more loosely than all existing expression
operators: `x -> a ? b : c` means `x -> (a ? b : c)`.

A pattern is one identifier or a shallow object destructuring pattern, such as
`{key,value}`. Destructuring still consumes one item; it is not a multi-argument
callback. The body may contain any existing expression, including nested
`map`/`filter` calls. Object-expression shorthand `{key,value}` means
`{key: key, value: value}`. Nested patterns, defaults, and rest bindings are
outside the initial scope. Patterns must be nonempty with distinct identifier
fields; aliases, quoted/computed keys, array patterns, and trailing commas are
not supported. Extra input fields are ignored. Every bound name must satisfy
the protected-name rules.

Both arrows, pattern binding, and callback-body parsing use shared expression
machinery. Quoted arrow text is not syntax, and commas inside a pattern do not
terminate the enclosing call argument. Preserve pattern, field, arrow, body,
and whole-callback spans through spanned and erased AST forms.

**A callback is syntax, never a value.** In expressions it is allowed only in
an argument position declared as a function parameter by the catalog. Directive
callback positions are a second explicit consumer of the same representation.
Top-level callback interpolation, callback values in arrays/objects, named or
returned callbacks, and callbacks used as operator operands are rejected with
located diagnostics. Parsing establishes the callback syntax; passive signature
validation establishes whether the receiving argument position accepts it.

This does not add closures or callable values to `serde_json::Value`.

### Scope and the local namespace

The iterable argument is evaluated in the enclosing scope before the callback
pattern is bound. Its bound names belong to the reserved `local` namespace:

| Read | Meaning inside a callback |
| --- | --- |
| `name` | Nearest local binding, otherwise normal lookup |
| `local.name` | Nearest local binding only |
| `fm.name` | Frontmatter property even when shadowed by a local |

Locals may shadow frontmatter and enclosing locals without a shadow advisory.
A present null binding still shadows; it never falls through to frontmatter.
Outer unshadowed locals remain accessible. Reserved namespace/global names
cannot be bound, using the site's actual binding view rather than a hardcoded
list. This includes `local` itself and unavailable registered host globals.

Entering a scope must isolate facts about a same-named document property or
outer local. Leaving it, including on failure, restores the enclosing scope.
Locals never mutate frontmatter or leak into siblings. Explicit and implicit
local reads share binding identity and refinement facts; `fm.name` remains a
separate subject. The shared static read inventory must recognize both forms
and never misreport them as undeclared document properties or host-global reads.

### Iterables, order, and destructuring

| Iterable argument | Runtime behavior |
| --- | --- |
| Array or tuple | Visit elements in source order |
| Object | Visit `{key: string, value: any}` entries in insertion order |
| Null, including an absent property | Return `[]` without invoking the callback |
| Empty collection | Return `[]` without invoking the callback |
| Non-null scalar | Shared binding type error naming the function and received type |

Both object-entry fields exist, including when the value is null. Preserve
more precise declared value types: a dictionary of `Person` supplies
`value: Person`; heterogeneous objects supply the union of property value
types. Arrays and tuples retain known element types. Null elements remain
actual items, unlike a null source.

An input must satisfy its parameter pattern before the callback runs. A present
null field is distinct from a missing field. Reject provably incompatible
patterns statically; validate uncertain shapes at runtime. Expressions do not
require the callback result to satisfy that input pattern.

Both functions always return arrays. Filtering an object returns retained
entry objects, not a reconstructed dictionary. Insertion order must survive
loading, conversion, and composition on every supported OS.

### `map(iterable, pattern => body)`

Evaluate the body once per item, in order, and return its results as an array.
Retain null results as array elements.

```text
map([1, 2, 3], n => n * 2)                  → [2, 4, 6]
map(["a", "b"], s -> upper(s))              → ["A", "B"]
map({Ada: 90, Grace: 70}, {key,value} => key) → ["Ada", "Grace"]
map([], n => n * 2)                         → []
map(null, n -> n * 2)                       → []
```

`{key,value} => key` is valid here because there is no directive body requiring
rebinding the resulting string to `{key,value}`.

### `filter(iterable, pattern => body)`

Retain the original items whose callback result is truthy, in source order.
The result is never an array of predicate results. Use existing truthiness:
null, false, zero, an empty string, an empty array, and an empty object are
falsey. Other values are truthy.

```text
filter([1, 2, 3, 4], n -> n % 2 == 0)        → [2, 4]
filter(["a", "", "b"], s => s)              → ["a", "b"]
filter({Ada: 90, Grace: 70}, {key,value} => value >= 80)
                                            → [{key: "Ada", value: 90}]
filter([], n => true)                        → []
filter(null, n -> true)                      → []
```

A truthy/falsey result does not stop iteration. Every item is tested unless an
error or shared budget exhaustion stops the call.

### Type propagation and passive validation

Reuse the existing `TypeScope`, call checker, and narrowing infrastructure.
A callback local receives the iterable's known item type, projected through
its pattern. Never take the type of a same-named frontmatter property and never
erase available item information merely because the catalog's signature is broad.

- For `people: Person[]`, `person` in either function has type `Person`.
- `map(people, person => person.name)` has an array element type derived from
  the declared `name` property.
- `filter(people, person => person.active)` preserves the person element type;
  apply existing predicate refinements where the shared engine can prove them.
- Tuple and union sources combine possible item types. Optional/null source
  arms contribute no items, while null element/result arms remain represented.
- Unknown input or result shapes remain unknown where analysis cannot establish
  more; open-object entries not covered by declared properties remain unknown.
  Do not invent a separate general-purpose inference engine.

All callback bodies receive passive syntax, signature, pattern, and call checks,
including for empty/null input and dead branches. A category 5 call such as
`map([], x => min(true, 5))` rejects before execution. Passive checking and DMLS
perform no callback I/O or provider evaluation. Guard facts about a local end
with its scope and do not narrow an unrelated document property.

### File predicate narrowing

Both `is_file_type(value)` and `file_exists(value)` narrow the checked value to
`file` on a truthy result. The former checks file-reference syntax; the latter
also checks that the referenced file exists. Existence is an observation at the
time of that check, not a guarantee that a later read cannot fail. Preserve the
existing resolver and remote-access policy rather than adding another path grammar.

An unknown value does not satisfy a required file type by itself. Either guard
can establish that suitability for a bound value. A false `file_exists` result
proves no negative file-type fact: a valid reference may name a missing file.
Use the shared predicate registration's true-implies `file` fact for both
predicates; `file_exists` must no longer project only `string`.

Facts apply to the value actually checked. They do not transfer to a separately
reevaluated read-side call merely because its expression text is identical.
For example, computing candidates once and filtering their bindings preserves
that identity:

```text
filter(map(manifests, t => frontmatter(t, 'include')),
       candidate => file_exists(candidate))
```

The retained candidates have file suitability through the shared refinement
engine. Do not introduce implicit cross-callback memoization to make repeated
`frontmatter(...)` reads appear to be the same value. Composition preflight may
evaluate discovery-supported expressions to enumerate typed collections and
identify child targets. It inspects candidate children before collection
conditions, selection, or stopping can exclude them, including recursively
through explicit local parameters. Schema validation and DMLS remain passive.
Targets dependent on pending shell output or other execution results discovery
cannot faithfully produce still fail preflight. Retained graph metadata must
distinguish iteration scopes; newly encountered runtime targets fail without
late approval. Graph reuse does not imply arbitrary callback-result caching.

### Function catalog and registration

The catalog remains `darkmatter/schemas/partials/functions.yaml`, loaded by
`compose/expression/catalog/mod.rs`, under the shared SimplifiedSchema contract.
Both functions have two arguments: `unknown[] | object | null` and a one-item
callback with broad input/output compatibility, including explicit null arms.
Runtime tuples are arrays with tuple schema information. Results are arrays;
the catalog's broad result contract must admit null elements/results as well.

The callback signature's broad fallback is not the call site's inferred type.
Implement shared iterable-item and callback-result type projection so the call
checker, composition, and DMLS retain known types. Pattern arity is one input
item regardless of how many names it binds. Use the existing binder for invalid
source kinds; do not reintroduce per-handler argument validators.

Extend executable registration for callback parameters without permitting
function-valued returns or stored callable values. A callback evaluation mode
must bypass ordinary eager value-argument evaluation while retaining registry
ownership of dispatch and signatures. `map` and `filter` are not lifecycle
verbs: YAML verb operands cannot carry callback syntax. Keep the existing
catalog-derived lifecycle inventories' exclusion for callback-taking functions.

Catalog `example(call => result)` parsing must distinguish both callback arrows
inside a call from the top-level example separator. Add executable examples for
both arrows, objects/destructuring, empty/null input, and nested callbacks.

### Shared callback and lexical-frame implementation contracts

Use one callback node with a reusable binding pattern, not `param: String`:

```text
BindingPattern = Identifier(name) | Object(fields)
Callback = { pattern: BindingPattern, body: Expr }
CallArgument = Value(Expr) | Callback(Callback)
```

Each object field binds its same-named property in the initial grammar. Maintain
spanned equivalents; arrows may normalize in formatting but both must parse.
The directive parser consumes this callback representation without embedding
callbacks in value expressions or maintaining a separate pattern parser.

Use one immutable lexical-frame model for runtime and static binding identity.
Each frame contains the pattern's bindings and a parent handle. Runtime bindings
retain values and provenance; analysis bindings retain declared/projected types
and source spans without requiring runtime values. Share binding identities and
lookup rules across these views, not a second independent name resolver.

Frame storage must support retained scopes: owned or reference-counted handles
can outlive a callback stack frame when a directive carries its scope into later
body stages. An immediate expression callback can use a short-lived view of
that same frame model. Do not make a borrowed linked `Locals<'a>` chain the only
public representation, and do not fix delayed evaluation by copying locals into
frontmatter. Reuse the enclosing composition request, observer, context, and
file-value origins; never recapture process state.

### Failures, effects, and shared budgets

Evaluate the iterable once and callbacks sequentially. The first failure stops
the call; no partial successful result is returned and completed effects are
not rolled back. Existing permissions, timeouts, caches, and shell authorization
remain authoritative. Neither a callback nor a declared type grants permission.

Wrap failures as typed context carrying the function, original array index or
object key, and underlying cause. Nested calls preserve the complete iteration
path and original expression/source spans. Fatality, diagnostic identity, and
file-resolution failure accessors must still reach the underlying cause.

Use the same visited-item budget as directive iteration during composition,
including nested expression calls inside directives and transcluded documents.
A rejected filter item still counts. Count once per source-item visit per
invocation; a nested invocation's visits are additional work. Standalone
expression entry points receive an evaluation-session budget rather than a
fresh unlimited allowance per nested call. Budget exhaustion stops the session
and remains request-fatal through nested composition and expression fallbacks;
handling an ordinary callback error cannot reset its counters. Passive checking
consumes no runtime iteration budget. The default is 100,000 visits per root
request or standalone evaluation session, configurable through positive library
limits; `md compose` exposes `--max-iteration-visits`. Reject zero, negative,
and overflowing values, with no unlimited setting.

Generated document output remains governed by the composition output budget;
expression arrays are values, not rendered bytes. Do not double-charge rendered
output merely because it passed through an expression result. Composition
allows 16 MiB (16,777,216 bytes) of cumulative UTF-8 collection expansion by
default, configurable through library options and `--max-expansion-bytes`.
Nested output copied into an enclosing expansion and cached fragments inserted
into an expansion count each time; ordinary pipeline passes and buffer
reallocations do not. Unrelated source text and preflight copies do not count.
Fail before an append exceeds the limit; exactly reaching it succeeds. Both
limits remain shared through nested composition and participate in composition
identity. These limits do not cap arbitrary handler-internal allocations.

### Surfaces and editor behavior

Both functions work in every expression surface: frontmatter and body
interpolation, conditional expressions, lifecycle expressions, and `md compose`.
Claudine uses the same engine without a separate feature opt-in. No production
prompt migration or dependency-policy change is required.

DMLS highlights both arrows and every pattern binding, shows the owning function
and projected type on hover, offers nearest locals and `local.*` members inside
the callback, and restores outer bindings after exit. Explicit `fm.*` completion
continues to describe frontmatter. Incomplete callbacks use shared parser recovery,
not a separate editor grammar.

## Non-goals

- First-class callbacks, named callable values, or returned closures.
- Multiple input parameters or index parameters such as `(item, i) => ...`.
- Nested patterns, destructuring defaults/rest, or object reconstruction by filter.
- Directive-only rendering options or implicit result-to-pattern rebinding.
- A new general-purpose inference engine or separate callback permission system.
- `reduce`, `sort_by`, `find`, `some`, `every`, `flat_map`, or a new file-search function.

## Acceptance Criteria

1. Both functions evaluate as documented through interpolation, conditionals,
   lifecycle evaluation, and `md compose`, with typed whole-value results and
   produced strings retaining data provenance.
2. A hermetic Claudine lifecycle fixture uses filter over dependency specs to
   drive `when:`/`error:` through its normal preparation/execution path. It
   accepts implemented/empty dependencies and rejects an unimplemented one
   without depending on live repository specs or changing production prompts.
3. Both arrows and shallow patterns parse through the shared spanned/erased
   AST, including nested calls, ternaries, comma boundaries, and quoted arrows.
   Misplaced callbacks, invalid patterns, and invalid argument positions have
   located library/CLI/DMLS diagnostics without first-class callback leakage.
4. Reserved namespace/global bindings are rejected. Local/frontmatter shadowing
   is valid without an advisory. Explicit `local.*`, explicit `fm.*`, bare
   reads, null shadowing, nested reuse, and scope restoration agree.
5. Bound roots and members are not reported as undeclared document properties.
   Actual undeclared reads still follow existing absence rules. Missing iterable
   arguments are handled by the nullable signature and return `[]`.
6. Callback errors stop subsequent evaluation, name the original index/key,
   preserve nested iteration paths and source spans, and retain typed causes
   and fatality through report-json and other diagnostic consumers.
7. Arrays, typed tuples, and insertion-ordered object entries work. Empty/null
   inputs perform no callbacks; null elements/results remain actual values.
   Non-null scalars fail through the binder. Object filter results are entry
   arrays and expression-map results need not satisfy the input pattern.
8. Nested callbacks capture enclosing locals and share the directive-compatible
   frame model. Test retained frame lifetime independently of a callback stack,
   source provenance, and restoration after success or failure.
9. The authoritative catalog registers both functions with executable examples
   and matching signatures. Callbacks are excluded from lifecycle verbs. Topic
   docs describe arrows, patterns, iterable types, locals, budgets, and results.
10. DMLS highlights both arrows and patterns, offers scoped completion/hover with
    known types, handles incomplete input, and never executes providers during
    analysis. Composition and editor diagnostic codes and spans agree.
11. Shared validation rejects wrong arity, non-callback arguments, invalid
    patterns, and provably invalid calls even on empty/null sources. Known input,
    field, map-result, and filter-item types propagate; unknowns remain unknown
    where justified, and shadowed document facts never leak into local facts.
    Both file predicates refine a checked binding to `file`; absent-file false
    results do not exclude `file`, and separate call evaluations do not share
    a fact solely because their source text matches.
12. Ordered/counted fakes prove one iterable evaluation, one callback per visited
    item, first-error stop, and shared budget enforcement across nested calls.
    Once directives land, add composition coverage showing expression calls
    cannot reset the enclosing directive budget or evade its output accounting.

## Decisions and Reconciliation

- The shared schema/signature infrastructure and type-aware evaluation
  prerequisites are implemented. Use their catalog, binder, TypeScope, static
  read inventory, and diagnostic registry; no temporary catalog is needed.
- **2026-10-27 author-directed alignment:** supersedes the earlier one-arrow,
  identifier-only, array-only, unknown-only, and proposed shadow-advisory
  contracts. Both forms share patterns, `local`, iterable semantics, known-type
  projection, and iteration budgets. The implementation log preserves the
  phase-one historical proposals and records the current handoff.
- The former `param: String` and stack-only borrowed `Locals` sketches are
  superseded by reusable binding patterns and retainable lexical frames.
- Invalid iterable kinds remain shared binder errors; broadening valid kinds
  does not introduce a new argument-error subsystem.
- Required expression callbacks, arbitrary iterable expressions, array results,
  and lack of directive rendering options are intentional surface differences.

## Remaining Implementation Decisions

1. Final diagnostic identities and public error/context shapes must follow the
   existing library registry. No shadow-advisory decision remains open.
2. Choose concrete Rust option/builder names using existing library conventions.
   Implement the settled limits and accounting above; no budget policy decision
   remains open.
3. Document how the existing broad catalog signature expresses nullable result
   elements while call-site projection preserves specific types. Use shared
   signature/type machinery, not a callback-only schema dialect.
