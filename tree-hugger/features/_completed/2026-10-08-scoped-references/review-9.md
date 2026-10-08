---
$schema: feature-review.yaml
ready: false
findings:
  - title: R9-F1 — Comments hide Swift generic constructor calls
    priority: medium
  - title: R9-F2 — Type-only imports compete with local value declarations
    priority: high
  - title: R9-F3 — Source spellings are mistaken for semantic name identities
    priority: high
  - title: R9-F4 — Import consumers reconstruct binding provenance from names
    priority: medium
  - title: R9-F5 — Cache validation accepts lost and contradictory import evidence
    priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-07T13:38:23-07:00
spec: 2026-10-08-scoped-references/spec.md
implemented: false
implemented_by: codex/gpt-6.1-sol
log: tree-hugger/features/2026-10-08-scoped-references/log.md
description: "A **feature** review of `2026-10-08-scoped-references/spec.md`"
feature: 2026-10-08-scoped-references/review-9.md
previous: 2026-10-08-scoped-references/review-8.md
next: 2026-10-08-scoped-references/review-10.md
---

# Scoped References: review 9

The feature is **not production ready**. Both review-8 findings are verified fixed across their full required instance and consumer inventories. Five newly discovered sibling defects remain: comments hide a Swift constructor, type-only TypeScript imports suppress proved local calls, equivalent name spellings produce false or missing bindings, downstream consumers guess import provenance from names, and damaged cached import evidence can turn Unknown recursion into No. No human decision is needed.

The specification, plan, both implementation logs, all eight earlier reviews and their frontmatter/body/instance/consumer inventories were reconciled with the implementation. Completion flags and the latest log's 77-child repair inventory were checked against code and public-result assertions. The accepted file-local and pinned-grammar boundaries remain unchanged. This review makes no production or test changes.

## Previous Finding Verification

