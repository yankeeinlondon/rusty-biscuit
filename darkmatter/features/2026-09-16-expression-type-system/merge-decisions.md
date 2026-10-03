# Schema-enhancement merge decisions

This record accompanies consolidation of `2026-09-21-schema-enhancements`
into `2026-09-16-expression-type-system`. The newer proposal receives a slight
preference; conflicting behavior is resolved explicitly with the author.
This is specification work, not evidence of runtime implementation.

## M1 — Shared conversion rules replace inconsistent legacy behavior

**Confirmed by the author, 2026-09-25: option 1.**

Adopt the newer shared argument-conversion rules. Functions declare what their
implementations need, and Darkmatter applies permitted conversions before
invoking them. Deliberate changes to existing behavior are documented and
tested. Do not preserve every historical conversion as a per-function
exception, or apply different rules to old and new functions.

- `min("4", 5)` accepts conversion of the string to the number four.
- `is_positive(true)` rejects a boolean rather than treating it as one.
- Inspecting predicates accepting `unknown`/`any` keep the original value:
  `is_number("4")` remains false.
- Invalid arguments do not reach the function implementation.
- Explicit conversion functions can still implement conversion as their
  documented purpose, using the shared conversion machinery.

The choice favors consistency and a simpler shared implementation over
preserving accidental differences. It supersedes blanket behavior-parity
requirements in the older charter for these approved changes only.

Missing/null handling, explicit fallback behavior, and other conflicting
contracts are separate decisions. No answer to those is implied by M1.

## M2 — Ordinary functions reject null unless their purpose requires it

**Confirmed by the author, 2026-09-26: option 1.**

Ordinary math, string, and collection operations reject null arguments rather
than silently returning null. Their declared parameter and return types must
reflect that contract. Authors handle optional values explicitly before a call,
for example `name ? lower(name) : null`.

- `lower(null)` and `min(null, 5)` fail argument validation before invocation.
- `first(null)` is invalid; `first([])` is a separate valid-input outcome whose
  existing empty-array behavior is not changed by this decision.
- Functions whose purpose includes inspecting or accepting absence retain
  explicitly declared null acceptance. Examples include `is_null`, inspecting
  predicates taking `unknown`/`any`, and the charter's `file_exists(null)` case.
- Null is not implicitly converted to zero, false, an empty string, or omission.
- Optional function arguments may be omitted; optionality does not itself
  permit a supplied null. Required argument presence remains independent of
  whether the declared value type admits null.
- Check every supplied argument. Null in one position does not suppress errors
  in another position.

M2 intentionally changes the existing null-propagating ordinary functions.
Migration must update declarations, dispatch tests, examples, and affected
shipped documents together. It does not change document-property optionality,
schema-stage null materialization, or short-circuit expression execution.

## Withdrawn question — Failed explicit numeric conversion

The author rejected the initial fallback question on 2026-09-26 because it
failed to identify the catalog and distinguish shared binding from explicit
conversion. The then-current `x: any` draft was not an approved contract.
M3 subsequently selected `numberlike`; M7 and M12–M14 settled conversion,
recovery, omission, and rounding. This withdrawn question is historical,
not an outstanding decision. See [number-contract.md](number-contract.md).

## M3 — Numberlike input and a useful fallback

**Confirmed by the author, 2026-09-26.**

The `number` function takes `numberlike`, not `any` or `string | number`.
Retain its fallback argument: recovery from conversion failure is central to
the function's utility. Keep the catalog description concise.

The draft retains `fallback?: number`; the author's requirement to retain the
argument does not by itself change it from optional to mandatory at each call.
M12 and M13 now define shared coercion recovery and the error when fallback
is omitted. Do not remove the fallback on the assumption that ordinary
argument validation makes it unreachable.

## M4 — Numeric strings share the number model's range

**Author direction, 2026-09-26.**

Numeric strings must use the same representable range as numbers, rather than
a separate conversion ceiling. Investigate integer treatment before adopting
an additional integer restriction. This does not settle omitted-fallback
behavior or authorize arbitrary-precision arithmetic.

Source review finds that schema numeric conversion already uses the JSON
number parser. Exact integer storage extends from `i64::MIN` through
`u64::MAX`; larger magnitudes can use finite `f64` storage with possible
rounding. `number(integer)` currently requires a whole numeric value, not
64-bit integer storage. The draft's additional integer-overflow rejection
was not an approved rule at this stage. M5 below settles the numeric ranges.
See `number-contract.md` for the distinct expression and suggestion-metadata
paths.

