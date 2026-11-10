---
kind: log
feature: 2026-11-08-template-types
implementation_1: "2026-11-09T13:24:29-08:00"
implementation_2: "2026-11-09T14:41:17-08:00"
---

# Template types: review/fix log

## Implementation of Review Findings #1

> **started at:** 2026-11-09T13:24:29-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-08-template-types/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before repairs
- the review has no `human_in_the_loop` items and no blocked findings; all three findings are unblocked

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | Tuple examples bypass read approval and lose their defining-file base | fixed | all children verified; review CLI reproduction now exits 1 with `dm.schema.slot_approval_required` (element and spread) |
| R1-F1.1 | Returned scalar example → approval required | clean | `template_type_imports::an_example_in_every_returned_or_imported_position_is_gated` |
| R1-F1.2 | Returned array atom example → approval required | clean | same test (array case) |
| R1-F1.3 | Returned object property example → approval required | clean | same test (object case) |
| R1-F1.4 | Returned union arm example → approval required | clean | same test (union arm case) |
| R1-F1.5 | Returned pattern-key property example → approval required | clean | same test (pattern-key case) |
| R1-F1.6 | Returned fixed tuple element example → approval required | fixed | `an_example_in_a_returned_tuple_position_is_gated_and_observed`, `refusing_an_example_in_a_returned_tuple_position_refuses_the_resolution` |
| R1-F1.7 | Returned tuple spread example → approval required | fixed | same two tests (spread case) |
| R1-F1.8 | Imported scalar example → approval required | clean | `an_example_in_every_returned_or_imported_position_is_gated` (imported Scalar) |
| R1-F1.9 | Imported array example → approval required | clean | same test (imported List) |
| R1-F1.10 | Imported object property example → approval required | clean | same test (imported Obj) |
| R1-F1.11 | Imported union arm example → approval required | clean | same test (imported Arm) |
| R1-F1.12 | Imported fixed tuple element example → approval required | fixed | `refusing_an_example_in_an_imported_tuple_position_refuses_the_resolution` (Pair, recursive Tree) |
| R1-F1.13 | Imported tuple spread example → approval required | fixed | same test (Rest) |
| R1-F1.14 | Transitive imported pattern-key example → approval required | clean | `an_example_in_every_returned_or_imported_position_is_gated` (Outer → `lib/deep.yaml`) |
| R1-F1.15 | Transitive imported tuple element example → approval required | fixed | `an_example_in_a_transitively_imported_tuple_position_is_gated_from_its_own_file` |
| R1-F1.16 | Transitive imported tuple spread example → approval required | fixed | same test (spread) |
| R1-F1.17 | Imported tuple element example reads `lib/ex.yaml` | fixed | `an_example_in_an_imported_tuple_position_reads_from_the_defining_file`: outer invalid file never admitted; equals static import |
| R1-F1.18 | Imported tuple spread example reads `lib/ex.yaml` | fixed | same test (spread) |
| R1-F1.19 | Function parameter/return: no unguarded example read | clean | `an_example_a_returned_definition_cannot_lower_is_never_read` |
| R1-F1.20 | `property(...)` declaration: no unguarded example read | clean | same test (`schema(property(...))`) |
| R1-F1.21 | Final example loader consumes only admitted examples | fixed | exhaustive `TypeExpr` match in `rehome_atom_examples` gates every example-bearing position before lowering; decision (no loader-side gate) recorded in `human_review_items` |
| R1-F1.22 | Darkmatter refuses ungranted tuple example | fixed | `darkmatter/lib/tests/l1/dynamic_schema_slot_template_types.rs::an_example_in_a_returned_tuple_needs_its_own_grant`: `APPROVAL_REQUIRED`, empty read set; granted run records `ex.yaml` |
| R1-F1.23 | (new) Static imports of a tuple type resolved tuple examples from the outer schema's base | fixed | static assertions in the R1-F1.17/18 test |
| R1-F1.24 | (new) Introduced patterns in tuple element/spread are checked | clean | `template_type_refusals::a_lookaround_pattern_in_a_returned_tuple_position_is_refused`, `..._in_an_imported_tuple_position_is_refused` |
| R1-F1.25 | (new) Active slots reachable through tuple positions are refused | clean | `template_type_refusals::a_slot_reached_through_a_tuple_position_is_a_reachable_slot` |
| R1-F2 | Import lookup requires grants for unused search candidates | fixed | all children verified; admission now lazy, per probe, first-match stop |
| R1-F2.1 | Direct returned bare-name import resolves with first-root grant only | fixed | `template_type_lazy_admission::a_returned_bare_name_import_is_asked_only_about_the_first_root_holding_it`; Darkmatter `a_bare_name_import_needs_a_grant_only_for_the_first_root_holding_it` |
| R1-F2.2 | Explicit returned import resolves | clean | `a_returned_explicit_import_is_asked_only_about_its_file` |
| R1-F2.3 | Returned bare-name example resolves with first-root grant only | fixed | `a_returned_bare_name_example_is_asked_only_about_the_first_root_holding_it` |
| R1-F2.4 | Explicit returned example resolves | clean | `a_returned_explicit_example_is_asked_only_about_its_file` |
| R1-F2.5 | Transitive bare-name dependency resolves | fixed | `a_transitive_bare_name_import_is_asked_only_about_the_first_root_holding_it` |
| R1-F2.6 | Static bare-name first-match control | clean | `the_static_bare_name_selection_probes_only_the_first_root_holding_it` plus static-equality assertions |
| R1-F2.7 | Admission asked only before each needed probe; missing first root still needs second-root grant | fixed | `a_bare_name_missing_from_the_first_root_asks_about_the_second_before_probing_it`; Darkmatter `a_bare_name_missing_from_the_first_root_needs_a_grant_for_the_next` |
| R1-F2.8 | (new) Sibling fallback asked about only after every root missed | fixed | `the_sibling_of_a_bare_name_is_asked_about_only_after_every_root_missed`, `the_sibling_is_never_asked_about_when_a_root_holds_the_name` |
| R1-F2.9 | (new) Non-bare reference with two candidates (implicit relative) stops at first match | fixed | `an_implicit_relative_import_stops_at_its_first_candidate_holding_it`, `..._asks_about_its_repository_candidate_only_when_needed`; biscuit-file `admitted_resolution_asks_only_about_candidates_the_search_reaches`, `admitted_resolution_stops_at_a_refused_candidate` |
| R1-F2.10 | (new) Probes made before a refusal were dropped from `searched` | fixed | asserted in the R1-F2.7 test |
| R1-F3 | Nested grouped union arms are never substituted | fixed | all children verified; review CLI shape now valid |
| R1-F3.1 | Bare slot | clean | `template_type_results::bare_slot_equals_static` |
| R1-F3.2 | Bare slot with `[]` | clean | `bare_slot_with_array_suffix_equals_static` |
| R1-F3.3 | Whole-property grouped union | clean | `whole_property_grouped_union_equals_static` |
| R1-F3.4 | Direct union arm | clean | `direct_union_arm_equals_static` |
| R1-F3.5 | Grouped union as a union arm | fixed | `grouped_union_as_a_union_arm_equals_static` |
| R1-F3.6 | Grouped union inside a nested group | fixed | `grouped_union_inside_a_nested_group_equals_static` |
| R1-F3.7 | Object property slot | clean | `ordinary_object_property_equals_static` |
| R1-F3.8 | Object property nested grouped union | fixed | `grouped_union_arm_in_an_object_property_equals_static`, `..._in_a_pattern_key_property_equals_static` |
| R1-F3.9 | Direct tuple element slot | clean | `direct_tuple_element_equals_static` |
| R1-F3.10 | Tuple element nested grouped union | fixed | `grouped_union_arm_in_a_tuple_element_equals_static` |
| R1-F3.11 | Named type body used twice | fixed | `grouped_union_arm_in_a_named_type_used_twice_equals_static` (one evaluation) |
| R1-F3.12 | Lowering receives fully substituted type | fixed | each row's helper asserts `is_resolved()` and `to_json_schema()` success |
| R1-F3.13 | CLI `md schema validate` succeeds | fixed | `darkmatter/cli/tests/l1/dynamic_schema_slot_template_types.rs::a_schema_file_with_a_template_type_in_a_nested_grouped_union_is_valid` |
| R1-F3.14 | (new) Array constraints and hoisted markers in nested groups | fixed | `nested_grouped_union_keeps_its_array_constraints`, `nested_grouped_union_hoists_a_postfixed_returned_union_marker`, `nested_grouped_union_takes_a_returned_null_arm_as_an_item` |
| R1-F3.15 | (new) Invariant broken only after nested substitution is located | fixed | `non_null_after_nested_substitution_is_a_located_invalid_expansion` |
| R1-F3.16 | (new) `requiredness()` read a nested item slot as known | fixed | `a_slot_in_an_item_union_leaves_presence_pending_until_it_resolves`; corrected `template_types_passive::requiredness_is_pending_even_when_the_suffix_says_required`; decision in `human_review_items` |
| R1-F3.17 | (new) Template constraint inside a nested group | clean | `nested_template_constraint_resolves` |
| R1-F3.18 | (new) Export round trip and slot collection | clean | `template_types_passive::export_round_trips_a_template_type_in_a_nested_grouped_union` |
| R1-F3.19 | (new) DMLS pending then resolved for nested shape | fixed | `darkmatter/dmls/tests/l1/schema_slots_editor.rs::a_template_type_in_a_nested_grouped_union_is_pending_then_resolved` |

