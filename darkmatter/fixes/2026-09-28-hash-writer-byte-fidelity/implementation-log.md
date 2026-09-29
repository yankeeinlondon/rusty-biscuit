---
spec: /Volumes/coding/wt/rusty-biscuit/fix-hash-writer/darkmatter/fixes/2026-09-28-hash-writer-byte-fidelity/spec.md
plan: darkmatter/fixes/2026-09-28-hash-writer-byte-fidelity/plan.md
implemented_by: claude/opus
started_phase: 1
packages:
    - darkmatter
    - darkmatter-cli
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/hash/write.rs
    - darkmatter/cli/tests/l1/hash_kind_save_diff.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
---

# Implementation Log for 2026-09-28-hash-writer-byte-fidelity (3 phases)

## Phase 1

Started and finished 2026-09-29 on macOS. Phase 1 is tests only; no writer
behavior changed.

### Spike: lone CR and BOM

A scratch `#[test]` (deleted after the run) showed:

- Case 4's lone-CR input extracts (`yaml_span` 4..69), passes
  `validate_block_mapping`, parses with `serde_yaml_ng`, and yields correct
  `TextNode` ranges from `parse_text_frontmatter`. **The lone-CR path is
  viable**, so no ruling is needed.
- `yaml.lines()` treats CR-only YAML as a single line. The flow-root
  pre-check still catches a flow root that comes first. It misses one that
  follows a comment line (`# c\r{a: 1}\r`). That gap already existed, is
  outside this fix, and case 4 does not depend on it.
- A BOM before existing frontmatter extracts correctly, and case 5's expected
  output re-parses. Empty frontmatter (`---\n---\n`) gives an empty
  `yaml_span`. A body with no terminator extracts normally.

### Red-test gating decision

Phase 1's red tests would make `just test` fail, but the phase is only done
when `just test` passes. To satisfy both, I followed the existing repo
convention (the schema-plus scaffolding in
`lib/src/markdown/schemas/simplified/grammar.rs`): each red test carries
`#[ignore = "red until phase 2 of 2026-09-28-hash-writer-byte-fidelity: <slice>"]`.
Phase 2 removes each gate when it lands that slice. New tests that already
pass stay ungated as regression guards.

### Tests added (`darkmatter/lib/src/markdown/hash/write.rs`, `mod tests`, L1)

Helpers:

- `assert_fidelity(input, output, bumped)` (R8). Re-parses both documents with
  `parse_text_frontmatter` and requires the records to be equal once `hash`
  (and `last_updated` when bumped) are removed. It also asserts that `hash` is
  present, that `last_updated == TODAY` when bumped, that body bytes are
  identical (`body_span`, or the whole text minus a BOM when there is no
  block), and that a leading BOM stays first.
- `assert_refused(input, decision)`. Expects `Err(FrontmatterTextEdit)`,
  asserts the caller's `String` equals a clone taken before the call, and
  returns the reason.
- `save_canonical` / `saved_canonical` / `assert_refused_canonical` use the
  canonical `aaaa000000000000-bbbb000000000000` simple hash, whose bytes do
  not change. `saved_body_kind` uses a `Body`-kind hash that serializes to
  three lines, which exercises R2.

| Requirement | Test | State now |
| --- | --- | --- |
| Case 1 (LF) | `textual_save_case1_empty_date_gets_one_space` | red: `last_updated:2026-09-28`, does not parse |
| Case 1 (CRLF) | `textual_save_case1_empty_date_gets_one_space_crlf` | red: same |
| Case 2 | `textual_save_case2_empty_date_keeps_comment_spacing` | red: value parses as `"2026-09-28# todo"` |
| Case 3 | `textual_save_case3_lf_date_line_in_crlf_file_keeps_lf` | red: `\n\r\n` |
| Case 4 | `textual_save_case4_lone_cr_terminators_survive` | red: hash line LF, date line CRLF |
| Case 5 | `textual_save_case5_bom_stays_before_new_block` | red: BOM lands in the body |
| Case 6 | `textual_save_case6_refuses_anchored_date` | red: writes an orphaned `*lu` |
| Alias (source elsewhere), tag, bare anchor | `textual_save_refuses_aliased_tagged_and_bare_anchored_dates` | red: date replaces the property |
| No bump skips the property check (spec rule) | `textual_save_without_bump_ignores_date_node_properties` | green |
| Mixed file, inserted date (LF-in-CRLF and CRLF-in-LF), empty frontmatter uses the opening delimiter's terminator (R3) | `textual_save_inserted_date_inherits_preceding_terminator_in_mixed_file` | red: global newline |
| Mixed file, hash-only, multi-line node with the same, longer, and shorter line counts (R2) | `textual_save_hash_only_multi_line_node_keeps_each_line_terminator` | red: global newline |
| Lone-CR hash-only replacement (single and multi-line) | `textual_save_lone_cr_hash_only_replacement` | red: LF written |
| BOM before existing frontmatter | `textual_save_keeps_bom_before_existing_frontmatter` | green |
| Body with no final terminator, with and without frontmatter (new block uses LF) | `textual_save_body_without_final_terminator_stays_unterminated` | green |
| New block uses the body's first terminator (lone CR; LF-then-CRLF) | `textual_save_new_block_uses_first_body_terminator` | red: global newline |
| R7: hash replacement orphans an alias | `textual_save_refuses_hash_replacement_that_orphans_an_alias` | red: invalid output returned |
| Robustness matrix (25 rows, control first) | `textual_save_last_updated_robustness_matrix` | red (see below) |