## M5 — Finite numbers and the integer upper limit

**Confirmed by the author, 2026-09-26.**

- Unconstrained `number` has the finite `f64` magnitude limit, with values
  between `-f64::MAX` and `f64::MAX`.
- The upper limit of `number(integer)` is
  18,446,744,073,709,551,615 (`u64::MAX`). The author corrected the initially
  discussed `i64::MAX` limit to `u64`. The agent incorrectly inferred that
  this also forbids negative integers; that inference is withdrawn.
- Numeric strings follow the same destination limits as native numbers.
  `"4.0"` can satisfy the integer constraint; `"4.2"` and
  values above `u64::MAX` cannot. Do not round or clamp an invalid value into
  the integer range. M16 selects `f64` storage; check the represented value against the
  mathematical upper bound, without rounding that bound up to `2^64`.

This limits larger whole-valued numbers; it does not establish an unsigned
integer type or reject negative integers. No lower bound is inferred from
the upper-limit correction. M16 subsequently selects `f64` as the default representation for all
Darkmatter numbers, including integer-constrained values.
Neither explicit-conversion fallback behavior nor a universal lossless-decimal
conversion policy is settled by this decision.

The question proposing that `is_integer(-4)` should become false was based
on the same mistaken unsigned restriction and is withdrawn. Negative whole
numbers do not create a conflict between rounding and an integer result.

## M6 — Simple large-number rules and numeric presentation

**Author direction, 2026-09-26.**

Support very large numbers where practical, avoid undue complexity in
Darkmatter, and make the rules easy to express. This principle guided the
arithmetic decision subsequently settled in M15; it does not require arbitrary precision or
specialized exact mixed-number arithmetic.

When presenting a numeric value with no fractional part, omit the decimal
suffix: `4.0` displays as `4`. Preserve the represented value and meaningful fractional digits. Formatting
must not narrow a value through an integer cast. M16 supersedes the earlier
requirement to preserve a separate exact integer representation. This is numeric presentation, not a rule
that rewrites ordinary string content or changes the value's declared type.

## M7 — Numeric representability failures belong to coercion

**Confirmed by the author, 2026-09-26.**

A number and a numeric string are subject to exactly the same representability
limit: whether they can be represented as finite `f64`. Values that cannot
produce an error in the coercion layer, before the function runs. The later M12
clarifies that `number` handles an unsuccessful coercion with a supplied valid
fallback; it supersedes this record's earlier assertion that fallback cannot
recover that error.

The agent's proposed recovery of out-of-range numeric text inside `number`
was rejected. The current `numberlike` schema's regex-only string arm is not
the intended complete coercion contract. This decision supersedes that
proposal in `number-contract.md` and `coercion-design.md`.

Do not manufacture a function-body range failure: M12 recovers the existing
shared coercion failure. Retain the fallback and the `numberlike` declaration.

## Diagnostic wording reconciliation — no new policy decision

The consolidation interview initially framed DMLS uncertainty warnings as a
new three-way policy choice. On review, the destination specification already
requires an error for a known invalid null argument, a warning for a known
concrete nullable union passed to a non-null parameter, and no blanket warning
for `unknown`. Preserve those recorded rules. The source design's prohibition
on uncertain cases becoming hard errors does not revoke the nullable warning;
its optional-hint wording is clarified accordingly. This is reconciliation of
existing requirements, not an author selection of a new warning policy.

Function result types are already part of the shared declaration contract.
Their use as arguments follows the same rules; a handler violating its declared
result type is an implementation defect. DMLS remains passive, and the library
continues to own diagnostic meaning and source locations. No CLI diagnostic
policy is reopened by this clarification.

## M8 — Darkmatter-owned built-in function catalog

**Previously settled by the author; reaffirmed 2026-09-26.**

The authoritative built-in declarations belong at
`darkmatter/schemas/partials/functions.yaml`. Claudine consumes Darkmatter's
shared definitions; Darkmatter must not depend on a Claudine-owned catalog.
Do not reopen this location choice.

The current Claudine file remains the working migration draft during this
specification review. Implementation transfers the reviewed declarations to
the authoritative location and updates consumers together. The existing
embedded `darkmatter/docs/schemas/expression-functions.yaml` remains the live
runtime catalog until the new grammar is implemented; it is then retired as
an independent authority. Neither draft syntax nor YAML parsing is evidence
that the runtime migration has occurred.

## Host predicate facts — retain the confirmed extension contract