### Progress

- starting the work on 'R1-F1 tuple examples bypass read approval' at 13:24:50
        - defect class: a recursive walk over a returned type definition skipped some type-structure positions (tuple elements and the tuple spread), so reads nested there escaped the slot gate and the defining-file base
        - repaired `rehome_atom_examples` (`simplified-schema/lib/src/resolve.rs`) as one exhaustive `TypeExpr` match; tuple elements and the spread now use the same gate, base directory, and read observations; no `_ =>` arm, so a new type position must be handled before the code compiles
        - discovered R1-F1.23: ordinary (non-template) imports of a tuple type also resolved tuple examples from the outer schema's directory; fixed by the same change
        - sibling walkers checked and clean: `slots::stamp_atom_origin`, `slots::reachable_slots`, `template_type::def_patterns`, `local_reference`, `TemplateTypes::nested`, `ImportEngine` expansion, `graph.rs` discovery, `recursive.rs::atom_refs`, `substitute.rs`, `atom_has_imports`, `query::collect_def_refs`, Darkmatter `pending_report.rs`
        - decided (R1-F1.21): no separate defensive check in the final JSON example loader; the lowered JSON has no origin record, so that check would need new state threaded through `ReadGate`; every example-bearing position is gated before lowering instead
        - verified the CLI reproduction now exits 1 with `dm.schema.slot_approval_required` for both tuple element and spread
        - new tests failed without the fix and pass with it; pattern and reachable-slot tuple tests pass either way, which confirms those walkers were already correct
