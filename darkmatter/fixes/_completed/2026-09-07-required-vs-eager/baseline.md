---
created: 2026-09-07
phase: 1
area: darkmatter
packages:
  - darkmatter
  - dmls
---

# Phase 1 Baseline — Existing Contract and Regression Surface

Recorded before any Phase 2–4 production edit. Everything below was observed on
the `feat/unifi` working tree at `2026-09-07`.

## 1. Repository state

`git status --short` reported 81 entries, all pre-existing work unrelated to
this fix except the already-landed runtime change:

- `darkmatter/lib/src/markdown/schemas/phase.rs` — `project_atom` already
  projects `Launch => property_eager && authored_required` and
  `Completion => authored_required`. The independent-axis runtime contract is
  **already implemented**; this fix does not touch it.
- `darkmatter/lib/tests/schema_phase_validation.rs` — already rewritten to the
  independent-axis expectations.
- `darkmatter/docs/inline/schema-validation.md` — its phase table is already
  correct; the surrounding explicit-`null` prose is not (Phase 4 Task 4.2).

> **Flag for Ken.** During this phase the entire worktree became *staged*
> (`git diff --cached --stat` reports 93 files, including unrelated Claudine,
> messenger, and prompts work). It was unstaged at phase start. Nothing in this
> phase ran `git add`; a hook or a concurrent session did it. Left untouched —
> unstaging would risk another session's work.

## 2. Upstream impact analysis

| Symbol | Risk | Direct | Notes |
|---|---|---|---|
| `schema_constraint_descriptors` (`lib/src/markdown/schemas/about.rs`) | **HIGH** | 6 | Matches the plan's known result exactly |
| `schema_hover_details` (`dmls/src/providers/frontmatter.rs`) | LOW | 3 | |
| `meta_schema_definition_hover_body` (`dmls/.../frontmatter.rs`) | LOW | 3 | |
| `is_required` (`dmls/.../frontmatter.rs`) | LOW | 3 | |
| `atoms_of` (`dmls/.../frontmatter.rs`) | MEDIUM | 12 | Not itself edited; the Phase 2.2 helper sits beside it |
| `type_fragment` (`lib/.../simplified/convert.rs`) | LOW | 1 | Comment-only edit |

The six direct `schema_constraint_descriptors` consumers are
`descriptor_traversal_order_is_deterministic`, `catalog_access_performs_no_capture`
(Darkmatter catalog tests), `run_about` (`md schema about`),
`constraint_completions` and `meta_schema_definition_hover_body` (DMLS), and
`eager_is_universal_in_catalog_and_round_trips` (Darkmatter phase test).

**No additional HIGH or CRITICAL result appeared.** Two findings worth carrying
into Phase 2:

1. `is_required` also feeds `shape_key_completions`, `top_level_key_completions`,
   `nested_key_completions`, and `schema_property_details`. Widening it to read
   `array_constraints` therefore changes **completion** labelling too, not just
   hover. That is the correct behavior (`file[](required)` *is* a presence
   rule), but it is a wider blast radius than "hover" implies.
2. `SchemaConstraintDescriptor::accepts_array_level()` is derived from
   `target_types`. The Phase 2.1 edit must keep the eager descriptor's
   `target_types: "all types"` or array-level constraint completion regresses.

## 3. Behavioral baseline

Focused run (`BISCUIT_TEST_FILTER='binary(schema_phase_validation) + test(/eager/)
+ test(/schema_hover/) + test(/meta_schema/) + test(/missing_required/) +
test(/required/)'`, `just test` from `darkmatter/`):

**173 tests run, 173 passed, 0 failed.**

- No assertion currently *fails*. The obsolete wording is pinned by a currently
  **green** assertion: `eager_schema_fixture_is_clean_and_catalog_driven`
  (`dmls/tests/lsp_session.rs:3658`) asserts the eager completion `detail`
  equals *"Requires the containing property at stabilized launch and
  completion. …"*. Phase 2.1 will turn it red; Phase 3.1 replaces it.
- `eager_is_universal_in_catalog_and_round_trips` reads the descriptor but only
  pins `target_types`, not the description — it survives Phase 2.1 unchanged.

### Confirmed coverage gaps

