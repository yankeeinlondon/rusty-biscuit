---
$schema: feature-review.yaml
ready: false
findings:
  - title: R9-F5 — Cache validation accepts lost and contradictory import evidence
    priority: high
  - title: R10-F1 — Approximate source scanning confuses literals and comment boundaries
    priority: high
  - title: R10-F2 — Single-segment Java on-demand imports are omitted
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-07T15:58:50-07:00
spec: 2026-10-08-scoped-references/spec.md
implemented: true
description: "A **feature** review of `2026-10-08-scoped-references/spec.md`"
feature: 2026-10-08-scoped-references/review-10.md
previous: 2026-10-08-scoped-references/review-9.md
log: tree-hugger/features/2026-10-08-scoped-references/log.md
implemented_by: codex/gpt-6.1-sol
next: 2026-10-08-scoped-references/review-11.md
---

# Scoped References: review 10

The feature is **not production ready**. Four review-9 findings are verified fixed across their required instances and consumers. R9-F5 is only partially fixed: cached imports can still be deleted, fabricated or given incorrect source/boundary metadata. The sweep also found a cache-scanner regression on valid literals/comments and a missing Java on-demand import form. No human decision blocks the repairs.

The specification, plan, original implementation log, review repair log and all nine earlier reviews were reconciled. Review claims were checked against production decisions, declared tests and independent shipped-CLI reproductions. This review changes only review metadata and the requested review document.

## Previous Finding Verification

The five review-9 parents are reconciled below against their body, frontmatter, required repairs, language/coordinate leaves, and the implementation ledger in `log.md`. Four are **verified fixed**; the fifth is **partially fixed**. No required repair acquired or lost an external prerequisite. The ledger contains 415 unique finding rows: five parents and 410 children (304 fixed, 76 clean, 30 unresolved). Its completion report correctly says **partial**, despite the earlier progress summaries. The previous review's `implemented: false` is preserved.

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
|---|---|---|---|---|
| R9-F1 — Comments hide Swift generic constructor calls | .01–.08; every .09 language; .C1–.C5, including conservative targets, ownership, type-argument usages, wrapper/coverage consumers, six cache stages and ID-selected reports | **verified fixed** | The tree-hugger invocation reader uses `next_named_syntax_sibling` before selecting generic arguments. Current L1 executes the complete invocation/negative matrices, commented Swift evidence/cache/report tests, Rust pattern/asserted-eval controls and revision-47 rejection. Expected Call, exact name span, retained Int Usage, candidate C, absent target ID, Unknown and empty established edges are independently asserted. | None of its required instances remains open. No grammar upgrade was needed or occurred. |
| R9-F2 — Type-only imports compete with local value declarations | .01–.08; namespace/require, ordinary `type` name, JS/Rust and all limited-language controls .09–.13; .C1–.C6; ledger commented default/namespace/require, entity-alias and cold-text leaves | **verified fixed** | The tree-hugger binding reader reads anonymous `type` tokens at statement/specifier roles and assigns TYPE only. L1 modifier/collision matrices assert exact local self-call IDs, Yes, two-way edges, no-ID type usages, value-category typeof, mixed dependencies, six cache stages, relation barriers and cold/warm CLI directions. Imports remain present. | New cache-list deletion sibling below concerns validation of stored facts, not unchanged value/type lookup. |
| R9-F3 — Source spellings are mistaken for semantic name identities | Every .01–.16 language/site leaf; .17 distinctions and .18 unsupported languages; .C1–.C6; ledger string hex/continuation/surrogate, escaped eval, bodyless signatures/overloads and encoded binding-witness controls | **verified fixed** | Inspected shared language-specific keys in scope tables, candidates, Rust owner relations, original-name recursion, diagnostics and stored witnesses. L1 `symbols_name_identity` executes 110 source rows through six actual cache stages with independent target/candidate IDs, false-edge absence, coverage, recursion, source spelling and owner expectations; related ID CLI, lost-ID, header and relation tests pass. Rust NFC is separate from ECMA escape decoding; ECMA literal decomposed names remain distinct. | No required identity repair remains. No compiler integration or receiver inference was added. |
| R9-F4 — Import consumers reconstruct binding provenance from names | .01.rs/.02.ts/.03.rs/.ts/.js, .04–.06 controls; .C1–.C5; .S01–.S10 provenance/span leaves, .S11 ordered dependencies, .S12 schema, .S13 raw resolver/capture-order | **verified fixed** | Resolver-selected `Occurrence.import_span` is assigned before raw evidence leaves resolution. Recursion selects the import by its exact span; CLI absent-ID classification uses the same occurrence. Current L1 verifies separate-scope alias controls, import versus local Calls, ordered summaries, strict provenance mutation matrix, six stages, schema 5.1, raw/public evidence agreement and cold/warm reports. | None. The temporary raw-resolver regression documented in the repair log was repaired before its final passing run and remains repaired here. |
| R9-F5 — Cache validation accepts lost and contradictory import evidence | .01.rs/ts/js; all ten producer languages in .02–.08, every .06/.07 coordinate leaf; six .09 empty producers; .10 structural controls; .C1–.C5; .S1 and seven language leaves, .S1.separate, .S2–.S5 | **partially fixed** | Current L1 restored-entry matrices verify strict shapes, source metadata for their enumerated controls, numeric geometry, selected-import/dependency consistency, all capabilities/projections, recomputation then actual Hit and record-only widening. Independent CLI probes confirm grouped deletion and statement-boundary failures, and discover additional role/source/list siblings below. Known-gap tests explicitly assert damaged Hits and therefore prove a defect, not repair. | Carry .02.rs/ts/js/php/py/go/scala, every .07 language, .S1 with seven leaves and .S2. .C1–.C3/.C5 remain incomplete; .C4 is also **partially fixed** because CLI still serves the new malformed facts, notwithstanding its ledger's fixed claim. New children below expand .02/.04/.05. No blocker. |

### Earlier findings and unresolved observations

