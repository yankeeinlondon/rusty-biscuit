---
$schema: feature-review.yaml
ready: false
findings:
  - title: R8-F1 — Comments corrupt import identities, qualifiers and binding exclusions
    priority: high
  - title: R8-F2 — Scala import collection omits rename and operator forms
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-07T12:27:52-07:00
spec: 2026-10-08-scoped-references/spec.md
implemented: true
implemented_by: codex/gpt-6.1-sol
log: tree-hugger/features/2026-10-08-scoped-references/log.md
description: "A **feature** review of `2026-10-08-scoped-references/spec.md`"
feature: 2026-10-08-scoped-references/review-8.md
previous: 2026-10-08-scoped-references/review-7.md
next: 2026-10-08-scoped-references/review-9.md
---

# Scoped References: review 8

The feature is **not production ready**. The enumerated review-7 repairs are verified fixed, including their added sibling inventories and downstream consumers. Two newly swept classes remain. First, comments in TypeScript and Scala imports are treated as meaningful syntax children. TypeScript can incorrectly prove that a function is not recursive; Scala can lose imports, truncate written qualifiers and suggest an imported token as a reference to an unrelated declaration. Second, Scala import collection omits unbraced renames and operator-name forms even without comments. No human decision is needed.

The specification, plan, both implementation logs, every historical finding inventory, review-7 frontmatter/body/consumer tables and its 164-child repair ledger were reconciled. The current repair pass changes regression tests; the production repair already exists in commit `580342c73`. Passing the earlier cases closes those instances, not every source shape handled by the same readers.

## Previous Finding Verification

