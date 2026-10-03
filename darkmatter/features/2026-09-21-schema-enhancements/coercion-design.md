# Coercion Design

This is the proposed function-binding contract for
`2026-09-21-schema-enhancements`. It replaces the preliminary design; it does
not describe implemented behavior. The companion `current-state-coercion.md`
is the compatibility baseline. The function catalog in
`claudine/docs/schemas/partials/functions.yaml` is a draft to reconcile with
this contract, not an authority for preserving its current broad types.

> **Status (updated per 2026-09-25 use-case review):** [spec.md](./spec.md) is
> the normative document. This design is supporting detail that must conform
> to the spec; where the two conflict, the spec wins. The use-case review
> settled null-propagation order, numeric precision, lazy `and`/`or`, and
> string-refinement inheritance, removed number → text and boolean → text,
> and replaced this document's four-way compatibility classification with the
> spec's five argument-type categories. `number()` failure behavior and the
> `round` fallback are per-function domain behavior, resolved in the catalog
> migration rather than by the coercion layer.
>
> **Updated per 2026-09-25 clarification round 3:** there is now **one**
> coercion engine and rule list, used both when frontmatter is validated
> against `$schema` and when a function is called. Number → text, boolean →
> text, and native → `json`/`yaml` serialization are back as always-safe
> conversions; constraints never produce design-time warnings on their own;
> and union selection was extended (since restated in round 4). Sections
> changed in this round are marked below.
>
> **Updated per 2026-09-25 clarification round 4:** every type has two
> interfaces (accepted inputs, and what the function is guaranteed to
> receive); `numberlike` and `boolish` are kept and deliver values exactly as
> sent; the union rule is restated as exact match, then the one reachable
> option, then a tie-breaker, replacing the numbered steps; refinements are
> judged like constraints at design time; and **every** catalog function
> migrates in one pass, so the first slice is a build order, not a stage.
>
> **Updated per 2026-09-25 clarification round 5:** DMLS call diagnostics and
> the strict-mode setting (`expressions.strict` in `.dmls.toml`) now
> ship in the spec, not parent Phase E; object options whose fields break ties
> toward different options are an ambiguity type error; and a native value
> against `json | yaml` serializes as JSON, the second of exactly two
> tie-breakers.
>
> **Updated per 2026-09-25 clarification round 6:** the strict-mode setting
> is confirmed as `expressions.strict`; `file_exists` has no exception to the
> engine (`file_exists(42)` checks for a file named `"42"`); and any tie the
> two tie-breakers do not settle is confirmed as an ambiguity type error.

The central rule is:

> Declare the types the function needs. Darkmatter binds caller values to those
> types using one shared, deterministic coercion layer. Invalid arguments never
> reach the function implementation.

Standard coercion is default-on. No strict modifier, per-function conversion
matrix, or new coercion syntax is proposed for the first implementation.
Inspecting functions such as `is_number(x: any)` already preserve their input:
`any` accepts the original value without conversion.

## Scope and Ownership

Darkmatter owns the compiled function descriptors, coercion rules, runtime
binder, and passive compatibility APIs. DMLS and Claudine consume these public
library surfaces; they must not reproduce the matrix or parse signatures into
separate semantic models.

_Updated per 2026-09-25 clarification round 3._ This design covers function
arguments, including nested parameter types, **and** frontmatter validation.
Both surfaces use one engine and one rule list with no per-surface exceptions:
today's `coerce_frontmatter` rules (feature 2026-05-28-schema-coercion) are
absorbed into it, and the spec lists the resulting intentional behavior
changes. Frontmatter keeps its own surrounding pipeline (presence and default
semantics, the compose write-back, deferral of shell-pending values); only the
conversion rules and union selection are shared. This design does not change
arithmetic operators, comparison operators, or truthiness.

Parsing a `function(...)` schema produces a callable *description*. It does not
create an executable function, authorize an effect, or imply that arbitrary
JSON values can become functions. Executable Darkmatter functions must also
have a registered implementation.

## Grammar and the Handler Contract

Use the grammar being added to `SimplifiedSchema`:

```yaml
min: |-
    function(
        parameters(a: number, b: number);
        returns(number);
        category(Math);
    ) -> Returns the smaller of two numbers.
```

`number` is the handler's input type. Numeric strings are accepted by the
binder; spelling the parameter `numberlike` merely to accommodate those callers
would obscure the handler contract. Similarly, use `boolean`, not `boolish`,
when the handler needs a boolean. Explicit unions remain appropriate when the
function actually operates on several types.

