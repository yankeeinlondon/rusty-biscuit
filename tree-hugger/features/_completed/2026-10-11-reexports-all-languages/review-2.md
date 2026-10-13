---
$schema: feature-review.yaml
ready: false
findings:
  - "R2-F1 — high: Conditional Python export-list edits become unconditional evidence"
  - "R2-F2 — high: Re-export cache tests exceed the Level 1 time budget"
observations:
  - Regression fixtures still depend on the active feature directory
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-11T10:35:50-07:00"
spec: 2026-10-11-reexports-all-languages/spec.md
implemented: true
description: "A **feature** review of `2026-10-11-reexports-all-languages/spec.md`"
feature: 2026-10-11-reexports-all-languages/review-2.md
previous: 2026-10-11-reexports-all-languages/review-1.md
next: 2026-10-11-reexports-all-languages/review-3.md
implemented_by: claude/opus
log: tree-hugger/features/2026-10-11-reexports-all-languages/log.md
---

# Review 2: Re-exports across all supported languages

The feature is **not ready**. Conditional Python export-list edits produce false complete reports, and two feature-added cache tests exceed the required five-second Level 1 budget. Two additional findings concern unusual spellings and are medium priority; they do not block readiness. No human decision or environmental prerequisite blocks the repairs.

All 18 enumerated instances from review 1 pass. That does not close their entire defect classes: the wider sweep found a remaining Lua write spelling and an ECMAScript regression introduced by the repair. The original repair cases and these new siblings are accounted for separately below.

## Previous Finding Verification

