---
$schema: feature-review.yaml
ready: true
findings: []
observations:
  - Regression fixtures still depend on the active feature directory
  - CommonJS documentation overstates final-interface guarantees
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-11T11:30:49-07:00"
spec: 2026-10-11-reexports-all-languages/spec.md
implemented: false
description: "A **feature** review of `2026-10-11-reexports-all-languages/spec.md`"
feature: 2026-10-11-reexports-all-languages/review-3.md
previous: 2026-10-11-reexports-all-languages/review-2.md
---

# Review 3: Re-exports across all supported languages

The feature is **ready under the accepted support contract**. Both high findings and both medium findings from review 2 are verified fixed. The original review-1 cases remain fixed, and the additional CommonJS overwrite repair recorded in the implementation ledger passes its enumerated cases. No unresolved required repair or human decision blocks readiness.

The broader probe found a boundary worth documenting: repeated keys inside a CommonJS object literal retain the earlier written forwarding evidence. That differs from the new handling of separate interface assignments, but the spec does not require a general final-object evaluator. This is an observation about the documentation's stronger promise, not another blocking repair cycle.

## Previous Finding Verification

Read [review 2](review-2.md), [review 1](review-1.md), the original [implementation log](implementation-log.md), and both repair inventories in [log.md](log.md). Reconciled frontmatter, body findings, child-instance tables, required repairs, ledger additions, and completion claims against source and running tests. No finding was disputed. Review 2 had no blocked finding, and no repair depended on a prerequisite changing during this cycle. The spec's **Author rulings**, item “C++20 modules: option 1,” accepted reporting module syntax as a grammar gap before implementation; that accepted limitation remains in place and is not reopened.