_Updated per 2026-09-25 clarification round 4:_ each parameter type describes two interfaces:
what the binder **accepts** from the caller (the conversion rules), and what
the handler is **guaranteed to receive**. `number` accepts `4` or `"4"` and
always delivers a number. `numberlike` accepts exactly the same inputs but
delivers the value as sent (`"02134"` stays `"02134"`); `boolish` does the same
for boolean inputs. Declare `numberlike` or `boolish` only when the handler
genuinely needs the caller's original form, for example to return the same
type it was given.

The compiler must preserve type structure and constraints instead of reducing
parameters to the six JSON runtime shapes. For example, `number(integer)` is a
constrained number, not a new primitive named `integer`; `file` and `json` have
validation requirements beyond being strings.

The invocation grammar has these presence rules:

| Declaration | Meaning |
|---|---|
| `parameters(value: string)` | One required, non-null argument. |
| `parameters(value: string \| null)` | One required argument; explicit null is valid. |
| `parameters(value?: string)` | Zero or one arguments; explicit null is invalid. |
| `parameters(value?: string \| null)` | Omission and explicit null are both valid and remain distinct. |
| `parameters(void)` | Exactly zero arguments. |

Recommended completion of the grammar: optional parameters must trail required
parameters, and one final rest parameter may follow them. Use the catalog's
`parameters(...values: any)` spelling: the type after `:` describes each
remaining argument. Thus `...values: number[]` means zero or more array
arguments, not zero or more numbers. A rest parameter cannot also be optional.
This is function-parameter syntax; tuple spreads use `...number[]` as specified
in the parent spec. Parameter names must be unique. Named-call syntax is not
introduced by giving parameters names.

Defaults do not repair invalid supplied values. If a parameter default is
supported by the surrounding grammar, compile and validate it once, and apply
it only to omission. Otherwise preserve omission in `BoundArgs` so the function
can implement documented domain behavior. Do not import frontmatter's
null-as-absent behavior into function parameters.

`category` is documentation metadata. `returns` describes successful results;
the binder does not coerce returns to conceal an implementation mismatch.
`fallible` describes domain failure after valid binding. Arity or input-type
errors alone do not justify marking `min` or a string predicate fallible.

## Binding Pipeline

```text
compiled signatures + caller arguments
                  |
          filter candidates by arity
                  |
       validate original values exactly
                  |
      if needed, construct coerced candidates
                  |
       validate complete candidate contracts
                  |
        select one unambiguous signature
                  |
          invoke its handler once
```

For ordinary eager functions, the evaluator evaluates arguments once, in its
established order. Each overload sees the same original values. Binding is
transactional: failure does not mutate caller values or commit partly converted
arrays or objects. Candidate testing never invokes handlers, probes files,
executes expressions, or performs provider requests.

The promise that an invalid call never invokes its handler does not undo
side effects of argument expressions already evaluated by the evaluator.

Lazy constructs require a distinct evaluator path. The draft catalog describes
`and` and `or` as short-circuiting; the binder must not eagerly evaluate their
remaining operands to select an overload. Preserve or explicitly settle their
evaluation strategy during migration. A function signature alone cannot encode
laziness; an internal evaluation-strategy registration may supply it without
inventing public coercion syntax.

Handlers receive validated, normalized arguments. A `BoundArgs` representation
is sufficient initially, provided it retains omission, nullable values, union
values, and nested structure. Accessors must not parse or coerce again.
Descriptor/handler type mismatches are implementation defects, not caller
`InvalidType` errors. Generated argument structs are optional later work; do
not force every number through `f64` merely to simplify accessor design.

## Standard Conversion Rules

_Updated per 2026-09-25 clarification round 3: this supersedes the use-case
review's "text → number and text → boolean only" rule. Number → text and
boolean → text are always safe, and native values serialize into `json` and
`yaml`; null is still never serialized._

Exact acceptance means the original value satisfies the complete target type,
including its passive constraints. Preserve that value. If it does not satisfy
the contract, try the permitted target-directed conversion and validate the
result. Failure of a same-type constraint does not permit arbitrary repair:
strings are not trimmed to satisfy a pattern and numbers are not clamped.

| Target | Mismatched source accepted for conversion | Bound representation |
|---|---|---|
| `number` | Numeric string under the grammar below | JSON number |
| `number(integer)` | Numeric string whose parsed value is integral | Integral JSON number |
| `boolean` | Boolean word under the grammar below | JSON boolean |
| `numberlike`, `boolish` | Nothing is converted; accepts what `number` / `boolean` accept (updated per 2026-09-25 clarification round 4) | The original value, unchanged |
| `string` | Number or boolean | Canonical text form, then validated |
| String refinements: `file`, dates, `url`, `email`, enums, string literals | Number or boolean, converted to its canonical text form | String satisfying the refinement |
| `json`, `yaml` | Native object, array, number, or boolean (not null) | Serialized text satisfying the content type |
| Typed array or tuple | Array with compatible shape | Recursively bound elements |
| Structured object | Object with compatible declared structure | Recursively bound declared fields |
| Other targets | No additional conversion | Original value must validate |