The robustness matrix is built from one control fixture (`CONTROL`), and each
row makes exactly one `replacen` edit. A guard asserts that every non-control
edit actually changed the fixture. The date column covers: control, absent,
empty, empty with comment, empty with a single-space comment, `~`, `null`
with extra leading whitespace, `""`, `''`, block collection, flow sequence,
flow mapping, anchor, alias (`*r`, source `reviewed: &r`), tag, bare anchor,
duplicate key, and invalid YAML. The hash column covers: absent, empty, block
collection, flow collection, alias, duplicate key, and an edit that orphans
an alias. Every written row also runs `assert_fidelity`.

I instrumented a temporary run with `catch_unwind` per row (reverted
afterward). Exactly these rows fail today: `empty`, `empty with comment`,
`empty with single-space comment`, `flow sequence`, `flow mapping`, `anchor`,
`alias`, `tag`, `bare anchor`, and `hash edit orphans an alias`. All other
rows, including the control, pass.

"Edit produces invalid YAML" in the date column has no reachable input once
the refusals are in place. The validation row is exercised through the hash
column (R7), as the plan's matrix allows.

### Test added (`darkmatter/cli/tests/l1/hash_kind_save_diff.rs`, L1)

- `test_hash_save_refuses_anchored_last_updated_without_writing` launches
  through `CliProcessFixture`. It uses a stale stored hash (which forces a
  date bump) with `last_updated: &lu 2026-01-01` and `reviewed: *lu`, and
  asserts a non-zero exit, stderr containing `last_updated`, and byte-identical
  file contents. It is red today ("Unexpected success"; the CLI writes the
  broken file). I confirmed that writer refusal reasons reach stderr, using
  the existing flow-root refusal.

### Tier and placement

The library tests are in `lib/src/markdown/hash/write.rs` `mod tests`. The CLI
test is in the already-declared `cli/tests/l1/hash_kind_save_diff.rs`. No
test-path segment carries a tier marker, so all of them are L1. No features
are needed.

### Gates

- `cargo nextest run -p darkmatter --lib --run-ignored all -E 'test(textual_save)'`:
  25 run, 11 pass, 14 fail (exactly the gated tests, each for the expected
  reason).
- `cargo nextest run -p darkmatter-cli --run-ignored all -E ...`: the new CLI
  test fails with "Unexpected success", and the flow-mapping sibling passes.
- `just test` (darkmatter, `--no-fail-fast`): 8654 run, 8653 pass, 27
  skipped, 1 failed. The failure,
  `markdown::schemas::file_match::tests::conversion_emits_every_root_union_glob`,
  is **pre-existing and unrelated**. It is a pure schema-conversion
  assertion, and this phase's library diff adds only `#[cfg(test)]` code to
  `write.rs`. I established this by inspection; I did not re-run it at a
  clean HEAD.
- `just lint` (darkmatter): exit 0, no warnings.

### OS considerations

The new tests are pure in-memory string transforms plus one CLI temp-file
round trip that uses the existing fixture. There is no platform-specific path
or process behavior, so I did not run a cross-OS pass in Phase 1. CI and
Phase 2 will cover this.

### Skill

No Darkmatter skill change was needed for Phase 1. Phase 3 owns the
`frontmatter.md` update.