The evidence below uses independently asserted expected names, coverage, qualifiers, and markers in the named tests, not merely agreement between two implementations. All named tests ran in the passing full L1 suite. The 47 additional public probes are described under Verification.

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
| --- | --- | --- | --- | --- |
| R2-F1 — Conditional Python export-list edits become unconditional evidence | R2-F1.1–R2-F1.5: conditional append, remove, extend, assignment/concatenation and `+=`; R2-F1.6–R2-F1.8: Perl, Lua and JS/TS controls; ledger R2-F1.9: membership, markers, diagnostic exemption, query/cache/CLI and stale revision | verified fixed | In tree-hugger, `all_method_call` sends append/remove through the same conditional gate in `apply_all` as the other edits. `python_all_edited_under_a_condition_is_computed_whatever_the_edit` checks the five Python paths; `python_unguarded_append_and_remove_are_definite` checks positive and negative controls. `a_conditional_all_append_does_not_exempt_the_import` retains the warning. `perl_guarded_push_unshift_and_tags_stay_conditional`, the Lua conditional-write tests and the CommonJS conditional tests check the other three mechanisms. Public guarded append/remove probes are limited, empty and unmarked; the unguarded append is complete and marked. Revision 55 and the revision-54 stale-cache row are present and run. | None. Conditional list folding no longer invents unconditional membership or absence. |
| R2-F2 — Re-export cache tests exceed the Level 1 time budget | R2-F2.1–R2-F2.5: complete/limited field matrices, complete/limited inconsistencies, not-applicable inconsistencies; R2-F2.6: widening, nullable names, duplicate/trailing JSON; ledger R2-F2.7: generated-row guard | verified fixed | In tree-hugger-cli, `matrix_row_tests!` schedules 44 complete-field, 13 limited-field, 15 complete-inconsistency, 4 limited-inconsistency and 2 not-applicable rows separately. Each row retains producer/hit controls, refusal reasons, recomputation and public output comparison. `every_reexport_matrix_row_has_a_generated_test` compares all five tables with their generated lists and rejects duplicate row names. Full-suite maximum for these rows: **3.319 s**. The three individual consumers also pass within budget. Both packages declare their test binaries, the modules are included, and tier coverage reports zero stranded tests. | None. No rows were removed, ignored or moved to a slower tier. |
| R2-F3 — Parenthesized Lua table writes leave stale exported fields | R2-F3.1–R2-F3.4: assembled/constructor tables, dot/bracket writes; R2-F3.5–R2-F3.6: nested mutations and bare/direct-return controls; ledger R2-F3.7–R2-F3.13: deeper parentheses, `and`/`or` targets, positive field writes, loader/constructor and dynamic-call spellings, repeated assignment targets, query/cache | verified fixed | In tree-hugger, `field_target` and `indexed_use` use shared table normalization: only a target the collector can fold is exempted from mutation limits. The eight new `lua_parenthesized_*`, `lua_write_through_an_and_or_expression_withholds_the_field` and `lua_field_written_twice_in_one_statement_is_withheld` tests check the ledger's added cases. Public dot/bracket overwrite probes retain only untouched `bar`; an `or` target limits coverage and withholds `foo`. `lua_parenthesized_spellings_survive_the_cache` checks cold/hit/wide/narrowed results. Loader and dynamic forms use the same normalization. | None for the required instances. Unknown computed string keys remain the documented bounded fallback. |
| R2-F4 — Bare variable redeclarations erase proven CommonJS forwards | R2-F4.1–R2-F4.5: whole module, member, namespace-object, destructured member and object in JS/TS; R2-F4.6–R2-F4.8: scoped/bare controls, actual writes, ES/TS readers; ledger R2-F4.9: incorrect comment | verified fixed | In tree-hugger, `Table::build` and nested-variable `add_writes` count initialized declarations rather than bare declarations. `cjs_bare_var_redeclaration_assigns_nothing_and_keeps_the_forward` asserts all five consumers in both extensions and the bare-first control. `cjs_nested_bare_var_and_top_level_function_of_the_same_name_keep_the_forward` checks the scoped controls. `cjs_initialized_var_redeclaration_and_block_function_rebind` retains actual-write negatives and an ES-import control. ES/TS dedicated readers continue to use `resolve_es`, separate from CommonJS binding candidates. The comment now describes initialized declarations. Public bare-var probes give complete module forwards in both extensions. | None. The repair regression reported by review 2 is closed. |
| R2-D1 — Later CommonJS interface writes leave earlier forwards complete (added by the repair ledger) | R2-D1.1–R2-D1.6: same-member/whole replacement, cross-root spelling, mutation, conditional/function/computed/merge/augmented writes; R2-D1.7–R2-D1.12: distinct-member/later-forward controls, deletion, detached `exports`, uncertain replacement; R2-D1.13–R2-D1.16: TS `export =`, imported-module marker, ES-import marker and diagnostics | verified fixed | `cjs::Interface` runs after CommonJS and TS entry collection and before import marking. `cjs_later_definite_write_replaces_the_earlier_forward`, `cjs_later_write_that_may_replace_or_mutates_the_forward_withholds_it`, `cjs_later_write_of_another_member_or_through_detached_exports_keeps_the_forward`, `ts_export_equals_and_module_exports_replace_each_other`, and both import-consumer tests assert all 16 ledger children. Public separate-member/whole-overwrite, detached and distinct-member probes match the expected results. ES imported `read` forwarded then overwritten is unmarked; the existing read occurrence still prevents an unused-import warning. | None for the enumerated repair. Repeated object-literal keys are a separate, previously unenumerated boundary discussed under Observations. |
| R1-F1 — Same-file binding checks confuse replaced or shadowed names with imported bindings | All eight original children below, plus the R2-F4 repair regression | verified fixed | Original cases continue to pass; the repeated-bare-variable regression is independently closed above. Exact-span import marking, kind projection and cache checks preserve the corrected report. | None. |
| R1-F1.1 — Python replacement | Named/namespace replacement, exclusive fallback branches, replaced import's marker and warning | verified fixed | `python_a_later_named_import_of_the_same_name_replaces_the_earlier_one`, `python_sequential_namespace_aliases_forward_only_the_final_module`, `python_fallback_imports_in_exclusive_branches_both_stay_conditional`, and `a_replaced_python_import_keeps_its_unused_import_warning`. | None. |
| R1-F1.2 — JS/TS whole-module shadowing | Nearer block binding; unaffected function-local control | verified fixed | `cjs_whole_module_binding_hidden_by_a_block_local_is_not_forwarded`, both extensions; nearest-scope resolution precedes emission. | None. |
| R1-F1.3 — JS/TS property consumers | Namespace/destructured, block, catch, loop and function paths | verified fixed | `cjs_property_forward_of_a_binding_hidden_by_a_nearer_declaration_is_not_forwarded` checks every listed path and positive controls in both extensions. | None. |
| R1-F1.4 — JS/TS object consumers | Shorthand/explicit values and unaffected sites | verified fixed | `cjs_object_literal_member_hidden_by_a_block_local_is_withheld_and_other_sites_stay` exercises both fixture edits; hidden imports are withheld while the remaining proved site survives. | None. |
| R1-F1.5 — ES import consumed by CommonJS | Report link, stored marker and direct import flag | verified fixed | `cjs_consumer_of_an_es_import_hidden_by_a_block_local_leaves_the_import_unmarked` asserts no false link and both flags false. | None. |
| R1-F1.6 — Replaced globals | `require`, `module`, `exports`, direct/bound CJS, TS import-require and `export =` | verified fixed | `cjs_reassigned_loader_and_interface_globals_are_not_proven` and `ts_export_equals_after_require_is_reassigned_is_not_proven`, including unaffected-global controls. | None. |
| R1-F1.7 — Scala import links | Hidden member, inaccessible scope, nearest enclosing import | verified fixed | `scala_export_through_a_name_a_member_definition_hides_does_not_link_the_import` and `scala_export_links_only_an_import_in_the_same_or_an_enclosing_scope`; written selector evidence survives while unrelated imports remain unmarked. | None. |
| R1-F1.8 — Lua unrelated locals | Whole module, direct constructor and assembled table; actual chunk-binding write control | verified fixed | Three `lua_write_to_a_function_local_keeps_*` tests and `lua_function_body_write_to_the_chunk_local_still_limits`; declaration identity separates the bindings. | None. |
| R1-F2 — Lua reports a definite interface after the returned table can be modified | R1-F2.1 conditional field, R1-F2.2 function-body write (called/uncalled), R1-F2.3 argument/alias/storage escape, R1-F2.4 constructor escape, R1-F2.5 unknown later key; subsequent R2-F3 spellings | verified fixed | `lua_conditional_field_write_withholds_the_field`, `lua_conditional_write_keeps_the_fields_it_does_not_name`, `lua_field_write_in_a_function_body_withholds_the_field`, `lua_returned_table_passed_to_a_call_is_limited`, `lua_returned_table_stored_elsewhere_is_limited`, `lua_constructor_initialized_table_passed_to_a_call_is_limited`, and `lua_unknown_key_write_withholds_every_field`. R2-F3 closes the enumerated parenthesized siblings. | None. Both incorrect extra entries and required untouched entries are asserted. |
| R1-F3 — Lua constructor keys produce incorrect exported names and miss overwrites | R1-F3.1 four quoted forms, R1-F3.2 local constructor, R1-F3.3 computed keys in both returned consumers/unreturned control, R1-F3.4 equivalent/repeated keys, R1-F3.5 number/boolean keys | verified fixed | `lua_string_literal_constructor_key_names_the_field`, `lua_string_literal_key_in_a_local_table_constructor_names_the_field`, both computed-key tests, `lua_equal_constructor_keys_share_one_field`, `lua_assignment_keys_share_one_field_however_spelled`, and `lua_number_and_boolean_keys_name_no_field`. A public repeated-key Lua constructor remains limited with no invented winner. | None. The previously accepted escaped-literal limitation is unchanged. |
| review-1.md — CLI help still promises re-exports in declaration filters | Both declaration/class `--exported` help strings | verified fixed | In tree-hugger-cli `cli/src/main.rs`, both strings describe exported declarations and synthetic Rust prelude records. Full-index wiring and declaration/prelude regression tests pass. | None. |
| review-1.md — Regression fixtures are coupled to the active feature directory; review-2 observation; ledger R2-O1 | Per-language audit-sample `include_str!` consumers | unresolved | Active-directory embeddings remain in the per-language test modules and compile today. The log explicitly defers relocation to the author's lifecycle move. | Nonblocking observation retained below. No required repair or readiness blocker; agents must not move the feature. |

