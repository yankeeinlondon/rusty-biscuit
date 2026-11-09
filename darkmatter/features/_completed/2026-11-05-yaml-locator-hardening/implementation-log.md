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
source_files_during_phase_2:
- darkmatter/lib/src/markdown/compose/expression/data/locate.rs
- darkmatter/lib/src/markdown/compose/report_diagnostics.rs
- darkmatter/lib/tests/l1/load_data_diagnostics.rs
- darkmatter/lib/tests/l1/load_data_execution.rs
- darkmatter/lib/tests/l1/yaml_document_loading.rs
- darkmatter/lib/tests/position_fixtures/mod.rs
- darkmatter/cli/tests/l1/load_data.rs
- darkmatter/cli/tests/l1/diagnostic_positions.rs
- darkmatter/dmls/src/diagnostics/frontmatter/position_parity_tests.rs
- darkmatter/dmls/tests/l1/yaml_document_editor.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
- .claude/skills/darkmatter/expressions.md
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
- |-
  The merge-key hint is added where `load_data` contract-mismatch records
  get their messages, `compose/report_diagnostics.rs` (`contract_records`),
  not under `compose/expression/data/` as the plan's Rule 5a says: that
  directory only raises the typed error, and every record's message is
  built in `report_diagnostics.rs`. It fires only for a YAML data file, when
  the problem is an unexpected property whose last path segment is exactly
  `<<` (so plain and quoted spellings alike, decided on the decoded key, not
  the source text). The engine's detail has no final period, so the record
  reads "...; unexpected property. YAML merge keys are not applied, so `<<`
  is read as an ordinary key." Alternatives: also hint for JSON/TOML files
  with a `"<<"` key (rejected: the sentence talks about YAML merge keys,
  which those formats do not have), or move message building into the data
  module (a refactor with no behavior gain). Redo cost if the reviewer wants
  other wording or scope: the `MERGE_KEY_HINT` constant and its guard, plus
  `load_data_diagnostics::an_unexpected_merge_key_explains_that_merge_keys_are_not_applied`.
- |-
  The plan's "missing child under an alias" case put the missing
  `extra` key beside a failing `port` under the same aliased object
  (`/one/extra` sorting after `/one/port`). The validator reports only the
  type failure when an object has both a type failure and a missing
  required key, so that case cannot produce `/one/extra`. The test
  (`a_missing_child_under_an_alias_is_located_at_the_alias`) uses
  conforming data (`base: &b {port: 1}`) so the only record is
  `/one/extra`, missing, at the alias (2:6). Ordering of a missing member
  after its present siblings is already covered by
  `a_missing_member_follows_the_present_members_of_its_parent`. Redo cost:
  none unless the validator changes to report both.
- |-
  The DMLS parity test (`position_parity_tests.rs`) matched each expected
  problem to a DMLS diagnostic by code and message. The new `alias_use`
  case has three problems with the same message (`"x" is not of type
  "number"`), so the match was ambiguous. The match now also requires the
  diagnostic to start at the library's byte offset, which keeps the test
  one-to-one; the recorded range text (`x`, `*b`, `*b`) is still asserted.
  Alternative: give each alias use a different failing value, which would
  stop the case from matching the specification's example. The plan's DMLS
  test also called its diagnostics "kind_check-graded"; they are
  `dm.schema.type_mismatch`, which is what the test asserts. Its Unicode
  probe put `é` on a line before the alias, which cannot tell UTF-16 from
  byte columns, so a third variant writes `é` earlier on the alias's own
  line (`platforms: [{name: é}, &p {name: [1]}, *p]`).
- |-
  The Rule 19 fallback fixture `&k a: 1\nb: 2\n` against
  `{ a: number, b: string }` does not fail: the number `2` converts to the
  string "2". The test uses `b: x` against `{ a: number, b: number }`,
  which keeps the same unproven layout (an anchored key) and a single
  mismatch at `/b`, reported with no line or column.
