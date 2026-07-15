---
$schema: "@.claudine/schemas/review.yaml"
ready: false
implemented: true
agent: codex/default
created: 2026-07-13T14:57:19-07:00
---

# Review 4 — Literal Expression

## Verdict

Not ready for production. The three Review 3 blockers are fixed: unresolved nested unions now
retain every arm, `suggest(...)` follows the selected or merged definition, and a valid alternate
union arm suppresses a false malformed-Expression diagnostic. The focused Level 1 suites, the
complete DMLS suite, the legacy CLI compatibility snapshots, and Clippy all pass.

One D3/acceptance-criterion-10 path still chooses an arbitrary arm. When a root `$schema` union
cannot be narrowed, properties unique to each arm remain visible, but a property shared by
multiple arms is overwritten by the last arm instead of becoming a merged property union. This
breaks value completion and the other schema-driven DMLS capabilities that consume the same
shape. In addition, acceptance criterion 8 has no direct binary regression asserting that
`md schema about` emits the new `literal` and `expression` keywords.

## Findings

### High — Unresolved root unions overwrite shared properties instead of merging them

`overlay_root_union` correctly retains distinct keys by applying every arm when
`select_literal_discriminant_arm` returns `None`. However, `overlay_arm` applies each property with
`SchemaShape::properties.insert`, so a same-named property from a later arm replaces the earlier
definition. `def_at_path_ctx` then borrows that single surviving top-level definition, affecting
value completion, hover, expression diagnostics, file navigation, and any other consumer of
`known_shape`.

For example, with root arms containing `state: literal(open)` and
`state: literal(done)`, omitting or misspelling the discriminant offers only the last arm's value;
the required merged behavior is to offer both. The same loss occurs for divergent Expression,
File, Enum, Boolean, and `suggest(...)` definitions. Arm order changes the editor behavior.

The existing root-union tests assert only distinct sibling keys. Worse, the updated
`suggest_phase1_union_selection_and_raw_schema_exclusion` expectation now locks in `arm-two` for a
shared property and describes last-declaration-wins as the shared authority. That is not the
union-preserving fallback required by D3 and acceptance criterion 10. The `overlay_root_union`
documentation says properties merge, so its comment has also drifted from the implementation;
here the specification and comment describe the intended behavior, while the code is wrong.

Build an owned merged shape for unresolved root arms using the same `merge_defs` rule introduced
for nested unions, then overlay that document shape onto base/extension properties so document
precedence is preserved without unioning the document definition with a lower-precedence
baseline. Add Level 1 cases for absent, unknown, duplicate, type-mismatched, and conflicting root
discriminants in both arm orders, including shared properties with divergent Literal,
Expression, File, and `suggest(...)` definitions. Matched roots must continue to expose only the
selected arm.

### High — `md schema about` does not directly test the two new public type entries

Acceptance criterion 8 requires `md schema about` to list both types. The library's descriptor
parity tests prove that `SimplifiedType` and the descriptor catalog contain the entries, and the
binary integration suite proves that the command renders a report. Neither test verifies the
user-facing requirement: `schema_about_lists_every_supported_type_keyword` uses a hard-coded
keyword list that omits `literal` and `expression` (and, despite its name, also omits existing
`yaml` and `json`). A renderer regression that drops either new row would remain green.

Extend the spawned-binary Level 1 test to assert `literal` and `expression` as distinct type rows,
not incidental substrings elsewhere in the report. Driving the assertion from the public
descriptor list is useful for exhaustiveness, but retain explicit checks for the two feature
keywords so the acceptance criterion remains visible.

### Low — The Review 3 comment-quality cleanup remains incomplete

The fixes add useful contract documentation around merged shapes and shared resolution, but many
new test comments still restate the fixture, branch, and assertions immediately below them. The
mixed-Expression and selected-arm suggestion tests are representative. This repeats the
HOW-narration and tautological-test-comment issue from Review 3. Keep comments that explain the
non-obvious union contract or arm-order invariant; remove prose that merely repeats test names,
literal inputs, and expected assertions.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| Literal grammar, typed values, serialization, arrays, constraints, validation, and coercion | Level 1 library acceptance tests | Appropriate; 38-test focused suite passes |
| Expression format, either-dialect parsing, coercion, pending values, and parse-only behavior | Level 1 library acceptance and DMLS no-side-effects tests | Appropriate and passing |
| Trigger matching and typed Literal equality | Level 1 matcher tests | Appropriate; focused matcher tests pass |
| Library property/root discriminant selection and validation diagnostics | Level 1 selector and validation tests | Appropriate and passing |
| Existing `md schema validate` pretty/JSON output parity | Level 1 spawned-binary byte comparisons | Appropriate; both snapshots pass |
| `md schema about` lists `literal` and `expression` | Indirect Level 1 descriptor parity plus a spawned-binary report test that omits both keywords from its assertions | Gap; the user-facing AC8 output is not directly verified |
| DMLS selected nested-arm completion, hover, diagnostics, navigation, and suggestions | Level 1 provider, diagnostic, and LSP-session tests | Appropriate and passing in both arm orders |
| DMLS unresolved nested-union merged behavior | Level 1 provider tests for keys and divergent value capabilities | Appropriate and passing for the Review 3 cases |
| DMLS unresolved root-union shared-property behavior | No conforming test; source and an integration expectation select the last arm | Gap; implementation violates D3/AC10 |
| DMLS mixed-union malformed-Expression suppression | Level 1 diagnostic tests with Enum/String alternate arms in both orders | Appropriate and passing |
| DMLS decoded-to-authored Expression ranges | Level 1 plain/quoted/escaped/multibyte tests | Appropriate and passing |
| Terminal rendering behavior | No feature-specific Level 2 evidence required | Not applicable; no rendering contract |
| Keyboard, mouse, paste, or IME behavior | No Level 3 evidence required | Not applicable; no OS-input contract |

## Verification performed for this review

- Inspected the complete specification, Reviews 1–3, the current Review 3 fixes, the schema
  library, DMLS provider/diagnostic paths, CLI tests, and GitNexus context for `known_shape` and
  its consumers.
- `cargo nextest run -p darkmatter --test schemas_literal_expression --color never`: 38 passed.
- `cargo nextest run -p darkmatter -E '<focused trigger/about filter>' --color never`: 7 passed,
  5,689 skipped.
- `cargo nextest run -p dmls --color never`: 566 passed. Of these, 563 ordinary tests count as
  Level 1 evidence; the three `level2_*` tests are not credited as Level 2 because that tier must
  run through the canonical recipe below.
- `cargo nextest run -p darkmatter-cli --test schema_about --color never`: 12 passed.
- `cargo nextest run -p darkmatter-cli --test schema_validate_baseline --color never`: 2 passed.
- `cargo clippy -p dmls -p darkmatter -p darkmatter-cli --all-targets --all-features --color never -- -D warnings`:
  passed with no warnings.
- `just test`: stopped at the non-interactive timeout after 2,390/5,585 tests passed with no
  failures; this review does not claim a complete area Level 1 run.
- `just test-l2`: the darkmatter library tier passed all 19 tests; the CLI tier reached 14 passes
  with no failures before the non-interactive timeout, so this review does not claim a complete
  area Level 2 run. No feature-specific Level 2 or Level 3 behavior exists.
- `cargo fmt --check` could not run because rustfmt is not installed for the pinned stable
  toolchain. `git diff --check` passed.