## Verification

Reviewed the accepted language contracts and audit, planner/report types, strict cache reader, report projection, import conversion and diagnostic consumers. Inspected the repairs in Python, Lua, ECMAScript and Scala and their public-result assertions. Schema is **5.2** and extractor revision is **55**; the earlier schema/revision upgrade rows remain live.

Checks run from `tree-hugger/`, unless stated otherwise:

- `just test reexport`: **433 passed**, 2,092 excluded; 7.088 seconds.
- `just test`: **2,525 passed**, zero skipped; 109.864 seconds. The repaired cache rows remain under five seconds in this full run, with a maximum of 3.319 seconds.
- `just lint`: passed for both packages.
- Root `just check-tier-coverage tree-hugger`: **zero stranded tests**.

The full run reported 15 slow tests. Every one already exists on feature base `1cb5837ed`, verified against the corresponding `cache_robustness.rs`, `golden.rs`, `refs.rs` and `report.rs` files. None is a new re-export test or a required R2-F2 repair left over. The review does not expand this feature into an inherited suite-speed cleanup.

An isolated probe wrote 47 source files and ran the shipped `hug symbols --file <absolute-path> --json-full` command uncached and twice cached. The cases include the prior Python append/remove failures and controls, Lua parenthesized and ambiguous-target repairs, bare-variable and block-function behavior, CommonJS replacement/detachment controls, and the object-member spellings described below. Expected names, source modules and coverage were stated before running the probe. The 29 cases checking required repairs and controls match those expectations; the remaining 18 demonstrate the object-literal documentation boundary in both JS and TS.

A temporary Rust example checked all 47 files through tree-hugger's direct report, complete index, selective query, function-filtered query, direct import flags and persistent cache cold/hit/widened/narrowed results. Cache hits were explicitly confirmed. All surfaces agree; agreement is used to trace the observations, not to assert that retained object-literal entries describe the runtime winner. The temporary example was removed. No production source or tests were modified by this review.

### Requirements and verification levels