The table includes every earlier finding, including the legacy reviews with one Findings section. Its historical child ranges remain unchanged. Evidence labeled review-8 L1 was independently rerun in the **1,462-test** area run. Tests were inspected for expected absences and independently stated required reads, rather than cached/uncached agreement alone. The added review-7 rows include the ledger's new sibling and consumer inventories. Independent probes mentioned in inherited historical rows are prior-review evidence unless explicitly identified as review-8; their permanent regression tests were rerun here. The review-7 verification rows and new finding probes are current independent evidence.

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
|---|---|---|---|---|
| `review-1.md — Lexical lookup crosses boundaries that do not establish a binding` | Rust inline modules, nested functions/constants/statics, closure controls, conditional chains/else/following statements and loop controls; TS/JS `with`, direct eval and defaults; occurrences, summaries, recursion, relation lookup and CLI | **verified fixed** | Review-8 L1: `LEXICAL_BOUNDARY_MATRIX`, boundary summaries/recursion, shared relation lookup and boundary CLI tests pass. Lookup enforces module/item boundaries and conditional endpoints. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-1.md — Named function expressions bind their names to outer declarations` | TS/JS ordinary and generator expression names, defaults/internal calls, final outer calls, unnamed/class/arrow controls; exclusions, placement, summaries, recursion, relation lookup and callers | **verified fixed** | Review-8 L1: `EXPRESSION_NAME_MATRIX`, expression-local summaries/recursion/relation and CLI callers pass. The shared binding inventory excludes expression names and confines visibility to the expression. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-1.md — Invocation classification loses calls across supported source forms` | Direct/wrapped calls in all 16 languages, TS type-only wrappers, member/generic wrappers, Perl/shell calls, receiver/argument/sequence/conditional/dereference negatives; reports, recursion, terminal-call and CommonJS readers | **verified fixed** | Review-8 L1: all-language invocation/negative matrices, wrapped binding, CLI, terminal-call and CommonJS tests pass. R6-F1 is outside the earlier enumerated wrappers. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-1.md — Cache validation accepts inconsistent reference evidence` | Wrong/malformed/lost retained target IDs, named/ownerless targets, owner/candidate identity, duplicate/omitted callable regions, structural matrix, summaries/dependencies/semantics, null and narrowed controls | **verified fixed** | Review-8 L1: structural/reference matrix, retained-evidence tests and two-way summary/semantic validation pass; strict stored types require keys and distinguish nullable values. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-2.md — Parameter defaults incorrectly see body-local var bindings` | Seven callable forms in TS/JS; ordinary/block/destructured/loop `var`; parameter visibility, redeclaration, hoisting, computed/destructured defaults and body eval; let/const/function controls, relation lookup and reports | **verified fixed** | Review-8 L1: both languages’ callable-form/body-declaration matrix, parameter-environment controls, relation lookup and CLI tests pass. `var_body` isolates body bindings from defaults. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-2.md — Pattern capitalization invents or omits Rust bindings` | Bare/modified capitalized bindings, lowercase constants in patterns/body/guards, paths/tuples/shorthands and irrefutable controls; walker, capture fallback, exclusions, declaration records, summaries, recursion and callers | **verified fixed** | Review-8 L1: pattern identity/record, summaries, recursion and CLI rows pass. Query repairs also remove Rust path/range captures; actual binding identity precedes record exclusion. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-2.md — Transparent call wrappers incorrectly limit coverage` | Rust parentheses/generics/type paths, all enumerated TS wrappers, JS parentheses and constructors; direct/self/member/value/sequence/conditional/dereference controls, eval, recursion and shared relation scopes | **verified fixed** | Review-8 L1: callee-coverage and complete/narrowed recursion matrices, shared operand/eval readers pass. Calls to another established declaration still prove No where coverage is complete. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-2.md — Cache validation accepts nonexistent evidence identities and missing callable regions` | Every-language phantom owners, applicable candidates/targets, owned/ownerless targets, Python/Lua empty callable regions, summaries, callable/narrowed entries and null/bodiless controls | **verified fixed** | Review-8 L1: every-language retained identity mutation tests and empty Python/Lua callable-region cases pass. Missing identities require a compatible omitted kind; regions use retained body facts. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R3-F1 — TypeScript instantiation expressions hide calls and constructor invocations | `.01–.18`, `.C1–.C7`, ledger `.R`: wrapper/member/constructor operands, usages/arguments and all-language controls; classification, candidates, summaries, coverage/recursion, projection/widening, CLI and revision invalidation | **verified fixed** | Review-8 L1: TS instantiation invocation/coverage/recursion, member candidates, cache narrowing/widening/hits, CLI and stale revision tests pass. Both classifier and operand reader share the instantiation rule. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R3-F2 — Rust associated value calls falsely prove absence of recursion | `.01–.16`, `.C1–.C5`: inherent/trait/enum/generic/wrapped value paths and unrelated initializer controls; function/constructor/usage controls, candidate fallback, coverage, summaries, narrow recursion, semantic widening and callees | **verified fixed** | Review-8 L1: associated value call, summary and complete/narrowed recursion rows, record-only semantic widening and CLI coverage pass. Unique members and uncertain candidates both check value kinds. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R3-F3 — Cache validation accepts false source facts and misordered or duplicate lists | `.L01–.L16.{region,path,candidates}`; `.S01–.S03.{rs,ts,js}.{duplicate,reverse}`; `.S04–.S10`; `.S09r.{rs,ts,js,go,py,java,cs,c,cpp,swift,scala,php,lua,perl,sh,zsh,omitted}`; `.C1–.C4`; `.M.kind` and eight owner/coverage coordinate leaves | **verified fixed** | Review-8 L1: all-language source-fact/candidate/region edits, duplicate/reversed summaries, path/start deletion, omitted-record and full leaf matrices pass. Retained geometry, written source paths, list order and summaries are checked. Perl collection was repaired at 19:02 before the earlier 19:14–19:25 cache repair; this cycle preserves that unblocking. Coordinated deletion of both redundant fields remains the review-4-excluded limit. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R3-F4 — Optional and tagged eval calls incorrectly hide lexical declarations | `.01–.07`, `.C1–.C5`: TS/JS optional/tagged/wrapped eval, genuine direct/sequence/member/alias controls; scope barriers, declared bases, summaries/candidates, coverage/recursion, filtering/cache/callers | **verified fixed** | Review-8 L1: `EVAL_CALL_FORMS`, scope/relation/summary/coverage/recursion, cache and CLI tests pass. Both binding and coverage distinguish optional/tagged calls from direct eval. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R4-F1 — Written-path collection drops qualifiers and can bind rooted names to locals | `.01–.28`, `.C1–.C7`: roots, nested C++ paths/generics, C# alias/conditional/generic paths, PHP nullsafe/namespaces, Perl package/receiver paths and all-language controls; exclusions, lookup/category/stopping, candidates/summaries, recursion, projection/cache/reporting/docs/revision | **verified fixed** | Review-8 L1: `WRITTEN_PATH_MATRIX`, rooted Rust target/summary/recursion, narrow/wide/warm/projection, text/JSON reports and stale-entry tests pass. Rooted names never fall into local lookup. Java receiver rows are now independently restored too. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R4-F2 — Invocation classification omits calls and mistakes PHP callable creation for execution | `.01–.24`, `.C1–.C6`: rooted calls, C# conditional/generic/delegate chains, PHP placeholder versus unpacking for named/qualified/static/instance/variable forms, variable constructors, Perl code-reference and name-only negatives; coverage, candidates/summaries, filtering/cache/reports/docs/revision | **verified fixed** | Review-8 L1: every .01–.24 invocation row, placeholder/unpacking negatives, conservative summaries/recursion, cache/projection and report tests pass. The invocation climb handles those roots, conditional members and variable calls. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R5-F1 — Invocation classification still omits constructed and implicit member targets | .01–.22, .C1–.C6; ledger .S1–.S20: Java/Scala/Swift/C++ constructed/generic/implicit targets, all-language negatives, binding/candidates, summaries, coverage/recursion, cache/projection, reports/docs/revision | **verified fixed** | All required review-5 rows pass current invocation/negative matrices, conservative-result assertions, six-stage cache tests and `refs::constructed_types_and_implicit_members_are_callees_and_callers`. Swift subscripts remain Usage. Scope is closed for those rows; ledger .S12’s additional grammar limitation is evaluated separately below. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R5-F2 — Unreported declaration tokens leak into reference occurrences | .01–.38, .C1–.C6 and .S1–.S20: all declaration sites/controls, constraints/defaults/body reads, exclusions/candidates, ownership/summaries/recursion, filter/cache/CLI/docs/revision | **verified fixed** | Review-8 `DECLARATION_EXCLUSION_MATRIX` asserts exact missing declaration offsets **and** retained real-read offsets; all rows pass. Shared Python/Scala pattern readers, wildcard controls, cache unreported-declaration stages and CLI suggestions pass. Exclusion precedes candidate selection. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R5-F3 — Receiver-qualified Java construction loses its written target | .01–.10, .C1–.C5, .S1–.S15; R5-X.01–.15 (fixed Python-star/PHP-static siblings plus controls): receiver/generic/anonymous paths, candidates/lookup, reports, cache/projection/docs | **verified fixed** | Current full-path matrix, conservative Java target assertions, `written_receiver_paths_hold_through_the_cache` and receiver text/JSON reports pass. Exact receiver paths, independent receiver/argument reads, Swift operator exclusion, Python star and PHP static paths are asserted. R5-F1’s generic-classification prerequisite was repaired before these dependent rows. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R5-F4 — Parameter captures turn type and default-value references into declarations | .01–.16, .C1–.C6, .S1–.S3: Python/C# callable forms, actual record lists/types/default reads, PHP constants/defaults and Rust pattern paths, counts/signatures/candidates/selection/cache/provenance/revision | **verified fixed** | Current parameter-capture matrix, signature tests, cache six-stage exact record/read assertions, CLI ambiguity and revision-43 invalidation pass. Python/C#/PHP/Rust queries field binding names precisely. R5-F4 was repaired before R5-F2 exclusion consumed its definition set. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R6-F1 — Force-unwrapped and null-forgiving invocations disappear from call reports | .01–.18, .C1–.C6: Swift/C# bare/grouped/member/repeated/optional assertions; all-language negatives; operands, spans/paths/candidates, summaries/coverage/recursion, six cache stages, reports/docs/revision | **partially fixed** | Current invocation/negative/conservative-result, cache-stage and CLI tests pass. Independent CLI probes confirm Calls and preserved receivers/paths. .17a grouped forms work; .17b still fails in the pinned parser. | Every supported required form and consumer is fixed. Literal .17b remains a grammar limitation under the accepted release boundary; no prerequisite was removed. |
| R6-F1.17b — Swift adjacent repeated/mixed assertions | cb!!() and cb?!(); classification, coverage and call reports | **still blocked** | Independent lint reports missing ! at both calls; cb stays Usage, while (cb!)!() and (cb!)?() are Calls. `swift_adjacent_unwrap_operators_are_a_grammar_limit` passes. Cargo.lock pins Swift 0.7.3; 0.7.1 is the manifest requirement. | Optional grammar work outside the accepted release contract; see Blocked Findings. The failing parser prerequisite existed before repair. |
| R6-F2 — Parameter readers mistake comments for default values | .01–.14, .C1–.C5: eight default-capable languages; C# methods/constructors/local functions, PHP functions/constructors/methods, Scala methods; names/types/arity/defaults/optionality, headers/cache/revision | **verified fixed** | Current comment-position/signature/header/six-stage-cache tests pass. C# skips extras before/after =; PHP/Scala read default_value; dead Swift helper read removed. Independent g(int x /* b */ = 1, int y = /* a */ 2) record has defaults 1/2, both optional. | None of the required cases remains failing. |
| R6-F2 ledger siblings — C# params and Swift aliases | Comments before params type/name and after the equals in a Swift type alias | **verified fixed** | `comments_beside_params_arrays_and_alias_targets_are_skipped` passes; both readers skip extras. Callable matrix checks record lists and arity. | None. |
| R6-F3 — Contextual syntax markers become false reference suggestions | .01–.11, .C1–.C6: Lua attributes, Java inference/legacy forms, modifiers; exclusions/candidates, ownership, summaries, reports and cache | **verified fixed** | Review-8 marker matrix, six-stage marker cache and cold/warm CLI tests pass. The R7-F1 Java static-import gate now rejects value-import evidence; Swift precedence markers are role-excluded. Independent static-import/Swift CLI controls confirm this. | Earlier enumerated cases and the R7-F1 recurrence are closed. |
| R6-F3.S1 — PHP class-relative markers | self/static/parent in type/new/instanceof slots; genuine same-spelled method reads | **verified fixed** | Current marker/cache/CLI rows pass. Independent PHP source retains self::static(), static::static(), parent::static() member Calls and excludes class-relative type/construction operands. | None. |
| R6-F3.S2 — Bash/Zsh declaration options | Leading -/+ words through --; ordinary commands/arguments/expansions | **verified fixed** | Both shell marker rows pass. Shared option count partitions options and operands; ordinary commands do not enter that rule. | None. |
| R6-F3.X1 — Lost initializer/default reads | .01–.17 plus .18: all 16 initializer readers, Java field/local/var/for initializers, TS/JS diagnostics defaults; records/references/cache/CLI | **verified fixed** | Current INITIALIZER_READ_MATRIX, public diagnostics-reference, cache and CLI tests pass. value is no longer a name field; parameter defaults are retained. | None of the enumerated cases remains failing. R7-F3 is a separate false-import sibling affecting the same public diagnostics-reference consumer. |
| R6-F3.X2 — Shell bare declaration operands | .01–.05: Bash/Zsh bare/word-shaped operands, expansions/calls, Bash integer control; cache/CLI/docs | **verified fixed** | Exact-offset, cache and CLI rows pass. Independent Zsh source excludes declaration operands while keeping four expansion reads and echo Call. Docs correctly allow excluded names without records. | None. |
| R5-F1.S12 — Swift specialized explicit initializer paths | C<Int>.init() and N.C<Int>.init(); qualifiers and conservative Calls/coverage | **still blocked** | Independent current CLI reports init Call with no path for both expressions. The pinned tree chooses comparison syntax. Accepted source-plus-pinned-grammar boundary predates all repairs. | Optional grammar work outside the required release contract; see Blocked Findings. Prerequisite unchanged before/after repair. |
| Earlier ledger observations — Java inferred-type markers and Lua attributes | Both observations carried into R6-F3.01/.02/.04/.05, including real same-spelled reads and suggestions | **verified fixed** | Current exact public tests and independent no-import Java/Lua probes pass. | No unchanged observation remains; static-import variant is also verified fixed under R7-F1. |
| R7-F1 — Syntax-marker exclusions still admit Java import-triggered and Swift precedence tokens | .01–.08a/.08b; .09/.10 controls; .11a–d/.12a–c/.13; every .14 language leaf; .C1–.C6; ledger .S1–.S7, including all .S5 language leaves | **verified fixed** | Inspected `SyntaxMarkers::nodes`, Java declaration/import gate, Swift role filters and their exclusion before resolution. Full `MARKER_EXCLUSION_MATRIX`, legacy/read/anonymous-region controls, marker cache stages and cold/warm ID-selected reports pass review-8 L1. Independent CLI retains only real Java `var()` and Swift group/type reads. | No earlier required marker case remains. No blocker changed. |
| R7-F2 — Declaration and import-name exclusions omit unsupported and compound name forms | .01–.15, including .02b, .05b, .09b, .10a–c, .11b, .12b and all eight .15 selectors; every .16/.17 clean-language leaf; .C1–.C7; ledger .S1–.S8 and .X1 | **verified fixed** | Shared `declared_name_tokens` is used by recorded-name ranges and the unreported-name inventory before lookup. Exact `DECLARATION_EXCLUSION_MATRIX` and every-shape six-stage cache pass; .12a preserves the genuine class ID, .12b has no fabricated forward/reverse edges. All Swift selectors, import reports, public diagnostics references and C/C++ elaborated-use record/read tests pass. Independent TS and Swift CLI controls confirm missing declaration tokens and retained actual reads. Java import masking was removed before these exclusions were verified; both decisions now pass together. | No enumerated declaration repair remains. R8-F1's comment-bearing Scala imported-token leak is a new sibling, not an unchanged .01–.15 reproduction. |
| R7-F3 — Java qualified references are misclassified as imports and disappear | .01–.13 including named annotation arguments/deep paths and genuine-import controls; every .14 reader; .C1–.C7; ledger .S1–.S7 and .X1–.X3 | **verified fixed** | Java capture is structurally inside `import_declaration`; all-query predicate guard passes. Full Java matrix/public diagnostics inventory, exact candidates, six cache stages, module/deep-annotation ID reports and imports tests pass. Other readers' binding/source-role matrices, alias dependencies, export exclusions, Go raw/blank/dot imports and revision-45 invalidation pass. Independent Java CLI has genuine imports only and preserves qualified annotation reads. | Original enumerated cases are closed. The ledger's broader claim that the import class has no remaining defect is disproved by R8-F1 and R8-F2; the comment and grammar-name variants were outside its enumerated shapes. |
| R7-OBS.1 — Python future imports in the separate type-owner relation reader | Future-import binding in relation/containment lookup, separate from conservative occurrence evidence | **unresolved** | The repair ledger explicitly defers this separate resolver consumer. Future-import occurrence exclusion and import facts pass the current Python matrix; the lexical relation reader still lacks the future-import arm. Accepted spec preserves containment/export contracts. | Nonblocking separate-contract work; retained below, not waived or claimed repaired. |
| R7-OBS.2 — Plain import reports reconstruct some syntax imperfectly | Language-specific reconstruction by `hug imports` text renderer; underlying binding records/JSON and required refs reports | **unresolved** | Ledger deferral remains. Inspected the separate import renderer; required refs text/JSON and exact import-record tests pass. Syntax reconstruction is outside the refs report contract. | Nonblocking separate display work; retained below. |


The original implementation log's accepted first-release decisions precede the repairs: Rust/TypeScript/JavaScript resolution, conservative evidence elsewhere, source plus the pinned grammar, file-local analysis and no receiver inference. None changed this cycle. The historical Perl qualifier prerequisite was removed before its cache repair, as recorded in the R3-F3 row. The single-field cache-path repair remains required and fixed; coordinated deletion of both redundant fields remains the accepted forgery limit. The two Swift parser prerequisites remain unchanged before and after repair. No human blocker was removed, and none is needed for either new finding.

## Unblocked Findings

### R8-F1 — High — Comments corrupt import identities, qualifiers and binding exclusions

**Defect class:** import readers and path/capture decisions use the position or adjacency of named syntax children without skipping grammar extras, so a comment changes binding identity, written paths or whether an import is collected.

In the **tree-hugger** library, [extract_ecma_import_metadata](../../lib/src/file/tree_file.rs:597), which preserves an alias's original target, reads `import_alias.named_child(1)`. That child can be a comment. [extract_scala_import_metadata](../../lib/src/file/tree_file.rs:947), which constructs import paths and identifies on-demand imports, similarly trusts immediate named siblings. [scala_import_path_start](../../lib/src/analysis/declaration_names.rs:420), which supplies the beginning of a Scala import path to both imports and occurrences, stops at a comment. The [Scala import query](../../lib/queries/vendor/scala/locals.scm:97) also relies on adjacency before a comma or wildcard/selector node, losing captures when a comment intervenes.

```typescript
import D = N.S;
function S() { D(); }
```

The accepted import-recursion rule requires `Unknown`: an external alias whose original final name is `S` could invoke the selected function. Inserting only `/* note */` after `D`, or after `=`, changes the imported original target to that comment and changes `S.semantics.is_recursive` to **`No`**. The import remains `Scoped` with no target ID; its owned call is retained, but `relations.references` and `relations.dependencies` now carry the comment instead of `N.S`. A line comment has the same effect. The pinned parser reports no syntax error.

```scala
import a.b._
object U { def b() = 1; def f() = b() }
```

Inserting `/* note */` between the final dot and `_` removes the on-demand import. The previously excluded import-path token `b` becomes a Usage with the local method as a NameOnly candidate. `hug refs --id <b-ID>` then lists this false suggestion at the import as well as the genuine body Call. Both JSON and text should retain only the genuine Call for this selected method. A comment before a comma similarly exposes the final bound name of the preceding import.

To reproduce the TypeScript result, save the control as `alias.ts`, run `hug symbols --json-full --no-cache alias.ts`, insert the comment, and repeat: the import original and S's recursion change despite unchanged call semantics. For Scala, save the second control as `alias.scala`, insert the comment and run `hug imports --json alias.scala` and `hug refs b --json alias.scala`; the import disappears and a suggestion at the import is added. Use `--verbose` on repeated cached runs to verify the warm result is actually served.

The sweep starts with copies of the library's [Scala import fixture](../../lib/tests/fixtures/imports.scala) and [TypeScript import fixture](../../lib/tests/fixtures/imports.ts). Scala controls preserve all six expected bindings. For TypeScript, the copied fixture's first import is changed to an entity alias and a small callable is added once to make the original target load-bearing; each test edit then inserts exactly one comment into that established control. Small sources below isolate the decisions and test same-spelled genuine calls. Both block and line comments were checked where the pinned grammar accepts them. Comments remain source text; a full written path may include that text, but it must not lose a qualifier or become a comment alone.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R8-F1.01 / TS entity alias before equals | `import D /* note */ = N.S`; also `import D // note` newline `= N.S` | Original target is comment; `S` becomes No; summaries carry comment | Original target N.S; genuine D Call remains Scoped/no ID; recursion Unknown | Error-free public API and full-index CLI; copied fixture's `readFile = N.S` has the same failure; all six cache stages |
| R8-F1.01.export / TS exported entity alias | `export import D /* note */ = N.S` | Same comment-valued original and false No; no export ID fabricated | Preserve N.S/Unknown and existing export shape | Error-free control/edit public CLI and all six API cache stages |
| R8-F1.02 / TS entity alias after equals | `import D = /* note */ N.S` | Same wrong identity/summaries/No | Same independent expected identity and Unknown | Error-free API/CLI; semantic widening, complete and warm results assert current false No |
| R8-F1.03 / Scala path walker | `import a /* note */ .b.C`; block and line comments between path segments; braced selectors and wildcard variants | Source b.C/b.C-selector/b for glob; occurrence b loses a qualifier | Preserve complete written prefix; still bind C/selector aliases or on-demand b; exclude binder and retain other qualifier reads | Copied ListBuffer fixture changes source to collection.mutable.ListBuffer; public imports/occurrences and six-stage path probe |
| R8-F1.04a / Scala plain selector source | `import a.b. /* note */ {C}`; line-comment variant | Source `/* note */.C` (or line-comment equivalent) | Selector C from the complete a.b path; no comment-only prefix | Copied Source/StdIn fixture corrupts both sources; API/CLI and cache |
| R8-F1.04b / Scala renamed selector source | `import a.b. /* note */ {C => D}`; `as` form uses the same selector-source reader | D's source `/* note */.C`; original C and alias D otherwise remain | Keep C original/D alias and complete a.b.C source | Arrow reproduction through six stages; the `as` branch is the same match arm and was separately CLI-probed below |
| R8-F1.05.star / Scala direct wildcard | `import a.b. /* note */ *`; block/line | No imports; b becomes a Usage | On-demand import b, original `*`, source a.b; b token excluded; actual body b remains | Error-free API/CLI; six-stage lookups and ID-selected default/Call reports |
| R8-F1.05.underscore / Scala direct wildcard | Same with `_` | Same lost import and false b suggestion | Same on-demand evidence; `_` and `*` have the same contract | Copied java.util._ fixture loses util; ID-selected b text/JSON cold Miss then warm Hit |
| R8-F1.05.given / Scala direct given import | Same with `given` | No imports; b becomes Usage | On-demand import b, original given, source a.b; keep required type/body reads | Error-free CLI/API and six cache stages; comment-free given control has exact expected import |
| R8-F1.06.mixed / Scala braced on-demand selectors | `import a.b. /* note */ {_, given T}` | No b import; b becomes Usage; T retained | b import with original `*`, complete source a.b; exclude b, retain T | Error-free API/CLI and six stages; within-brace comment control stays clean |
| R8-F1.06.given / Scala braced given selector | `import a.b. /* note */ {given T}` | Same missing b import | b import with original given; retain T | Error-free API/CLI and six stages |
| R8-F1.07.simple / Scala pre-comma capture | `import C /* note */, d.E` | Only E imported; C at 1:8 is a false incoming suggestion to U.C | Import C and E; exclude C's binder; keep the body C Call | Error-free public diagnostics references/API/cache; ID-selected C JSON/text and both Call directions |
| R8-F1.07.qualified / Scala pre-comma capture | `import a.b.C /* note */, d.E` | Only E imported; C at 1:12 becomes Usage with path a.b.C | Both imports; qualifier a/b reads remain, bound C token excluded | Public diagnostics references, full indexes and six-stage probe |
| R8-F1.08 / Scala clean comment positions | Trailing comment after final import, after wildcard, after comma; inside selector list; before/after rename arrow or as | Expected imports, source paths and real reads remain | Preserve these positive controls; distinguish a bound alias from its read original | Error-free direct CLI/API controls: end/star-suffix/multipath/alias-before/alias-after/within-brace rows |
| R8-F1.09.ts / TS clean import forms | Entity path comment inside N.S; trailing comment; require alias; default/named/aliased/namespace imports | Identity comes from actual target path/name fields; require source m retained | Comments must not affect binding or original terminal name; maintain actual no-ID import Calls | Direct CLI and public cache controls; genuine import/dependency matrix passes L1 |
| R8-F1.09.js / JS reader | Named import with comment around alias; default and namespace controls | Original name/source/alias correct | Preserve c→d from m and its actual d Call | Direct CLI/API/cache plus shared ECMA matrix; JS has no entity-equals alias syntax |
| R8-F1.10.rs / Rust reader | Commented use path/list, renamed import, self, bare/braced/direct glob, extern alias controls | Expected bindings/sources/originals retained | Preserve exact dependencies and glob/self distinction; actual call remains | Direct CLI/API/cache controls and current Rust import-binding matrix; no import-name comment poisoning |
| R8-F1.10.java / Java reader | Comment between qualified import components and after final name | C source retains complete prefix as written; real C type read retained | Exclude imported C only; preserve actual qualifier/type reads | Error-free CLI/API/cache; source fields and structurally scoped query inspect clean |
| R8-F1.10.cs / C# reader | Comment beside alias equals and in target path; using/static/global controls | D original C, source full a.b.C spelling; real D usage retained | Field-selected target/name must remain independent of comments | CLI/API/cache and full CSHARP_USING_MATRIX; target selection skips extras |
| R8-F1.10.php / PHP reader | Comment in use path and before alias; single/grouped/function/constant controls | D original B; source preserves full path; real D type read retained | Preserve final binding and complete prefix | CLI/API/cache and PHP use-clause matrix; path children filtered by kind |
| R8-F1.10.swift / Swift reader | Comment in selective path; all eight selectors and whole-module controls | Bar binding/source intact, Foo and genuine later Bar read retained | Exclude final selective token only; whole-module tokens excluded | Direct CLI/API/cache and every-selector L1 matrix; final-name selection filters actual identifiers |
| R8-F1.10.py / Python reader | Parenthesized from-import list with line comment, dotted/aliased/future/wildcard controls | Correct C/E bindings and E original D; actual C/E reads retained | Comments do not create imports or suppress required reads | Direct CLI/API/cache and Python import matrix; fields/identifier-only captures remain clean |
| R8-F1.10.go / Go reader | Comment between alias and quoted path; raw/blank/dot controls | D source a/b, original b, actual D/F usages retained | Preserve metadata/reads with no phantom alias | Direct CLI/API/cache and full Go matrix; path/name fields independent of extras |
| R8-F1.11.{c,cpp,lua,perl,bash,zsh} / remaining readers | Effective import-definition capture inventory | No `local.definition.import` capture | Clean for this class: these import collectors cannot mistake an extra for an imported target; broader import support not claimed | Effective query invariant and all-query guard/current per-language occurrence matrices |

