---
phases: 5
created: 2026-05-23
start_phase: 1
source_files_during_phase_1:
  - darkmatter/lib/src/style/schema/hr.rs
  - darkmatter/lib/src/layout/types.rs
  - darkmatter/lib/src/style/apply.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/style/descriptor.rs
  - darkmatter/lib/src/style/coverage_tests.rs
  - darkmatter/lib/src/style/parse.rs
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/inline/types.rs
  - darkmatter/lib/src/markdown/block/rule_processor.rs
  - darkmatter/lib/src/markdown/block/hr_builder.rs
  - darkmatter/lib/src/style/parse.rs
  - darkmatter/lib/src/markdown/render_tree/fold.rs
  - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_3:
  - darkmatter/lib/src/style/apply.rs
  - darkmatter/lib/src/style/mod.rs
  - darkmatter/lib/src/style/parse.rs
  - darkmatter/lib/src/style/descriptor.rs
  - darkmatter/lib/src/style/coverage_tests.rs
  - darkmatter/lib/src/layout/page.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/darkmatter/SKILL.md
packages:
  - darkmatter
---

# Execution Plan - Sub-Spec #6: HR Migration

This plan executes the migration of Horizontal Rule (HR) styling from legacy top-level frontmatter and bespoke inline attributes to the structured `style.hr` frontmatter schema.

## Phase 1: Schema and Data Model

Define the core data structures for HR styling and integrate them into the existing style schema. This phase removes the `CommonStyle` flattening in favor of a specialized HR schema.

- [ ] Update `darkmatter/lib/src/style/schema/hr.rs` to define `HrKind`, `HrWeight`, and `HrAlignment` enums with correct `serde` attributes.
- [ ] Refactor `HrStyle` in `darkmatter/lib/src/style/schema/hr.rs` to remove `#[serde(flatten)] pub common: CommonStyle` and explicitly add `width`, `max_width`, `color`, `bg_color`, `alignment`, `kind`, and `weight`.
- [ ] Add `Hr` variant to `PageComponent` enum in `darkmatter/lib/src/layout/types.rs`.
- [ ] Update `StyleFrontmatter` in `darkmatter/lib/src/style/schema/mod.rs` to ensure it uses the updated `HrStyle`.
- [ ] Verify schema deserialization with new unit tests in `darkmatter/lib/src/style/schema/hr.rs`.

## Phase 2: Parser and Compatibility

Implement logic to handle legacy top-level `hr:` frontmatter and inline `style:` attributes while providing deprecation warnings.

- [ ] Modify `StyleFrontmatter` parsing logic (likely in `darkmatter/lib/src/style/parse.rs` or `mod.rs`) to detect and capture top-level `hr:` blocks from the raw YAML/TOML.
- [ ] Implement a merge mechanism that populates `StyleFrontmatter::hr` from top-level `hr:` if `style.hr` is missing or incomplete, emitting `StyleWarningKind::Deprecated`.
- [ ] Update `HorizontalRuleAttrs` in `darkmatter/lib/src/markdown/inline/types.rs` to include `kind` and `legacy_style`.
- [ ] Update `RuleProcessor` in `darkmatter/lib/src/markdown/block/rule_processor.rs` to parse both `kind` and `style` keys from inline YAML attributes.
- [ ] Implement a way to pass inline deprecation warnings (e.g., `--- { style: waves }`) from `RuleProcessor` up to the final warning reporter.

## Phase 3: Wiring and Application

Wire the new HR style properties into the style application pipeline and update descriptors.

- [ ] Implement `apply_hr_style` in `darkmatter/lib/src/style/apply.rs` (following the pattern of other `apply_*` functions).
- [ ] Implement mapping logic between `HrKind`/`HrWeight`/`HrAlignment` and `biscuit_terminal::components::horizontal_rule` types.
- [ ] Add validation in `apply_hr_style` to return `StyleApplyError` if both `width` and `max_width` are provided.
- [ ] Update `darkmatter/lib/src/style/descriptor.rs`:
    - [ ] Add `hr.weight` descriptor.
    - [ ] Set `sub_spec: 6` for all `hr.*` keys and remove `KnownButInactive` status.
    - [ ] Update `hr.alignment` validation to accept `full`.
- [ ] Advance `ACTIVE_STYLE_WIRING_SUB_SPEC` to `6` in `darkmatter/lib/src/style/mod.rs`.

## Phase 4: Rendering and Integration

Ensure the terminal and browser renderers correctly apply the migrated HR styles and handle component-level backgrounds.

- [ ] Update `darkmatter/lib/src/markdown/block/hr_builder.rs` to consume HR defaults from the `StyleFrontmatter`.
- [ ] Integrate `PageComponent::Hr` into the background/color wrapper logic in `darkmatter/lib/src/render/terminal/mod.rs` (or equivalent sub-spec #5 location).
- [ ] Ensure `HorizontalRule` rendering (terminal and browser) uses the resolved `RuleStyle`, `RuleWeight`, and `RuleAlignment`.
- [ ] Verify that `style.hr.color` correctly sets the rule stroke color.

## Phase 5: Documentation and Validation

Update documentation and perform final verification of the implementation.

- [ ] Update `darkmatter/docs/rendering/hr.md` to reflect the new canonical styling paths.
- [ ] Update `darkmatter/docs/rendering/style.md` to document `style.hr` as active and top-level `hr` as deprecated.
- [ ] Verify that `--strict-style` correctly traps deprecated HR usage.
- [ ] Run the 14-point test suite defined in the sub-spec.
- [ ] Ensure no regressions in previous sub-spec tests.
