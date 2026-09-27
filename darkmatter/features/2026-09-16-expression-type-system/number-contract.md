# Numeric values, conversion, and presentation

This is the consolidated numeric contract for the expression type system,
reflecting M3–M7 and M12–M16 in [merge-decisions.md](merge-decisions.md).
It specifies intended behavior; runtime migration remains unimplemented.

## Representation and limits

All Darkmatter numbers use `f64` unless a compelling reason justifies an
explicit exception. This includes numeric literals, schema values, converted
numeric strings, function arguments and results, calculations, and numeric
comparisons. Integer constraints do not select a different storage type.
Existing JSON integer variants are not a reason to preserve a second numeric
model. Any exception must document its purpose and conversion boundaries;
none is approved by this specification.

A `number` must be finite: its magnitude cannot exceed `f64::MAX`. Conversion
uses normal binary floating-point rounding, including underflow to zero.
Representability does not mean exact decimal preservation. There is no
arbitrary-precision decimal or exact-integer calculation subsystem.

`number(integer)` additionally requires the represented value to have no
fractional part and to be mathematically at most `u64::MAX`. The upper bound
does not prohibit negative values. Apply any explicit `min` or `max` constraints
as well. Do not truncate, saturate, or clamp a value to make it pass validation.

For finite `f64` values, test that ceiling as `value < 2^64`.
`u64::MAX as f64` rounds up to `2^64` and is not a valid inclusive upper bound.
The greatest admissible positive float is `18446744073709549568`. Text spelling
`18446744073709551615` rounds to `2^64`: valid as `number`, invalid as
`number(integer)`. This is the same rule for text and native numbers.
Constraints inspect represented values, not a separate exact decimal original.

## One numeric-string conversion function

Darkmatter exposes one pure canonical numeric-string conversion function,
returning `f64` or a typed conversion failure. The coercion layer, functions
accepting `numberlike`, and passive literal checks use it. It owns spelling,
parsing, finite-range checks, and failure reasons. Reuse a parsed result when
available; no handler maintains another regex or independent parsing policy.
Exact Rust naming is an implementation detail.

Accepted text uses ASCII decimal syntax `^-?[0-9]+(\.[0-9]+)?$`.
Examples: `"4"`, `"04"`, `"-3.5"`, `"4.0"`. Leading plus, exponent notation,
leading/trailing decimal points, whitespace, separators, `NaN`, and infinity
are not accepted numeric strings. Native numeric values have no text-spelling
restriction. Booleans, null, arrays, and objects are not numeric conversions.

`numberlike` retains the number-or-numeric-text declaration, with full finite
range validation rather than regex-only acceptance. A consumer needing a
numeric value uses the canonical converter. Ordinary handlers needing a number
declare `number` and receive the binder's numeric result.

## Explicit conversion and recovery

```text
number(x: numberlike, fallback?: number) -> number
round(x: numberlike, fallback?: number) -> number
```

For both functions, shared compiled recovery metadata selects a valid supplied
fallback whenever conversion of `x` fails, including unsupported source types
and numeric range errors. The failure still originates in coercion. Invalid
raw inputs do not reach a numeric handler. Without a fallback, propagate the
coercion error; do not return zero or null. Validate a supplied fallback even
when `x` succeeds. Missing `x` remains an arity error.

`number` returns the selected numeric value. `round` rounds it to the nearest
whole number, with halfway values away from zero. A selected fallback is also
rounded. Both return `number`, so rounding introduces no integer ceiling.
Input rejection alone does not make a handler `fallible`; neither operation
has an additional domain failure after successful binding.

| Call | Required result |
|---|---|
| `number("4")`, `number("4", 7)` | 4 |
| `number("pear", 7)`, `number(null, 7)`, `number([], 7)` | 7 |
| `number("pear")` | Coercion error |
| `number("4", "pear")` | Invalid fallback error |
| `number()` | Missing required argument error |
| `round("3.7")` | 4 |
| `round("pear", 2.7)` | 3 |
| `round("pear")` | Coercion error |
| `round(-2.5)` | -3 |

