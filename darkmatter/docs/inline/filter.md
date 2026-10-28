# Filter Directive

**Planned:** `::filter` and `::end-filter` are not implemented yet. This page
records their intended behavior.

Use `::filter` to render selected items from a typed collection. Its positional
function decides which items to keep; the body receives each retained original
item. In contrast, [map](./map.md) transforms each retained item before binding
it for the body. Both directives share collection types, options, local scope,
composition rules, and shell approval behavior.

## Syntax and a First Example

```md
::filter <source> "<pattern> => <predicate>" <optional options>
content using the retained item's local bindings
::end-filter
```

The quoted predicate is optional and positional; there is no `fn=` option.
Both `->` and `=>` are accepted with identical semantics in all anonymous
functions. Sources are written directly, such as `tasks`, `ctx.packages`,
`fm.tasks`, or `local.team.members`, without interpolation delimiters.

```md
---
$schema:
    tasks: "{ title: string(required), completed: boolean(required) }[](required)"
tasks:
    - { title: Write docs, completed: false }
    - { title: Publish release, completed: true }
---
::filter tasks "task => !task.completed"
- {{task.title}}
::end-filter
```

Output:

```md
- Write docs
```

The body receives the task object, not the predicate's boolean result.

Omitting the predicate is equivalent to `"i => true"`: retain every item and
bind it as `i`. This includes falsey items such as `0`, `false`, or null. An
explicit `"i => i"` instead retains only truthy items. Predicates use the
existing Darkmatter truthiness rules, rather than requiring boolean results.

## Iterable Sources and Types

Arrays and tuples yield their elements. Objects yield entries with the general
shape `{key: string, value: any}`; both fields are present, including when the
value is null. Declared value types are preserved: a dictionary of `Person`
values exposes `value: Person`. Differently typed object properties contribute
a union of value types.

The source needs a statically known iterable type from the effective schema,
a namespace declaration, or an enclosing local binding. Literal frontmatter
values alone do not establish that declaration.

| Source type or value | Behavior |
| --- | --- |
| Declared array, tuple, or object | Iterate its elements or entries |
| `any[]` | Iterable, with unknown element types |
| `any` alone | Does not establish static iterability |
| Union of iterable alternatives | Accepted, with combined item types |
| Optional or nullable iterable | Accepted; missing or null renders nothing |
| Empty iterable | Render nothing |
| Union containing a non-null scalar alternative | Needs a guard that establishes iterability |
| Non-null scalar at runtime | Error |
| Undeclared source | Static typing error by default |

A declared optional source's absence differs from an undeclared source. Normal
schema validation still rejects a missing required frontmatter property.
Null inside an array is an actual item and is tested normally, unlike a null
source, which has no items.

The planned `--allow untyped-filter-variables` recovery permits an untyped
source, with corresponding library policy support. It bypasses only the static
source-typing requirement; it does not make scalars iterable, waive expression
errors, or authorize shell commands.

## Predicate Binding and Destructuring

The positional parameter pattern binds the original item while the predicate
evaluates. If the predicate is truthy, that same original item supplies the
body's bindings. There is no mapping-result binding or transformation stage.

Shallow object destructuring is supported:

```md
---
$schema:
    scores: "{ Ada: number(required), Grace: number(required) }(required)"
scores: { Ada: 90, Grace: 70 }
---
::filter scores "{key,value} -> value >= 80"
- {{key}}: {{value}}
::end-filter
```

Output:

```md
- Ada: 90
```

The pattern binds the input entry, not the predicate's truthy result. Nested
patterns, defaults, and rest bindings are outside the initial scope. Missing
pattern fields are errors: reject provably incompatible input shapes
statically and check uncertain shapes at runtime.

Use `::map` when the body should receive a transformed value. A filter predicate
may compute a value to test its truthiness, but that value never replaces the
item in the body.

## The Local Namespace and Nesting

Bindings belong to the reserved `local` namespace:

| Reference | Resolution |
| --- | --- |
| `foo` | Nearest local `foo`, otherwise normal lookup |
| `local.foo` | Nearest local `foo` only |
| `fm.foo` | Frontmatter `foo`, even when shadowed by a local |

A local holding null still shadows frontmatter. Inner map or filter bindings
may shadow outer locals; leaving the inner scope restores the outer meaning.
Reserved namespaces and registered host globals cannot be shadowed. Bindings
never modify frontmatter or leak to siblings.

Given declared teams with `active`, `name`, and a typed `members` collection:

```md
::filter teams "team => team.active"
## {{team.name}}
::filter team.members "member => member.active"
- {{member.name}} belongs to {{team.name}}
::end-filter
::end-filter
```

The source and block-level `when` use the enclosing scope before the new item
binding exists. Each anonymous function has its own parameter scope; an
option's parameter does not introduce a body binding.

A transclusion inside the body can use locals in its target, condition, and
explicit child parameters. The included document does not inherit these local
bindings automatically. Pass iteration values through the existing `set`
options in [block transclusion](../transclusion/block-transclusion.md).
Ordinary frontmatter inheritance retains its existing behavior.

## Options and Evaluation Order

| Option | Meaning |
| --- | --- |
| `when="expression"` | Gate the entire block once using the enclosing scope |
| `filter="item => predicate"` | Additional per-item selection after the positional predicate |
| `sort="item => key"` | Sort selected original items by a computed key |
| `sort-order="desc"` | Descending instead of default ascending order |
| `while="item => predicate"` | Stop before the first falsey result |
| `until="item => predicate"` | Stop before the first truthy result |