Reviewed the previous review's body, frontmatter, child-instance tables and repair requirements against both `implementation-log.md` and the repair ledger in `log.md`. There are no earlier implementation-review files in this directory. No finding was disputed. The author accepted the support contracts before implementation; C++ module syntax remains a reported grammar gap by that accepted decision. No finding in review 1 required removing another blocker, and none was newly unblocked by a changed prerequisite.

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
| --- | --- | --- | --- | --- |
| R1-F1 — Same-file binding checks confuse replaced or shadowed names with imported bindings | R1-F1.1–R1-F1.8; report, import markers, direct imports, diagnostics, queries, filtering, cache and CLI | regressed | All eight original children pass the focused suite and public probes. New repeated-`var` handling adds the false rejection in R2-F4; the earlier implementation had no repeated-declaration rejection pass. | R2-F4 is a new medium repair regression, not an unchanged failing reproduction. |
| R1-F1.1 — Python replacement | Sequential named and namespace imports; preserve exclusive fallback imports; overwritten import marker false and unused warning retained | verified fixed | `python_a_later_named_import_of_the_same_name_replaces_the_earlier_one`, `python_sequential_namespace_aliases_forward_only_the_final_module`, `python_fallback_imports_in_exclusive_branches_both_stay_conditional`, and `a_replaced_python_import_keeps_its_unused_import_warning` pass. CLI reproduction reports only `new`, markers `[false,true]`, and an unused warning for `old`. | None for this child. |
| R1-F1.2 — JS/TS whole-module shadowing | Block local must not forward outer required module; unaffected function-local control must retain it | verified fixed | `cjs_whole_module_binding_hidden_by_a_block_local_is_not_forwarded` passes for both extensions; public CLI probes produce limited coverage and no entry. | None. |
| R1-F1.3 — JS/TS property consumers | Namespace, destructured, block, catch, loop and nested-function paths | verified fixed | `cjs_property_forward_of_a_binding_hidden_by_a_nearer_declaration_is_not_forwarded` checks every listed path in both extensions, including unaffected positive controls; public property probes agree. | None. |
| R1-F1.4 — JS/TS object consumers | Shorthand and explicit property values; retain unaffected fixture sites | verified fixed | `cjs_object_literal_member_hidden_by_a_block_local_is_withheld_and_other_sites_stay` passes with both fixture edits; public object probes withhold the hidden binding. | None. |
| R1-F1.5 — ES import consumed by CommonJS | Both extensions; report link, stored marker and direct import flag false | verified fixed | `cjs_consumer_of_an_es_import_hidden_by_a_block_local_leaves_the_import_unmarked` passes; public API example compares direct import flags with all query import markers. | None. |
| R1-F1.6 — Replaced globals | `require`, `module`, `exports`; direct/bound CommonJS and TS `export =`/import-require consumers | verified fixed | `cjs_reassigned_loader_and_interface_globals_are_not_proven` and `ts_export_equals_after_require_is_reassigned_is_not_proven` pass, including unaffected-global controls; CLI probes agree. | None. |
| R1-F1.7 — Scala import links | Member hides import; other-scope import inaccessible; nearest enclosing import selected | verified fixed | Both Scala repair tests pass. Public hidden-member and separate-object probes preserve the written export while leaving unrelated imports unmarked; query/direct import markers agree. | None. |
| R1-F1.8 — Lua unrelated locals | Whole-module, constructor and assembled-table returns; unrelated local assignment versus actual chunk-binding write | verified fixed | Three `lua_write_to_a_function_local_keeps_*` tests and the scope controls pass. Three public snippets preserve complete module/field results across queries and cache. | None. |
| R1-F2 — Lua reports a definite interface after the returned table can be modified | R1-F2.1–R1-F2.5; classify disqualifying uses of returned table; direct/query/cache/CLI reports | partially fixed | All enumerated children pass. New `table_use`/`indexed_use` accepts parenthesized chunk-level targets as handled, but `assign` does not actually fold them into fields; R2-F3 reproduces stale entries. Python list edits in R2-F1 are a newly discovered sibling of conditional-interface certainty, outside the earlier enumerated Lua cases. | R2-F3 remains medium; R2-F1 is separately high. No blocker. |
| R1-F2.1 — Conditional field write | Withhold affected field; retain untouched fields | verified fixed | `lua_conditional_field_write_withholds_the_field` and `lua_conditional_write_keeps_the_fields_it_does_not_name` pass; copied-shape CLI probe is limited and empty for the affected field. | None. |
| R1-F2.2 — Function-body field write | Called and uncalled functions | verified fixed | `lua_field_write_in_a_function_body_withholds_the_field` passes both variants; public function/call probe withholds the field. | None. |
| R1-F2.3 — Table argument escape | Assembled table passed to ordinary call; alias/storage siblings | verified fixed | `lua_returned_table_passed_to_a_call_is_limited` and `lua_returned_table_stored_elsewhere_is_limited` pass; public call probe is limited/empty. | None. |
| R1-F2.4 — Constructor-table escape | Constructor-initialized local table passed to call; positive unescaped control | verified fixed | `lua_constructor_initialized_table_passed_to_a_call_is_limited` passes; public constructor/call probe is limited/empty. | None. |
| R1-F2.5 — Unknown later key | Unknown field may replace every named field | verified fixed | `lua_unknown_key_write_withholds_every_field` passes; public unknown-key assignment withholds every field. | None. |
| R1-F3 — Lua constructor keys produce incorrect exported names and miss overwrites | R1-F3.1–R1-F3.5; direct/local constructors, assignment/member key identity and downstream reports | verified fixed | All children below pass; shared key interpretation covers constructors, assignments, member reads and metatable keys. Direct/selective/complete/filtered/cache/CLI probes preserve the corrected reports. | None for the required class. Escaped literals remain the documented bounded limitation permitted by review 1. |
| R1-F3.1 — Direct quoted key | Four quote styles produce `x` | verified fixed | `lua_string_literal_constructor_key_names_the_field` passes all quoting styles; public `["x"]` control names `x`. | None. |
| R1-F3.2 — Local constructor quoted key | Local returned table names `read` | verified fixed | `lua_string_literal_key_in_a_local_table_constructor_names_the_field` and public local-constructor probe pass. | None. |
| R1-F3.3 — Computed constructor key | Both returned constructor consumers; unreturned-table control | verified fixed | Both computed-key tests pass; public computed constructor is limited, with no invented key name. | None. |
| R1-F3.4 — Equivalent/repeated keys | Identifier and quoted keys collide; assignment spellings share identity | verified fixed | `lua_equal_constructor_keys_share_one_field` and `lua_assignment_keys_share_one_field_however_spelled` pass. Import-involved repeated constructor keys are withheld, consistent with the repair log's explanation that constructor assignment order cannot prove a winner. | None. |
| R1-F3.5 — Non-string keys | Numeric and boolean keys emit no named forward | verified fixed | `lua_number_and_boolean_keys_name_no_field` passes; public numeric-key constructor emits no entry and remains complete. | None. |
| review-1.md — CLI help still promises re-exports in declaration filters | Both declaration/class `--exported` help strings | verified fixed | Both comments in tree-hugger-cli `cli/src/main.rs` now describe declarations and synthetic Rust prelude records; they match the spec's CLI contract. | None. |
| review-1.md — Regression fixtures are coupled to the active feature directory | Every new audit-sample `include_str!` consumer | unresolved | Tests still embed active-directory samples. The ledger explicitly defers moving them to the author's lifecycle step. Tests compile and run today. | Nonblocking observation retained below; no requested repair or readiness blocker. |

