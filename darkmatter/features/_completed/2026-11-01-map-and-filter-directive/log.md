---
feature: 2026-11-01-map-and-filter-directive
implementation_1: "2026-10-28T05:05:02-07:00"
implementation_2: "2026-10-28T15:09:50-07:00"
---

# Map and Filter Directive — Review/Implement Log

## Implementation of Review Findings #1

> **started at:** 2026-10-28T05:05:02-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-map-and-filter/darkmatter/features/2026-11-01-map-and-filter-directive/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review `implemented` flag confirmed `false` before repairs began

### Finding Ledger

| ID       | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| -------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1    | (high) Selection guards lose refinements across destructured and object bindings | fixed | all 7 children resolved (1.1–1.3, 1.7 fixed; 1.4–1.6 clean); destructured fields alias item id + member path |
| R1-F1.1  | Filter body after filter/while/until: destructured body, guard reads `p.members` → nested `::map members` renders both members | fixed | `scope_schedule::destructured_filter_body_with_an_object_guard::{filter,while,until}_slot` (failed at HEAD, pass now); analysis + dmls `object_guard` cases |
| R1-F1.2  | Filter body after filter/while/until: object body, guard destructures `members` → nested `::map p.members` renders both | fixed | `scope_schedule::object_filter_body_with_a_destructured_guard::*`; dmls `destructured_guard` case |
| R1-F1.3  | Map callback/result after filter/while/until: `{members} => {members}`, guard reads `p.members` → renders both | fixed | `scope_schedule::destructured_map_result_with_an_object_guard::*`; `a_map_result_field_named_like_a_guarded_input_field_is_a_new_binding` keeps result identity distinct |
| R1-F1.4  | Clean control: object params `p`/`q`, guard reads `q.members` → renders | clean | `object_filter_body_with_a_differently_named_object_guard::*` pass |
| R1-F1.5  | Clean control: both callbacks destructure `members` → renders | clean | `destructured_filter_body_with_a_destructured_guard::*`, `destructured_map_result_with_a_destructured_guard::*` pass |
| R1-F1.6  | Clean control: file field checked via object param then read destructured with `frontmatter(path,'title')` → succeeds | clean | `a_file_field_checked_through_an_object_parameter_is_read_through_a_destructured_one` with uncertain-call denied, all 3 slots |
| R1-F1.7  | Editor and passive call-check consumers share `BodyScopes`/`ItemFrame` → one canonical original-field identity | fixed | `collection_analysis::a_guard_fact_crosses_destructured_and_object_spellings_in_the_shared_scopes`; dmls `collection_editor::guard_facts_cross_destructured_and_object_spellings_in_editor_diagnostics` (incl. shadow negative) |
| R1-F2    | (high) Runtime diagnostic projections drop authored locations or iteration identity | fixed | all 16 children resolved; `MarkdownError::InIteration(IterationFailure)` + located `CollectionError::Budget{site}` + `Diagnostic.{span,iteration_path}` |
| R1-F2.1  | Item-visit budget reservation → authored collection location + original item path | fixed | `provenance.rs` generated row test (map+filter); budget boundary tests tightened |
| R1-F2.2  | Authored-body copy budget → authored location + path | fixed | `provenance.rs` row test (map+filter) |
| R1-F2.3  | Interpolation insertion budget → authored interpolation span + path | fixed | `provenance.rs` row test (map+filter) |
| R1-F2.4  | Ordinary shell output insertion budget → authored shell site + path | fixed | `provenance.rs` row test (map+filter) |
| R1-F2.5  | Shell-block output insertion budget → authored shell-block site + path | fixed | `provenance.rs` row test (map+filter); located at shell-block opener |
| R1-F2.6  | Child output insertion budget → authored transclusion site + path | fixed | `provenance.rs` row test (map+filter); `a_child_failure_under_a_repeated_file_joins_the_parent_and_child_paths` |
| R1-F2.7  | Cached child budget replay → replaying child site + applicable iteration identity | fixed | `provenance.rs` row test; cached-replay test tightened to assert source + path |
| R1-F2.8  | Repeated-body interpolation runtime failure → location + `data[1]` | fixed | `provenance.rs` row test (map+filter) |
| R1-F2.9  | Repeated page-block gate failure → authored file line + `data[1]` | fixed | `provenance.rs` row test; orchestrator CLI repro: source line 7 col 15, span, `data[1]` |
| R1-F2.10 | Nested collection gate failure → authored gate location + enclosing `data[1]` | fixed | `provenance.rs` row test (map+filter) |
| R1-F2.11 | Repeated transclusion target expression failure → target span + `data[1]` | fixed | `provenance.rs` row test (map+filter) |
| R1-F2.12 | Repeated transclusion condition failure → condition location + `data[1]` | fixed | `provenance.rs` row test (map+filter) |
| R1-F2.13 | Shell argument validation (null item) → authored site + `data[0]` | fixed | `provenance.rs` row test (map+filter) |
| R1-F2.14 | Shell-block argument validation (null item) → authored command site + `data[0]` | fixed | `provenance.rs` row test (map+filter) |
| R1-F2.15 | Clean control: callback failure CLI reports location + `data[1]` | clean | `provenance.rs` row test still green, now also asserts span; `nested_failures_report_every_enclosing_position` green |
| R1-F2.16 | Library `report_diagnostics::error_diagnostics` projection carries typed span/path for every host | fixed | `the_shared_record_carries_typed_frames_and_the_authored_span`; CLI `report_locates_*` parity tests; CLI path repair removed (library owns provenance) |
| R1-F3    | (high) Static collection rejection discards earlier recovery warnings | fixed | all 7 children resolved; `analyze_collection_blocks_report` returns recovered prefix with rejection, fed to precheck `Advisories::recovered` |
| R1-F3.1  | Recovered map source before static source rejection → warning + error | fixed | `pipeline::recovery_prefix` map row (failed before fix); CLI `a_recovered_source_warning_survives_a_later_static_rejection_in_the_report` |
| R1-F3.2  | Recovered filter source before static source rejection → warning + error | fixed | `pipeline::recovery_prefix` filter row (failed before fix); CLI test covers both families |
| R1-F3.3  | Clean control: map recovery before runtime pattern failure → both retained | clean | `recovery_prefix` runtime-pattern map row passed before and after |
| R1-F3.4  | Clean control: filter recovery before runtime pattern failure → both retained | clean | `recovery_prefix` runtime-pattern filter row passed before and after |
| R1-F3.5  | Clean control: recovery without later failure → one warning, success | clean | `recovery_prefix` recovery-alone rows: exactly one warning, success |
| R1-F3.6  | Pattern, sort-key, protected-name static rejections (map + filter) → warning + error | fixed | six `recovery_prefix` rows (pattern/sort/protected × map/filter) failed before, pass now |
| R1-F3.7  | Call-precheck authored-order prefix filtering reused for collection recoveries | fixed | `a_recovered_source_after_a_static_rejection_is_not_reported` (authored-order prefix); `an_unrecovered_source_is_the_failure_before_a_later_rejection` |
| R1-F4    | (low) Expression reference retains obsolete lookup and file-guard behavior | fixed | all 3 children fixed in `docs/topics/darkmatter-expressions.md`; siblings in `docs/inline/interpolation.md` and skill `expressions.md`/`compose.md` fixed |
| R1-F4.1  | "unprefixed name is a document property" lookup text → nearest-local lookup | fixed | bare-name lookup prose, root-classification diagram, new "The local namespace" section; `just test docs_corpus` 6/6, `current_root` 5/5 |
| R1-F4.2  | `file_exists` guard description and predicate-registration example → file suitability on true side | fixed | guard table and registration-table example now state `file` suitability on true side, nothing on false; later-read caveat kept |
| R1-F4.3  | Reserved-root table includes `local` | fixed | "Five reserved prefixes" table with `local.*` row; `narrative_doc_function_table` 1/1 |

