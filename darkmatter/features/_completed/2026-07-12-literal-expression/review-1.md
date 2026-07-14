---
$schema: "@.claudine/schemas/review.yaml"
ready: false
implemented: true
agent: codex/default
created: 2026-07-13T00:20:04-07:00
---

# Review 1 — Literal Expression

## Verdict

Not ready for production. The core Literal grammar, conversion, coercion, trigger matching, and
library-side union narrowing are implemented cleanly and the focused Level 1 suites pass. DMLS
does not consistently honor the accepted expression dialect, omits root-union completion
narrowing, and drops Literal completions from unions containing Expression. The shared
discriminant selector also narrows a sparse multi-discriminant union when the specification says
it must fall back to ordinary union behavior.

## Findings

### High — DMLS rejects condition-dialect expressions that schema validation accepts

The `expression` format correctly validates with `parse_condition`, so the specification's own
example `is_agent() && os == "macos"` is valid. DMLS diagnostics instead call
`overlay::expressions::parse`, which delegates to value-dialect `parse_spanned`. That parser rejects
`&&`, causing a valid Expression-typed frontmatter value to receive
`dm.expression.malformed`. Expression hover also shares the value-dialect AST path, so condition
expressions cannot receive the promised equivalent intelligence consistently.

Use one span-carrying condition-mode parse authority for Expression-typed frontmatter values and
retain value-mode parsing for body interpolation. Add DMLS completion/hover/diagnostic tests using
both `&&` and `||`; assert the same corpus accepted by the schema format is not diagnosed as
malformed by DMLS.

### High — Root discriminated-union key completion is not implemented

The specification explicitly includes a property union **or the root `$schema` union** in D3.
`known_shape` only overlays `SimplifiedSchema::Single`; it ignores
`SimplifiedSchema::Union`. Consequently, top-level key completion for a root union cannot inspect
its arms, cannot invoke `select_literal_discriminant_arm`, and cannot narrow to the matching arm.
The library validation test covers root-union diagnostic narrowing, but the DMLS tests exercise
only a nested property union despite the plan claiming root/property coverage.

Preserve the effective root union for completion, select its arm with the shared library selector,
and offer that arm's remaining keys. Add matched, absent, unknown, duplicate, typed-mismatch, and
conflicting root-discriminant DMLS tests.

### High — Sparse multi-discriminant unions can narrow without full agreement

`select_literal_discriminant_arm` skips a present discriminant key when an arm does not declare
that key. This can select an arm based on `kind: a` even when the same instance contains a second
qualifying discriminant such as `mode: unknown` that selects no arm. The specification requires
every present qualifying discriminant to select the same arm; unknown or conflicting
discriminants must preserve ordinary union behavior.

Require every present qualifying discriminant to produce one matching arm and require all selected
indices to agree. Add sparse-arm cases where discriminant keys occur in overlapping subsets of
arms, including one known plus one unknown value. Verify both library diagnostics and DMLS key
completion retain union behavior.

### High — An Expression arm suppresses Literal value completions in the same union

`value_completions` returns immediately when any union arm is Expression-typed. The Literal loop
therefore never runs for schemas such as `[literal(auto), expression]`. This contradicts D2's
requirement that every Literal arm be offered together with the normal scaffolds for non-Literal
arms. The current union test covers Literal plus Number and multiple Literal arms, but not Literal
plus Expression.

Accumulate Literal items and Expression candidates instead of returning early, then deduplicate
the merged completion list. Add the mixed-union case in both arm orders and confirm `auto` remains
preselected while catalog-backed expression candidates are also present.

### Medium — The byte-identical CLI compatibility baseline is not an executable regression

Acceptance criterion 8 requires existing `md schema validate` output to remain byte-identical.
The phase-one baseline files are referenced only by planning documents; no test reads or compares
them. The focused tests verify new behavior but cannot catch formatting or diagnostic-shape drift
for the recorded legacy cases.

Turn the captured pretty/JSON output into an integration or snapshot assertion, or replace the
artifacts with equivalent checked-in snapshots exercised by `just test`.

### Low — The feature test module still describes removed ignored scaffolds

The module documentation in `schemas_literal_expression.rs` says every test is ignored, the
feature does not exist, and `literal`/`expression` are unknown keywords. All 37 tests are now
active and passing. This is stale behavior documentation and conflicts with the repository's
comment-quality rule.

Replace the scaffold narrative with the permanent acceptance-test purpose, or remove it if the
filename and test names already carry the necessary information.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| Literal grammar, typed scalar values, canonical round-trip, arrays, constraints | Level 1 grammar/unit/integration tests | Appropriate |
| Literal validation, optionality, defaults, coercion, and mixed property unions | Level 1 library/compose tests | Appropriate |
| Expression format, either-dialect validation, coercion, and type mismatch behavior | Level 1 library tests | Appropriate for library behavior |
| Trigger matching | Level 1 matcher tests | Appropriate |
| Library discriminant narrowing | Level 1 selector and validation tests | Sparse multi-key agreement case is missing and behavior is incorrect |
| `md schema about` descriptors | Level 1 catalog tests/manual CLI exercise recorded in the plan | Appropriate for content; no real-terminal presentation is claimed |
| Existing `md schema validate` output parity | Captured files/manual comparison | Not an executable regression |
| DMLS Expression completion/hover/diagnostics | Level 1 provider and in-process LSP tests | Correct level, but condition-dialect behavior is missing/broken |
| DMLS Literal completion/hover/code action | Level 1 provider/unit tests | Correct level, but mixed Literal/Expression union completion is missing |
| DMLS discriminated-union completion/diagnostics | Level 1 provider/diagnostic tests | Correct level, but root-union and sparse multi-key cases are absent |
| No expression evaluation / DMLS passivity | Level 1 parser-path and no-side-effects tests | Appropriate level; implementation is parser-only |
| Area Level 2 gate | Real-terminal package recipe | Feature has no terminal-specific behavior; gate is repository release hygiene |
| Keyboard/mouse/paste/IME behavior | None | Not applicable; Level 3 is not required |

## Verification performed for this review

- Inspected the complete specification, execution plan, feature-only commit diff, schema engine,
  DMLS providers/diagnostics, and test sources.
- `cargo nextest run -p darkmatter --test schemas_literal_expression --color never`: 37 passed.
- `cargo nextest run -p dmls -E 'test(/expression|literal|discriminant/)' --color never`: 48
  passed, 476 filtered out.
- `just test-l2`: the Darkmatter library tier passed 19/19 real-terminal tests. The broader area
  run was terminated at the non-interactive execution limit while compiling `darkmatter-cli`, so
  the CLI and DMLS Level 2 tiers were not independently completed in this review. No failure was
  observed before termination.
- No Level 3 run was performed because the feature has no OS keyboard or mouse behavior.
