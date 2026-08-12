---
$schema: "@.claudine/schemas/review.yaml"
ready: false
implemented: true
agent: codex/default
created: 2026-07-13T12:59:35-07:00
---

# Review 3 — Literal Expression

## Verdict

Not ready for production. The Review 2 fixes are present: selected nested arms now drive ordinary
value completion, hover, expression diagnostics, and file navigation, while every ordinary arm of
a top-level property union contributes its completion candidates. The focused Level 1 suites and
CLI compatibility snapshots pass.

Three DMLS union paths still violate the specification. An unresolved nested discriminated union
collapses to its first inline-object arm instead of preserving merged union behavior;
`suggest(...)` completion bypasses selected-arm resolution; and a malformed Expression diagnostic
is emitted whenever any property-union arm is Expression, even when another arm accepts the value.
These are user-observable editor-intelligence requirements with no Level 1 regressions and remain
production-readiness blockers.

## Findings

### High — Unresolved nested discriminated unions still collapse to the first arm

`nested_shape_for_completion` selects a matched arm correctly, but its absent/unknown/duplicate/
conflicting fallback calls `inline_object_shape`, which returns only the first inline-object arm.
That contradicts D3 and acceptance criterion 10: when narrowing is unavailable, completion must
retain merged union behavior rather than guess an arm.

For `{ kind: literal(created), path: string } | { kind: literal(deleted), reason: string }`, a
`change:` mapping with no `kind` (or an unknown `kind`) hides `reason` when the created arm is first.
The same first-arm choice affects value completion, hover, expression gating, and navigation for
same-named properties whose types differ between arms. The existing
`sibling_completion_without_discriminant_keeps_union_behavior` test only checks that the shared
`kind` key appears; it never asserts that `path` and `reason` from both arms remain available, and
its comment incorrectly labels first-arm behavior as union behavior.

Represent an unresolved nested shape as a merged union view rather than a borrowed first arm. Add
Level 1 cases for absent, unknown, duplicate, type-mismatched, and conflicting nested
discriminants, in both arm orders, covering sibling keys and divergent value types.

### High — `suggest(...)` completion bypasses selected-arm resolution

`value_completions` calls `suggestion_completions` before `def_at_path_ctx`. That path delegates to
the context-free `suggestions_for_path`, whose documented ancestor traversal chooses the first
inline-object arm and has no authored-instance input. Consequently, a `suggest(...)` property on a
selected second arm offers nothing, while suggestions from a non-selected first arm can leak into
the active mapping.

This violates D2's requirement to retain the usual non-Literal completion candidates and D3's
requirement that editor intelligence follow the selected arm. Route suggestion lookup through the
same context-aware property resolution as Literal, Expression, Enum, Boolean, and File completion;
prefer extending the library query with an explicit selected property definition or instance
context rather than adding a DMLS-only union algorithm. Test both arm orders plus unresolved-union
fallback behavior.

### High — Alternate valid union arms do not suppress malformed Expression diagnostics

`expression_atom` treats a property as Expression-typed when any arm is Expression, and
`expression_values` consequently sends every scalar value through `expression_diagnostics`.
Parsing failure then unconditionally emits `dm.expression.malformed`, without asking whether a
non-Expression arm accepts the value. For example, an Enum or String arm can validly accept `1 +`
while the Expression arm rejects it, yet DMLS still shows a malformed-expression warning even
though the union validates.

Honor union semantics before emitting the dedicated diagnostic: report malformed Expression only
when the value is expected to satisfy the Expression arm and no alternate arm validates it. Keep
Expression completion/hover available for mixed unions, but do not turn a valid alternate-arm
value into a false warning. Add Level 1 tests with Expression plus String/Enum in both arm orders,
including one alternate-valid malformed expression and one value invalid under every arm.

### Low — The latest fix adds implementation-narrating comments

The new `value_completions` and selected-arm test comments repeatedly narrate loop branches,
candidate lists, prior behavior, and assertions already visible in the code. This conflicts with
the repository's comment-quality rule against HOW-narration and tautological examples. Retain only
the non-obvious contracts—declaration-order preservation, Literal preselection on deduplication,
and the requirement that every value capability share one selected-arm authority.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| Literal grammar, typed values, serialization, arrays, constraints, validation, and coercion | Level 1 library tests | Appropriate and passing |
| Expression format, dialect acceptance, coercion, pending values, and passivity | Level 1 library and no-side-effects tests | Appropriate and passing |
| Trigger matching and typed Literal equality | Level 1 matcher tests | Appropriate and passing |
| Library root/property discriminant narrowing and fallback diagnostics | Level 1 selector and validation tests | Appropriate and passing |
| Existing `md schema validate` pretty/JSON output parity | Level 1 spawned-binary byte comparisons | Appropriate and passing |
| DMLS selected-arm Literal/Expression/Enum/Boolean/File completion, hover, diagnostics, and navigation | Level 1 provider/diagnostic tests | Correct level and passing for a matched arm |
| DMLS unresolved nested-union behavior | No direct Level 1 test; source chooses the first inline-object arm | Gap; implementation violates D3/AC10 |
| DMLS selected-arm `suggest(...)` completion | No direct Level 1 test; context-free query chooses the first ancestor arm | Gap; implementation bypasses selection |
| DMLS mixed-union Expression diagnostics | No direct Level 1 test; any Expression arm triggers parsing | Gap; valid alternate-arm values receive a false warning |
| DMLS decoded-to-authored expression ranges | Level 1 plain/quoted/escaped/multibyte tests | Appropriate and passing |
| Terminal rendering behavior | No feature-specific Level 2 evidence required | Not applicable; no terminal-rendering contract |
| Keyboard/mouse/paste/IME behavior | No Level 3 evidence | Not applicable |

## Verification performed for this review

- Inspected the complete specification, Reviews 1–2, the Review 2 fix commit, schema library,
  DMLS provider/diagnostic paths, GitNexus resolver context, and focused tests.
- `cargo nextest run -p darkmatter --test schemas_literal_expression --color never`: 38 passed.
- `cargo nextest run -p dmls -E 'test(/expression|literal|discriminant|root_union|selected_arm|union/)' --color never`:
  81 passed, 469 skipped by the filter.
- `cargo nextest run -p darkmatter-cli --test schema_validate_baseline --color never`: 2 passed.
- `cargo clippy -p dmls -p darkmatter -p darkmatter-cli --all-targets --all-features --color never -- -D warnings`:
  passed with no warnings after a cold-cache retry.
- No feature-specific Level 2 or Level 3 run was performed because the feature adds no terminal
  rendering or OS-input behavior. The execution plan records a prior green area Level 2 run.