All anonymous functions accept either arrow. `when` is an ordinary condition,
not an anonymous function. `while` and `until` are mutually exclusive. Options
other than the ordering of these operations do not depend on their textual
order in the directive.

```mermaid
flowchart TD
    W{Block when is truthy?} -->|No| X[Render nothing]
    W -->|Yes| S[Resolve source once]
    S --> P[Positional predicate on original items]
    P --> F[Optional additional filter predicate]
    F --> O[Optional stable sort]
    O --> T[Take prefix with while or until]
    T --> R[Bind retained original item and render body]
```

A falsey `when` prevents all runtime source and item evaluation. If omitted,
the block is enabled. The positional predicate is evaluated once per visited
item; `filter=` is evaluated only when that first predicate is truthy. Both
conditions must pass. For example, assuming declared task fields:

```md
::filter tasks "task => !task.completed" when="show_tasks" filter="task -> task.priority > 0"
- {{task.title}}
::end-filter
```

Rejected items never reach sorting, stopping, or body rendering. All selectors
and stopping predicates receive original items, not predicate results. For
`[1, 2, 3, 4]`, either `while="i => i < 3"` or `until="i -> i == 3"` retains
`1, 2` when selection otherwise keeps all items. The stopping item is excluded.

Without sorting, processing is lazy: a stopping boundary prevents later item
predicates from running. Sorting examines all selected items first. Resolve
the source once per invocation and parse the expressions once. An inner filter
is a separate invocation for each reached enclosing iteration.

## Ordering and Sorting

Without sorting, arrays and tuples preserve element order and objects preserve
insertion order, including authored YAML key order. Selection preserves the
relative order of retained items.

For declared people with `active`, `name`, and numeric `age` properties:

```md
::filter people "person => person.active" sort="person -> person.age" sort-order="desc"
- {{person.name}}
::end-filter
```

Sort keys are computed once per selected item, before stopping. Numeric keys
compare numerically; string keys use case-sensitive, locale-independent
ordering. Equal keys preserve their previous order even in a descending sort.
Missing or null keys, mixed numeric/string keys, and object or array keys are
errors. Use an explicit fallback or normalization in the selector when needed,
such as `sort="person => lower(person.name)"`.

The comparator never executes expressions. Preserve source indices and object
keys through sorting so diagnostics identify the original data.

## Composition and Inserted Data

Maps, filters, and conditional regions share one Markdown-aware structural
traversal. Document frontmatter preparation finishes before body evaluation.
An outer false condition skips an inner filter; an inner condition can read an
outer iteration's local bindings. Iterations do not rerun frontmatter or
restart the whole document pipeline.

Carry immutable lexical scopes and source provenance through interpolation,
shell handling, and transclusion. Repeated bodies remain authored templates.
Values interpolated into them retain data provenance and cannot become new
executable directives. Do not rescan concatenated output. This follows
[Inserted Text Is Data](./interpolation.md#inserted-text-is-data), including
its existing treatment of authored replacements and explicit `as_markdown()`.

## Validation and Shell Approval

Validation checks source types, predicate parameter patterns, expressions, and
body bindings without executing predicates or performing their effects.
Composition and DMLS use the same types and diagnostics. A predicate does not
replace the body's item type with the predicate result type. Existing static
call checks still apply inside skipped regions.

Every authored shell command requires explicit whitelist permission regardless
of selection conditions. For a string-typed `pkg`:

```md
::filter packages "pkg => starts_with(pkg, 'biscuit-')"
::shell just test {{pkg}}
::end-filter
```

The existing [shell approval system](./preflight-checks.md) requests permission
for the typed template `just test {string}`, without enumerating packages or
executing the predicate. At each reached execution, the actual value passes
through existing template argument checks. A placeholder cannot add shell
operators, redirection, or additional commands. A type declaration establishes
an argument's type, not authorization. No filter-specific permission system
is introduced.

## Failures, Limits, and Diagnostics

Source, predicate, pattern, sort, and stopping-expression failures fail
composition; there is no automatic skip-on-error behavior. Body directives
retain their existing failure policies and explicit recoveries. Missing/null
sources and empty selections render nothing. Already performed effects are
not rolled back if a later iteration fails.

Maps and filters share request-wide iteration and generated-output budgets,
including across nested maps, filters, and transcluded documents. Count each
source item visited once, even when rejected by the first predicate. Sorting
or testing an item again at a later stage does not count it again. Budget
exhaustion fails composition. Existing transclusion depth limits remain
separate. Defaults and public configuration controls are to be selected before
implementation using representative workloads.

Diagnostics name the authored file, line, expression, original index or key,
and nested iteration path. Report static defects once; attach iteration
context to runtime failures. Sorting must not replace source positions with
output positions.

## Markdown Boundaries

The body repeats with its authored whitespace; no implicit separator is added.
Its newlines and blank lines repeat too. An empty filter removes its directive
region without adding a placeholder or blank line. Normal cleanup runs later.

Opening and closing directives occupy their own lines. `::end-filter` accepts
only trailing whitespace. Directive-looking text in fenced or indented code
blocks stays literal. Maps, filters, and conditional blocks must nest without
crossing boundaries. A mismatched closer reports its location and the still-open
inner block. Preserve list/block-quote container context and body indentation
through the shared structural scanner.