All earlier reviews were compared by defect class and inventory, including the eight legacy headings in reviews 1 and 2. The table below retains their enumerated scopes. Its evidence prefixed **Historical** comes from review 9 and is not represented as a new independent probe. **Current verification** is this review's rerun of the named permanent matrices plus inspection of the corresponding invariants; earlier passing rows do not close later siblings. Accepted grammar and separate-contract observations remain separate from required release repairs.

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
|---|---|---|---|---|
| `review-1.md — Lexical lookup crosses boundaries that do not establish a binding` | Rust inline modules, nested functions/constants/statics, closure controls, conditional chains/else/following statements and loop controls; TS/JS `with`, direct eval and defaults; occurrences, summaries, recursion, relation lookup and CLI | **verified fixed** | **Historical (review 9):** Review-9 L1: `LEXICAL_BOUNDARY_MATRIX`, boundary summaries/recursion, shared relation lookup and boundary CLI tests pass. Lookup enforces module/item boundaries and conditional endpoints. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-1.md — Named function expressions bind their names to outer declarations` | TS/JS ordinary and generator expression names, defaults/internal calls, final outer calls, unnamed/class/arrow controls; exclusions, placement, summaries, recursion, relation lookup and callers | **verified fixed** | **Historical (review 9):** Review-9 L1: `EXPRESSION_NAME_MATRIX`, expression-local summaries/recursion/relation and CLI callers pass. The shared binding inventory excludes expression names and confines visibility to the expression. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-1.md — Invocation classification loses calls across supported source forms` | Direct/wrapped calls in all 16 languages, TS type-only wrappers, member/generic wrappers, Perl/shell calls, receiver/argument/sequence/conditional/dereference negatives; reports, recursion, terminal-call and CommonJS readers | **verified fixed** | **Historical (review 9):** Review-9 L1: all-language invocation/negative matrices, wrapped binding, CLI, terminal-call and CommonJS tests pass. R6-F1 is outside the earlier enumerated wrappers. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-1.md — Cache validation accepts inconsistent reference evidence` | Wrong/malformed/lost retained target IDs, named/ownerless targets, owner/candidate identity, duplicate/omitted callable regions, structural matrix, summaries/dependencies/semantics, null and narrowed controls | **verified fixed** | **Historical (review 9):** Review-9 L1: structural/reference matrix, retained-evidence tests and two-way summary/semantic validation pass; strict stored types require keys and distinguish nullable values. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-2.md — Parameter defaults incorrectly see body-local var bindings` | Seven callable forms in TS/JS; ordinary/block/destructured/loop `var`; parameter visibility, redeclaration, hoisting, computed/destructured defaults and body eval; let/const/function controls, relation lookup and reports | **verified fixed** | **Historical (review 9):** Review-9 L1: both languages’ callable-form/body-declaration matrix, parameter-environment controls, relation lookup and CLI tests pass. `var_body` isolates body bindings from defaults. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-2.md — Pattern capitalization invents or omits Rust bindings` | Bare/modified capitalized bindings, lowercase constants in patterns/body/guards, paths/tuples/shorthands and irrefutable controls; walker, capture fallback, exclusions, declaration records, summaries, recursion and callers | **verified fixed** | **Historical (review 9):** Review-9 L1: pattern identity/record, summaries, recursion and CLI rows pass. Query repairs also remove Rust path/range captures; actual binding identity precedes record exclusion. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-2.md — Transparent call wrappers incorrectly limit coverage` | Rust parentheses/generics/type paths, all enumerated TS wrappers, JS parentheses and constructors; direct/self/member/value/sequence/conditional/dereference controls, eval, recursion and shared relation scopes | **verified fixed** | **Historical (review 9):** Review-9 L1: callee-coverage and complete/narrowed recursion matrices, shared operand/eval readers pass. Calls to another established declaration still prove No where coverage is complete. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-2.md — Cache validation accepts nonexistent evidence identities and missing callable regions` | Every-language phantom owners, applicable candidates/targets, owned/ownerless targets, Python/Lua empty callable regions, summaries, callable/narrowed entries and null/bodiless controls | **verified fixed** | **Historical (review 9):** Review-9 L1: every-language retained identity mutation tests and empty Python/Lua callable-region cases pass. Missing identities require a compatible omitted kind; regions use retained body facts. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R3-F1 — TypeScript instantiation expressions hide calls and constructor invocations | `.01–.18`, `.C1–.C7`, ledger `.R`: wrapper/member/constructor operands, usages/arguments and all-language controls; classification, candidates, summaries, coverage/recursion, projection/widening, CLI and revision invalidation | **verified fixed** | **Historical (review 9):** Review-9 L1: TS instantiation invocation/coverage/recursion, member candidates, cache narrowing/widening/hits, CLI and stale revision tests pass. Both classifier and operand reader share the instantiation rule. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R3-F2 — Rust associated value calls falsely prove absence of recursion | `.01–.16`, `.C1–.C5`: inherent/trait/enum/generic/wrapped value paths and unrelated initializer controls; function/constructor/usage controls, candidate fallback, coverage, summaries, narrow recursion, semantic widening and callees | **verified fixed** | **Historical (review 9):** Review-9 L1: associated value call, summary and complete/narrowed recursion rows, record-only semantic widening and CLI coverage pass. Unique members and uncertain candidates both check value kinds. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R3-F3 — Cache validation accepts false source facts and misordered or duplicate lists | `.L01–.L16.{region,path,candidates}`; `.S01–.S03.{rs,ts,js}.{duplicate,reverse}`; `.S04–.S10`; `.S09r.{rs,ts,js,go,py,java,cs,c,cpp,swift,scala,php,lua,perl,sh,zsh,omitted}`; `.C1–.C4`; `.M.kind` and eight owner/coverage coordinate leaves | **verified fixed** | **Historical (review 9):** Review-9 L1: all-language source-fact/candidate/region edits, duplicate/reversed summaries, path/start deletion, omitted-record and full leaf matrices pass. Retained geometry, written source paths, list order and summaries are checked. Perl collection was repaired at 19:02 before the earlier 19:14–19:25 cache repair; this cycle preserves that unblocking. Coordinated deletion of both redundant fields remains the review-4-excluded limit. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R3-F4 — Optional and tagged eval calls incorrectly hide lexical declarations | `.01–.07`, `.C1–.C5`: TS/JS optional/tagged/wrapped eval, genuine direct/sequence/member/alias controls; scope barriers, declared bases, summaries/candidates, coverage/recursion, filtering/cache/callers | **verified fixed** | **Historical (review 9):** Review-9 L1: `EVAL_CALL_FORMS`, scope/relation/summary/coverage/recursion, cache and CLI tests pass. Both binding and coverage distinguish optional/tagged calls from direct eval. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R4-F1 — Written-path collection drops qualifiers and can bind rooted names to locals | `.01–.28`, `.C1–.C7`: roots, nested C++ paths/generics, C# alias/conditional/generic paths, PHP nullsafe/namespaces, Perl package/receiver paths and all-language controls; exclusions, lookup/category/stopping, candidates/summaries, recursion, projection/cache/reporting/docs/revision | **verified fixed** | **Historical (review 9):** Review-9 L1: `WRITTEN_PATH_MATRIX`, rooted Rust target/summary/recursion, narrow/wide/warm/projection, text/JSON reports and stale-entry tests pass. Rooted names never fall into local lookup. Java receiver rows are now independently restored too. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R4-F2 — Invocation classification omits calls and mistakes PHP callable creation for execution | `.01–.24`, `.C1–.C6`: rooted calls, C# conditional/generic/delegate chains, PHP placeholder versus unpacking for named/qualified/static/instance/variable forms, variable constructors, Perl code-reference and name-only negatives; coverage, candidates/summaries, filtering/cache/reports/docs/revision | **verified fixed** | **Historical (review 9):** Review-9 L1: every .01–.24 invocation row, placeholder/unpacking negatives, conservative summaries/recursion, cache/projection and report tests pass. The invocation climb handles those roots, conditional members and variable calls. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R5-F1 — Invocation classification still omits constructed and implicit member targets | .01–.22, .C1–.C6; ledger .S1–.S20: Java/Scala/Swift/C++ constructed/generic/implicit targets, all-language negatives, binding/candidates, summaries, coverage/recursion, cache/projection, reports/docs/revision | **verified fixed** | **Historical (review 9):** All required review-5 rows pass current invocation/negative matrices, conservative-result assertions, six-stage cache tests and `refs::constructed_types_and_implicit_members_are_callees_and_callers`. Swift subscripts remain Usage. Scope is closed for those rows; ledger .S12’s additional grammar limitation is evaluated separately below. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R5-F2 — Unreported declaration tokens leak into reference occurrences | .01–.38, .C1–.C6 and .S1–.S20: all declaration sites/controls, constraints/defaults/body reads, exclusions/candidates, ownership/summaries/recursion, filter/cache/CLI/docs/revision | **verified fixed** | **Historical (review 9):** Review-9 `DECLARATION_EXCLUSION_MATRIX` asserts exact missing declaration offsets **and** retained real-read offsets; all rows pass. Shared Python/Scala pattern readers, wildcard controls, cache unreported-declaration stages and CLI suggestions pass. Exclusion precedes candidate selection. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R5-F3 — Receiver-qualified Java construction loses its written target | .01–.10, .C1–.C5, .S1–.S15; R5-X.01–.15 (fixed Python-star/PHP-static siblings plus controls): receiver/generic/anonymous paths, candidates/lookup, reports, cache/projection/docs | **verified fixed** | **Historical (review 9):** Current full-path matrix, conservative Java target assertions, `written_receiver_paths_hold_through_the_cache` and receiver text/JSON reports pass. Exact receiver paths, independent receiver/argument reads, Swift operator exclusion, Python star and PHP static paths are asserted. R5-F1’s generic-classification prerequisite was repaired before these dependent rows. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R5-F4 — Parameter captures turn type and default-value references into declarations | .01–.16, .C1–.C6, .S1–.S3: Python/C# callable forms, actual record lists/types/default reads, PHP constants/defaults and Rust pattern paths, counts/signatures/candidates/selection/cache/provenance/revision | **verified fixed** | **Historical (review 9):** Current parameter-capture matrix, signature tests, cache six-stage exact record/read assertions, CLI ambiguity and revision-43 invalidation pass. Python/C#/PHP/Rust queries field binding names precisely. R5-F4 was repaired before R5-F2 exclusion consumed its definition set. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R6-F1 — Force-unwrapped and null-forgiving invocations disappear from call reports | .01–.18, .C1–.C6: Swift/C# bare/grouped/member/repeated/optional assertions; all-language negatives; operands, spans/paths/candidates, summaries/coverage/recursion, six cache stages, reports/docs/revision | **partially fixed** | **Historical (review 9):** Current invocation/negative/conservative-result, cache-stage and CLI tests pass. Independent CLI probes confirm Calls and preserved receivers/paths. .17a grouped forms work; .17b still fails in the pinned parser. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Every supported required form and consumer is fixed. Literal .17b remains a grammar limitation under the accepted release boundary; no prerequisite was removed. |
| R6-F1.17b — Swift adjacent repeated/mixed assertions | cb!!() and cb?!(); classification, coverage and call reports | **still blocked** | **Historical (review 9):** Independent lint reports missing ! at both calls; cb stays Usage, while (cb!)!() and (cb!)?() are Calls. `swift_adjacent_unwrap_operators_are_a_grammar_limit` passes. Cargo.lock pins Swift 0.7.3; 0.7.1 is the manifest requirement. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Optional grammar work outside the accepted release contract; see Blocked Findings. The failing parser prerequisite existed before repair. |
| R6-F2 — Parameter readers mistake comments for default values | .01–.14, .C1–.C5: eight default-capable languages; C# methods/constructors/local functions, PHP functions/constructors/methods, Scala methods; names/types/arity/defaults/optionality, headers/cache/revision | **verified fixed** | **Historical (review 9):** Current comment-position/signature/header/six-stage-cache tests pass. C# skips extras before/after =; PHP/Scala read default_value; dead Swift helper read removed. Independent g(int x /* b */ = 1, int y = /* a */ 2) record has defaults 1/2, both optional. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | None of the required cases remains failing. |
| R6-F2 ledger siblings — C# params and Swift aliases | Comments before params type/name and after the equals in a Swift type alias | **verified fixed** | **Historical (review 9):** `comments_beside_params_arrays_and_alias_targets_are_skipped` passes; both readers skip extras. Callable matrix checks record lists and arity. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | None. |
| R6-F3 — Contextual syntax markers become false reference suggestions | .01–.11, .C1–.C6: Lua attributes, Java inference/legacy forms, modifiers; exclusions/candidates, ownership, summaries, reports and cache | **verified fixed** | **Historical (review 9):** Review-9 marker matrix, six-stage marker cache and cold/warm CLI tests pass. The R7-F1 Java static-import gate now rejects value-import evidence; Swift precedence markers are role-excluded. Independent static-import/Swift CLI controls confirm this. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Earlier enumerated cases and the R7-F1 recurrence are closed. |
| R6-F3.S1 — PHP class-relative markers | self/static/parent in type/new/instanceof slots; genuine same-spelled method reads | **verified fixed** | **Historical (review 9):** Current marker/cache/CLI rows pass. Independent PHP source retains self::static(), static::static(), parent::static() member Calls and excludes class-relative type/construction operands. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | None. |
| R6-F3.S2 — Bash/Zsh declaration options | Leading -/+ words through --; ordinary commands/arguments/expansions | **verified fixed** | **Historical (review 9):** Both shell marker rows pass. Shared option count partitions options and operands; ordinary commands do not enter that rule. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | None. |
| R6-F3.X1 — Lost initializer/default reads | .01–.17 plus .18: all 16 initializer readers, Java field/local/var/for initializers, TS/JS diagnostics defaults; records/references/cache/CLI | **verified fixed** | **Historical (review 9):** Current INITIALIZER_READ_MATRIX, public diagnostics-reference, cache and CLI tests pass. value is no longer a name field; parameter defaults are retained. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | None of the enumerated cases remains failing. R7-F3 is a separate false-import sibling affecting the same public diagnostics-reference consumer. |
| R6-F3.X2 — Shell bare declaration operands | .01–.05: Bash/Zsh bare/word-shaped operands, expansions/calls, Bash integer control; cache/CLI/docs | **verified fixed** | **Historical (review 9):** Exact-offset, cache and CLI rows pass. Independent Zsh source excludes declaration operands while keeping four expansion reads and echo Call. Docs correctly allow excluded names without records. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | None. |
| R5-F1.S12 — Swift specialized explicit initializer paths | C<Int>.init() and N.C<Int>.init(); qualifiers and conservative Calls/coverage | **still blocked** | **Historical (review 9):** Independent current CLI reports init Call with no path for both expressions. The pinned tree chooses comparison syntax. Accepted source-plus-pinned-grammar boundary predates all repairs. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Optional grammar work outside the required release contract; see Blocked Findings. Prerequisite unchanged before/after repair. |
| Earlier ledger observations — Java inferred-type markers and Lua attributes | Both observations carried into R6-F3.01/.02/.04/.05, including real same-spelled reads and suggestions | **verified fixed** | **Historical (review 9):** Current exact public tests and independent no-import Java/Lua probes pass. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | No unchanged observation remains; static-import variant is also verified fixed under R7-F1. |
| R7-F1 — Syntax-marker exclusions still admit Java import-triggered and Swift precedence tokens | .01–.08a/.08b; .09/.10 controls; .11a–d/.12a–c/.13; every .14 language leaf; .C1–.C6; ledger .S1–.S7, including all .S5 language leaves | **verified fixed** | **Historical (review 9):** Inspected `SyntaxMarkers::nodes`, Java declaration/import gate, Swift role filters and their exclusion before resolution. Full `MARKER_EXCLUSION_MATRIX`, legacy/read/anonymous-region controls, marker cache stages and cold/warm ID-selected reports pass review-8 L1. Independent CLI retains only real Java `var()` and Swift group/type reads. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | No earlier required marker case remains. No blocker changed. |
| R7-F2 — Declaration and import-name exclusions omit unsupported and compound name forms | .01–.15, including .02b, .05b, .09b, .10a–c, .11b, .12b and all eight .15 selectors; every .16/.17 clean-language leaf; .C1–.C7; ledger .S1–.S8 and .X1 | **verified fixed** | **Historical (review 9):** Shared `declared_name_tokens` is used by recorded-name ranges and the unreported-name inventory before lookup. Exact `DECLARATION_EXCLUSION_MATRIX` and every-shape six-stage cache pass; .12a preserves the genuine class ID, .12b has no fabricated forward/reverse edges. All Swift selectors, import reports, public diagnostics references and C/C++ elaborated-use record/read tests pass. Independent TS and Swift CLI controls confirm missing declaration tokens and retained actual reads. Java import masking was removed before these exclusions were verified; both decisions now pass together. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | No enumerated declaration repair remains. R8-F1's comment-bearing sibling is now verified fixed. R9-F3 concerns new equivalent-name spellings, not unchanged .01–.15 reproductions. |
| R7-F3 — Java qualified references are misclassified as imports and disappear | .01–.13 including named annotation arguments/deep paths and genuine-import controls; every .14 reader; .C1–.C7; ledger .S1–.S7 and .X1–.X3 | **verified fixed** | **Historical (review 9):** Java capture is structurally inside `import_declaration`; all-query predicate guard passes. Full Java matrix/public diagnostics inventory, exact candidates, six cache stages, module/deep-annotation ID reports and imports tests pass. Other readers' binding/source-role matrices, alias dependencies, export exclusions, Go raw/blank/dot imports and revision-45 invalidation pass. Independent Java CLI has genuine imports only and preserves qualified annotation reads. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Original enumerated cases are closed. R8-F1/R8-F2's later import variants are now fixed. R9-F2–F5 identify further category, identity, provenance and stored-import siblings; a broad class-closure claim remains unsupported. |
| R7-OBS.1 — Python future imports in the separate type-owner relation reader | Future-import binding in relation/containment lookup, separate from conservative occurrence evidence | **unresolved** | **Historical (review 9):** The repair ledger explicitly defers this separate resolver consumer. Future-import occurrence exclusion and import facts pass the current Python matrix; the lexical relation reader still lacks the future-import arm. Accepted spec preserves containment/export contracts. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Nonblocking separate-contract work; retained below, not waived or claimed repaired. |
| R7-OBS.2 — Plain import reports reconstruct some syntax imperfectly | Language-specific reconstruction by `hug imports` text renderer; underlying binding records/JSON and required refs reports | **unresolved** | **Historical (review 9):** Ledger deferral remains. Inspected the separate import renderer; required refs text/JSON and exact import-record tests pass. Syntax reconstruction is outside the refs report contract. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Nonblocking separate display work; retained below. |
| R8-F1 — Comments corrupt import identities, qualifiers and binding exclusions | .01, .01.export, .02; .03, .04a/.04b; .05.star/.underscore/.given; .06.mixed/.given; .07.simple/.qualified; .08; .09.ts/.js; every .10/.11 language leaf; ledger .S1/.S2; .C1–.C8 | **verified fixed** | **Historical (review 9):** Current `COMMENTED_IMPORT_MATRIX`, shared `for_each_import_row`/`assert_commented_import_row` assertions and exact metadata/read controls pass in the 1,471-test run. Checked comment-free and edited originals/qualifiers, exclusions before candidate selection, genuine reads, dependencies, Unknown recursion, six cache stages, public diagnostics, relation barriers, export controls and cold/warm CLI imports/refs JSON/text. Revision is 47; revision-46 entries miss. The repair skips extras before selecting entity targets and Scala prefix/name nodes. No blocker existed or was removed. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | All required instances/consumers are fixed. Swift generic adjacency R9-F1 is a newly discovered sibling at another reader, not this import reproduction. |
| R8-F2 — Scala import collection omits rename and operator forms | .01.single/.multiple/.nested; .02.escaped/.original; .03.direct/.multiple/.braced; .04.arrow/.as including mixed aliases; .05.direct including rooted qualifier, .05.braced/.wildcard; .06; every .07 language/control leaf; ledger .S1; .C1–.C6 | **verified fixed** | **Historical (review 9):** Current Scala query captures unbraced renames and operator names; reader uses the clause's path context and includes operator qualifiers. Public import matrix asserts exact binding/original/source, hidden-alias absence and retained reads for all rows; all six cache stages, diagnostics, independent relation barrier controls and cold/warm CLI imports/refs pass. R8-F1's comment prerequisite and this syntax repair coexist before verification; neither accepted grammar boundary changed. Reviewed the log's entire 77-child combined inventory, including clean controls and added siblings, rather than just its completion flag. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | All required instances/consumers are fixed. Quoted/escaped semantic identity R9-F3 is a new sibling, not the fixed Scala naming alternatives. |
| review-6.md — C++ parenthesized-allocation grammar observation | `new (C)` and corresponding invocation/coverage consumers | **still blocked** | **Historical (review 9):** Review 6 explicitly classified this as optional: clang accepts the source, but the pinned parser has missing type nodes. Current grammar/query versions and accepted pinned-grammar prerequisite are unchanged; no repair removed it. **Current:** corresponding named L1 regressions rerun and invariant inspected; no new historical compiler/CLI probe claimed. | Optional grammar observation, carried under Blocked Findings; no required release repair or human decision. |

## Unblocked Findings

### R9-F5 — High — Cache validation accepts lost and contradictory import evidence

**Defect class:** cached import records are checked for field types and token geometry without proving their complete binding inventory, import grammar role, enclosing statement or all source metadata, so public results can lose imports or publish invented facts.

This is a **partial repair of the carried finding**, with newly discovered siblings explicitly identified below. In tree-hugger, [validate_imports](../../lib/src/cache/stored_evidence.rs:308) now checks strict shapes before every capability is served. However, [uncovered_source](../../lib/src/cache/import_source.rs:463) masks an entire retained statement, hiding its missing sibling imports; [validate_metadata](../../lib/src/cache/import_source.rs:129) sometimes skips source comparison or returns success without establishing an import keyword/role. Valid coordinates and containment do not prove a statement's actual syntax boundary.

The original controls now reject erased ordinary imports, wrong aliases/originals, impossible IDs, invalid coordinates, duplicate records and malformed JSON. Those repairs must remain. Required residual reproductions are unchanged: retain D and delete E from a grouped statement, or extend a stored statement into the following function using matching source coordinates. The new table distinguishes those from later source-role siblings.

To reproduce, write the source shown below, run `hug --verbose --json-full symbols input.<extension>` twice with an isolated `TMPDIR`, copy the one JSON entry under that temporary directory's `tree-hugger/cache/hug/`, apply the listed edit to the copy, and rerun the same command. Restore the exact original entry before every edit. Unedited controls must Hit. A defective edit must Miss and restore the independently listed source facts; warm equality to the damaged result is insufficient.

```typescript
import { f as D, g as E } from 'a'; function f(){}
```

Delete only E's element in `imports`. The CLI still reports an actual Hit, zero parses and only D. Both D→f and E→g from a are required. The same failure occurs with default-plus-named and default-plus-namespace clauses, not just brace selectors.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R9-F5.02.rs / R9-F5.S1.rs | `use a::{f as D,g as E}; fn f(){}`; delete E | Hit, one import | Miss, D/f/a and E/g/a restored | Current CLI `group-delete.rs`; permanent public-API characterization rerun |
| R9-F5.02.ts / R9-F5.S1.ts | `import {f as D,g as E} from 'a'`; delete E | Hit, one import | Miss, both imports | Current CLI `group-delete.ts`; permanent characterization |
| R9-F5.02.js / R9-F5.S1.js | Same JS clause | Hit, one import | Miss, both imports | `group-delete.js`; permanent characterization |
| R9-F5.02.py / R9-F5.S1.py | `from a import f as D,g as E` | Hit, one import | Miss, both imports | `group-delete.py`; permanent characterization |
| R9-F5.02.go / R9-F5.S1.go | `import (D "a/f"; E "b/g")` | Hit, one import | Miss, D/f/a/f and E/g/b/g | `group-delete.go`; permanent characterization |
| R9-F5.02.scala / R9-F5.S1.scala | `import a.{f => D,g => E}` | Hit, one import | Miss, D/f/a.f and E/g/a.g | `group-delete.scala`; permanent characterization |
| R9-F5.02.php / R9-F5.S1.php | `use A\{F as D,G as E}` | Hit, one import | Miss, D/F/A\F and E/G/A\G | `group-delete.php`; permanent characterization |
| R9-F5.R10.group.ts / new sibling | Default-plus-named `import D,{g as E} from 'a'`; delete E | Hit, one import | Miss, D and E restored | `group-variant.ts` |
| R9-F5.R10.group.js / new sibling | Default-plus-namespace `import D,* as E from 'a'`; delete E | Hit, one import | Miss, D and E restored | `group-variant.js` |
| R9-F5.R10.group.py / new sibling | `import a as D,b as E`; delete E | Hit, one import | Miss, both modules/aliases | `group-variant.py` |
| R9-F5.R10.group.php / new sibling | `use A\F as D,B\G as E`; delete E | Hit, one import | Miss, both sources/aliases | `group-variant.php` |
| R9-F5.R10.group.scala / new sibling | `import a.f,b.g`; delete g | Hit, one import | Miss, f/a.f and g/b.g | `group-variant.scala` |
| R9-F5.R10.type.ts / new sibling of .02.ts | `import type {F} from 'a'`, with/without comments; replace imports by [] | Hit, no imports | Miss, F/source a restored; preserve TYPE-only facts | Both named-type deletion probes; `proves_import_binding` omits `type` followed by `{` |
| R9-F5.R10.type.controls / clean siblings | Specifier `type F`, default type F, namespace type F, require type F; whole-list deletion | Miss, one correct import | Keep rejection and recomputation | All four modifier-form CLI controls |
| R9-F5.R10.source.php / new sibling of .04.php | Second source in ungrouped aliased/unaliased PHP clauses; absent/null/empty/changed | All eight edits Hit, bad source retained | Miss; second source B\G restored, preserving actual alias/original | `php-second-source.*` and `php-second-unaliased.*`; PHP branch skips statements containing a comma outside a group |
| R9-F5.R10.source.java / new sibling of .04.java | `import p.C` and `import static C.f`; source absent/null/empty/changed | All eight edits Hit, bad source retained | Miss; source p or C restored | `java-single.*` / `java-static.*`; Java branch incorrectly requires a dot within the source prefix itself |
| R9-F5.R10.source.controls / clean siblings | Grouped Rust/TS/Go; multi Python/Scala; deep Java package; same four source edits | All Miss/recompute | Preserve exact source/alias/original | 24 independent metadata edits; original ten-producer permanent matrix also passes |
| R9-F5.R10.role.{rs,ts,js,java,cs,php,py,go,swift} / new sibling of .05 | Append an import named f whose exact name span is the real function f token, with null source/alias/original/ID and a name-sized statement span | Hit, invented second import | Miss; only the actual original import remains | Nine shipped-CLI `invented-role` probes; the original valid import remains unchanged |
| R9-F5.R10.role.scala / clean sibling | Same invented-role edit | Miss; original import restored | Preserve rejection; do not relax geometry/source checks | `invented-role.scala` |
| R9-F5.07.{rs,ts,js,java,php,py,go} / R9-F5.S2 | Extend statement end and matching end point to include following declaration | Hit, wrong statement retained | Miss; actual grammar statement restored | Seven `boundary-end` CLI probes; TS permanent characterization |
| R9-F5.07.{cs,scala,swift} / R9-F5.S2 clean end controls | Same endpoint extension | Miss/recompute | Keep rejection | Three `boundary-end` probes; these controls do not close start-boundary siblings |
| R9-F5.07.{rs,ts,js,java,cs,php,py,go,swift} / R9-F5.S2 start siblings | Prefix source with a newline; extend start/start point to byte 0/line 1/column 1 | Hit, extra source included | Miss; actual import keyword boundary | Nine `boundary-start` probes; one coherent span-object edit |
| R9-F5.07.scala / R9-F5.S2 clean start control | Same start extension | Miss/recompute | Keep rejection | `boundary-start.scala`; boundary-role inventory still incomplete, so retain the parent's required check |
| R9-F5.S1.separate / clean | Remove one complete separate statement's import | Miss; both restored, later Hit | Preserve this repaired case | Permanent public-API test rerun |
| R9-F5.01/.03–.06/.08–.10, .S3/.S5 / earlier controls | Strict JSON, selected dependencies, genuine nullable/source-less imports, every numeric leaf, empty producer languages and prior CLI schema/revision cells | Required earlier controls pass | Preserve them while closing the new role/source siblings | Declared public `symbols_cache_imports`, strict evidence/provenance and CLI robustness matrices in current L1 |

Every language suffix above is a separate required child. Retain all original .07 coordinate leaves, .S1 children and .S2; passing scalar geometry cells does not establish syntax boundaries. Java/C#/Swift have no equivalent grouped clause under this inventory: the ordinary separate-statement and single import controls remain applicable. C/C++/Lua/Perl/Bash/Zsh have no import captures; their populated-entry rejection remains required and passes the public matrix.

| Child ID | Decision/consumer | Required remaining result |
|---|---|---|
| R9-F5.C1 | Stored reader and source consistency | Prove every retained record's import role, complete per-statement inventory and actual statement boundaries. Close the Java/PHP source comparison gaps. Never equate a parser branch's inability to prove a fact with permission to accept it. Preserve legitimate absent/null forms. |
| R9-F5.C2 | Capabilities, kinds and projection | Validate before Declarations-only, References-only, Occurrences, Semantics and projected queries are served. `StoredEntry` already calls the shared validator before projection; fix that validator rather than introducing a report-only check. Permanent capability/omitted-kind controls pass for repaired mutations; these new mutations still enter the common reader. |
| R9-F5.C3 | Lookup, recomputation and semantic widening | Every defective edit must Miss, restore exact public imports/occurrences/dependencies/recursion, and subsequently Hit. Valid record-only semantic widening must still parse zero files. Do not claim characterization tests asserting bad Hits as regression protection for the intended behavior. |
| R9-F5.C4 | CLI imports, full indexes and refs | Incorrect metadata and invented imports are directly visible in cached full indexes. The CLI import collector and refs/recursion consumers all read this shared entry. Verify imports JSON/text and ID-selected default/callers/callees after repair; retain selected import provenance and source spellings. |
| R9-F5.C5 | Complete matrix and maintenance | Replace both known-gap characterizations with correct-result assertions; add every new child above to the declared L1 matrix. Update cache docs and reader comments; the claim that producer checks reject meaningful metadata loss is still too broad for these forms. Invalidate changed stored evidence. |

**Required repair:** close every carried residual and new sibling above, preserving all clean controls. No missing grammar, human decision or accepted coordinated-forgery exception blocks this work. Appending one impossible import, deleting one member or changing one source field leaves the other source/dependency facts intact; these are checkable contradictions, not the previously excluded deletion of every witness of one fact.

### R10-F1 — High — Approximate source scanning confuses literals and comment boundaries

**Defect class:** a cache-only tokenizer approximates language lexical rules, treating literal contents as imports and swallowing real imports under the wrong comment-nesting rule, causing both unnecessary reanalysis and acceptance of lost required results.

This is a **repair regression on new inputs** introduced by the review-9 cache validator, related to R9-F5 and its `.S4` literal controls. In tree-hugger, [tokens](../../lib/src/cache/import_source.rs:14) permits regex literals only after a small preceding-token list, does not recognize Swift extended-string or C# raw-string delimiters, and nests block comments for every language. The real grammar already distinguishes these contexts. Earlier regex-after-equals, Rust raw-string and triple-string controls still pass; they did not cover these decisions.

```javascript
function f() { if (true) /import D/.test(''); }
```

An unmodified second and third full-index run both Miss and parse again, although the file has zero imports and no syntax errors. The spec requires actual Hits with zero parses, so correct recomputed output does not make this clean.

```javascript
/* /* */ import { f as D } from 'a'; function f(){}
```

JavaScript closes this non-nesting comment at the first `*/`. The source has one real import. Clear only the tool-produced cache's imports list: the validator treats the outer comment as still open and serves [] as Hit. TypeScript, Java, C#, Go and PHP have the same mismatch. Genuine nested Rust/Scala/Swift controls must retain their different rule.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R10-F1.01.{ts,js} / regex context | Regex after control-header `)` (if/while/for), else/do, throw/yield/await/typeof/void/delete, default export, arithmetic/shift/relational/bitwise/logical operators and unary plus/minus/bitwise NOT | Unchanged entry Misses; parses each run; zero actual imports | Actual Hit/zero parses; contents never import evidence | Current paired-language control/keyword/operator sweeps; original if/while/else sources have syntax=[] |
| R10-F1.02.{ts,js} / regex controls | Regex after assignment, return, closed block, case colon, argument/array/conditional slots, null coalescing, logical NOT and comparison operators ending in equals; ordinary string/template | Hit; zero imports | Preserve genuine Hits | Current literal/regex control probes; shared ECMA branch invariant plus permanent equals/division controls |
| R10-F1.03.swift / extended string | `#" quote " import A.B "#`, double-# counterpart, extended multiline string containing an ordinary triple-quote sequence | Miss/reparse; zero imports | Hit/zero parses; respect matching # delimiters | Three current raw-string probes, all grammar syntax=[] |
| R10-F1.04.swift / controls | Ordinary and ordinary multiline string containing import text | Hit; zero imports | Preserve | Two current controls, syntax=[] |
| R10-F1.05.cs / raw string | `class C { string S = """ quote " using A.B; """; }` | Miss/reparse; zero imports | Hit/zero parses; match full quote-run delimiter | Current three-quote raw-string probe, syntax=[] |
| R10-F1.06.cs / controls | Four-quote delimiter with triple-quote contents; verbatim doubled quotes; interpolated string | Hit; zero imports | Preserve all three; four-quote control must remain in delimiter sweep | Current C# controls, syntax=[] |
| R10-F1.07.{ts,js,java,cs,go,php} / non-nesting block comments | Import after `/* /* */`; unedited control then imports=[] | Valid control Hit; edited list incorrectly Hit/zero imports | Preserve control; reject edit and restore one actual import | Six deletion probes plus six separate unmodified-source controls |
| R10-F1.08.{rs,scala,swift} / nested comment controls | `/* /* */ */` then real import; imports=[] | Miss/recompute one import | Preserve language-specific nesting | Three current deleted-list probes |
| R10-F1.09.rs / opaque controls | Raw strings with # delimiters, non-ASCII character literal; permanent macro/lifetime cases | Hit; import-like text is opaque | Preserve literal/macro opacity and real imports outside it | Current raw/char probes and declared permanent opaque-source corpus |
| R10-F1.10.java / literal control | Text block containing import text | Hit/zero imports | Preserve | Current text-block probe, syntax=[] |
| R10-F1.11.go / literal control | Backtick string containing an import | Hit/zero imports | Preserve | Current raw-string probe, syntax=[] |
| R10-F1.12.scala / literal control | Triple-quoted string containing import text | Hit/zero imports | Preserve | Current triple-string probe, syntax=[] |
| R10-F1.13.py / literal/comment controls | Ordinary, triple and f-string contents; Python # comments | Hit/zero imports; # handling confined to appropriate language branch | Preserve | Current three literal controls; precise `tokens` language branch and permanent Python controls |
| R10-F1.14.php / literal/comment controls | Heredoc/nowdoc, # and non-nesting block comments | Heredoc/nowdoc controls Hit; block-comment deletion fails .07.php | Preserve heredoc/nowdoc/# opacity; restore non-nesting block-comment rule | Current literal probes, syntax=[]; shared PHP-only # branch; .07.php reproduction |
| R10-F1.15.{c,cpp,lua,pl,sh,zsh} / no-import consumers | Populated cached imports versus genuinely empty producers | Populated entries rejected; genuine empty inventories preserved | Clean: no import-detection lexer arm exists for these six; do not add unsupported import inference | `validate_imports` rejects populated lists before token roles; `proves_import_binding` has no arm; public six-language producer tests rerun |