## Verification

Read the finalized spec, plan, both implementation logs, audit contracts, repair diffs, extractors, consumers, query projection, strict cache validation and tests. Schema remains 5.2; extractor revision is 54. The revision-53 cache-upgrade test runs and passes.

Checks from `tree-hugger/`:

- `just test reexport`: 339 passed, 2,090 excluded; 7.449 seconds. The complete consistency test took 5.082 seconds.
- `just test`: 2,429 passed, zero skipped; 122.627 seconds, 23 slow tests. The feature-specific budget violations are recorded in R2-F2; nextest's passing status does not override the review's five-second rule.
- `just lint`: passed for both packages.
- Root `just check-tier-coverage tree-hugger`: zero stranded tests.

A temporary probe copied the reported source shapes into isolated files and checked independently stated expected names/coverage. Forty-seven files were run through shipped CLI full-index output uncached and in two cached passes. A temporary Rust example then checked those same files through `TreeFile::reexports()`, direct import flags, complete/selective/function-filtered queries, cache widening and confirmed cache hits. It found identical reports across those surfaces and zero syntax diagnostics. Additional focused CLI controls checked Perl list consumers and the nearby negatives below. The example was removed; no production code or tests were edited.

CLI reproduction for each snippet below:

```sh
cargo run --color=never -q -p tree-hugger-cli --   symbols --file /absolute/path/to/sample.py --json-full --no-cache
```

Use the corresponding file extension for Lua, JavaScript or TypeScript. Lua snippets deliberately use semicolons: without one, a following parenthesized expression can be parsed as a continuation of the previous call expression rather than the intended assignment.

| User-facing requirement | Appropriate level | Evidence and limits |
| --- | --- | --- |
| All 16 language contracts, names/forms/qualifiers/scopes, negatives and parse recovery | L1 public API with pinned grammar fixtures | Per-language tests run. Original repair cases pass; new extractor failures below remain. |
| Report and import-marker agreement across API, query, filtering, cache and CLI | L1 public API plus hermetic CLI/cache | Equivalence tests and 47 public probes agree; agreement also preserves the newly reproduced wrong results. |
| Import exemptions; Rust/ES handling and prelude behavior | L1 diagnostics and CLI | Consumer/prelude tests pass. Conditional Python append incorrectly suppresses an unused warning. |
| Stale/damaged cache rejection and strict JSON fields | L1 persisted-entry edits through public CLI | Matrix and upgrade tests pass functionally; two added tests exceed the L1 budget. |
| No target-file reads or unrequested expensive passes | L1 work counters and absent-target fixtures | Existing work-count and absent-target tests pass. |
| Full-index JSON; declaration-only normal listings | L1 shipped CLI | Wiring, import conversion and full-index tests pass. No new styled view or keyboard behavior requires L2/L3. |

### Input Robustness Matrix

Applicable only to persisted cache JSON, not to the source-language grammars. Checked the plan's concrete columns against the strict reader and `reexport_columns`/span/limited-report matrices. Report, coverage, state, limits, entries, form/evidence, nullable and required names/paths/scopes/restrictions, exclusions/caveats, flags, linked/null import spans, coordinates, import markers and completed capabilities have defined controls and absent/null/wrong-type/element/empty/duplicate/trailing outcomes. Functional refusal and recomputation tests pass, including well-typed inconsistencies. No new reader correctness defect emerged. R2-F2 concerns how those tests are scheduled and executed, not weakening their matrix.

## Unblocked Findings

### R2-F1 — high: Conditional Python export-list edits become unconditional evidence

**Authority — spec:** the design requires “Report conditional syntax without evaluating build conditions or presenting the resulting interface as unconditional.” Its accepted Python audit explicitly makes conditional `__all__` a coverage limit and says branch assignments are not merged. The diagnostic contract says a “guessed public surface must not suppress a warning.”

**Defect class:** mutable interface-list updates lose their conditional execution when folded into final membership.

In tree-hugger, [all_method_call](../../lib/src/analysis/reexports/python.rs:375) passes `conditional` to `.extend()` but directly records literal `.append()` and `.remove()` operations without it. The final membership calculation then treats those edits as definite.

```python
from m import read
__all__ = []
if enabled:
    __all__.append("read")
```