These are direct conversion rules, not paths through a conversion graph.
Do not chain conversions, wrap a scalar in an array, flatten an array, parse a
string into an object or array, or stringify a container into plain `string`.
Booleans do not become numbers. Null does not become zero, false, an empty
string, `"null"`, or omission.

### Numeric and Boolean Boundaries

_Updated per 2026-09-25 clarification. This section previously pinned the
existing schema numeric regex and six boolean spellings; the spec's approved
conversions replace both, for frontmatter as well as function calls (updated
per 2026-09-25 clarification round 3)._

A numeric string is accepted for a `number` target when, after trimming
surrounding whitespace, it is an ASCII decimal number that may use:

- a leading sign, `"-3.5"` or `"+4"`;
- an exponent, `"1e3"` or `"2.5E-2"`;
- a leading dot, `".5"`; and
- underscore digit separators between digits, `"1_000"`.

So `" 4 "`, `"+4"`, `"1e3"`, `".5"`, `"04"`, `"4.0"`, and `"1_000"` all bind.
`NaN`, `infinity`, hexadecimal, and locale separators such as `"1,000"` are
rejected. A trailing dot (`"5."`), a leading or doubled underscore, and an
empty or whitespace-only string are also rejected; treat these edge spellings
as an interpretation to confirm, not a ruling. Native JSON numbers are not
subject to a string-spelling rule.

Check integrality on the parsed numeric value: `"4.0"` can bind to
`number(integer)`; `"4.2"` cannot. Never truncate, round, saturate, or use a
fallback during argument conversion.

_Precision updated per 2026-09-25 use-case review (spec decision 23)._
Whole-number text becomes an exact signed 64-bit integer when it fits:
`"9007199254740993"` binds as exactly `9007199254740993`. Whole-number text
that is out of range, or would lose digits, is a type error rather than a
silently imprecise float. Ordinary decimals (`"0.1"`, `"3.14"`) use normal
floating point; reject non-finite results and underflow to zero for a nonzero
input. "Whole-number text" means a spelling with no decimal point and no
exponent; `"1e3"` and `"4.0"` take the floating-point path (confirmed in clarification round 3). Freeze fixtures for range,
precision, and signed zero; with one engine, schema and function paths share
the same limits by construction. This does not promise
arbitrary-precision decimal arithmetic.

A boolean word is accepted for a `boolean` target when, after trimming
surrounding whitespace, it is one of `true`/`false`, `yes`/`no`, `on`/`off`,
or `1`/`0`. Matching is case-insensitive (`"Yes"`, `"OFF"`), confirmed in
clarification round 3; whitespace trimming remains an interpretation to
confirm. Only strings convert:
a native number `1` does not bind to `boolean`, and a boolean never binds to
`number`.

_Updated per 2026-09-25 clarification round 3:_ the reverse direction is
always safe. A `string` target accepts a native number or boolean as its
canonical text form (`2026` → `"2026"`, `1.2` → `"1.2"`, `true` → `"true"`).
Every such value has exactly one text form, so meaning is unchanged. The
engine sees the value YAML produced, so an unquoted `1.10` is already `1.1`.

### Refinements and Content Types

Constraints are checked against the bound value. Numeric bounds, integrality,
string patterns, and enum membership can eliminate a conversion candidate.
Numeric and boolean literals can use their primitive conversion followed by
equality validation. A matching string remains a string; its content is never
evaluated.

_Updated per 2026-09-25 clarification round 3:_ for `json` and `yaml`, a string
is already encoded content: validate its contents and preserve it. Do not
serialize an invalid content string into a quoted string to make it valid. A
native object, array, number, or boolean is serialized to JSON or YAML text and
then validated; serializing changes representation, not meaning. `null` is not
serialized. Explicit content types still need dedicated descriptors; treating
them as plain `string` would lose their validation.

_Updated per 2026-09-25 clarification round 3:_ refined text types (including
string literals) accept text, including the text produced by number → text
and boolean → text, and then apply their own validation. `file_exists(2026)`
against `file_exists(value: string | file | null): boolean` binds `"2026"` and
checks for a file of that name, with no design-time warning; the parent spec's
advisory always-false exception for known unsupported types is retired
(updated per 2026-09-25 clarification round 6, spec decision 63). A number sent to a `date` becomes text and then fails the date
check normally. Constraints (`min`, `max`, `pattern`, `integer`) are enforced
on the final value and never repaired.