The regex child requires both languages and every listed context. The operator sweep ran this exact template for each entry, independently expecting zero imports and a warm Hit:

```javascript
function f(x) { return x OP /import D/; }
// Failing OP: + - * / % ** << >> >>> < > & | ^ && ||
// Clean OP: <= >= == != === !== ??
// Unary failing: + - ~ void typeof delete
// Unary clean: !
// Also failing: export default /import D/;
```

Argument, array, conditional, case, assignment, return and closed-block contexts remain clean. This table is a lexical-rule inventory, not a proposal to keep extending a preceding-token heuristic one keyword per review.

| Child ID | Decision/consumer | Required result |
|---|---|---|
| R10-F1.C1 | Token collection and lexical category | Use accurate language-specific literal delimiters, regex context and comment rules, or retained grammar-derived evidence. Do not use text inside literals as source binding proof. |
| R10-F1.C2 | Positive inventory, missing-inventory detection and metadata | Feed correct facts into `proves_import_binding`, `uncovered_source`, `absent_statement_is_produced` and `validate_metadata`; all four reuse the tokenizer. Preserve both required source bindings and genuine opaque text. |
| R10-F1.C3 | Shared cache serving, projections and widening | Verify cold Miss, repeated unchanged Hits/zero parses, edited Miss/recompute and subsequent Hit for every affected row. No report-specific workaround; validation precedes every capability/kind projection. |
| R10-F1.C4 | Public summaries/semantics and CLI reports | Recomputed public imports, source spans, selected-import provenance, dependencies and recursion must match the real source. Check full indexes/imports plus refs directions in cold/warm text/JSON; preserve anonymous regions and unsupported-language Unknown. |
| R10-F1.C5 | Tests/docs/comments/revision | Register the complete two-sided lexical corpus in live L1; assert intended Hits/recomputed facts, not current bad behavior. Update cache lexical claims and extractor invalidation. Existing L2 report coverage remains applicable; no keyboard requirement is added. |

