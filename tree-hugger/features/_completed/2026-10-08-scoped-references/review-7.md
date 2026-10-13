---
$schema: feature-review.yaml
ready: false
findings:
  - title: R7-F1 — Syntax-marker exclusions still admit Java import-triggered and Swift precedence tokens
    priority: medium
  - title: R7-F2 — Declaration and import-name exclusions omit unsupported and compound name forms
    priority: high
  - title: R7-F3 — Java qualified references are misclassified as imports and disappear
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-06T22:54:38-07:00
spec: 2026-10-08-scoped-references/spec.md
implemented: true
implemented_by: codex/gpt-6.1-sol
log: tree-hugger/features/2026-10-08-scoped-references/implementation-log.md
description: "A **feature** review of `2026-10-08-scoped-references/spec.md`"
feature: 2026-10-08-scoped-references/review-7.md
previous: 2026-10-08-scoped-references/review-6.md
next: 2026-10-08-scoped-references/review-8.md
---

# Scoped References: review 7

The feature is **not production ready**. Earlier concrete repairs pass, but three swept defect classes still produce incorrect reports. Java static imports disable marker exclusions, Swift precedence syntax produces false suggestions, declaration forms leak their names, and Java qualified references disappear after being mistaken for imports. None needs a human decision.

The specification, plan, original implementation log, historical reviews and repair ledger were reconciled with source and public results. Completion flags are claims, not verification. Only this review, review 6’s next link and the specification’s iteration count are changed; existing implementation edits are preserved.

## Previous Finding Verification

Every historical finding is retained, including the two legacy reviews with a single Findings section. Earlier child inventories are preserved for reconciliation with the ledger. “Verified fixed” closes enumerated cases and consumers, rather than every member of a defect class. Evidence labeled current L1 was independently rerun in this review’s **1,427-test** run. Historical mentions of later R6 discoveries are assessed by the current rows below.

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
|---|---|---|---|---|
| `review-1.md — Lexical lookup crosses boundaries that do not establish a binding` | Rust inline modules, nested functions/constants/statics, closure controls, conditional chains/else/following statements and loop controls; TS/JS `with`, direct eval and defaults; occurrences, summaries, recursion, relation lookup and CLI | **verified fixed** | Current L1: `LEXICAL_BOUNDARY_MATRIX`, boundary summaries/recursion, shared relation lookup and boundary CLI tests pass. Lookup enforces module/item boundaries and conditional endpoints. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-1.md — Named function expressions bind their names to outer declarations` | TS/JS ordinary and generator expression names, defaults/internal calls, final outer calls, unnamed/class/arrow controls; exclusions, placement, summaries, recursion, relation lookup and callers | **verified fixed** | Current L1: `EXPRESSION_NAME_MATRIX`, expression-local summaries/recursion/relation and CLI callers pass. The shared binding inventory excludes expression names and confines visibility to the expression. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-1.md — Invocation classification loses calls across supported source forms` | Direct/wrapped calls in all 16 languages, TS type-only wrappers, member/generic wrappers, Perl/shell calls, receiver/argument/sequence/conditional/dereference negatives; reports, recursion, terminal-call and CommonJS readers | **verified fixed** | Current L1: all-language invocation/negative matrices, wrapped binding, CLI, terminal-call and CommonJS tests pass. R6-F1 is outside the earlier enumerated wrappers. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-1.md — Cache validation accepts inconsistent reference evidence` | Wrong/malformed/lost retained target IDs, named/ownerless targets, owner/candidate identity, duplicate/omitted callable regions, structural matrix, summaries/dependencies/semantics, null and narrowed controls | **verified fixed** | Current L1: structural/reference matrix, retained-evidence tests and two-way summary/semantic validation pass; strict stored types require keys and distinguish nullable values. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-2.md — Parameter defaults incorrectly see body-local var bindings` | Seven callable forms in TS/JS; ordinary/block/destructured/loop `var`; parameter visibility, redeclaration, hoisting, computed/destructured defaults and body eval; let/const/function controls, relation lookup and reports | **verified fixed** | Current L1: both languages’ callable-form/body-declaration matrix, parameter-environment controls, relation lookup and CLI tests pass. `var_body` isolates body bindings from defaults. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-2.md — Pattern capitalization invents or omits Rust bindings` | Bare/modified capitalized bindings, lowercase constants in patterns/body/guards, paths/tuples/shorthands and irrefutable controls; walker, capture fallback, exclusions, declaration records, summaries, recursion and callers | **verified fixed** | Current L1: pattern identity/record, summaries, recursion and CLI rows pass. Query repairs also remove Rust path/range captures; actual binding identity precedes record exclusion. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-2.md — Transparent call wrappers incorrectly limit coverage` | Rust parentheses/generics/type paths, all enumerated TS wrappers, JS parentheses and constructors; direct/self/member/value/sequence/conditional/dereference controls, eval, recursion and shared relation scopes | **verified fixed** | Current L1: callee-coverage and complete/narrowed recursion matrices, shared operand/eval readers pass. Calls to another established declaration still prove No where coverage is complete. | Earlier enumerated inventory closed; later siblings are assessed below. |
| `review-2.md — Cache validation accepts nonexistent evidence identities and missing callable regions` | Every-language phantom owners, applicable candidates/targets, owned/ownerless targets, Python/Lua empty callable regions, summaries, callable/narrowed entries and null/bodiless controls | **verified fixed** | Current L1: every-language retained identity mutation tests and empty Python/Lua callable-region cases pass. Missing identities require a compatible omitted kind; regions use retained body facts. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R3-F1 — TypeScript instantiation expressions hide calls and constructor invocations | `.01–.18`, `.C1–.C7`, ledger `.R`: wrapper/member/constructor operands, usages/arguments and all-language controls; classification, candidates, summaries, coverage/recursion, projection/widening, CLI and revision invalidation | **verified fixed** | Current L1: TS instantiation invocation/coverage/recursion, member candidates, cache narrowing/widening/hits, CLI and stale revision tests pass. Both classifier and operand reader share the instantiation rule. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R3-F2 — Rust associated value calls falsely prove absence of recursion | `.01–.16`, `.C1–.C5`: inherent/trait/enum/generic/wrapped value paths and unrelated initializer controls; function/constructor/usage controls, candidate fallback, coverage, summaries, narrow recursion, semantic widening and callees | **verified fixed** | Current L1: associated value call, summary and complete/narrowed recursion rows, record-only semantic widening and CLI coverage pass. Unique members and uncertain candidates both check value kinds. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R3-F3 — Cache validation accepts false source facts and misordered or duplicate lists | `.L01–.L16.{region,path,candidates}`; `.S01–.S03.{rs,ts,js}.{duplicate,reverse}`; `.S04–.S10`; `.S09r.{rs,ts,js,go,py,java,cs,c,cpp,swift,scala,php,lua,perl,sh,zsh,omitted}`; `.C1–.C4`; `.M.kind` and eight owner/coverage coordinate leaves | **verified fixed** | Current L1: all-language source-fact/candidate/region edits, duplicate/reversed summaries, path/start deletion, omitted-record and full leaf matrices pass. Retained geometry, written source paths, list order and summaries are checked. Perl collection was repaired at 19:02 before the earlier 19:14–19:25 cache repair; this cycle preserves that unblocking. Coordinated deletion of both redundant fields remains the review-4-excluded limit. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R3-F4 — Optional and tagged eval calls incorrectly hide lexical declarations | `.01–.07`, `.C1–.C5`: TS/JS optional/tagged/wrapped eval, genuine direct/sequence/member/alias controls; scope barriers, declared bases, summaries/candidates, coverage/recursion, filtering/cache/callers | **verified fixed** | Current L1: `EVAL_CALL_FORMS`, scope/relation/summary/coverage/recursion, cache and CLI tests pass. Both binding and coverage distinguish optional/tagged calls from direct eval. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R4-F1 — Written-path collection drops qualifiers and can bind rooted names to locals | `.01–.28`, `.C1–.C7`: roots, nested C++ paths/generics, C# alias/conditional/generic paths, PHP nullsafe/namespaces, Perl package/receiver paths and all-language controls; exclusions, lookup/category/stopping, candidates/summaries, recursion, projection/cache/reporting/docs/revision | **verified fixed** | Current L1: `WRITTEN_PATH_MATRIX`, rooted Rust target/summary/recursion, narrow/wide/warm/projection, text/JSON reports and stale-entry tests pass. Rooted names never fall into local lookup. Java receiver rows are now independently restored too. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R4-F2 — Invocation classification omits calls and mistakes PHP callable creation for execution | `.01–.24`, `.C1–.C6`: rooted calls, C# conditional/generic/delegate chains, PHP placeholder versus unpacking for named/qualified/static/instance/variable forms, variable constructors, Perl code-reference and name-only negatives; coverage, candidates/summaries, filtering/cache/reports/docs/revision | **verified fixed** | Current L1: every .01–.24 invocation row, placeholder/unpacking negatives, conservative summaries/recursion, cache/projection and report tests pass. The invocation climb handles those roots, conditional members and variable calls. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R5-F1 — Invocation classification still omits constructed and implicit member targets | .01–.22, .C1–.C6; ledger .S1–.S20: Java/Scala/Swift/C++ constructed/generic/implicit targets, all-language negatives, binding/candidates, summaries, coverage/recursion, cache/projection, reports/docs/revision | **verified fixed** | All required review-5 rows pass current invocation/negative matrices, conservative-result assertions, six-stage cache tests and `refs::constructed_types_and_implicit_members_are_callees_and_callers`. Swift subscripts remain Usage. Scope is closed for those rows; ledger .S12’s additional grammar limitation is evaluated separately below. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R5-F2 — Unreported declaration tokens leak into reference occurrences | .01–.38, .C1–.C6 and .S1–.S20: all declaration sites/controls, constraints/defaults/body reads, exclusions/candidates, ownership/summaries/recursion, filter/cache/CLI/docs/revision | **verified fixed** | Current `DECLARATION_EXCLUSION_MATRIX` asserts exact missing declaration offsets **and** retained real-read offsets; all rows pass. Shared Python/Scala pattern readers, wildcard controls, cache unreported-declaration stages and CLI suggestions pass. Exclusion precedes candidate selection. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R5-F3 — Receiver-qualified Java construction loses its written target | .01–.10, .C1–.C5, .S1–.S15; R5-X.01–.15 (fixed Python-star/PHP-static siblings plus controls): receiver/generic/anonymous paths, candidates/lookup, reports, cache/projection/docs | **verified fixed** | Current full-path matrix, conservative Java target assertions, `written_receiver_paths_hold_through_the_cache` and receiver text/JSON reports pass. Exact receiver paths, independent receiver/argument reads, Swift operator exclusion, Python star and PHP static paths are asserted. R5-F1’s generic-classification prerequisite was repaired before these dependent rows. | Earlier enumerated inventory closed; later siblings are assessed below. |
| R5-F4 — Parameter captures turn type and default-value references into declarations | .01–.16, .C1–.C6, .S1–.S3: Python/C# callable forms, actual record lists/types/default reads, PHP constants/defaults and Rust pattern paths, counts/signatures/candidates/selection/cache/provenance/revision | **verified fixed** | Current parameter-capture matrix, signature tests, cache six-stage exact record/read assertions, CLI ambiguity and revision-43 invalidation pass. Python/C#/PHP/Rust queries field binding names precisely. R5-F4 was repaired before R5-F2 exclusion consumed its definition set. | Earlier enumerated inventory closed; later siblings are assessed below. |