The destination specification already confirms that hosts supply declarations
and implementations using the same contract as built-ins, including optional
true-result facts. Retain that requirement. The source design's proposed
library-owned starting point is an implementation slice, not permission to
remove host support from the completed feature. Darkmatter interprets the
facts; DMLS consumes them passively. No host analysis callbacks are introduced.

## M9 — Validate frontmatter after merging the supplied object

**Confirmed by the author, 2026-09-26.**

`validate_schema(file, obj)` loads the page, merges `obj` into its frontmatter,
then validates the combined frontmatter. It does not ignore `obj`, replace
the entire frontmatter with `obj`, or validate `obj` in isolation. The
one-argument form continues validating the page's own frontmatter.

Use the existing explicit frontmatter override merge: nested objects merge,
supplied values win conflicts, and arrays and explicit null replace the prior
value. Validate an in-memory page; do not write the merged frontmatter to the
source file or mutate caller state. This reuses the existing override behavior
rather than introducing another merge policy. File access retains its existing
authorization boundary; DMLS must not invoke the function during analysis.

Validation failures return false; file-loading and schema-processing errors
remain operation errors. Preserve the existing success case when the combined
page declares no schema. The current implementation ignores `obj`; migration
must correct it and test that the supplied object can change the result.

Required examples: a supplied required field repairs an incomplete page;
an invalid override makes a previously valid page fail; unrelated page fields
survive; an empty object gives the same result as the one-argument form; nested
objects, array replacement, and explicit null follow the existing override
rules. Repeated validation leaves the file and both inputs unchanged.

## M10 — Preserve text-based containment

**Confirmed by the author, 2026-09-26.**

`contains` retains its existing operation: a string haystack uses case-sensitive
substring search; array elements and object values are compared to the needle
using rendered text. Objects search values, not keys. `contains([4], "4")`
remains true. Do not replace this with type-sensitive equality or add separate
containment functions.

Rendering preserves string content, renders null as empty text, renders
booleans and numbers as scalar text, and renders arrays and objects as compact
JSON. M6's numeric presentation applies. The needle remains `any` because
inspecting/rendering arbitrary values is the operation. Under M1, numeric and
boolean haystacks can convert to the string arm; M2 rejects a null haystack.
Null array elements and null needles remain valid inspected values.

## M11 — Length rejects booleans

**Confirmed by the author, 2026-09-26: option 3.**

`length` accepts strings, numbers, arrays, and objects. It rejects boolean
inputs rather than returning the legacy zero or converting them to strings.
Null rejection was already settled by M2. Strings count Unicode characters,
arrays count elements, objects count keys, and numbers count characters in
their displayed text under M6. Empty strings and containers return zero.

Retain a boolean-source exclusion in the shared compiled function contract,
checked before ordinary boolean-to-string conversion and before invocation.
This is an explicitly approved exception to that conversion for `length`,
not a second legacy conversion system or a reason to alter other functions.
The parameter union alone cannot express the exclusion. All consumers must
use the same metadata: `length(true)` and `length(false)` are coercion-layer
errors at runtime and definite invalid-call diagnostics in passive analysis.
`length("true")` remains valid and returns four. Input rejection alone does
not make the handler fallible.

## M12 — Number uses its fallback whenever the input cannot be a number

**Confirmed by the author, 2026-09-26.**

A supplied valid fallback is always used when the value passed to `number`
cannot be used as a number. This covers malformed numeric text, unsupported
values, and numeric representability failures. The coercion layer still owns
the conversion rules and its errors; the explicit converter handles an
unsuccessful coercion by selecting the fallback. Invalid source values do not
reach a numeric handler, and ordinary numeric functions do not gain recovery.

Keep `parameters(x: numberlike, fallback?: number)`. Preserve the recovery
policy in shared compiled metadata so passive call analysis and execution
agree; the declaration's value type alone does not express this operation.
Validate a supplied fallback independently, even if `x` is usable. An invalid
fallback is an argument error, not permission to silently choose zero.
Missing the required `x` argument remains an arity error.

`number("pear", 7)`, `number(null, 7)`, and `number([], 7)` return 7, as does
numeric text whose magnitude cannot be represented. `number("4", 7)` returns
4. This is explicit conversion recovery, including for absence, not implicit
null propagation in ordinary functions. M13 settles behavior with no supplied
fallback. The earlier proposal to invent a separate failure
inside the numeric handler remains withdrawn.

## M13 — Failed conversion without fallback errors; one numeric-string parser