**Required repair:** close .01/.03/.05/.07 and all their leaves, preserve .02/.04/.06/.08–.15, verify .C1–.C5. The new source scanner is the regression; do not remove valid supported syntax or reclassify these error-free sources as grammar limitations.

### R10-F2 — Medium — Single-segment Java on-demand imports are omitted

**Defect class:** import collection and metadata readers enumerate qualified-path nodes but omit the simple-name alternative of the same grammar role, losing required bindings and exposing import tokens as reference suggestions.

This is a **newly discovered sibling** of R7-F3's genuine-import inventory and R8-F2's omitted import syntax alternatives. In tree-hugger, the [Java import query](../../lib/queries/vendor/java/locals.scm) captures only `import_declaration (scoped_identifier ...)`; [extract_java_import_metadata](../../lib/src/file/tree_file.rs:824), which builds the public import record, likewise reads only scoped_identifier. The pinned parser represents the target of `import p.*;` as an identifier. No grammar upgrade is necessary.

```java
import p.*;
class p {}
```

The shipped CLI reports no import even without caching. The imported path token p becomes a NameOnly Usage suggesting the local class p. A deeper `import p.q.*; class q {}` correctly reports one on-demand import and excludes its final bound token q, while retaining the independent qualifier p read. `hug imports` JSON/text also omits the simple-path import. The new cache validator sees an import in the source and repeatedly rejects the tool's own empty inventory, amplifying the collector gap.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R10-F2.01.java / ordinary on-demand | `import p.*; class p {}` | imports=[]; p NameOnly Usage; warm Miss | Import name/alias p, source p, original `*`; no occurrence at its bound token; actual warm Hit | Current full-index, imports JSON/text, reference evidence and CLI refs probes; syntax=[] |
| R10-F2.02.java / static on-demand | `import static p.*; class p {}` | Same omission/false suggestion/warm Miss | Same p/* metadata and token exclusion under the existing on-demand record convention | Current ordinary/static paired index/imports/refs probes; syntax=[] |
| R10-F2.03.java / commented simple path | `import p /*c*/ . /*d*/ *; class W {}` | imports=[]; warm Miss | One correctly located import, written source p; no comment-derived name | Current commented import index/JSON/text/lint probe |
| R10-F2.04.java / clean alternatives | `import p.q.*`, `import static p.C.*`, `import p.C` | Correct q/C records and warm Hits | Preserve final binding, metadata, independent qualifiers and exclusions | Current three controls; q class has no occurrence at import q; ordinary p.C source p remains required under R9-F5 metadata sibling |
| R10-F2.05.rs / on-demand parity | `use a::*`, `use a::{*}` | a/source a/original *; Hit | Preserve both | Current two public-index/import-report controls |
| R10-F2.06.scala / on-demand parity | `import a._`, `import a.*`, `import a.{_}` | a/source a/original *; Hit | Preserve all three | Current three controls |
| R10-F2.07.py / on-demand parity | `from a import *` | a/source a/original *; Hit | Preserve | Current control |
| R10-F2.08.go / on-demand parity | `import . "a"` | a/source a/original *; Hit | Preserve | Current control |
| R10-F2.09.{ts,js} / namespace parity | `import * as p from 'a'` | p/source a/original *; Hit | Preserve each alias and uncertainty boundary | Current two controls |
| R10-F2.10.cs / static simple name | `using static C` | C/source C; Hit | Preserve actual import | Current control |
| R10-F2.11.{php,swift} / simple-name parity | `use A`; `import A` | A/source A; Hit | Preserve each real import | Current two controls |
| R10-F2.12.{c,cpp,lua,pl,sh,zsh} / unsupported import collectors | Effective query inventory | No import captures | Clean for this class: no corresponding capture-shape decision; conservative language evidence remains | All-query inventory and six-producer L1 controls; do not infer additional imports |

| Child ID | Decision/consumer | Required result |
|---|---|---|
| R10-F2.C1 | Query, capture alternatives and metadata | Capture the simple-name on-demand target within the actual import node, read its source and wildcard metadata, preserve ordinary/static/deep/comment controls, and update vendored-query provenance. Avoid restoring broad qualified-name captures outside imports. |
| R10-F2.C2 | Exclusions, real reads, candidates and summaries | Exclude only the final captured binding token before candidate selection; preserve true qualifier/type/body reads. Keep Java NameOnly/Unresolved and Limited unsupported_language; no established edges or fabricated recursion proof. |
| R10-F2.C3 | Public diagnostics and import reports | Verify imported_symbols, complete imports records, referenced_symbols and imports JSON/text for all three defective forms; same-named declarations must receive no false suggestion from the imported token. |
| R10-F2.C4 | Filtering and six cache stages | Assert independently expected imports/token absence in narrow Miss, narrow Hit, zero-parse semantic widening, kind widening, complete Hit and projected Hit; preserve Unknown recursion and correct conservative candidates elsewhere. |
| R10-F2.C5 | CLI refs and maintenance | ID-selected default reports must omit the false token suggestion, with callers/callees still excluding Usage. Verify cold/warm JSON/text, docs, query provenance and revision invalidation in declared L1 targets. Preserve current L2 rendering. |

**Required repair:** close .01–.03, verify .C1–.C5 and preserve every clean parity/control row. This is a source collection defect with downstream cache/report effects, not a reason to relax rejection of missing cached imports.

## Blocked Findings

No **required** release repair is blocked. No human decision is requested.

- **R6-F1.17b — Swift adjacent repeated/mixed assertions:** **still blocked** by the unchanged pinned parser for `cb!!()`/`cb?!()`. Supported grouped forms and the grammar-limit test pass. It existed before the repair and no prerequisite was removed.
- **R5-F1.S12 — Swift specialized explicit initializer paths:** **still blocked** by comparison parsing of `C<Int>.init()`/`N.C<Int>.init()`. No pinned grammar change removed this prerequisite. Supported generic constructors, including R9-F1's comments, remain required and repaired.
- **review-6.md — C++ parenthesized-allocation grammar observation:** **still blocked**, optional; `new (C)` lacks the parser's expected type nodes. Query/grammar prerequisite unchanged.

The accepted source-plus-pinned-grammar, file-local, Rust/TypeScript/JavaScript resolution and conservative-other-language boundaries are recorded in [the original implementation log](implementation-log.md:365). They preceded these repairs and are not reopened. The historical Perl qualifier prerequisite was repaired before the dependent R3-F3 cache repair; its unblocking remains recorded in the previous-finding table.

## Nonblocking Carried Observations

- **R7-OBS.1 — Python future imports in the separate type-owner relation reader:** **unresolved**, still separate-contract work. Occurrence/import facts pass; the relation reader's missing future-import arm remains. The spec preserves that separate containment contract. No prerequisite changed.
- **R7-OBS.2 — Plain import syntax reconstruction:** **unresolved**, separate display work. Underlying binding JSON and required refs rendering pass their earlier cases. This does not excuse omitted Java bindings or bad stored metadata, which are required findings above.

## Recurrence

`recurrence: true`. Every earlier review's classes and instance inventories were compared, including legacy Findings sections; earlier fixed cases and later siblings remain distinct.

- **R9-F5** carries an **unchanged failing reproduction** of its grouped deletion and statement-boundary repairs. This repeats the stored-evidence class of review 1's “Cache validation accepts inconsistent reference evidence,” review 2's “Cache validation accepts nonexistent evidence identities and missing callable regions,” and R3-F3. New `R9-F5.R10.*` children are **newly discovered siblings**: mixed import clauses, type-only named list deletion, shallow Java/later PHP source metadata and invented import roles. Missed decisions are complete capture inventory, source role, metadata and producer boundary; missed consumers include every cache capability, projected indexes, imports reports, provenance and record-only semantics. Correct earlier malformed-JSON/geometry controls do not close these siblings.
- **R10-F1** is a **repair regression on new input shapes** from R9-F5's new source scanner. It repeats the source-role/exclusion class seen in R7-F3 and the earlier requirement to keep strings/comments out of reference evidence. `.S4`'s equals-regex/raw/triple controls remain passing; missing decisions are regex context, extended literal delimiters and per-language comment nesting. Both false refusal of valid entries and false acceptance of deleted imports must be repaired together across the four tokenizer consumers.
- **R10-F2** is a **newly discovered sibling**, not a failed rerun of the earlier deep Java import cases. It repeats R7-F3's import capture/source-role inventory and R8-F2's omitted grammar alternatives. Missed decisions are the direct identifier alternative, wildcard metadata, import-token exclusion and source/cache consistency; downstream public diagnostics, candidate suggestions, imported records and CLI reports are required consumers. No evidence attributes this source omission to the review-9 repair.

## Requirement Verification and Test Levels

| Requirement | Strongest applicable verification | Assessment |
|---|---|---|
| Scopes, declaration order/hoisting, type/value categories, lexical name identity, patterns and aliases | L1 public API, six-stage cache and hermetic shipped-CLI matrices | Earlier required inventories including R9-F1–F4 pass; new Java import collector sibling remains. |
| Calls versus usages, wrappers, qualifiers, owners, candidates and source spans | L1 exact positive/negative source matrices, summaries and ID reports | Previous required shapes pass; R10-F2 has a false suggestion. No new receiver inference requirement. |
| Direct recursion Yes/No/Unknown, ordered relationships, selected import provenance and declaration filtering | L1 complete/narrowed summaries, public cache widening, geometry/provenance corruption and CLI | R9-F2–F4 repairs verified; stored import consistency remains partially repaired under R9-F5. |
| Valid warm Hits and zero-parse projection/semantic widening; malformed entries recompute | L1 actual lookup/work counters plus independent shipped CLI mutation probes | R9-F5 and R10-F1 remain gaps. Correct fresh output alone does not satisfy the Hit contract. |
| CLI selectors, ambiguity, repeated rows, both directions, JSON/text and file-local empty boundary | L1 live declared CLI suite | Earlier inventory passes; new Java false suggestions/import omissions remain. |
| Grouping, styling, wrapping and report boundaries in a real terminal | L2 tmux capture through live area recipe | Current run passed, backend run=1/skip=0/panic=0. Harness retains background spawning and did not request focus. |
| Keyboard, hotkeys, paste/IME/mouse | No such feature requirement | L3 unnecessary. |

Current verification:

- `just test -j 4` in tree-hugger: **1,540 passed, 0 skipped**, 248.691 seconds; nextest run `9c05f794-3f79-4223-b5df-7bfbb67a303a`. Known-gap tests were counted as executed tests, **not** repaired requirements.
- `just lint`: **passed**, library and CLI all-target Clippy; no formatter.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_declarations_in_tmux`: **1 passed**, 5.771 seconds; **tmux run=1, skip=0, panic=0**. 403 unrelated tests were filtered out, not claimed executed.
- Root `just check-tier-coverage tree-hugger`: **zero stranded tests**. New identity/import matrix modules are declared by library `tests/l1/main.rs`; CLI tests are declared by `tests/symbols/main.rs`. Ordinary test names select L1; terminal-tests is enabled by the live L2 recipe and declared CI feature configuration.
- **263 independent CLI probe rows** across ten scratch sweeps, with repeated source controls counted as rows rather than unique fixtures. Every mutation restores a tool-produced cache entry, and every control reports its actual lookup. Findings state expected metadata/token absence independently. Literal sources with lint evidence have empty syntax diagnostics. The failed initial Java wildcard mutation setup was corrected into a separate source-collection reproduction; no result from that aborted sweep is claimed.
- Scratch probes are evidence, not permanent tests. They are retained locally under `/tmp/scoped-review10-evidence/`; the report includes their source shapes/edits/results so it does not depend on that temporary directory surviving. No production or test source changed in this review. Cross-OS execution belongs to CI and does not determine readiness.

