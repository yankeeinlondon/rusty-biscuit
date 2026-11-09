---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-yaml-locator-hardening/spec.md"
plan: "darkmatter/features/2026-11-05-yaml-locator-hardening/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages:
- simplified-schema
- darkmatter
source_files_during_phase_1:
- simplified-schema/lib/src/mapping_orders.rs
- simplified-schema/lib/src/positions.rs
- simplified-schema/lib/tests/l1/alias_positions.rs
- simplified-schema/lib/tests/l1/main.rs
- darkmatter/lib/src/markdown/compose/expression/data/locate.rs
- darkmatter/lib/src/markdown/schemas/tests/positions.rs
- darkmatter/lib/tests/l1/load_data_diagnostics.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
decisions_for_review:
- |-
  Three Darkmatter tests that the plan schedules for rewriting in Phase 2
  (Rule 18) were rewritten in Phase 1 instead, because Phase 1's change in
  simplified-schema already makes them fail, and Phase 1 must leave the
  Darkmatter suite green. Only the alias cases were touched:
  `locate.rs` `yaml_orders_by_authored_index_even_without_positions` became
  `yaml_orders_by_authored_index_and_places_an_alias`, using exactly the
  assertions the plan gives for Wave 3; the `pointer_only!` entry
  `a_yaml_alias_reports_pointers_only` became the standalone test
  `a_yaml_alias_under_a_sequence_root_is_located_at_its_token` (`/0` at 1:3,
  `/1` at 2:3); and `schemas/tests/positions.rs`
  `unsupported_yaml_keeps_validity_and_top_level_positions` (not named in
  the plan) became `an_alias_keeps_validity_and_nested_positions`, with `/n`
  now at its value (5:6) and new rows for a failure reached through the
  alias. The multi-line flow test and the `pointer_only!` macro were left for
  Phase 2, because the flow suppression is still in place and those tests are
  still true. Alternative: leave Darkmatter red between phases, which the
  phase gates forbid. If the reviewer disagrees, nothing needs to be redone;
  Phase 2 only has fewer tests left to rewrite.
- |-
  An anchored original scalar (`- &a x`) is located at its anchor (`&a`,
  column 3), not at the value text `x`. This is how the shared YAML locator
  already places anchored nodes, and the plan records the same for an
  anchored mapping (`base: &b {port: x}`). The `/base/port` acceptance case
  still points at `x`, because there the failing node sits inside the
  anchored mapping. Following the plan (no locator changes), I did not move
  an anchored scalar's start past its anchor. Alternative: narrow anchored
  scalar spans to their text in `simplified/source.rs`, which would change
  positions in documents with no alias too. Redo cost: one locator change and
  the expected column in `a_yaml_alias_under_a_sequence_root_is_located_at_its_token`.
message_to_agent: |-
  Phase 1 is done: `PositionMap::project` now projects aliases as leaves at
  their `*name` token. Darkmatter frontmatter and `load_data` already pick
  this up for alias files, so three alias-only Darkmatter tests were rewritten
  in Phase 1 (see decisions_for_review). For Phase 2, Wave 3:
  `yaml_orders_by_authored_index_and_places_an_alias` already exists in
  `locate.rs` (do not add it again); `a_yaml_alias_reports_pointers_only` is
  gone from `pointer_only!` in `load_data_diagnostics.rs` (only the
  multi-line-flow entry remains, and it goes when you remove the
  suppression). I also dropped the now-false clause "as it does a file with
  an alias" from the `has_multi_line_flow_collection` doc; the function
  itself is yours to delete. Inline SimplifiedSchema objects cannot name a
  `<<` property (`{ <<: ... }` is a parse error, "unterminated pattern key"),
  so use pattern-key contracts (`{ <string>: ... }`) for `<<` cases, as the
  plan already says. When an object has both type failures and a missing
  required key, the validator reports only the type failures, so test a
  missing child under an alias with otherwise-valid children.
  `.claude/skills/darkmatter/schema.md` still says the alignment witness
  "holds no alias"; that is Phase 3's Rule 21 edit.
---

# Implementation Log for 2026-11-05-yaml-locator-hardening (3 phases)

## Phase 1

Started and finished 2026-11-08. Every Phase 1 task in the plan is checked.

### What changed

