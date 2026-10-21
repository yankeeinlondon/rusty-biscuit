---
kind: feature
created: 2026-10-20
status: draft-spec
area: darkmatter
clarified: false
reviewed: false
implemented: false
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
    - 2026-10-28-schema-enhancements
    - 2026-10-31-type-aware-evaluation
    - 2026-10-16-type-narrowing
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

## Design

### The lambda argument

A lambda is written `name => body`:

- `name` is one identifier. It binds the current item while `body` is
  evaluated.
- `body` is any expression, including a ternary, a function call, or another
  `map`/`filter` with its own lambda.
- `=>` binds more loosely than every other operator, so `x => a ? b : c`
  is `x => (a ? b : c)`.

**A lambda is an argument, never a value.** It may appear only in an argument
position the function catalog declares as a function parameter. Everywhere
else it is a parse error with its own diagnostic:

- `{{ x => x }}`, `a: "{{ f }}"` where `f` would hold a lambda, a lambda inside
  an array or object literal, or as an operand of an operator, are errors;
- a lambda therefore never reaches frontmatter, interpolation output, JSON, or a
  shell command, and the value model (`serde_json::Value`) does not change.

This keeps closures out of the evaluator: a lambda is evaluated in place, by
the function that receives it, while the enclosing scope is still live.

### Scope

- The iterable argument is evaluated in the enclosing scope before the
  callback parameter is bound. Within `body`, the nearest local parameter
  takes precedence over the shorthand lookup of a frontmatter property.
  Explicit `fm.name` access continues to read frontmatter.
- The parameter shadows a document property of the same name inside `body`
  only. `filter(tags, tags => tags != "x")` is valid but earns a lint advisory.
  The advisory is the draft proposal; its policy still requires confirmation.
- The parameter may not take the name of a namespace or global (`fm`, `ctx`,
  `env`, `current`, `err`, `timing`, and the host globals such as Claudine's
  `seq`, `loop`, `group`). That is an error, because shadowing `ctx` would
  silently change what every `ctx.*` read inside the body means.
- `body` may read anything the enclosing expression can: document properties,
  namespaces, and the parameters of enclosing lambdas.
- Reusing an enclosing lambda's parameter name follows these same scope
  rules: the inner parameter hides it only inside the inner `body`. This
  reuse introduces no additional prohibition or advisory.
- The undeclared-property advisory (`dm.expression.undeclared_property`) never
  reports a lambda parameter, and dot or bracket access on it
  (`dep.name`, `dep[0]`) reads the item, not the document.

### `map(list, item => body)`

Returns a new list holding `body` evaluated for each item, in order.

```text
map([1, 2, 3], n => n * 2)            → [2, 4, 6]
map(["a", "b"], s => upper(s))        → ["A", "B"]
map([], n => n * 2)                   → []
map(null, n => n * 2)                 → []
```

### `filter(list, item => body)`

Returns the items for which `body` is truthy, in their original order.
Truthiness is the engine's existing rule (`null`, `false`, `0`, `""`, `[]`,
and `{}` are falsy).

```text
filter([1, 2, 3, 4], n => n % 2 == 0) → [2, 4]
filter(["a", "", "b"], s => s)        → ["a", "b"]
filter([], n => true)                 → []
filter(null, n => true)               → []
```

### Inputs and errors

| `list` is | `map` / `filter` returns |
| --- | --- |
| an array | the result above |
| `null`, including an absent property | `[]` |
| a string, number, boolean, or object | a type error naming the function and the received type |

- Both functions always return arrays on success. Each takes two arguments:
  an array or `null`, and a one-parameter inline function. This does not
  introduce a variadic calling convention.
- `body` is evaluated once per item, in order, and never for an empty list
  or `null` input. A `null` element inside an array is an actual item and is
  mapped or tested normally; it is not treated as an absent iterable.
  Passive syntax and schema validation still applies to the callback even
  when no items will be evaluated; it performs no callback I/O.
- An error inside `body` fails the whole call. The message names the function
  and the zero-based index of the failing item, then carries the inner error,
  for example: `filter(): item 2: frontmatter(): file argument not resolved`.
  Evaluation stops at the first callback error; later items are not evaluated.
  Nested calls retain each enclosing function and failing item index while
  preserving the underlying error's source provenance.
- `filter` does not stop early because of a truthy or falsy result: it tests
  every item unless an error stops the call.

### Function catalog

Darkmatter owns the expression engine and the function signatures, typed
through `SimplifiedSchema`. The Darkmatter schema checker and DMLS must
understand and enforce the signatures of both functions. The iterable
parameter accepts `unknown[] | null`; the callback takes one
`unknown | null` item and returns `unknown | null`; the function result is
always `unknown[]`. The explicit null arms preserve null items and results
under the shared function-signature rules; array item types retain their
normal semantics. `filter` applies existing truthiness to the callback result.