| R6-F1 — Force-unwrapped and null-forgiving invocations disappear from call reports | .01–.18, .C1–.C6: Swift/C# bare/grouped/member/repeated/optional assertions; all-language negatives; operands, spans/paths/candidates, summaries/coverage/recursion, six cache stages, reports/docs/revision | **partially fixed** | Current invocation/negative/conservative-result, cache-stage and CLI tests pass. Independent CLI probes confirm Calls and preserved receivers/paths. .17a grouped forms work; .17b still fails in the pinned parser. | Every supported required form and consumer is fixed. Literal .17b remains a grammar limitation under the accepted release boundary; no prerequisite was removed. |
| R6-F1.17b — Swift adjacent repeated/mixed assertions | cb!!() and cb?!(); classification, coverage and call reports | **still blocked** | Independent lint reports missing ! at both calls; cb stays Usage, while (cb!)!() and (cb!)?() are Calls. `swift_adjacent_unwrap_operators_are_a_grammar_limit` passes. Cargo.lock pins Swift 0.7.3; 0.7.1 is the manifest requirement. | Optional grammar work outside the accepted release contract; see Blocked Findings. The failing parser prerequisite existed before repair. |
| R6-F2 — Parameter readers mistake comments for default values | .01–.14, .C1–.C5: eight default-capable languages; C# methods/constructors/local functions, PHP functions/constructors/methods, Scala methods; names/types/arity/defaults/optionality, headers/cache/revision | **verified fixed** | Current comment-position/signature/header/six-stage-cache tests pass. C# skips extras before/after =; PHP/Scala read default_value; dead Swift helper read removed. Independent g(int x /* b */ = 1, int y = /* a */ 2) record has defaults 1/2, both optional. | None of the required cases remains failing. |
| R6-F2 ledger siblings — C# params and Swift aliases | Comments before params type/name and after the equals in a Swift type alias | **verified fixed** | `comments_beside_params_arrays_and_alias_targets_are_skipped` passes; both readers skip extras. Callable matrix checks record lists and arity. | None. |
| R6-F3 — Contextual syntax markers become false reference suggestions | .01–.11, .C1–.C6: Lua attributes, all Java inference slots, legacy types/imports, all-language modifiers; exclusions/candidates, ownership/conservative summaries, reports and cache stages | **regressed** | Every original exact marker/cache/CLI row passes; independent Lua probe retains only initializer reads. The new Java legacy guard also accepts static value imports as type evidence, reintroducing markers under the new import shape, R7-F1. Swift precedence markers are pre-existing siblings. | Repair R7-F1. Earlier no-import Java/Lua cases are fixed; the new static-import failure is a repair regression, not an unchanged reproduction. |
| R6-F3.S1 — PHP class-relative markers | self/static/parent in type/new/instanceof slots; genuine same-spelled method reads | **verified fixed** | Current marker/cache/CLI rows pass. Independent PHP source retains self::static(), static::static(), parent::static() member Calls and excludes class-relative type/construction operands. | None. |
| R6-F3.S2 — Bash/Zsh declaration options | Leading -/+ words through --; ordinary commands/arguments/expansions | **verified fixed** | Both shell marker rows pass. Shared option count partitions options and operands; ordinary commands do not enter that rule. | None. |
| R6-F3.X1 — Lost initializer/default reads | .01–.17 plus .18: all 16 initializer readers, Java field/local/var/for initializers, TS/JS diagnostics defaults; records/references/cache/CLI | **verified fixed** | Current INITIALIZER_READ_MATRIX, public diagnostics-reference, cache and CLI tests pass. value is no longer a name field; parameter defaults are retained. | None of the enumerated cases remains failing. R7-F3 is a separate false-import sibling affecting the same public diagnostics-reference consumer. |
| R6-F3.X2 — Shell bare declaration operands | .01–.05: Bash/Zsh bare/word-shaped operands, expansions/calls, Bash integer control; cache/CLI/docs | **verified fixed** | Exact-offset, cache and CLI rows pass. Independent Zsh source excludes declaration operands while keeping four expansion reads and echo Call. Docs correctly allow excluded names without records. | None. |
| R5-F1.S12 — Swift specialized explicit initializer paths | C<Int>.init() and N.C<Int>.init(); qualifiers and conservative Calls/coverage | **still blocked** | Independent current CLI reports init Call with no path for both expressions. The pinned tree chooses comparison syntax. Accepted source-plus-pinned-grammar boundary predates all repairs. | Optional grammar work outside the required release contract; see Blocked Findings. Prerequisite unchanged before/after repair. |
| Earlier ledger observations — Java inferred-type markers and Lua attributes | Both observations carried into R6-F3.01/.02/.04/.05, including real same-spelled reads and suggestions | **verified fixed** | Current exact public tests and independent no-import Java/Lua probes pass. | No unchanged observation remains; static-import variant is R7-F1. |

