# Coercion Design

This is the proposed function-binding contract for
`2026-09-21-schema-enhancements`. It replaces the preliminary design; it does
not describe implemented behavior. The companion `current-state-coercion.md`
is the compatibility baseline. The function catalog in
`claudine/docs/schemas/partials/functions.yaml` is a draft to reconcile with
this contract, not an authority for preserving its current broad types.

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

This design covers function arguments, including nested parameter types. It
does not change arithmetic operators, comparison operators, truthiness, or
frontmatter presence/default semantics. Share primitive conversion code with
frontmatter coercion, but audit the different binding policies explicitly.
Existing frontmatter root unions use ordered selection, for example; function
unions below do not. Sharing helpers must not silently change either policy.

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

Exact acceptance means the original value satisfies the complete target type,
including its passive constraints. Preserve that value. If it does not satisfy
the contract, try the permitted target-directed conversion and validate the
result. Failure of a same-type constraint does not permit arbitrary repair:
strings are not trimmed to satisfy a pattern and numbers are not clamped.

| Target | Mismatched source accepted for conversion | Bound representation |
|---|---|---|
| `number` | Numeric string under the grammar below | JSON number |
| `number(integer)` | Numeric string whose parsed value is integral | Integral JSON number |
| `boolean` | `"true"`, `"false"`, `"True"`, `"False"`, `"TRUE"`, `"FALSE"` | JSON boolean |
| `string` | JSON number or boolean | Canonical scalar string |
| String refinements, including `file`, dates, enums, and string literals | Only conversions explicitly inherited by that schema type | String satisfying the refinement |
| `json` / `yaml` | Non-string native JSON value, including null | Serialized content string, validated in the target format |
| Typed array or tuple | Array with compatible shape | Recursively bound elements |
| Structured object | Object with compatible declared structure | Recursively bound declared fields |
| Other targets | No additional conversion | Original value must validate |

These are direct conversion rules, not paths through a conversion graph.
Do not chain `number → string → boolean`, wrap a scalar in an array, flatten an
array, parse a string into an object, or stringify a container for an ordinary
`string` parameter. Booleans do not become numbers. Null does not become zero,
false, an empty string, or omission. Serialization to the explicit content
types `json` and `yaml` is the stated exception for containers and null.

### Numeric and Boolean Boundaries

Use the existing schema numeric shape with ASCII digits:
`^-?[0-9]+(\.[0-9]+)?$`. This accepts `"4"`, `"04"`, `"-3.5"`, and `"4.0"`.
It rejects `"+4"`, `"1e3"`, `".5"`, `"5."`, whitespace, separators, `NaN`, and
infinity. Native JSON numbers are not subject to a string-spelling rule.

Check integrality on the parsed numeric value: `"4.0"` can bind to
`number(integer)`; `"4.2"` cannot. Never truncate, round, saturate, or use a
fallback during argument conversion. Preserve native integer storage and parse
integer-shaped strings through integer paths where representable. Reject
integer overflow rather than silently switching to an imprecise float.
Fractional forms follow Darkmatter's JSON-number precision policy; reject
non-finite results and underflow to zero for a nonzero input. Before rollout,
freeze fixtures for numeric range, large integral decimals, precision, and
signed zero so both schema and function paths have the same documented limits.
This proposal does not promise arbitrary-precision decimal arithmetic.

The six boolean spellings above deliberately pin the documented schema matrix.
Do not infer arbitrary mixed-case acceptance from prose describing `boolish`
as case-insensitive. Reconcile that documentation discrepancy against the
shared recognizer during implementation and record any chosen policy change.
`"yes"`, `"no"`, `"on"`, `"off"`, `"1"`, and `"0"` are rejected.

### Refinements and Content Types

Constraints are checked against the bound value. Numeric bounds, integrality,
string patterns, and enum membership can eliminate a conversion candidate.
String literals retain the schema rule that they do not accept scalar
stringification. Numeric and boolean literals can use their primitive
conversion followed by equality validation. A matching string remains a string;
its content is never evaluated.