| R1-F2.X1 | (sweep) claudine `compose_failed_code` must classify wrapped failures by the inner error | fixed | `without_iteration()` used; claudine `just test render` 482/482 (subagent) |
| R1-F2.X2 | (sweep) claudine `map_compose_error` routes a top-level `ShellExpansion` to `ShellExpansionFailed`; a wrapped shell failure inside a repeated copy now rides `ComposeFailed` | fixed (decision) | code stays `composition.shell_expansion` through `compose_failed_code`; wrapper keeps location + iteration path that `ShellExpansionFailed` cannot carry; recorded in `human_review_items` |

### Progress

- starting the work on 'R1-F1 selection guard identity' at 05:05:40
        - defect class: two spellings of one original item's field (a destructured field and an object parameter's member) were given different type-scope identities, so a guard fact recorded under one spelling was invisible under the other
        - sibling sites swept: `ItemFrame::enter`/`new` (iterable.rs, the defect; fixed), `LexicalTypes` subject/shape lookup (lexical.rs; fixed to carry a member path), `Subject::local` (projection.rs; takes the local's prefix path), `TypeScope::projected` (mod.rs; walks only the uncovered path), `facts.rs` (identity-agnostic, clean), `collection_blocks/scopes.rs` (only frame construction changed; `unknown_binding` fallback keeps fresh untyped locals, correct), `analysis.rs`, `call_precheck.rs`, `directive_targets.rs` (excludes locals), shell blocks / shell expansion / preflight (all read through `BodyScopes::scope_at`, clean), dmls `providers/collections.rs` (library scope, plus an untyped fallback for headers that do not validate yet; clean), claudine (no use of the changed API)
        - change: a destructured field is now bound with `TypedLocal::aliasing_field(name, item_id, field, shape)`, i.e. the subject `item.field`; `LexicalTypes::shape_of` became `shape_below(id, path)` (longest covering prefix), and `ItemFrame::new` lost its `patterns` argument and per-field slot ids
        - tests added: `scope::iterable` unit tests `a_destructured_field_and_an_object_member_are_one_subject_in_both_directions`, `a_map_result_field_gains_input_facts_only_through_its_mapping_expression`, `a_shadowing_destructured_field_of_another_item_does_not_inherit_outer_facts`; `collection_blocks::scope_schedule` macro matrix (6 rows x filter/while/until slots = 18 tests) plus `a_map_result_field_named_like_a_guarded_input_field_is_a_new_binding`, `an_inner_destructured_field_does_not_inherit_the_outer_items_guard_facts`, `a_file_field_checked_through_an_object_parameter_is_read_through_a_destructured_one`; `collection_analysis::a_guard_fact_crosses_destructured_and_object_spellings_in_the_shared_scopes`; dmls `collection_editor::guard_facts_cross_destructured_and_object_spellings_in_editor_diagnostics`
        - baseline proof: with the five source files reverted to HEAD, the crossing rows (destructured body + object guard, destructured map + object guard, all slots) and both negative/positive pair tests failed, while the clean controls passed
        - commands: `just test collection` 418/418 pass; `just test scope` 364/364 pass; `just test type_scope` 46/46, `just test iterable` 32/32, `just test lexical` 27/27, `just test directive_target` 12/12, `just test precheck` 20/20, `just test shell_template` 104/104, `just test docs_corpus` 6/6; `just lint` clean
        - decision: alias destructured fields to the item id + member path (the review's direction) instead of mapping `p.members` onto a destructured field's slot id, because the member-path form also covers deeper reads (`members.x` == `p.members.x`) and index spellings (`p['members']`) through the existing `Subject` path without a second lookup table
        - decision: an identity map `"p => p"` does not carry a member fact (`p.members`) into the result, because expression typing of the whole binding does not fold member facts; the spec only asks for the mapping expression's own type to flow, so this was left unchanged and no test asserts it either way
        - docs: `docs/topics/expression-engine.md`, `docs/inline/filter.md`, `docs/inline/map.md`, and the darkmatter skill's `compose.md` now state that the two spellings share facts and that a map result is a new binding
- work completed for 'R1-F1 selection guard identity' at 05:20:30
- starting the work on 'R1-F2 runtime diagnostic provenance' at 05:20:35
        - defect class: a failure raised while a repeated body was being processed lost its authored site or its original item path when it was converted into a budget error, an ordinary body-stage error, or a report record, so hosts had to rebuild provenance (or could not)
        - sibling sites swept: structural engine visit/copy reservations and callback budget (fixed, located at the collection opener / copied text / callback), engine page-block and nested collection `when=` gates (fixed, wrapped), interpolation + directive-target passes (fixed via `anchor_authored_failure`), interpolation byte charge (fixed), shell-expansion template read / prepare / execute / output charge (fixed), shell-block stage failure and output charge (fixed, located at the block the stage entered last), transclusion prepare `when=` / scoped resolution / target resolution / URL refusal (fixed), fatal child resolution failure and child output charge (fixed, located at the including `::file`), cached child replay (fixed: reserves with the including site's item path), `report_diagnostics::error_diagnostics` (fixed, one projection), CLI `CollectionFailure` path repair (removed), fatality checks `transclusion_failure_is_fatal`, `nested::is_structural`, `call_diagnostic` condition lookup, `with_on_disk_source` (fixed to read through the wrapper), `as_block_error` registry (arm added), claudine `compose_failed_code` (reads through the wrapper so codes are unchanged); clean: preflight discovery (binds null instead of failing), call precheck / static collection rejection (static, reported once, no path), frontmatter transclusion / toc / file-links (no per-copy evaluation), DMLS (does not consume compose failures), cache single-flight typed replay (stores only `Collection` errors, unchanged)
        - changes: `CollectionError::Budget { site: Option<CollectionSite>, error }` with `CollectionError::budget` / `is_unlocated_budget` and `with_source` locating it; new `IterationFailure { location, iteration_path, error }` and `MarkdownError::InIteration` (message, code, and recoverability are the wrapped error's; `MarkdownError::without_iteration` for classification); `site_scope::scope_failure` / `BodyProvenance::scope_failure` as the one place a stage attaches provenance; `SiteLookup::entered`; `Diagnostic` gains `span: Option<Range<usize>>` and `iteration_path: IterationPath` (both always serialized); `IterationPath`/`IterationFrame`/`IterationPosition` derive `Serialize`, `IterationPath::joined`; `report_diagnostics` joins nested paths outermost first, falls back to the innermost proven site, fills `span`, and appends `at <path>`
        - tests added: `collection_blocks::provenance` macro matrix of 15 rows (F2.1-F2.15) x `map`/`filter` = 30 tests checking typed error and shared record (code, file, line, span, path, message suffix), plus `every_case_has_a_generated_test_and_every_test_a_case`, `the_shared_record_carries_typed_frames_and_the_authored_span` (F2.16), `a_child_failure_under_a_repeated_file_joins_the_parent_and_child_paths`, `a_static_failure_is_reported_once_with_no_iteration`; CLI `collection_blocks::report_locates_{a_visit_exhaustion_in_a_map,a_visit_exhaustion_in_a_filter,a_repeated_interpolation_failure_in_a_map,a_repeated_gate_failure_in_a_filter,a_repeated_child_insertion_in_a_map}` (report-json + terminal path)
        - tests tightened: `collection_blocks::budget` (`assert_boundary` and the excessive-append, distinct-children, and cached-replay cases now assert a located source and a non-empty path), `collection_effects::a_failure_inside_a_repeated_body_keeps_its_authored_location` (+ `data[1]`), `collection_discovery::local_templates::a_null_or_container_local_fails_the_shell_input_check_and_runs_nothing` (+ `data[0]`, line 7), `collection_discovery::scoped_children::a_target_discovered_for_no_site_is_not_approved_late` (+ `names[1]`); `compose_report_json` key list and exact record updated for the two new fields; `nested_failures_report_every_enclosing_position` unchanged and green
        - baseline proof: with `scope_failure` neutered (no locating, no wrapping) 12 of the first 35 provenance tests failed before fail-fast (later-stage rows and the child join), the engine rows kept passing because they are located in the engine; restored afterwards
        - commands: `just test provenance::` 63/63 pass; `just test collection` 457/457; `just test report` 382/382; `just test budget` 21/21; `just test transclu` 340/340; `just test interpolat` 464/464; `just test shell` 1109/1109; `just test page_block` 97/97; `just test nested` 391/391; `just test cache` 255/255; `just test diagnostic` 228/228; `just test iteration` 19/19; `just test lifecycle` 468/468; `just test guard` 259/259; `just test layout` 229/229; `just test registry` 222/222; `just test as_block_error` 4/4 (was 0/1 before the registry arm); claudine `just test render` 482/482; `cargo check -p claudine -p claudine-cli --tests` clean; `just lint` clean
        - decision: one wrapper variant (`MarkdownError::InIteration`) applied only at a stage's error exit and only for a site inside a copy, instead of adding an iteration field to each of the Interpolation / PageBlock / Transclusion / ShellExpansion / ShellBlock variants; recovery checks that matched variants now read `without_iteration()`, so no recoverable error became fatal or the reverse and no code changed
        - decision: budget failures keep their own variant (`Collection(Budget)`) and get a site at the charge caller rather than being wrapped, so their path is not counted twice and the cached-failure store still sees them
        - decision: a page-block gate's message keeps its body-relative "line N" text (the ordinary page-block engine words it the same way); the record's `source`/`span` carry the authored file line
        - decision: a shell-block failure is located at the block opener the stage entered (`SiteLookup::entered`), not the individual command line, to avoid threading command spans out of the shell-block stage
        - decision: `span` is filled only for located compose failures (call-check and warning records keep `None`) so DMLS/CLI record parity for call findings is unchanged
        - docs: `docs/cli/report-json.md` (eight fields, new section on failures inside bodies with a real example, updated `dm.collection.*`/`dm.iteration.*` rows), `docs/inline/map.md`, `docs/inline/filter.md`, darkmatter skill `compose.md`; doc comments of `report_diagnostics`, `CollectionError::Budget`/`with_source`, `IterationFailure`, `MarkdownError::InIteration`/`without_iteration`/`with_on_disk_source`, `scope_failure`, interpolation `run_stage`, CLI `collection_failure`
        - orchestrator check: CLI repro of F2.9 (`::block when="frontmatter(i,'title')"` in `::map data`) now reports `source` line 7 col 15, `span`, and `iteration_path: [{collection: data, position: 1}]`; the message's "at line 2" is the document-body-relative line, the same convention an ordinary page block outside any collection uses (verified with a plain document: "at line 3" body-relative, unchanged from base), so it is consistent rather than drifted
        - claudine was touched (`claudine/lib/src/composition/error/render/mod.rs`) to look through the new `MarkdownError::InIteration` wrapper; claudine is added to the final verification scope
- work completed for 'R1-F2 runtime diagnostic provenance' at 05:58:10
- starting the work on 'R1-F3 recovery warnings before static rejection' at 05:58:15
        - defect class: a batch validator that returns an error drops the diagnostic prefix it collected before the failure; `analyze_collection_blocks` built `recovered` and then returned only the first `CollectionError`, so `call_precheck::check_body` never saw a waived source when any later region had a static defect
        - sibling sites swept: `collection_blocks/analysis.rs` (the instance; fixed); `call_precheck.rs::check_body` (consumer; now feeds the prefix into `Advisories::recovered`, whose existing authored-order `retain(*at < order)` in `check_document` decides what survives); its other surfaces (block/transclusion conditions, interpolation, collection expression sites) already record advisories before choosing a rejection (no change); `collection_blocks/scopes.rs` and `engine.rs` collect no warnings before an `Err` (runtime preservation was already correct, F3.3/F3.4 controls green before the fix); shell/preflight discovery runs the same `run_call_precheck` through the pipeline (fixed by the same change); DMLS `overlay/collections.rs` always passes both recovery booleans `false`, so it can never have a recovery warning to keep (no change); claudine `collection_host_compat` uses the unchanged `analyze_collection_blocks` signature (no change)
        - changes: new public `analyze_collection_blocks_report` -> `CollectionAnalysisReport { analysis, rejection }` (re-exported from `collection_blocks`); its `recovered`/`diagnostics` hold only waived sources authored before the rejected span; `analyze_collection_blocks` is now a thin wrapper with an unchanged signature and contract (doc points at the report form); `check_body` consumes the report and keeps the lenient `BodyScopes::scan` fallback on rejection as before; module doc of `call_precheck.rs` states the kept-with-rejection rule
        - docs: `docs/inline/map.md` and `docs/inline/filter.md` (recovery warning reported once; kept ahead of a later static defect; a recovery after the defect is not reported), `docs/cli/report-json.md` (failure ordering bullet), darkmatter skill `compose.md` (discovery boundary) and `dmls.md` (drift fixed: it claimed DMLS shows untyped sources as warnings when a recovery is configured; the overlay hard-codes both recoveries off, so the code was taken as correct and the skill corrected)
        - tests: `lib/tests/l1/collection_blocks/pipeline.rs::recovery_prefix` (14 tests, 12 generated by `recovery_rows!`, one per F3.1-F3.6 row and family, through `compose_with_warnings` + `report_diagnostics::{warning_diagnostic, error_diagnostics}`, temp-dir anchored; plus a recovery-after-rejection prefix test and an unrecovered-source-is-the-failure test); `lib/tests/l1/collection_analysis.rs::the_report_keeps_recovered_sources_before_the_rejection`; `cli/tests/l1/compose_collection_controls.rs::a_recovered_source_warning_survives_a_later_static_rejection_in_the_report` (`--allow untyped-*-variables --output report-json`, both families, exact `[(warning), (invalid_source error)]` rows)
        - old-behavior proof: before the repair `just test recovery_prefix` ran 14: 6 passed (controls), 8 failed (every static row, `left: []`); the CLI test failed with only `[("dm.collection.invalid_source", "error")]` when the drop was temporarily reinstated, then the source was restored
        - commands (from `darkmatter/`): `just test recovery_prefix` 14/14 passed; `just test collection` 473 passed; `just test precheck` 20 passed; `just test recover` 122 passed; `just test report` 385 passed; `just lint` exit 0, no warnings
        - decisions: added a report-form function rather than changing `analyze_collection_blocks`'s `Result` (keeps DMLS, claudine, and existing analysis tests untouched, and keeps one analysis pass, no re-evaluation); the prefix is trimmed in the library too (not only in the precheck) so the public report is self-consistent; with a rejection, a recovery warning is reported ahead of it rather than judged by the blocking policy, which is the existing precheck rule for every earlier advisory (`Blocked` is chosen only when nothing is rejected); `--deny untyped-*-variables` turns the recovery off, so the untyped source becomes the first failure itself (covered by `an_unrecovered_source_is_the_failure_before_a_later_rejection`)
- work completed for 'R1-F3 recovery warnings before static rejection' at 06:08:20
- starting the work on 'R1-F4 expression reference drift' at 06:08:25
        - defect class: topic documentation stating lookup and guard contracts the delivered code no longer has (bare names as document properties only, four reserved roots, `file_exists` proving only a present string); verified against `catalog/roots.rs` (roots `fm ctx env current local`), `binding.rs` (`ScopedLookup`: reserved/global, nearest local, then document; `local.*` absent outside a scope; `check_local_names`), and `functions/paths.rs` (`file_exists` registered `TrueImplies file`, tests in `functions/predicates.rs`); the code was right, the docs were wrong
        - edits in `docs/topics/darkmatter-expressions.md`: absent-property rule (L101), value sources and bare-name lookup with `local` reserved and `fm.local` (L123-133), root-classification prose and mermaid with `local` and a local-binding step (L148-162), guard table merges `file_exists`/`is_file_type` to `file` suitability (L340) plus a note on why the false side proves nothing (L350), registration-table example (L384), freshly evaluated read vs separate later read kept and made concrete with `file_exists(frontmatter(...))` (L366-370), "Five reserved prefixes" with a `local.*` row (L509, L520), bare-name paragraph inside local scopes (L590), new `### The local namespace` section (L657), evaluate shortcut has no locals (L1656), `ResolvedBinding` includes locals (L1703), host-binding order and local shadowing (L1734), `associate` rejects `local` (L1775), "no host globals" line (L1838); generated function table untouched (its `file_exists` row states no narrowing, so no source change)
        - siblings swept (`darkmatter/docs/`, `darkmatter/README.md`, `.claude/skills/darkmatter/`): fixed `docs/inline/interpolation.md` L141 and the declares table (L154-155, `local` and local bindings are silent); skill `expressions.md` L63 (`file_exists` true-implies `file`) and L270 (resolution order includes nearest local); skill `compose.md` L395 (bare name is a document property only outside a local scope); checked clean: `docs/inline/map.md`, `filter.md`, `iterables.md`, `topics/expression-engine.md`, `inline/preflight-checks.md` L531, `inline/fm-interpolation.md` L158, `schemas/parsing/grammar.md` L148, `lsp/features.md`, README, skill `dmls.md`
        - verification (from `darkmatter/`): `just test docs_corpus` 6 passed; `just test expression_doc` matched 0 tests, so ran the generated-table guard `just test narrative_doc_function_table` 1 passed and `just test query_vocabulary_link` 1 passed; `just test current_root` 5 passed (doc phrase contracts); `just test interpolation` 431 passed; `just test docs` 7 passed; `just test _doc` 427 passed; `just lint` exit 0
- work completed for 'R1-F4 expression reference drift' at 06:15:00
- starting phase verification (full `just test`, `just lint`, `just check-tier-coverage` in darkmatter; `just test` and `just lint` in claudine) at 06:15:05
        - darkmatter `just test`: 13523 run, 13523 passed, 10 skipped (EXIT 0)
        - darkmatter `just lint`: EXIT 0; `just check-tier-coverage darkmatter`: 0 stranded
        - claudine `just test`: 11713 run, 11713 passed, 15 skipped (EXIT 0); claudine `just lint`: EXIT 0
        - cross-OS check skipped: the repairs change type-scope identity, error wrapping, and report fields, with no path, process, or platform-specific code, so they carry no notable cross-OS risk; CI's normal per-OS evidence covers them
- phase verification completed at 06:20:05

### Completion Report

- **outcome:** complete
- **elapsed:** about 1 h 15 min (05:05:02 → 06:20:10)
- **parent findings:** total 4 = fixed 4 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- **instance rows:** 33 review children + 2 sweep rows; 26 fixed, 9 clean, 0 open
- verified repairs:
        - R1-F1: a destructured field now uses the same `Subject` (item id + member path) as an object-member read, so guard facts reach both spellings in composition, passive analysis, and DMLS; map results and shadowing bindings keep distinct identities
        - R1-F2: failures inside repeated copies carry authored location, span, and original iteration path through `MarkdownError::InIteration` and located `CollectionError::Budget`; `report_diagnostics` projects `span` and `iteration_path` for every host, and the CLI's own path repair was removed; claudine classifies wrapped failures by their inner error
        - R1-F3: `analyze_collection_blocks_report` returns the recovered-source prefix with the rejection, and the call precheck's existing authored-order filtering keeps earlier recovery warnings in the failure report
        - R1-F4: the expression reference, interpolation doc, and darkmatter skill now describe nearest-local bare lookup, the reserved `local` root, and `file_exists` true-side `file` suitability
- open findings: none
- decisions recorded in the review's `human_review_items` (six "Repair cycle 1" entries)
- changed files (this cycle):
        - library: `lib/src/markdown/compose/expression/scope/{iterable,lexical,mod,projection}.rs`, `collection_blocks/{analysis,engine,error,mod,scopes}.rs`, `call_precheck.rs`, `body_origin.rs`, `site_scope.rs`, `report_diagnostics.rs`, `context/report.rs`, `inline/{collection_blocks,interpolation,shell_expansion}.rs`, `interpolation/rewrite.rs`, `nested.rs`, `pipeline/phases.rs`, `transclusion/engine.rs`, `expression/{iteration,diagnostics}.rs`, `markdown/{types.rs,errors/mod.rs}`
        - CLI: `cli/src/collection_failure.rs`, `cli/src/commands/compose/report.rs`
        - claudine: `claudine/lib/src/composition/error/render/mod.rs`
        - tests: `lib/tests/l1/collection_blocks/{provenance.rs (new),pipeline.rs,scope_schedule.rs,budget.rs}`, `lib/tests/l1/{collection_analysis,collection_blocks,collection_discovery,collection_effects,report_diagnostics,shared_diagnostics}.rs`, `cli/tests/l1/{collection_blocks,compose_collection_controls,compose_report_json}.rs`, `dmls/tests/l1/{collection_editor,expression_call_diagnostics}.rs`
        - docs/skills: `docs/topics/{darkmatter-expressions,expression-engine}.md`, `docs/inline/{map,filter,interpolation}.md`, `docs/cli/report-json.md`, `.claude/skills/darkmatter/{compose,dmls,expressions}.md`
- final verification (all findings): darkmatter `just test` 13523/13523, `just lint` clean, tier coverage 0 stranded; claudine `just test` 11713/11713, `just lint` clean

## Implementation of Review Findings #2

> **started at:** 2026-10-28T15:09:50-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-map-and-filter/darkmatter/features/2026-11-01-map-and-filter-directive/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- review `implemented` flag confirmed `false` before repairs began
- the review has one parent finding (R1-F2, recurrence of the unchanged R1-F2.14 instance), no blockers, and no `human_in_the_loop` items

### Finding Ledger

| ID         | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ---------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F2      | (high) Runtime diagnostic projections still locate shell-block command failures at the opener | fixed | All eight child/sweep rows reconciled by orchestrator inspection and public API/CLI assertions; full Darkmatter L1, lint, and tier coverage pass. |
| R1-F2.14   | Shell-block argument validation, null item in the **first** command → structured source/span name command line 8, path `data[0]` (map and filter) | fixed | provenance::{map,filter}::shell_block_argument_validation: line 8, column 1, exact command-start byte, bounded authored span, data[0]. |
| R1-F2.14a  | Shell-block argument validation, null item in a **later** command after `echo first` → command line 9, path `data[0]` (map and filter) | fixed | provenance::{map,filter}::shell_block_later_command_validation: line 9 and exact command site after echo first. |
| R1-F2.14b  | CLI wiring: `md compose --output report-json` reports the command line, not the opener | fixed | CLI report_locates_a_later_shell_block_command_in_each_kind: line 9, column 1, exact byte start, span, file, data[0], nothing executed. |
| R1-F2.13   | Clean control: ordinary `::shell` argument validation keeps the failing directive's line and item | clean | provenance::{map,filter}::{shell_argument_validation,shell_later_argument_validation}: directive lines 7/8, exact command-start byte and data[0]. |
| R1-F2.5    | Clean control: whole shell-block output charge keeps the block opener | clean | provenance::{map,filter}::shell_block_output_insertion: combined-output exhaustion remains at block opener line 7 with data[0]; charging uses edit.range independently. |
| R1-F2.16   | Projection: library record/report-json carry the stage-supplied command location without CLI reconstruction | clean | Shared error_diagnostics record assertions and report_locates_ CLI controls agree; library/CLI projection unchanged, no message reconstruction. |
| R1-F2.X3 | Sweep: first/later execution failures retain their own command site after all commands were prepared | fixed | provenance::{map,filter}::shell_block_{earlier,later}_command_execution: missing executable on lines 8/9 retains exact authored command site after preparation. |
| R1-F2.X4 | Sweep: earlier multiline argument, CRLF/multibyte text, and quoted containers retain later command location | fixed | provenance::{map,filter}::shell_block_{after_multiline_argument,crlf_command,quoted_command}_validation: exact authored start and columns (1 or 3); multiline cases failed before repair and pass now. |

### Progress

- starting the work on 'R1-F2 — shell-block command failures located at the opener' at 06:26:17
        - defect class: a failure raised by one unit of work inside an enclosing region is located at the region's site rather than the unit's own authored site
        - root cause confirmed by the orchestrator: `shell_blocks/mod.rs` calls `SiteHook::enter(pair.span.start)` once per block, and `pipeline/phases.rs::run_shell_blocks_operation` locates any stage failure at `SiteLookup::entered()`, so every command failure (prepare or execute) reports the opener
        - delegated to one foreground `rust-developer` subagent, which owns its log items for this finding


- resumed the interrupted iteration ledger for this run; all existing child IDs are retained and will be reconciled before completion

- continued the repair with `darkmatter`, `rust`, and `rust-testing` skills; inspected the interrupted command-offset handoff and preserved its changes
        - sibling sweep: template construction (`block_site`), all-command preparation (`prepare_site`), sequential execution (`execute_prepared_directive`), authored-template fallback, ordinary shell stages, block parse failures, output edit charging, `BodyOrigin`/`scope_failure`, library `error_diagnostics`, and CLI report-json
        - command construction, preparation, and execution enter the current command offset; block parse failures keep the opener, and combined-output charging independently uses the replacement edit's block range
        - reproduced a further instance: multiline interpolation in an earlier argument shifts a later command; the interrupted fallback inferred its current offset from an authored line count, yielding an unlocated source. Both map/filter regression cases failed before the repair
        - repaired that instance by matching current line starts to authored command starts through the existing `AuthoredBody` projection; CRLF, multibyte preceding text, and blockquote-prefix controls exercise the same mapping
        - strengthened library and CLI controls to assert command-start byte offsets, columns, bounded authored spans, original iteration paths, and no CLI execution on argument validation failure
- focused verification completed:
        - `just test provenance::`: 77 passed, including both directive families and the row-list coverage guard
        - `just test report_locates_`: 6 passed, including later-command report-json wiring for both families
        - `just test shell_blocks::`: 102 passed, including prepare/execute failure controls and existing parser/output behavior
        - `just test shell_block_after_multiline_argument_validation`: first valid regression run failed both cases; after projection repair, both passed
        - one intermediate test edit had a Rust string-quoting compilation error; corrected before regression reproduction and all final focused checks passed
        - declared targets verified: library L1 main declares collection_blocks, which declares provenance; CLI L1 main declares collection_blocks; these names contain no tier markers
        - docs/comments synchronized in shell-blocks topic, report-json reference, darkmatter compose skill, and BlockTemplates docs; no new public API, configuration loader, dependency, or OS-specific behavior
- work completed for 'R1-F2 — shell-block command failures located at the opener' at 15:20:32

- orchestrator reconciliation: inspected command-site entry during template construction, preparation, and execution; current-to-authored command-start projection; `BodyOrigin` fallback; separate block-output charging; shared library record projection; CLI byte/column/path assertions; declared L1 targets and live tier names
        - all eight child/sweep rows have independently specified expected locations and outcomes; R1-F2 remains the only parent finding
        - existing interrupted changes in `pipeline/phases.rs` are included in this cycle's verified repair: the adapter uses the entered command offset when wrapping the stage failure
        - verification scope is darkmatter only: the repair changes an internal stage and its tests, with no public API or downstream consumer changes
        - input robustness matrix does not apply to this text-grammar/runtime diagnostic repair; no file-format/configuration loader changed
        - no notable cross-OS risk: no OS APIs, path comparison, process-launch implementation, or platform-specific branches changed; portable CRLF and multibyte geometry are covered in L1, with normal CI providing per-OS coverage
- starting phase verification (full `just test`, then `just lint` and `just check-tier-coverage darkmatter`) at 15:21:00

- phase gates verified:
        - darkmatter `just test`: 13,538 passed, 10 skipped; exit 0; 155.242 seconds reported by nextest
        - darkmatter `just lint`: exit 0; library, CLI, DMLS, Zed CLI, and Zed WASM checks pass; `_lint` runs Clippy and no formatting command
        - repository root `just check-tier-coverage darkmatter`: exit 0; 0 stranded tests. Initial package-area invocation reported the recipe absent; reran from its owning root successfully
        - seven preexisting tests exceeded five seconds under the full suite; rerunning each alone to verify load sensitivity, without rerunning the full suite

- isolated timing verification completed; all seven tests passed below five seconds:
        - `just test injection_shaped_names_never_execute`: 1 passed, 1.292 s
        - `just test scalar_unions`: 1 passed, 2.523 s
        - `just test ctrl_c_runs_cancelled_then_finalize_and_exits_130`: 1 passed, 0.372 s
        - `just test every_suggested_rule_approves_the_command`: 1 passed, 2.066 s
        - `just test the_shared_corpus_valid_depth_fixtures_raise_no_schema_diagnostic`: 1 passed, 2.280 s
        - `just test the_shared_corpus_valid_generated_verb_fixtures_raise_no_schema_diagnostic`: 1 passed, 1.903 s
        - `just test every_shared_declaration_publishes_the_baseline_verdict_at_its_anchor`: 1 passed, 2.025 s
        - classified as load-sensitive existing tests; no retries, timing overrides, tier changes, or further full-suite run

### Completion Report

- **outcome:** complete — implementation complete, ready for review
- **elapsed:** 15 min 50 s (15:09:50 → 15:25:40 local)
- **parent findings:** total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- **child/sweep rows:** 8 total = fixed 5 + clean 3; no pending or open rows
- verified repairs:
        - R1-F2.14 / R1-F2.14a: first/later shell-block command failures retain their own authored line, column, span, and original iteration path for map and filter
        - R1-F2.14b / R1-F2.16: the shared library record and CLI report-json agree on command provenance, with no message reconstruction; null argument validation launches nothing
        - R1-F2.X3: execution re-enters each prepared command, so failures in the first or later command preserve the correct site
        - R1-F2.X4: multiline earlier arguments use authored-source projection rather than inferred line positions; CRLF, multibyte text, and quoted containers retain exact command starts
        - R1-F2.13 / R1-F2.5: ordinary directives retain their failing directive location, and combined block-output exhaustion retains its block insertion site
- open findings/child IDs: none; no blockers, disputes, author decisions, or deferred performance measurements
- changed files in this cycle, including the verified interrupted repair:
        - `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs`
        - `darkmatter/lib/src/markdown/compose/pipeline/phases.rs` (existing interrupted command-site adapter retained and verified)
        - `darkmatter/lib/tests/l1/collection_blocks/provenance.rs`
        - `darkmatter/cli/tests/l1/collection_blocks.rs`
        - `darkmatter/docs/inline/shell-blocks.md`
        - `darkmatter/docs/cli/report-json.md`
        - `.claude/skills/darkmatter/compose.md`
        - this log and `review-2.md` completion metadata
- final verification for R1-F2 and every child:
        - focused: `just test provenance::` 77 passed; `just test report_locates_` 6 passed; `just test shell_blocks::` 102 passed
        - full: darkmatter `just test` 13,538 passed, 10 skipped; `just lint` passed; root `just check-tier-coverage darkmatter` 0 stranded; seven individually rerun timing controls passed below 3 s
        - public API and CLI tests provide L1 evidence; no terminal/browser behavior requires L2/L3, and no OS-specific implementation requires a cross-check
        - test and lint transcripts: `/tmp/darkmatter-review2-full-test.log`, `/tmp/darkmatter-review2-lint.log`, `/tmp/darkmatter-review2-tier.log`; temporary transcripts supplement the durable results recorded here
- review metadata preserves `ready: false`, findings, and reviewer body for independent review; `implemented: true` records only this completed repair cycle
- no formatting command or commit was run