Accepted release decisions remain in force: Rust/TypeScript/JavaScript resolution, conservative evidence elsewhere, file-local analysis, no receiver inference and the pinned grammar (original implementation log’s accepted rulings, and specification Goal and boundaries). No human blocker or resolved-language prerequisite was removed this cycle. Perl cache support was unblocked by collection before its cache repair, as recorded in the historical inventory. The single-field cache-path deferral remains repaired; coordinated deletion of both redundant fields remains the accepted forgery limit.

The ledger’s three-fixed-parents claim overstates closure: R6-F1 has a split grammar-limited child, and R6-F3’s new guard creates R7-F1. Its earlier concrete instances and initializer/shell repairs are verified. This review preserves review 6’s implemented and implemented_by claims.

## Unblocked Findings

### R7-F1 — Medium — Syntax-marker exclusions still admit Java import-triggered and Swift precedence tokens

**Defect class:** identifier-shaped syntax markers become references when a marker gate accepts the wrong category of evidence or omits a supported grammar role.

In tree-hugger, [java_declared_type_name](../../lib/src/analysis/syntax_markers.rs:103) identifies legacy types named var, but its import branch never checks the static token. A modern Java static import can name a field or method called var. Its false type flag makes [SyntaxMarkers::nodes](../../lib/src/analysis/syntax_markers.rs:35), which supplies exclusion tokens before lookup, discard **every Java var marker in the file**.

```java
import static p.Tools.var;
record G() {
    void g() { var x = 1; int var = 2; use(var); }
}
```

The record syntax rules out a legacy type named var. The inferred-type token nevertheless becomes Usage/NameOnly with the local var as its candidate. Removing only the import fixes it. The local variable’s ID-selected report contains two suggestions instead of one real argument read. A static method import also triggers this; `var x=var()` must exclude the first token and keep the second as a Call.

Swift precedence syntax is a separate missed sibling. In `precedencegroup P { associativity: left }`, associativity and left read no binding, but are grammar identifiers. [syntax_markers](../../lib/src/analysis/syntax_markers.rs:44), the library’s role-based marker dispatcher, has no Swift arm. Same-spelled variables therefore receive false suggestions. Precedence-group declaration names belong to R7-F2; names after higherThan/lowerThan are real group references and must remain.