For `json` and `yaml`, a string is already encoded content: validate its contents
and preserve it. Do not serialize an invalid content string into a quoted
string to make it valid. A native object becomes serialized text, not a parsed
object passed to the handler. Parsing validates the content format only.
Explicit content types must have dedicated descriptors; treating them as plain
`string` would lose these rules.

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

Report nested failures with a path, such as argument `query`, `/labels/2`.
A bad element prevents the entire call from reaching its handler. Exact `any`,
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

1. If the original value satisfies any arm, preserve it. Multiple exact arms
   are harmless because a union does not select different implementations.
2. Otherwise, independently bind against each arm and discard failed candidates.
3. Group successful candidates by the resulting typed value. If there is one
   distinct result, accept it; if none, fail; if more than one, report ambiguity.

Equality here includes JSON shape and semantic numeric equality: `4` and `4.0`
are the same numeric result, but `4` and `"4"` are different. Object key order
is irrelevant. If future bound values carry semantic metadata, that metadata
must participate in result equivalence too.

Thus `"4"` stays a string for `string | number`. For
`number | number(integer)`, `"4"` becomes one numeric result even though both
arms validate it. Rejecting that overlap would make harmless subtype unions
fail. By contrast, `"true"` against `boolean | json` remains the original
string because it is already valid JSON content. Exact acceptance precedes
consideration of a converted boolean.

Do not distribute every nested union into a Cartesian product in advance.
Resolve locally and retain alternatives only where enclosing constraints need
them. If analysis cannot finish within its complexity budget, it must return
an unresolved result, never choose the first arm.

### Overload Selection

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
- reason: incompatible type, malformed conversion, out-of-range conversion,
  violated constraint, or ambiguous union conversion; and
- candidate diagnostics when overload resolution found no valid signature.

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
| `round` | Propose `number → number`; malformed inputs fail binding. The existing conversion fallback needs an explicit compatibility ruling. |
| `number(value, fallback?)` | Accept `any` for the conversion subject and `number` for a supplied fallback. Conversion is the operation; reuse the shared value converter. |
| `contains`, `length` | Declare their actual semantic input shapes, including `any` if every JSON shape has intended behavior. Do not narrow solely for appearance. |
| List renderers | Accept `any[]` when rendering heterogeneous elements is the operation; rendering is not argument coercion. |
| `ensure_leading`, `ensure_trailing` | Retain a string/number union if type-preserving concatenation remains intended behavior. |
| Provider identifiers | Declare meaningful numeric/string alternatives and their passive constraints; contextual provider lookup remains domain logic. |

For the proposed `number` operation, a failed conversion uses a valid supplied
fallback; without one it returns a domain error. A malformed supplied fallback
fails binding even if the primary value would convert. This removes the current
silent-zero behavior and must be called out as a compatibility change. Mark a
single signature with an optional fallback `fallible` because some valid calls
can fail; use separate arity overloads only if precise per-overload fallibility
is useful.

Null propagation is also domain behavior, not a global shortcut. If retained,
declare nullable parameters and returns, bind all arguments, then apply the
function's null rule. A null argument must not hide another argument's invalid
type. This intentionally differs from implementations that return null before
checking the remaining arguments.

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

The richer classification is:

| Classification | Guarantee |
|---|---|
| `Exact` | Every value in the source type satisfies the target unchanged. |
| `Guaranteed` | Every source value binds uniquely, but some require conversion. |
| `Conditional` | Success depends on the value or cannot be proven with available information. |
| `Impossible` | No value in the source type can bind successfully. |

`may_bind` is false only for `Impossible`. A caller needing proof of safety
checks for `Exact` or `Guaranteed`. A concrete value check uses the runtime
rules, so literals get definitive results without invoking a function.
A source type with no inhabitants should be tracked as unreachable by the
analyzer, not treated as a possible runtime argument.

| Source → target | Classification |
|---|---|
| `number → number` | Exact |
| `number → string` | Guaranteed |
| `string → number` | Conditional |
| literal `"4" → number` | Guaranteed |
| literal `"pear" → number` | Impossible |
| `number → number(integer)` | Conditional |
| `number | null → number` | Conditional |
| `any → number` | Conditional |
| `object → number` | Impossible |