This parses cleanly and reports complete coverage, an unconditional declared `read` entry, `reexported: true`, and no unused-import warning. The contract requires limited/computed-export-list coverage and no unconditional membership derived from this edit. Starting with `__all__ = ["read"]` and conditionally removing `read` instead produces a false complete negative. Feature flags and optional exports are ordinary inputs, so this is high.

| Child ID / site | Shape tested | Observed result | Expected result | Evidence |
| --- | --- | --- | --- | --- |
| R2-F1.1 — Python append | Empty list; `if enabled: __all__.append("read")` | Complete, unconditional declared entry, marker true | Limited; no unconditional list-derived forward or exemption | Public CLI/API/query/cache probe; unguarded append control correctly forwards. |
| R2-F1.2 — Python remove | Initial `["read"]`; conditional `.remove("read")` | Complete, empty report, marker false | Limited; removal is not proof of absence | Public CLI/API/query/cache probe; unguarded removal control correctly removes. |
| R2-F1.3 — Python extend | Empty list; conditional `.extend(["read"])` | Limited/computed-export-list; no entry | Same | Public probe; `apply_all` checks `conditional`. Clean. |
| R2-F1.4 — Python assignment / concatenation | Conditional literal assignment | Limited/computed-export-list; no entry | Same, regardless of literal expression shape | Public assignment probe; all literal assignment/concatenation reads pass through `apply_all` before membership. Clean by that invariant. |
| R2-F1.5 — Python augmented assignment | Conditional `+= ["read"]` | Limited/computed-export-list; no entry | Same | Public probe; same conditional gate. Clean. |
| R2-F1.6 — Perl list, tag and push/unshift consumers | Guarded literal `@EXPORT_OK`, `%EXPORT_TAGS`, `push` and `unshift` | Entries carry `conditional: true` | Written export-list sites must not claim unconditional exposure | Public guarded assignment/tag/push/unshift probes; `ExportItem.conditional` survives `resolve`. Clean for this class. |
| R2-F1.7 — Lua returned-table conditional mutation | Guarded field write after imported field assignment | Limited/rebound; affected entry withheld | Same under the accepted Lua safety rule | Prior repair test and public probe. Clean for ordinary target spelling; separate unusual spelling in R2-F3. |
| R2-F1.8 — JS/TS conditional forwarding | Guarded CommonJS assignment | Entry carries `conditional: true` | Preserve written site's condition | `cjs_conditional_dynamic_and_merge_forms_are_limited_and_the_guarded_one_is_conditional` and guarded try/ternary tests pass for the supported mechanism. Clean. |

The other extractors report dedicated syntax or initial value copies rather than folded mutable export-list edits; no equivalent list-update decision exists there.

**Consumer trace.** Conditionality is lost at collection, then affects `declared_all_names`, candidate membership and `build`. Import marking accepts the resulting declared link; direct imports and CLI conversion preserve it, and the unused-import exemption accepts it. Whole-file query projection and cache validation preserve the internally consistent report. Public probes confirm the error through every listed surface. Declaration listings and Rust prelude selection remain separate and need no change.

**Required repair:** apply the existing conditional-list gate to append and remove, without evaluating conditions. Add positive unconditional controls and conditional negatives through the public report and marker/diagnostic consumer. Keep independent redundant-alias/star evidence under its existing contract. Bump extraction revision and retain the matching stale-cache row.

### R2-F2 — high: Re-export cache tests exceed the Level 1 time budget

**Authority — spec:** acceptance requires running `just test` and focused L1 coverage for cache rejection. This review's explicit test contract states: “An L1 test over 5 s counts as failing,” and prohibits thread fan-out inside a test and matrices of process spawns inside one test. The two tests below were added by this feature, so they are inside the review boundary.

**Defect class:** cache-validation matrices multiply subprocess work inside a single scheduled test, exceeding its execution budget under normal suite load.

In tree-hugger-cli, [check_reexport_columns](../../cli/tests/symbols/cache_robustness.rs:2634) spawns one thread per field column. Each column runs CLI/cache cells while nextest is also running other tests. [check_inconsistencies](../../cli/tests/symbols/cache_robustness.rs:2690) instead runs all cases serially inside one test. The original implementation log already noted the feature's slow cache matrices; inherited use of the same pattern elsewhere is not an exception for newly added tests.