## Input Robustness Matrix

The changed format is the persistent **JSON cache**. There is no new manifest, lockfile or configuration format. The strict occurrence/coverage/provenance matrices remain selected by L1 and pass; all newly discovered import-reader issues are titled and listed in frontmatter above.

H = correct Hit; M = correct Miss/recompute; H! = defective acceptance where M is required; — = shape does not apply. Optional columns allow H for **independently valid absent/null producer states**, and require M when a written fact is lost. The absence/null distinction is therefore based on the producer contract and retained evidence, never a blanket default. Every numeric column also tests zero and out-of-source values; genuine byte zero remains H, one-based point zero is M.

The declared `symbols_cache_imports` JSON corpus starts from tool-produced controls in Rust, TypeScript, JavaScript, Java, C#, PHP, Python, Go, Scala and Swift, plus grouped/generic producer alternatives. Each cell restores its entry, queries the public result, asserts independently expected metadata/coordinates, then verifies warm Hit and zero-parse semantic widening. All ten producer languages use the same stored field reader. Public no-import controls additionally cover C/C++/Lua/Perl/Bash/Zsh. The existing prior occurrence/coverage/span/path matrices cover their full field/leaf columns; selected-import span and repeated start keys have their own strict matrix under R9-F4.

### Selected-import provenance

