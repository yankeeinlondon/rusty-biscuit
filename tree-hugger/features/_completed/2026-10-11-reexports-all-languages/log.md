---
implementation_1: "2026-10-11T09:32:20-07:00"
implementation_2: "2026-10-11T10:45:54-07:00"
deferred_perf_measurement: false
---

# Review/Implement Log: reexports-all-languages

## Implementation of Review Findings #1

> **started at:** 2026-10-11T09:32:20-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'tree-hugger/features/2026-10-11-reexports-all-languages/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before any repair
- inventory: 3 high findings (R1-F1, R1-F2, R1-F3), 2 nonblocking observations (O1 CLI help drift, O2 fixture coupling)

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
|---------|-----------------------------------------------------------|-------------|----------------------------------|
| R1-F1   | Same-file binding checks confuse replaced or shadowed names with imported bindings | fixed | all children R1-F1.1–R1-F1.8 fixed; sibling sweep over python.rs, scala.rs, ecma/, lua.rs, mod.rs/common.rs recorded under Progress |
| R1-F1.1 | Python sequential named/namespace imports of the same name: only the final import is forwarded; replaced import marker false | fixed | `python_a_later_named_import_of_the_same_name_replaces_the_earlier_one`, `python_sequential_namespace_aliases_forward_only_the_final_module`, `reexports_consumers::a_replaced_python_import_keeps_its_unused_import_warning`; try/except control `python_fallback_imports_in_exclusive_branches_both_stay_conditional` |
| R1-F1.2 | JS/TS CommonJS `module.exports = m` where `m` is a block-shadowing local: no required-module forward | fixed | `cjs_whole_module_binding_hidden_by_a_block_local_is_not_forwarded` (.js/.ts): no entry, `shadowed_global`; function-local control keeps the forward |
| R1-F1.3 | JS/TS property forwarding through a shadowed namespace/destructured binding (block, catch, loop): no forward from outer import | fixed | `cjs_property_forward_of_a_binding_hidden_by_a_nearer_declaration_is_not_forwarded`: block const/let, catch, for…of, for(let), nested function |
| R1-F1.4 | JS/TS object-literal `module.exports = { a }` under shadowing `a`: no false forward; other sites kept | fixed | `cjs_object_literal_member_hidden_by_a_block_local_is_withheld_and_other_sites_stay` on copied `pos-cjs-destructure.js` |
| R1-F1.5 | JS/TS ES import consumed by CommonJS under shadowing local: no link, marker false | fixed | `cjs_consumer_of_an_es_import_hidden_by_a_block_local_leaves_the_import_unmarked`: marker and `is_reexport` false |
| R1-F1.6 | JS/TS assignment to `require`/`module`/`exports` (incl. TS `export =`): limited, no definite forward | fixed | `cjs_reassigned_loader_and_interface_globals_are_not_proven`, `ts_export_equals_after_require_is_reassigned_is_not_proven` |
| R1-F1.7 | Scala selector import links: no link to hidden (object member) or other-scope import | fixed | `scala_export_through_a_name_a_member_definition_hides_does_not_link_the_import`, `scala_export_links_only_an_import_in_the_same_or_an_enclosing_scope` |
| R1-F1.8 | Lua function-local same-spelling assignment must not rebind the chunk-level import (3 fixtures) | fixed | `lua_write_to_a_function_local_keeps_the_returned_module` / `_returned_constructor` / `_assembled_table` (three copied fixtures); negative control `lua_function_body_write_to_the_chunk_local_still_limits` |
| R1-F2   | Lua reports a definite interface after the returned table can be modified | fixed | all children fixed; `Scan::table_uses` classifies every resolved use of the returned table |
| R1-F2.1 | Conditional write `if c then M.foo = 1 end`: limited, no definite `foo` | fixed | `lua_conditional_field_write_withholds_the_field`, `lua_conditional_write_keeps_the_fields_it_does_not_name` |
| R1-F2.2 | Function-body write to `M.foo`: limited | fixed | `lua_field_write_in_a_function_body_withholds_the_field` (called and uncalled) |
| R1-F2.3 | Argument escape `configure(M)`: limited | fixed | `lua_returned_table_passed_to_a_call_is_limited` (`dynamic_forwarding`), plus alias escapes in `lua_returned_table_stored_elsewhere_is_limited` |
| R1-F2.4 | Constructor-initialized local table + argument escape: limited | fixed | `lua_constructor_initialized_table_passed_to_a_call_is_limited` with complete control |
| R1-F2.5 | Unknown later key `M[key] = 1`: limited | fixed | `lua_unknown_key_write_withholds_every_field` |
| R1-F3   | Lua constructor keys produce incorrect exported names and miss overwrites | fixed | all children fixed; shared `table_key` for constructors, assignments and member reads |
| R1-F3.1 | Direct returned constructor `["x"] = m.x`: name `x` | fixed | `lua_string_literal_constructor_key_names_the_field` (copied `pos-table-ctor.lua`, four quoting styles) |
| R1-F3.2 | Local-table constructor `{["read"] = m.read}`: name `read` | fixed | `lua_string_literal_key_in_a_local_table_constructor_names_the_field` |
| R1-F3.3 | Computed key `[key] = m.read` (both consumers): no named entry, limited | fixed | `lua_computed_constructor_key_withholds_every_field`, `lua_computed_key_in_an_unreturned_table_does_not_limit` |
| R1-F3.4 | `x = m.x, ["x"] = 1`: no `x` forward | fixed | `lua_equal_constructor_keys_share_one_field`: every repeat with an import-derived value limited, no entry (Lua §3.4.9 undefined order); assignment path `lua_assignment_keys_share_one_field_however_spelled` |
| R1-F3.5 | Numeric key `[1] = m.read`: no named entry | fixed | `lua_number_and_boolean_keys_name_no_field` |
| O1      | Observation (nonblocking): `--exported` help says "re-exports included" | fixed | `cli/src/main.rs` `--exported` help for symbols and classes now says declaration-only, matching `docs/cli.md` |
| O2      | Observation (nonblocking): L1 tests embed samples from the active feature dir | deferred | nonblocking; relocating samples belongs to the author's lifecycle move, recorded for that step |

### Progress