| Child ID / site | Shape tested | Observed result | Expected result | Evidence |
| --- | --- | --- | --- | --- |
| R2-F2.1 — Complete report field matrix | All field/shape cells through CLI | 13.447 s full suite; 4.254 s focused | Every scheduled L1 test stays within 5 s under suite load | `just test`, `just test reexport`; per-column thread fan-out in helper. |
| R2-F2.2 — Complete well-typed inconsistency matrix | Fifteen edits plus setup/control CLI runs | 9.320 s full suite; 5.082 s focused | Within 5 s without dropping cases | Same runs; serial loop in helper. |
| R2-F2.3 — Limited field matrix | Limit and span cells through same thread helper | 1.119 s full suite; 2.170 s focused | Within 5 s | Clean in measured runs; preserve cells when changing shared scheduling. |
| R2-F2.4 — Limited inconsistency matrix | Four edited reports through serial helper | 2.800 s full suite; 0.755 s focused | Within 5 s | Clean in measured runs. |
| R2-F2.5 — Not-applicable inconsistency matrix | Two language/state edits | 0.319 s full suite | Within 5 s | Clean in measured run. |
| R2-F2.6 — Widening, nullable-name and duplicate/trailing consumers | Existing feature-added individual CLI tests | 2.212 s, 1.176 s and 1.283 s full suite | Within 5 s | Clean in measured run. |

Swept the feature's extraction, equivalence, consumer, cache-upgrade and cache-rejection tests; only R2-F2.1 and R2-F2.2 exceeded the budget. Older slow cache/reference/report tests are outside this feature and are not required repairs.

**Required repair:** let nextest schedule the expensive matrix rows separately, or reduce their public CLI work enough to meet the budget. Preserve the shared matrix and add a guard against untested columns/cases when generating separately named tests. Keep every original refusal reason, positive control and public output assertion. Do not add retries, exclusions, a slow-tier label or a larger timeout. Verify the result in the package's full L1 run; isolated passing timings alone do not establish closure.

### R2-F3 — medium: Parenthesized Lua table writes leave stale exported fields

**Authority — spec:** the accepted Lua facts require the last field write to be import-derived; “a later `M.field = x` overrides.” The report must name the interface that those bounded facts establish. Parentheses preserve the target table rather than making a different binding.

**Defect class:** mutation classification accepts a target spelling as already folded into the interface, while the field collector does not handle that spelling.

```lua
local m = require("m");
local M = {};
M.foo = m.foo;
(M).foo = 1;
return M
```

The result is complete with the stale named `foo` forward. In tree-hugger, [table_use](../../lib/src/analysis/reexports/lua.rs:772) unwraps parentheses and [indexed_use](../../lib/src/analysis/reexports/lua.rs:806) exempts chunk-level named writes; [field_target](../../lib/src/analysis/reexports/lua.rs:582) requires a bare identifier, so `assign` never records the replacement. The spelling is unusual enough to be medium, not a blocker.

| Child ID / site | Shape tested | Observed result | Expected result | Evidence |
| --- | --- | --- | --- | --- |
| R2-F3.1 — Assembled table / dot target | `(M).foo = 1` | Complete, stale `foo` | Complete with no `foo`, or limited with affected field withheld | Public API/query/cache/CLI probe. |
| R2-F3.2 — Assembled table / bracket target | `(M)["foo"] = 1` | Same stale forward | Same | Same public surfaces. |
| R2-F3.3 — Constructor table / dot target | `local M={foo=m.foo}; (M).foo=1` | Same stale forward | Same | Same public surfaces. |
| R2-F3.4 — Constructor table / bracket target | `local M={foo=m.foo}; (M)["foo"]=1` | Same stale forward | Same | Same public surfaces. |
| R2-F3.5 — Nested-block/function mutation | Parenthesized write inside `if` or function | Limited/rebound, no affected entry | Same | Public CLI controls; mutation classification correctly withholds nested writes. Clean. |
| R2-F3.6 — Ordinary target and direct return | Bare `M.foo=1`; direct `return {foo=m.foo}` | Complete empty; complete `foo`, respectively | Same | Public controls. Clean. |

These errors survive report assembly, kind projection and cache hits. Lua has no import records, so import exemptions are not a consumer. A small consistent target normalization or a limited fallback is sufficient; do not add general expression evaluation. If maintaining another accepted spelling is disproportionate, document this narrow limitation and conservatively withhold its affected field.

### R2-F4 — medium: Bare variable redeclarations erase proven CommonJS forwards