Passive `file` validation may recognize syntax and retain origin metadata, but
existence, remote availability, and contextual resolution remain domain work
using the captured composition context. Neither static checking nor overload
selection may perform those effects. A syntactically valid file reference can
therefore bind successfully and still fail during a fallible file operation.

### Containers

Recursion follows declared type structure. A plain `object` accepts an object
unchanged; it does not infer field types. A structured object binds its declared
fields and applies its own required-field and additional-property rules.
Function parameter optionality does not rewrite those nested schema rules.
Arrays retain order, objects retain keys, and tuples enforce their declared
positions and length before success. Omitted tuple slots are not synthesized
as null. A rest parameter binds each supplied argument to its element contract.

_Updated per 2026-09-25 clarification round 3:_ in frontmatter, recursion now
reaches every declared depth, including nested objects; today's
`coerce_frontmatter` stops at top-level properties, typed arrays, and inline
objects.

Report nested failures with a path, such as argument `query`, `/labels/2`.
A bad element prevents the entire call from reaching its handler. A missing
required tuple slot is a type error at its path; a single value is never
wrapped into a list. (Confirmed per 2026-09-25 use-case review; the spec's
Containers table gives the worked outcomes.) Exact `any`,
`any[]`, or opaque `object` matches preserve their contents without recursive
normalization.

## Unions and Overloads

A union describes accepted values for one parameter. An overload describes a
complete call contract, including its return type and fallibility. Use the
existing YAML list representation for overloaded function properties:

```yaml
pr_list:
    - |-
        function(
            parameters(count: number(integer; min(1)));
            returns(string[]);
            fallible;
        )
    - |-
        function(
            parameters(query: object);
            returns(string[]);
            fallible;
        )
```

This illustrates the catalog's integer shorthand and string-array result.
The final query arm needs a declared structured query type and its bounds;
`object` here is not a claim that every object is a valid provider query.
The count overload is domain shorthand, not a global integer-to-object coercion.

### Union Selection

_Updated per 2026-09-25 clarification round 4 (spec decision 50): this
restatement replaces the numbered priority list from the use-case review and
round 3, which in turn replaced the former "group by distinct result, else
ambiguity" rule and frontmatter's "first arm in index order" root-union rule._

1. **Exact match.** If the original value satisfies any option (arm),
   preserve it. Multiple exact options are harmless because a union does not
   select different implementations. A value that a `numberlike` or `boolish`
   option accepts is an exact match of it.
2. **One reachable option.** Otherwise, an option is reachable when a
   standard conversion produces a value satisfying it, constraints included.
   If exactly one option is reachable, use it; if none is, fail binding.
3. **Tie-breaker.** Only when several options are reachable with different
   results is a tie-breaker needed. There are exactly two (updated per
   2026-09-25 clarification round 5, spec decisions 59–61):
   - **number over boolean** for text reading as both (`"1"`, `"0"`) against
     a union with number and boolean options;
   - **JSON over YAML** for a native object, array, number, or boolean
     against a union with `json` and `yaml` options, because JSON text is
     also valid YAML: `{ port: 8080, debug: true }` binds
     `{"port":8080,"debug":true}`.

   Any other tie fails binding with an ambiguous-union reason (updated per
   2026-09-25 clarification round 6, spec decision 64: confirmed, so that
   adding a conversion can never turn into a silent, order-dependent guess).
   Structured-object
   options apply the tie-breakers field by field: `{ id: "1" }` against
   `{ id: number } | { id: boolean }` binds `{ id: 1 }`. When tied fields
   point to different options, binding fails with an ambiguous-union reason
   naming every reading: `{ a: "1", b: "1" }` against
   `{ a: number, b: boolean } | { a: boolean, b: number }` could be
   `{ a: 1, b: true }` or `{ a: true, b: 1 }`.

Thus `"4"` stays a string for `string | number`; `"1"` becomes the number `1`
for `number | boolean` but `true` for `number(min(5)) | boolean`; `2026`
becomes `"2026"` for `string | string[]` because only `string` is reachable;
and for `number | number(integer)`, `"4"` becomes one numeric result.
`"true"` against `boolean | json` remains the original string because it is
already valid JSON content.

The same rule governs frontmatter property-level and root-level unions. Text
against `json | yaml` is not a tie: it is content, validated and preserved as
an exact match of each option it is valid for (updated per 2026-09-25
clarification round 5).

Do not distribute every nested union into a Cartesian product in advance.
Resolve locally and retain alternatives only where enclosing constraints need
them. If analysis cannot finish within its complexity budget, it must return
an unresolved result, never choose the first arm.

### Overload Selection

_Confirmed per 2026-09-25 use-case review (spec decision 26)._

