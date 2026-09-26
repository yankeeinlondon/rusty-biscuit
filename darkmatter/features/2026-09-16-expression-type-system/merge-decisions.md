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

## Next decision — Missing values in ordinary functions

The existing math, string, and collection functions often return null when
given null. The newer draft signatures commonly exclude null, which would
instead reject those calls. Decide whether to require explicit null handling
by authors, retain nullable contracts in the affected functions, or retain
them only for selected function families. No decision has been recorded yet.