The Java sweep inserted one import into an unchanged control, including a shipped-fixture copy. The Swift sweep varied each attribute/value, kept same-spelled ordinary reads, and used a Swift fixture copy. Actionable sources have error-free pinned trees. These expectations come from token roles, not cached/uncached agreement.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R7-F1.01 / Java local | Static field/method import; var v=id(xs) | var Usage/NameOnly | Exclude marker; retain id Call/xs Usage | Full-index CLI 5:9; no-import control excludes it |
| R7-F1.02 / Java final local | final var w=id(v) | Same false suggestion | Exclude var; retain id/v | Same sweep 6:15 |
| R7-F1.03 / Java enhanced for | for(var y:xs) | Same | Exclude var; retain xs/body y | Same 7:14 |
| R7-F1.04 / Java resource | try(var r=open(xs)) | Same | Exclude var; retain open/xs/r | Same 8:14 |
| R7-F1.05 / Java lambda | (var z)->z | Same, anonymous owner | Exclude var; keep body z and region | Same 9:58 |
| R7-F1.06 / Java record pattern | o instanceof P(var a) | Same | Exclude var; retain P/o/body a | Same 10:28 |
| R7-F1.07 / Java for initializer | for(var i=0;i<var;i++) | Both tokens Usage | Exclude marker 11:14; keep real parameter read 11:29 | Same: eight var suggestions instead of one across .01–.07 |
| R7-F1.08a / static field import | import static p.Tools.var | Activates false legacy-type flag | A value import is not evidence of a type named var | Independent import edit, pinned static token |
| R7-F1.08b / static method import | import static p.Methods.var; var x=var() | Marker Usage plus real Call | Exclude marker; retain Call | Independent CLI method-call control |
| R7-F1.09 / import controls | None; ordinary/static wildcard; static import ending in another name; var only in intermediate qualifier | Inference markers excluded | Clean; intermediate spelling does not establish a type | Eight-import sweep; actual qualifier var retained |
| R7-F1.10 / legacy controls | Class/interface/enum/annotation type/type parameter named var; ordinary single-type import | Real var type use retained | Preserve provable legacy reads | Independent sources and current legacy matrix. Annotation declaration-token leak is R7-F2.01 |
| R7-F1.11a / Swift associativity directive | associativity: left/right/none | Directive Usage | Exclude directive name | Error-free precedence_group_attribute tree; same-spelled local and later read |
| R7-F1.11b / Swift assignment directive | assignment: true | assignment Usage | Exclude directive; true is already not captured | Same |
| R7-F1.11c / Swift higherThan directive | higherThan: P | higherThan Usage | Exclude directive; retain P | Same |
| R7-F1.11d / Swift lowerThan directive | lowerThan: Q | lowerThan Usage | Exclude directive; retain Q | Same |
| R7-F1.12a / Swift left value | associativity: left | Usage/NameOnly to local left | Exclude built-in value; keep ordinary left reads | 9:18 extra versus 16:57 required |
| R7-F1.12b / Swift right value | associativity: right | Same | Same role-based exclusion | Three-group CLI sweep; later right read retained |
| R7-F1.12c / Swift none value | associativity: none | Same | Same role-based exclusion | Same, later none read retained |
| R7-F1.13 / Swift group references | higherThan P, lowerThan Q, operator’s group P | Usages retained | Clean; preserve real group names | Exact attribute/operator roles and independent CLI |
| R7-F1.14.lua / Lua attributes | const/close attribute identifiers versus initializer/later reads | Attribute token excluded; actual reads retained | Clean for these roles; preserve both excluded markers and retained reads | Role-based Lua attribute arm; exact MARKER_EXCLUSION_MATRIX and independent CLI |
| R7-F1.14.php / PHP relative operands | self/static/parent type/new/instanceof versus actual method names | Relative marker excluded; actual method Calls retained | Clean for these roles; preserve both excluded markers and retained reads | Role-based PHP class operand arm; exact matrix/cache/CLI controls |
| R7-F1.14.bash / Bash options | declare -i, local -- versus expansions/ordinary commands | Options excluded; actual command/read tokens retained | Clean for these roles; preserve both excluded markers and retained reads | declaration_command-only option rule; shell exact-offset rows |
| R7-F1.14.zsh / Zsh options | typeset +x, integer operands versus expansions/commands | Options/declared operands excluded; expansions retained | Clean for these roles; preserve both excluded markers and retained reads | Same local option rule; exact-offset rows and independent CLI |
| R7-F1.14.rs / Rust modifiers | pub/mut/impl/dyn in modifier/operator roles | No marker occurrences; ordinary same-spelled permitted names keep binding behavior | Clean for these roles; preserve both excluded markers and retained reads | Reference query captures identifier/type/field names; these modifier roles are anonymous keywords, never candidate names |
| R7-F1.14.ts / TypeScript modifiers | visibility/static/async/type operators | No modifier occurrences; real identifiers retained | Clean for these roles; preserve both excluded markers and retained reads | Grammar keyword roles versus identifier captures; existing supported-binding/exclusion controls |
| R7-F1.14.js / JavaScript modifiers | static/async/await operator roles | No modifier occurrences; real identifiers retained | Clean for these roles; preserve both excluded markers and retained reads | Same ECMA role distinction, current supported-binding controls |
| R7-F1.14.py / Python keywords | async/await and match/type in keyword roles | No keyword occurrences; usable match/type names retained | Clean for these roles; preserve both excluded markers and retained reads | Keyword node roles versus identifier captures; current Python declaration/read controls |
| R7-F1.14.go / Go keywords | go/defer, actual any/comparable names | Keywords absent; type-name reads retained | Clean for these roles; preserve both excluded markers and retained reads | Keywords are not identifier nodes; any/comparable remain names, current all-language rows |
| R7-F1.14.c / C storage modifiers | static/extern/typedef roles | No storage-keyword occurrences; real type/value names retained | Clean for these roles; preserve both excluded markers and retained reads | Anonymous keyword tokens versus identifier/type_identifier query captures |
| R7-F1.14.cpp / C++ modifiers | static/constexpr/auto/decltype roles | No tested modifier occurrences; actual declared type/value reads retained | Clean for these roles; preserve both excluded markers and retained reads | Built-in/modifier grammar roles versus named reference captures; current C++ matrices |
| R7-F1.14.cs / C# modifiers | record/readonly/async/var/await; usable dynamic/nameof controls | Modifiers absent; genuine named-type/name reads retained | Clean for these roles; preserve both excluded markers and retained reads | var is implicit_type, keywords are not identifier captures; current C# matrices |
| R7-F1.14.scala / Scala modifiers | given/using/inline/lazy roles | No modifier occurrence; actual bounds/body reads retained | Clean for these roles; preserve both excluded markers and retained reads | Keyword roles and explicit given-name exclusion; independent given/abstract-member controls |
| R7-F1.14.perl / Perl declaration keywords | my/our/state/local versus actual variables/calls | Keywords absent; true variable/call reads retained | Clean for these roles; preserve both excluded markers and retained reads | Reference query captures variables/call names/package identifiers, not declaration keywords; current exact Perl controls |

