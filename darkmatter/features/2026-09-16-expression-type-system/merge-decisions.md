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

## Next decision — Failed explicit numeric conversion

The existing `number(value, fallback?)` returns zero when conversion fails and
no usable fallback is supplied. The newer proposal uses a valid explicit
fallback, otherwise returns an error; an invalid supplied fallback fails
argument validation even when the main value would convert. Resolve the
failure contract before migrating this function. No decision is recorded yet.
