# Current-State Function Argument Coercion

This addendum records how Darkmatter's expression functions currently fit
caller-supplied values to their parameters. It is a baseline for
`2026-09-21-schema-enhancements`, not a proposed coercion policy. The function
schema work can preserve, regularize, or intentionally replace these behaviors,
but it should not change them accidentally.

The current implementation does not have one universal argument-coercion layer.
Some functions share strict type-checking helpers, some share numeric or string
conversion helpers, and a small number implement bespoke rules in their own
handlers. Consequently, a declared parameter type alone does not always reveal
which caller inputs the runtime accepts.

## Core Runtime Values

Expression arguments arrive at functions as JSON values in one of six shapes:

- `null`
- boolean
- number
- string
- array
- object

There is no separate runtime type for `numberlike`, `boolish`, dates, paths, or
identifiers. Those concepts are recognized by individual functions after the
expression evaluator has produced one of the six JSON shapes.

Function calls are also arity-checked. Most functions require an exact number
of arguments. Optional and variadic behavior is implemented explicitly by the
handler rather than inferred from a shared runtime signature.

## The Default Posture Is Strict

The clearest common rule is that typed function parameters are strict unless a
function explicitly opts into conversion:

- a string parameter requires a JSON string;
- a number parameter requires a JSON number;
- an array parameter requires a JSON array; and
- passing another type produces a function argument error.

For example, the math functions `min`, `max`, and `abs` require actual numbers.
They do not accept numeric strings. String mutations such as `lower`, `upper`,
and `replace` require actual strings. `first` and `last` require actual arrays.

This is implemented by the shared `require_number`, `require_string`, and
`require_array` helpers in
[`functions/mod.rs`](../../../darkmatter/lib/src/markdown/compose/expression/functions/mod.rs).

## Null Handling

Many of the newer typed functions use a null-propagating contract:

1. arity is checked;
2. if any argument is `null`, the function returns `null`; and
3. otherwise, each argument must have the required type.

This behavior covers the core math functions, `first` and `last`, string
predicates and mutations, `terminal`, date formatting, and the list-rendering
functions. Here, `null` is neither converted nor reported as a type error.

This is not universal. Inspecting predicates consume `null` as a value, while
some conversion and legacy functions assign it their own meaning. Null behavior
therefore remains part of each function's contract.

## Inspecting Predicates Do Not Coerce

The predicates `is_string`, `is_number`, `is_array`, `is_null`, `is_object`,
and `is_integer` inspect the value's existing JSON type. They always return a
boolean and do not null-propagate.

Examples:

| Call | Result | Reason |
|---|---:|---|
| `is_number(4)` | `true` | The value is a JSON number. |
| `is_number("4")` | `false` | The numeric string is not converted. |
| `is_integer(4.0)` | `true` | The JSON number has no fractional component. |
| `is_integer("4")` | `false` | The string is not converted. |
| `is_null(null)` | `true` | `null` is inspected directly. |

`is_empty` is also an inspecting predicate, but recognizes several shapes:
`null`, an empty string, an empty array, and an empty object are empty. Numbers,
booleans, and non-empty containers are not.

## Numeric Conversion

Darkmatter has a reusable numeric conversion with this behavior:

| Incoming value | Numeric result |
|---|---|
| JSON number | The represented number. |
| Numeric string | The parsed number. |
| `true` | `1` |
| `false` | `0` |
| `null` | No conversion. |
| Array or object | No conversion. |
| Nonnumeric string | No conversion. |

`is_positive` and `is_negative` use this conversion and return an error when it
fails. Unlike most null-propagating predicates, they treat `null` as a failed
numeric conversion.

`number(value, default?)` and `round(value, default?)` use the same conversion,
but failed conversion selects the optional default, or zero when no default is
provided. The default is itself numerically converted; a default that cannot be
converted collapses to zero.

Numeric comparisons use a more permissive derivative outside function calls:
failed conversion becomes zero. Arithmetic operators use a different rule
again: they accept numbers and numeric strings but reject booleans and other
shapes. These operator rules matter when specifying a coherent language policy,
but they should not be mistaken for the default function-call behavior.

The conversion helpers are `to_number` and `to_number_coerce` in
[`expression/mod.rs`](../../../darkmatter/lib/src/markdown/compose/expression/mod.rs).

## Scalar Stringification

Some permissive functions use Darkmatter's scalar string representation:

| Incoming value | String representation |
|---|---|
| `null` | Empty string |
| Boolean | `"true"` or `"false"` |
| Number | Its canonical printed representation |
| String | The original string |
| Array or object | Compact JSON |

This is broader than a schema parameter of type `string`: it is an explicit
conversion performed by selected functions. It is implemented by
`scalar_string` in
[`expression/mod.rs`](../../../darkmatter/lib/src/markdown/compose/expression/mod.rs).

The conversion appears in several distinct contracts:

- `contains` compares values through their string representations;
- `has_key` converts the requested key to a string;
- flat list renderers require an array but stringify every element;
- JSON rendering uses the same compact representation; and
- Markdown list rendering stringifies scalar labels while recursively handling
  nested arrays and objects.

Because arrays and objects stringify as JSON, the term "scalar string" is
historical rather than a restriction on accepted input.

## String-or-Number Concatenation