| Child ID | Decision/consumer | Required repair or verified invariant |
|---|---|---|
| R7-F1.C1 | Collection and marker gate | Distinguish static value imports from established legacy type evidence; add every Swift directive/value above by role. Preserve R6-F3.06 legacy reads. |
| R7-F1.C2 | Exclusion/category/candidates/lookup | Correct excluded_names before fallback. Marker tokens must never reach NameOnly lookup; real reads keep their conservative IDs/confidence. |
| R7-F1.C3 | Ownership/summaries/coverage/recursion | Keep the Java lambda’s anonymous boundary after excluding its marker. Limited languages retain Limited coverage, empty established summaries and Unknown recursion. |
| R7-F1.C4 | Filtering/projection/cache/widening | Public API reproduces extra rows through narrow Miss → Hit → semantic widening without a parse → kind widening → complete Hit → projected Hit, for Java and Swift. Pin required absences and real-read offsets. |
| R7-F1.C5 | Default and Call reports | Cold/warm ID-selected JSON/text list false suggestions with actual warm Hits. Call-only filters already reject these Usages. Preserve real var Calls and ordinary Swift reads; repairing Call reports alone is insufficient. |
| R7-F1.C6 | Tests/docs/comments/invalidation | Extend declared marker/cache/CLI matrices with the complete inventory, correct legacy-import and precedence documentation/comments, and invalidate old extractor output. No resolver/version integration needed. |

**Required repair:** close .01–.08 and every .11/.12 leaf, preserve .09/.10/.13/.14, and verify .C1–.C6. Java is a review-6 repair regression on a new input shape; Swift precedence markers are pre-existing siblings.

### R7-F2 — High — Declaration and import-name exclusions omit unsupported and compound name forms

**Defect class:** exclusion recognizes selected declaration kinds or only a whole captured name span, leaving unreported declaration names and child tokens of compound declarations visible as reads.

The tree-hugger [declaration-name inventory](../../lib/src/analysis/declaration_names.rs:22) supplements records, and [Resolver::occurrence](../../lib/src/analysis/occurrences.rs:506) excludes exact token ranges. Several grammar declaration kinds are absent from both that inventory and [is_named_definition_node](../../lib/src/file/extract/classify.rs:870), the older name-role filter. A compound namespace record spans A.B while references capture A and B separately. TypeScript’s supported binding path has the same compound-name gap, promoting a token to a **false established class ID**.

```java
class A {}
class G { @interface A { int value() default external; } @A void g() {} }
```

Only A at 2:59 is a reference. The declared annotation type at 2:22 must be excluded; external remains a read. Selecting the outer class currently suggests both A tokens.

```typescript
class A {} namespace A.B { export function f(){} } let x:A;
```

Both A at 1:22 and A at 1:58 are Scoped to the class ID. The first declares a namespace; only the latter is a type read. Compiler-generated namespace initialization is not a source occurrence.

The sweep compared declaration offsets with independent real-use offsets, checked error-free pinned trees and public results, and used copies of Java/C#/C++/TypeScript/Swift/Scala/Perl fixtures. It covered named grammar declarations and compound package/namespace forms rather than stopping at annotations.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R7-F2.01 / Java annotation types | Top-level/nested @interface A; elements/defaults and actual @A | Declaration A Usage; element name excluded, real default/use kept | Exclude annotation-type name; retain genuine reads | CLI/tree: nested 2:22 extra vs 2:59 required; top-level 1:12 extra |
| R7-F2.02a / simple Java modules | module m, open module n | Declared names Usage | No declaration-name occurrence | CLI/tree: m 1:8, n 2:13 extra |
| R7-F2.02b / qualified Java modules | module a.b | Currently hidden by R7-F3’s false imports | Exclude name tokens after repairing imports; keep directive reads | Grammar module_declaration.name scoped_identifier; masked case, not a clean exclusion |
| R7-F2.03 / Java packages | package a.b, simple control, later a/b types | Compound declaration a/b Usage | Exclude declaration components; keep later types | 1:9/1:11 extra vs 1:46/1:51 required |
| R7-F2.04 / C# static constructors | static G(), ordinary constructor, later G type | Static name Usage with Class/Constructor candidates | Exclude static declaration; retain type read | 1:18 extra vs 1:42 required; static constructor has no record by design |
| R7-F2.05a / C# custom events | event Action E {add{} remove{}} vs field-like event | Accessor-event E Usage; field-like name excluded | Exclude event name, retain type and later E read | 2:31 extra vs 2:54 required |
| R7-F2.05b / explicit-interface events | event Action IFace.E {add{} remove{}} | E declaration Usage; IFace/type reads kept | Exclude E only; preserve IFace/type | Error-free event_declaration.name tree |
| R7-F2.06 / C# external aliases | extern alias A/B; later A::T/B::U | Declaration aliases Usage | Exclude binders; retain real qualified usages | 1:14/1:30 extra vs 1:63/1:71 required |
| R7-F2.07 / C# block namespace | namespace A.B {...} | Both declaration components Usage | Exclude both; retain real A/B reads | 1:11/1:13 extra vs 1:47/1:52 required |
| R7-F2.08 / C# file namespace | namespace A.B; | Same extra components | Same exclusion; simple-name control remains clean | Independent CLI/tree and one compound Namespace record |
| R7-F2.09a / C++ nested namespace | namespace A::B, C::D::E | All declaration components Usage | Exclude every component; keep actual A/B method Calls | Two-depth CLI: 1:47/1:50 extra vs Calls 1:72/1:79 |
| R7-F2.09b / C++ inline namespace | namespace A::inline B | Both components Usage | Exclude A/B despite inline token | Error-free pinned tree, 1:11/1:21 extra |
| R7-F2.10a / Scala qualified package | package a.b; later a/b type reads | Declaration a/b Usage | Exclude declared path; keep types | 1:9/1:11 extra vs 4:17/4:32 required |
| R7-F2.10b / Scala braced package | package a.b {class G} | Same extra components | Same token exclusion | Independent CLI, error-free tree |
| R7-F2.10c / Scala stacked/plain packages | package a; package b on consecutive lines | Both simple names Usage | Exclude each package declaration | Independent CLI: 1:9/2:9 extra |
| R7-F2.11a / Perl qualified package | package A::B; later A()/B() | Both package segments Usage with sub candidates | Exclude declaration; retain Calls | 1:9/1:12 extra vs Calls 1:31/1:36 |
| R7-F2.11b / Perl versioned/braced package | package A::B v1.0 {sub f{}} | Same extra segments | Same exclusion; simple-package name remains excluded | Error-free independent CLI/tree |
| R7-F2.12a / TypeScript dotted namespaces | A.B and A.B.C, plain-name control, later A type | Declaration prefix A Scoped to class A | Exclude declaration prefix; retain true type read and lexical context | 1:22 extra vs 1:58 required; public cache stages preserve false ID |
| R7-F2.12b / TypeScript named-owner consumer | class A {}; function g(){ namespace A.B{} } | Declaration A is Scoped to class A, g.references includes A and A.referenced_by includes g | Exclude declaration token and both fabricated summary edges | Independent full-index CLI; this is parseable unfinished source, not compiler-valid namespace placement. Exclusion applies to declaration syntax regardless of compiler validation |
| R7-F2.13 / Swift macros | macro foo()=...; same-spelled function and actual foo() | Macro declaration foo Usage | Exclude macro name; preserve real Call and signature reads | macro_declaration(simple_identifier); 2:7 extra vs Call 3:1 |
| R7-F2.14 / Swift precedence groups | precedencegroup P; operator group P | Declaration P Usage with type candidate | Exclude declaration P; keep group reference | 2:17 extra vs 3:21 required; attribute markers are R7-F1 |
| R7-F2.15.{typealias,class,enum,protocol,struct,func,var,let} / Swift selective imports | Each of eight import selectors with Foo.Name, and later Name reads | Final imported binding Name Usage; simple-import control excluded | Exclude the binding token, retain actual later reads; no global name filter | Eight error-free selector probes; struct Bar at 2:19 extra vs real Bar 3:8 |
| R7-F2.16 / other named declarations | Java ordinary types/elements/generics/constructors; C# instance constructors/delegates/destructors/properties/patterns/simple namespaces; C++ unions/aliases/concepts/templates; Scala abstract members/givens/extensions; Swift protocol members; Perl prototypes | Tested declaration names excluded, actual type/default/body reads retained | Clean for these forms | Independent CLI controls, grammar-name inventory and current DECLARATION_EXCLUSION_MATRIX. Accessor keywords are not identifier captures |
| R7-F2.17 / remaining language sites | Rust/JS binding sites; Python alias/named-expression/pattern names; Go package/binders; C names; PHP trait/alias declarations; Lua locals/table names; shell operands/options | Current required exclusion/read rows pass | Clean for earlier inventory; preserve independently expected real reads | Full all-language declaration/initializer/parameter matrices plus Go/Python/PHP/shell CLI controls |