Filter by arity, then validate each full signature. A signature that accepts
all arguments unchanged outranks any signature requiring conversion. Among
successful converted signatures, there is no additional ranking by number of
conversions, catalog order, or perceived specificity.

Exactly one signature at the best available tier wins. Two distinct exact
signatures, or two converted signatures with no exact winner, produce an
ambiguous-call error even if their bound arguments happen to be equal: they
may select different handlers or result contracts. Reject duplicate signatures
at catalog compilation. Authors should use a parameter union for one operation
that accepts overlapping types instead of relying on a specificity heuristic.

With overloads `f(string)` and `f(number)`, `f("4")` selects the string overload.
With `f(number)` and `f(number(integer))`, `f(4)` is ambiguous. Reordering the
catalog must never change either outcome. Do not retry a different overload
when the selected handler later returns a domain error.

## Invalid Input and Fallibility

Expose one `InvalidType`-like binding error for an argument that cannot satisfy
its parameter contract. Keep distinct typed reasons inside it rather than
forcing clients to handle unrelated top-level type/coercion/constraint errors.

Conceptually, it carries:

- function name and candidate/selected signature identity;
- parameter name and argument index;
- nested value path and available call/argument source spans;
- expected type with constraints and actual source type;
- received value as well as its type;
- reason: incompatible type, malformed conversion, out-of-range conversion,
  violated constraint, or ambiguous union, carrying each reading the value
  could have taken (updated per 2026-09-25 clarification round 5: ties the two
  tie-breakers do not settle, such as object options whose fields disagree,
  restore the ambiguous-union reason that round 4 had removed); and
- candidate diagnostics when overload resolution found no valid signature,
  explaining why each candidate failed.

Every argument is checked; one error reports all failing arguments.

Arity and ambiguous-overload failures are sibling binding errors, also carrying
the function name and source location. Do not arbitrarily blame the first
catalog signature when several candidates fail. Rich diagnostics can be built
lazily; a boolean compatibility check should not allocate error messages.

Example: `min("pear", 5)` produces `InvalidType` for argument `a`, expected
`number`, actual `string`, reason `MalformedConversion`. `min([], 5)` uses
`IncompatibleType`. Neither invokes `min`. A positive-number constraint failing
on `-1` is likewise an `InvalidType` binding failure with a constraint reason.

A fallible handler may return a domain error after binding. For example,
`date(value: string, format: string)` can receive an exact string that its date
parser cannot interpret. If its parameter instead declares a date refinement,
that refinement's passive validation failure belongs to binding. Where the
contract draws that boundary determines the error category.

Successful results must satisfy `returns`; a violation is an implementation
contract defect. Do not turn it into caller blame, retry binding, or silently
coerce the returned value. Verify result contracts in conformance tests.

## Explicit Conversions and Domain Operations

Narrowing parameters does not mean banning functions whose *operation* is
conversion. These are the justified exceptions to conversion-free handlers:

| Function family | Contract and responsibility |
|---|---|
| `is_number`, `is_string`, `is_integer` | Accept `any`; inspect the original value. `is_number("4")` remains false. |
| `is_positive`, `is_negative` | Propose `number`; the binder handles numeric strings. Booleans become invalid inputs under the standard matrix. |
| `round` | Propose `number → number`; malformed inputs fail binding. The fallback is a per-function catalog-migration item (see below). |
| `number(value, fallback?)` | Accept `any` for the conversion subject and `number` for a supplied fallback. Conversion is the operation; its failure behavior is a per-function catalog-migration item. |
| `contains`, `length` | Declare their actual semantic input shapes, including `any` if every JSON shape has intended behavior. Do not narrow solely for appearance. |
| List renderers | Accept `any[]` when rendering heterogeneous elements is the operation; rendering is not argument coercion. |
| `ensure_leading`, `ensure_trailing` | Retain a string/number union if type-preserving concatenation remains intended behavior. |
| Provider identifiers | Declare meaningful numeric/string alternatives and their passive constraints; contextual provider lookup remains domain logic. |

_Updated per 2026-09-25 use-case review:_ what `number("pear")` and
`number("pear", 7)` return, and the `round` fallback, are domain behavior, not
coercion-layer rules. The spec lists them as per-function items for the catalog
migration. This document's earlier proposal (a failed conversion uses a valid
supplied fallback, otherwise a domain error that makes `number` `fallible`) is
input to that item, not contract. A malformed supplied fallback such as `"x"`
fails binding in the layer regardless.

Null propagation is also domain behavior, not a global shortcut (confirmed per
2026-09-25 use-case review). A function receives null only where it declares
`| null`; it binds all arguments, then applies its own null rule. A null
argument never hides another argument's invalid type. This intentionally
differs from implementations that return null before checking the remaining
arguments.

## Public Compatibility API and DMLS