The `as` rename, comments after entity-alias targets and ordinary ECMA imports are preserved controls for the same parser decisions; they must be present in the permanent repair matrix. A Scala line comment immediately before a comma produces a parser error in the pinned grammar and is not used as a required error-free reproduction.

| Child ID | Decision/consumer | Required result and independently checked evidence |
|---|---|---|
| R8-F1.C1 | Import collection, original identity and path start | Read the semantic target rather than a positional comment; make every Scala import shape and prefix walk tolerate extras. Keep comma boundaries and all .08–.11 controls. Query context remains structurally import-scoped. |
| R8-F1.C2 | Exclusion, categories, candidates and stopping | A missing Scala capture currently bypasses `definition_ranges`, allowing an imported token into NameOnly candidate lookup; remove these extra Usages before candidates. Genuine body b/C Calls must remain conservative candidates, with no invented target ID. TS D must remain an established import binding with absent external ID, not degrade to Unresolved to hide the wrong metadata. |
| R8-F1.C3 | Summaries, ownership, coverage and recursion | TS owned D Call must use N.S in references/dependencies and keep S Unknown; current comment-bearing result falsely yields No under Complete coverage. Scala must stay Limited/Unknown with empty established summaries; keep named/file-level regions. Recursion lookup's original-terminal-name input is load-bearing even though target IDs do not change. |
| R8-F1.C4 | Narrowing, projection, persistence and widening | Independent scratch public API checks 56 sources through narrow Miss → narrow Hit → semantic widening (`parsed: false`) → all-kind widening → complete Hit → type-only projected Hit. Warm/record-only stages parse zero files. Current wrong identities, missing imports and extra tokens survive every applicable stage. Assert correct identities/paths/read offsets, summaries and Unknown, not just equality to an equally wrong fresh result. |
| R8-F1.C5 | Public diagnostic-reference reader | Scala imported C/b leaks through `TreeFile::referenced_symbols` as well as occurrences. Restore exclusions there while retaining every genuine qualifier/body read and existing diagnostic severity/default enablement. TS D's real call remains in this API; its metadata defect must not remove that read. |
| R8-F1.C6 | CLI imports, declarations and refs | Full-index JSON and import JSON expose wrong metadata; ID-selected refs JSON includes S's false No and comment-valued dependencies. Scala cold/warm ID-selected default JSON/text includes false import-token suggestions; callers correctly excludes those Usages and retains genuine Calls, callees is unchanged for the selected empty b/C bodies. Both cache runs were actual Miss/Hit, with zero warm parses. Check all directions and preserve file-local boundary/selection. |
| R8-F1.C7 | Shared relation/type-owner lookup and export consumers | Clean independent consumer: `Lexical::build` reads `file::extract::import_binding`, not ImportRecord original-name metadata. Its TS alias arm finds the identifier by kind; its Scala reader explicitly skips comment/block_comment and reads alias fields. It therefore preserves binding barriers for these comments. Keep these category/lookup stopping invariants and stable declaration identities; exported entity-alias control/edit CLI produces no fabricated export ID. Verify existing imported-base/path matrices after repair, without a containment/export redesign. |
| R8-F1.C8 | Regression coverage, documentation, comments and invalidation | Extend declared source/import, summary/recursion, public-reference, six-stage cache and CLI matrices for every required child/clean control above. Review affected doc comments and current import/recursion docs; update query provenance if query patterns change, and invalidate old extractor output because Rust path/metadata code changes alter stored results. No new crate, compiler integration or format reader is needed. |