These fields apply to each occurrence in the JSON cache. `import_span` and its repeated start key are required even for a local call whose genuine value is null; nested leaves apply only to an import-backed occurrence. The restored-entry public matrix covers every field in Rust, TypeScript and JavaScript, with local-shadow and wrong-scope-import controls. Other languages never select an import binding under the accepted conservative resolver; their null witness keys still deserialize strictly.

| Shape | `O.import_span` | `O.import_start_byte` | `O.import_span.start_byte` | `O.import_span.end_byte` | `O.import_span.start` | `O.import_span.end` | `O.import_span.start.line` | `O.import_span.start.column` | `O.import_span.end.line` | `O.import_span.end.column` |
|---|---|---|---|---|---|---|---|---|---|---|
| Control | H | H | H | H | H | H | H | H | H | H |
| Absent | M | M | M | M | M | M | M | M | M | M |
| Explicit null | H genuine local; M bound import | H genuine local; M bound import | M | M | M | M | M | M | M | M |
| Wrong whole type | M | M | M | M | M | M | M | M | M | M |
| One wrong element | — | — | — | — | — | — | — | — | — | — |
| Every element wrong | — | — | — | — | — | — | — | — | — | — |
| Empty | M | M | M | M | M | M | M | M | M | M |
| Duplicate key | M | M | M | M | M | M | M | M | M | M |
| Trailing/invalid content | M | M | M | M | M | M | M | M | M | M |