| Child ID | Decision/consumer | Required repair or checked invariant |
|---|---|---|
| R7-F2.C1 | Collection/name spans/exclusion | Add every missing declaration site without synthetic records; exclude actual child name-token spans for compound declarations. TypeScript must use its binding/exclusion truth. |
| R7-F2.C2 | Binding/categories/candidates | Suppress the TypeScript declaration prefix before it establishes the class ID, and limited-language tokens before NameOnly lookup. Preserve alias/category and genuine-read evidence. |
| R7-F2.C3 | Ownership/summaries/coverage/recursion | Independent .12b produces a g-to-A summary and its reverse edge. Shared exclusion must remove both as well as the occurrence. Preserve named/anonymous regions, recursion controls and limited-language uncertainty. |
| R7-F2.C4 | Filtering/projection/cache/widening | Independent six-stage API probes reproduce Java annotation, C# event/static constructor, TS namespace, Swift macro/selective-import errors. Assert exact declared-token absence and real-read presence at actual Hits, including omitted kinds. |
| R7-F2.C5 | Default/Call reports and selection | Cold/warm ID-selected JSON/text show the extra rows; TypeScript shows an established reference. Remove declaration Usages while preserving real Calls/Usages, name/ID selection and ambiguity. Call-only directions already reject these Usages. |
| R7-F2.C6 | Import repair dependency | R7-F3 currently masks qualified Java module declarations. Repair both decisions together: removing false imports must restore annotation/directive reads without exposing module declaration tokens. This is actionable implementation work, not a human blocker. |
| R7-F2.C7 | Tests/docs/comments/revision | Extend declared L1 exclusion/cache/CLI matrices with every row and positive/negative control, update current docs/affected comments and invalidate stale extractor results. Keep IDs/counts unless intentionally repairing a false declaration. |

**Required repair:** close .01–.15, including every named leaf, verify .C1–.C7 and preserve .16/.17. These are newly discovered siblings outside the earlier enumerated exclusion cases. High priority reflects the false established TypeScript reference as well as extra suggestions elsewhere.

### R7-F3 — Medium — Java qualified references are misclassified as imports and disappear

**Defect class:** an import reader accepts broad syntax captures without enforcing their context predicate, then removes genuine references as purported import bindings.

In tree-hugger, the [Java locals query](../../lib/queries/vendor/java/locals.scm:110) captures identifiers beneath **any scoped_identifier**, relying on a custom has-ancestor predicate for import_declaration. The [imported_symbols loop](../../lib/src/file/tree_file.rs:167), which constructs import evidence, does not enforce that predicate. [should_skip_import](../../lib/src/file/tree_file.rs:473) rejects package names and intermediate components of real imports, but returns false outside an import. [definition_ranges](../../lib/src/file/tree_file.rs:1996) then excludes the false imports from both occurrence evidence and the older public diagnostics-reference API.

```java
class A {}
@p.q.A class G {}
```

Changing only @A to @p.q.A removes A Usage instead of preserving it with path p.q.A. The complete index invents imports p/q/A; `hug imports` lists them. The class A’s ID-selected reference report becomes empty, on a cold miss and an actual warm hit. Both occurrence evidence and `TreeFile::referenced_symbols`, the library’s public diagnostics-reference reader, lose the real annotation reference. Arguments in a qualified annotation remain reads; its qualified name disappears.