Every prior finding and unresolved ledger observation is accounted for below. Historical child ranges retain their original scope: passing those rows does not close newly discovered siblings. Named permanent regression tests were rerun in this review's **1,471-test L1 run** and their expected positive and negative results inspected. Independent probes in inherited historical rows are earlier-review corroboration unless explicitly labeled current review-9 evidence; they are not claimed as newly executed. The two added review-8 rows record current verification of every required instance/consumer, including their added ledger siblings. The accepted grammar limitations existed before the repairs and still do; no required repair is awaiting a decision.

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
|---|---|---|---|---|
| `review-1.md — Lexical lookup crosses boundaries that do not establish a binding` | Rust inline modules, nested functions/constants/statics, closure controls, conditional chains/else/following statements and loop controls; TS/JS `with`, direct eval and defaults; occurrences, summaries, recursion, relation lookup and CLI | **verified fixed** | Review-9 L1: `LEXICAL_BOUNDARY_MATRIX`, boundary summaries/recursion, shared relation lookup and boundary CLI tests pass. Lookup enforces module/item boundaries and conditional endpoints. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-1.md — Named function expressions bind their names to outer declarations` | TS/JS ordinary and generator expression names, defaults/internal calls, final outer calls, unnamed/class/arrow controls; exclusions, placement, summaries, recursion, relation lookup and callers | **verified fixed** | Review-9 L1: `EXPRESSION_NAME_MATRIX`, expression-local summaries/recursion/relation and CLI callers pass. The shared binding inventory excludes expression names and confines visibility to the expression. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-1.md — Invocation classification loses calls across supported source forms` | Direct/wrapped calls in all 16 languages, TS type-only wrappers, member/generic wrappers, Perl/shell calls, receiver/argument/sequence/conditional/dereference negatives; reports, recursion, terminal-call and CommonJS readers | **verified fixed** | Review-9 L1: all-language invocation/negative matrices, wrapped binding, CLI, terminal-call and CommonJS tests pass. R6-F1 is outside the earlier enumerated wrappers. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-1.md — Cache validation accepts inconsistent reference evidence` | Wrong/malformed/lost retained target IDs, named/ownerless targets, owner/candidate identity, duplicate/omitted callable regions, structural matrix, summaries/dependencies/semantics, null and narrowed controls | **verified fixed** | Review-9 L1: structural/reference matrix, retained-evidence tests and two-way summary/semantic validation pass; strict stored types require keys and distinguish nullable values. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-2.md — Parameter defaults incorrectly see body-local var bindings` | Seven callable forms in TS/JS; ordinary/block/destructured/loop `var`; parameter visibility, redeclaration, hoisting, computed/destructured defaults and body eval; let/const/function controls, relation lookup and reports | **verified fixed** | Review-9 L1: both languages’ callable-form/body-declaration matrix, parameter-environment controls, relation lookup and CLI tests pass. `var_body` isolates body bindings from defaults. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-2.md — Pattern capitalization invents or omits Rust bindings` | Bare/modified capitalized bindings, lowercase constants in patterns/body/guards, paths/tuples/shorthands and irrefutable controls; walker, capture fallback, exclusions, declaration records, summaries, recursion and callers | **verified fixed** | Review-9 L1: pattern identity/record, summaries, recursion and CLI rows pass. Query repairs also remove Rust path/range captures; actual binding identity precedes record exclusion. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-2.md — Transparent call wrappers incorrectly limit coverage` | Rust parentheses/generics/type paths, all enumerated TS wrappers, JS parentheses and constructors; direct/self/member/value/sequence/conditional/dereference controls, eval, recursion and shared relation scopes | **verified fixed** | Review-9 L1: callee-coverage and complete/narrowed recursion matrices, shared operand/eval readers pass. Calls to another established declaration still prove No where coverage is complete. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-2.md — Cache validation accepts nonexistent evidence identities and missing callable regions` | Every-language phantom owners, applicable candidates/targets, owned/ownerless targets, Python/Lua empty callable regions, summaries, callable/narrowed entries and null/bodiless controls | **verified fixed** | Review-9 L1: every-language retained identity mutation tests and empty Python/Lua callable-region cases pass. Missing identities require a compatible omitted kind; regions use retained body facts. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R3-F1 — TypeScript instantiation expressions hide calls and constructor invocations | `.01–.18`, `.C1–.C7`, ledger `.R`: wrapper/member/constructor operands, usages/arguments and all-language controls; classification, candidates, summaries, coverage/recursion, projection/widening, CLI and revision invalidation | **verified fixed** | Review-9 L1: TS instantiation invocation/coverage/recursion, member candidates, cache narrowing/widening/hits, CLI and stale revision tests pass. Both classifier and operand reader share the instantiation rule. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R3-F2 — Rust associated value calls falsely prove absence of recursion | `.01–.16`, `.C1–.C5`: inherent/trait/enum/generic/wrapped value paths and unrelated initializer controls; function/constructor/usage controls, candidate fallback, coverage, summaries, narrow recursion, semantic widening and callees | **verified fixed** | Review-9 L1: associated value call, summary and complete/narrowed recursion rows, record-only semantic widening and CLI coverage pass. Unique members and uncertain candidates both check value kinds. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R3-F3 — Cache validation accepts false source facts and misordered or duplicate lists | `.L01–.L16.{region,path,candidates}`; `.S01–.S03.{rs,ts,js}.{duplicate,reverse}`; `.S04–.S10`; `.S09r.{rs,ts,js,go,py,java,cs,c,cpp,swift,scala,php,lua,perl,sh,zsh,omitted}`; `.C1–.C4`; `.M.kind` and eight owner/coverage coordinate leaves | **verified fixed** | Review-9 L1: all-language source-fact/candidate/region edits, duplicate/reversed summaries, path/start deletion, omitted-record and full leaf matrices pass. Retained geometry, written source paths, list order and summaries are checked. Perl collection was repaired at 19:02 before the earlier 19:14–19:25 cache repair; this cycle preserves that unblocking. Coordinated deletion of both redundant fields remains the review-4-excluded limit. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R3-F4 — Optional and tagged eval calls incorrectly hide lexical declarations | `.01–.07`, `.C1–.C5`: TS/JS optional/tagged/wrapped eval, genuine direct/sequence/member/alias controls; scope barriers, declared bases, summaries/candidates, coverage/recursion, filtering/cache/callers | **verified fixed** | Review-9 L1: `EVAL_CALL_FORMS`, scope/relation/summary/coverage/recursion, cache and CLI tests pass. Both binding and coverage distinguish optional/tagged calls from direct eval. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R4-F1 — Written-path collection drops qualifiers and can bind rooted names to locals | `.01–.28`, `.C1–.C7`: roots, nested C++ paths/generics, C# alias/conditional/generic paths, PHP nullsafe/namespaces, Perl package/receiver paths and all-language controls; exclusions, lookup/category/stopping, candidates/summaries, recursion, projection/cache/reporting/docs/revision | **verified fixed** | Review-9 L1: `WRITTEN_PATH_MATRIX`, rooted Rust target/summary/recursion, narrow/wide/warm/projection, text/JSON reports and stale-entry tests pass. Rooted names never fall into local lookup. Java receiver rows are now independently restored too. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R4-F2 — Invocation classification omits calls and mistakes PHP callable creation for execution | `.01–.24`, `.C1–.C6`: rooted calls, C# conditional/generic/delegate chains, PHP placeholder versus unpacking for named/qualified/static/instance/variable forms, variable constructors, Perl code-reference and name-only negatives; coverage, candidates/summaries, filtering/cache/reports/docs/revision | **verified fixed** | Review-9 L1: every .01–.24 invocation row, placeholder/unpacking negatives, conservative summaries/recursion, cache/projection and report tests pass. The invocation climb handles those roots, conditional members and variable calls. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R5-F1 — Invocation classification still omits constructed and implicit member targets | .01–.22, .C1–.C6; ledger .S1–.S20: Java/Scala/Swift/C++ constructed/generic/implicit targets, all-language negatives, binding/candidates, summaries, coverage/recursion, cache/projection, reports/docs/revision | **verified fixed** | All required review-5 rows pass current invocation/negative matrices, conservative-result assertions, six-stage cache tests and `refs::constructed_types_and_implicit_members_are_callees_and_callers`. Swift subscripts remain Usage. Scope is closed for those rows; ledger .S12’s additional grammar limitation is evaluated separately below. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R5-F2 — Unreported declaration tokens leak into reference occurrences | .01–.38, .C1–.C6 and .S1–.S20: all declaration sites/controls, constraints/defaults/body reads, exclusions/candidates, ownership/summaries/recursion, filter/cache/CLI/docs/revision | **verified fixed** | Review-9 `DECLARATION_EXCLUSION_MATRIX` asserts exact missing declaration offsets **and** retained real-read offsets; all rows pass. Shared Python/Scala pattern readers, wildcard controls, cache unreported-declaration stages and CLI suggestions pass. Exclusion precedes candidate selection. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R5-F3 — Receiver-qualified Java construction loses its written target | .01–.10, .C1–.C5, .S1–.S15; R5-X.01–.15 (fixed Python-star/PHP-static siblings plus controls): receiver/generic/anonymous paths, candidates/lookup, reports, cache/projection/docs | **verified fixed** | Current full-path matrix, conservative Java target assertions, `written_receiver_paths_hold_through_the_cache` and receiver text/JSON reports pass. Exact receiver paths, independent receiver/argument reads, Swift operator exclusion, Python star and PHP static paths are asserted. R5-F1’s generic-classification prerequisite was repaired before these dependent rows. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R5-F4 — Parameter captures turn type and default-value references into declarations | .01–.16, .C1–.C6, .S1–.S3: Python/C# callable forms, actual record lists/types/default reads, PHP constants/defaults and Rust pattern paths, counts/signatures/candidates/selection/cache/provenance/revision | **verified fixed** | Current parameter-capture matrix, signature tests, cache six-stage exact record/read assertions, CLI ambiguity and revision-43 invalidation pass. Python/C#/PHP/Rust queries field binding names precisely. R5-F4 was repaired before R5-F2 exclusion consumed its definition set. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R6-F1 — Force-unwrapped and null-forgiving invocations disappear from call reports | .01–.18, .C1–.C6: Swift/C# bare/grouped/member/repeated/optional assertions; all-language negatives; operands, spans/paths/candidates, summaries/coverage/recursion, six cache stages, reports/docs/revision | **partially fixed** | Current invocation/negative/conservative-result, cache-stage and CLI tests pass. Independent CLI probes confirm Calls and preserved receivers/paths. .17a grouped forms work; .17b still fails in the pinned parser. | Every supported required form and consumer is fixed. Literal .17b remains a grammar limitation under the accepted release boundary; no prerequisite was removed. |
| R6-F1.17b — Swift adjacent repeated/mixed assertions | cb!!() and cb?!(); classification, coverage and call reports | **still blocked** | Independent lint reports missing ! at both calls; cb stays Usage, while (cb!)!() and (cb!)?() are Calls. `swift_adjacent_unwrap_operators_are_a_grammar_limit` passes. Cargo.lock pins Swift 0.7.3; 0.7.1 is the manifest requirement. | Optional grammar work outside the accepted release contract; see Blocked Findings. The failing parser prerequisite existed before repair. |
| R6-F2 — Parameter readers mistake comments for default values | .01–.14, .C1–.C5: eight default-capable languages; C# methods/constructors/local functions, PHP functions/constructors/methods, Scala methods; names/types/arity/defaults/optionality, headers/cache/revision | **verified fixed** | Current comment-position/signature/header/six-stage-cache tests pass. C# skips extras before/after =; PHP/Scala read default_value; dead Swift helper read removed. Independent g(int x /* b */ = 1, int y = /* a */ 2) record has defaults 1/2, both optional. | None of the required cases remains failing. |
| R6-F2 ledger siblings — C# params and Swift aliases | Comments before params type/name and after the equals in a Swift type alias | **verified fixed** | `comments_beside_params_arrays_and_alias_targets_are_skipped` passes; both readers skip extras. Callable matrix checks record lists and arity. | None. |
| R6-F3 — Contextual syntax markers become false reference suggestions | .01–.11, .C1–.C6: Lua attributes, Java inference/legacy forms, modifiers; exclusions/candidates, ownership, summaries, reports and cache | **verified fixed** | Review-9 marker matrix, six-stage marker cache and cold/warm CLI tests pass. The R7-F1 Java static-import gate now rejects value-import evidence; Swift precedence markers are role-excluded. Independent static-import/Swift CLI controls confirm this. | Earlier enumerated cases and the R7-F1 recurrence are closed. |
| R6-F3.S1 — PHP class-relative markers | self/static/parent in type/new/instanceof slots; genuine same-spelled method reads | **verified fixed** | Current marker/cache/CLI rows pass. Independent PHP source retains self::static(), static::static(), parent::static() member Calls and excludes class-relative type/construction operands. | None. |
| R6-F3.S2 — Bash/Zsh declaration options | Leading -/+ words through --; ordinary commands/arguments/expansions | **verified fixed** | Both shell marker rows pass. Shared option count partitions options and operands; ordinary commands do not enter that rule. | None. |
| R6-F3.X1 — Lost initializer/default reads | .01–.17 plus .18: all 16 initializer readers, Java field/local/var/for initializers, TS/JS diagnostics defaults; records/references/cache/CLI | **verified fixed** | Current INITIALIZER_READ_MATRIX, public diagnostics-reference, cache and CLI tests pass. value is no longer a name field; parameter defaults are retained. | None of the enumerated cases remains failing. R7-F3 is a separate false-import sibling affecting the same public diagnostics-reference consumer. |
| R6-F3.X2 — Shell bare declaration operands | .01–.05: Bash/Zsh bare/word-shaped operands, expansions/calls, Bash integer control; cache/CLI/docs | **verified fixed** | Exact-offset, cache and CLI rows pass. Independent Zsh source excludes declaration operands while keeping four expansion reads and echo Call. Docs correctly allow excluded names without records. | None. |
| R5-F1.S12 — Swift specialized explicit initializer paths | C<Int>.init() and N.C<Int>.init(); qualifiers and conservative Calls/coverage | **still blocked** | Independent current CLI reports init Call with no path for both expressions. The pinned tree chooses comparison syntax. Accepted source-plus-pinned-grammar boundary predates all repairs. | Optional grammar work outside the required release contract; see Blocked Findings. Prerequisite unchanged before/after repair. |
| Earlier ledger observations — Java inferred-type markers and Lua attributes | Both observations carried into R6-F3.01/.02/.04/.05, including real same-spelled reads and suggestions | **verified fixed** | Current exact public tests and independent no-import Java/Lua probes pass. | No unchanged observation remains; static-import variant is also verified fixed under R7-F1. |
| R7-F1 — Syntax-marker exclusions still admit Java import-triggered and Swift precedence tokens | .01–.08a/.08b; .09/.10 controls; .11a–d/.12a–c/.13; every .14 language leaf; .C1–.C6; ledger .S1–.S7, including all .S5 language leaves | **verified fixed** | Inspected `SyntaxMarkers::nodes`, Java declaration/import gate, Swift role filters and their exclusion before resolution. Full `MARKER_EXCLUSION_MATRIX`, legacy/read/anonymous-region controls, marker cache stages and cold/warm ID-selected reports pass review-8 L1. Independent CLI retains only real Java `var()` and Swift group/type reads. | No earlier required marker case remains. No blocker changed. |
| R7-F2 — Declaration and import-name exclusions omit unsupported and compound name forms | .01–.15, including .02b, .05b, .09b, .10a–c, .11b, .12b and all eight .15 selectors; every .16/.17 clean-language leaf; .C1–.C7; ledger .S1–.S8 and .X1 | **verified fixed** | Shared `declared_name_tokens` is used by recorded-name ranges and the unreported-name inventory before lookup. Exact `DECLARATION_EXCLUSION_MATRIX` and every-shape six-stage cache pass; .12a preserves the genuine class ID, .12b has no fabricated forward/reverse edges. All Swift selectors, import reports, public diagnostics references and C/C++ elaborated-use record/read tests pass. Independent TS and Swift CLI controls confirm missing declaration tokens and retained actual reads. Java import masking was removed before these exclusions were verified; both decisions now pass together. | No enumerated declaration repair remains. R8-F1's comment-bearing sibling is now verified fixed. R9-F3 concerns new equivalent-name spellings, not unchanged .01–.15 reproductions. |
| R7-F3 — Java qualified references are misclassified as imports and disappear | .01–.13 including named annotation arguments/deep paths and genuine-import controls; every .14 reader; .C1–.C7; ledger .S1–.S7 and .X1–.X3 | **verified fixed** | Java capture is structurally inside `import_declaration`; all-query predicate guard passes. Full Java matrix/public diagnostics inventory, exact candidates, six cache stages, module/deep-annotation ID reports and imports tests pass. Other readers' binding/source-role matrices, alias dependencies, export exclusions, Go raw/blank/dot imports and revision-45 invalidation pass. Independent Java CLI has genuine imports only and preserves qualified annotation reads. | Original enumerated cases are closed. R8-F1/R8-F2's later import variants are now fixed. R9-F2–F5 identify further category, identity, provenance and stored-import siblings; a broad class-closure claim remains unsupported. |
| R7-OBS.1 — Python future imports in the separate type-owner relation reader | Future-import binding in relation/containment lookup, separate from conservative occurrence evidence | **unresolved** | The repair ledger explicitly defers this separate resolver consumer. Future-import occurrence exclusion and import facts pass the current Python matrix; the lexical relation reader still lacks the future-import arm. Accepted spec preserves containment/export contracts. | Nonblocking separate-contract work; retained below, not waived or claimed repaired. |
| R7-OBS.2 — Plain import reports reconstruct some syntax imperfectly | Language-specific reconstruction by `hug imports` text renderer; underlying binding records/JSON and required refs reports | **unresolved** | Ledger deferral remains. Inspected the separate import renderer; required refs text/JSON and exact import-record tests pass. Syntax reconstruction is outside the refs report contract. | Nonblocking separate display work; retained below. |
| R8-F1 — Comments corrupt import identities, qualifiers and binding exclusions | .01, .01.export, .02; .03, .04a/.04b; .05.star/.underscore/.given; .06.mixed/.given; .07.simple/.qualified; .08; .09.ts/.js; every .10/.11 language leaf; ledger .S1/.S2; .C1–.C8 | **verified fixed** | Current `COMMENTED_IMPORT_MATRIX`, shared `for_each_import_row`/`assert_commented_import_row` assertions and exact metadata/read controls pass in the 1,471-test run. Checked comment-free and edited originals/qualifiers, exclusions before candidate selection, genuine reads, dependencies, Unknown recursion, six cache stages, public diagnostics, relation barriers, export controls and cold/warm CLI imports/refs JSON/text. Revision is 47; revision-46 entries miss. The repair skips extras before selecting entity targets and Scala prefix/name nodes. No blocker existed or was removed. | All required instances/consumers are fixed. Swift generic adjacency R9-F1 is a newly discovered sibling at another reader, not this import reproduction. |
| R8-F2 — Scala import collection omits rename and operator forms | .01.single/.multiple/.nested; .02.escaped/.original; .03.direct/.multiple/.braced; .04.arrow/.as including mixed aliases; .05.direct including rooted qualifier, .05.braced/.wildcard; .06; every .07 language/control leaf; ledger .S1; .C1–.C6 | **verified fixed** | Current Scala query captures unbraced renames and operator names; reader uses the clause's path context and includes operator qualifiers. Public import matrix asserts exact binding/original/source, hidden-alias absence and retained reads for all rows; all six cache stages, diagnostics, independent relation barrier controls and cold/warm CLI imports/refs pass. R8-F1's comment prerequisite and this syntax repair coexist before verification; neither accepted grammar boundary changed. Reviewed the log's entire 77-child combined inventory, including clean controls and added siblings, rather than just its completion flag. | All required instances/consumers are fixed. Quoted/escaped semantic identity R9-F3 is a new sibling, not the fixed Scala naming alternatives. |
| review-6.md — C++ parenthesized-allocation grammar observation | `new (C)` and corresponding invocation/coverage consumers | **still blocked** | Review 6 explicitly classified this as optional: clang accepts the source, but the pinned parser has missing type nodes. Current grammar/query versions and accepted pinned-grammar prerequisite are unchanged; no repair removed it. | Optional grammar observation, carried under Blocked Findings; no required release repair or human decision. |

## Unblocked Findings

### R9-F1 — Medium — Comments hide Swift generic constructor calls

**Defect class:** an invocation reader uses named-sibling adjacency without skipping grammar extras, so inserting a comment changes a valid constructor invocation into a value usage.

The tree-hugger library's [generic_operand](../../lib/src/analysis/occurrences.rs:1703), which finds the name invoked by a generic expression, requires a Swift `type_identifier`'s immediate next named sibling to be `type_arguments`. A comment is also a named sibling. The pinned parser produces a `constructor_expression` containing `user_type(type_identifier, multiline_comment, type_arguments)` without syntax errors for both these spellings:

```swift
struct C<T> {}
func run() { _ = C<Int>() }          // reported C: Call
func edited() { _ = C /* n */ <Int>() } // reported C: Usage; required: Call
```

Run `hug symbols --json-full --no-cache example.swift`. Select the C declaration's ID with `hug refs --id <C-ID> --callers --json example.swift`: the commented constructor disappears from suggestions. Selecting the enclosing callable with `--callees` also loses that call. A copy of `sample.swift`, extended once with `Box<T>` and a `Box<Int>()` control, reproduces the same loss after inserting exactly one comment. This is supported syntax, distinct from the accepted specialized-`.init` grammar limitation.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R9-F1.01 / Swift unqualified generic constructor | `C /* n */ <Int>()`, also `C/*n*/<Int>()` | C is Usage; candidate C remains | C is Call, NameOnly with C candidate; Int remains Usage | Error-free public index and pinned-tree inspection; fixture control/edit; all six public cache stages |
| R9-F1.02 / Swift qualified constructor | `N.C /* n */ <Int>()` | C Call, written path N.C | Clean: preserve Call, qualifier and independent N/Int usages | Independent CLI and six stages; qualified-parent climb reaches the user type |
| R9-F1.03 / Swift other comment position | `C<Int /* n */>()` | C Call, Int Usage | Clean: comment inside type arguments must not change invocation | Error-free CLI and six stages |
| R9-F1.04 / Swift type annotation | `func run(x: C /* n */ <Int>) {}` | C and Int Usage | Clean: a generic annotation is never a constructor call | Independent index and six stages |
| R9-F1.05 / Swift operator/implicit-member path reader | `-. /* n */ some.f()`, `-.some /* n */ .f()`, `!. /* n */ some.f()` | some Usage; f Call; written path retains leading dot and comment, excludes unary operator | Clean: preserve the complete written member path and actual invoked member | Independent indexes; compared with `-.some.f()` control |
| R9-F1.06 / Rust pattern first-child reader | Comments before match arms and inside `Some(f)`/`ref f` patterns | Body call binds pattern variable, without outer function ID | Clean: comments must not remove a binding or expose an outer function | Pinned tree places the pre-arm comment outside `match_pattern`; independent public targets and coverage |
| R9-F1.07 / TypeScript assertion last-child reader | Comments before/after operand and inside `<any>` in `(<any>f)()`; wrapped eval control | Calls retained; direct eval remains a binding barrier | Clean: preserve Call; genuine eval cannot become an ordinary lexical lookup | Independent control/edit indexes; comment is outside the selected operand or skipped by the enclosing wrapper |
| R9-F1.08 / Scala postfix and Perl invocation readers | Existing direct/postfix forms; Perl `defined`/`exists` followed by a line comment before `&f` | Perl f remains Usage; direct Scala generic f remains Call | Clean: existence tests do not invoke f; generic application does | Public CLI probes and grammar-field/operand invariants; permanent invocation/negative matrices rerun |
| R9-F1.09.rs / Rust | `f /* n */ :: /* n */ <i32>()` | Call | Clean: f Call, type argument Usage | Independent direct control/edit CLI |
| R9-F1.09.ts / TypeScript | `(f /* n */ <number>)()` | Call | Clean: f Call | Independent control/edit CLI |
| R9-F1.09.js / JavaScript | `(/* n */ f /* n */)()` | Call | Clean: f Call | Independent control/edit CLI |
| R9-F1.09.java / Java | `new C /* n */ <String>()` | C Call | Clean: constructor Call; String Usage | Independent control/edit CLI |
| R9-F1.09.cs / C# | `f /* n */ <int>()` | Call | Clean: f Call | Independent control/edit CLI |
| R9-F1.09.cpp / C++ | `f /* n */ <int>()` | Call | Clean: f Call | Independent control/edit CLI |
| R9-F1.09.scala / Scala | `f /* n */ [Int]()` | Call | Clean: f Call; Int Usage | Independent control/edit CLI |
| R9-F1.09.go / Go | `(f /* n */ [int])()` | f Usage | Clean under the existing pinned-grammar boundary: indexing does not prove a generic callee; do not promote arbitrary indexing | Same Usage as unedited control; explicit limited-language coverage and documented Go index-expression invariant |
| R9-F1.09.c / C | `(/* n */ f /* n */)()` | Call | Clean: f Call | Independent control/edit CLI |
| R9-F1.09.py / Python | Line comments inside parenthesized f | Call | Clean: f Call | Independent error-free control/edit CLI |
| R9-F1.09.lua / Lua | Line comments inside parenthesized f | Call | Clean: f Call | Independent error-free control/edit CLI |
| R9-F1.09.php / PHP | `(/* n */ $f /* n */)()` | name f Call | Clean: variable invocation Call; bare-function-name parentheses retain the existing negative rule | Independent control/edit CLI; source token includes `$`, reported target name is f |
| R9-F1.09.pl / Perl | Line comment between f and argument parentheses | Call | Clean: f Call | Independent control/edit CLI |
| R9-F1.09.sh / Bash | Trailing command comment | Call | Clean: command f Call | Independent control/edit CLI |
| R9-F1.09.zsh / Zsh | Trailing command comment | Call | Clean: command f Call | Independent control/edit CLI |

| Child ID | Decision/consumer | Required result and verification |
|---|---|---|
| R9-F1.C1 | Operand selection, wrapper climb and reference capture | Skip extras in the Swift generic adjacency decision; preserve all .02–.09 controls and exact name spans. The existing capture already finds C, so adding a report filter or a second query is insufficient. |
| R9-F1.C2 | Candidate selection, ownership, coverage and summaries | Keep C as a conservative candidate, not a Scoped constructor ID. Keep enclosing run ownership, Int usage, Limited/unsupported_language coverage, Unknown recursion and empty established edges. These are correct currently. |
| R9-F1.C3 | Narrowing, projection and cache widening | The wrong Usage survives narrow Miss, narrow Hit, zero-parse semantic widening, kind widening, complete Hit and projected Hit. Assert independently expected Call at every stage. |
| R9-F1.C4 | CLI default/callers/callees, text/JSON | C's default report currently suggests Usage; callers omits it, and run's callees omits it. ID selection must preserve uncertainty while retaining the Call. Cold/warm JSON and text were probed; warm runs report one Hit and zero parses. |
| R9-F1.C5 | Other invocation consumers and maintenance | Preserve the shared operand rules used by coverage, wrapped-member capture, terminal-call/CommonJS readers and diagnostics. Add this shape to declared L1 matrices, update relevant comments/current invocation docs and invalidate revision-47 output. No L3 requirement or grammar upgrade. |

**Required repair:** fix R9-F1.01, verify .C1–.C5, preserve every clean child. Sweep extras at every sibling-position decision above; do not broadly classify generic type usages as calls.

### R9-F2 — High — Type-only imports compete with local value declarations

**Defect class:** import binding collection discards the TypeScript `type` modifier, allowing a type-only alias to enter value lookup and suppress an established local binding.

The tree-hugger library's [ECMA binding_sites](../../lib/src/analysis/scope/ecma.rs:230), which builds lexical bindings, assigns every import `ANY_CATEGORY` and `callable: true`. Its [import_names](../../lib/src/analysis/scope/ecma.rs:261) returns only name nodes, losing statement-level and per-specifier type-only information. [reference_category](../../lib/src/analysis/scope/ecma.rs:311) and [ScopeTree::visible](../../lib/src/analysis/scope/mod.rs:523) then see an import and a local function competing in the value category.

```typescript
// types.ts
export interface F { x?: number }