### Import list, binding and containers

| Shape | `imports` | `name` | `alias` | `source` | `span` | `statement_span` | `symbol` |
|---|---|---|---|---|---|---|---|
| Control | H | H | H | H | H | H | H |
| Absent | M | M | H valid None; M written fact | H valid None; M written fact | M | H valid None; M written fact | M |
| Explicit null | M | M | H valid None; M written fact | H valid None; M written fact | M | H valid None; M written fact | M |
| Wrong whole type | M | M | M | M | M | M | M |
| One wrong element | M | — | — | — | — | — | — |
| Every element wrong | M | — | — | — | — | — | — |
| Empty | M | M | M | M | M | M | M |
| Duplicate key | M | M | M | M | M | M | M |
| Trailing/invalid content | M | M | M | M | M | M | M |

### Original identity

| Shape | `symbol.name` | `symbol.qualified_name` | `symbol.id` | `symbol.source_span` |
|---|---|---|---|---|
| Control | H | H | H | H |
| Absent | M | H valid None; M written fact | H valid None; M written fact | H valid None; M written fact |
| Explicit null | M | H valid None; M written fact | H valid None; M written fact | H valid None; M written fact |
| Wrong whole type | M | M | M | M |
| One wrong element | — | — | — | — |
| Every element wrong | — | — | — | — |
| Empty | M | M | M | M |
| Duplicate key | M | M | M | M |
| Trailing/invalid content | M | M | M | M |