**Required repair:** close .01–.07 and all their named leaves, verify .C1–.C8 and preserve .08–.11. This is shared source-evidence work; report-specific filtering or rejecting all commented imports would lose provable results. No parser upgrade, receiver inference or human permission is required. Priority is high because the TypeScript result violates the requirement that `No` be proven from correct call evidence.


### R8-F2 — Medium — Scala import collection omits rename and operator forms

**Defect class:** the import query and path readers enumerate ordinary identifier names and braced renames only, omitting valid unbraced renames and symbolic-name alternatives that the pinned grammar already exposes.

In **tree-hugger**, the [Scala import query](../../lib/queries/vendor/scala/locals.scm:97) captures a rename only beneath `namespace_selectors` and captures direct bound names only as `identifier`. Its reader's [selector_source](../../lib/src/file/tree_file.rs:946) assumes a rename's parent is that selector group. Simply adding an unbraced capture would still lose its path. [scala_import_path_start](../../lib/src/analysis/declaration_names.rs:420) also stops at `operator_identifier`, so even a captured ordinary final name can lose symbolic qualifiers.

```scala
import a.b.C as D
object U { def f() = D() }
```

This error-free source reports **zero imports**, although it binds D with original C and source a.b.C. The documented import contract permits renames and does not require braces. Likewise `import a.b.+` binds `+`, and `import a.{C as ++}` binds `++`; both report zero imports. `import a.+.C` does report C but stores only `C` as its source. These are different missing grammar alternatives from R8-F1's comment-sensitive decisions and need their own regression rows.