- |-
  The plan asked for the alias, flow, and merge-key text to replace the
  "YAML positions that cannot be proven" section of the two Darkmatter docs.
  I put that material in a new section, "YAML anchors, aliases, and merge
  keys", just before it, and kept the old heading for the remaining
  fallback (an anchored key), because located aliases are no longer an
  "unproven" case and the remaining fallback still belongs under that name. `loading-data.md` also
  gained a "YAML merge keys" row in its format-behavior table. Alternative:
  one combined section under the old heading, which reads oddly since most
  of it describes what is now located. Redo cost: merging or renaming two
  headings in two docs.
source_files_during_phase_3:
- darkmatter/lib/src/markdown/compose/expression/data/parse.rs
- darkmatter/lib/src/markdown/schemas/yaml_documents/marks.rs
docs_updated_during_phase_3:
- darkmatter/docs/cli/data-contracts.md
- darkmatter/docs/topics/loading-data.md
- simplified-schema/docs/topics/diagnostics.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
- .claude/skills/darkmatter/schema.md
source_code:
- simplified-schema/lib/src/mapping_orders.rs
- simplified-schema/lib/src/positions.rs
- simplified-schema/lib/tests/l1/alias_positions.rs
- simplified-schema/lib/tests/l1/main.rs
- darkmatter/lib/src/markdown/compose/expression/data/locate.rs
- darkmatter/lib/src/markdown/compose/expression/data/parse.rs
- darkmatter/lib/src/markdown/compose/report_diagnostics.rs
- darkmatter/lib/src/markdown/schemas/tests/positions.rs
- darkmatter/lib/src/markdown/schemas/yaml_documents/marks.rs
- darkmatter/lib/tests/l1/load_data_diagnostics.rs
- darkmatter/lib/tests/l1/load_data_execution.rs
- darkmatter/lib/tests/l1/yaml_document_loading.rs
- darkmatter/lib/tests/position_fixtures/mod.rs
- darkmatter/cli/tests/l1/load_data.rs
- darkmatter/cli/tests/l1/diagnostic_positions.rs
- darkmatter/dmls/src/diagnostics/frontmatter/position_parity_tests.rs
- darkmatter/dmls/tests/l1/yaml_document_editor.rs
documentation:
- darkmatter/docs/cli/data-contracts.md
- darkmatter/docs/topics/loading-data.md
- simplified-schema/docs/topics/diagnostics.md
- .claude/skills/darkmatter/expressions.md
- .claude/skills/darkmatter/schema.md
completed_phase: 3
implemented: true
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

## Phase 2

Started and finished 2026-11-08. Every Phase 2 task in the plan is checked.

### What changed

- `lib/src/markdown/compose/expression/data/locate.rs` (Rules 13, 14):
  `has_multi_line_flow_collection` and the `SchemaValueKind` import are
  deleted; `Format::Yaml` holds a plain `PositionMap` (`PositionMap::new()`
  when the file did not load, whose `byte_range` is `None`, as the old
  `None` was). The module and variant docs say an alias is placed at its
  token, a multi-line flow collection is located, and an unproven layout is
  pointer-only. The flow test became
  `a_multi_line_flow_collection_keeps_yaml_positions`, and
  `an_unproven_yaml_layout_is_unpositioned_but_keeps_authored_order` covers
  Rule 19 at unit level. `yaml_orders_by_authored_index_and_places_an_alias`
  was already rewritten in Phase 1.
- `lib/src/markdown/compose/report_diagnostics.rs` (Rule 5a): the merge-key
  hint, `MERGE_KEY_HINT`, appended to a YAML unexpected-property record whose
  key is `<<` (see decisions_for_review for placement and wording).