- work completed for 'R1-F1 tuple examples bypass read approval' at 13:37:40
- starting the work on 'R1-F2 import lookup requires grants for unused search candidates' at 13:38:10
        - defect class: an ordered first-match lookup that needs approval asked for approval of every possible location up front, turning "use the first one found" into "every location must be granted"
        - one selector remains: `select` (`simplified-schema/lib/src/resolve/select.rs`) takes an optional admission check called immediately before each probe; the static path passes none, so its order and behavior are unchanged; `planned_probes` and up-front admission were removed
        - a non-bare reference (`lib/types.yaml`) searches the referencing folder then the repository root inside biscuit-file's `resolve_detailed`, so the gate had to move there too: new public `FileReference::resolve_detailed_admitted(ctx, admit)` asks before each candidate's containment check and probe and stops at the first match; a recursive `%` reference admits every search root before the walk because the walk lists all roots before choosing (same set the static walk touches)
                - decision: gate inside biscuit-file rather than re-implementing its candidate search and boundary checks in simplified-schema; this crosses into the biscuit-file package area
        - discovered and fixed R1-F2.8 (sibling fallback now asked about only after every root missed), R1-F2.9 (non-bare reference with two candidates), R1-F2.10 (probes made before a refusal were dropped; now reported to `observe` and kept in `searched`)
        - a refused sibling check returns the refusal instead of reporting "missing"
        - `select_schema_file_checked` returns `Refused { error: Box<SchemaError>, attempted }` (boxed for clippy's large-error lint)
        - Darkmatter's `DarkmatterTemplateReads::admit` judges one path per call and plans nothing: clean
        - tests proven against pre-fix code: 7 of 11 new library tests and 2 of 2 Darkmatter tests failed before the fix; all pass after
        - docs updated: `simplified-schema/docs/topics/language/slots.md`, `biscuit-file/docs/topics/file-references.md`, `.claude/skills/simplified-schema/templates.md`
- work completed for 'R1-F2 import lookup requires grants for unused search candidates' at 13:58:50
- starting the work on 'R1-F3 nested grouped union arms are never substituted' at 13:59:20
        - defect class: substitution over a recursive type structure replaced slots only at the top of each union position; grouped unions nested in union arms, nested groups, object/pattern-key properties, tuple elements, and named type bodies were traversed but never substituted
        - `TypeSubstitution::nested` became `TypeSubstitution::atom` (`simplified-schema/lib/src/slots/template_type.rs`): each union arm and nested group substitutes its own arms first, then runs `atom_union`, keeping array constraints and hoisting markers onto the group's array list as the import engine does; the first replaced slot locates `check_union_invariants` / `TupleType::check_element` failures on the enclosing definition
        - discovered and fixed R1-F3.16: `PropertyDef::requiredness()` reported known presence for a slot inside an array's item union, although a returned `string(required)` item hoists `required` onto the property; it is now `Pending` unless the rest of the definition already requires the property, and `TypeDefinition::is_required()` follows `requiredness() == Required`; the passive-test control that called this "presence known" was corrected
                - decision: `has_pending_type()` keeps meaning "top-level atom is a slot" (DMLS pending hover); widening it would print "type pending[]" for `({{ t }} | string)[]`
        - sibling sites clean: slot collection, origin stamping, the template-constraint pass (R1-F3.17), passive export round trip (R1-F3.18), Darkmatter `pending_report.rs`, Claudine slot detection (`reachable_slots`), phase projection
        - DMLS pending-to-resolved transition covered for the nested shape (R1-F3.19)
        - docs: `slots.md` positions table gains the grouped-union row; `requiredness()` row covers item slots; `.claude/skills/simplified-schema/templates.md` updated
        - checked and left as is: the refused-position diagnostic lists "a property's type … property-level union arm, or tuple element", the spec's own wording; a grouped union written as a property's type is that property's type, so the message stays correct
        - checked and left as is: DMLS hover on a concrete property-level union shows its first arm by existing design (review decision 25 already accepted this)
        - tests proven against pre-fix code: nine library tests plus the CLI and DMLS tests failed with "1 unresolved" before the fix; all pass after
- work completed for 'R1-F3 nested grouped union arms are never substituted' at 14:16:30
- fixed a pre-existing, unrelated rustdoc defect the R1-F3 subagent found: `simplified-schema/lib/src/lib.rs:19` linked the ambiguous `format` (module and macro); now `[`format`](mod@format)`; `cargo doc` with `-D rustdoc::broken_intra_doc_links` succeeds
- full gates (run after all three repairs, with no subagent running):
        - `simplified-schema: just test`: 1829 passed; minimal-feature run 1567 passed
        - `simplified-schema: just lint`: clean
        - `biscuit-file: just test`: 1281 passed (plus 6 passed, 104 skipped in its second target)
        - `biscuit-file: just lint`: clean
        - `darkmatter: just test`: 16331 passed, 10 skipped (141 s)
        - `darkmatter: just lint`: exit 0 (including the Zed WASM extension check)
        - `claudine: just test`: 11859 passed (1 slow), 15 skipped; run because `TypeDefinition::is_required()` / `requiredness()` changed under R1-F3.16
        - `claudine: just lint`: exit 0
        - `just check-tier-coverage` for simplified-schema, darkmatter, biscuit-file: 0 stranded tests each
- cross-OS check: `just cross-check biscuit-file --os windows` (biscuit-file path normalization in the new admission path is the notable OS risk) could not run: `Host key verification failed` connecting to `build-win-native`, the same host-connection failure the review recorded
        - accepting a new host key non-interactively would change SSH trust, so it was not attempted; no Windows test failure was observed, so `os_failing` is not set; nightly CI covers native Windows and WSL2

### Completion Report

- **outcome: complete**: all three findings are repaired and every required local check passes; elapsed 2026-11-09T13:24:29 → 14:36 (about 1 h 12 min)
- parent findings: total 3 = fixed 3 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- instance rows: 57 total (22 + 3 new for R1-F1, 7 + 3 new for R1-F2, 13 + 6 new for R1-F3); none pending, blocked, unresolved, or disputed
- verified repairs:
        - R1-F1: example rehoming walks tuple elements and spreads with the slot gate and the defining-file base; also fixed the same wrong-base bug for static tuple imports (R1-F1.23)
        - R1-F2: template-type reads are admitted one location at a time immediately before each probe, in the static selector's order with its first-match stop; biscuit-file gained `FileReference::resolve_detailed_admitted` so non-bare references gate the same way
        - R1-F3: grouped unions holding a template type are substituted at every nesting depth with markers, array constraints, and invariants preserved; `requiredness()` is now pending for a slot in an item union (R1-F3.16)
- open findings: none
- not done: the Windows cross-check (rig unreachable: SSH host key verification failed); this is advisory evidence, not a required local check, and CI's nightly covers it
- decisions added to the review's `human_review_items`: the biscuit-file admitted-resolution API, refused sibling check behavior, item-union requiredness, and no loader-side example gate
- changed files in this cycle:
        - R1-F1: `simplified-schema/lib/src/resolve.rs`, `simplified-schema/lib/tests/l1/template_type_imports.rs`, `simplified-schema/lib/tests/l1/template_type_refusals.rs`, `darkmatter/lib/tests/l1/dynamic_schema_slot_template_types.rs`
        - R1-F2: `biscuit-file/lib/src/file_reference/mod.rs`, `biscuit-file/lib/src/file_reference/resolve.rs`, `biscuit-file/lib/tests/l1/detailed_resolution.rs`, `biscuit-file/docs/topics/file-references.md`, `simplified-schema/lib/src/resolve/select.rs`, `simplified-schema/lib/src/resolve.rs`, `simplified-schema/lib/src/slots/reads.rs`, `simplified-schema/lib/tests/l1/template_type_lazy_admission.rs` (new), `simplified-schema/lib/tests/l1/main.rs`, `darkmatter/lib/tests/l1/dynamic_schema_slot_template_types.rs`
        - R1-F3: `simplified-schema/lib/src/slots/template_type.rs`, `simplified-schema/lib/src/simplified/types.rs`, `simplified-schema/lib/src/schema/view.rs`, `simplified-schema/lib/tests/l1/template_type_results.rs`, `simplified-schema/lib/tests/l1/template_types_passive.rs`, `darkmatter/cli/tests/l1/dynamic_schema_slot_template_types.rs`, `darkmatter/dmls/tests/l1/schema_slots_editor.rs`
        - docs and skills (R1-F2, R1-F3): `simplified-schema/docs/topics/language/slots.md`, `.claude/skills/simplified-schema/templates.md`
        - unrelated rustdoc fix: `simplified-schema/lib/src/lib.rs`
- final verification commands: see "full gates" above; focused runs per finding were `simplified-schema: just test template_type` (151 passed at the end), `darkmatter: just test dynamic_schema_slot_template` (40 passed), `darkmatter: just test schema_slots_editor` (53 passed), `claudine: just test-cli completion_committed_prompt_schema` (13 passed)

## Implementation of Review Findings #2

> **started at:** 2026-11-09T14:41:17-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-08-template-types/review-2.md'
- this is iteration 2 of the review-to-implement cycle 
- review frontmatter `implemented` confirmed `false` before repairs
- the review has no `human_in_the_loop` items and no blocked findings; R2-F1 is unblocked and recurrent (sibling of R1-F2)

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R2-F1 | File lookup skips an earlier unsaved schema buffer | fixed | every child fixed or clean; selection is now buffer-aware per candidate for gated and ungated lookups; full gates green |
| R2-F1.1 | Returned named import, implicit relative, first candidate buffer-only, fallback on disk, both granted → import first candidate's buffer | fixed | `template_type_lazy_admission::a_returned_import_selects_an_open_buffer_at_its_first_candidate_over_a_later_file` (imports, asked/observed probes, `searched` = buffer only) |
| R2-F1.2 | Same, only local folder granted → select granted buffer; never request fallback | fixed | `…::a_returned_import_of_an_open_buffer_never_asks_about_the_later_candidate`; Darkmatter `dynamic_schema_slot_template_types::an_unsaved_buffer_at_the_first_candidate_needs_a_grant_only_for_itself` |
| R2-F1.3 | Same with fallback file removed, local grant only → select buffer without fallback probe | fixed | `…::a_returned_import_of_an_open_buffer_with_no_later_file_never_probes_further` |
| R2-F1.4 | Transitive returned import (`Outer@./outer.yaml` imports `Person@lib/types.yaml`) → publish buffer's `name` definition | fixed | `…::a_transitive_returned_import_selects_the_open_buffer_its_file_names` (`name` accepted, `age` rejected, fallback never asked) |
| R2-F1.5 | Static named import through shared selector → import first candidate's buffer | fixed | `…::a_static_import_selects_an_open_buffer_at_its_first_candidate_over_a_later_file` |
| R2-F1.6 | Returned example `string(example(lib/example.yaml))` → consume first candidate's buffer | fixed | `…::a_returned_example_selects_an_open_buffer_at_its_first_candidate_over_a_later_file` |
| R2-F1.7 | Static example through final example loader → consume first candidate's buffer | fixed | `…::a_static_example_selects_an_open_buffer_at_its_first_candidate_over_a_later_file` |
| R2-F1.8 | Root `$schema` reference `lib/schema.yaml` → `referenced_files`/export identify first buffer | fixed | `…::a_root_schema_reference_selects_an_open_buffer_at_its_first_candidate_over_a_later_file` (`referenced_files`, `searched`, export has `name` not `age`) |
| R2-F1.9 | Public `select_schema_file` result consumed by DMLS reference projection → `Found` names first buffer | fixed | `…::the_public_selection_finds_an_open_buffer_at_its_first_candidate_and_probes_no_further`; DMLS `schema_slots_editor::template_types::a_new_unsaved_file_at_a_returned_import_first_candidate_is_its_file` |
| R2-F1.10 | Final strict validation after row .1 → accept `name`, reject `age` | fixed | shared `assert_buffer_person` strict-validates (accept `{"name":"x"}`, reject `{"age":3}`) in .1, .2, .5, .12, .14 |
| R2-F1.11 | Control: explicit relative `./lib/types.yaml` buffer-only → uses buffer | clean | control `…::an_explicit_returned_import_selects_its_open_buffer` (single candidate, unchanged behavior) |
| R2-F1.12 | Control: bare-name schema root, first root buffer-only, only first granted → stop at first root's buffer | clean | control `…::a_returned_bare_name_import_stops_at_an_open_buffer_in_the_first_root` |
| R2-F1.13 | Missing-all-disk, implicit relative, all granted → use first buffer, stopping before fallbacks | fixed | `…::a_returned_import_found_only_as_an_open_buffer_records_no_probe_after_it` (fallback probes no longer made) |
| R2-F1.14 | Control: edited buffer over existing first disk file, local grant only → use edited buffer | clean | control `…::a_returned_import_reads_the_unsaved_edit_of_its_first_file` |
| R2-F1.15 | Sibling (sweep): DMLS link graph `graph/arena.rs::locate` used open buffers only after the whole disk search missed → an unsaved middle candidate must outrank a later disk file | fixed | `graph::arena::tests::test_an_unsaved_buffer_at_a_middle_candidate_outranks_a_later_existing_file`; uses the same biscuit-file presence hook with graph documents as the predicate |
| R2-F1.16 | Sibling (sweep): consumers of `selection.attempted` (DMLS `references.rs::probes`, `named_types.rs`, gated probe observations, `searched` sets) → stop at the buffer and keep earlier probes | clean | they read the selector's probe list, which now ends at the matched buffer; asserted in .1/.3/.13 |
| R2-F1.17 | Sibling (sweep): `context::resolve_reference` and providers, `markdown/fs.rs::resolve_entry_in_context`, `file_context::resolve_document_file_ref`, `schema/source.rs::resolve_target` | clean | none of these consults open buffers, so there is no after-search buffer ordering to get wrong |
| R2-F1.18 | Sibling (sweep): kind registry and trigger discovery | clean | they read every root file and buffer and treat duplicate claims as errors; not first-match, so search order never picks a winner |
- starting the work on 'R2-F1: File lookup skips an earlier unsaved schema buffer' at 14:41:48
        - defect class: an ordered first-match file search consults a virtual (open-buffer) source only after the whole on-disk search finishes, so a later on-disk candidate (or its admission) wins over an earlier candidate that exists only as a buffer
        - delegated to a foreground rust-developer subagent with the full child inventory; it reports log items back to the orchestrator
        - design: new public `FileReference::resolve_detailed_with_presence(ctx, admit, present)` in biscuit-file; for each direct candidate it admits, checks containment, probes disk, and only on a miss asks `present`; `true` records the candidate as `Matched` and ends the search (no later admission or probe); `resolve_detailed_admitted` is that call with `present` always false
        - `simplified-schema` `select.rs` routes both the gated and the ungated selection through it with `buffers::is_open`; the after-search `buffers::first_open` fallback (non-bare branch and schema-root loop) was removed
        - decision: recursive (`%`) references keep walking only the disk; matching buffers against a recursive pattern would duplicate biscuit-file's walk rules, and the old fallback never selected a buffer for them either (`probed_paths()` omits recursive roots)
        - decision: a present location is recorded as `ProbeDisposition::Matched` rather than a new variant (a new variant would break an exhaustive match in `sniff`)
        - sweep found one more instance of the class, in the DMLS Markdown link graph (`arena.rs::locate`), fixed as R2-F1.15; R2-F1.16–.18 checked clean
        - every new regression was confirmed red with the predicate stubbed to `false` (or the old `locate` restored) and green with the fix
        - docs/skills updated: `biscuit-file/docs/topics/file-references.md`, `simplified-schema/docs/topics/selecting-schema-files.md`, `darkmatter/dmls/docs/file-references.md`, `darkmatter/docs/lsp/architecture.md`, `.claude/skills/biscuit-file/references/file-references.md`, `.claude/skills/simplified-schema/templates.md`, `.claude/skills/darkmatter/dmls.md`
- work completed for 'R2-F1: File lookup skips an earlier unsaved schema buffer' at 15:14:05
- full gates (orchestrator, after all changes, no subagent running):
        - `biscuit-file: just test`: 1285 passed (plus 6 passed, 104 skipped in its second target); `just lint`: exit 0
        - `simplified-schema: just test`: 1842 passed; minimal-feature run 1567 passed; `just lint`: exit 0
        - `darkmatter: just test`: 16334 passed, 10 skipped (160 s); `just lint`: exit 0 (including the Zed WASM extension check)
        - `claudine: just test-cli completion_committed_prompt_schema`: 13 passed (consumer of the selector)
        - `just check-tier-coverage` for simplified-schema, biscuit-file, darkmatter: 0 stranded each
- cross-OS check: `just cross-check simplified-schema --os windows` could not run: `Host key verification failed` connecting to the Windows rig (same as iteration 1); accepting a new host key non-interactively would change SSH trust, so it was not attempted; no Windows test failure was observed, so `os_failing` is not set

### Completion Report

- **outcome: complete**: R2-F1 is repaired at every listed site plus one discovered sibling, and every required local check passes; elapsed 2026-11-09T14:41:17 → 15:14:05
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- instance rows: 18 total (14 from the review + 4 from the sweep): 12 fixed, 6 clean; none pending, blocked, unresolved, or disputed
- verified repairs: an open buffer at an admitted, contained candidate holding no file is now the match at that candidate, before any later candidate is admitted or probed, for returned and static imports, examples, root `$schema` references, the public `select_schema_file`, and the DMLS link graph
- open findings: none
- not done: Windows cross-check (rig unreachable over SSH); advisory, nightly CI covers native Windows and WSL2
- decisions added to the review's `human_review_items`: recursive references stay disk-only; presence recorded as `Matched`; repair widened to the DMLS Markdown link graph
- changed files in this cycle (all R2-F1):
        - `biscuit-file/lib/src/file_reference/mod.rs`, `biscuit-file/lib/src/file_reference/resolve.rs`, `biscuit-file/lib/tests/l1/detailed_resolution.rs`, `biscuit-file/docs/topics/file-references.md`
        - `simplified-schema/lib/src/resolve/select.rs`, `simplified-schema/lib/src/resolve/buffers.rs`, `simplified-schema/lib/tests/l1/template_type_lazy_admission.rs`, `simplified-schema/docs/topics/selecting-schema-files.md`
        - `darkmatter/dmls/src/graph/arena.rs`, `darkmatter/lib/tests/l1/dynamic_schema_slot_template_types.rs`, `darkmatter/dmls/tests/l1/schema_slots_editor.rs`, `darkmatter/dmls/docs/file-references.md`, `darkmatter/docs/lsp/architecture.md`
        - `.claude/skills/biscuit-file/references/file-references.md`, `.claude/skills/simplified-schema/templates.md`, `.claude/skills/darkmatter/dmls.md`
- final verification: the full gates above; the subagent's focused runs were `biscuit-file: just test detailed_resolution` (22 passed), `simplified-schema: just test template_type` (164 passed) and `template_type_lazy_admission` (24 passed), `darkmatter: just test dynamic_schema_slot_template` (41), `schema_slots_editor` (54), `graph::` (144), `link` (533)