// example.ts
import type { F } from "./types";
function F(): void { F(); }
let value: F;
```

`tsc --noEmit --skipLibCheck example.ts` succeeds. The body F call must target the local function's ID, with complete coverage and recursion Yes; the annotation must retain the type import with no external ID. `hug symbols --json-full --no-cache example.ts` instead reports a NameOnly call with the local function as a candidate, recursion Unknown, and no self-reference/reverse edge. `hug refs --id <function-ID> --callers --json example.ts` puts the provable call in suggestions. The source itself supplies the modifier and local declaration; no dependency loading or type inference is needed.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R9-F2.01 / Statement-level named import | `import type { F }`; local function F and F annotation | Body Call NameOnly; recursion Unknown | Body Scoped to function ID, Yes; annotation Scoped import/no ID | Error-free public CLI, successful tsc, six cache stages |
| R9-F2.02 / Statement-level named alias | `import type { F as Alias }`; local function Alias | Same competition | Scoped self-call/Yes; Alias type annotation remains imported | Successful tsc, CLI and six stages |
| R9-F2.03 / Type-only default import | `import type Def`; exported default interface, local function Def | Same competition | Scoped self-call/Yes; Def annotation uses type import | Successful tsc, CLI and six stages |
| R9-F2.04 / Per-specifier modifier | `import { type F }` | Same competition | Same split of value and type lookup | Successful tsc, CLI and six stages |
| R9-F2.05 / Per-specifier alias | `import { type F as Alias }` | Same competition | Same split; no external target ID fabricated | Successful tsc, CLI and six stages |
| R9-F2.06 / Mixed type/value specifiers | `import { type F as Alias, G as g }`; Alias self-call and g call | Alias NameOnly/Unknown; g correctly Scoped import | Alias Scoped self-call/Yes; g remains import dependency G | Successful tsc, independently expected imports/occurrences, six stages |
| R9-F2.07 / Modifier comments | Comments around statement-level `type`, original, alias and per-specifier `type` | Same defect; metadata/comments otherwise preserved | Same semantic categories as comment-free forms | Successful tsc, error-free CLI, six stages; both modifier positions checked |
| R9-F2.08 / Value query in a type expression | `import type {F}; function F(){}; type T = typeof F` | typeof F Usage is NameOnly | Usage Scoped to the local function ID; do not mistake a type-expression context for the type-name category | Public index and six stages; F is an interface in the source module and a value function locally |
| R9-F2.09.namespace / Type-only namespace import | `import type * as NS`; `let v: NS.F` with no local collision | NS Scoped import, F conservatively unresolved | Clean: keep the proved namespace alias and written type path | Public index and six stages. A same-scope function NS is rejected by tsc and is not a required positive reproduction. |
| R9-F2.09.require / Type-only external require alias | `import type Q = require("./types")`; `let v: Q.F` | Q Scoped import, F unresolved | Clean: retain type-qualified alias; cannot become a runtime import binding | Public index/six stages. Conflicting same-scope function Q is rejected by tsc and excluded from the required positive cases. |
| R9-F2.10 / Identifier literally named type | `import { type as g }`; call g | Scoped import, original type, No for run | Clean: `type` here is the exported name, not a modifier | Public CLI and six stages; grammar-role distinction |
| R9-F2.11 / Noncolliding type import and mixed positive controls | Imported T, local recursive f; imported T parameter plus ordinary g call | f Scoped/Yes; T type import; g import | Clean: preserve all required type usages and ordinary value imports | Independent public indexes/six stages |
| R9-F2.12.js / JavaScript import rules | Ordinary named/default/namespace aliases and calls | Existing aliases establish imports | Clean: JS has no type-only import modifier; ordinary call bindings must remain | Shared ECMA permanent matrices plus R9-F1/R8 commented-import public controls |
| R9-F2.12.rs / Rust categories | `use`/extern aliases and globs | Existing category and glob behavior | Clean: Rust use imports can bring type/value/macro names; the accepted rule is intentionally different | Category/use matrix rerun; Rust rule cannot be copied from TS |
| R9-F2.13.{go,py,java,cs,c,cpp,swift,scala,php,pl,sh,zsh,lua} / Other languages | Binding/recursion dispatch | No Scoped binding or proven No under these language rules | Clean for this resolved-category class: LanguageRules is absent and all regions are Limited/unsupported_language | Exhaustive LanguageRules dispatcher, every-language occurrence and recursion tests rerun; does not waive their import metadata or cache defects |

| Child ID | Decision/consumer | Required result and verification |
|---|---|---|
| R9-F2.C1 | Collection and import grammar roles | Retain modifier meaning per binding, including default, namespace, require aliases, mixed named specifiers and comments. Preserve ordinary exported name type. Apply the modifier at its actual grammar role, not by searching statement text. |
| R9-F2.C2 | Categories, lookup stopping, ambiguity and candidates | Type-only imports must not compete in runtime-value lookup. Local F must establish its ID rather than degrade to a candidate. Preserve annotation/type-query distinctions and external type import bindings. Do not discard the import entirely. |
| R9-F2.C3 | Summaries, coverage and recursion | Restore forward/reverse self-edges and Yes for every valid local self-call. Mixed g remains a dependency; no type-only import becomes a runtime dependency through that call. Actual F annotation remains a type import usage. |
| R9-F2.C4 | Whole-file filters and six cache stages | Every defective positive row was run through all six stages; Unknown/NameOnly survives zero-parse widening and warm Hits. Assert Yes, correct IDs, summaries and type reads independently in complete and kind-narrowed results. |
| R9-F2.C5 | CLI selection, both directions and other shared lookup consumers | Current ID-selected incoming/callers are suggestions; callees labels the self-call name_only. Restore established rows, preserving file-local selection and no-ID type evidence. The relation reader separately collects import barriers with `import_binding` and filters shared import sites out; preserve its type-owner barriers and category contract when changing shared binding facts. |
| R9-F2.C6 | Tests/docs/comments/invalidation | Add the whole modifier/collision matrix to declared L1 occurrence, summary/recursion, cache and CLI targets; retain all actual type-read controls and shared relation tests. Update current name-category/import docs, drifted comments and extractor revision. No human design choice or compiler-backed resolver expansion. |

**Required repair:** close .01–.08 and verify .C1–.C6, preserving .09–.13. Conservatively losing a self-call proved by the file is a required defect, even though the reported Unknown is safer than a false Yes.

### R9-F3 — High — Source spellings are mistaken for semantic name identities

**Defect class:** lexical lookup, candidate grouping and recursion compare source spellings directly instead of the language's identifier or imported-name identity, producing false established edges and false Yes/No answers.

The tree-hugger library's [ScopeTree::bind/visible](../../lib/src/analysis/scope/mod.rs:399), used to place and find a name in its lexical scope, keys the table on borrowed source text. [Resolver::by_name/records_named](../../lib/src/analysis/occurrences.rs:465) does the same for candidates. [import_original_name](../../lib/src/analysis/mod.rs:551), which decides whether a no-ID import call could name the enclosing callable, compares a raw identifier/path or quoted import original with the declaration name. None supplies a semantic comparison key.

```rust
fn r#f() { f(); }                     // actual self-call; reported No
fn f(x: fn()) { let r#f = x; f(); }   // calls the local value; reported Yes
```

```typescript
function \u0066() { f(); }                      // actual self-call; reported No
function f(x: () => void) { const \u0066=x; f(); } // local value; reported Yes
import { "S" as D } from "./example";
export function S() { D(); }                    // imported S could be itself; reported No
```

The last source compiles with tsc and uses a self-module import only to demonstrate why No is unsafe; the tool must still report Unknown, not load that module or fabricate an external ID. The unquoted `import { S as D }` control already reports Unknown. Rust raw identifiers and ECMA Unicode escapes likewise do not create new names. Rust also normalizes identifiers to NFC: `fn café() { café(); }` (the call uses `e` plus combining acute accent) compiles with rustc but reports No. In JS/TS, those two literal Unicode sequences are **different** identifiers; that is a necessary negative control, not a request for global Unicode normalization.

For the raw Rust shadow case, full indexes contain a false self-edge and `hug refs --id <f-ID> --callers` lists the local-value call as established. For raw/escaped declaration self-calls, the incoming report is empty and callees lists an unresolved call. The spelling-only edit in copies of `sample.rs` and `sample.ts`, each extended once with a recursive positive control, changes Yes to No. All isolated cache sources in this review parse without syntax diagnostics.

In the instance table, `\u0066` means one written backslash followed by `u0066`; brace escapes and actual non-ASCII text were tested separately. Keep source spans and written qualifiers as written. Comparison keys can be canonical without replacing the occurrence's source spelling or inventing declaration IDs.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R9-F3.01.rs / Rust callable definition and reference | `fn r#f(){f()}`, inverse `fn f(){r#f()}`; same-raw-spelling control | Mixed spellings Unresolved/No; same spelling Scoped/Yes | Both mixed forms Scoped to own ID/Yes; keep control | Public indexes, ID reports and six stages; fixture-copy one-spelling edit |
| R9-F3.01.ts / TypeScript callable definition/reference | Escaped declaration, escaped call, `\u{66}` declaration, native café with escaped é | Unresolved/No | Scoped own-ID self-call/Yes | Error-free public CLI/six stages; ordinary escaped self/import sources pass tsc |
| R9-F3.01.js / JavaScript callable definition/reference | Escaped declaration/call, native café with escaped é | Unresolved/No | Scoped own-ID self-call/Yes | Error-free public CLI and six stages |
| R9-F3.02.rs / Rust local declaration and parameter | `let r#f=x`, parameter `r#f`; plain body f call | Binds enclosing f ID; false self-edge/Yes | Bind actual local value; no enclosing f edge; indirect coverage/Unknown | Public records/occurrences/summaries, ID reports and six stages |
| R9-F3.03.rs / Rust pattern and loop sites | `Some(r#f)` in match/if-let, `for r#f in xs`, closure parameter `r#f` | Calls outer f ID; named owners can falsely be Yes | Bind each local pattern/loop/closure name; preserve scopes and owner isolation; no outer f ID | Public match/conditional/loop/closure probes and six stages |
| R9-F3.04.rs / Rust bare-pattern identity | Raw const c, raw unit struct C, raw const-generic C, each read under plain spelling in pattern/body | Pattern read omitted; body appears bound to invented local (or type use unresolved) | Pattern and body read the established constant/unit/generic binding; exclude only actual binders, not these required reads | Exact public occurrence lists and six stages; constant-generic binding legitimately has no declaration ID |
| R9-F3.05.rs / Rust declaration kinds and categories | Raw struct/enum/trait/type alias, const/static/macro, generic type parameter; plain equivalent reads | Type/value/macro reads Unresolved; generic T Unresolved | Correct type/value/macro binding; generic T Scoped/no ID; preserve macro expansion limitation and kind categories | Public `rawkinds`, `rawgeneric` indexes and six stages; each symbol kind's actual rule checked |
| R9-F3.06.rs / Rust associated member and impl owner | Raw method f with `Self::f`; raw C declaration with `impl C` and `Self::f` | Raw method self-call Unresolved/No; raw owner loses parent ID and leaves f NameOnly/Unknown | Under the existing single-inherent-impl rule, target own ID/Yes and attach the established owner; preserve receiver uncertainty elsewhere | Public full index, relation facts and six stages; no instance receiver inference |
| R9-F3.07.rs / Rust imports and extern aliases | Raw original `use other::r#f as D` inside f; raw alias D with plain D call; raw extern alias c with `c::option::Option` | Original comparison falsely No; alias/qualifier Unresolved and false unused-import diagnostic | Potential self-import Unknown; establish actual local alias/no external ID; qualifier read retained; no false unused-import warning | Public import/diagnostic/occurrence facts and six stages |
| R9-F3.08.rs / Rust Unicode normalization | Literal NFC declaration and decomposed equivalent call | Unresolved/No | Own-ID self-call/Yes | rustc accepts it; public index and six stages |
| R9-F3.09.{ts,js} / Recorded variable and parameter sites | Escaped var/let/const/destructured/formal/arrow/single-arrow parameter, plain f body call | Established outer f target, false edges/Yes where owned by f | Local binding wins; actual value call remains indirect; owner-isolated arrows do not refer to outer f | Public per-site indexes; TS/JS value, parameter, destructuring and arrow readers swept; applicable sources traversed six stages |
| R9-F3.10.{ts,js} / Unreported bindings | Escaped catch name and `for` var/let/const name | Established outer f target; false Yes | Scoped actual local without record where applicable, no outer target/edges; Unknown when owned by f | Public catch/loop indexes and six stages; each loop placement tested |
| R9-F3.11.{ts,js} / Expression-local names | Escaped named function, generator and class expression; same-named outer f/C | Internal read/call binds outer declaration | Internal expression name establishes its own binding without outer ID; declaration token still excluded; later outer reads remain possible | Public indexes/six stages; named function/generator and class roles checked separately |
| R9-F3.12.ts / TypeScript kinds and generic sites | Escaped interface/type/enum/namespace/class/type parameter, plain type/constructor reads; escaped ambient function; escaped const-arrow callable | Type reads and constructor/self-call Unresolved; const arrow falsely No | Bind actual declarations/type parameters; const-arrow self-call Yes; bodyless ambient declaration stays Unknown, caller has the established target | Public per-kind indexes and six stages |
| R9-F3.13.ts / Overload identity | `function \u0066(x:number):number; function f(x:any){return x}; … f(1)` | Call Scoped to implementation; escaped overload omitted | NameOnly with all matching declaration candidates; no overload selection by argument type | Public full index and six stages; spelling must not remove ambiguity |
| R9-F3.14.{ts,js} / Member candidate names | Ordinary f/#f declaration, `this.\u0066()` / `this.#\u0066()` | Unresolved with no candidates | NameOnly including the same-named member; no Scoped method ID; Unknown remains | Public per-language public/private member indexes; TS rows through six stages |
| R9-F3.15.{ts,js} / Import local aliases | Escaped named/default/namespace aliases; TS require/entity aliases; plain alias use | Unresolved, missing dependency, false unused-import warning | Alias/qualifier Scoped to actual import/no external ID; real usage and source/original facts retained; member dispatch remains uncertain | Public indexes/diagnostics; named/default/namespace plus both TS-equals forms swept; six stages for applicable rows |
| R9-F3.16.{ts,js} / Imported original identity | Plain quoted `"S"`, `"\u0053"`, `"\u{53}"`, escaped identifier S; ordinary S control | Quoted/escaped originals make false No; ordinary S makes Unknown | Equivalent original S names yield Unknown; preserve source representation and external no-ID boundary | TS quoted/escaped self-module sources pass tsc; public TS/JS indexes and six stages |
| R9-F3.17 / Necessary negative/control distinctions | Same escaped spelling on both sides; ordinary ASCII names; JS/TS literal NFC versus decomposed spelling; string original `"A.S"` | Same-spelling self-call Yes; distinct Unicode/string names do not become self identities | Clean: language-specific equality only; a dot inside a string-named export is not a namespace separator | Independent complete indexes/six stages; permanent ordinary-name matrices rerun |
| R9-F3.18.{go,py,java,cs,c,cpp,swift,scala,php,pl,sh,zsh,lua} / Other resolution paths | No-rules resolver and semantic dispatch | All references remain uncertain; recursion cannot become proven Yes/No through their lexical names | Clean for this resolved-identity class under accepted first-release breadth; do not expand their resolver or rewrite documented backticked/source names | Exhaustive LanguageRules and unsupported-coverage invariants, per-language tests rerun. Their import-cache facts remain required under R9-F5. |

| Child ID | Decision/consumer | Required result and verification |
|---|---|---|
| R9-F3.C1 | Definition facts, all binding sites, pattern decisions and lookup stopping | Use one language-aware identity comparison across captured/light/unreported bindings, hoisting, order, category and visibility lookup. Preserve actual declaration token exclusion; raw constants must still be reads. Do not repair only recorded functions while leaving for/catch/expression/pattern sites on source text. |
| R9-F3.C2 | Candidate grouping, member/owner lookup and ambiguity | Canonical equality must feed the candidate index and explicit Rust type/member lookup before filtering. Preserve sorted/deduplicated candidate IDs and overload uncertainty; no first-match fallback. The shared relation tree and import barriers need the same comparison when establishing Rust impl owners. |
| R9-F3.C3 | Summaries and recursion | Restore required self-edges/Yes and remove false outer edges/Yes. Compare imported originals by identifier or decoded string-name identity, not quote characters or a path separator inside a string literal. Preserve source spelling in reports. Current complete coverage allows false No; existing limitations do not cure that. |
| R9-F3.C4 | Stored validation, record-only semantics and projection | All observed failures survive real warm Hits and zero-parse semantic widening. When mixed spellings become correct Scoped references, strict cache name/ID and summary checks must compare semantic identity while still checking source-token geometry; raw string equality cannot reject the repaired entries. Exercise omitted-kind owners/targets and canonical-equivalent member candidates too. |
| R9-F3.C5 | CLI, diagnostics, selection and public evidence | ID-selected incoming/callers currently invent outer references or omit actual self-calls; callees shows unresolved targets. Preserve written name-token spans/paths, correct suggestions and absent-ID classification, and remove false unused-import diagnostics. Verify exported/imported record facts and shared type-owner consumers without broadening their contract. |
| R9-F3.C6 | Permanent regression matrix and maintenance | Add all .01–.18 rows to live declared L1 targets with independent expected IDs, absences, edges, coverage and recursion, including legitimate same-spelling and language-specific non-equivalence controls. Update relevant current docs/comments and revision invalidation. No compiler integration is needed; compiler probes here independently validate language semantics. |

**Required repair:** close every defective .01–.16 leaf, preserve .17/.18 and verify .C1–.C6. A source-wide replacement that destroys written spans or applies Rust NFC normalization to JS/TS would introduce a new defect.

### R9-F4 — Medium — Import consumers reconstruct binding provenance from names

**Defect class:** consumers discard the particular binding proved for an occurrence and reconstruct import meaning from a file-wide or owner-wide name set, mixing unrelated scopes and local bindings with imports.

The tree-hugger library's [semantic_facets](../../lib/src/analysis/mod.rs:464), which calculates cached and fresh recursion facets, puts **every** import's local/original names in one set. [may_call_itself](../../lib/src/analysis/mod.rs:530) uses that set for each Scoped/no-ID call. The CLI's [build_report](../../cli/src/refs.rs:291), which classifies outgoing reference rows, instead checks whether the owner's dependencies contain the call's name. Collection and [bind_references](../../lib/src/analysis/mod.rs:372) know the actual import through the aligned `bound_imports` list; that provenance is dropped before these consumers.

```rust
mod other { pub fn g() {} pub fn f() {} }
fn f() { use other::g as q; q(); }
fn h() { use other::f as q; q(); }
```

f's dependency correctly names q with original g. Its recursion nevertheless becomes Unknown solely because h has a different q importing f. Changing h's alias to r makes f No, although f's body and binding are unchanged. Under the current accepted original-name recursion rule, f must stay No for both controls.

```typescript
import { G as D } from "./types";
function run(xs: (() => void)[]) {
  D();
  for (const D of xs) { D(); }
}
```

Both Calls are Scoped/no ID, but only the first is an import. `hug refs run --callees --json` labels **both** `absent_id: import`; text calls the loop binding “outside this file.” The second must be `binding_without_record`. This is a counterexample to the implementation log's Phase 5 claim that dependency-name membership identifies an occurrence's binding. It does not challenge the accepted file-local/external-ID boundary or require receiver inference.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R9-F4.01.rs / Rust recursion | Above separate functions, repeated q alias; renamed-alias control | f Unknown versus No | No in both; only actual bound q→g may affect f | Independent complete indexes, correct dependency g checked; six stages |
| R9-F4.02.ts / TypeScript recursion | Root `import D=N.other`, S calling D; namespace M importing `D=N.S`; rename M's D to E | S Unknown versus No | No in both; namespace M's alias cannot affect root S | Both sources pass tsc; independent indexes/six stages, correct dependency N.other |
| R9-F4.03.ts / TypeScript outgoing classification | Imported D then unreported `for (const D of xs)` in same owner | Both rows import | First import, second binding_without_record | ID-selected/default/callers/callees, JSON/text, actual cold/warm cache; six public stages |
| R9-F4.03.js / JavaScript outgoing classification | Same import/for source in JS | Both rows import | Same independently expected split | Same shipped-CLI/public-cache checks |
| R9-F4.03.rs / Rust outgoing classification | Imported D then `Some(mut D) => D()` in same owner | Both rows import | Pattern-local call binding_without_record; outer call import | Same shipped-CLI/public-cache checks; explicit moded pattern proves a local binding |
| R9-F4.04 / Collection and relationship summaries | All rows above | Correct actual bound import selected; dependency lists remain correct | Clean: preserve these facts and ordered edges; retain local value bindings | Independently compared import originals/dependencies; bound_imports alignment inspected |
| R9-F4.05 / Other scope/name controls | Distinct aliases; unique import; nonconflicting local binding; JS root imports; source spellings from R9-F3 | Unique imports and renamed-scope controls correct | Clean: no unrelated import may change a call; ordinary root-only JS import declarations cannot create the valid nested-import counterexample | Explicit control indexes, existing imported/unreported binding matrices; name-spelling interactions covered separately by R9-F3 |
| R9-F4.06.{go,py,java,cs,c,cpp,swift,scala,php,pl,sh,zsh,lua} / No-rules consumers | Conservative occurrences | No Scoped/no-ID import classification; Unknown recursion | Clean for this provenance consumer: the Scoped branches are unreachable under the accepted language support table | Exhaustive confidence/coverage invariant and all-language tests rerun |

| Child ID | Decision/consumer | Required result and verification |
|---|---|---|
| R9-F4.C1 | Preserve actual binding provenance | Keep enough of the collected binding/import decision to tell an import, an unreported local and an unresolved binding apart at each occurrence. The existing summaries remain useful but their name membership cannot reconstruct this fact. |
| R9-F4.C2 | Recursion and imported originals | Fresh and stored-occurrence semantic calculation must use the selected import, not a same-named import from another scope. Preserve No for established calls to other originals, Unknown for potentially self-imports and Yes for own-ID Calls. |
| R9-F4.C3 | Storage, validation and all projections | Six-stage public probes reproduce both source classes. Retain provenance in narrow entries, zero-parse semantic widening, kind widening, complete Hits and type-only projections; validate its alignment/consistency. Do not solve only the fresh path by keeping an ephemeral list. |
| R9-F4.C4 | CLI both directions, text/JSON and ownership | Import/local Call rows keep their actual owner and distinct sites. Current incoming/default/callers for run are correctly empty; outgoing classification is wrong. Cold/warm text and JSON both mislabel local calls, with one Hit and zero warm parses. Preserve these incoming controls and the file-local boundary. |
| R9-F4.C5 | Tests/docs/comments/revision | Add scope-alias and import/local-collision rows for Rust/TS/JS to the existing L1 semantic/cache/refs targets. Correct the misleading implementation assumption in current code comments/docs; leave the historical implementation log as a claim reconciled by the new repair log. Invalidate changed persisted output. |

**Required repair:** close .01–.03 and every language leaf, preserve .04–.06, verify .C1–.C5. No new external IDs, compiler resolution or human sign-off is required.

### R9-F5 — High — Cache validation accepts lost and contradictory import evidence

**Defect class:** the strict cache reader validates occurrence/coverage structures but reads import facts through permissive public structs without checking their source, identity or agreement with retained dependencies, allowing damaged entries to be served as evidence.

The tree-hugger library's [StoredEntry](../../lib/src/cache/symbol_cache.rs:547), which reads persistent JSON entries, holds `imports: Vec<ImportRecord>` using the public [ImportRecord](../../lib/src/shared/schema_v2/mod.rs:59). Its nullable fields and nested public SymbolRef accept omitted keys as None. [validate_evidence](../../lib/src/cache/stored_evidence.rs:526) checks that a dependency has a same-named no-ID Scoped occurrence, but does not require that dependency's original identity to agree with any retained import. Imports themselves have no corresponding geometry/identity validation. The strict occurrence types do not protect this sibling structure.

Use this correctly repaired source:

```typescript
namespace N { export function S() { D(); } }
import D /* n */ = N. /* n */ S;
```

Seed a functions-only `SymbolCache::query` with Occurrences and no Semantics. The unedited warm entry is Hit with original N.S and dependency N.S; widening to Semantics parses zero files and reports Unknown. Restore that exact tool-written entry before each edit. These **single** edits are accepted as Hit with zero parses:

- Replace `imports` with `[]`; keep the dependency and occurrence unchanged.
- Delete `imports[0].symbol.qualified_name`, or set it to null/empty, while the dependency still says N.S.
- Set the alias to an empty string; the call/record/dependency still name D.

Each changes the subsequent zero-parse semantic widening to **No**. The wrong result is computed from damaged cached facts, whereas fresh analysis remains Unknown. Entries holding Declarations alone or References alone also serve the deleted original as Hit; later semantic widening reparses and restores it in those two controls. Validation therefore must not depend on requesting Occurrences or Semantics.

The sweep also edits all imported-name/source/identity fields and every import/statement span leaf in copies of the ten supported import fixtures. All tool-written controls are actual Hits. Typed malformed values, bad array elements, duplicate keys and trailing garbage are rejected; deleted meaningful nullable facts, empty/contradictory strings, impossible IDs, duplicate import records, zero coordinates and out-of-source geometry are not. For the six languages whose shipped query captures no imports, inserting a cloned tool-written import into their empty list is also Hit. This is a checkable contradiction to the effective-query invariant, not the accepted coordinated-forgery limitation.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R9-F5.01.{rs,ts,js} / Retained imported dependencies | Clear imports; delete/null/empty required original; empty alias | Hit; occurrence-backed semantic widening can change Unknown to No | Miss/recompute correct original/dependency/Unknown; preserved summaries cannot describe an absent/different import | Public cache mutation matrix from correct controls; TS and ordinary Rust/JS self-import controls |
| R9-F5.02.{rs,ts,js,java,cs,php,py,go,scala,swift} / Imports list | absent/null/wrong field or elements, empty and duplicated list, duplicate key/trailing input | Structural bad shapes Miss; empty/duplicate lists Hit | Structural controls stay Miss; populated-tool-entry deletion/duplication cannot silently become genuine empty/duplicate imports | Full per-format matrix; every control Hit, restored before each single edit |
| R9-F5.03.{same ten languages} / Binding name and alias | empty/wrong name; delete/null/empty or change a written alias | Nonempty contradictions/empty names Hit; meaningful alias loss Hit | Import name must agree with its name token/symbol and binding role; no fabricated empty alias; preserve real no-alias and on-demand controls | Full name/alias columns, typed-error controls and source/summary invariants |
| R9-F5.04.{same ten languages} / Source and original identity | Delete/null/empty/change a written source or original | Hit; absent/null conflated; metadata lost | Reject lost required source/original facts; preserve genuine source-less TS entity aliases and genuinely unaliased imports | Full source/original columns; public metadata and supported-language semantic widening |
| R9-F5.05.{same ten languages} / Nested symbol identity | empty/contradictory symbol.name; empty/invented symbol.id | Hit, including 64-character invented IDs | Bound import symbol name must agree with the import; external target ID remains absent under this file-local contract | Public results through the full symbol-name/ID columns; null/absent ID controls are legitimate |
| R9-F5.06.{same ten languages} / Name-span geometry | Each of start/end byte, start/end point, all four line/column leaves; zero/huge values | Missing/wrong typed leaves Miss; zero or out-of-source numeric leaves Hit | Name span must name the actual bound token with valid one-based coordinates and source boundaries | All eight leaf columns independently edited; whole-object empty/null controls rejected |
| R9-F5.07.{same ten languages} / Statement-span geometry | Omit/null span; all eight leaves, zero/huge values | Span loss Hit; invalid numeric coordinates Hit | Preserve the actual import statement and compatible name-span containment; legitimate nullable cases need explicit controls | Whole-object plus every leaf matrix; all source statements known from tool-written fixture controls |
| R9-F5.08.{same ten languages} / Import symbol source_span | Insert a well-typed copied span into symbol.source_span | Hit with a source span the producer never emits | Reject impossible import evidence; keep genuine absent/null controls | Public field insertion in all ten sources; `import_record` always emits no symbol source span |
| R9-F5.09.{c,cpp,lua,pl,sh,zsh} / Empty-import languages | Insert one copied well-typed import into a no-import fixture entry | Hit | Miss: shipped effective Imports query has no import capture, so this populated entry is inconsistent | Six public cache mutation probes; existing compiled-query capture inventory is the independent invariant |
| R9-F5.10 / Strict structural controls | Bad whole type/elements, duplicate keys, trailing garbage; legitimate absent ID/source/original controls | Miss for typed malformed shapes; genuine nullable controls Hit | Clean: retain strict structure and each explicitly valid optional/empty state | One JSON-format matrix, 2,014 cells across ten language controls plus supplementary nullable-span and six no-import probes; detailed tables below |

“Same ten languages” in .03–.08 means a separate required child for each of Rust, TypeScript, JavaScript, Java, C#, PHP, Python, Go, Scala and Swift; .06/.07 additionally retain a separate child for every listed coordinate leaf. These are the same reader, but different producer facts and nullable forms; none may be omitted from the repair ledger.

| Child ID | Decision/consumer | Required result and verification |
|---|---|---|
| R9-F5.C1 | Strict stored representation and semantic consistency | Inventory/write/read the complete import shape, distinguish lost data from legitimate None, enforce allowed IDs, exact name/alias roles and source/span consistency. Check dependencies against retained import facts; do not replace a damaged field with a permissive default. |
| R9-F5.C2 | Every capability and kind coverage | Probe Declarations-only, References-only, Occurrences without Semantics, complete Semantics and type/function projections. Import validation must run even when no occurrence list is retained or its owner kind is omitted. A complete semantic entry rejects a deleted original with `Miss(Inconsistent("semantics that the stored evidence does not produce"))`; this independent control was run after seeding Semantics. Preserve that clean case. |
| R9-F5.C3 | Cache lookup, widening and recomputation | Assert Miss and correct fresh public metadata/Unknown, not merely an error returned by a deserializer. Zero-parse widening is allowed only from valid evidence; repeated warm lookup after recomputation must actually Hit. Preserve one entry per file and the existing strict occurrence/coverage matrices. |
| R9-F5.C4 | CLI consumers and filtering | Full indexes and cached imports text/JSON expose the edited fields; cached refs depends on the same name/identity/span facts. Reject before CLI import selection, grouping, rendering and recursion use them. All CLI paths consume this common SymbolCache reader; report-specific cleanup cannot fix the public API. |
| R9-F5.C5 | Complete robustness test and maintenance | Add one public-API JSON matrix from tool-produced fixture entries with independently expected imports, source positions and semantic results, covering every field/shape/language leaf above. Include positive source-less, no-alias, on-demand, genuinely empty-import and omitted-kind controls. Update current cache docs/comments and invalidate revision-47 entries. No new format/dependency or expansion of the accepted forgery limit. |

**Required repair:** close .01–.09 and all leaves, verify .C1–.C5, preserve .10. Empty imports are legitimate only where the producer actually has none; a retained dependency whose import vanished is not an indistinguishable coordinated deletion.

### Nonblocking carried observations

- **R7-OBS.1 — Python future imports in the separate type-owner relation reader:** **unresolved**. The relation import reader still has no future-import arm; occurrence/import facts are correct. This remains separate-contract work under the accepted preservation of containment behavior, with a relation regression required when that reader is repaired.
- **R7-OBS.2 — Plain import syntax reconstruction:** **unresolved**. The separate imports renderer still reconstructs some language-specific syntax imperfectly. Underlying fixed import facts and required refs reports are independently checked. This remains nonblocking display work; it does not waive R9-F4's required false binding label or R9-F5's incorrect stored facts.

## Blocked Findings

No **required** repair is blocked. The frontmatter flags refer to the required release inventory.

- **R6-F1.17b — Swift adjacent repeated/mixed assertions:** **still blocked** by the unchanged pinned Swift grammar; `cb!!()` and `cb?!()` remain outside its supported call tree. Grouped counterparts and the current grammar-limit test pass.
- **R5-F1.S12 — Swift specialized explicit initializer paths:** **still blocked** by the unchanged comparison-parsing context of the pinned grammar for `C<Int>.init()`/`N.C<Int>.init()`. Supported generic constructions remain required; the comment-sensitive constructor in R9-F1 has a correct constructor tree and is not covered by this exception.
- **review-6.md — C++ parenthesized-allocation grammar observation:** **still blocked**, optional. The earlier `new (C)` source passes clang but the pinned parser produces missing type nodes; the grammar/query versions are unchanged. It was explicitly excluded from actionable invocation repairs in review 6, not silently waived here.

The accepted first-release breadth in [the original implementation log](implementation-log.md:365) is Rust/TypeScript/JavaScript resolution, conservative occurrences elsewhere, the source plus pinned grammar, file-local analysis and no receiver inference. Those choices preceded repair and remain unchanged. No grammar prerequisite was removed before or after this repair cycle. The historical Perl qualifier prerequisite was removed before its dependent cache repair; the table retains that unblocking. No human reaffirmation is needed. The five required new findings all have usable current syntax/evidence, so no blocker or accepted choice excuses them.

## Recurrence

Every earlier review's body, frontmatter, instance/consumer inventory and repair ledger was compared with this review's cases. Earlier enumerated cases are passing; these are **newly discovered siblings**, not an unchanged failing review-8 reproduction. No new repair regression is asserted without evidence that the review-8 change introduced it.

- **R9-F1** repeats the grammar-extra adjacency class of **review 6 / R6-F2 — Parameter readers mistake comments for default values** and **review 8 / R8-F1 — Comments corrupt import identities, qualifiers and binding exclusions**. It also falls in the invocation class of **review-1.md — Invocation classification loses calls across supported source forms**, **R3-F1**, **R5-F1** and **R6-F1**. The missed decision is Swift generic sibling selection; the missed consumers are Call filtering, wrapper operands and cached evidence. The earlier import/default and force-unwrap cases remain fixed.
- **R9-F2** is a newly discovered import/category sibling of **review 7 / R7-F3 — Java qualified references are misclassified as imports and disappear**, whose ledger swept TS/JS aliases but omitted type-only category decisions, and the earlier lexical binding class. The missing information is the statement/specifier type modifier before category filtering and candidate selection; the downstream losses are self-edges, Yes recursion and established CLI callers. The named ordinary-import reproductions are still fixed.
- **R9-F3** repeats semantic binding-identity failures from **review-2.md — Pattern capitalization invents or omits Rust bindings** and expression shadowing from **review-1.md — Named function expressions bind their names to outer declarations** on equivalent source spellings. Its import-original cases are naming-alternative siblings of **R8-F1** and **R8-F2**. Earlier plain-spelling pattern and expression cases pass. Missed decisions are canonical identity before scope lookup, category filtering, overload/member candidates, bare-pattern interpretation and import-original recursion comparison; missed consumers include summaries, owner relations, diagnostics, strict-cache name checks and ID-selected reports.
- **R9-F4** repeats the earlier scoped-binding/import-evidence class at **downstream consumers**, rather than the repaired collection. Collection proves the right binding, but that decision is discarded and guessed again from names. The missed consumers are file-wide recursion and owner-wide outgoing absent-ID classification, including record-only cache widening. Phase 5's log description is an implementation claim contradicted by these collisions, not an accepted waiver of correct reports.
- **R9-F5** repeats **review-1.md — Cache validation accepts inconsistent reference evidence**, **review-2.md — Cache validation accepts nonexistent evidence identities and missing callable regions** and **R3-F3 — Cache validation accepts false source facts and misordered or duplicate lists** in the previously unenumerated **imports** structure. Every earlier occurrence/coverage/path/list column still passes. Imports were omitted from that strict-field/geometry/summary sweep; the downstream false No occurs specifically during Occurrences→Semantics widening. The full new reader/field/language inventory is now explicit. Accepted coordinated deletion of redundant qualifier facts remains outside this finding.

`recurrence: true` records these repeated classes. It does not change the verified status of the original enumerated repair instances or move nonblocking grammar observations into the release contract.

## Requirement Verification and Test Levels

| User-facing requirement | Verification level present | Assessment |
|---|---|---|
| Lexical scopes, order/hoisting, categories, patterns, expression names and aliases | L1 public API/CLI matrices plus independent source controls | Original cases pass; R9-F2 and R9-F3 retain required category/identity repairs. |
| Calls versus usages, constructors, wrappers, receivers and repeated sites | L1 all-language invocation/negative matrices and source edits | Original forms pass; error-free commented Swift constructors fail R9-F1. |
| Spans, owners, declaration/import exclusions, genuine reads and qualifiers | L1 exact permanent matrices, independent public indexes and source-token comparisons | R8 cases fixed; spelling equivalence and import provenance need R9-F3/R9-F4. Do not remove genuine reads as a conservative workaround. |
| Ordered relationships and honest Yes/No/Unknown recursion | L1 complete/narrowed summary/semantic/cache tests plus independently compiled source probes | Required false/missing proofs remain in R9-F2/R9-F3/R9-F4/R9-F5. Compiler probes establish source semantics only; no compiler integration is requested. |
| Whole-file filtering, projection, one shared collection and record-only widening | L1 work counters, query/cache/projection matrices | Infrastructure passes. Independent six-stage probes show wrong source facts persist; zero-parse widening also consumes damaged imports under R9-F5. |
| Strict persistent-cache handling | L1 tool-written JSON controls and public lookup/mutation results | Earlier fields pass. Imported facts have a missing strictness/consistency matrix, R9-F5. |
| Name/ID/smart-case selection, ambiguity, locals, empty reports, both directions and file-local boundary | L1 refs suite and independent ID-selected cold/warm JSON/text | Original controls pass; R9-F1–F4 produce missing, false or misclassified report rows. |
| Terminal report grouping, styling, wrapping, empty boundary | L2 actual tmux pane capture through live area recipe | Passed: backend run=1, skip=0, panic=0. No focus request; no claim that WezTerm/Kitty content was independently verified this iteration. |
| Keyboard input, hotkeys, paste/IME/mouse | No such requirement in this feature | L3 unnecessary. |

Review-9 verification:

- `just test -j 4` in tree-hugger: **1,471 passed, 0 skipped**, 219.537 seconds. This includes the complete R8 import/comment/alternative, public-reference, six-stage cache, relation, CLI and revision-46 invalidation tests, and every named earlier regression matrix.
- `just lint`: **passed** for library and CLI; no formatter run.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_declarations_in_tmux`: **1 selected test passed**, 5.653 seconds; **tmux run=1, skip=0, panic=0**. Other tests were excluded by the filter, not claimed as executed.
- Root `just check-tier-coverage tree-hugger`: **zero stranded tests**. Library `autotests=false` declares `tests/l1/main.rs`, which includes the reviewed modules. CLI `tests/symbols/main.rs` includes refs/robustness/upgrade modules. Their ordinary names select L1; terminal targets require terminal-tests, enabled by the live recipe and CI metadata.
- **95 error-free source copies** traversed six public cache stages in three scratch sweeps (4 + 55 + 36; repeated inputs across sweeps are not claimed unique). Each had narrow Miss, narrow Hit, zero-parse semantic widening, kind widening, complete Hit and projected Hit. All warm/record-only stages parsed zero files. Each finding's expected IDs, kinds, metadata or semantics is independently stated above; equality to fresh wrong output is not the correctness criterion.
- A further public JSON import-field sweep covered **2,014 cells** over ten language controls, restoring the exact tool-produced entry before each edit; supplementary source-span/no-import/capability probes are described in R9-F5. Typed errors recomputed their control. Tests on strings/nulls/empty lists recorded the public Hit/miss and post-widening result, not just deserialization success.
- Shipped-CLI probes checked default/callers/callees by declaration ID, plus cold/warm JSON/text. Repeated warm reports showed one actual Hit and zero parses. Fixture-copy controls establish load-bearing source edits for R9-F1 and R9-F3; tsc/rustc probes independently confirm the disputed supported-language identities and valid type-only declarations.

Scratch probes are review evidence, **not permanent regression tests**. No test or production source was added/edited by this review. The required repairs must add their full matrices to live declared L1 targets and preserve the existing L2 report coverage. Skills used: tree-hugger, rust, rust-testing and biscuit-test-harness. Cross-OS execution evidence is left to CI and does not determine readiness.

## Input Robustness Matrix

The feature's changed file-format reader is the persistent **JSON cache**, not a manifest or lockfile parser. Source grammar repairs do not add another configuration format. The occurrence/coverage matrices below remain verified by the rerun permanent tests. **R9-F5 adds the missing imports columns**; all defects found in this section are titled and listed in frontmatter.

For prior columns, H means an actual valid Hit; M means miss/recompute of the correct control; — means no applicable element shape. Valid nullable/empty controls remain explicit rather than defaults for missing keys. The previously accepted coordinated deletion of both redundant path fields remains unchanged.

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


The strict **occurrence/coverage** stored structs require their evidence keys and distinguish present-null from missing. The named permanent tests rerun their complete field/leaf matrices and public miss/recompute results. This invariant does **not** extend to the public import structs, as the new columns show.

### Imported facts: JSON reader and ten producer languages

The following tables report actual public lookups from 2,014 restored-entry cells across Rust, TypeScript, JavaScript, Java, C#, PHP, Python, Go, Scala and Swift. Each column was exercised in every language. Controls are copied tool-produced imports; only one JSON field changes per cell. A row reports the same result in every applicable language unless its cell states a nullable distinction. This is one format reader with ten producer contracts, not ten different JSON parsers.

**H** = legitimate control Hit. **H!** = defective edit accepted as Hit; expected **M**, meaning reject/recompute the correct public imports and semantics. **H/H!** distinguishes genuine absence from deleting a meaningful written value. **M** = correct rejection. **—** = shape inapplicable to a scalar/object. A numeric zero is not an empty collection; it is listed separately as a source-consistency edit. Duplicate *records* are distinct from duplicate JSON *keys*.

Nullable rules are source-dependent. Swift's chosen import genuinely lacks alias/original; the TypeScript entity-alias control genuinely lacks source. Every import target ID is genuinely absent. These controls must stay accepted. Removing a written alias/source/original from the other controls is a different outcome, even though the current reader conflates it with genuine absence. An empty module string may be valid in an independently matching source; the tested empty-string edits remove a known nonempty source and retain its contradictory dependencies/spans. Do not reject legitimate syntax merely to pass these mutations.

#### List, binding and metadata

| Shape | `imports` | `name` | `alias` | `source` | `span` | `statement_span` | `symbol` |
|---|---|---|---|---|---|---|---|
| Unedited control | H | H | H | H | H | H | H |
| Absent | M | M | H/H! | H/H! | M | H! | M |
| Explicit null | M | M | H/H! | H/H! | M | H! | M |
| Wrong whole type | M | M | M | M | M | M | M |
| One wrong element | M | — | — | — | — | — | — |
| All elements wrong | M | — | — | — | — | — | — |
| Empty string/list/object; numeric 0 | H! | H! | H! | H! | M | M | M |
| Duplicate JSON key | M | M | M | M | M | M | M |
| Trailing invalid content | M | M | M | M | M | M | M |
| Contradictory string / duplicate records / out-of-source number | H! | H! | H! | H! | — | — | — |

#### Original identity and name-span structure

| Shape | `symbol.name` | `symbol.qualified_name` | `symbol.id` | `span.start_byte` | `span.end_byte` | `span.start` | `span.end` |
|---|---|---|---|---|---|---|---|
| Unedited control | H | H | H | H | H | H | H |
| Absent | M | H/H! | H | M | M | M | M |
| Explicit null | M | H/H! | H | M | M | M | M |
| Wrong whole type | M | M | M | M | M | M | M |
| One wrong element | — | — | — | — | — | — | — |
| All elements wrong | — | — | — | — | — | — | — |
| Empty string/list/object; numeric 0 | H! | H! | H! | H! | H! | M | M |
| Duplicate JSON key | M | M | M (supplement) | M | M | M | M |
| Trailing invalid content | M | M | M | M | M | M | M |
| Contradictory string / duplicate records / out-of-source number | H! | H! | H! | H! | H! | — | — |

#### Name-span coordinates and statement bounds

| Shape | `span.start.line` | `span.start.column` | `span.end.line` | `span.end.column` | `statement_span.start_byte` | `statement_span.end_byte` | `statement_span.start` |
|---|---|---|---|---|---|---|---|
| Unedited control | H | H | H | H | H | H | H |
| Absent | M | M | M | M | M | M | M |
| Explicit null | M | M | M | M | M | M | M |
| Wrong whole type | M | M | M | M | M | M | M |
| One wrong element | — | — | — | — | — | — | — |
| All elements wrong | — | — | — | — | — | — | — |
| Empty string/list/object; numeric 0 | H! | H! | H! | H! | H if unchanged 0; H! otherwise | H! | M |
| Duplicate JSON key | M | M | M | M | M | M | M |
| Trailing invalid content | M | M | M | M | M | M | M |
| Contradictory string / duplicate records / out-of-source number | H! | H! | H! | H! | H! | H! | — |

#### Statement-span points and coordinates

| Shape | `statement_span.end` | `statement_span.start.line` | `statement_span.start.column` | `statement_span.end.line` | `statement_span.end.column` |
|---|---|---|---|---|---|
| Unedited control | H | H | H | H | H |
| Absent | M | M | M | M | M |
| Explicit null | M | M | M | M | M |
| Wrong whole type | M | M | M | M | M |
| One wrong element | — | — | — | — | — |
| All elements wrong | — | — | — | — | — |
| Empty string/list/object; numeric 0 | M | H! | H! | H! | H! |
| Duplicate JSON key | M | M | M | M | M |
| Trailing invalid content | M | M | M | M | M |
| Contradictory string / duplicate records / out-of-source number | — | H! | H! | H! | H! |

The supplementary duplicate-null-ID test rejected duplicate `symbol.id` keys in **all ten** controls; it fills the column whose genuine input omitted that key. For every coordinate column, huge `999999999` is independently out of source and was incorrectly accepted. Line/column zero is always invalid because positions are one-based; byte zero can be legitimate only where it actually matches that token/statement. All geometry cells need agreement with the source, not just a numeric type check.

#### Import target source span and genuinely empty import producers

| Shape | `imports[*].symbol.source_span`, ten producers | Imports list, C/C++/Lua/Perl/Bash/Zsh |
|---|---|---|
| Unedited control | H: genuinely absent | H: genuine [] |
| Absent | H | M: required imports key |
| Explicit null | H | M: list required |
| Wrong whole type | M for 123 | M: shared strict list reader |
| One/every wrong element | — | M: shared typed-vector reader |
| Empty | M for {} | H: producer has no import capture |
| Duplicate key | M: derived stored/public-field duplicate invariant; same nullable-ID mechanism independently probed | M: shared required-list reader |
| Trailing/invalid input | M: whole JSON-document check, also tested in all earlier import columns | M: same document check |
| Well-typed invented evidence | H!: copied span inserted, although `import_record` always emits None | H!: one copied import inserted in all six source controls |

The .08 source-span insertion and .09 empty-producer tests are independent supplemental probes, not included in the 2,014-cell count. The empty-producer malformed-shape cells share the exact list reader already exercised across ten fixtures; the precise invariant is that the list deserializes before language dispatch. Their producer-specific contradiction was independently run in each of the six languages.

The matrix covers absent/null distinctions, whole/element type errors, emptiness, duplicate keys and invalid document endings before checking semantic contradictions. Required new tests should walk these columns through the public query result from a real-tool fixture, restore the control for every edit, and assert correct recomputed metadata/semantics. Separate tests that merely deserialize individual hand-written values would leave R9-F5's zero-parse false-No consumer unverified.

## Nonblocking Recommendations

Keep the existing single shared occurrence collection, indexed scope/candidate lookups and record-only widening. The findings require better decision inputs and validated provenance, not a new compiler integration or report-specific analysis pass. No unrelated performance refactor is required for readiness. The two historical display/relation observations and optional grammar work remain explicitly separated above; all required repairs appear in the five titled findings and frontmatter inventory.