| User-facing requirement | Appropriate level | Evidence |
| --- | --- | --- |
| All 16 languages have explicit positive/negative/grammar-gap contracts, names, forms, scope, visibility, type-only and conditional qualifiers | L1 public API with pinned grammar fixtures | Per-language `symbols_reexports_*` modules run in the declared library L1 target; original repair and new conditional/normalization assertions pass. Accepted C++/TS/Scala/JS grammar limits are tested rather than treated as complete negatives. |
| Direct API, complete/selective/filtered reports, import flags and cache agree | L1 public API and hermetic cache | `reexports_equivalence`, per-repair consumers, cross-root CLI cache test and the additional 47-file probe; expected correctness is established separately by fixture assertions. |
| Exempt only supported imported forwards; retain Rust/ES semantics and Rust prelude behavior | L1 diagnostics and shipped CLI | `reexports_consumers`, Rust self/private and prelude tests, full-index import-conversion tests. Guarded Python append keeps its warning; overwritten CommonJS imports lose their marker. |
| Strict persisted JSON and stale-entry rejection | L1 public CLI/cache edits | All generated matrix rows, inconsistency tests, duplicate/trailing tests and revision-upgrade rows run successfully within the feature-test budget. |
| No forwarding-target reads, repeated extraction or unrequested expensive passes | L1 filesystem fixtures and work counters | `extraction_reads_no_forwarding_target_and_shares_one_definitions_pass`, dependency-free snippets and narrowing/widening/hit counters. |
| Full-index JSON exposes the report; ordinary declaration listings remain declaration-only | L1 shipped CLI | Wiring, conversion, report and golden tests. No styled view or input behavior was added, so no new L2 or L3 requirement applies. |

### Input Robustness Matrix

The applicable input format is **persisted cache JSON**. Source-language grammars are excluded from this configuration/file-format matrix. The plan's **Frozen field columns** and tree-hugger-cli's `reexport_columns` / `limited_reexport_columns` remain the concrete field inventory: report, coverage/state/limits/reasons and spans, entries, form/evidence, exported/original/local names, source/scope/restriction, exclusions, caveats, flags, linked/null import spans, forwarding/statement spans and their coordinate leaves, import markers, and completed capabilities. Schema and revision are covered by the existing upgrade/identity checks.

Checked that each shared field column still walks its unedited producer control, absent, explicit-null, wrong whole-field type, wrong single/all elements where applicable, empty and duplicate shapes. Raw duplicate/trailing-document checks are retained. Nullable names/import links are served only where the form permits; missing required keys are refused. Empty complete/limited/not-applicable producer controls and well-typed inconsistent report/marker/capability edits are tested through public cache refusal, recomputation and output. The strict report deserializer and `validate_reexports` preserve the same decisions. Splitting scheduling did not discard cells or replace public assertions with parser-only checks. No additional reader defect was found.

## Unblocked Findings

None. There is no high or medium finding requiring a repair in this iteration.

## Blocked Findings

None. No human review or environmental prerequisite is needed to close this review.

## Recurrence

No finding is carried forward or newly raised, so `recurrence` is false. The binding, conditional-interface and target-normalization findings that recurred between reviews 1 and 2 are closed for every required child and consumer above. The object-literal observation is outside the enumerated separate-write repair and does not establish an additional final-runtime-interface requirement.

## Observations

### Regression fixtures still depend on the active feature directory

The unresolved observation from both earlier reviews remains: per-language L1 tests embed static audit samples from the active feature directory. They compile and run today. When the author moves the feature after review closes, preserve those samples and update the embedding paths in the same change. This lifecycle concern does not block readiness and is not an instruction for an agent to move the feature now.

### CommonJS documentation overstates final-interface guarantees

In tree-hugger, [object_members](../../lib/src/analysis/reexports/ecma/cjs.rs) records shorthand and explicit object members independently; the new interface-write pass settles separate assignments, not later members inside the same object literal. Public probes in both JavaScript and TypeScript show that `{read, read: 1}`, `{read: m.read, "read": 1}`, a literal computed replacement key, a same-name method/getter and `{m, m: 1}` retain the earlier complete forwarding entry. `{read: m.read, read: n.read}` retains both written sites. An ES-import version retains its import marker. Distinct-key and later-import-positive controls are correct; separate same-member/whole assignments now remove stale entries, and Lua's independently specified repeated-constructor-key policy withholds its result. The API/query/cache/CLI trace preserves these outcomes. The spec explicitly says “Distinct sites that expose the same name stay separate” and defines bounded source evidence; it does not impose a general CommonJS object-construction evaluator. Narrow the current [documentation](../../docs/reexports.md), which says a CommonJS forward is reported “only when it is still the file's interface,” to the supported separate-write analysis and explain the object-literal boundary. Repeated object keys are unusual, and another general overwrite/protection layer would cost more than this limitation warrants.
