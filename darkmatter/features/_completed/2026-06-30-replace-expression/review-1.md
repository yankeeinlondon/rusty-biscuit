---
ready: true
agent: codex/default
created: 2026-06-30T13:02:28
---

# Review: Replace Expression Functions

## Verdict

Production ready: **yes**.

The implementation satisfies the specification for `replace`, `replace_first`, and `replace_last`. The feature is a pure in-memory expression-engine change, so Level 1 verification is the appropriate rigor level for the user-observable behavior in this spec. No Level 2 or Level 3 coverage is required because the feature does not introduce terminal rendering, terminal input, keyboard handling, paste, mouse, IME, scrolling, or other emulator-observable behavior.

## Findings

No blocking findings.

## Verification Matrix

| Requirement | Implementation | Strongest verification | Assessment |
| --- | --- | --- | --- |
| `replace(x, find, replacement)` replaces all non-overlapping literal matches | `replace` uses `str::replace` after arity, null, and string checks in `functions.rs` | Level 1 unit tests: `replace_core_behavior`, `replace_is_case_sensitive_and_literal` | Appropriate |
| `replace_first(x, find, replacement)` replaces only the leftmost match | `replace_first` uses `str::replacen(..., 1)` | Level 1 unit tests: `replace_core_behavior`, dispatch test | Appropriate |
| `replace_last(x, find, replacement)` replaces only the rightmost match | `replace_last` uses `rfind` and UTF-8-safe slicing | Level 1 unit tests: `replace_core_behavior`, literal replacement case | Appropriate |
| Empty `find` is a no-op for all three functions | Explicit `find.is_empty()` guard returns the original subject | Level 1 unit test: `replace_empty_find_is_noop` | Appropriate |
| No match returns unchanged for all three functions | Standard replace behavior plus `rfind` `None` branch | Level 1 unit test: `replace_no_match_is_noop` | Appropriate |
| Matching is literal and case-sensitive | Uses standard string APIs, no regex or case-folding | Level 1 unit test: `replace_is_case_sensitive_and_literal` | Appropriate |
| Null propagation in every argument position | `any_null(args)` before string extraction | Level 1 unit test: `replace_null_propagates_in_every_position` | Appropriate |
| Non-string arguments error in every argument position | `require_string` for subject, find, and replacement | Level 1 unit test: `replace_type_mismatch_errors_in_every_position` | Appropriate |
| Exactly three arguments required | `require_args(..., 3)` in all handlers | Level 1 unit test: `replace_arity_errors` | Appropriate |
| Canonical names and underscore-free aliases dispatch | `PURE_FUNCTIONS` includes `replace`, `replace_first`/`replacefirst`, and `replace_last`/`replacelast` | Level 1 unit test: `replace_dispatches_by_canonical_name_and_alias` | Appropriate |
| Runtime signatures, descriptor catalog, and docs table stay in sync | Descriptor entries and generated docs table were updated | Level 1 parity tests: descriptor signature set, arity dispatchability, docs table sync | Appropriate |

## Notes

- Targeted replace-expression tests passed with `cargo nextest run --package darkmatter 'replace' --lib`; 74 tests passed, with one unrelated flaky retry in `markdown::compose::replacement::tests::test_boolean_value_coerced`.
- Catalog/docs parity passed with `cargo nextest run --package darkmatter --lib -E 'test(narrative_doc_function_table_matches_catalog) | test(descriptor_signature_set_equals_dispatchable_signature_set) | test(every_descriptor_overload_is_dispatchable_at_its_declared_arity)'`.
- Full `just test` did not complete because pre-existing layout/page tests failed early with code block/background assertions outside this feature area. I did not count those unrelated failures against this review.