- plan: findings are processed serially by foreground subagents; R1-F1 is split by language family (F1-a Python+Scala, F1-b ECMAScript, F1-c Lua) because the binding code is per extractor; R1-F2 and R1-F3 follow, both in `lua.rs`
        - `EXTRACTOR_REVISION` is bumped once (53 → 54) by the first repair, with its cache-upgrade row; later repairs in this same uncommitted cycle share it
- starting the work on 'R1-F1 (a) Python and Scala binding replacement/scope' at 09:32:51
        - defect class: name-based binding tracking ignores replacement or lexical scope, producing false import provenance and missing required forwards
        - Python: `later_import_replacement`/`statement_relation` in `python.rs`; an import that always runs later replaces an earlier same-name import (no limit), one that may run adds `rebound_binding` on the earlier import; exclusive `try`/`except` and `if`/`elif`/`else` arms stay alternatives; unaliased `import pkg.a`/`import pkg.b` bind one object
        - Scala: `link_import` links only an import that precedes the export in the same or an enclosing scope, nearest wins, and no hiding `val`/`var`/`def`/`object`/`enum`/`given`/class parameter in between
        - `EXTRACTOR_REVISION` 53 → 54; cache-upgrade row `entry_from_extractor_revision_fifty_three_is_not_served_after_binding_scope_repairs`; goldens `rev.53` → `rev.54`
        - 8 new L1 tests (5 Python, 2 Scala, 1 consumer); 6 of them shown failing on pre-fix sources
        - subagent checks: `just test reexport` 300 passed; `just test` 2390 passed; `just lint` passed
        - residual (nonblocking, recorded): named import followed by a star import keeps both forwards (needs cross-file knowledge); Scala inherited members and `case class` companions are not seen as hiding names
- work completed for 'R1-F1 (a) Python and Scala binding replacement/scope' at 09:49:38
- starting the work on 'R1-F1 (b) ECMAScript CommonJS/ES binding scope and loader globals' at 09:49:38
        - defect class (ECMAScript instance): binding checks looked names up by spelling, ignoring nearer declarations and reassignment of `require`/`module`/`exports`
        - new `Table::resolve_at` (scope walk over blocks, function params, catch, `for`/`for…of` heads, `switch`, class-expression names, hoisted `var`); `global_replaced` detects writes to the loader/interface globals; `Binding::loader` marks bindings made by the global `require`
        - every name-to-binding resolver in `ecma/` routed through it: `cjs::whole`, `cjs::property`, `object_members`, `is_shadowed`/`global_shadowed`, ES `export_statement`/`export default`/`export =`/`export import`; `Table::add_writes` now counts for-of targets, hoisted block `var` and repeated `var`
        - limit reason chosen: `shadowed_global` (spec enum "require/module/exports/import shadowed"; matches existing block-shadow fixture)
        - 8 new L1 tests (both extensions each), all 8 shown failing on pre-fix ecma sources
        - subagent checks: `just test reexport` 308 passed; `just test` 2398 passed; `just lint` clean
        - residual (nonblocking): sloppy-mode block-function hoisting, `with`/`eval` not modeled; a global reassignment anywhere in the file withholds forwards (conservative)
- work completed for 'R1-F1 (b) ECMAScript CommonJS/ES binding scope and loader globals' at 10:03:46
- starting the work on 'R1-F1 (c) Lua function-local rebinding' at 10:03:46
        - defect class (Lua instance): names were matched by spelling across the file, so a nearer `local`, `local function`, parameter or loop variable was treated as the chunk-level `require` local or returned table
        - every Lua name use now resolves to its declaration (`resolve`, `Scan::bound`, writes keyed by declaration id): write collection (`collect_writes`), `lookup`, `table_mut`/`field_target`, `returned_single`, `check_require_shadow`, each `require(..)` call; `calls_require`/`forwarding_damage`/`pcall(require, ..)` clean (limit-only or chunk-level only)
        - extra sibling fixes found by the sweep: a later chunk-level `local m`/`local function m` hides the earlier require local; `function m() end` is a write to local `m` (correct Lua semantics: the statement assigns the in-scope local; kept); `<const>`/`<close>` attributes no longer shift multiple-assignment pairing
        - 9 new L1 tests; 8 shown failing on pre-fix `lua.rs`
        - subagent checks: `just test reexport` 316 passed; `just lint` passed