The pinned Java grammar’s complete scoped_identifier parent inventory was swept. Module declaration names are declarations under R7-F2; module-directive operands are reads. All direct/qualified controls below parse without errors. The import capture is shared by every defective site, so a fix for annotations alone is incomplete.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R7-F3.01 / marker annotation | @A → @p.q.A | A Usage disappears; p/q/A become imports | Retain qualified annotation target and genuine qualifier evidence; no imports | One-edit CLI control, public referenced_symbols, default cold/warm reports |
| R7-F3.02 / annotation arguments | @p.q.A(value=external) | Name lost; value/external retained | Keep annotation target independently of real arguments | CLI/error-free annotation.name tree |
| R7-F3.03 / requires directive | requires a.b; simple control | Qualified names disappear into imports | Retain reference operands/path; no import bindings | Full-directive CLI/tree inventory |
| R7-F3.04 / exports package | exports p.q | Operands become imports | Retain referenced package; not a declaration | Same |
| R7-F3.05 / exports target modules | exports p.q to x.y,z.w | Both module paths become imports | Retain every target-module operand/path | Same, both comma-separated elements |
| R7-F3.06 / opens package | opens r.s | Operands become imports | Retain package references | Same |
| R7-F3.07 / opens target modules | opens r.s to t.u | Module path becomes imports | Retain target-module references | Same |
| R7-F3.08 / uses service | uses C → uses a.b.C | C disappears | Retain C Usage/path and real qualifier reads | One-edit control and public diagnostics-reference API |
| R7-F3.09 / provides service | provides C with D → qualified service | Provided type becomes imports | Retain provided-service target/path | Same control/edit |
| R7-F3.10 / provider list | d.e.D,f.g.F | Every provider disappears as imports | Retain every distinct provider | Full-directive source, both elements |
| R7-F3.11 / nested paths | Three-plus segments in every applicable slot | Each segment can become a phantom import | Enforce context at every nesting depth | Grammar inventory: annotations/imports/packages/modules/directives plus recursive scoped_identifier.scope |
| R7-F3.12 / genuine imports | Ordinary/static single and on-demand clauses | Imports reported from actual import nodes | Preserve real imports and existing final-component filtering | Eight-import sweep, fixture/public import controls |
| R7-F3.13 / declarations/other paths | package a.b, module a.b, regular qualified type paths, simple annotations/directives | Package imports skipped; module declaration falsely imported; scoped_type_identifier unaffected | No declaration imports; R7-F2 excludes declaration tokens. Real type and simple-reference controls retained | CLI/tree distinguishes name-node categories; module case currently masks missing exclusion |
| R7-F3.14.rs / Rust import reader | extern/use/alias/list captures | Captures structurally inside import syntax | Clean for false-import-context class; preserve aliases | [Effective Rust import patterns](../../lib/queries/vendor/rust/locals.scm:1); current public alias/binding rows |
| R7-F3.14.ecma / TS and JS readers | import_specifier and namespace_import | Structurally import-scoped | Clean; keep original-versus-alias decisions | [Shared ECMA import patterns](../../lib/queries/vendor/ecma/locals.scm:28); current imported-binding/default rows |
| R7-F3.14.go / Go reader | import_spec/import_spec_list | Structurally import-scoped | Clean; preserve aliases/path controls | [Go import patterns](../../lib/queries/vendor/go/locals.scm:64), public imports and all-language rows |
| R7-F3.14.py / Python reader | aliased_import, import_statement, import_from_statement | Structurally import-scoped | Clean; preserve actual names/read controls | [Python import patterns](../../lib/queries/vendor/python/locals.scm:23) plus current fixture/alias tests |
| R7-F3.14.cs / C# reader | using_directive captures | Structurally import-scoped | Clean for false ancestry; external alias exclusion is R7-F2 | [C# import patterns](../../lib/queries/vendor/c_sharp/locals.scm:82) and namespace/alias probes |
| R7-F3.14.scala / Scala reader | import_declaration | Structurally import-scoped | Clean for ancestry; package declarations are R7-F2 | [Scala import pattern](../../lib/queries/vendor/scala/locals.scm:93); package/import role comparison |
| R7-F3.14.swift / Swift reader | import_declaration | Structurally import-scoped | Clean for ancestry; final child exclusion is R7-F2 | [Swift import pattern](../../lib/queries/vendor/swift/locals.scm:1) and eight selective-import probes |
| R7-F3.14.php / PHP reader | namespace_use_clause | Structurally import-scoped | Clean for ancestry; retain genuine trait/member reads under existing contract | [PHP import pattern](../../lib/queries/vendor/php_only/locals.scm:75) and current import/marker controls |
| R7-F3.14.c / C reader | Effective import-definition patterns | No import-definition capture | Clean for this overcapture: the query cannot invent an import; this does not imply broader import support | Effective c locals query inventory and current occurrence fixtures |
| R7-F3.14.cpp / C++ reader | Effective import-definition patterns | No import-definition capture | Clean for this overcapture: the query cannot invent an import; this does not imply broader import support | Effective cpp locals query inventory and current occurrence fixtures |
| R7-F3.14.lua / Lua reader | Effective import-definition patterns | No import-definition capture | Clean for this overcapture: the query cannot invent an import; this does not imply broader import support | Effective lua locals query inventory and current occurrence fixtures |
| R7-F3.14.perl / Perl reader | Effective import-definition patterns | No import-definition capture | Clean for this overcapture: the query cannot invent an import; this does not imply broader import support | Effective perl locals query inventory and current occurrence fixtures |
| R7-F3.14.bash / Bash reader | Effective import-definition patterns | No import-definition capture | Clean for this overcapture: the query cannot invent an import; this does not imply broader import support | Effective bash locals query inventory and current occurrence fixtures |
| R7-F3.14.zsh / Zsh reader | Effective import-definition patterns | No import-definition capture | Clean for this overcapture: the query cannot invent an import; this does not imply broader import support | Effective zsh locals query inventory and current occurrence fixtures |

| Child ID | Decision/consumer | Required repair or verified invariant |
|---|---|---|
| R7-F3.C1 | Import collection/context/provenance | Enforce actual import ancestry or structurally scope the query. Sweep nested paths, package exclusion and true-import intermediate filtering together. Update provenance for query edits. |
| R7-F3.C2 | Definitions/exclusions/occurrences | Correct collection before definition_ranges removes real reads. Restore every required Usage/path before ownership/candidates. Keep explicit declaration exclusions from R7-F2. |
| R7-F3.C3 | Candidates/summaries/coverage/recursion | Restored Java reads can have NameOnly candidates, never invented IDs. Keep Limited coverage, empty established summaries and Unknown recursion. No resolver expansion. |
| R7-F3.C4 | Selective filtering/cache | Both missing-reference sources remain wrong through all six public cache stages, including semantic widening without a parse and actual projected Hits. Imports and evidence must both become correct; validation cannot fix a collector’s false source facts. |
| R7-F3.C5 | Public reference/diagnostics consumer | referenced_symbols independently loses qualified annotation/service references but retains unqualified controls. Repair its shared import input too; do not change severity/confidence/default enablement. |
| R7-F3.C6 | CLI refs/imports | imports invents annotation/directive entries; cold/warm ID-selected default text/JSON omit real suggestions. Assert restored offsets/paths and no imports in files without import clauses. These Usages remain outside callers/callees. |
| R7-F3.C7 | Tests/docs/comments/invalidation | Add all rows to declared L1 matrices, including diagnostics-reference API, import report and cache stages; update current docs/reader comments and invalidate old extractor evidence. |

**Required repair:** close .01–.11/.13 with correct context, preserve .12/.14, verify .C1–.C7, and coordinate declaration exclusions with R7-F2. This is a pre-existing import-reader sibling, not an initializer-repair regression.

## Blocked Findings

No required implementation repair is blocked by a human decision or unavailable environment. All three frontmatter findings are actionable.