The spec's boolean type-to-type API needs a precise meaning. Knowing only that
an input is `string` cannot prove whether it contains `"4"` or `"pear"`.
Likewise, `number → number(min(0))` depends on the value.

Provide a cheap boolean query and a richer classification from the same engine.
The following names and shapes are illustrative public API design, not existing
Rust symbols:

```rust
fn may_bind(source: TypeId, target: TypeId) -> bool;
fn compatibility(source: TypeId, target: TypeId) -> Compatibility;
fn check_value(value: &Value, target: TypeId) -> Result<BindingPlan, BindingError>;
fn bind_call(function: FunctionId, args: &[Value]) -> Result<BoundCall, BindingError>;
```

Prefer the name `may_bind` over `can_coerce` to communicate uncertainty and
include exact acceptance. Its contract is **not proven
impossible**: false proves no source value can bind; true means at least a
potential match, including cases the analyzer cannot decide. It is a
conservative possibility check, not proof that a call is safe. An exact
existential decision for arbitrary patterns and refinements is not promised.

_Updated per 2026-09-25 use-case review:_ the richer classification must
return the spec's five argument-type categories, or map exactly onto them. The
former four classifications map as follows:

| Former classification | Spec category |
|---|---|
| `Exact` | 1 — perfect type match |
| `Guaranteed` | 1 — number or boolean → text, native → `json`/`yaml`, and literals whose conversion is certain, such as `"4"` → `number` (updated per 2026-09-25 clarification round 3) |
| `Conditional` | split into 2 (`T \| null` → `T`), 3 (might fit: unions, text → number or boolean), and 4 (`unknown`/`any`); constraints alone never make a pair conditional (updated per 2026-09-25 clarification round 3) |
| `Impossible` | 5 — known to be wrong |

`may_bind` is false only for category 5. A concrete value check uses the
runtime rules, so literals get definitive results (1 or 5) without invoking a
function. A source type with no inhabitants should be tracked as unreachable
by the analyzer, not treated as a possible runtime argument.

| Source → target | Category |
|---|---|
| `number → number` | 1 |
| `number → string` | 1 (always-safe conversion; updated per round 3) |
| `number → string(min(3); max(3))` | 1 (constraints checked at runtime; updated per round 3) |
| `string → number` | 3 |
| literal `"4" → number` | 1 |
| literal `"pear" → number` | 5 |
| `number → number(integer)` | 1 (updated per round 3) |
| `number \| null → number` | 2 |
| `number \| boolean → number` | 3 |
| `numberlike → number` | 1 (updated per 2026-09-25 clarification round 4) |
| `number → numberlike` | 1 (updated per 2026-09-25 clarification round 4) |
| `string → numberlike` | 3 (updated per 2026-09-25 clarification round 4) |
| `string → file`, `number → enum(...)` | 1: refinements are judged like constraints; only literals are judged by value (updated per 2026-09-25 clarification round 4) |
| `boolean → number` | 5 |
| `any → number` | 4 |
| `object → number` | 5 |

Source unions quantify over their possible values: one matching source arm is
not proof that the whole union is safe. Target unions must include the same
ambiguity rules as runtime. Container reasoning must include shape constraints:
for example, unconstrained `string[] → number[]` is conditional, and even
`object[] → number[]` can succeed for an empty array. Element incompatibility
alone is insufficient to prove the entire array type impossible.

_Updated per 2026-09-25 clarification round 3:_ Darkmatter rejects a category 5
call (including known arity errors, bad literals, and definite ambiguity) at
composition, before evaluation; that ships with the spec and applies to every
function (updated per 2026-09-25 clarification round 4). DMLS maps categories
to diagnostics as the spec's table defines — category 5 is an error; category 3
is a warning by default; categories 2 and 4 warn only in strict mode. _Updated
per 2026-09-25 clarification round 5:_ the DMLS diagnostics and the strict-mode
setting (`expressions.strict` in `.dmls.toml`, overridable through
`workspace/configuration`; name confirmed and made the only call-diagnostic
setting, with no per-function setting such as the parent's `file_exists`
unknown-input warning, updated per 2026-09-25 clarification round 6) ship in
the spec, not parent Phase E. DMLS obtains
every category from `compatibility` and the whole-call analysis, so an editor
error and a composition-time rejection can never disagree.
Analyze whole signatures for overloaded calls. Per-argument booleans cannot
prove that one consistent overload accepts the call. When different possible
inputs select different overloads, retain the union of possible returns and
fallibility rather than choosing one prematurely.

Type predicates require trusted narrowing metadata; `returns(boolean)` and a
category label do not express a type guard. Start with library-owned predicate
facts tied to registered functions. A successful coercion of a variable at a
call site does not narrow or rewrite the original variable: binding converts
the argument copy. Narrow only facts justified by control flow.