- work completed for 'R1-F1 (c) Lua function-local rebinding' at 10:10:56
- starting the work on 'R1-F2 Lua returned table modified after an interface snapshot' at 10:10:56
        - defect class: a mutable interface is reported from an earlier snapshot without accounting for later writes or escape to code that can change it
        - new `Scan::table_uses`/`table_use`/`indexed_use` classify every reference that resolves to the returned table's declaration: reads and chunk-level literal-key writes pass; a nested-block or function-body literal-key write gives `rebound_binding` and withholds that field; a non-literal-key write withholds every field; any escape (call argument, method receiver `M:f()`, alias, stored in a table, `M or x`, nested `return M`) gives `dynamic_forwarding` and withholds every field
        - limit reasons follow the spec enum: `rebound_binding` = "interface table reassigned" (partially), `dynamic_forwarding` = runtime forwarding (same family as metatables)
        - samples `neg-computed-key.lua` and `neg-field-in-function.lua` were asserted as complete negatives; they are now limited (that was the defect); `amb-copy-loop.lua` gains `rebound_binding`
        - 10 new L1 tests (direct API and query path), 3 existing tests adjusted to the corrected contract
        - subagent checks: `just test reexport` 326 passed; `just test` 2416 passed; `just lint` clean
        - residual (nonblocking, by contract): `pairs(M)` and other argument uses now limit (the audit's no-argument-escape rule)
- work completed for 'R1-F2 Lua returned table modified after an interface snapshot' at 10:24:29
- starting the work on 'R1-F3 Lua constructor keys' at 10:24:29
        - defect class: constructor key expressions were treated as resolved field names, so equivalent keys failed to share an identity and computed keys became fabricated names
        - one key reader `table_key` (via `field_key` for constructors, `index_key` for assignments/member reads): identifier or verbatim string literal (any quoting) → `Key::Name`; number/boolean → `Key::NonString` (skipped, never collides with a string field); anything else, including escaped strings, a long bracket starting with a newline, and `""` → `Key::Unknown` (documented limitation, no decoding layer)
        - an unknown key in a returned constructor withholds every field with `rebound_binding` (same as `M[key] = v`); a table never returned gets no limit
        - sweep fixes beyond the review: `field_target`, `function M.f()`/`M:f()`, member read `m["x"]`, `indexed_use` (number-key writes no longer limit), metatable check now recognizes `{["__index"] = m}`
        - orchestrator decision on the subagent's open question: Lua 5.4 §3.4.9 leaves constructor assignment order undefined when a key repeats, so source order is not proof; a repeated constructor key where either value is import-derived now fails the whole constructor (`rebound_binding`, no entries); repeats of purely local values are unaffected. `lua_equal_constructor_keys_share_one_field` rewritten to assert all four shapes limited plus a local-only repeat control; docs and skill updated
        - 13 new Lua L1 tests + 1 ECMAScript check (`cjs_string_literal_property_key_is_the_unquoted_name`)
        - checks: `just test reexport` 339 passed (after the orchestrator change); `just lint` passed (subagent)
- work completed for 'R1-F3 Lua constructor keys' at 10:32:01
- starting the work on 'O1/O2 observations' at 10:32:01
        - O1: `--exported` help in `cli/src/main.rs` (symbols and classes args) said "re-exports included"; the spec's CLI-behavior section requires declaration-only wording, so the two doc comments were aligned with `docs/cli.md` (small, inside spec scope)
        - O2: fixtures embedded from the feature directory are a lifecycle-move concern for the author; nonblocking and deferred (moving samples now would churn every `include_str!` path while the spec is under review)
- work completed for 'O1/O2 observations' at 10:32:01
- starting final verification at 10:32:01
- final verification (orchestrator, after the last change), from `tree-hugger/`:
        - `just test`: 2429 run, 2429 passed, 0 skipped (152.5 s; 30 slow under host load, no failure; baseline at review was 2381)
        - `just lint`: passed for `tree-hugger` and `tree-hugger-cli`
        - `just test reexport`: 339 passed
        - from the repo root, `just check-tier-coverage tree-hugger`: 0 stranded
        - OS risk: the repairs are tree-walking and string logic with no path, process or filesystem handling, so no cross-OS rig run was needed; CI covers Linux on the PR
- work completed for final verification at 10:35:08

### Completion Report

- **outcome: complete**; elapsed 1 h 3 min (09:32:20 → 10:35:08)
- parent findings: total 3 = fixed 3 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0 + disputed 0
        - instance rows: 18 of 18 fixed (R1-F1.1–R1-F1.8, R1-F2.1–R1-F2.5, R1-F3.1–R1-F3.5)
        - observations, nonblocking: O1 fixed, O2 deferred to the author's lifecycle move
- verified repairs:
        - R1-F1: binding checks are now scope- and replacement-aware in Python (later imports replace earlier ones; exclusive branches stay alternatives), Scala (links only an earlier import in the same or an enclosing scope with no hiding definition), ECMAScript (`resolve_at` scope walk at every resolver, reassigned `require`/`module`/`exports` → `shadowed_global`) and Lua (every name resolved to its declaration)
        - R1-F2: every use of the returned Lua table is classified; nested or function-body writes withhold that field (`rebound_binding`), unknown-key writes and escapes withhold every field
        - R1-F3: one Lua key reader for constructors, assignments and member reads; quoted literal keys name the field, numbers/booleans name none, unknown keys and import-involved repeated constructor keys withhold the constructor
- open findings: none. Residual, nonblocking notes for the next review: a named Python import followed by a star import keeps both forwards (needs cross-file knowledge); Scala inherited members and `case class` companions are not modelled as hiding names; ECMAScript sloppy-mode block-function hoisting and `with`/`eval` are not modelled; escaped Lua string keys are a documented limitation
- changed files:
        - library: `lib/src/analysis/reexports/{python.rs,scala.rs,lua.rs}`, `lib/src/analysis/reexports/ecma/{bindings.rs,cjs.rs,mod.rs}`, `lib/src/file/mod.rs` (`EXTRACTOR_REVISION` 53 → 54)
        - CLI: `cli/src/main.rs` (O1 help text), `cli/tests/symbols/cache_upgrade.rs` (revision 53 row), `cli/tests/fixtures/golden/symbols.{json,jsonl}` (`rev.54`)
        - tests: `lib/tests/l1/{symbols_reexports_python_perl.rs,symbols_reexports_swift_scala_csharp.rs,symbols_reexports_ecma.rs,symbols_reexports_go_lua.rs,reexports_consumers.rs}`
        - docs and skill: `docs/reexports.md`, `.claude/skills/tree-hugger/{analysis-pipeline.md,testing.md}`
- verification commands tied to findings: `just test reexport` (R1-F1, R1-F2, R1-F3 focused, 339 passed); `just test` (all findings plus goldens/cache-upgrade, 2429 passed); `just lint` (all, passed); `just check-tier-coverage tree-hugger` (new tests selected, 0 stranded)

## Implementation of Review Findings #2

> **started at:** 2026-10-11T10:45:54-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'tree-hugger/features/2026-10-11-reexports-all-languages/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before any repair
- inventory: 2 high findings (R2-F1, R2-F2), 2 medium findings (R2-F3, R2-F4), 1 nonblocking observation (R2-O1 fixture coupling); no blockers; recurrence: true (R2-F1, R2-F3, R2-F4 are siblings of R1-F1/R1-F2)

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
|---------|-----------------------------------------------------------|-------------|----------------------------------|
| R2-F1   | Conditional Python export-list edits become unconditional evidence | fixed | all children fixed or clean; extra Python siblings fixed (nested `__all__` calls in expressions; `__all__` edits/`global __all__` in function/class/lambda/comprehension bodies); orchestrator CLI probes confirmed |
| R2-F1.1 | Python conditional `__all__.append("read")` on empty list: limited, no unconditional forward, marker false, unused warning kept | fixed | `python.rs` `all_method_call` routes append through `apply_all`; `python_all_edited_under_a_condition_is_computed_whatever_the_edit`, `reexports_consumers::a_conditional_all_append_does_not_exempt_the_import` (warning kept), control `python_unguarded_append_and_remove_are_definite`; CLI probe: limited `computed_export_list`, marker false |
| R2-F1.2 | Python conditional `__all__.remove("read")` on `["read"]`: limited, removal not proof of absence | fixed | same gate for remove; same tests; CLI probe limited `computed_export_list` |
| R2-F1.3 | Python conditional `.extend([...])`: limited/computed (clean control) | clean | already gated by `apply_all`; `if extend` case added to `python_all_edited_under_a_condition_is_computed_whatever_the_edit` |
| R2-F1.4 | Python conditional literal assignment / concatenation: limited (clean control) | clean | already gated; `if assign`/`if concat` cases added; `python_all_set_under_a_condition_is_computed` |
| R2-F1.5 | Python conditional `+= [...]`: limited (clean control) | clean | already gated; `if +=` case added |
| R2-F1.6 | Perl guarded list/tag/push/unshift: entries `conditional: true` (clean) | clean | Perl keeps per-item `conditional` (no fold); new `perl_guarded_push_unshift_and_tags_stay_conditional` passes without code change |
| R2-F1.7 | Lua guarded field write: limited/rebound (clean) | clean | `lua_conditional_field_write_withholds_the_field`, `lua_conditional_write_keeps_the_fields_it_does_not_name`, `lua_field_write_in_a_function_body_withholds_the_field` |
| R2-F1.8 | JS/TS guarded CommonJS forward: `conditional: true` (clean) | clean | `cjs_conditional_dynamic_and_merge_forms_are_limited_and_the_guarded_one_is_conditional`, `cjs_try_and_ternary_guarded_assignments_are_conditional`; overwrite gap found here became R2-D1 |
| R2-F1.9 | Consumers: declared membership, import marker, unused-import exemption, query/cache/CLI agree; extractor revision bump + stale-cache row | fixed | `EXTRACTOR_REVISION` 54→55; `cache_upgrade::entry_from_extractor_revision_fifty_four_is_not_served_after_conditional_list_edits_are_limited`; golden `rev.55`; consumer test above |
| R2-F2   | Re-export cache tests exceed the Level 1 time budget | fixed | all matrices split into per-row `#[test]`s via `matrix_row_tests!`; full `just test` (173.6 s, heavy load): slowest feature test 4.324 s; every slow test is pre-feature |
| R2-F2.1 | Complete report field matrix (`check_reexport_columns`): each scheduled L1 test < 5 s under full suite | fixed | `complete_reexport_report_fields::*` (44 tests), thread fan-out removed; full run max 2.805 s (`::reexports`) |
| R2-F2.2 | Complete well-typed inconsistency matrix (`check_inconsistencies`): < 5 s, no dropped cases | fixed | `complete_reexport_inconsistencies_are_refused::*` (15 tests); full run ≤ 0.82 s |
| R2-F2.3 | Limited field matrix: stays < 5 s, cells preserved | fixed | same fan-out pattern → `limited_reexport_report_fields::*` (13 tests); full run max 2.779 s |
| R2-F2.4 | Limited inconsistency matrix: stays < 5 s | fixed | `limited_reexport_inconsistencies_are_refused::*` (4 tests) ≤ 0.27 s |
| R2-F2.5 | Not-applicable inconsistency matrix: stays < 5 s | fixed | `not_applicable_language_is_complete_only_as_not_applicable::*` (2 tests) ≤ 0.27 s |
| R2-F2.6 | Widening / nullable-name / duplicate-trailing tests: stay < 5 s | clean | single-case tests, 0.39–0.69 s in full run; no change needed |
| R2-F2.7 | Guard test: generated per-row tests match the shared matrix (no untested columns/cases) | fixed | `every_reexport_matrix_row_has_a_generated_test` compares each table with the generated list; mutation (broken refusal reason) caught |
| R2-F3   | Parenthesized Lua table writes leave stale exported fields | fixed | shared `unparenthesized`/`table_identifier`/`callee` normalization in `lua.rs`; siblings R2-F3.7–F3.12 fixed; CLI probes confirmed |
| R2-F3.1 | Assembled table `(M).foo = 1`: no stale `foo` | fixed | `lua_parenthesized_chunk_level_write_replaces_the_field`: complete, `bar` only; CLI probe complete, no `foo` |
| R2-F3.2 | Assembled table `(M)["foo"] = 1`: no stale `foo` | fixed | same test |
| R2-F3.3 | Constructor table `(M).foo = 1`: no stale `foo` | fixed | same test; CLI cache miss/hit agree |
| R2-F3.4 | Constructor table `(M)["foo"] = 1`: no stale `foo` | fixed | same test; CLI probe complete, no `foo` |
| R2-F3.5 | Nested block/function parenthesized write: limited (clean control) | clean | `lua_parenthesized_nested_write_still_withholds_the_field` |
| R2-F3.6 | Bare `M.foo=1` and direct `return {foo=m.foo}` controls (clean) | clean | bare control in `lua_parenthesized_chunk_level_write_replaces_the_field`; direct return in `lua_reassigned_table_and_import_metatable_keep_their_reasons` |
| R2-F3.7 | Discovered: `((M)).foo = 1` → no stale `foo` | fixed | `lua_parenthesized_chunk_level_write_replaces_the_field` |
| R2-F3.8 | Discovered: write through `(M or {})` / `(x or M)` / `(M and x)[..]` → limited `rebound_binding`, field withheld | fixed | `lua_write_through_an_and_or_expression_withholds_the_field` |
| R2-F3.9 | Discovered: `(M).foo = m.foo` as the only write → complete `foo` (was missing) | fixed | `lua_parenthesized_chunk_level_write_of_an_import_forwards_the_field` |
| R2-F3.10 | Discovered: `(require)("m")`, `require(("m"))`, `local M = ({..})` → forward reported | fixed | `lua_parenthesized_requires_and_constructors_forward` |
| R2-F3.11 | Discovered: parenthesized `setmetatable`/`pcall`/`module` forms → limited `dynamic_forwarding` | fixed | `lua_parenthesized_dynamic_forms_still_limit` |
| R2-F3.12 | Discovered (adjacent class): `M.foo, M.foo = m.foo, 1` → limited `rebound_binding`, field withheld | fixed | `lua_field_written_twice_in_one_statement_is_withheld` |
| R2-F3.13 | Cache/query consumers of the parenthesized spellings agree with uncached result | fixed | `lua_parenthesized_spellings_survive_the_cache` (cold, hit, complete index, narrowed) |
| R2-F4   | Bare variable redeclarations erase proven CommonJS forwards | fixed | `bindings.rs` `initialized_names` used by `Table::build` and nested-`var` `add_writes`; siblings (`var m; var m = require`, block `function m`, `export var m`) fixed |
| R2-F4.1 | JS/TS whole module after bare `var m;`: complete module forward | fixed | `cjs_bare_var_redeclaration_assigns_nothing_and_keeps_the_forward` (.js/.ts); CLI probe complete `module` from `m` |
| R2-F4.2 | JS/TS member `exports.read=m.read` after bare `var m;`: complete named forward | fixed | same test |
| R2-F4.3 | JS/TS object member `module.exports={m}`: complete namespace entry | fixed | same test |
| R2-F4.4 | JS/TS destructured member after bare `var read;`: complete named forward | fixed | same test; CLI probe (.ts) complete `named read` |
| R2-F4.5 | JS/TS destructured object after bare `var read;`: complete named forward | fixed | same test |
| R2-F4.6 | Block/function bare declaration control (clean) | clean | `cjs_nested_bare_var_and_top_level_function_of_the_same_name_keep_the_forward` |
| R2-F4.7 | Repeated initialized `var m={}`: limited (clean control) | clean | `cjs_initialized_var_redeclaration_and_block_function_rebind`: limited, no entry |
| R2-F4.8 | ES/TS syntax consumers through `resolve_es` unaffected (clean invariant) | clean | ES/TS syntax consumers resolve through `resolve_es` in `ecma/mod.rs`; ES imports never enter the `cjs` map |
| R2-F4.9 | Stale comment claiming every second `var` replaces the value corrected | fixed | `Table::build` comment now says only initialized redeclarations replace a value |
| R2-D1   | Discovered (sweep of R2-F1/R2-F3 class): a later CommonJS write to `module.exports` / `exports.x` / `module.exports.x` leaves the earlier forward reported as complete | fixed | `cjs::Interface` post-pass: later definite write drops the stale entry; guarded/nested/augmented/merge/onto-forwarded-module writes limit with `rebound_binding`; 16 shapes (D1.1–D1.16) in `symbols_reexports_ecma.rs` + consumer marker test; CLI probes complete, no stale entry |
| R2-D1.1 | `exports.x = req.x; exports.x = 1` → no stale entry | fixed | `symbols_reexports_ecma.rs` later-write tests (.js/.ts); CLI probe |
| R2-D1.2 | `module.exports.x` / `exports.x` overwrite each other → no stale entry | fixed | same tests |
| R2-D1.3 | `module.exports = require("a"); module.exports = {}` → no stale `module` | fixed | same tests; CLI probe |
| R2-D1.4 | member write onto a forwarded whole module → limited `rebound_binding` | fixed | same tests |
| R2-D1.5 | `exports.x = ..; module.exports = {..}` → stale member dropped | fixed | same tests |
| R2-D1.6 | guarded / function-body / computed / `Object.assign` / augmented later writes → limited, entry withheld | fixed | same tests |
| R2-D1.7 | later write of a different member → earlier forward kept, complete (control) | clean | same tests |
| R2-D1.8 | plain value then import-derived write → later forward reported | clean | same tests |
| R2-D1.9 | `delete exports.x` → entry dropped | fixed | same tests |
| R2-D1.10 | `module.exports = {}; exports.x = req.x` → detached `exports`, no entry | fixed | same tests |
| R2-D1.11 | guarded `module.exports = {}` before `exports.x = ..` → limited | fixed | same tests |
| R2-D1.12 | `module.exports = require("a"); exports.y = 1` → complete, forward kept (control) | clean | same tests |
| R2-D1.13 | TS `export = require("a"); module.exports = {}` → no stale entry | fixed | same tests |
| R2-D1.14 | TS `module.exports = require("b"); export = require("a")` → only `a` | fixed | same tests |
| R2-D1.15 | TS `import x = require; export = x; module.exports.y = 1` → limited, import marker false | fixed | same tests |
| R2-D1.16 | ES import forwarded via `exports.x = a` then overwritten → marker and `is_reexport` false; no unused warning since `a` is read (existing lint contract) | fixed | `reexports_consumers.rs` new test |
| R2-O1   | Observation (nonblocking): regression fixtures embed active feature-directory samples | deferred | nonblocking; relocating audit samples is the author's lifecycle step (agents never move the feature); tests compile and run today |

### Progress

- starting the work on 'R2-F1 — Conditional Python export-list edits become unconditional evidence' at 10:46:21
        - defect class: mutable interface-list updates lose their conditional execution when folded into final membership
        - fixed `all_method_call` in `lib/src/analysis/reexports/python.rs`: literal `.append()`/`.remove()` now go through `apply_all`, so a conditional edit is a `computed_export_list` limit like conditional `.extend()`
        - sibling found while sweeping: an `__all__` method call nested in an expression (`enabled and __all__.append("x")`, a call argument) was not seen at all; it is now a `computed_export_list` limit
        - bumped `EXTRACTOR_REVISION` 54 -> 55, added the revision-54 stale-cache row, regenerated `rev.55` in the golden fixtures
        - sibling found while sweeping: `__all__` method calls and `global __all__` inside function, class, lambda and comprehension bodies were dropped (complete negative); now `computed_export_list` via `note_deferred_all_edits`
        - Perl (additive items carry `conditional`), Lua (guarded field write is `rebound_binding`) and JS/TS (sites keep `conditional`; no list fold) verified clean; added `perl_guarded_push_unshift_and_tags_stay_conditional` for the previously untested guarded push/unshift/tags shapes
        - docs: new "Python `__all__` edits" section in `docs/reexports.md`; skill `testing.md` and `analysis-pipeline.md` updated
        - checks: `just test reexport` 343 passed (6.3 s); `just test cache_upgrade` 57 passed; `just test golden` 19 passed; full `just test` 2434 passed (155 s, slow tests are the pre-existing R2-F2 set); `just lint` clean
        - R2-F1 implementation complete, ready for review
        - orchestrator check: diff of `python.rs` reviewed; append/remove now route through `apply_all`
        - discovered sibling (orchestrator reproduction, CLI `--json-full --no-cache`): JS `exports.x = require("a").x; exports.x = 1;` → complete named `x`; `module.exports = require("a"); module.exports = {};` → complete `module` from `a`. A later write to the interface makes the earlier forward stale; spec lists `rebound_binding` ("interface table reassigned") as a JS limit → new ledger row **R2-D1**
- work completed for 'R2-F1 — Conditional Python export-list edits become unconditional evidence' at 10:56:22
- starting the work on 'R2-F2 — Re-export cache tests exceed the Level 1 time budget' at 10:56:22
        - defect class: cache-validation matrices multiply subprocess work inside a single scheduled test, exceeding its execution budget under normal suite load
        - baseline (`just test cache_robustness`, host load avg ~62/16 cores): complete field matrix 13.8 s, complete inconsistency matrix 9.2 s, limited field matrix 5.6 s, limited inconsistency 5.0 s, duplicate/trailing 6.8 s, nullable 1.7 s, widening 2.0 s, not-applicable 0.45 s
        - `cli/tests/symbols/cache_robustness.rs`: removed `check_reexport_columns` (per-column `thread::scope` fan-out) and the serial `check_inconsistencies` loop; the five re-export matrices are now shared tables (`reexport_columns`, `limited_reexport_columns`, `complete_/limited_/not_applicable_reexport_inconsistencies`) with one generated `#[test]` per row via `matrix_row_tests!` (44 complete-field, 13 limited-field, 15 complete-inconsistency, 4 limited-inconsistency, 2 not-applicable tests); every refusal reason, the cache-writing and unedited-hit controls and the uncached-output comparison run in each row's test
        - guard: `every_reexport_matrix_row_has_a_generated_test` compares each table's row names with its generated list (order and uniqueness)
        - mutation check: a wrong refusal reason on `repeated_caveats` fails its generated test; source restored
        - sweep: widening, nullable-name and duplicate/trailing are single-case tests with at most two cells (no matrix); revision 51–54 `cache_upgrade` rows are one test each; left unchanged. Pre-feature thread-fan-out matrices in the same file (`persisted_symbol_fields_*`, `attribute_and_modifier_*`, `reference_evidence_*`, `every_kind_evidence_*`, `retained_geometry_*`, `deleted_written_paths_*`, `summary_and_evidence_*`, `metrics_counts_*`) are out of scope and still exceed 5 s
        - docs: `docs/cache-design/robustness.md` test column and skill `testing.md` updated for the generated test names
        - checks: `just test cache_robustness` 95 passed, max generated 2.886 s; full `just test` 2508 passed (117.6 s, 16 slow, host load avg 29–52), max generated 2.519 s, no feature test over 2.3 s otherwise; every test over 5 s predates the feature; `just lint` clean; `just check-tier-coverage tree-hugger` 0 stranded
        - R2-F2 implementation complete, ready for review
        - orchestrator check: `check_reexport_columns`/`check_inconsistencies` removed; per-row tests generated by `matrix_row_tests!`; guard `every_reexport_matrix_row_has_a_generated_test` present
- work completed for 'R2-F2 — Re-export cache tests exceed the Level 1 time budget' at 11:03:52
- starting the work on 'R2-F3 — Parenthesized Lua table writes leave stale exported fields' at 11:03:52
        - defect class: mutation classification accepts a target spelling as already folded into the interface, while the field collector does not handle that spelling
        - baseline probe (CLI `--json-full --no-cache`, 38 shapes): R2-F3.1–R2-F3.4 and `((M)).foo = 1` complete with stale `foo`; `(M or {}).foo = 1`, `(x or M).foo = 1`, `(M and x).foo = 1` complete with stale `foo` (table_use walks through `and`/`or`, indexed_use exempts them, nothing folds them); `(M).foo = m.foo` alone complete with no `foo`; R2-F3.5/R2-F3.6 controls clean
        - read-side and call-site siblings also misreport as complete negatives: `local M = ({foo = m.foo})`, `(require)("m")`, `require(("m"))`, `setmetatable({}, ({__index = m}))`, `(setmetatable)(...)`, `pcall((require), "m")`, `(pcall)(require, "m")`, `(module)("x")`; clean: `(m).foo`, `(m)["foo"]`, `return (M)`, `return ((M))`, `local m = (require("m"))`, `return ({...})`, `(M):f()`, `configure((M))`, `(M)[key]`, `(M)[1]`
        - fixed `lib/src/analysis/reexports/lua.rs` with one shared normalization: `unparenthesized` (strips any depth of parentheses), `table_identifier` (the identifier an index expression's table names) and `callee`; `field_target` (so `assign`/`define_function` fold `(M).x`/`(M)["x"]`) and `indexed_use` (exempts only writes `table_identifier` names) now use the same function, so a chunk-level write through `(M or t)`/`(t and M)` is a `rebound_binding` write withholding its field instead of an exempt-but-unfolded write
        - same normalization applied to the read/call siblings: `call_name`/`require` loader (`(require)("m")`, `(setmetatable)(..)`, `(pcall)(..)`, `(module)(..)`), `string_argument` (`require(("m"))`), `declare` (`local M = ({..})`), `scan_dynamic` metatable argument (`({__index = m})`), `is_protected_require` first argument (`pcall((require), ..)`); `returned_single` reuses it
        - sibling found outside the spelling class while sweeping: `M.foo, M.foo = m.foo, 1` reported complete with no `foo`, but Lua leaves the order of one statement's assignments undefined; now the field is `undecided` (withheld, `rebound_binding` when its table is returned, cleared by a later definite write), matching the repeated-constructor-key rule
        - left as is (conservative, honest): `M[("foo")] = 1` and `{[("foo")] = m.foo}` stay `Key::Unknown` → limited with every field withheld
        - after-fix probe (same 38 shapes plus 6 extra): R2-F3.1–R2-F3.4 and `((M)).foo` complete, `bar` only; `and`/`or` writes limited `rebound_binding`, `bar` kept; all read/call siblings now forward or limit as expected; CLI cached run of R2-F3.3 (miss then hit) reports complete `bar`
        - tests (`lib/tests/l1/symbols_reexports_go_lua.rs`): `lua_parenthesized_chunk_level_write_replaces_the_field` (R2-F3.1–R2-F3.4 + bare control), `lua_parenthesized_chunk_level_write_of_an_import_forwards_the_field`, `lua_parenthesized_nested_write_still_withholds_the_field` (R2-F3.5), `lua_write_through_an_and_or_expression_withholds_the_field`, `lua_parenthesized_requires_and_constructors_forward`, `lua_parenthesized_dynamic_forms_still_limit`, `lua_field_written_twice_in_one_statement_is_withheld`, `lua_parenthesized_spellings_survive_the_cache` (cold/hit/wide/narrowed); each under 0.04 s
        - mutation check: restoring the bare-identifier `field_target` fails three of the new tests; source restored
        - docs: `docs/reexports.md` Lua section (parentheses paragraph and example, `and`/`or` and same-statement rows, last-write wording); skill `analysis-pipeline.md` Lua paragraph; `lua.rs` module doc and `table_use`/`TableUse`/`TableUses` comments updated
        - checks: `just test lua` 95 passed; `just test reexport` 424 passed (7.0 s); `just test golden` 19 passed; `just lint` clean; no `EXTRACTOR_REVISION` bump (ships under 55)
        - R2-F3 implementation complete, ready for review
        - orchestrator check: subagent's note that ECMA files "changed while it worked" verified false alarm: `ecma/*.rs` mtimes are 09:55–09:58 (iteration 1, uncommitted), before this cycle started
- work completed for 'R2-F3 — Parenthesized Lua table writes leave stale exported fields' at 11:11:18
- starting the work on 'R2-F4 — Bare variable redeclarations erase proven CommonJS forwards' at 11:11:18
        - subagent started R2-F4 at 11:14:21: read review-2 R2-F4, `ecma/bindings.rs`, `ecma/cjs.rs`, `ecma/mod.rs`; probing baseline next
        - defect class: a binding declaration is classified as a value replacement even when it has no initializer
        - baseline probe (CLI `--json-full --no-cache`, .js and .ts): R2-F4.1–R2-F4.5 limited `rebound_binding`, no entry; `var m; var m = require("m")` also limited; `{ function m() {} }` with `var m = require` complete (stale: node 22 shows Annex B makes `m` the function); top-level `function m() {}` complete (node 22: the `var` initializer wins, correct)
        - fixed `ecma/bindings.rs`: `Table::build` now counts names two module-scope declarators *initialize* (new `initialized_names`, shared with the nested-`var` branch of `add_writes`); comment rewritten to say a bare `var m;` assigns nothing; `add_writes` now treats a `function`/generator declaration in a module-level block as a write of a tracked non-ES-import name (sloppy Annex B), `ecma/mod.rs` scan collects those declarations
        - siblings: nested bare `var m;` in a block (observed complete, expected complete); `for (var m in ..)` (limited, expected limited); `for (var m;;)` (complete, expected complete); top-level `function m(){}` (complete, expected complete); block `function m(){}` (complete → limited, fixed); block function under a nearer `let m` or inside a function (complete, expected complete); ES import name with block function in an ES module (complete, expected complete); TS `declare var m;` and `export var m;` (complete after fix; `export var m;` was limited); `let`/`const` redeclarations are syntax errors (not modeled)
        - tests: `cjs_bare_var_redeclaration_assigns_nothing_and_keeps_the_forward` (F4.1–F4.5 + bare-first control), `cjs_nested_bare_var_and_top_level_function_of_the_same_name_keep_the_forward` (F4.6), `cjs_initialized_var_redeclaration_and_block_function_rebind` (F4.7 + block function + ES-module control); mutation (restore `declared_names` in `Table::build`) fails the first; restored
        - docs: `docs/reexports.md` JS/TS shadowing paragraph + example; skill `analysis-pipeline.md` CommonJS paragraph
- work completed for 'R2-F4 — Bare variable redeclarations erase proven CommonJS forwards' at 11:16:41
- starting the work on 'R2-D1 — Later CommonJS interface writes leave earlier forwards complete' at 11:16:41
        - defect class: a later write to the interface overrides an earlier forward, but the earlier forward is still reported as the file's interface
        - baseline probe (CLI `--json-full --no-cache`, 20 shapes): every later-write shape complete with the stale entry (`exports.x` then `exports.x`, `module.exports.x` then `exports.x`, `module.exports = require` then `= {}` / `.y = 1`, `exports.x` then `module.exports = {..}`, guarded/function/computed/merge/augmented/`delete` later writes, `module.exports = {}` then `exports.x = require(..).x`, TS `export = require` then `module.exports = {}`, `module.exports = require("b"); export = require("a")` reported both)
        - fixed with one post-pass, `cjs::Interface` (`ecma/cjs.rs`): collects every interface write (assignment, augmented, update, `delete`, `Object.assign` merge, TS `export =`) through unshadowed `module`/`exports`, numbers member writes by the epoch of definite `module.exports =` writes, and `settle`s each `declared` entry by its innermost write: later definite same-member or whole write → dropped; `exports.` write after a definite whole write → dropped (detached); guarded/function-held/augmented/computed/merge later write, an uncertain earlier whole write under an `exports.` forward, or a member write onto a forwarded whole module → `rebound_binding`, entry withheld. `merge_root` shared with `merge_call`; `ecma/mod.rs` scan also collects `unary_expression`
        - after-fix probe: all 26 shapes match the independently stated expectations; earlier plain value then later forward reports the later forward; write of another member and `exports.y` after a whole forward keep the forward complete; CLI cache miss then hit identical
        - existing tests adjusted (audit samples untouched): `pos-cjs-destructure.js` combines two `module.exports =` writes and a later `exports.d`, so only `b`/`c` survive (separate inline snippet keeps `{ a }` + `module.exports.d = a` coverage); `amb-cjs-conditional.js` line 1 guarded forward is replaced by line 2, checked alone; the block-local variant now writes `module.exports.d = a`
        - consumers: dropped entry leaves the ES import marker and `is_reexport` false; no unused-import warning in either case because `exports.x = a` reads `a` (lint references, not the exemption)
        - tests: `cjs_later_definite_write_replaces_the_earlier_forward`, `cjs_later_write_that_may_replace_or_mutates_the_forward_withholds_it`, `cjs_later_write_of_another_member_or_through_detached_exports_keeps_the_forward`, `ts_export_equals_and_module_exports_replace_each_other`, `cjs_es_import_forwarded_then_replaced_leaves_the_import_unmarked` (symbols_reexports_ecma.rs, both extensions), `an_ecma_import_forwarded_through_a_replaced_commonjs_member_is_not_flagged` (reexports_consumers.rs); all under 0.1 s; mutation (skip `settle`) fails 8 tests; restored
        - not modeled (documented): `Object.defineProperty(exports, ..)`
        - docs: `docs/reexports.md` JS/TS later-write paragraph, table and example; skill `analysis-pipeline.md` CommonJS paragraph; `cjs.rs` module doc
        - checks: `just test reexport` 433 passed (7.1 s); `just test ecma` 87 passed; `just test cjs` 27 passed; `just test golden` 19 passed; `just lint` clean; no `EXTRACTOR_REVISION` bump (ships under 55)
        - R2-F4 and R2-D1 implementation complete, ready for review
- work completed for 'R2-D1 — Later CommonJS interface writes leave earlier forwards complete' at 11:23:01
- reconciliation started at 11:23:50
        - independent public CLI probes (`hug symbols --json-full --no-cache`), observed = expected for each:
                - R2-F1.1 guarded append: `limited [computed_export_list]`, no entry, marker false
                - R2-F1.2 guarded remove: `limited [computed_export_list]`, no entry
                - unguarded append control: complete, `named/declared read`, marker true
                - R2-F3.1 `(M).foo = 1` and R2-F3.4 constructor + `(M)["foo"] = 1`: complete, no `foo`
                - R2-F4.1 `var m = require("m"); var m; module.exports = m`: complete `module` from `m`
                - R2-F4.4 (.ts) destructured + bare `var read;`: complete `named read`
                - R2-D1 `exports.x` overwrite and `module.exports = {}` overwrite: complete, no stale entry
        - full `just test` (tree-hugger/): 2525 passed, 0 skipped, 0 failed (125.3 s; re-run 173.6 s under heavy host load, 34 slow)
                - every test over 5 s predates this feature (checked with `git log -S` against base `1cb5837ed`; names in the feature diff appear only in spec/plan prose)
                - slowest feature-added test: `cache_upgrade::entry_from_extractor_revision_fifty_one_is_not_served_after_the_reexports_bump` 4.324 s; generated cache matrix rows ≤ 2.805 s
        - `just lint` (tree-hugger/): exit 0; root `just check-tier-coverage tree-hugger`: 0 stranded
        - `docs/reexports.md` and `docs/cache-design/robustness.md` contain no feature/spec references
        - no cross-OS run: the changes are pure syntax-tree logic and test scheduling, with no path, process, or `cfg(os)` code; CI covers Linux/macOS on the PR
- reconciliation completed at 11:30:33

### Completion Report

- **outcome: complete**: every review finding is repaired and verified, and all required checks pass
- elapsed: 10:45:54 → 11:30:33 (about 44 min)
- parent findings (review-2): total 4 = fixed 4 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0 + disputed 0
        - R2-F1 (high) fixed: guarded `__all__` append/remove are now a `computed_export_list` limit; also fixed two Python siblings (`__all__` calls nested in expressions; edits in function/class/lambda/comprehension bodies)
        - R2-F2 (high) fixed: the five re-export cache matrices are now one generated `#[test]` per row, with a guard test; no thread fan-out; all rows under 3 s in the full suite
        - R2-F3 (medium) fixed: shared Lua target normalization, so parenthesized writes fold or limit; siblings R2-F3.7–R2-F3.13 fixed
        - R2-F4 (medium) fixed: only initialized redeclarations replace a CommonJS binding; block-function and `export var` siblings fixed; comment corrected
- discovered parent (outside the review's list, same class as R1-F2/R2-F3): R2-D1 fixed: later CommonJS/TS interface writes drop or limit the stale earlier forward (children R2-D1.1–R2-D1.16)
- instance rows: 0 open; every child is fixed or clean with evidence in the ledger
- nonblocking observation R2-O1 **deferred**: fixtures embed active feature-directory samples; next action: the author updates the `include_str!` paths when moving the feature to `_completed`
- no performance measurement was required or deferred
- extraction changes ship under `EXTRACTOR_REVISION` "55" (one bump for the cycle) with the revision-54 stale-cache row
- changed files (this cycle):
        - `tree-hugger/lib/src/analysis/reexports/python.rs` (R2-F1)
        - `tree-hugger/lib/src/analysis/reexports/lua.rs` (R2-F3)
        - `tree-hugger/lib/src/analysis/reexports/ecma/bindings.rs`, `ecma/cjs.rs`, `ecma/mod.rs` (R2-F4, R2-D1)
        - `tree-hugger/lib/src/file/mod.rs` (revision 55)
        - `tree-hugger/lib/tests/l1/symbols_reexports_python_perl.rs`, `reexports_consumers.rs` (R2-F1, R2-D1)
        - `tree-hugger/lib/tests/l1/symbols_reexports_go_lua.rs` (R2-F3)
        - `tree-hugger/lib/tests/l1/symbols_reexports_ecma.rs` (R2-F4, R2-D1)
        - `tree-hugger/cli/tests/symbols/cache_robustness.rs` (R2-F2)
        - `tree-hugger/cli/tests/symbols/cache_upgrade.rs`, `cli/tests/fixtures/golden/symbols.json`, `symbols.jsonl` (R2-F1.9)
        - `tree-hugger/docs/reexports.md` (R2-F1, R2-F3, R2-F4, R2-D1), `tree-hugger/docs/cache-design/robustness.md` (R2-F2)
        - `.claude/skills/tree-hugger/analysis-pipeline.md`, `.claude/skills/tree-hugger/testing.md`
- final verification:
        - `just test` (tree-hugger/): 2525/2525 passed → all findings
        - `just test reexport` 433 passed; `just test lua` 95; `just test ecma` 87; `just test cjs` 27; `just test cache_robustness` 95; `just test cache_upgrade` 57; `just test golden` 19 → R2-F1, R2-F3, R2-F4, R2-D1, R2-F2
        - `just lint` exit 0; `just check-tier-coverage tree-hugger` 0 stranded → R2-F2 (generated tests are scheduled in L1)
        - orchestrator CLI probes of the review's reproductions → R2-F1.1, R2-F1.2, R2-F3.1, R2-F3.4, R2-F4.1, R2-F4.4, R2-D1
- implementation complete, ready for review