The carried **R6-F1.17b** (Swift adjacent cb!!()/cb?!()) and **R5-F1.S12** (specialized explicit initializer paths) remain blocked by the pinned grammar. The specification’s Goal and boundaries section requires source plus the pinned grammar; the original implementation log’s accepted first-release breadth keeps Swift limited. These are **nonblocking grammar limitations outside that accepted release contract**, not permission to defer a supported form or any finding above. The prerequisites existed before repair and remain unchanged. Current reference docs record them; a grammar upgrade is optional future work. No human reaffirmation is required.

## Recurrence

- **R7-F1** repeats review 6’s **R6-F3 — Contextual syntax markers become false reference suggestions**. Java static value imports are a **repair regression** introduced by the legacy-type gate on a new import shape; Swift precedence directives/values are **newly discovered pre-existing siblings** outside the earlier ordinary-modifier examples. Missed decisions: import category, whole-file marker gate and precedence roles; missed consumers: default suggestions and every cache/projection stage. Original no-import Java/Lua cases pass.
- **R7-F2** repeats review 5’s **R5-F2 — Unreported declaration tokens leak into reference occurrences**, the exclusion part of `review-1.md — Named function expressions bind their names to outer declarations`, and the declaration-token part of `review-2.md — Pattern capitalization invents or omits Rust bindings`. These are **newly discovered siblings**, not unchanged earlier rows or demonstrated repair regressions. Missed sites: unreported named declarations and children of compound names across the full table; missed decisions/consumers: exclusion, supported TypeScript binding, candidates, summaries, imports, default reports and cache/projection.
- **R7-F3** repeats the source-role misclassification class of review 5’s **R5-F4 — Parameter captures turn type and default-value references into declarations**: a real reference becomes a binding before exclusion. It is a **newly discovered pre-existing import-reader sibling**, not a parameter-capture regression. Missed decisions: custom predicate enforcement, import ancestry and every scoped_identifier parent; missed consumers: public diagnostics references, imports/default reports and stored/projection results. Earlier Python/C#/PHP/Rust capture rows pass.

## Requirement Verification and Test Levels

| User-facing requirement | Strongest verification inspected/run | Assessment |
|---|---|---|
| Lexical order/categories/aliases/hoisting and item/dynamic/parameter boundaries; uncertain receivers | L1 scope/relation/occurrence/CLI matrices | Earlier required repairs pass; TS compound declaration exclusion is R7-F2. Accepted limited-language breadth preserved. |
| Calls versus Usages, wrappers, constructors and repeated sites | L1 all-language exact invocation/negative/summary/recursion/cache/CLI matrices plus independent Swift/C# probes | Supported R6 wrapper forms pass; adjacent Swift spellings remain explicit parser limits. |
| Declaration/import/marker exclusion, genuine reads, spans/paths/owners | L1 all-language matrices plus independent CLI/tree/API sweeps | R7-F1/F2/F3 remain correctness gaps despite passing earlier cases. |
| Shared parameter facts/defaults/arity/optionality and reference headers | L1 comment-position/signature/cache/header tests | R6-F2 and params/alias siblings verified fixed. |
| Honest recursion, summaries and ownership | L1 complete/narrowed recursion, relationship and ownership matrices | Prior required cases pass; new repairs must preserve regions and conservative coverage. |
| Filtering/projection/work avoidance, persistence/widening, malformed evidence and stale versions | L1 query/counter/cache/upgrade matrices and independent six-stage probes | Infrastructure passes; incorrect source facts persist at genuine Hits. |
| Exact/smart-case/ID selection, ambiguity/empty/errors, both Call directions and file-local boundary | L1 refs suite and independent cold/warm JSON/text | Selection contract passes; extra/missing incoming rows are the new findings. |
| Grouped terminal rows, wrapping, styling/links and boundary on empty reports | L2 real tmux capture, level2_declarations_in_tmux | Passed through live recipe without requesting focus. No full WezTerm/Kitty pass claimed. |
| Keyboard encoding/hotkeys | No such feature requirement | L3 unnecessary. |

Verification performed:

- `just test -j 4` in tree-hugger: **1,427 passed, 0 skipped**, 180.866 seconds.
- `just lint`: **passed** for library and CLI.
- `just test-l2 level2_declarations_in_tmux`: **1 passed**, 4.823 seconds; an actual refs rendering check.
- Root `just check-tier-coverage tree-hugger`: **zero stranded tests**. Declared library l1 and CLI symbols targets include the repair tests; terminal targets have enabled terminal-tests metadata and a live L2 recipe.
- Independent shipped-CLI controls/edits: every Java inference/import mode, Swift precedence directives/values and real reads, all R7-F2 declaration forms, and all R7-F3 scoped_identifier parent roles. Selected default reports were checked in JSON/text with cold Miss/warm Hit statistics; public imports and diagnostics-reference results were checked too. Primary fixture copies and error-free pinned trees anchor actionable source forms.
- Independent public cache API reproduced Java/Swift markers, Java/C#/TS/Swift declaration leaks and Java missing references through narrow Miss, narrow Hit, semantics widening without a parse, kind widening, complete Hit and projected Hit. Expected offsets/roles above, rather than output agreement, determine correctness.
- Independent Swift lint/tree probes confirm carried grammar errors and specialized-path loss, alongside grouped assertion Calls. Cargo.lock’s actual Swift version is 0.7.3.

Scratch probes were compiled outside the repository against the current library; they are not permanent regression tests. No implementation source, test target, formatting command or commit was changed by this review. Cross-OS evidence is left to CI and did not decide readiness.

## Input Robustness Matrix

This feature’s format reader is the persistent **JSON cache**. There is no new manifest, lockfile or configuration reader. The latest repair changes source-grammar classification and parameter extraction, not that strict cache reader. Its entry/record and reference-evidence mutation matrices were inspected and rerun through the permanent public CLI tests in the full L1 suite. Accepted coordinated-forgery limits remain unchanged; the new findings concern incorrect tool-produced source facts.

Each matrix restores a tool-written control before one edit and checks both the public result and actual hit/miss. O is an owned Scoped call, N a NameOnly member call, Q an unresolved qualified call, and R a limited region. H means an actual Hit; M means a miss with recomputation of the independently expected control; — marks an inapplicable element shape. Null/empty H controls are explicitly permitted values, not defaults for omitted keys. Duplicate-key edits repeat a key inside its object; trailing-content edits append invalid text. Entry/record coverage columns are also pinned in the permanent tests and current [cache design](../../docs/cache-design.md).

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

No additional performance gate, resolver breadth or architecture refactor is requested. Preserve indexed lookup, the shared definition/occurrence pipeline and record-only semantic widening. Grammar upgrades are optional future work; they do not block the three actionable repairs above.