`darkmatter/schemas/partials/functions.yaml` gains both functions using the shared
`SimplifiedSchema` function-signature contract defined by
`2026-10-28-schema-enhancements`. That shared grammar is a prerequisite;
planning determines implementation order. This feature does not introduce a
temporary catalog representation or a parallel checking path.

The shared registration and checking machinery must accept inline lambda
arguments for these executable functions. This extends the schema-enhancement
proposal that rejects executable registrations with function parameters, but
does not allow first-class function values. Schema checking and DMLS remain
passive: they validate the callback's form and signature without executing it
or performing the callback's I/O. No broader catalog or schema rewrite is
authorized by this feature.

The schema-enhancements inline example form, `example(call => result)`, splits
at the `=>` outside every parenthesis, so a lambda inside the call is not
mistaken for the separator:

```yaml
example(filter([1, 2, 3], n => n > 1) => [2, 3])
```

That spec's parser for examples must honor this rule; this spec adds a test
for it once both exist.

### Surfaces

The functions are expression functions, so they are available on every
surface that evaluates expressions: frontmatter interpolation, `::block when`,
lifecycle `when:`, and `md compose`. Claudine consumes Darkmatter's expression
engine and therefore gains access to these functions without a separate
opt-in. No production prompt migration or dependency policy change is
required by this feature. The editor (DMLS) must:

- highlight `=>` and the parameter, and resolve a hover on the parameter to
  "lambda parameter of `filter`";
- offer the parameter in completions inside `body`, ahead of document
  properties;
- report a misplaced lambda and a namespace-named parameter as diagnostics
  with source spans on the lambda.

## Non-goals

- Lambdas as values: storing, returning, or passing a named function.
- Multi-parameter lambdas, including an item index (`(item, i) => ...`). This
  feature takes a single item parameter; index access is deferred.
- A single-result file search. If one is added later it is named
  `find_first_file`, not `find_file`.
- `reduce`, `sort_by`, `find`, `some`, `every`, `flat_map`. Each is a small
  addition once lambdas exist, and each should be justified by a use.
- Type inference for lambda bodies. `2026-10-31-type-aware-evaluation` owns
  type-aware evaluation and `2026-10-16-type-narrowing` owns narrowing; a
  lambda parameter's type is `unknown` until they extend it.

## Acceptance Criteria

1. `map` and `filter` evaluate as in the tables and examples above, in
   interpolation, `::block when`, and lifecycle `when:`.
2. A controlled fixture exercised through Claudine's normal lifecycle
   expression path uses `filter` to select dependencies from fixture
   specifications and drives a `when:`/`error:` rule. It rejects the fixture
   with an unimplemented dependency and accepts the fixture with all
   dependencies implemented. The check must not depend on the current
   lifecycle state of repository specifications or modify a production
   prompt's policy.
3. A lambda outside a function-parameter position is a parse error with a
   dedicated diagnostic and span, in `md compose` and in DMLS.
4. A parameter named after a namespace or global is an error; a parameter
   named after a document property shadows it inside `body` only, with an
   advisory.
5. A lambda parameter is never reported as an undeclared property.
6. An error inside `body` names the function and item index, stops evaluation
   before later items, and preserves the inner error's source provenance.
   Nested failures retain each enclosing function and item index.
7. `null` or absent input and empty arrays return `[]` without evaluating
   the callback; other non-array input is a type error. Arrays containing
   `null` elements evaluate the callback for those elements normally, and
   `map` preserves null callback results as array elements. Empty and null
   iterables still receive passive callback syntax and signature validation
   without callback execution or I/O.
8. Nested lambdas see their enclosing parameters
   (`map(groups, g => filter(g.items, i => i != g.skip))`).
9. Both functions are in the catalog with executable examples, and the
   expressions topic documents lambdas, scope, and both functions.
10. DMLS highlights the arrow and parameter, identifies the owning function
    on parameter hover, and offers local parameters ahead of document
    properties in callback completions. After leaving a callback, its local
    parameter is no longer offered or resolved; enclosing bindings are
    restored after an inner callback ends.
11. Schema checking and DMLS reject missing or extra arguments, a non-function
    callback argument, and callbacks with anything other than one item
    parameter through the shared signature rules. Valid inline callbacks
    are accepted consistently without passive validation executing them.

## Open Questions

1. **Document-property shadowing advisory.** Should an otherwise valid local
   parameter that hides a shorthand frontmatter property produce the draft's
   proposed advisory? Local scope and lookup semantics are already agreed;
   this question concerns the advisory only.
2. **Diagnostics.** The intended errors and advisories are described above,
   but their precise diagnostic identities still need review under existing
   diagnostic policies.
3. **Operational constraints.** Expected list sizes, evaluation cost limits,
   and the behavior of expensive or effectful expressions inside callbacks
   have not yet been specified.