**Authority — spec:** the JavaScript/TypeScript contract includes CommonJS forwarding through imported bindings and requires every agreed static construct to remain extractable. “Reassigned” is the loss-of-proof condition; a bare repeated `var m;` performs no assignment. This is also a repair regression: the review-1 implementation had no duplicate-`var` rejection loop, and the repair diff adds it.

**Defect class:** a binding declaration is classified as a value replacement even when it has no initializer.

```javascript
var m = require("m");
var m;
module.exports = m;
```

In tree-hugger, [Table::build](../../lib/src/analysis/reexports/ecma/bindings.rs:65) adds every repeated module-scope `var` name to `rebound`, regardless of whether it writes a value. Reports become limited/rebound and lose a proven forward. This is medium because redundant module-scope declarations are an unusual authoring pattern.

| Child ID / site | Shape tested | Observed result | Expected result | Evidence |
| --- | --- | --- | --- | --- |
| R2-F4.1 — JS/TS whole module | Required namespace followed by bare `var m;`, then `module.exports=m` | Limited, no entry | Complete module forward from `m` | Both extensions, public API/query/cache/CLI. |
| R2-F4.2 — JS/TS member | Same prefix, `exports.read=m.read` | Limited, no entry | Complete named `read` forward | Same surfaces. |
| R2-F4.3 — JS/TS object member | Same prefix, `module.exports={m}` | Limited, no entry | Complete namespace entry `m` | Same surfaces. |
| R2-F4.4 — JS/TS destructured member | `var {read}=require("m"); var read; exports.read=read` | Limited, no entry | Complete named forward | Same surfaces. |
| R2-F4.5 — JS/TS destructured object | Same prefix, `module.exports={read}` | Limited, no entry | Complete named forward | Same surfaces. |
| R2-F4.6 — Block/function bare declaration | Bare `var m` in a block or unrelated function | Original complete module forward retained | Same | Public controls; nested initializer check/function scope prevents false writes. Clean. |
| R2-F4.7 — Actual replacement | Repeated initialized `var m={}` | Limited, no entry | Same bounded rebinding result | Public control and repair test. Clean. |
| R2-F4.8 — ES/TS syntax consumers | Named/default exports and `export =`/`export import` resolve only ES/import-require entries through `resolve_es` | CJS `var` bindings are not candidates | No new requirement to extend those consumers to CJS variables | Precise invariant: these readers use `resolve_es`, while the added duplicate-var pass affects CommonJS `resolve_any`. |

The mistaken `rebound` decision passes through `resolve_any`/`resolve_at` to every CommonJS whole/property/object consumer, then removes entries before marker generation and serialization. Kind projection and cache hits retain the lost result. Required-module locals have no ES `ImportRecord`, so this instance does not create a direct-import exemption. Declaration/prelude behavior stays separate.

A small initializer check should preserve the valid bare-redeclaration control while retaining true-write negatives. Correct the new comment that says every second `var` replaces the value; it currently restates the incorrect decision.

## Blocked Findings

None. Accepted language support decisions remain accepted; no human review is needed for these findings.

## Recurrence

- **R1-F1 in review-1.md, binding replacement and shadowing:** R2-F4 is a **repair regression**, not an unchanged instance. The new duplicate-declaration pass confuses declaration with assignment; missed consumers are CommonJS whole, member, namespace-object and destructured-object forwards in both extensions. All original R1-F1 children now pass.
- **R1-F2 in review-1.md, mutable interfaces reported with unwarranted certainty:** R2-F3 is a **newly discovered sibling** outside the earlier enumerated target spellings. Collection and the new mutation classifier disagree about parenthesized chunk-level writes. R2-F1 is a **newly discovered sibling mechanism** in conditional Python list folding; review 1 explicitly inspected Python list edits separately but did not enumerate conditional append/remove. Its lost condition propagates into membership, import markers and diagnostic exemption. All original five Lua cases now pass.
- R2-F2 is a newly reported test-execution defect, not a recurrence of the three extraction findings. R1-F3's required key cases are closed.

The medium spellings do not justify building a general interpreter or an expanding protection layer. Their bounded collector decisions are the complete repair inventory carried here; no stronger runtime guarantee is requested.

## Observations

### Regression fixtures still depend on the active feature directory

The previous observation remains: per-language L1 tests embed static audit samples from the active feature directory. They currently compile and run, so this does not block readiness. The repair ledger deliberately leaves relocation to the author's lifecycle move; preserve the fixtures and update their embedding paths at that step. No agent should move the feature as part of this cycle.
