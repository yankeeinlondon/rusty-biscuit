---
kind: feature
created: 2026-10-20
status: draft-spec
area: darkmatter
clarified: false
reviewed: false
implemented: false
review_iterations: 0
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

On 2026-10-20, Phase 1 of `2026-10-27-darkmatter-lifecycle-ownership` ran even
though `2026-10-16-type-narrowing`, one of the two specs in its `depends-on`,
was still a draft. The implement prompt (`prompts/_implement/implement-plan.md`)
had no gate on `depends-on`. The natural place for one is the prompt's own
lifecycle:

```yaml
has-unmet-deps: filter(depends-on, i => frontmatter(find_file('**/'+ i + '/spec.md'),'implemented') ? false : true
```

None of it can be written today: `depends-on` is a list of any length, and an
expression can neither iterate over it nor carry a per-item expression. The
gate was added instead as a `shell:` action in `start` that runs
`scripts/spec_dependencies.py` through `just spec-dependencies`. That works,
but it moves a one-line rule into a script, a shell approval, and a Python
dependency, and it cannot run in `initialize` (shell is refused before
preflight).

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
4. **`implemented: true` is weaker than the shell gate's rule.** The gate
   requires the dependency's directory to be under `_completed` (its review
   cycle closed). The expression form of that rule is
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

- The parameter shadows a document property of the same name inside `body`
  only. `filter(tags, tags => tags != "x")` is valid but earns a lint advisory.
- The parameter may not take the name of a namespace or global (`fm`, `ctx`,
  `env`, `current`, `err`, `timing`, and the host globals such as Claudine's
  `seq`, `loop`, `group`). That is an error, because shadowing `ctx` would
  silently change what every `ctx.*` read inside the body means.
- `body` may read anything the enclosing expression can: document properties,
  namespaces, and the parameters of enclosing lambdas.
- The undeclared-property advisory (`dm.expression.undeclared_property`) never
  reports a lambda parameter, and dot or bracket access on it
  (`dep.name`, `dep[0]`) reads the item, not the document.

### `map(list, item => body)`

Returns a new list holding `body` evaluated for each item, in order.

```text
map([1, 2, 3], n => n * 2)            → [2, 4, 6]
map(["a", "b"], s => upper(s))        → ["A", "B"]
map([], n => n * 2)                   → []
```

### `filter(list, item => body)`

Returns the items for which `body` is truthy, in their original order.
Truthiness is the engine's existing rule (`null`, `false`, `0`, `""`, `[]`,
and `{}` are falsy).

```text
filter([1, 2, 3, 4], n => n % 2 == 0) → [2, 4]
filter(["a", "", "b"], s => s)        → ["a", "b"]
filter([], n => true)                 → []
```

### Inputs and errors

| `list` is | `map` / `filter` returns |
| --- | --- |
| an array | the result above |
| `null` (absent property) | `null`, following the engine's null propagation |
| a string, number, boolean, or object | a type error naming the function and the received type |

- `body` is evaluated once per item, in order, and never for an empty list.
- An error inside `body` fails the whole call. The message names the function
  and the zero-based index of the failing item, then carries the inner error,
  for example: `filter(): item 2: frontmatter(): file argument not resolved`.
- There is no short-circuit; `filter` evaluates `body` for every item.

### Function catalog

`docs/schemas/expression-functions.yaml` gains both functions with a parameter
whose type is a function of one argument. The catalog's parameter grammar has
no such type today:

- `2026-10-28-schema-enhancements` introduces a `function` type and rewrites
  the catalog in that grammar. If it lands first, the parameter is written in
  that grammar (`function(parameters(item: unknown); returns(unknown))`).
- If this spec lands first, it adds the narrowest catalog extension that
  describes "a lambda of one parameter", and the schema-enhancements rewrite
  absorbs it.

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
lifecycle `when:`, and `md compose`. The editor (DMLS) must:

- highlight `=>` and the parameter, and resolve a hover on the parameter to
  "lambda parameter of `filter`";
- offer the parameter in completions inside `body`, ahead of document
  properties;
- report a misplaced lambda and a namespace-named parameter as diagnostics
  with source spans on the lambda.

## Non-goals

- Lambdas as values: storing, returning, or passing a named function.
- Multi-parameter lambdas, including an item index (`(item, i) => ...`); see
  open questions.
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
2. The motivating gate, written with `filter`, refuses
   `2026-10-27-darkmatter-lifecycle-ownership` while
   `2026-10-16-type-narrowing` is active, and passes a spec whose dependencies
   are all satisfied, through `claudine compose` on a real prompt.
3. A lambda outside a function-parameter position is a parse error with a
   dedicated diagnostic and span, in `md compose` and in DMLS.
4. A parameter named after a namespace or global is an error; a parameter
   named after a document property shadows it inside `body` only, with an
   advisory.
5. A lambda parameter is never reported as an undeclared property.
6. An error inside `body` names the function and item index.
7. `null` input returns `null`; a non-list input is a type error.
8. Nested lambdas see their enclosing parameters
   (`map(groups, g => filter(g.items, i => i != g.skip))`).
9. Both functions are in the catalog with executable examples, and the
   expressions topic documents lambdas, scope, and both functions.

## Open Questions

1. **Expose the item index?** A two-parameter form `(item, index) => body`
   is common in other languages but has no use case here yet. Recommendation:
   defer.
2. **Should `null` input return `[]` instead of `null`?** `[]` lets
   `length(filter(...))` work on an absent property without a fallback, but
   breaks the engine's rule that an absent input propagates `null`.
   Recommendation: keep `null`, since the interpolation fallback
   `filter(x, ...) || []` already covers it.
3. **Replace the shell gate?** Once this lands, the `depends-on` gate in
   `implement-plan.md` could move to `initialize` as a `when:`/`error:` pair
   with no shell approval. The script also resolves the older path-form
   entries (`../01-work-accounting/spec.md`) and reports ambiguous names,
   which a one-line `find_files` glob does not. Decide when this spec is
   planned.