The sweep varied position (direct/braced/comma-separated/nested), original and alias name kind (ordinary/backticked/operator), hidden aliases, qualifier kind, wildcard and given controls. The existing Scala fixture was copied for a control/edit reproduction: its two braced java.util renames are replaced by equivalent comma-separated unbraced `as` renames. Expected JList/JMap bindings and sources are stated independently; both disappear. Other import readers were checked for analogous name alternatives using the R8-F1 controls, TS/JS string-named import originals and pinned grammar/inventory invariants below.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R8-F2.01.single | `import a.b.C as D` / shorter a.C path | No imports; actual D read/Call remains conservative | D alias, original C, source a.b.C (or a.C); alias token excluded, qualifiers/original/body reads retained | Error-free CLI/API and all six cache stages |
| R8-F2.01.multiple | `import a.b.C as D, x.y.E as F` | No imports | D and F with respective original names/complete sources | CLI/API/six stages; copied JList/JMap fixture edit |
| R8-F2.01.nested | `object U { import a.b.C as D; def f()=D() }` | No import D | Same import facts; real owned Call and conservative ownership/coverage preserved | Error-free full index/API/six stages |
| R8-F2.02.escaped | ``import a.b.C as `D` `` | No imports | Backticked D alias with original C/full path, consistent with braced escaped-name control | Error-free CLI/API/six stages |
| R8-F2.02.original | `import a.b.++ as sum` | No imports | sum alias, original ++, source a.b.++ | Error-free CLI/API/six stages; braced operator-original control passes |
| R8-F2.03.direct | `import a.b.+` | No imports | Imported +, source a.b.+ | Error-free CLI/API/six stages; cold import JSON/warm text says no imports |
| R8-F2.03.multiple | `import a.b.+, c.d.**` | No imports | Both symbolic bindings with independent full sources | Error-free CLI/API/six stages |
| R8-F2.03.braced | `import a.{+, ** as mul}` | Only mul reported | Both + and mul; mul original ** stays correct | Error-free CLI/API/six stages |
| R8-F2.04.arrow | `import a.{C => ++}` | No imports | ++ alias, original C, source a.C | Error-free CLI/API/six stages; cold/warm imports reports |
| R8-F2.04.as | `import a.{C as ++}`; mixed C as D, ++ as sum, E as ** | Symbolic aliases omitted; ordinary D/sum kept | Retain all non-hidden aliases, including ++/** and their originals/sources | Error-free CLI/API/six stages |
| R8-F2.05.direct | `import +.C`, `import a.+.C` | C source C | Full +.C/a.+.C; no fabricated local binding ID | Error-free CLI/API/six stages; cold/warm JSON |
| R8-F2.05.braced | `import a.+.{C}` | C source +.C | Complete source a.+.C | Error-free CLI/API/six stages |
| R8-F2.05.wildcard | `import a.+.*` | No imports | On-demand import + with original *, full source a.+ | Error-free CLI/API/six stages |
| R8-F2.06 / Scala clean alternatives | Braced arrow/as with ordinary aliases, symbolic original ++ => sum, hidden C as _, backticked direct/qualifier names, this/super/_root_ paths, ordinary/given/wildcard controls | Expected bindings/paths kept; hidden alias binds nothing | Preserve all independently expected positive/negative controls | CLI/API/six stages; existing Scala matrix/metadata tests pass |
| R8-F2.07.ecma / TS and JS | String-named original `import { "a-b" as c } from "m"`, ordinary/default/namespace/require/equal-alias controls | c import with original literal and source m; real c Call retained | Preserve alias binding; do not interpret the string as a local declaration | Error-free direct CLI; field/kind-based ECMA query invariant and current binding/cache matrices |
| R8-F2.07.{rs,java,cs,php,py,go} / Rust, Java, C#, PHP, Python, Go | Qualified/plain/aliased/list/on-demand forms inventoried in R8-F1 and current public matrices | Enumerated required names remain | Clean for these syntax alternatives: each reader uses its grammar's actual identifier/path/alias fields; Scala unbraced as/operator nodes are absent from these grammars | Public binding matrices and exact context/kind invariants in effective queries; no claim that every language supports Scala syntax |
| R8-F2.07.swift / Swift | All eight selective imports/whole-module paths; attempted selective operator `import func Foo.+` | Normal names work; operator form produces parser ERROR | Preserve supported selectors. Operator spelling is not an error-free alternative in this pinned parser and is not a required new binding case | Independent CLI plus current every-selector matrix; no Swift grammar upgrade requested |
| R8-F2.07.{c,cpp,lua,perl,bash,zsh} | Effective import-definition capture inventory | No import capture to omit these alternatives from | Clean for this collector class by invariant, without claiming broad import support | Same effective query inventory as R8-F1.11 |
| Child ID | Decision/consumer | Required repair or verified invariant |
|---|---|---|
| R8-F2.C1 | Collection, name kind, original identity and full path | Support every required rename/name/qualifier alternative above in the shared query/reader; unbraced rename metadata must use its own clause/path context. Preserve comma/source boundaries, deduplication and hidden aliases. Apply R8-F1's comment controls to every new alternative. |
| R8-F2.C2 | Exclusion, real reads, candidates and ownership | Ordinary unbraced alias exclusion already uses the shared as-rename inventory; actual D reads/Calls remain conservative. Keep that invariant when adding import records. Operator names outside current reference captures must not become synthetic resolved declarations or fake edges; Limited coverage remains explicit. Captured qualifier/original reads must neither disappear nor be invented. |
| R8-F2.C3 | Summaries, recursion, type-owner lookup and exports | Scala remains Limited/Unknown with no established summaries. Independent relation reader `scala_import_names` already handles unbraced as, symbolic paths and aliases by fields/kinds, so import lookup barriers must stay correct; this is not permission to add lexical resolution or infer owners from import metadata. Stable declarations/export identities must not change. |
| R8-F2.C4 | Selective filtering and cache stages | All failing forms and controls reproduce through narrow Miss/Hit, zero-parse semantic widening, kind widening, complete Hit and projected Hit. Assert complete independently expected import lists/sources/aliases, not merely cached/fresh equality. |
| R8-F2.C5 | CLI/public API consumers and tests | `TreeFile::imported_symbols`, full indexes and cold/warm imports JSON/text currently omit bindings or report truncated sources. Public diagnostic/occurrence reads do not gain permission to become Scoped in Scala. Add declared L1 fixture matrices for the full inventory, public references, imports and all cache stages; preserve existing refs selection/Call filters and conservative results. |
| R8-F2.C6 | Current docs, comments, provenance and invalidation | Update actual import examples/name alternatives and affected comments; update query provenance and invalidate changed extractor output. No new dependency or performance gate is needed. |

**Required repair:** close .01–.05 and every leaf, verify .C1–.C6, preserve .06/.07, and cross these repaired forms with R8-F1's comment positions. No grammar prerequisite or human decision blocks this work. This does not request general operator-reference extraction beyond the feature's existing conservative capture contract; it requires correct import facts for parseable binding forms.

### Nonblocking carried observations

- **R7-OBS.1 — Python future-import handling in the separate relation reader:** still unresolved. Add that reader's future-import arm and a type-owner relation regression when repairing that separate contract. Occurrence/import facts remain verified; this is not a required expansion of Python occurrence resolution.
- **R7-OBS.2 — Reconstructed plain import syntax:** still unresolved. Improve the separate `hug imports` renderer with language-specific output tests. Required refs text/JSON, import binding records and JSON remain independently checked. This observation does not waive R8-F1's incorrect underlying import facts.

## Blocked Findings

No required repair is blocked. `has_blocked_findings: false` and `blocked: false` refer to the required release inventory.

- **R6-F1.17b — Swift adjacent repeated/mixed assertions:** still blocked by the pinned grammar; `cb!!()`/`cb?!()` still produce missing-`!` syntax errors and Usage, while `(cb!)!()`/`(cb!)?()` remain Calls. Independent review-8 CLI probe and current grammar-limit test agree.
- **R5-F1.S12 — Swift specialized explicit initializer paths:** still blocked by the pinned grammar in the documented comparison-parsing context; `C<Int>.init()` and `N.C<Int>.init()` still report an init Call without its specialized path in the independent review-8 probe. This is context-sensitive; supported generic construction controls still pass.

These are unchanged optional grammar expansions outside the accepted release boundary, established by the specification's Goal and boundaries and original implementation log's accepted first-release breadth. They existed before repair, and no prerequisite has been removed. Human reaffirmation is unnecessary. Review 6's nonblocking C++ parenthesized-allocation observation is likewise optional pinned-grammar work, not a required repair newly waived here.

## Recurrence

**R8-F1** repeats **review-6 / R6-F2 — Parameter readers mistake comments for default values** at different syntax readers: a named extra occupies a positional/sibling slot and is mistaken for semantic input. The original parameter/default cases remain fixed. It also repeats the source-role/import-evidence class of **review-7 / R7-F3 — Java qualified references are misclassified as imports and disappear**, especially that repair ledger's **R7-F3.S6** Scala imports and **R7-F3.X1** TypeScript entity aliases; the Scala extra-reference symptom belongs to **review-5 / R5-F2 — Unreported declaration tokens leak into reference occurrences** too. Comparisons with reviews 1–4 found related invocation/path/cache classes but no earlier enumerated comment-bearing import reproduction.

These are **newly discovered comment-bearing siblings outside the earlier cases**, with a demonstrated **repair regression** in the TypeScript alias metadata: commit `580342c73` added the `named_child(1)` alias arm while repairing import evidence. Its comment-valued identity and false No did not follow from the previous absence of that original-name metadata. That commit also added the Scala prefix/selector readers and adjacency-sensitive query shapes; the new comments expose incomplete repairs to their earlier broken source/binding inventory, not a failure of the original comment-free Java ancestry reproduction. No required instance is an unchanged failing review-7 headline source.

Missed decisions were named-extra skipping, query adjacency, comma/path boundaries, selector prefix/original identity and the imported terminal name used to stop recursion lookup. Missed consumers were public diagnostics references, NameOnly incoming suggestions, import summaries/dependencies, recursion, warm/projected results and CLI declaration JSON. The full required child/consumer list is now R8-F1.01–.11 and .C1–.C8; a passing comment-free import matrix is insufficient to close it.

**R8-F2** repeats **review-7 / R7-F3**, specifically its ledger's **R7-F3.S6 — Scala qualifiers/renames/selectors/on-demand imports**. These are **newly discovered grammar-form siblings**, not unchanged earlier comment-free braced rename cases: collection omitted unbraced contexts and operator-name alternatives, while prefix lookup stopped at symbolic segments. The missed consumers are public imports/complete indexes, every cache stage and JSON/text import reports. The existing independent relation reader already handles those names; repairing only it would leave reports incorrect. R8-F2 retains its complete instance and consumer inventory separately from the comment class.

## Requirement Verification and Test Levels

| User-facing requirement | Verification present | Assessment |
|---|---|---|
| Lexical order/hoisting/shadowing, parameter/body environments, categories, aliases and supported item boundaries | L1 scope/occurrence/relation/CLI matrices, rerun | Earlier enumerated repairs pass; import original-name evidence is R8-F1 and missing Scala forms are R8-F2. |
| Invocation versus value reads, wrappers, constructors, repeated sites and conservative receivers | L1 all-language invocation/negative/path/coverage matrices, rerun | Supported forms pass; accepted Swift parser limits remain explicit. |
| Declaration/import/marker exclusion, genuine reads, qualifiers, spans and owners | L1 exact matrices, public API and independent shipped-CLI edits | R7 cases pass; comment-bearing Scala imports remain R8-F1; missing import forms/paths remain R8-F2. |
| Correct relationship summaries and honest recursion | L1 complete/narrowed summaries/recursion/cache plus independent alias probes | Gap: wrong import identity falsely proves No; R8-F1.C3. |
| Whole-file filtering/projection, one shared collection, work avoidance, actual hits/widening, strict malformed-cache handling | L1 counters/query/cache/robustness/upgrade matrices plus independent six-stage API | Infrastructure passes; wrong source facts are faithfully cached, so output agreement is not correctness proof. |
| Name/ID/smart-case selection, ambiguity, local/empty/errors, both Call directions and file-local boundary | L1 refs suite and independent cold/warm JSON/text | Earlier cases pass; default suggestions/declaration metadata are incorrect in R8-F1; public import facts are incomplete in R8-F2. |
| Terminal grouping, colors/styles, wrapping and empty-report boundary | L2 actual tmux pane capture through live area recipe | Passed, backend run=1/skip=0/panic=0. No focus request; no WezTerm/Kitty execution claim. |
| Keyboard encoding, hotkeys, paste/IME/mouse behavior | No such requirement in this feature | L3 unnecessary. |

Review-8 verification:

- `just test -j 4` in tree-hugger: **1,462 passed, 0 skipped**, 219.507 seconds.
- `just lint`: **passed** for library and CLI; no formatter run.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_declarations_in_tmux`: **1 selected test passed**, 5.546 seconds; **tmux run=1, skip=0, panic=0**.
- Root `just check-tier-coverage tree-hugger`: **zero stranded tests**. Library `tests/l1/main.rs` includes the occurrence/cache/public-reference modules; CLI `tests/symbols/main.rs` includes refs/cache/upgrade modules. Their names select L1. The terminal target and live recipe enable `terminal-tests`, also present in CI metadata.
- Independent shipped-CLI and public API sweeps use static fixture copies and one-comment edits, with positive controls and explicit expected metadata/read offsets/recursion. **56 source copies** traversed all six cache stages in the first sweep; **25 additional sources** (including the complete R8-F2 alternatives and exported TS alias control/edit and copied unbraced-rename fixture edit) traversed the same stages; syntax diagnostics distinguish supported failures from parser-limited probes. Scratch experiments are not counted as permanent tests.
- All earlier reviews' frontmatter/body/required-repair inventories were compared with review-7 and its ledger. Original implementation claims and `implemented_by` are preserved. Cross-OS evidence is left to CI and does not determine readiness.

## Input Robustness Matrix

The only changed file-format reader in this feature is the persistent **JSON cache**. The latest source repairs add no manifest, lockfile or configuration reader. Its permanent entry/record/reference-evidence matrices ran in the full review-8 L1 suite, through public hit/miss results from tool-written controls. R8-F1 concerns valid source facts computed incorrectly, not acceptance of malformed cache JSON.

The tables below retain every occurrence/coverage leaf from the prior review; capabilities and covered-kind/entry/record columns also run in the permanent tests and are specified in the current cache-design topic page. Each shape restores the unedited control before one mutation. O is an owned Scoped call, N a NameOnly member call, Q an unresolved qualified call and R a limited region. H means an actual Hit; M means a miss/recompute of the independently expected control; — means no applicable element shape. Legitimate null/empty H controls are explicit values, not defaults for absent keys. Duplicate-key and trailing-content edits are rejected. No new matrix column was introduced by the latest test-only repair.

### Capabilities and evidence lists

| Shape | `completed_capabilities` | `occurrences` | `reference_coverage` |
|---|---|---|---|
| Control | H | H | H |
| Absent | M | M | M |
| Explicit null | M | M | M |
| Wrong whole type | M | M | M |
| One wrong element | M | M | M |
| Every element wrong | M | M | M |
| Empty | M | M | M |
| Duplicate key | M | M | M |
| Trailing/invalid content | M | M | M |

### Occurrence fields

| Shape | `O.owner` | `O.owner_span` | `O.target` | `O.span` | `O.kind` | `O.confidence` | `N.candidates` |
|---|---|---|---|---|---|---|---|
| Control | H | H | H | H | H | H | H |
| Absent | M | M | M | M | M | M | M |
| Explicit null | M | M | M | M | M | M | M |
| Wrong whole type | M | M | M | M | M | M | M |
| One wrong element | — | — | — | — | — | — | M |
| Every element wrong | — | — | — | — | — | — | M |
| Empty | M | M | M | M | M | M | M |
| Duplicate key | M | M | M | M | M | M | M |
| Trailing/invalid content | M | M | M | M | M | M | M |

### Owner and target reference leaves

| Shape | `O.owner.id` | `O.owner.name` | `O.owner.qualified_name` | `O.target.id` | `O.target.name` | `O.target.qualified_name` |
|---|---|---|---|---|---|---|
| Control | H | H | H | H | H | H |
| Absent | M | M | M | M | M | M |
| Explicit null | M | M | M | M | M | H |
| Wrong whole type | M | M | M | M | M | M |
| One wrong element | — | — | — | — | — | — |
| Every element wrong | — | — | — | — | — | — |
| Empty | M | M | M | M | M | M |
| Duplicate key | M | M | M | M | M | M |
| Trailing/invalid content | M | M | M | M | M | M |

### Candidate leaves and written paths

| Shape | `N.candidates.0.id` | `N.candidates.0.name` | `N.candidates.0.qualified_name` | `N.target.qualified_name` | `Q.target.qualified_name` |
|---|---|---|---|---|---|
| Control | H | H | H | H | H |
| Absent | M | M | M | M | M |
| Explicit null | M | M | M | M | M |
| Wrong whole type | M | M | M | M | M |
| One wrong element | — | — | — | — | — |
| Every element wrong | — | — | — | — | — |
| Empty | M | M | M | M | M |
| Duplicate key | M | M | M | M | M |
| Trailing/invalid content | M | M | M | M | M |

### Coverage fields

| Shape | `R.owner` | `R.span` | `R.state` | `R.reasons` | `R.owner.id` | `R.owner.name` | `R.owner.qualified_name` |
|---|---|---|---|---|---|---|---|
| Control | H | H | H | H | H | H | H |
| Absent | M | M | M | M | M | M | M |
| Explicit null | M | M | M | M | M | M | M |
| Wrong whole type | M | M | M | M | M | M | M |
| One wrong element | — | — | — | M | — | — | — |
| Every element wrong | — | — | — | M | — | — | — |
| Empty | M | M | M | M | M | M | M |
| Duplicate key | M | M | M | M | M | M | M |
| Trailing/invalid content | M | M | M | M | M | M | M |

### Occurrence span

| Shape | `O.span.start_byte` | `O.span.end_byte` | `O.span.start` | `O.span.end` | `O.span.start.line` | `O.span.start.column` | `O.span.end.line` | `O.span.end.column` |
|---|---|---|---|---|---|---|---|---|
| Control | H | H | H | H | H | H | H | H |
| Absent | M | M | M | M | M | M | M | M |
| Explicit null | M | M | M | M | M | M | M | M |
| Wrong whole type | M | M | M | M | M | M | M | M |
| One wrong element | — | — | — | — | — | — | — | — |
| Every element wrong | — | — | — | — | — | — | — | — |
| Empty | M | M | M | M | M | M | M | M |
| Duplicate key | M | M | M | M | M | M | M | M |
| Trailing/invalid content | M | M | M | M | M | M | M | M |

### Owner span

| Shape | `O.owner_span.start_byte` | `O.owner_span.end_byte` | `O.owner_span.start` | `O.owner_span.end` | `O.owner_span.start.line` | `O.owner_span.start.column` | `O.owner_span.end.line` | `O.owner_span.end.column` |
|---|---|---|---|---|---|---|---|---|
| Control | H | H | H | H | H | H | H | H |
| Absent | M | M | M | M | M | M | M | M |
| Explicit null | M | M | M | M | M | M | M | M |
| Wrong whole type | M | M | M | M | M | M | M | M |
| One wrong element | — | — | — | — | — | — | — | — |
| Every element wrong | — | — | — | — | — | — | — | — |
| Empty | M | M | M | M | M | M | M | M |
| Duplicate key | M | M | M | M | M | M | M | M |
| Trailing/invalid content | M | M | M | M | M | M | M | M |

### Coverage span

| Shape | `R.span.start_byte` | `R.span.end_byte` | `R.span.start` | `R.span.end` | `R.span.start.line` | `R.span.start.column` | `R.span.end.line` | `R.span.end.column` |
|---|---|---|---|---|---|---|---|---|
| Control | H | H | H | H | H | H | H | H |
| Absent | M | M | M | M | M | M | M | M |
| Explicit null | M | M | M | M | M | M | M | M |
| Wrong whole type | M | M | M | M | M | M | M | M |
| One wrong element | — | — | — | — | — | — | — | — |
| Every element wrong | — | — | — | — | — | — | — | — |
| Empty | M | M | M | M | M | M | M | M |
| Duplicate key | M | M | M | M | M | M | M | M |
| Trailing/invalid content | M | M | M | M | M | M | M | M |

### Stored path starts

| Shape | `O.path_start_byte` | `N.path_start_byte` | `Q.path_start_byte` |
|---|---|---|---|
| Control | H | H | H |
| Absent | M | M | M |
| Explicit null | H | M | M |
| Wrong whole type | M | M | M |
| One wrong element | — | — | — |
| Every element wrong | — | — | — |
| Empty | M | M | M |
| Duplicate key | M | M | M |
| Trailing/invalid content | M | M | M |

The strict stored structs deny unknown fields, require every evidence key and use a present-nullable deserializer instead of ordinary Option defaults. Typed vectors reject bad elements; no filter silently discards them. Single-field path or redundant-start loss, retained identity/name/geometry inconsistencies, summary/list ordering and duplicate rows recompute. Legitimate unqualified nulls, unreported/import IDs and genuinely omitted record kinds remain accepted. The previously accepted coordinated deletion of both redundant path fields is unchanged. The new findings concern source facts the cache correctly preserves, not malformed-cache acceptance.

## Nonblocking Recommendations

Preserve the existing single definition/reference pipeline, indexed binding lookup and record-only semantic widening. The import repair should use semantic grammar roles and shared extra-skipping decisions, with no report-specific correction or additional performance gate. No required repair is hidden in this section.