Source unions quantify over their possible values: one matching source arm is
not proof that the whole union is safe. Target unions must include the same
ambiguity rules as runtime. Container reasoning must include shape constraints:
for example, unconstrained `string[] → number[]` is conditional, and even
`object[] → number[]` can succeed for an empty array. Element incompatibility
alone is insufficient to prove the entire array type impossible.

DMLS should report a hard error only when the call is proven invalid, including
known arity errors, bad literals, and definite ambiguity. Unknown frontmatter
values and conditional coercions should not generate default error noise;
hover or an optional hint can explain that runtime binding remains necessary.
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
compiled structure and may return `Conditional` when proof exceeds a bounded
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
analysis may conservatively return `Conditional` for difficult constraint
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

### First End-to-End Slice

Prove the design with a small set of real functions before expanding across the
catalog:

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
structured diagnostics, DMLS call checking, and descriptor-derived documentation.
Resolve numeric precision, nullable contracts, and conversion fallback behavior
for these functions before migrating them. Do not preserve legacy quirks by
adding binder exceptions.

Expand to the remaining catalog only after this slice demonstrates that the
interfaces fit together: runtime and literal static checks agree, invalid calls
never invoke handlers, exact inspection remains intact, constrained and
overloaded calls resolve correctly, and conditional editor results do not
become false errors. The acceptance criteria below govern the full migration.

### Catalog-Wide Rollout

The executable descriptor must derive from the authored grammar and retain
parameter names, constraints, optional/rest status, unions, overloads, returns,
and fallibility. Register handlers by stable function/signature identity without
restating the signature in a second handwritten dispatch table. Reject malformed
signatures and missing/duplicate registrations before serving calls.

Generate global coercion documentation from rule descriptors and function
reference entries from compiled signatures. Authored descriptions explain only
domain semantics not already expressed by those contracts.

Migration should implement the chosen contract, not first encode every legacy
quirk as a permanent union or strict mode:

1. Reconcile every catalog entry against the behavior inventory. Record each
   behavior as preserved, intentionally changed, or awaiting a policy ruling.
2. Normalize grammar: `number(integer)`, `?:`, `parameters(void)`, YAML overload
   lists, and the agreed rest syntax. Replace accommodation-only `any`,
   `numberlike`, and `boolish` parameters with handler types.
3. Resolve remaining contract decisions before moving the affected function:
   null propagation, `round` defaults, explicit `number` failure behavior,
   numeric limits, string refinement inheritance, and lazy evaluation.
4. Compile descriptors and introduce shared primitive conversions, passive
   checking, and transactional binding. Keep frontmatter policy differences
   explicit and covered by existing behavior fixtures.
5. Route each migrated function through binding, remove its local arity/type
   coercion, and update its public description, fallibility, and examples in
   the same change. Preserve explicit domain transformations as named operations.
6. Switch DMLS and generated documentation to the same compiled catalog and
   compatibility APIs. Close the migration only when every provided function
   is accounted for.

The draft already needs reconciliation: `min` is marked fallible for what may
only be argument rejection; `length` mentions objects but omits them from its
signature; old `string(optional)` forms coexist with `?:`; zero-argument calls
use `parameters()`; and `round` advertises a fallback that conflicts with a
narrow numeric subject. These are catalog/spec issues, not reasons for the
binder to infer undocumented exceptions.

## Acceptance Criteria

Implementation is ready for review when these behaviors have evidence:

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
  samples find no false `Impossible` verdicts and no failures under `Exact` or
  `Guaranteed`; full-call checks cover overload ambiguity separately.
- Passive checking invokes no handlers or external effects. Lazy operands keep
  their agreed evaluation behavior.
- Every registered signature has accepted, rejected, normalized, return-contract,
  and applicable domain-error fixtures through normal dispatch. Every preserved
  legacy behavior or intentional compatibility change is traceable to a ruling.
- Catalog examples parse through the new grammar, generated docs reflect the
  descriptors, and DMLS consumes the library API without a duplicate matrix.
- Warm compatibility and batch benchmarks establish whether further
  optimization or concurrency is justified.

These are proposed implementation checks. This document revision changes no
runtime behavior and does not claim those checks have already passed.
