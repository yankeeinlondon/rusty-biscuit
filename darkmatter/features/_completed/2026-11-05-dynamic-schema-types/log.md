---
feature: "2026-11-05-dynamic-schema-types"
implementation_1: "2026-11-08T11:49:32-08:00"
implementation_2: "2026-11-08T14:49:39-08:00"
implementation_3: "2026-11-08T15:42:47-08:00"
---

# Review-to-Implement Log: Dynamic Schema Types

## Implementation of Review Findings #1

> **started at:** 2026-11-08T11:49:32-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-dynamic-schema-types/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was already `false`; it was left `false` before repairs began
- the review has no `human_in_the_loop` items, no blocked findings, and no recurrence; every finding is repairable this cycle

### Finding Ledger

Test paths: `lib` = `darkmatter/lib/tests/l1/`, `cli` = `darkmatter/cli/tests/l1/dynamic_schema_slots.rs`, `dmls` = `darkmatter/dmls/tests/l1/schema_slots_editor.rs`, `ss` = `simplified-schema/lib/tests/l1/dynamic_slots.rs`. Every "fixed" test below was confirmed red before its repair and passes after it; the final full-suite results are in the Completion Report.

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | Several slot paths perform reads before approval | fixed | every child below fixed or clean |
| R1-F1.1 | Inline enum slot: approval-required before data bytes are read | fixed | lib `dynamic_schema_slot_read_order.rs::inline_slots_are_judged_before_their_data_is_read::an_enum`; `$schema` excluded from every frontmatter interpolation pass |
| R1-F1.2 | Inline suggestion slot | fixed | `…::a_suggestion_list` |
| R1-F1.3 | Inline literal slot | fixed | `…::a_literal` |
| R1-F1.4 | Inline default slot | fixed | `…::a_default` |
| R1-F1.5 | Inline minimum slot | fixed | `…::a_minimum` |
| R1-F1.6 | Inline maximum slot | fixed | `…::a_maximum` |
| R1-F1.7 | Inline pattern slot | fixed | `…::a_pattern`; `a_granted_enum_resolves_through_the_slot` (a granted list stays a list); cli `compose_reads_nothing_for_an_unapproved_inline_slot` |
| R1-F1.8 | Imported data contract authorized before opening | fixed | `a_contract_import_is_judged_before_it_is_opened` (new `buffers::guarded` read guard + `ResolutionContext::gate_schema_reads`) |
| R1-F1.9 | `validate_schema` reads permitted page, stops before its schema | fixed | `validate_schema_reads_the_permitted_page_and_stops_before_its_schema` |
| R1-F1.10 | `find_files` directory approval before enumeration | fixed | `listings_are_judged_before_the_folder_is_enumerated::find_files` |
| R1-F1.11 | `find_first_index` | fixed | `…::find_first_index` |
| R1-F1.12 | `find_last_index` | fixed | `…::find_last_index` |
| R1-F1.13 | Direct `load_data` control | clean | `direct_reads_stay_judged_before_reading::load_data` |
| R1-F1.14 | `frontmatter` control | clean | `…::frontmatter` |
| R1-F1.15 | `try_frontmatter` control | clean | `…::try_frontmatter_keeps_the_refusal` |
| R1-F1.16 | `markdown_title` control | clean | `…::markdown_title` |
| R1-F1.17 | `markdown_body_empty` control | clean | `…::markdown_body_empty` |
| R1-F1.18 | (discovered) slot cache rehashed a cached entry's inputs before judging grant coverage | fixed | `session_tests::reuse::an_uncovered_cache_entry_is_refused_before_its_inputs_are_reread` |
| R1-F1.19 | (discovered) pre-flight and post-call check named listings by pattern, so no folder grant could allow them | fixed | `a_persisted_folder_grant_lets_each_listing_run`, `the_folder_grant_pre_flight_offers_covers_each_listing` |
| R1-F1.20 | (sibling) render-time "did you mean" sibling listing | clean | only `errors/blocks.rs::interpolation_block` calls `suggest_sibling_files`, for `MarkdownError::Interpolation` (document expressions); no schema/slot code produces it |
| R1-F2 | Overridden trigger slots still evaluate | fixed | every child below fixed or clean |
| R1-F2.1 | Document over trigger: success, no question, no read | fixed | lib `dynamic_schema_slot_precedence.rs::compose::a_document_redeclaring_the_trigger_property_composes_without_reading_its_data`; cli `compose_succeeds_without_reading_a_trigger_slot_the_document_redeclares` (`read_set: []`), `schema_validate_is_valid_when_the_document_redeclares_the_trigger_slot_property` |
| R1-F2.2 | Later trigger over earlier | fixed | `compose::a_later_trigger_redeclaring_the_property_discards_the_earlier_slot`; cli `compose_succeeds_without_reading_a_trigger_slot_a_later_trigger_redeclares`, `schema_validate_is_valid_when_a_later_trigger_redeclares_the_slot_property`; `an_earlier_trigger_never_discards_a_later_one` |
| R1-F2.3 | Unmatched payload control | clean | `compose::an_unmatched_template_trigger_reads_nothing` |
| R1-F2.4 | Effective dynamic payload control | clean | lib `a_selected_trigger_slot_with_no_override_still_needs_approval`; cli and dmls counterparts |
| R1-F2.5 | One shared surviving-slot selection for pre-flight and runtime | fixed | `discarded_by_higher_layers` used by `pending_templates_for` and `resolve_trigger_layers`; `preflight::pre_flight_asks_nothing_about_a_slot_the_document_discards`, `…_a_later_trigger_discards` |
| R1-F2.6 | `md schema validate` / `md clean` approval step | fixed | `validation::validation_with_a_discarded_slot_is_valid_and_asks_nothing`; cli `clean_reports_no_findings_when_the_document_redeclares_the_trigger_slot_property` |
| R1-F2.7 | Reference check | fixed | each lib compose case runs the passive reference check first |
| R1-F2.8 | DMLS | fixed | dmls `a_trigger_slot_the_document_redeclares_is_never_pending_or_reported` |
| R1-F2.9 | Slot-cache identity includes the omitted properties | fixed | `a_trigger_resolved_without_a_property_is_cached_apart_from_its_whole_payload` |
| R1-F2.10 | Pending reporting ignores discarded slots | fixed | `preflight::a_payload_left_without_slots_is_not_a_pending_template`, `validation::a_discarded_slot_never_leaves_the_effective_schema_pending` |
| R1-F2.11 | Named-type definitions reached only by discarded properties | fixed | `named_types::a_definition_reached_only_by_discarded_properties_is_never_read`; control `a_definition_a_surviving_property_reaches_still_resolves` |
| R1-F2.12 | Root-union template document: pre-flight matches runtime | fixed | lib `template_union_document::*` (4 tests incl. some-arms control); ss `a_root_union_arm_template_declares_what_every_arm_declares` (`PendingSchema::declared_properties`) |
| R1-F3 | Inline and trigger slot failures lose authored locations | fixed | every child below fixed or clean |
| R1-F3.1 | Compose inline schema positioned | fixed | lib `dynamic_schema_slot_positions.rs::inline_document::compose_reports_a_wrong_result…` (doc, line 4, col 12, exact span); cli `compose_reports_an_inline_slot_failure…` |
| R1-F3.2 | Schema-validate inline schema | fixed | lib `inline_document::schema_validate_reports_a_wrong_result…`; cli `schema_validate_reports_an_inline_slot_failure…` |
| R1-F3.3 | Compose trigger-inline payload | fixed | lib `trigger_inline::compose_reports_an_unapproved_read…` (trigger file, line 5, col 14); cli `compose_reports_a_trigger_slot_failure…` |
| R1-F3.4 | Schema-validate trigger-inline payload | fixed | lib `trigger_inline::schema_validate_…`; cli `schema_validate_reports_a_trigger_slot_failure…` |
| R1-F3.5 | Compose referenced schema control | clean | lib `controls::*` (line 2, col 12) |
| R1-F3.6 | Schema-validate referenced schema control | clean | lib `controls::*` |
| R1-F3.7 | Compose imported named type control | clean | lib `controls::*` (line 3, col 14) |
| R1-F3.8 | Schema-validate imported type control | clean | lib `controls::*` |
| R1-F3.9 | DMLS inline slot uses shared mapping | fixed | dmls `an_inline_slot_is_ranged_at_its_own_authored_bytes_through_a_quoted_carrier`; editor-side `authored_slot_span` lookup removed |
| R1-F3.10 | Quoted carriers (single, double with escapes) | fixed | lib `carriers::*` |
| R1-F3.11 | Block carriers (`\|-`, `>-` folded, `\|2+`) | fixed | lib `carriers::*` |
| R1-F3.12 | Approval-required, inline | fixed | `an_unapproved_read_is_at_the_slot_in_the_document` |
| R1-F3.13 | `dm.data.*` failure keeps code and `data_location` | fixed | `a_data_failure_keeps_its_code_and_data_location_at_the_slot` |
| R1-F3.14 | Rejected pattern | fixed | `a_rejected_pattern_is_at_the_slot_in_the_document` |
| R1-F3.15 | Dynamic default | fixed | `an_inadmissible_dynamic_default_is_at_the_slot_in_the_document` |
| R1-F3.16 | Root-union arm | fixed | `a_root_union_arm_slot_is_at_the_slot_in_the_document` |
| R1-F3.17 | Identical slots each at their own bytes | fixed | `two_identical_slots_are_each_at_their_own_bytes` |
| R1-F3.18 | Trigger payload cache keyed on trigger text | fixed | `an_edit_above_an_unchanged_payload_moves_the_position` |
| R1-F3.19 | Trigger payload reference | clean | `a_referenced_payload_slot_is_in_the_schema_file` |
| R1-F3.20 | `--schema` override (no authored text): null, not placeholder | clean | `unproven::an_override_schema_slot_names_no_position` |
| R1-F3.21 | Cycle | clean | `cycle::a_cycle_is_at_the_outer_slot_in_its_file` |
| R1-F3.22 | YAML document root `$schema` | fixed | lib `yaml_document::…` |
| R1-F3.23 | DMLS unsaved edit moves range | clean (guard) | dmls `an_unsaved_edit_above_the_schema_moves_the_slot_range` |
| R1-F3.24 | `md clean` | fixed | cli `clean_reports_an_inline_slot_failure…` |
| R1-F3.25 | Refused read inside a slot (sub-span) | fixed | lib `refused::*` |
| R1-F3.26 | Contract template position | clean | lib `contract_template::*` |
| R1-F3.27 | DMLS trigger related information | fixed | dmls `a_trigger_slot_points_at_its_position_in_the_trigger_file` |
| R1-F3.28 | (discovered, M3) contract-template slot failure flattened to `dm.schema.slot_failed` | fixed | lib `contract_template::*` (wrong shape → `dm.schema.slot_wrong_shape`; unapproved → `dm.schema.slot_approval_required` with `grants_needed`; data → `dm.data.*` with `data_location` line 2), compose and validation |
| R1-F4 | Unsaved data buffers are ignored outside YAML and JSON | fixed | every child below fixed or clean |
| R1-F4.1 | JSON5 buffer | fixed | dmls `unsaved_buffers::json5` |
| R1-F4.2 | JSONL buffer | fixed | `unsaved_buffers::jsonl` |
| R1-F4.3 | TOML buffer | fixed | `unsaved_buffers::toml` |
| R1-F4.4 | CSV buffer | fixed | `unsaved_buffers::csv` |
| R1-F4.5 | TSV buffer | fixed | `unsaved_buffers::tsv`; guard `every_loadable_format_has_a_row` (exhaustive over `DataFormat`) |
| R1-F4.6 | YAML control | clean | `unsaved_buffers::yaml` |
| R1-F4.7 | JSON control | clean | `unsaved_buffers::json` |
| R1-F4.8 | (sibling) Markdown read by `frontmatter()` from a slot | fixed | `unsaved_buffers::markdown` |
| R1-F4.9 | (sibling) contract import with unusual extension | fixed | `an_imported_schema_with_an_unusual_extension_is_read_from_its_buffer` |
| R1-F4.10 | (discovered) closing a buffer did not revert to disk (buffer read recorded disk `stat`) | fixed | `closing_the_buffer_reverts_to_the_disk_contents`; lib `a_read_of_an_open_buffer_is_rehashed_once_the_buffer_closes` |
| R1-F4.11 | Dirty non-YAML buffer loses automatic grant | fixed | `a_dirty_buffer_loses_the_automatic_grant` |
| R1-F5 | Stale completion candidates have no stale marker | fixed | every child below fixed or clean |
| R1-F5.1 | Completion during held refresh marked stale | fixed | dmls `while_a_refresh_is_held_the_last_good_snapshot_is_served_marked_stale` (note in `detail`; labels/edits/order unchanged; no note after release) |
| R1-F5.2 | Hover control | clean | same test; hover now reads the shared `slot_note` |
| R1-F5.3 | Diagnostic status control | clean | same test (`dm.schema.slot_pending` information diagnostic) |
| R1-F5.4 | Array-item completion | fixed | `stale_responses::an_array_item_candidate_from_a_refreshing_snapshot_is_marked_stale` |
| R1-F5.5 | Union value completion | fixed | `…::a_union_value_candidate_from_a_refreshing_snapshot_is_marked_stale` |
| R1-F5.6 | Union item completion | fixed | `…::a_union_item_candidate_from_a_refreshing_snapshot_is_marked_stale` |
| R1-F5.7 | Suggestion completion | fixed | `…::a_suggestion_from_a_refreshing_snapshot_is_marked_stale` |
| R1-F5.8 | Pending literal candidates marked pending | fixed | `…::literal_candidates_offered_before_the_first_resolution_are_marked_pending` |
| R1-F5.9 | Body expression / collection-opener hovers | fixed | `…::a_body_expression_hover_from_a_refreshing_snapshot_is_marked_stale` |
| R1-F5.10 | "Add required key" quick fix inserting a slot value | fixed | `…::a_quick_fix_inserting_a_slot_value_from_a_refreshing_snapshot_is_marked_stale` |