### Name span, all geometry leaves

| Shape | `span.start_byte` | `span.end_byte` | `span.start` | `span.end` | `span.start.line` | `span.start.column` | `span.end.line` | `span.end.column` |
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

### Statement span, all geometry leaves

| Shape | `statement_span.start_byte` | `statement_span.end_byte` | `statement_span.start` | `statement_span.end` | `statement_span.start.line` | `statement_span.start.column` | `statement_span.end.line` | `statement_span.end.column` |
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


These shape cells report the permanent corpus controls, not universal closure across every source grammar alternative. The source-dependent failing alternatives below override the corresponding M claim for those inputs. An empty import list is H only for an independently empty producer; genuine absent statement spans are checked with extern-crate and future-import controls.

The remaining semantic cells extend the same JSON matrix rather than adding a new format:

| Edit/class | Column(s) and supported producer cases | Current result | Required result |
|---|---|---|---|
| Delete grouped member | imports; seven grouped languages plus five mixed variants | H! | M, complete inventory restored |
| Delete named type import list | imports; TS statement-level type `{F}`, plain/commented | H! | M; preserve valid TYPE-only binding |
| Lose/change required shallow source | source; Java p.C/C.f and later ungrouped PHP source, all absent/null/empty/changed cells | H! | M; exact source restored; typed/duplicate/trailing cells remain M |
| Append impossible but source-aligned record | imports/name/span/statement_span/symbol; nine producers, Scala clean | H! in nine; M in Scala | M, actual source imports only |
| Change statement to oversized source-consistent span | statement_span byte/point object; end and start sweep above | H! in applicable rows | M, true enclosing statement restored |
| Clear imports after non-nesting comment | imports; TS/JS/Java/C#/Go/PHP | H! | M, actual import restored |
| Keep unedited opaque regex/raw-string source | imports control; TS/JS/Swift/C# literal rows | M! (false refusal) | H/zero parses; preserve genuine empty inventory |
| Unedited short Java on-demand import | imports control; ordinary/static/commented Java p.* | M! plus missing source fact | Repair producer; required nonempty control then H |

These cells are not waived by passing shape tests. For every defective source/statement/list cell, retain every other row in its column—absent, null, wrong field/element types, empty, duplicate key and invalid trailing content—and the independently valid optional/empty controls. A test that deliberately asserts H! documents remaining work and cannot close it.

## Nonblocking Recommendations

Keep the shared indexed resolver, one occurrence collection and exact per-occurrence import provenance. The cache repairs should use reliable producer evidence without rebuilding an approximate second parser one syntax exception at a time. No unrelated refactor or performance benchmark is required. Human review and optional grammar work do not keep this review unready; the three concrete required findings do.