- `simplified-schema/lib/src/mapping_orders.rs`: `pub(crate) fn pointers()`.
- `simplified-schema/lib/src/positions.rs`:
    - `matches_orders` takes `&mut SchemaValueNode` and a boundary set. An
      alias scalar (detected only by `alias_token` at the node's start) gets
      its span narrowed to the token, and its pointer is recorded.
    - `project` checks `mappings + covered_by_aliases(..) == orders.len()`.
      `covered_by_aliases` makes one pass over the indexed pointers and tests
      the whole pointer and each `/`-terminated prefix against the set. It is
      skipped when no alias was seen.
    - The Rule 9 doc fixes: `PositionMap` type doc, `PositionSource::Located`,
      the `project` `## Returns`, `byte_range`, `matches_orders`, and
      `build_position_map`.
- `darkmatter/lib/src/markdown/compose/expression/data/locate.rs`: one
  alias test rewritten and one drifted doc clause dropped (see the decisions).
- No locator change was needed (Rule 12): every new alias test passed against
  the existing `simplified/source.rs`.

### Requirement → test mapping

| Requirement | Test(s) |
|---|---|
| Control: no alias, positions as before | `alias_positions::a_document_without_an_alias_is_located_as_before` |
| Spec example 1 (`/servers/1/port` 4:12, text `x`) | `alias_positions::an_unrelated_alias_keeps_the_failing_value_located` |
| Spec example 2 (1:17 / 2:6 / 2:6, ranges `x`/`*b`, byte starts 16/24/32) | `alias_positions::every_use_of_a_failing_anchor_points_at_its_alias_token` |
| Alias to scalar, sequence, mapping; neighbor stays located | `alias_positions::aliases_to_scalars_sequences_and_mappings_keep_neighbor_positions`, `positions::tests::aliases_and_literal_merge_keys_are_projected` |
| Sequence root | `alias_positions::aliases_under_a_sequence_root_are_projected`, `positions::tests::an_alias_under_a_list_root_is_projected` (rewritten, Rule 8) |
| Flow sequence/mapping aliases, repeated uses | `alias_positions::aliases_in_flow_collections_and_repeated_uses_are_located` |
| Several failing children, missing child, unknown key under an alias | `alias_positions::a_missing_or_unknown_child_under_an_alias_points_at_the_alias` |
| Escaped keys (`/`, `~`) | `alias_positions::keys_with_slash_and_tilde_resolve_below_an_alias` |
| Literal `<<` (bare, quoted, sequence of aliases), key order kept, nothing inherited | `alias_positions::literal_merge_keys_are_ordinary_keys_located_at_their_alias` |
| Refusal: anchored key, undefined alias | `alias_positions::an_unproven_layout_keeps_the_top_level_fallback`, `positions::tests::an_unquoted_template_keeps_the_top_level_fallback` (template fixture kept, Rule 8) |
| `/a` boundary does not hide `/ab`; key-order mismatch beside an alias refuses | `alias_positions::an_alias_boundary_does_not_hide_a_mismatch_beside_it`, `positions::tests::an_alias_boundary_covers_only_whole_segments` |
| Alias range is only the token (block with trailing comment, flow) | `positions::tests::a_projected_alias_leaf_spans_only_its_token` |
| `*` in a comment, quoted, or block scalar is not an alias | `positions::tests::a_trailing_comment_and_a_quoted_or_block_star_are_not_aliases` |
| Byte columns (`é`), CRLF, nonzero `base_line` | `alias_positions::columns_count_bytes_and_lines_follow_crlf_and_the_base_line` |
| Performance: 200 aliases add no anchor search (Rule 10) | `positions::tests::projecting_many_aliases_searches_no_anchor` |
| Darkmatter consumers see the improved positions | `locate::tests::yaml_orders_by_authored_index_and_places_an_alias`, `load_data_diagnostics::a_yaml_alias_under_a_sequence_root_is_located_at_its_token`, `schemas::tests::positions::an_alias_keeps_validity_and_nested_positions` |

Boundary and fidelity: the behavior sits at simplified-schema's public API
(`build_position_map`, `PositionMap::project`, `locate_all`, `byte_range`,
with real `SimplifiedSchema` validation). These are F1 in-process L1 tests in
the consolidated `l1` binary (`alias_positions` is declared in `main.rs`).
The Darkmatter end-to-end paths (`load_data` CLI records, frontmatter
parity across lib, CLI, and DMLS, DMLS ranges) are Phase 2's Wave 4.

### Gates

- `simplified-schema`: `just test` gave 1664 passed. `just lint` was clean.
- `darkmatter`: `just test` gave 15683 passed and 10 skipped (lib, cli,
  dmls, and zed-dmls). `just lint` was clean, including the wasm32 check.
    - The first two `just test` runs failed fast on the alias tests described
      in decisions_for_review. Both pass after the rewrites.
- Dependents: `claudine` uses `PositionMap`. Its targeted
  `cargo nextest run -p claudine -E 'test(/claudine_schemas/) | test(/composition::schema/)'`
  gave 925 passed. `messenger` depends on simplified-schema but never uses
  positions, so it was not run.
- The gate-skip calculator was not consulted, because the gates were run.
- Cross-OS check: skipped. There is no OS-specific code, path handling, or
  `cfg` in this change.
- No test failed or was skipped beyond the suite's existing 10 skips.
