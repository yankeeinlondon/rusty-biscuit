---
$schema: "@.claudine/schemas/review.yaml"
ready: false
implemented: true
agent: codex/default
created: 2026-07-13T09:14:49-07:00
---

# Review 2 — Literal Expression

## Verdict

Not ready for production. The Review 1 fixes are present and pass their focused Level 1 tests:
condition-dialect DMLS parsing, root-union narrowing, sparse-discriminant agreement,
Literal-plus-Expression completion, executable CLI output baselines, and stale acceptance-test
module documentation have all been addressed. The core Literal/Expression schema behavior is
also green.

Two DMLS paths still do not provide the schema-driven editor behavior required by D1/D2. A
selected nested discriminated-union arm is used for sibling-key completion but not for value
completion, hover, expression diagnostics, or navigation. Separately, a union accumulates every
Literal and Expression candidate but only the first ordinary completable arm, omitting valid
scaffolds from later Enum/Boolean/File arms. Both are user-observable completion/intelligence
requirements with no Level 1 regression and therefore remain high-severity readiness gaps.

## Findings

### High — Selected nested union arms lose value intelligence after key completion

`nested_shape_for_completion` correctly calls `discriminated_arm_shape`, so a mapping tagged for
the second inline-object arm offers that arm's sibling keys. Every value-oriented path then goes
back through `def_at_path`, which calls `nested_shape` and unconditionally chooses the first
inline-object arm. This split affects `value_completions`, `schema_hover`, `expression_values`, and
`nav_targets`.

For example, given a `change` union whose first arm is `{ kind: literal(created), path: file }` and
second arm is `{ kind: literal(deleted), when: expression, state: literal(done) }`, authoring
`kind: deleted` offers `when` and `state` as keys. Once those keys exist, however:

- `when: ctx.` receives no Expression catalog completion or Expression hover;
- a malformed `when` value emits the generic schema problem instead of the required
  `dm.expression.malformed` diagnostic;
- `state:` does not offer the exact preselected Literal value; and
- a selected-arm `file` property would not participate in file completion/navigation unless the
  first arm happened to declare the same key compatibly.

Use one context-aware schema-path resolver for both key and value operations. At each ancestor,
select the discriminated arm from the authored mapping when possible, falling back to the existing
union behavior only when selection is absent or ambiguous. Add Level 1 tests with the selected arm
in both first and second position covering Expression completion/hover/diagnostics, Literal value
completion, schema hover, and file navigation.

### High — Later non-Literal union arms do not contribute their normal completions

D2 requires a union to offer every Literal value plus the usual scaffolds for its non-Literal
arms. `value_completions` now accumulates all Literal values and Expression catalog items, but it
still calls `completable_atom`, which returns only the first Enum/Boolean/File arm. A schema such as
`[literal(auto), enum(fit, fill), boolean]` therefore offers `auto`, `fit`, and `fill`, but silently
omits `true` and `false`; reversing the ordinary arms drops the enum members instead. The same
first-arm loss applies to competing File and Enum/Boolean arms.

Accumulate the normal completion output from every non-Literal arm, then apply the existing
deduplication while preserving declaration order and Literal preselection. Add Level 1 cases that
permute Literal with Enum, Boolean/Boolish, File, and Expression arms and assert that every arm's
eligible candidates remain present without duplicates.

### Low — Expression dialect documentation still contradicts the parser

The feature specification says the value dialect treats `&&` as a syntax error, and the new
`overlay::expressions::parse` rustdoc says body interpolation rejects both `&&` and `||`. The parser
already accepts `&&` in both modes and accepts `||` in both modes with different semantics:
fallback in value/interpolation mode and logical OR in condition mode. The newly added unit test
correctly pins that behavior, so the prose is stale rather than the code being wrong.

Update the specification and rustdoc to state the actual distinction. This matters because the
stale wording led the prior review to describe `&&` as value-dialect-invalid even though the parser
and its existing tests say otherwise.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| Literal grammar, typed scalar values, round-trip serialization, arrays, and constraints | Level 1 library tests | Appropriate and passing |
| Literal validation, optionality, defaults, coercion, and mixed property unions | Level 1 library/compose tests | Appropriate and passing |
| Expression format, condition-superset validation, coercion, and type mismatch behavior | Level 1 library tests | Appropriate and passing |
| Trigger matching | Level 1 matcher tests | Appropriate |
| Library root/property discriminant narrowing, including sparse multi-key agreement | Level 1 selector and validation tests | Appropriate and passing |
| Existing `md schema validate` pretty/JSON output parity | Level 1 spawned-binary byte comparisons over seven legacy fixtures | Appropriate and passing |
| DMLS Expression completion/hover/diagnostics on top-level properties | Level 1 provider/diagnostic tests | Appropriate and passing |
| DMLS Literal completion/hover/code action on top-level properties | Level 1 provider/code-action tests | Appropriate and passing |
| DMLS root-union and nested sibling-key narrowing | Level 1 provider tests | Appropriate and passing |
| DMLS Expression/Literal intelligence inside a selected nested union arm | No direct test; source resolves values through the first arm | Gap; implementation is incorrect |
| DMLS completion across all non-Literal arms of a mixed union | No direct test; source selects only the first completable arm | Gap; implementation is incomplete |
| DMLS decoded-to-authored ranges for plain/quoted/escaped/multibyte expression scalars | Level 1 provider/diagnostic tests | Appropriate |
| No expression evaluation / DMLS passivity | Level 1 parser-path and no-side-effects tests | Appropriate |
| Terminal rendering behavior | No feature-specific L2 evidence required | Not applicable; the feature adds no terminal rendering contract |
| Keyboard/mouse/paste/IME behavior | No Level 3 evidence | Not applicable |

## Verification performed for this review

- Inspected the complete specification, Review 1, the current staged fixes, schema resolution,
  discriminant selection, DMLS provider/diagnostic paths, and focused test sources.
- `cargo nextest run -p darkmatter --test schemas_literal_expression --color never`: 38 passed.
- `cargo nextest run -p dmls -E 'test(/expression|literal|discriminant|root_union/)' --color never`:
  63 passed, 475 filtered out.
- `cargo nextest run -p darkmatter-cli --test schema_validate_baseline --color never`: 2 passed.
- `cargo clippy -p dmls -p darkmatter -p darkmatter-cli --all-targets --all-features --color never -- -D warnings`:
  passed with no warnings.
- No feature-specific Level 2 or Level 3 run was performed because Literal/Expression schema and
  LSP behavior does not depend on a real terminal emulator or OS input encoder.