DMLS checks the complete call, including recovery. A statically unusable subject
with a valid fallback is not an invalid call. Ordinary numeric functions retain
their coercion errors and do not inherit this recovery policy.

## Calculation and display

Numeric arithmetic and comparisons use `f64`. All six numeric equality and
ordering operators must agree on the represented operands. Non-finite results
are errors. Existing division/remainder-by-zero errors remain. String
concatenation, non-numeric comparisons, truthiness, and lazy evaluation retain
their established contracts.

Integers through `2^53` can be represented exactly; beyond that, adjacent
integers may become indistinguishable. For example, `2^53 + 1` evaluates to
`2^53` in `f64`. That documented rounding is accepted; saturating casts through
`i64` are not. An integer-constrained operand does not constrain every result:
`1 / 2` produces the number `0.5`.

Numeric display omits a redundant decimal suffix: `4.0` displays as `4`,
`-4.0` as `-4`, and `4.5` as `4.5`. Formatting preserves the represented value
without narrowing integer casts. Ordinary string `"4.0"` stays unchanged.
Display cannot recover precision already lost during parsing or calculation.

## Implementation acceptance cases

These are required future checks, not executed runtime test results.

- Exercise identical decimal strings through the canonical parser, ordinary
  coercion, and explicit `numberlike` consumers. Acceptance and numeric results
  agree; only declared recovery changes failure handling.
- Cover zero, signed zero, negative whole values, fractions, leading zeros,
  non-ASCII digits, malformed strings, values around `2^53`, finite boundaries,
  overflow, and underflow. No special text-only range applies.
- Integer validation accepts `4.0`, `-1`, and `18446744073709549568`; rejects
  `4.2` and `18446744073709551616`. Text spelling `u64::MAX` rounds to the latter
  value and is rejected by that constraint, not by unconstrained conversion.
- A decimal string of 400 nines causes an ordinary numeric coercion error;
  `number` and `round` select a valid supplied fallback for that same failure.
- Numeric equality and ordering agree on rounded large values. Calculation
  overflow errors instead of producing null, infinity, a wrapped value, or an
  integer endpoint. Whole-valued display has no redundant `.0`.
- Test literals, variable inputs, nested numeric values, schemas, and function
  results through normal entry points, not only the helper. Numeric
  normalization belongs to value ingestion, not mutation by passive analysis.

## Historical implementation evidence — inspected 2026-09-26

The live catalog is `darkmatter/docs/schemas/expression-functions.yaml`; the
review draft is `claudine/docs/schemas/partials/functions.yaml`. Neither draft
syntax nor this document proves the replacement binder is implemented.

Source inspection under `darkmatter/lib/src/markdown` found:

| Surface | Existing behavior to migrate or preserve deliberately |
|---|---|
| `schemas/coerce.rs`, `coerce_to_number` | ASCII decimal recognition followed by `serde_json::Number` parsing; integer and floating variants are retained. |
| `schemas/simplified/convert.rs`, `numberlike_fragment` | A number or regex-only numeric string; the string arm lacks finite-range validation. |
| `schemas/simplified/convert.rs`, `number_fragment` | Integer validation lacks the selected implicit upper bound. |
| `compose/expression/functions/mod.rs`, `number_fn`, `round_fn` | Accept booleans numerically, default failed conversion to zero, and cast whole results through `i64`. These behaviors are replaced. |
| `compose/expression/mod.rs` | Arithmetic and ordering already use `f64`; equality uses rendered text, and literal/display paths include narrowing integer casts. Numeric paths must be consistent. |
| `compose/expression/functions/mod.rs`, `is_integer` | Inspects original numeric values for integrality; negative whole values are valid. It is not a storage-width test. |
| `schemas/simplified/grammar.rs`, `interpret_suggestion_parts` | Numeric suggestions have a stricter canonical decimal round-trip check. Audit metadata parsing under the common representation policy; do not accidentally retain a second number model. |

The inspected dependency graph used `serde_json` 1.0.149 with `float_roundtrip`
and without `arbitrary_precision`, and `jsonschema` 0.55.0. Existing JSON
storage supports exact `i64`/`u64` variants plus finite floats. That is historical
evidence, superseded by M16's representation decision, not an approved exception.