`ensure_leading` and `ensure_trailing` implement a narrower conversion:

- strings and numbers are accepted;
- `null` propagates;
- booleans, arrays, and objects are rejected;
- both accepted operands are compared and concatenated through their string
  forms; and
- when the original value is numeric and the added affix is numeric or a
  numeric string, the combined result is parsed back into a JSON number when
  representable.

If the requested prefix or suffix is already present, these functions preserve
the original JSON value and its type. Their result type therefore depends on
both the input types and whether a mutation was necessary.

## Polymorphic Functions

Several functions accept multiple shapes as an intentional part of their
domain rather than as general-purpose coercion.

### `contains`

`contains(haystack, needle)` accepts every JSON shape:

- arrays compare each element's string representation with the needle's string
  representation;
- objects compare their values, not their keys;
- strings perform substring matching; and
- other scalar haystacks are stringified and searched as strings.

This is implicit string conversion, including for equality within arrays and
objects. Values of different JSON types can therefore compare as equal when
their rendered strings are equal.

### `length`

`length(value)` uses shape-specific behavior:

- strings count Unicode characters;
- arrays count elements;
- objects count keys;
- numbers count characters in their printed representation; and
- booleans and `null` produce zero.

The boolean and null cases are fallbacks, not errors or null propagation.

### Provider Identifiers

`pr` accepts either a positive integer or a string containing a positive
decimal identifier or canonical HTTP(S) URL. `cicd` similarly accepts a
positive integer or a non-empty, nonzero provider-native string. Integer inputs
are converted to their decimal string representation before provider lookup.

These are domain-specific unions followed by normalization to an internal
string, rather than instances of the general scalar-string rule.

### Structured Queries

`pr_list` and `cicd_list` accept either:

- a positive integer as shorthand for a result limit; or
- a structured query object.

Once an object is selected, its keys and values are validated against a closed
query vocabulary. The top-level integer shorthand should be represented as an
overload or union in a schema, not as an object coercion.

## Function-Family Summary

| Family | Representative functions | Current caller-input treatment |
|---|---|---|
| Strict number | `min`, `max`, `abs` | Number only; `null` propagates. |
| Strict string | `lower`, `replace`, `date`, `terminal` | String only; `null` propagates. |
| Strict array | `first`, `last`, list renderers | Array only; `null` propagates. Some renderers stringify elements. |
| Inspecting predicate | `is_string`, `is_number`, `is_integer` | No conversion; always returns a boolean. |
| Coercing numeric predicate | `is_positive`, `is_negative` | Number, numeric string, or boolean; failed conversion errors. |
| Numeric converter | `number`, `round` | Same numeric conversion; failed conversion uses a default or zero. |
| String/number mutation | `ensure_leading`, `ensure_trailing` | Strings and numbers only; conditionally reconstructs a number. |
| Broad polymorphism | `contains`, `length` | Shape-specific behavior, including implicit stringification or fallback values. |
| Identifier union | `pr`, `cicd` | Positive integer or constrained string, normalized to string. |
| Query union | `pr_list`, `cicd_list` | Positive integer shorthand or validated object. |

## Relationship to Schema Coercion

Darkmatter's frontmatter schema coercion is a separate stage with a separate
policy. It can normalize boolish and numberlike strings, stringify scalar
values for string-shaped schema types, serialize native values for YAML/JSON
content types, and recurse through selected arrays and objects. Those
conversions happen while fitting document state to a declared schema.

Expression function argument handling happens later, when an evaluated JSON
value is passed to a function handler. A coercion supported by frontmatter
schema validation is not automatically available to a function parameter. For
example, schema coercion can turn `"4"` into a number when a property declares
`number`, but `min("4", 5)` still passes a string directly and fails its strict
number check.

The current schema behavior is documented in
[`schema-definition.md`](../../../darkmatter/docs/topics/schema-definition.md#type-coercion).
The current expression-level contract is documented in
[`darkmatter-expressions.md`](../../../darkmatter/docs/topics/darkmatter-expressions.md#function-contracts),
although that document does not yet contain the full function-level inventory
recorded here.

## Implications for Function Schemas and Static Analysis

The current catalog records parameter types, optionality, variadic status,
return types, and fallibility. It does not encode the conversion policy used by
each parameter or handler. Treating every runtime acceptance as the declared
parameter type would lose useful distinctions:

- `min(a: number, b: number)` is strict even though other numeric functions
  accept numeric strings and booleans;
- `is_positive(val: any)` accepts many values syntactically, but only a
  number-coercible subset succeeds;
- `contains` accepts any value but compares through stringification;
- `ensure_leading` has a string-or-number input union and a conditional result
  type; and
- provider query functions have real overloads rather than a single coerced
  parameter shape.

Static analysis can describe the current implementation accurately if it keeps
three questions separate:

1. **Accepted input type:** which JSON shapes may reach the handler without an
   argument-type error?
2. **Normalization:** how is each accepted shape transformed before the
   function's domain logic runs?
3. **Failure behavior:** does a failed conversion produce `null`, an error, a
   caller-supplied default, or a built-in fallback?

A useful default rule for the proposed schema system is therefore available
from the code: parameters are strict unless their schema explicitly declares a
union or conversion. The exceptional behavior above then needs an explicit
representation rather than being inferred from broad types such as `any` or
from a function's implementation.