### Performance and Passivity

Parse signatures, resolve named types, compile constraints, and validate the
catalog once per schema revision. Use compact interned type IDs, a direct
primitive relation table, precomputed arity ranges, and cached relations for
compound types. Cache keys include catalog/rule revision; source-document
changes must not reuse stale literal or call results.

The primitive boolean query should need no parsing, value allocation, regex
compilation, diagnostic formatting, or I/O. Compound type checks traverse
compiled structure and may return category 3 when proof exceeds a bounded
analysis budget. Never mistake an exhausted budget for incompatibility.
Concrete runtime binding must either finish validation or return an explicit
resource-limit error; it cannot accept an unresolved candidate.

A batch API may accept pairs of type IDs and fill caller-owned output storage
in input order. Begin with a sequential loop sharing caches. Do not add threads,
async tasks, or synchronization per primitive pair: measure representative DMLS
workloads first. Benchmark cold catalog preparation separately from warm
primitive, constrained, union, container, and whole-call checks. Performance
claims require measurements, not merely the presence of a batch API.

All analysis is passive. It cannot evaluate expressions, resolve live provider
state, fetch files, or call handlers. Value checks and runtime conversion must
share the same rule implementation; static analysis adds conservative proofs
around it rather than maintaining another conversion policy.

## Catalog Integration and Migration

### Implementation Structure

Build three cooperating parts inside Darkmatter, rather than separate runtime
and editor implementations:

1. **Compiled function contracts.** Compile the authored grammar into
   descriptors preserving constraints, presence, unions, and overloads. Runtime
   calls consume those descriptors without parsing signature strings. The same
   descriptors drive generated documentation.
2. **Shared value binding.** Bind a concrete value to a target descriptor by
   preserving an exact match, producing a validated conversion, or returning a
   structured failure. Call binding adds arity, overload selection, and
   function/parameter context around that value-binding operation.
3. **Passive type compatibility.** Reason over source and target descriptors
   using the shared conversion rules. Expose both `may_bind` and the richer
   classification, plus whole-call analysis for DMLS. This part must never
   invoke a function to discover whether an input works.

Implement the concrete binder before building broad static inference. Static
analysis may conservatively return category 3 for difficult constraint
implication or overlapping structured unions; runtime binding must establish a
definitive result or return an explicit resource-limit error. The first version
does not need a general solver for arbitrary schema constraints.

Optimize the common single-signature, primitive-parameter path first. Preserve
exact values without cloning where ownership permits, and allocate converted
values only when needed. Precompiled descriptors and cached type relations take
priority over concurrent batch execution. Measure before adding concurrency.

`BoundArgs` does not itself prove that a handler's accessor agrees with its
descriptor. Validate registration structure before serving calls, and cover
accessor/descriptor agreement through dispatch conformance tests. Treat an
accessor mismatch as an implementation defect. Generated typed adapters can
strengthen this boundary after the contract settles; they are not a prerequisite
for the first implementation.

### Suggested Implementation Order

_Updated per 2026-09-25 clarification round 4: every catalog function migrates
in one pass (spec decision 52). This slice, formerly a pilot delivered before
a staged rollout, is now only a suggested order of work inside that pass; it
is not a stopping point or a separate delivery._

Build the design first around a small set of real functions, then carry the
same pass through the rest of the catalog:

| Function | What it proves |
|---|---|
| `min` | Ordinary numeric binding, exact-value preservation, numeric-string conversion, and invalid-input rejection before invocation. |
| `is_number` | An `any` parameter reaches the handler unchanged; inspection and predicate narrowing do not accidentally become coercion. |
| `number(value, fallback?)` | Explicit conversion remains a domain operation using the shared converter; omission, fallback binding, and domain failure remain distinct. |

Include a constrained parameter and an overload in this slice as well. Use the
integer-count arm of `pr_list` to exercise both its constrained count and its
object alternative once their contracts are settled. Test binding passively
without performing provider requests. This avoids inventing overlapping
overloads solely to demonstrate the machinery.

Carry the slice through authored grammar, compiled descriptors, normal dispatch,
structured diagnostics, and descriptor-derived documentation. (DMLS call
checking ships in the same spec and follows once the compatibility API exists;
updated per 2026-09-25 clarification round 5.)
Numeric precision and nullable contracts are settled (2026-09-25 use-case
review), and frontmatter validation moves onto the same engine in this slice
(updated per 2026-09-25 clarification round 3); `number()` failure behavior is a per-function item resolved
within the spec's migration. Do not preserve legacy quirks by
adding binder exceptions.