| Gap | Owner |
|---|---|
| DMLS hover eager detection reads only `atom.constraints`, so `file[](eager)` discloses nothing | Phase 2.2 / 2.3 |
| DMLS `is_required` reads only `atom.constraints`, so `file[](required)` loses its `Required` marker | Phase 2.2 |
| `schema_hover_details` (schema-bound instance hover) never mentions eager | Phase 2.3 |
| No DMLS strict-mode absence discriminator; default mode suppresses all `missing_required` | Phase 3.2 |
| No DMLS coverage for a supplied-but-invalid eager value's range | Phase 3.3 |

## 4. Darkmatter cells closed in this phase

Two acceptance cells were genuinely absent, so they were added to
`darkmatter/lib/tests/schema_phase_validation.rs` (no production code changed):

- `array_level_required_owns_presence_independently_of_array_level_eager` —
  `file[](required)`, `file[](required; eager)`, and `file(required)[]` under
  both phases. **Non-vacuity proven:** temporarily narrowing `project_atom`'s
  `authored_required` to `atom.constraints` alone makes this test the only
  failure of the 18 in the binary. Reverted immediately.
- `required_eager_is_mandatory_at_both_phases_including_explicit_null` — the
  fourth matrix cell end to end (absence, explicit `null`, wrong type, supplied
  valid) at Launch *and* Completion. This is a **characterization** test, not a
  discriminator: the retired `Launch => property_eager` rule produced the same
  verdicts for this cell. It is retained under AC8, which asks for direct tests
  of all four combinations.

Both pass against the current implementation, confirming **no runtime defect**
and that Phases 2–4 stay limited to catalog wording, DMLS metadata inspection,
tests, docs, and comments.

## 5. AC1–AC10 implementation test map

| AC | Behavior | Test | Status |
|---|---|---|---|
| AC1 | `string(eager)`, `file(eager)`, `string(required)`, `string(required; eager)` are valid | `eager_is_universal_in_catalog_and_round_trips`, `eager_controls_validation_timing_and_required_controls_presence`, `file_lazy_eager_required_matrix` (Darkmatter); `eager_schema_fixture_is_clean_and_catalog_driven` (DMLS) | Retained; DMLS fixture gains bare `file(eager)` in Phase 3.1 |
| AC2 | Strict-mode absence: eager-only clean, `required; eager` diagnosed | *new* strict-mode LSP session test | **Phase 3.2** |
| AC3 | Supplied invalid eager value → typed diagnostic at the value range | *new* LSP session test | **Phase 3.3** |
| AC4 | Hover distinction (definition + instance), eager-only omits `Required` | *new* `frontmatter.rs` unit cases + *new* LSP hover test | **Phase 2.4 / 3.4** |
| AC5 | Completion detail says absence is allowed unless `required` | `eager_schema_fixture_is_clean_and_catalog_driven` (rewritten) | **Phase 2.1 + 3.1** |
| AC6 | One wording authority | *new* unit assertion comparing hover prose to `schema_constraint_descriptors()`; grep gate at checkpoint 2 | **Phase 2.4** |
| AC7 | No DMLS phase fork | grep for `SchemaPhase` / `validate_for_phase` in `dmls/src` must stay clean | **Checkpoint 2** |
| AC8 | Four-cell matrix + null + invalid + array ownership | `file_lazy_eager_required_matrix`, `eager_array_placement_owns_items_or_property`, `array_level_required_owns_presence_independently_of_array_level_eager` (new), `required_eager_is_mandatory_at_both_phases_including_explicit_null` (new); DMLS array-placement hover | Darkmatter side **closed in Phase 1**; DMLS side Phase 3.5 |
| AC9 | Documentation parity | `shipped_schema_and_trigger_corpus_parses_passively` (passive corpus) + checkpoint-4 grep sweep | **Phase 4** |
| AC10 | Passive, headless, temp-workspace-only | Existing `no_side_effects.rs`; all new DMLS tests use `tempfile::tempdir` + `ClientFixture` | Enforced per test |

Passive shipped-artifact corpus coverage already exists
(`shipped_schema_and_trigger_corpus_parses_passively`) and the end-to-end
shipped-artifact path is `real_shipped_inline_schema_uses_normal_resolution_and_phase_path`;
neither needs a new sibling because Phases 2–4 change wording, not grammar.

## 6. Validation checkpoint 1

- [x] `string(eager)`, `file(eager)`, `string(required)`, `string(required; eager)`
      all parse today.
- [x] Eager-only absence is allowed by Darkmatter at both phases.
- [x] Intended production edits are limited to the `about.rs` eager descriptor
      plus DMLS constraint-marker inspection and the two hover bodies.
- [x] All tests run in this phase are passive: no shell execution, no remote
      access, no host-window activation, temporary workspaces only.