**Confirmed by the author, 2026-09-26: option 1, with shared-parser requirement.**

If `number` cannot use the supplied value as a number and no fallback is
supplied, return the coercion error. Do not silently return zero or null.
`number("pear")` errors; `number("pear", 7)` returns 7. The successful result
type stays `number`. Arity errors and invalid supplied fallbacks remain errors.

Darkmatter must expose one canonical numeric-string conversion function used
by both the coercion layer and implementations accepting `numberlike`. It owns
accepted numeric spelling, parsing, representability checks, and typed failure
reasons. No second regex/parser or independent `str::parse::<f64>()` policy
belongs in a handler. Return `f64`, following the subsequent M16 representation decision. Numeric validation and conversion must agree on the same
input, including boundary values. Reuse a parsed result when available rather
than requiring repeat parsing. Exact Rust names are implementation details.

Fixtures must exercise identical strings through the canonical routine,
ordinary numeric argument coercion, and an explicit `numberlike` consumer.
They must agree on acceptance and numeric results; only an explicitly declared
fallback policy changes how a failed result is handled.

## M14 — Round converts with fallback, then rounds

**Confirmed by the author, 2026-09-26, after reviewing the signature.**

```text
round(x: numberlike, fallback?: number) -> number
```

Use the same canonical numeric conversion and explicit recovery as `number`
(M12–M13), then round the selected numeric value to the nearest whole number.
Halfway values round away from zero. A valid supplied fallback is rounded too;
failed conversion without a fallback returns the coercion error. Every supplied
fallback is independently validated, and missing `x` remains an arity error.

- `round("3.7")` returns 4.
- `round("pear", 2.7)` returns 3.
- `round("pear")` returns a coercion error.
- `round(-2.5)` returns -3.

The return type is `number`: rounding must not introduce the integer
constraint's narrower upper limit. Preserve an already whole numeric input
without a narrowing `i64` cast. Do not remove the fallback, return it unrounded,
or create a second numeric parser.

## M15 — Simple floating-point arithmetic and comparison

**Confirmed by the author, 2026-09-26: option 1.**

Numeric calculations and numeric comparisons use `f64`. Accept normal binary
floating-point rounding, including indistinguishable adjacent integers above
`2^53`. Non-finite calculation results are errors, never saturated integers,
wrapped values, or successful infinities. Numeric equality and ordering must
agree on the represented operands. Preserve established string concatenation,
non-numeric comparisons, truthiness, and short-circuit evaluation.

This expressly extends the earlier function-only scope to numeric operator
consistency. It does not authorize a separate exact-integer arithmetic engine.

## M16 — All Darkmatter numbers use f64 by default

**Confirmed by the author, 2026-09-26.**

All numbers in Darkmatter use `f64` unless a specific, compelling reason
justifies an exception. This applies to numeric values entering through literals,
schemas, coercion, function arguments, calculations, and returned results.
`number(integer)` constrains the represented value; it does not select integer
storage. Existing JSON integer variants do not by themselves justify an
exception. Any exception must document its purpose, boundary conversions, and
observable precision behavior. No exact-integer exception is approved here.

This supersedes earlier requirements in M5, M6, and M13 to preserve exact
integer storage. The integer ceiling remains mathematically `u64::MAX`; in
`f64`, implement its upper-bound check as `value < 2^64`, not comparison against
`u64::MAX as f64` (which rounds upward). Thus text spelling `u64::MAX` rounds to
`2^64`: valid as an unconstrained number, invalid under the integer ceiling.
Constraints apply to represented values, consistently for text and numbers;
there is no separate decimal-precision validation system. Negative whole
values remain allowed unless an explicit minimum excludes them.

## Consolidation outcome

The behavior interview is closed. The consolidated `spec.md` and its
`function-contracts.md`, `number-contract.md`, and `declarations-design.md`
annexes apply M1–M16 together with the prior diagnostic, predicate, access, and
ownership decisions. `catalog-audit.md` records the source review and remaining
implementation acceptance work. The source proposal was marked historical on 2026-09-26; that marking was
reversed on 2026-09-27 because this consolidation had not seen the proposal's
2026-09-25 clarification rounds, and reconciliation is pending;
no lifecycle directories were moved and no runtime implementation is claimed.

Grammar completion, metadata field names, and catalog corrections are reviewable
specification edits. They do not imply the author individually approved every
line of the previous agent's draft. Historical withdrawn questions above are
retained only to explain superseded assertions; they are not outstanding work.