- `lib/tests/l1/load_data_execution.rs`: one drifted comment ("Aliases only
  make contract violations pointer-only") corrected; the test is unchanged.
- `.claude/skills/darkmatter/expressions.md`: the `load_data` locating bullet
  no longer says an alias or multi-line flow file reports pointers only, and
  names the merge-key hint.
- No other Darkmatter source changed (Rule 15): `just test` passed with no
  other edits.

### Requirement → test mapping

| Requirement | Test(s) |
|---|---|
| Spec example 1 (`/servers/1/port` 4:12) through `load_data` | `load_data_diagnostics::an_alias_beside_an_unrelated_failure_keeps_it_located` |
| Spec example 2 (1:17 / 2:6 / 3:6, three records in authored order) | `load_data_diagnostics::every_use_of_a_failing_anchor_is_one_record_at_its_alias` |
| Missing child under an alias keeps its pointer at the alias | `load_data_diagnostics::a_missing_child_under_an_alias_is_located_at_the_alias` |
| Several failing children of one alias share its position | `load_data_diagnostics::failing_children_of_one_alias_share_its_position_with_distinct_pointers` |
| Alias to scalar / sequence / mapping; neighbors located | `load_data_diagnostics::an_alias_to_a_{scalar,sequence,mapping}_keeps_its_neighbors_located` |
| Literal `<<` (bare, quoted), sequence of aliases | `load_data_diagnostics::a_literal_merge_key_is_an_ordinary_key_located_at_its_alias`, `a_quoted_literal_merge_key_…`, `a_sequence_of_aliases_is_located_item_by_item` |
| Merge-key hint (both spellings) and unchanged control message | `load_data_diagnostics::an_unexpected_merge_key_explains_that_merge_keys_are_not_applied` |
| Multi-line flow: objects, nested mapping, comment `# ]`, quoted `"}"`/`"]"`, alias item, key after `]` | `load_data_diagnostics::a_multi_line_flow_collection_is_located_inside_and_after_it`, `a_failing_alias_item_in_a_multi_line_flow_sequence_is_located_at_its_token`, `a_multi_line_flow_sequence_of_scalars_is_located` (replaces the `pointer_only!` entry; the macro is gone), `locate::tests::a_multi_line_flow_collection_keeps_yaml_positions` |
| Character column (not byte column) in `data_location` | `load_data_diagnostics::an_alias_position_is_a_character_column` |
| LF / CRLF / BOM / BOM+CRLF identical | `load_data_diagnostics::alias_positions_survive_line_endings_and_a_byte_order_mark` |
| Counts and order equal the hand-expanded data | `load_data_diagnostics::aliases_keep_the_records_and_order_of_the_same_data_written_out` |
| Rule 19 unproven-layout fallback (pointer only) | `load_data_diagnostics::an_unproven_yaml_layout_reports_pointers_only`, `locate::tests::an_unproven_yaml_layout_is_unpositioned_but_keeps_authored_order` |
| CLI wiring: control exit 0; alias records, schema-valid envelope, read set, no later action; `source` unchanged | `cli load_data::conforming_aliased_yaml_lets_the_later_action_run`, `an_aliased_yaml_mismatch_is_located_at_each_alias_and_runs_no_later_action` |
| CLI multi-line flow `/items/1` 3:3 | `cli load_data::a_multi_line_flow_sequence_is_located` |
| Frontmatter (lib, CLI, DMLS; LF and CRLF) | `ALIAS_USE`, `ALIAS_BESIDE_UNRELATED_FAILURE` in `position_fixtures/mod.rs`, run by lib `diagnostic_positions::*`, CLI `json_reports_each_alias_use_at_its_token{,_with_crlf_line_ends}`, `json_keeps_a_failure_beside_an_alias_located`, DMLS `position_parity_tests::*` |
| Whole-file YAML validation (control + BOM/CRLF/Unicode variants, ranges `x`/`*b`) | `yaml_document_loading::control_an_aliased_document_validates`, `an_alias_use_is_located_at_its_token` |
| DMLS alias range ends after `*p`; UTF-16 columns | `dmls yaml_document_editor::an_alias_use_is_ranged_at_its_token` |
| Malformed-input behavior unchanged | existing `load_data_diagnostics` malformed tests and `load_data_execution` `yaml_*_is_malformed` rows, unmodified and green |

Boundary and fidelity: `load_data` cases are in-process at the library's
public result (`error_diagnostics` of a real composition, F2, L1); the CLI
tests spawn the real `md` binary once per behavior to prove wiring (F3, L1,
in the consolidated `cli/tests/l1` binary); the DMLS tests run the real
provider registry over an overlay (L1). All new tests are declared in
existing `l1` binaries and carry no tier marker.

### Gates

- `darkmatter`: `just lint` clean (including the wasm32 check);
  `just test` gave 15710 passed and 10 skipped (the suite's existing skips).
- Dependents: `claudine` and `claudine-cli`, narrowed to the tests that
  compose with `load_data` or schema slots
  (`cargo nextest run -p claudine -p claudine-cli -E 'test(/composition::sequence::tests|completion::engine::tests|schema_completion::tests/)'`):
  251 passed. simplified-schema was not changed in this phase.
- The gate-skip calculator was not consulted, because the gates were run.
- Cross-OS check: skipped; no path, process, or `cfg` code changed.
- No test failed or was skipped beyond the existing 10 skips. During
  development the DMLS parity test failed on the new alias case because of
  an ambiguous message match (see decisions_for_review); it is fixed.
- Checkpoint grep: the only remaining "pointer only" hit outside
  `features/`/`fixes/` is `darkmatter/docs/cli/data-contracts.md` (Phase 3,
  Rule 21).

## Phase 3

Started and finished 2026-11-08. Documentation and verification only; no
behavior changed.

### Summary of the whole implementation (Rule 22)

- Phase 1 (simplified-schema): `PositionMap::project` treats an alias as an
  authored leaf at its `*name` token and accounts for the indexed mappings
  below it, so an alias no longer forces the top-level fallback.
- Phase 2 (darkmatter): `load_data`'s YAML locator stopped discarding the map
  for a multi-line flow collection, `Format::Yaml` holds a plain
  `PositionMap` (the `Option` is removed, Rule 13; `PositionMap::new()` when
  the file did not load), and the merge-key hint was added to an unexpected
  `<<` record of a YAML data file.
- **Intentional replacement:** the whole-file "pointer only" exception that
  `2026-11-04-loading-data` documented (any alias or multi-line flow
  collection made every contract mismatch in the file pointer-only) is
  replaced. Pointer-only now applies only to a layout the position map cannot
  prove (for example an anchored key `&k a: 1`). That spec is not edited.
- Locator defects fixed under Rule 12: none; no change to
  `simplified/source.rs` was needed in any phase.

### What changed

- `darkmatter/docs/cli/data-contracts.md`: new section "YAML anchors,
  aliases, and merge keys" (what an alias is, the three-use example with its
  1:17 / 2:6 / 3:6 table, an alias beside an unrelated failure at 4:12, a
  multi-line flow example at 3:10, merge keys not applied with the hint's
  full message); "YAML positions that cannot be proven" keeps its heading
  and now names only the unproven-layout fallback, with the
  `&k a: 1` example. The duplicate-key, second-document, non-string-key, and
  tag bullets are unchanged.
- `darkmatter/docs/topics/loading-data.md`: a shorter "YAML aliases and
  merge keys" section, the "cannot be proven" section rewritten the same way,
  and a "YAML merge keys" row in the format-behavior table.
- `simplified-schema/docs/topics/diagnostics.md`: after the `byte_range`
  paragraph, an alias (or a path below one) is reported at its `*name` token,
  with an example, and an unprovable document falls back to top-level keys
  (every problem at its top-level key's line, column 1, as
  `positions::tests::an_unquoted_template_keeps_the_top_level_fallback`
  shows).
- `.claude/skills/darkmatter/schema.md`: the alignment-witness bullet no
  longer says the tree must hold no alias; aliases are leaves at their token,
  `<<` is an ordinary key, and the fallback is for unproven layouts (anchored
  keys, unquoted `{{ }}`).
- Drift found by the Wave 6 grep and fixed (comment-only, code assumed
  right): `darkmatter/lib/src/markdown/compose/expression/data/parse.rs`
  (`positions` doc said a YAML file with an alias or a multi-line flow
  collection is never positioned) and
  `darkmatter/lib/src/markdown/schemas/yaml_documents/marks.rs` (module doc
  said `PositionMap` refuses a file with an alias).
- Re-read for drift, no change needed: the module and type docs of
  `simplified-schema/lib/src/positions.rs`, `mapping_orders.rs`, and
  `darkmatter/.../data/locate.rs` (updated in Phases 1 and 2).
- Remaining grep hits (`holds no alias`, `file with an alias`, `contains an
  alias`, `Pointer only`, `pointer-only`, `multi-line flow`) outside
  `features/`/`fixes/`, each checked true: `data-contracts.md` "Pointer only"
  (the unproven-layout bullet); `schemas/definition.md:857` (a load error is
  located "even in a file with an alias", still true); `expressions.md` skill
  (Phase 2 text, accurate); `hash/write.rs`, `frontmatter.md`,
  `dmls/src/overlay/expressions.rs`, `simplified/source.rs`,
  `yaml_scalar.rs`, `content-policy`, `claudine` schemas matrix (other
  locators or in-place editing, unrelated to `PositionMap`); `biscuit-file`
  and `biscuit-terminal` (other "pointer only" meanings); test section
  headers in `load_data_diagnostics.rs` and `cli/tests/l1/load_data.rs`
  ("…are located").

### Requirement → test mapping

Phase 3 changed no behavior: the doc and skill text, and the two
corrected comments, describe behavior already pinned by Phase 1 and Phase 2
tests (see those mappings). Each example in the new docs is the input of an
existing test:

| Doc claim | Test |
|---|---|
| Three uses: 1:17 / 2:6 / 3:6 | `load_data_diagnostics::every_use_of_a_failing_anchor_is_one_record_at_its_alias` |
| Alias beside an unrelated failure: `/servers/1/port` 4:12 | `load_data_diagnostics::an_alias_beside_an_unrelated_failure_keeps_it_located` |
| Missing member and several failures under one alias | `a_missing_child_under_an_alias_is_located_at_the_alias`, `failing_children_of_one_alias_share_its_position_with_distinct_pointers` |
| Multi-line flow, `x` on line 3 at column 10 | `a_multi_line_flow_collection_is_located_inside_and_after_it` (same line layout) |
| Merge-key hint text, YAML-only, plain and quoted | `an_unexpected_merge_key_explains_that_merge_keys_are_not_applied` |
| `/server/<</port` at the `*b` | `a_literal_merge_key_is_an_ordinary_key_located_at_its_alias`, `positions::tests::aliases_and_literal_merge_keys_are_projected` |
| `&k a: 1` then `b: x` is `/b` with no position | `an_unproven_yaml_layout_reports_pointers_only` |
| Fallback: line of the top-level key, column 1 | `positions::tests::an_unquoted_template_keeps_the_top_level_fallback` |
| `byte_range` covers only `*b` | Phase 1 `alias_positions` tests |

### Gates

- `simplified-schema`: `just test` 1454 passed, 0 skipped; `just lint` clean.
- `darkmatter`: `just test` 15710 passed (4 slow, all pre-existing
  `claudine_editor_parity` / `content_policy_baseline` DMLS tests over 5 s,
  none touched by this change), 10 skipped (the suite's existing skips);
  `just lint` clean, including the wasm32 check.
- Dependents: `claudine` and `claudine-cli`, narrowed as in Phase 2
  (`cargo nextest run -p claudine -p claudine-cli -E 'test(/composition::sequence::tests|completion::engine::tests|schema_completion::tests/)'`):
  251 passed.
- The gate-skip calculator was not consulted, because the gates were run.
- Cross-OS check: skipped; no path, process, or `cfg` code changed in any
  phase of this feature.
- No test failed during this phase.

### Terminal state

Implementation complete, ready for review. The feature directory was not
moved to `_completed`, and nothing was committed.