### Progress

- starting the work on 'R1-F1: Several slot paths perform reads before approval' at 11:50:02
        - defect class: an effectful read or listing reachable from slot evaluation performs its I/O before the slot grant gate judges the target
        - discovery: the library `compose_with_warnings` path alone does not reproduce R1-F1.1–7 (its in-pipeline gate fails first); the read happens in the reference check `md compose` runs before composing (`ComposeOptions::for_reference_check` graph preparation), whose frontmatter interpolation evaluated the inline `$schema` subtree; pre-flight (`collect.rs`) already excluded `$schema`, the pipeline did not
                - the new tests run the reference check, then the composition, as `md compose` does
        - sibling sites checked
                - dispatch table input readers: `load_data`, the `frontmatter` family, `markdown_*` (already gated before reading: R1-F1.13–17 clean); `validate_schema`, `find_files`, `find_first_index`, `find_last_index` (defective); path functions, index arithmetic, `has_skill`/`has_local_skill`, local `file_exists` make existence observations only (clean, confirmed by grep: the only I/O sites in `functions/` are `load_markdown`'s read, `find_index_endpoint`'s `read_dir`, and `find_files`' `list_files`)
                - schema opens while a slot is active: contract imports and `validate_schema`'s schema both read through `simplified_schema::resolve::buffers::read`
                - frontmatter passes walking `$schema`: pass 1 and pass 2 (both through `interpolate_frontmatter_impl`), pre-flight best effort (already excluded)
                - process slot cache: `SlotSession::serves` reread (rehashed) a cached entry's inputs before judging grant coverage (new row R1-F1.18)
                - pre-flight target naming: a listing was collected as the argument path (`<folder>/*.md`, `<folder>/review-1.md`), so the offered folder grant never covered the listing; and the post-call check named a `find_files` observation by its relative pattern, so no grant could allow it (new row R1-F1.19)
                - render-time "did you mean" sibling suggestions (`file_suggestions.rs`) list a folder when a not-found diagnostic is rendered; not reached during slot evaluation and not in the observation log; left unchanged (R1-F1.20)
        - repairs
                - `frontmatter_interpolation.rs`: `interpolate_frontmatter_impl` always excludes `$schema` (`with_schema_excluded`), covering every pass and caller
                - `simplified-schema` `resolve::buffers::guarded`/`unguarded`/`ReadGuard`: a host guard asked before every schema source read (buffer or disk); a refusal fails the read as `PermissionDenied`
                - `ResolutionContext::gate_schema_reads`: runs a schema resolution under a guard calling the slot gate; used by `DataRuntime::prepare_contract` (now takes the `ResolutionContext`; a refused preparation is not memoized) and `validate_schema`
                - `find_files` authorizes each pattern root, `find_*_index` the existing parent folder, before enumerating; the post-call check judges a `Glob` observation by its pattern's roots (`glob_listing_roots`)
                - `targets.rs::listed_folder`: pre-flight names the folder a listing enumerates
                - `session::resolve_contract_slots` evaluates nested template slots under `buffers::unguarded`, so they keep their own gates only
                - `SlotSession::serves` judges grant coverage before rehashing inputs; new work counter `SlotInputRehash`
                - docs: `docs/topics/schemas/schema-slots.md` "What a slot may read"; `simplified-schema/docs/topics/selecting-schema-files.md`; skills `darkmatter/requests.md`, `darkmatter/schema.md`, `simplified-schema/SKILL.md`
        - tests (all failed before the repair, pass after)
                - `darkmatter/lib/tests/l1/dynamic_schema_slot_read_order.rs`: one per R1-F1.1–12, controls R1-F1.13–17, `a_granted_enum_resolves_through_the_slot`, `a_persisted_folder_grant_lets_each_listing_run`, `the_folder_grant_pre_flight_offers_covers_each_listing`
                - `darkmatter/cli/tests/l1/dynamic_schema_slots.rs::compose_reads_nothing_for_an_unapproved_inline_slot`
                - `session_tests.rs::reuse::an_uncovered_cache_entry_is_refused_before_its_inputs_are_reread`
                - `simplified-schema` `buffers::tests::a_guard_is_asked_before_every_read_and_a_refusal_reads_nothing`, `an_unguarded_scope_suspends_the_guards_around_it`
        - orchestrator check of R1-F1.20 (sibling "did you mean" hints): clean. `suggest_sibling_files` is called only from `errors/blocks.rs::interpolation_block`, which renders `MarkdownError::Interpolation` (document expressions, outside Q5); no code under `lib/src/markdown/schemas/` produces that error, and `$schema` is now excluded from interpolation, so a slot failure never reaches the listing
- work completed for 'R1-F1: Several slot paths perform reads before approval' at 12:21:28
- starting the work on 'R1-F2: Overridden trigger slots still evaluate' at 12:21:28
        - defect class: a layer's slots are resolved or prompted for before layer precedence decides which properties survive
        - sibling sites checked
                - runtime trigger-layer resolution (`mod.rs::resolve_trigger_layers`): resolved every slot of each matched payload, whatever the document or a later trigger redeclared (R1-F2.1, R1-F2.2); fixed
                - pre-flight collection (`pending_templates_for`): filtered questions against the document only, never a later trigger, and judged a union document by any arm's names; fixed and now shares the runtime selection (R1-F2.5)
                - document `$schema` layer resolution: highest layer, never replaced, so all its slots are effective; clean
                - baseline / catalog / kind definitions: refuse slots; clean
                - every downstream consumer goes through those two sites: `md schema validate` and `md clean` (`approve_slot_reads` then `effective_for`), compose pre-flight (`compose_slot_approvals`), the reference check, DMLS (`overlay/schema.rs` `effective_for` + `templated` from `pending_templates_for`; slot jobs run `effective_for` with a session); fixed through the shared selection (R1-F2.6 to R1-F2.8)
                - slot cache key (`slots/cache.rs::resolution_key`): would have keyed a pruned resolution like the whole one when the dropped property held no slot; fixed by adding the omitted names to the trigger template identity (R1-F2.9)
                - pending reporting: a payload whose only slots were discarded still raised `UnresolvedSlots` without a session, so DMLS called the schema templated; fixed: such a payload resolves through `resolve_without_slots` and is not listed by `pending_templates_for` (R1-F2.10)
                - named-type imports: a slot in a stored definition that only discarded properties reach was still collected (`collect_slots` walks every definition); fixed: pruning drops unreached definitions (R1-F2.11)
                - `md schema triggers` `pending: true`: a fact about the trigger file, not a document's layering; left unchanged
        - repairs
                - `simplified-schema` `PendingSchema::without_properties` (drops named top-level properties and the definitions only they reach before resolution; `omitted_properties`, `is_union_arm`, `resolve_without_slots`); `slots::omitting` scope consumed by the replayed entry's lowering in `apply_active`; root-union arm sites call `apply_active_to_arm`, which marks the template `union_arm`; `DefinitionTable::retain`; `atom_refs` now also walks `Constraint::Property` declarations
                - `darkmatter` `mod.rs`: `declared_by_json` / `declared_by_template` / `declared_by_payload` and the one selection `discarded_by_higher_layers`, used by `pending_templates_for` and `resolve_trigger_layers`; `resolve_trigger_layers` collects every matched payload first, then resolves each template without what higher layers redeclare; `resolve_pending_payload` adds the omitted names to the cache identity and resolves a slot-less survivor without the session
                - docs: `darkmatter/docs/topics/schemas/schema-slots.md` "A replaced property's slots never run"; `simplified-schema/docs/topics/language/slots.md` "Resolving only the properties that survive"; skills `darkmatter/schema.md`, `simplified-schema/SKILL.md`
        - tests (each failed with the selection disabled, except the controls)
                - `darkmatter/lib/tests/l1/dynamic_schema_slot_precedence.rs` (new): compose R1-F2.1–4, an earlier trigger never discards a later one, surviving sibling slot, root-union every-arm rule, named-type definition dropped / kept, pre-flight asks nothing / asks once, no pending template, validation valid and asks nothing, no `UnresolvedSlots` without a session, cache identity across an edit dropping the override (fails when the omitted names leave the identity)
                - `darkmatter/cli/tests/l1/dynamic_schema_slots.rs`: `md compose` exit 0 with empty read set and `md schema validate` valid for R1-F2.1 and R1-F2.2, `md clean` no findings, approval-required control
                - `darkmatter/dmls/tests/l1/schema_slots_editor.rs`: no slot diagnostic for a redeclared trigger slot; approval-required control
                - `simplified-schema/lib/tests/l1/dynamic_slots.rs::surviving_properties` (4 tests)
        - decisions
                - prune inside the library replay (an `omitting` scope) rather than resolve and discard: discarded properties never reach substitution, so no placeholder value and no lazy validation are needed
                - a union layer replaces a property only when every arm declares it, matching `merge_layer`'s per-arm copy
                - pre-flight cannot see the other arms of a root-union document that is itself a template (the template is the first slotted arm), so it treats that document as declaring nothing; runtime uses the resolved JSON and is exact. Residual: pre-flight may over-ask in that one case; nothing is read
                - a matched payload whose resolution fails on demand (programmatic registries only) now fails before an earlier template resolves, because every matched payload is collected first
        - follow-up: root-union template document pre-flight (R1-F2.12)
                - defect: a root-union document `$schema` that is itself a template reported only the arm whose slot stopped resolution, and `declared_by_template` treated a union arm as declaring nothing, so pre-flight asked about (and `pending_templates_for` listed) a trigger slot whose property every arm redeclares, while runtime, reading the resolved `anyOf`, discarded it. This closes the residual recorded in the R1-F2 decisions above
                - repair (library): `resolve.rs::pending_union_arm` intercepts the template error at every root-union arm site (inline and referenced arms, document and standalone unions) and marks the template (`Template::mark_union_arm`) with the names every arm declares: arms already lowered from their JSON `properties`, the pending arm from its template, later inline arms from their parsed shape, later referenced arms as they resolve (a template's unevaluated). No slot is evaluated. `apply_active_to_arm` no longer marks the arm itself; the union does, once it knows every arm. New public `PendingSchema::declared_properties()`
                - also fixed: a referenced root-union arm that was a template was reported as a whole single-object template, so a host could have treated that one arm's properties as the union's (and `without_properties` would have pruned that arm alone); it is now marked as a union arm like an inline one
                - repair (darkmatter): `declared_by_template` reads `declared_properties()`, so `discarded_by_higher_layers` gets the same document names before and after resolution
                - tests (red with the fix disabled, except the controls): `dynamic_schema_slot_precedence.rs::template_union_document` (pre-flight asks nothing and reads nothing when every arm redeclares `os`; only the document template is pending; composition succeeds with a pure `enum({{ ['Linux', 'Windows'] }})` slot and the trigger data unread; control: one arm only, pre-flight asks once about `a.trigger.yaml`); `simplified-schema` `dynamic_slots.rs::surviving_properties::a_root_union_arm_template_declares_what_every_arm_declares` (first/later slotted arm, a referenced arm that does and does not declare `os`, a single object)
                - docs: removed the pre-flight limitation bullet from `darkmatter/docs/topics/schemas/schema-slots.md` and stated the exact rule; `simplified-schema/docs/topics/language/slots.md` documents `declared_properties()`; skills `darkmatter/schema.md`, `simplified-schema/SKILL.md` updated
                - decisions: compute the names at the union, when the template error is met, rather than in Darkmatter from the raw `$schema` value (which would be a second reader of arms and miss referenced arms); read later inline arms as parsed (import expansion never adds a top-level property) and resolve later referenced arms, matching what runtime's `declared_by_json` sees in each `anyOf` arm, including empty for an arm that is itself a union
                - verification: darkmatter `just test dynamic_schema_slot_precedence` 21/21, `just test slot` 288/288, `just test trigger` 650/650, `just lint` clean; simplified-schema `just test` 1640/1640 + 1437/1437 minimal, `just lint` clean
- work completed for 'R1-F2: Overridden trigger slots still evaluate' at 12:56:55
- starting the work on 'R1-F3: Inline and trigger slot failures lose authored locations' at 12:56:55
        - defect class: a schema value reaches slot resolution detached from the authored text it came from, so its failures cannot be projected to a proven position. The library parsed an inline `$schema` value (document frontmatter, trigger payload, YAML document root) from its YAML value alone, so slot spans were relative to each type string, and no failure carrier held the text they belonged to; `report_diagnostics` could position only slots whose origin named a schema file the resolution had read
        - sibling sites checked (route → result before the repair)
                - document inline `$schema` (compose, `md schema validate`, `md clean`, pre-flight `pending_templates_for`): file, line, column, span all null — defect
                - document inline root-union arms: null — defect
                - referenced `$schema` file, imported named type (`Name@file`): positioned — clean (controls R1-F3.5–8)
                - trigger inline payload: trigger file named, line, column, span null — defect; trigger payload reference (`$schema: ./x.yaml`): positioned — clean
                - embedded trigger bundles: a slot there is a load failure, never a pending template, so there is no slot-resolution carrier — not in this class
                - whole YAML document root `$schema` (`yaml_documents::prepare_yaml_contract`): null — defect
                - library facade `SimplifiedSchema::try_from_markdown` with a root union holding a file arm: the inline arms were re-parsed without projection, so their slots were spanned in the type string while the rest of the layer used document coordinates — defect (latent; no host consumed it)
                - data-contract nested templates (`resolve_contract_slots`): the failure is reported at the reading slot, positioned, with code `dm.schema.slot_failed` — clean for position
                - DMLS worker, unsaved buffers: the worker's `Markdown` is built from the buffer text, so the same projection applies; the editor-side text search (`authored_slot_span`) matched the first `{{ … }}` with the same trimmed expression, so a second identical slot or a quoted carrier with escapes was ranged wrongly or fell back — defect
                - failure carriers: shape errors, approval-required, `dm.data.*`, rejected patterns, inadmissible defaults all go through `slot_resolution_records` — fixed together; the cycle carrier built at `resolve_in_mode` dropped the pending's texts — fixed; a refused read's sub-span inside the expression was offset from the expression's start even through a YAML escape — fixed (falls back to the whole expression)
                - process-wide caches: the slot resolution key held only `span.start`; the trigger payload memo did not key on the trigger file's text, so an edit above an unchanged payload served stale spans — both fixed
        - repairs (simplified-schema)
                - `resolve::AuthoredSchemaValue { file, text, mapping }` and `resolve_authored_yaml_schema` / `resolve_authored_schema_with_roots`: the value is located once under its `$schema` key with the frontmatter (tolerant) grammar and verified node for node against the parsed value; only when every inline schema in it projects is it used. `resolve_inline_mapping` and `resolve_root_union` project each parse (first pass and replay) through `LocatedSchemaValue` (`simplified/source.rs`), reusing `project_located_schema_suggestions`, the located mapping the Markdown factory already used (human review #6), so plain, quoted, and block carriers share one mapping
                - `PendingSchema::authored_source()` / `slot_text(&slot)`; `SlotResolutionFailure::of_pending` keeps the same text and answers `slot_text`; a trigger payload's authored file is the canonical path its slots' origin names
                - the Markdown factory's root-union-with-file-arm path projects its inline arms through the same located value
        - repairs (darkmatter)
                - `schemas/mod.rs::authored_document_schema` supplies the document's loaded text and frontmatter block to `effective_for` and `pending_templates_for` (same keys for pre-flight pins and runtime), only when the `$schema` mentions `{{` and never for an override
                - `LoadedTrigger::text` (discovery's read) feeds `resolve_file_payload`, and is part of the payload memo key; `yaml_documents` passes the YAML document's text
                - `session.rs::failure_of` and the resolution-level `cycle_error` build failures with `of_pending`; the slot cache key adds `span.end`
                - `report_diagnostics`: one `failure_text` / `template_slot_text` lookup and one `slot_record` builder for failures and pending records; `proven_slot_span` checks the bytes are a `{{ … }}` and refuses an offset sub-span through escapes
                - DMLS `diagnostics/slots.rs`: `authored_slot_span` removed; a local slot is ranged at the record's span when `template_slot_text` shows the resolution read the buffer's current text, else at the `$schema` value
                - fixed a guard red at HEAD from R1-F1: `type_scope_ownership_guard` counted `buffers::unguarded` in `resolve_contract_slots` as an unreviewed `TypeScope::unguarded`; allowlisted with its reason and a baseline snippet
        - tests (each red before the repair unless marked control or guard)
                - `darkmatter/lib/tests/l1/dynamic_schema_slot_positions.rs` (new, 27): R1-F3.1–4 exact file/line/column/span for compose and validate; controls R1-F3.5–8 (green before, retained); carriers single-quoted, double-quoted with escapes, `|-`, `>-` folded across lines, `|2+` (R1-F3.10–11); inline approval-required (R1-F3.12), `dm.data.contract_mismatch` with `data_location` (R1-F3.13), pattern rejected (R1-F3.14), inadmissible default (R1-F3.15), root-union arm (R1-F3.16), identical slots at their own bytes (R1-F3.17), edit above an unchanged trigger payload (R1-F3.18, red with the memo key's text removed), referenced trigger payload (R1-F3.19, control), override `$schema` stays null (R1-F3.20, control), cycle at the outer slot (R1-F3.21, control), YAML document root `$schema` (R1-F3.22), refused binding plain and through an escape (R1-F3.25; the escaped case red with the sub-span check disabled), contract template failure at the reading slot (R1-F3.26, control)
                - `darkmatter/cli/tests/l1/dynamic_schema_slots.rs`: `md compose` and `md schema validate` JSON for the inline slot (R1-F3.1–2) and the trigger slot (R1-F3.3–4), `md clean` JSON for the inline slot (R1-F3.24)
                - `darkmatter/dmls/tests/l1/schema_slots_editor.rs`: inline slots in a double-quoted carrier and a second identical slot each ranged at their own bytes (R1-F3.9, red against the old text lookup); an unsaved edit above `$schema` moves the range (R1-F3.23, guard: green with the old lookup too); a trigger slot's related information points at its position in the trigger file (R1-F3.27)
                - `simplified-schema/lib/tests/l1/dynamic_slots.rs::authored_source` (7): document carrier spans, failure keeps the text, trigger payload file and spans, root-union arm, unproven text claims nothing, no text keeps type-string spans, Markdown factory root union with a file arm (red with its projection disabled)
        - decisions
                - project in the library at parse time (first pass and every replay) rather than in Darkmatter after the fact: slots are only identifiable by their spans, and two identical expressions at the same type-string offset are indistinguishable afterwards
                - prove before projecting: the located `$schema` entry must verify against the parsed value, and a dry projection must succeed for every inline arm; otherwise no text is attached and every position stays null (an override, a YAML alias, a value the text does not say)
                - the frontmatter (tolerant) locator grammar for the containing mapping, because other frontmatter keys may use presentations the strict schema-document grammar rejects; the verification of the `$schema` subtree keeps the result proven
                - a document's slots keep `origin: None` (they still evaluate in the request context); the failure's authored text names the document file, so the record's file comes from the text, not the origin
                - DMLS publishes the library's span only when the failure read the buffer's current text, never a stale one; alternatives were the old text search (wrong for duplicates and escapes) or always trusting the span (wrong between an edit and its re-resolution)
                - a refused read's sub-span is kept only when the authored expression is byte-for-byte its text; through an escape the whole expression is the proven range, rather than projecting the sub-span through the scalar map (a second mapping for one rare case)
                - observed, not repaired here: a contract template's own slot failure reaches the record as `dm.schema.slot_failed` with the inner code only in the message (a typed-cause question for M3, not a position one)
        - verification: darkmatter `just test slot` 323/323, `diagnostic` 427/427, `trigger` 657/657, `report_diagnostics` 34/34, `schema_slots_editor` 19/19, `dynamic_schema_slot_positions` 27/27, `yaml_document` 91/91, `payload_memo` 29/29, `clean` 676/676, `guard` 269/269, `effective_schema_layering` 6/6, `just lint` clean; simplified-schema `just test` 1647/1647 + 1437/1437 minimal, `just lint` clean
        - follow-up: typed cause of a contract-template slot failure (R1-F3.28)
                - defect: a slot whose `load_data` contract named a template whose own slot failed published `dm.schema.slot_failed`, the inner code only in the message; only a cycle was carried through (`session::is_cycle`)
                - repair: `host::contract_template_failure` finds the `SchemaError::SlotResolution` inside `DataError::ContractSchema`; `host::failure_code` gives the outer `SlotFailure` the inner code (`nested_resolution_code`: cycle, then grant-dependent, then `dm.iteration.*`, then the first failure, since this code also decides caching, reference-check deferral, and limit handling); `report_diagnostics::with_expression_cause` publishes one record per inner record with its code, `data_location`, `grants_needed`, and reference fields, while the outer slot's `source`, `span`, and message stay primary; `session::is_cycle` removed (subsumed)
                - repair: `resolve_contract_slots` wrapped no unusable value: `substitute`'s `SchemaError::SlotResult` reached the contract bare, so it is now `SlotResolutionError::Schema` in a `SlotResolution`, as the library's own replay reports it; the result-kind code mapping moved to `codes::slot_result_code`, shared by the host and the report
                - sibling sweep: cycles through contracts already kept `dm.schema.slot_cycle` (`cycle` module green before and after); `validate_schema` never resolves a template (its `DarkmatterSchemas` has no slot session), so no slot failure crosses that boundary: a page with a template fails as `UnresolvedSlots` flattened into `ExpressionError::Other` (`dm.expression.evaluation_failure` in a body, `dm.schema.slot_failed` in a slot); whether it should resolve through the active session is a behavior decision, not repaired here; DMLS publishes the same records (`template_slot_records`) and inherits the repair
                - tests: `dynamic_schema_slot_positions::contract_template` now 3 tests, each red before the repair (wrong shape, approval required with `grants_needed`, `dm.data.*` with its `data_location` file and line), compose and validation each; the earlier position-only control folded into them
                - decision: no secondary location; `Diagnostic` has no related-location field and adding one changes all 17 output schemas, so the inner slot's position is not published
                - verification: darkmatter `just test dynamic_schema_slot_positions` 29/29, `slot` 325/325, `contract` 231/231, `report_diagnostics` 34/34, `guard` 269/269, `just lint` clean
        - orchestrator decision: `validate_schema` naming a page whose schema is a template keeps failing closed as unresolved (it builds without a slot session); spec S4 notes re-entry needs the shared stack but no requirement mandates resolving there, and failing closed matches "cannot report success"; recorded as a human-review item
- work completed for 'R1-F3: Inline and trigger slot failures lose authored locations' at 13:52:03
- starting the work on 'R1-F4: Unsaved data buffers are ignored outside YAML and JSON' at 13:52:03
        - defect class: an editor host filters authoritative unsaved text by file extension before deciding what a resolution depends on
                - site: `OverlayState::set_schema_buffer` returned unless the path was YAML or `.json`, so the one `SourceBuffers` snapshot (`TriggerRegistries::source_buffers`) every slot job, YAML-document preparation, and standalone resolution reads through never held any other open buffer; the slot job then re-read the disk and installed a completed, stale snapshot
                - what a resolution depends on is known only once it runs (a `load_data` file of any format, a Markdown file `frontmatter()` reads, a contract import of any extension), so the filter cannot be correct for slots
        - repairs
                - `dmls/src/overlay/mod.rs` `set_schema_buffer`: the extension filter is removed; every open buffer is recorded, and kind-registry/contract staleness keeps its own path checks (`joins_kind_sources` still admits only YAML for a never-read path inside a root, `kind_sources`/`deps` lookups are by path), so trigger and kind routing stay as they were for paths they never read
                - `lib/src/markdown/schemas/slots/cache.rs` `InputCheck::capture`: a read served from an open buffer keeps no `stat` (R1-F4.10); before, the disk file's `stat` was recorded for buffer bytes, so after the buffer closed `holds(true)` trusted the unchanged `stat` and the worker served the resolution of the unsaved text (true of YAML too)
                - doc comments corrected with the behavior: `set_schema_buffer`, `ServerState::note_schema_buffer`, `TriggerRegistries::source_buffers`/`set_buffer`, `Preparation::buffers`, `SlotJob::buffers`, `standalone_named_types`
                - docs: `docs/topics/schemas/schema-slots.md` ("What refreshes a resolution": every format, close reverts, dirty buffer gets no automatic grant), `docs/lsp/architecture.md` (a job carries every open buffer), `.claude/skills/darkmatter/dmls.md` (invalidation note: every buffer, never filter by extension)
        - child dispositions (tests in `dmls/tests/l1/schema_slots_editor.rs` module `unsaved_buffers`, real server, slot worker, filesystem, persisted grant, block YAML carrier; each first rejects Haiku on disk, the only edit adds Haiku to the buffer, the disk file is asserted unchanged)
                - R1-F4.1 JSON5: fixed; `unsaved_buffers::json5` red before (stale `"Haiku" is not one of "Linux"`), green after
                - R1-F4.2 JSONL: fixed; `unsaved_buffers::jsonl` red before, green after
                - R1-F4.3 TOML `os` array: fixed; `unsaved_buffers::toml` red before, green after
                - R1-F4.4 CSV `os` column mapped with `map(…, row => row.os)`: fixed; `unsaved_buffers::csv` red before, green after
                - R1-F4.5 TSV: fixed; `unsaved_buffers::tsv` red before, green after
                - R1-F4.6 YAML control: retained; `unsaved_buffers::yaml` green before and after (replaces the former `an_unsaved_edit_of_the_data_file_refreshes_the_snapshot`, now a macro row)
                - R1-F4.7 JSON control: retained; `unsaved_buffers::json` green before and after
                - R1-F4.8 Markdown file read by `frontmatter()` from a slot: fixed by the same repair; `unsaved_buffers::markdown` red before, green after
                - R1-F4.9 contract import with an unusual extension (`Os@./os.schema`): fixed by the same repair; `unsaved_buffers::an_imported_schema_with_an_unusual_extension_is_read_from_its_buffer` red before, green after
                - R1-F4.10 closing a buffer reverts to disk: fixed in `InputCheck::capture`; `unsaved_buffers::closing_the_buffer_reverts_to_the_disk_contents` red before both repairs and still red with only the DMLS repair (empty diagnostics: the stat-trusted cache served the closed buffer's result), green after; lib unit `slots::session::tests::a_read_of_an_open_buffer_is_rehashed_once_the_buffer_closes` red before, green after
                - R1-F4.11 dirty non-YAML buffer under the automatic grant: fixed by the same repair (`AutoGrant::clean_content` already compared `buffers::text`, but never saw a TOML buffer); `unsaved_buffers::a_dirty_buffer_loses_the_automatic_grant` red before (the dirty buffer's stale read was granted and judged), green after (`dm.schema.slot_approval_required`)
                - guard: `format_rows!` emits one `#[test]` per row and a `FILES` list; `unsaved_buffers::every_loadable_format_has_a_row` requires a row for every `biscuit_file::DataFormat` a slot can read, through an exhaustive match, so a new format fails to compile until it has a row
        - sibling sweep (clean)
                - router `didOpen`/`didChange`/`didClose` call `note_schema_buffer` for every path, and `slots().invalidate_paths` there, on save, and on watched events is extension-neutral; the only filter was the overlay setter
                - slot watch registration (`workspace/watch.rs::slot_watch_registration`) watches each recorded path whatever its extension; `slot_reads` records every local observation
                - data loader: every format reads through the one `simplified_schema::resolve::buffers::read` in `data/load.rs` before `parse_text`; no branch reads the disk directly. The Markdown readers (`frontmatter`, `markdown_title`, siblings) share `fs.rs`'s `buffers::read`
                - dirty-buffer grant (`grants/auto.rs::clean_content`) and dependency rehash on open buffers (`cache.rs::holds`, human_review #68) are extension-neutral and kept; #68's invalidation is now also correct after a close (R1-F4.10)
                - `didSave`'s `is_yaml_path` gate on `invalidate_trigger_inputs` is trigger routing, not slot dependency: it is left extension-specific (a saved buffer's bytes equal the buffer already read; a client watcher reports the save of any other file)
        - observed, not repaired (outside the class)
                - a Markdown document's non-slot schema assembly (`schema_for`) reads `$schema` targets and imports from disk; documented as such in `docs/topics/schemas/dmls-schema-support.md`
                - a data file that exists only in an editor buffer (never saved) is not found by `load_data`: reference resolution checks the disk before the read
        - decisions
                - one buffer set for every consumer rather than a second, slot-only set: alternatives were a separate slot buffer map in `OverlayCache` (two snapshots to keep in step on open/change/close) or widening the filter to a list of data extensions (the same defect for the next format, Markdown, and imports); cost accepted: a Markdown keystroke during an in-flight registry preparation now records a change, which keeps that preparation's root reads out of the shared cache once
                - no `stat` for buffer-served reads rather than clearing the slot cache on close: the check stays the one rule ("unchanged `stat` stands for unchanged bytes only when the bytes came from the file")
                - kept the names `set_schema_buffer`/`note_schema_buffer` (tests and the skill name them); their docs now say every buffer
        - verification: darkmatter `just test schema_slots_editor` 30/30, `overlay` 278/278, `slot` 337/337, `watch` 51/51, `yaml_document` 91/91, `registry` 246/246, `buffer` 42/42, `trigger` 657/657, `a_read_of_an_open_buffer_is_rehashed` 1/1; `cargo nextest run -p darkmatter-lsp --features effects-instrumentation` (L1 tiers) 1799/1799; `just lint` clean; every new test under 0.5 s
        - orchestrator decision: two adjacent behaviors are outside this defect class and retained — a Markdown document's static (slot-free) `$schema` import assembly still reads disk (pre-existing, documented), and a data file that exists only as an unsaved, never-saved buffer is not found by `load_data` (path resolution checks disk first; it would need a buffer-backed candidate resolver); recorded as human-review items
- work completed for 'R1-F4: Unsaved data buffers are ignored outside YAML and JSON' at 14:02:50
- starting the work on 'R1-F5: Stale completion candidates have no stale marker' at 14:02:50
        - defect class: an editor response built from a schema snapshot omits that snapshot's stale/pending status
        - shared status source: `providers/frontmatter.rs::slot_note` (from `SlotStatus`: `unresolved` → pending note, else `refreshing` → stale note); hover's `with_slot_marker` now reads it, so hover and every other response share one wording
        - R1-F5.1 completion response (fixed): `frontmatter::completion` passes every ordinary candidate (`schema_completion`: scalar, union value, union item, array item, suggestion, property name, file path, `style.*`) through `with_slot_notes`, which appends the note to `detail` (` · ` after an existing detail, never replacing it); labels, text edits, and order are untouched; nothing new runs on the request path (the note is read from the already-installed `SlotStatus`)
                - `schema_slots_editor::while_a_refresh_is_held_the_last_good_snapshot_is_served_marked_stale` now asserts, beside its hover and diagnostic checks: current completion has no detail; held completion has the stale note on every item and equals the current list once `detail` is removed; after release the list (now with Haiku) has no note; red before the fix (no detail), green after
        - R1-F5.2 hover (clean, retained): frontmatter hover still carries the italic note; wording unchanged (now built from `slot_note`)
        - R1-F5.3 diagnostic status (clean, retained): one `dm.schema.slot_pending` information diagnostic during a held refresh; the existing assertions still pass
        - R1-F5.4 array-item completion (fixed by the boundary repair): `stale_responses::an_array_item_candidate_from_a_refreshing_snapshot_is_marked_stale` (`enum({{ … }})[]`), red before, green after
        - R1-F5.5 union value completion (fixed by the boundary repair): `stale_responses::a_union_value_candidate_from_a_refreshing_snapshot_is_marked_stale` (`enum({{ … }}) | number`), red before, green after
        - R1-F5.6 union item completion (fixed by the boundary repair): `stale_responses::a_union_item_candidate_from_a_refreshing_snapshot_is_marked_stale` (`(enum({{ … }}) | number)[]`), red before, green after
        - R1-F5.7 suggestion-derived completion (fixed by the boundary repair): `stale_responses::a_suggestion_from_a_refreshing_snapshot_is_marked_stale` (`string(suggest({{ … }}))[]`), red before, green after
        - R1-F5.8 pending literal candidates, human_review #73 (fixed): while the first resolution is held, `enum(Haiku, {{ … }})` offers `Haiku` with the pending note, so the literal parts never look complete; the same note covers approval-required and failed slots (both serve only literal parts); `stale_responses::literal_candidates_offered_before_the_first_resolution_are_marked_pending`, red before, green after, with a resolved control (no note)
        - R1-F5.9 body expression hover (fixed): `dsl::hover` built the `{{ … }}` interpolation hover, a collection opener's hover, and a `::shell` hole's type line from the document's schema (enum members, declared types) without the marker; each now goes through `frontmatter::with_slot_marker`; `stale_responses::a_body_expression_hover_from_a_refreshing_snapshot_is_marked_stale` (current: Linux, no note; held: note; released: Haiku, no note), red before, green after
        - R1-F5.10 quick fix "Add required key" (fixed): `code_actions::add_missing_required_keys` scaffolds a `literal(...)` value from the served schema, which a slot may supply; when it scaffolds one and the slots are not current, the title ends with the note in parentheses; `stale_responses::a_quick_fix_inserting_a_slot_value_from_a_refreshing_snapshot_is_marked_stale` (strict `.dmls.toml`, `literal({{ load_data(…)[0] }}; required)`), red before, green after; pending cannot reach it (an unresolved template is not judged, so no missing-required diagnostic exists, and its template shape drops slot constraints)
        - sweep, shown independent of resolved slot values (clean)
                - completion resolve: not advertised (`CompletionOptions::resolve_provider` unset); every item is complete when returned
                - signature help and inlay hints: not advertised by `capabilities.rs`
                - semantic tokens: syntactic only (expression and directive token families from document text); slots never fill a type keyword, so which values are expressions does not depend on a resolution
                - other code actions (grant, create file, migrate style, close block, key reference, concatenation): built from diagnostic payloads and document text, not schema values
                - body/collection key completion (`expression_property_keys`): property names only, which slots never supply; the frontmatter key candidates are still marked, since they come from the served shape
                - document symbols, definition, document links, folding: from the AST and `$schema` file references, not resolved values
                - whole YAML documents: slots are refused there (`slot_prohibition_code`), so `slots` is always `None`
        - decisions
                - note in `detail`, not `documentation`: `detail` is the field clients show beside the focused item; alternatives were `documentation` (hidden until expanded, and two content shapes to append to) or a label suffix (would change the label and the filter text, which the finding forbids)
                - decorate every ordinary frontmatter candidate rather than only value candidates: they are all projected from the served shape, matching hover, which already marks every frontmatter hover; alternative was value candidates only, which would leave pending key candidates looking complete
                - leave schema-authoring candidates (`type_definition_argument`, `meta_schema_completion`) undecorated: they come from the SimplifiedSchema grammar catalog, never from a resolution; alternative was decorating everything, which adds noise while writing the template itself
                - pending note for every unresolved state (first load, approval required, failed): all three serve only the literal parts, and hover already used that note for all three
                - mark only schema-reading DSL hovers (interpolation, collection opener, `::shell` hole), not directive file targets, wiki, or literal hovers, which do not read the schema
                - quick fix: mark the title only when a literal was scaffolded; a bare key scaffold inserts nothing a slot supplied
        - docs: `docs/topics/schemas/schema-slots.md` "In the editor" table and example; `docs/lsp/features.md` 4.11 table and the meaning of "marked"; skill `.claude/skills/darkmatter/dmls.md` "Schema slots"
        - verification: darkmatter `just test schema_slots_editor` 37/37, `completion` 286/286, `hover` 152/152, `frontmatter` 1191/1191, `slot` 344/344, `code_action` 10/10, `dsl` 76/76; `just lint` clean; every new test under 0.4 s; red-before proven for the completion cells by restoring the undecorated `schema_completion` call, and for the hover and quick-fix cells by running them before the repair
- work completed for 'R1-F5: Stale completion candidates have no stale marker' at 14:22:58
- orchestrator reconciliation at 14:24–14:36
        - inspected the combined diff of `frontmatter_interpolation.rs` (`with_schema_excluded` applied inside `interpolate_frontmatter_impl`) and `dmls/src/overlay/mod.rs` (extension filter removed); both match the subagents' reports
        - concurrent commits by the author during the cycle (`cc57fdb4c`, `501dca87f`, `f3d003407`, `4a32670e3`, `597d464a0`, `410fb26b8`) captured parts of this cycle's work; `597d464a0` also carried the author's unrelated `as_markdown` state-inheritance changes (`compose/nested.rs`, `compose/transclusion/*`, `context/options.rs`, `pipeline/mod.rs`, `schemas/partials/functions.yaml`, `nested_composition.rs`, `compose_nested_markdown.rs`, `preflight_child_state_parity.rs`), which are not part of these repairs but are covered by the full-suite run below
        - subagent rule breaks reported honestly: two subagents each used `--no-fail-fast` once for a red-proof run of their new tests (not a full suite)
        - one guard (`type_scope_ownership_guard::static_presence_decisions_read_the_shared_type_scope`) was red at an intermediate commit after R1-F1 (`buffers::unguarded` in `resolve_contract_slots`); the R1-F3 subagent added an allowlist entry with its reason; `just test guard` 269/269

### Completion Report

- **outcome: complete**; elapsed 2 h 46 min (11:49:32 → 14:36)
- parent findings: total 5 = fixed 5 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - R1-F1 fixed (12 required children fixed, 5 controls clean, 2 discovered defects fixed, 1 sibling clean)
        - R1-F2 fixed (2 required children fixed, 2 controls clean, 8 consumer/sibling rows fixed including the root-union pre-flight gap R1-F2.12)
        - R1-F3 fixed (5 required children fixed, 4 controls clean, 19 sibling rows fixed or clean, including the M3 typed-cause defect R1-F3.28)
        - R1-F4 fixed (5 required children fixed, 2 controls clean, 4 sibling/discovered rows fixed including close-reverts-to-disk R1-F4.10)
        - R1-F5 fixed (1 required child fixed, 2 controls clean, 7 sibling rows fixed)
- open finding or child IDs: none
- decisions the spec does not settle were made, tested, and added to `review-1.md` `human_review_items` (12 new items; `human_review: true`)
- no performance measurement was required by these findings; `deferred_perf_measurement` not set
- verification (final, after every change, no subagent running)
        - `darkmatter/`: `just test` → 15584 passed, 10 skipped, 0 failed, 0 slow (all findings)
        - `darkmatter/`: `just lint` → clean (all findings)
        - `simplified-schema/`: `just test` → 1647/1647 plus minimal-feature 1437/1437 (R1-F1 read guard, R1-F2 pruning and `declared_properties`, R1-F3 authored projection)
        - `simplified-schema/`: `just lint` → clean
        - `just check-tier-coverage darkmatter` → 0 stranded tests
        - consumers: `cargo check --tests -p claudine-cli -p claudine-gen -p claudine -p messenger` → compiled cleanly (public simplified-schema/darkmatter surface changes)
        - cross-OS: `just cross-check darkmatter --os windows` → pass; `just cross-check simplified-schema --os windows` → pass (path-sensitive changes to listing grant roots, buffer `stat` capture, and authored-path projection)
- changed files for this cycle (committed by the author mid-cycle or still uncommitted)
        - simplified-schema source: `lib/src/resolve/buffers.rs` (F1), `lib/src/{resolve.rs,slots.rs,slots/pending.rs,slots/substitute.rs,simplified/definitions.rs}` (F2, F3), `lib/src/{errors.rs,schema/layer.rs,simplified/mod.rs,simplified/source.rs}` (F3)
        - darkmatter lib source: `compose/frontmatter_interpolation.rs`, `compose/expression/{resolve_ctx.rs,read_gate.rs,data/load.rs,data/runtime.rs,functions/fs.rs}`, `schemas/slots/grants/targets.rs`, `work_counters.rs` (F1); `schemas/mod.rs`, `schemas/resolve.rs` (F2, F3); `compose/report_diagnostics.rs`, `schemas/triggers/{assemble,discovery}.rs`, `schemas/yaml_documents/mod.rs`, `schemas/slots/{codes,host}.rs` (F3); `schemas/slots/{session,cache}.rs` (F1–F4)
        - DMLS source: `diagnostics/slots.rs` (F3); `overlay/{mod,registry}.rs`, `router.rs`, `slot_jobs.rs` (F4); `providers/{frontmatter,dsl,code_actions}.rs` (F5)
        - tests: `lib/tests/l1/dynamic_schema_slot_read_order.rs` (F1, new), `dynamic_schema_slot_precedence.rs` (F2, new), `dynamic_schema_slot_positions.rs` (F3), `type_scope_ownership_guard.rs`, `main.rs`; `lib/src/markdown/schemas/slots/{session_tests,tests}.rs`; `cli/tests/l1/dynamic_schema_slots.rs` (F1–F3); `dmls/tests/l1/schema_slots_editor.rs` (F2–F5); `simplified-schema/lib/tests/l1/dynamic_slots.rs` (F2, F3)
        - docs: `darkmatter/docs/topics/schemas/schema-slots.md` (F1–F5), `darkmatter/docs/lsp/{architecture,features}.md` (F4, F5), `simplified-schema/docs/topics/{selecting-schema-files.md,language/slots.md}`
        - skills: `.claude/skills/darkmatter/{schema,requests,dmls}.md`, `.claude/skills/simplified-schema/SKILL.md`

## Implementation of Review Findings #2

> **started at:** 2026-11-08T14:49:39-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-dynamic-schema-types/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- the review's `implemented` flag was already `false`; it was left `false` before repairs began
- the review has no `human_in_the_loop` items, no blocked findings, no recurrence, and no robustness-matrix finding (the grant-store loader was re-checked by the reviewer and found clean)

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R2-F1 | Cache reuse omits implicit and transitive request inputs | fixed | every child below fixed or clean; dependency summary now recorded during evaluation (`slots/dependencies.rs`) instead of derived from `pending.slots()` |
| R2-F1.1 | Clock dispatch (14 date functions): changed captured day must not reuse an old date-dependent schema | fixed | `lib/src/markdown/schemas/slots/session_tests.rs` `consumed::a_new_captured_day_is_a_miss_for_a_date_function`; hit control `another_time_on_the_same_captured_day_is_a_hit`; all 14 date functions share the one `call_function` dispatch that records captured calls |
| R2-F1.2 | `has_command`: executable removed between sessions → new session observes removal | fixed | `consumed::a_removed_executable_is_a_miss_for_has_command` (temp executable, cold control) |
| R2-F1.3 | `has_binary` (shared command probe): same | fixed | `consumed::a_removed_executable_is_a_miss_for_has_binary` |
| R2-F1.4 | `ipv4`: changed captured addresses → new member | fixed | `consumed::changed_captured_ipv4_addresses_are_a_miss` |
| R2-F1.5 | `ipv6`: changed captured addresses → new member | fixed | `consumed::changed_captured_ipv6_addresses_are_a_miss` |
| R2-F1.6 | `package`: changed captured package → new value | fixed | `consumed::a_changed_captured_package_is_a_miss` |
| R2-F1.7 | `package_area`: changed captured area → new value | fixed | `consumed::a_changed_captured_package_area_is_a_miss` |
| R2-F1.8 | `has_agentic_cli`: changed captured installation → new literal | fixed | `consumed::a_changed_agentic_cli_scan_is_a_miss` |
| R2-F1.9 | Nested contract's captured (clock) input propagates to outer cache entry | fixed | `consumed::a_nested_contracts_captured_day_is_part_of_the_outer_entry`; contract-memo replay in `DataRuntime::prepare_contract` (extra instance) |
| R2-F1.10 | Nested contract's volatile (`remote_vendor`) class propagates to outer entry | fixed | `consumed::a_nested_contracts_volatile_call_is_never_served_to_another_command` |
| R2-F1.11 | Later root-union resolution round classified before sharing | fixed | `consumed::a_later_root_union_rounds_volatile_call_is_never_served_to_another_command`; every round reaches `SlotHost::resolve`, so no simplified-schema change |
| R2-F1.12 | Clean control: direct volatile slot resolves once per command | clean | `reuse::a_volatile_slot_resolves_once_per_command` still passes |
| R2-F1.13 | Clean control: explicit environment input change is a miss | clean | `reuse::a_changed_environment_input_is_a_miss` still passes |
| R2-F1.14 | Clean control: file/content dependencies and read-set replay | clean | `reuse::` source/data/import/missing-file/failure/read-set-replay tests still pass |
| R2-F1.15 | Clean control: permission acceptance of a cache hit | clean | `reuse::narrower_grants_are_not_served…`, `an_uncovered_cache_entry_is_refused_before_its_inputs_are_reread` still pass |
| R2-F2 | Referenced schemas and validation sessions omit required function captures | fixed | every child below fixed or clean; `SessionObservations` (`slots/observations.rs`) captures a slot's Repo/Network/Agent group when the slot first evaluates, once per session, from the request's (or given context's) authority |
| R2-F2.1 | Referenced `$schema` during composition: five captured functions resolve | fixed | `lib/tests/l1/dynamic_schema_slot_captures.rs` `<fn>::referenced_schema_composition` for all five functions |
| R2-F2.2 | Imported named definition during composition | fixed | `<fn>::imported_type_composition` |
| R2-F2.3 | Selected trigger payload during composition | fixed | `<fn>::selected_trigger_composition`; guard `the_trigger_case_selects_its_payload` |
| R2-F2.4 | Validation/clean constructor (`SlotSession::for_context`) | fixed | `<fn>::validating_session`, `<fn>::validating_session_for_a_referenced_schema` (assert typed `default`) |
| R2-F2.5 | DMLS `run_job` worker | fixed | `dmls/tests/l1/schema_slots_editor.rs` `captured_functions::{ipv4,ipv6,has_agentic_cli,package,package_area}` through `run_job` |
| R2-F2.6 | Clean control: inline composition | clean | `<fn>::inline_composition` passes for all five |
| R2-F2.7 | Clean control: clock functions | clean | every `is_today::*` row and `captured_functions::is_today` pass |
| R2-F2.8 | Clean contract: command functions take captured PATH | clean | every `has_command::*`/`has_binary::*` row and `captured_functions::has_command` pass; probes read the context's PATH |
| R2-F2.9 | Reference-check resolution session | fixed | `<fn>::reference_check_session` (`for_context`) and extra instance `<fn>::reference_validation_of_a_referenced_schema` (`for_request` in reference-check mode) |
| R2-F2.10 | Nested data-contract slots | fixed | `<fn>::nested_contract_composition` (outer enum loads `[]` under `Row[]@./contract.yaml`, folder grant) |

### Progress

- starting the work on 'R2-F1: Cache reuse omits implicit and transitive request inputs' at 14:50:11
        - defect class: a shared-cache entry's eligibility and dependency checks were derived from the template's first-pass slot text rather than from what evaluation actually consumed
        - discovered: `env` is already part of the request-context identity, so `env` context checks repeat the key; captured calls and `ctx` groups are what matter
        - discovered: the data memo's contract cache was a further instance — a memo hit dropped what the contract's slots observed/consumed; fixed by storing and replaying them in `prepare_contract`
        - discovered: no `validate_schema` or import path resolves a nested template except `resolve_contract_slots`; later root-union rounds all reach `SlotHost::resolve`, so simplified-schema needed no change
        - completed: `call_function` (the one dispatch point) records every volatile call and every captured call (name, args, slot context, BLAKE3 digest of the answer); `serves`/pins re-call captured functions against the requesting command's observations and compare digests; any volatile call makes the entry command-local
        - completed: removed the static `is_volatile`/`context_checks` scans over `pending.slots()` (replaced, not supplemented)
        - decision (for human review): re-check captured calls by re-calling them rather than making such results command-local; keeps cross-command reuse for date/package schemas, exact per call
        - completed: `docs/topics/schemas/schema-slots.md` and the darkmatter `schema.md` skill updated
        - verification: `just test slot` 357 passed; `just test expression` 1227; `just test load_data` 332; `just test contract` 234; `just test schema` 2883; `just lint` clean
        - negative proof: emptying the recorded summary made 12 of 13 new tests fail (the same-day hit control passes by design)
- work completed for 'R2-F1: Cache reuse omits implicit and transitive request inputs' at 15:08:31
- starting the work on 'R2-F2: Referenced schemas and validation sessions omit required function captures' at 15:08:31
        - defect class: context requirements were discovered from the document's text, so captures needed by separately discovered schema slots were never supplied to their resolution session
        - completed: `SlotSession::for_request` grows observations from the request's repository and context authority (a frozen caller-supplied context grows nothing, matching document expressions); `for_context` captures from the given context's env and repository root and never reads the process directory; `SlotSession::new` captures nothing
        - completed: capture is decided per evaluated slot from its parsed calls, so later union rounds and contract slots are covered; pure/file-only schemas capture nothing
        - completed: R2-F1's cache re-check now reads the same per-session observations as evaluation
        - discovered: reference validation through a `for_request` session in reference-check mode (`md graph`, `md validate refs`) was a further instance; fixed and tested
        - discovered: compose discovers triggers only inside a repository, so the trigger fixture runs `git init` and a guard test proves the trigger is really applied
        - swept: `for_request` (pipeline default, command and reference-check sessions), `for_context` (`md schema validate`, `md clean`, DMLS `run_job`), `SlotSession::new` (only used without a launch context), `SlotEvaluator::new` (only via the session), pre-flight, YAML-document validation, trigger assembly, cache `serves`/pins — all covered
        - checked sibling: the Agent group scans the process `PATH` lazily; this is the existing capture provider shared with document `ctx.agent`/`has_agentic_cli`, and Ruling Q11's captured-PATH contract names only `has_command`/`has_binary`, so it is left as is (noted for human review)
        - decision (for human review): compose grows a copy of the request context rather than the shared request epoch, so a document's `ctx.tailnet` and a schema's `ipv4()` may come from two captures in one command
        - negative proof: with capture disabled, 40 library rows and 5 editor rows failed; all controls passed
        - verification: `just test dynamic_schema_slot_captures` 73 passed; `just test 'captured_functions::'` 7; `just test slot` 437; `just test capture` 293; `just test context_construction` 5; `just test schema` 2963; `just test context` 505; `just lint` clean
        - docs: `docs/topics/schemas/schema-slots.md`, `docs/lsp/architecture.md`, darkmatter `schema.md`/`dmls.md` skills; drifted `for_context`/`for_request`/`with_observations` docs corrected
- work completed for 'R2-F2: Referenced schemas and validation sessions omit required function captures' at 15:26:50
- starting final verification (`just test`, `just lint`, `just check-tier-coverage darkmatter`) at 15:26:50
        - `just test` (darkmatter area, L1): 15677 passed, 10 skipped; none at or above 5 s
        - `just lint` (darkmatter area): clean, including the zed-dmls wasm32-wasip2 check
        - `just check-tier-coverage darkmatter`: no stranded tests
        - `just cross-check darkmatter --os windows`: 11347 passed, 66 skipped; three tests at or above 5 s on Windows (`current_root_migration_guard::the_removed_nesting_appears_only_where_the_allowlist_expects`, two `schemas_constraints_block_matrix` cases) are outside this cycle's changes
- final verification completed at 15:35:38

### Completion Report

- **outcome:** complete
- **elapsed:** about 45 minutes (14:49:39 → about 15:35 local)
- **parent findings:** total 2 = fixed 2 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- **instance rows:** R2-F1.1–R2-F1.11 fixed, R2-F1.12–R2-F1.15 clean; R2-F2.1–R2-F2.5, R2-F2.9, R2-F2.10 fixed, R2-F2.6–R2-F2.8 clean
- **open findings:** none
- verified repairs:
        - R2-F1: a shared-cache entry's dependency summary is now recorded during evaluation (`slots/dependencies.rs`). Any volatile call at any depth makes the entry command-local. A captured call is re-checked by calling it again against the requesting command's observations and comparing a BLAKE3 digest of the answer. Nested contract slots, the contract memo, and later root-union rounds all contribute to the summary.
        - R2-F2: `SessionObservations` (`slots/observations.rs`) captures the Repo, Network, or Agent group the first time a slot that needs it evaluates, at most once per session. It captures from the request's authority (compose) or the given context's environment and repository (validate, clean, reference checks, DMLS worker), never the process directory.
- decisions recorded for human review (also in the review's `human_review_items`):
        - R2-F1: captured calls are re-checked by re-calling them, rather than making such results command-local
        - R2-F2: compose grows a copy of the request context instead of sharing the request epoch, so a document's `ctx.*` and a schema slot may observe separate captures in one command
        - R2-F2: the Agent group keeps its existing lazy scan of the process `PATH` (shared with document `ctx.agent`); Q11's captured-PATH contract names only `has_command`/`has_binary`
- changed files this cycle:
        - new: `darkmatter/lib/src/markdown/schemas/slots/dependencies.rs`, `darkmatter/lib/src/markdown/schemas/slots/observations.rs`, `darkmatter/lib/tests/l1/dynamic_schema_slot_captures.rs`
        - modified: `darkmatter/lib/src/markdown/schemas/slots/{mod.rs,session.rs,cache.rs,host.rs,evaluator.rs,session_tests.rs}`, `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs`, `darkmatter/lib/src/markdown/compose/expression/data/runtime.rs`, `darkmatter/lib/src/markdown/compose/context/{mod.rs,options.rs}`, `darkmatter/lib/src/markdown/compose/context/capture/{groups.rs,mod.rs,observations.rs}`, `darkmatter/lib/tests/l1/main.rs`, `darkmatter/dmls/tests/l1/schema_slots_editor.rs`
        - docs/skills: `darkmatter/docs/topics/schemas/schema-slots.md`, `darkmatter/docs/lsp/architecture.md`, `.claude/skills/darkmatter/schema.md`, `.claude/skills/darkmatter/dmls.md`
- final verification (all from the darkmatter area unless noted):
        - `just test`: 15677 passed (verifies R2-F1 and R2-F2 together with everything else)
        - `just lint`: clean
        - `just check-tier-coverage darkmatter` (repository root): no stranded tests
        - `just cross-check darkmatter --os windows`: pass (11347 passed)
        - simplified-schema was not changed this cycle, so its gates were not rerun
- no performance measurement was required; `deferred_perf_measurement` not set

## Implementation of Review Findings #3

> **started at:** 2026-11-08T15:42:47-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-dynamic-schema-types/review-3.md'
- this is iteration 3 of the review-to-implement cycle 
- the review's `implemented` flag was already `false`; it was left `false` before repairs began
- the review has no `human_in_the_loop` items and no blocked findings; it reports a recurrence (R3-F1 is a sibling of R1-F1.18), so the recurrence's sites are the minimum scope
- the input robustness matrix was re-checked by the reviewer (grant store loader clean); this cycle adds no loader

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R3-F1 | Nested contract cache hits lose the defining schema's read permissions | fixed | every child below fixed or clean; allowed reads (with each read's defining file) now live in `SlotDependencies.reads`, recorded by `RecordingGates` at every depth, stored/replayed by the contract memo, cache hits and pins, and checked by `serves` before any rehash |
| R3-F1.1 | Nested contract content read: warm and cold both require the contract's read grant | fixed | `lib/src/markdown/schemas/slots/session_tests.rs` `nested_grants::a_nested_contracts_content_read_needs_its_own_grant_warm_and_cold` (red before, green after) |
| R3-F1.2 | Nested contract existence probe (`file_exists`) outside the repository: warm and cold both require a grant | fixed | `lib/src/markdown/schemas/slots/session_tests.rs` `nested_grants::a_nested_contracts_existence_probe_needs_its_own_grant_warm_and_cold` |
| R3-F1.3 | Further nested contract resolution: permissions preserved at every level | fixed | `lib/src/markdown/schemas/slots/session_tests.rs` `nested_grants::a_deeper_nested_contracts_probe_needs_its_own_grant_warm_and_cold` (`Pattern@./inner.yaml`) |
| R3-F1.4 | Contract memo and its replay: defining-file permission records replayed with observations/dependencies | fixed | `lib/src/markdown/schemas/slots/session_tests.rs` `nested_grants::a_memoized_nested_contract_replays_its_reads_for_another_command`: asserts a memo hit (one evaluation) on the second template, then refusal warm and cold; `ContractSlotInputs.dependencies` now carries the reads |
| R3-F1.5 | Outer input-reading function's post-call check: retains nested requirement rather than replacing it | fixed | `authorize_slot_observations` still records under the outer file; the nested record is added alongside it, never replaced; shown by the warm refusals in .1–.4 |
| R3-F1.6 | Shared-cache permission acceptance (`serves`): checks every original defining-file/target requirement before rehash/serve | fixed | `SlotSession::serves` checks every `entry.dependencies.reads` record against its own defining file before inputs are rehashed; `CacheEntry.reads` removed (one inventory); shown by .1–.4 |
| R3-F1.7 | Clean control: ordinary top-level template resolution refuses warm and cold | clean | `lib/src/markdown/schemas/slots/session_tests.rs` `nested_grants::a_shared_template_read_directly_needs_its_grant_warm_and_cold` (control added; refuses warm and cold) |
| R3-F1.8 | Clean invariant: imported definitions and later root-union rounds use the host's `RecordingGates` | clean | `resolve_fresh` still passes one `RecordingGates` host to `pending.resolve` for imports and later rounds; existing `reuse::*` and `consumed::a_later_root_union_rounds_volatile_call…` pass |
| R3-F1.9 | Clean contract: within-command pins keep the once-per-run decision | clean | `pinned()` does not re-check grants within a command (Q17); it now replays `pin.dependencies` (with reads) to enclosing collections; `reuse::one_command_pins_its_snapshot`, `reuse::a_rewrite_during_the_run_re_resolves_under_the_grant_decided_earlier` pass |
| R3-F1.10 | Clean invariant: nested listings and remote reads need explicit grants at both origins | clean | `lib/src/markdown/schemas/slots/session_tests.rs` `nested_grants::a_nested_contracts_listing_needs_an_explicit_grant_warm_and_cold`; URLs: `InputCheck::rechecks()` is false for URL/listing observations, so such entries are `only_for` their session, and the slot remote runtime only allows explicitly granted hosts (`SlotGrants::fetch_policy`) |
| R3-F1.11 | Discovered: a cache hit or pin replayed inside an enclosing collection carries its reads | fixed | reads now live in the replayed `SlotDependencies`, so `dependencies::replay` carries them; no production path was found that serves a cached template inside another slot's collection, so this is structural (covered by the same code as .4) |

### Progress

- starting the work on 'R3-F1: Nested contract cache hits lose the defining schema's read permissions' at 15:43:10
        - defect class: a nested evaluation keeps the values it read but loses the defining-file provenance of each allowed read, so a cache entry reused by another command is re-authorized against the wrong (outer) defining file
        - subagent (rust-developer) log items:
                - 15:44:09 repair started; read review-3, the session, cache, dependencies and runtime code, and the existing `reuse::`/`consumed::` tests
                - 15:48:00 fix: `SlotDependencies.reads` (deduplicated, merged) and `dependencies::note_read`; `RecordingGates` now records each allowed read into the thread-local dependency frames; `resolve_contract_slots` uses the recording gates instead of plain `RunGates`; the separate `CacheEntry.reads` vector was removed (replaced, not supplemented); `serves` checks `dependencies.reads` against each read's own defining file before any input is rehashed
                - 15:49:00 added `session_tests.rs::nested_grants` (6 tests: R3-F1.1–.4, .7, .10), each with broad grants, a narrowed warm request, and a cold control on a fresh fixture; 6/6 green
                - 15:50:00 negative proof A (plain `RunGates` restored for nested contracts): .1–.4 red, controls green; proof B (`note_read` a no-op): 5 red, listing control green; both restored
                - 15:51:00 scoped gates: `just test slot` 443, `just test contract` 247, `just test load_data` 332, `just test dynamic_schema` 195, all passing; `just lint` clean
                - 15:52:00 docs: `docs/topics/schemas/schema-slots.md` reuse rules and `.claude/skills/darkmatter/schema.md` cache sentence updated
                - 15:52:40 sweep: no other production plain-`RunGates` site (only `approval_tests.rs`), one `cache::store` site, one contract-memo store, DMLS and triggers go through `SlotSession`; added R3-F1.11 (a cache hit or pin inside an enclosing collection now replays its reads, since reads live in the replayed dependencies)
        - orchestrator review: inspected the combined diff; checked that no slot-resolution path evaluates on another thread (the only `thread::spawn` sites are memo unit tests), so thread-local recording loses no read; a caller waiting on the same contract memo slot receives the reads through the stored `ContractSlotInputs` replay
        - decision (for human review): reads are recorded only in the dependency frames, rather than in an extra `Arc<Mutex>` recorder alongside them, so there is one inventory that cannot drift; same thread-locality constraint as observation and dependency capture
        - decision: tests use `SlotSession::resolve` (the path the existing `reuse::`/`consumed::` tests use), not `DarkmatterSchemas::effective_for`, so the memo case can count evaluations; cold controls use fresh fixtures, not a process-cache clear
- work completed for 'R3-F1: Nested contract cache hits lose the defining schema's read permissions' at 15:53:52
- starting final verification (`just test`, `just lint`, `just check-tier-coverage darkmatter`) at 15:53:52
        - `just test` (darkmatter area, L1): 15683 passed, 10 skipped; 7 tests ran above 5 s while the host load average was about 40–50 from other builds on the host (`ld`/`rustc` from another session); each was rerun alone through `just test <filter>` and passed in 1.5–3.2 s, so they are recorded as load-sensitive; none is in code this cycle changed (`expression_type_scope::every_guard_side_admits_…` ×2, `glob_implementation_guard::file_references_have_one_glob_implementation`, `compose_shell_template::every_suggested_rule_approves_the_command`, `claudine_editor_parity::the_shared_corpus_valid_{depth,generated_verb}_fixtures_…`, `content_policy_baseline::every_shared_declaration_publishes_…`)
        - `just lint` (darkmatter area): clean, including the zed-dmls wasm32-wasip2 check
        - `just check-tier-coverage darkmatter` (repository root): no stranded tests
        - `just cross-check darkmatter --os windows`: pass, 11353 passed, 66 skipped
- final verification completed at 16:09:11

### Completion Report

- **outcome:** complete
- **elapsed:** about 27 minutes (15:42:47 → 16:09 local)
- **parent findings:** total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- **instance rows:** R3-F1.1–R3-F1.6 and the discovered R3-F1.11 fixed; R3-F1.7–R3-F1.10 clean
- **open findings:** none
- **recurrence:** the R1-F1.18 permission-preservation class is closed at its inventory source: there is now one record of allowed reads (`SlotDependencies.reads`), filled by the one recording gate used by both top-level and nested resolution, and carried by every store/replay path (cache entry, pin, contract memo)
- verified repair:
        - R3-F1: every read a slot's gate allows is recorded with the schema file that defined that slot, at every nesting depth. The contract memo, cache hits, and pins replay those records to enclosing resolutions. `serves` refuses an entry unless the requesting command's grants cover every record, before any input is rehashed. Negative proofs: restoring plain `RunGates` for nested contracts turned .1–.4 red; disabling recording turned five tests red
- decisions recorded for human review (also in the review's `human_review_items`):
        - R3-F1: allowed reads are recorded only in the thread-local dependency frames, replacing the separate per-resolution recorder, so there is one inventory; no slot resolution path evaluates on another thread
- changed files this cycle:
        - `darkmatter/lib/src/markdown/schemas/slots/{dependencies.rs,session.rs,cache.rs,session_tests.rs}`
        - `darkmatter/lib/src/markdown/compose/expression/data/runtime.rs` (doc comment only)
        - docs/skills: `darkmatter/docs/topics/schemas/schema-slots.md`, `.claude/skills/darkmatter/schema.md`
- final verification (verifies R3-F1 and all earlier repairs together):
        - `just test nested_grants`: 6 passed (R3-F1.1–.4, .7, .10)
        - `just test slot` 443, `just test contract` 247, `just test load_data` 332, `just test dynamic_schema` 195: all passed
        - `just test` (darkmatter): 15683 passed; `just lint`: clean; `just check-tier-coverage darkmatter`: no stranded tests; Windows cross-check: pass
        - simplified-schema was not changed this cycle, so its gates were not rerun
- no performance measurement was required; `deferred_perf_measurement` not set