Once the slice shows the interfaces fit together (runtime and literal static
checks agree, invalid calls never invoke handlers, exact inspection remains
intact, constrained and overloaded calls resolve correctly), continue the same
pass across the remaining catalog. The migration is not done, and does not
merge, until every function has moved. The acceptance criteria below govern
the full migration.

### Catalog-Wide Migration

The executable descriptor must derive from the authored grammar and retain
parameter names, constraints, optional/rest status, unions, overloads, returns,
and fallibility. Register handlers by stable function/signature identity without
restating the signature in a second handwritten dispatch table. Reject malformed
signatures and missing/duplicate registrations before serving calls.

Generate global coercion documentation from rule descriptors and function
reference entries from compiled signatures. Authored descriptions explain only
domain semantics not already expressed by those contracts.

Migration should implement the chosen contract, not first encode every legacy
quirk as a permanent union or strict mode. _All steps below happen in one
pass over every catalog function (updated per 2026-09-25 clarification round 4)._

1. Reconcile every catalog entry against the behavior inventory. Record each
   behavior in the behavior-change ledger as preserved or intentionally
   changed, with a reason for each change; nothing is left awaiting a ruling.
   The human reviews the ledger before merge.
2. Normalize grammar: `number(integer)`, `?:`, `parameters(void)`, YAML overload
   lists, and the agreed rest syntax. Replace accommodation-only `any`
   parameters with handler types. Keep `numberlike` or `boolish` only where
   the handler needs the caller's original form; otherwise use `number` or
   `boolean`.
3. Resolve the per-function items within the spec's migration: the `round`
   fallback, and `number()` failure behavior and parameter name.
4. Compile descriptors and introduce the shared engine, passive checking, and
   transactional binding. Move frontmatter validation onto the engine and
   update existing frontmatter fixtures whose expectations change because of
   the spec's listed behavior changes (updated per 2026-09-25 clarification
   round 3; there are no longer frontmatter policy differences to preserve).
5. Route every function through binding, remove its local arity/type
   coercion, and update its public description, fallibility, and examples.
   Preserve explicit domain transformations as named operations. Delete the
   old catalog once nothing reads it.
6. Switch DMLS and generated documentation to the same compiled catalog and
   compatibility APIs. Close the migration only when every provided function
   is accounted for.

The draft already needs reconciliation: `min` is marked fallible for what may
only be argument rejection; `length` mentions objects but omits them from its
signature (it becomes `length(val: string | any[] | object): number`); old `string(optional)` forms coexist with `?:`; zero-argument calls
use `parameters()`; and `round` advertises a fallback that conflicts with a
narrow numeric subject. These are catalog/spec issues, not reasons for the
binder to infer undocumented exceptions.

## Acceptance Criteria

Implementation is ready for review when these behaviors have evidence:

- Frontmatter validation and function binding produce identical keep, convert,
  and error outcomes for the same value/type pairs, and no frontmatter-only
  conversion code remains (updated per 2026-09-25 clarification round 3).
- A call such as `min("4", 5)` binds to numbers; invalid inputs produce
  function-aware binding errors and a handler-invocation counter stays zero.
- Missing and null arguments, optional/rest positions, constrained numeric
  boundaries, content strings, and nested failures obey the rules above.
- Union and overload order does not affect results. Subtype union overlaps,
  multiple exact overloads, multiple converted results, and empty containers
  have explicit fixtures.
- Binding is atomic, preserves exact inputs, and never mutates source values.
  Rebinding a successful result to its target preserves that result.
- Literal static checks and runtime binding agree. Generated source-type
  samples find no false category 5 verdicts and no failures under category 1
  (updated per 2026-09-25 use-case review); full-call checks cover overload
  ambiguity separately.
- Passive checking invokes no handlers or external effects. Lazy operands keep
  their agreed evaluation behavior.
- Every registered signature has accepted, rejected, normalized, return-contract,
  and applicable domain-error fixtures through normal dispatch. No function
  performs its own coercion. Every preserved legacy behavior or intentional
  compatibility change is recorded in the reviewed behavior-change ledger
  (updated per 2026-09-25 clarification round 4).
- `numberlike` and `boolish` deliver values exactly as sent, in function calls
  and in frontmatter (updated per 2026-09-25 clarification round 4).
- Catalog examples parse through the new grammar, generated docs reflect the
  descriptors, and DMLS consumes the library API without a duplicate matrix.
- DMLS reports rows a–e of the spec's worked example with the expected
  severities under default and strict settings, and underlines a category 5
  argument as an error exactly where composition rejects the document
  (updated per 2026-09-25 clarification round 5).
- Warm compatibility and batch benchmarks establish whether further
  optimization or concurrency is justified.

These are proposed implementation checks. This document revision changes no
runtime behavior and does not claim those checks have already passed.
