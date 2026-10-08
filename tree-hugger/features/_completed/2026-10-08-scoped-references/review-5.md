---
$schema: feature-review.yaml
ready: false
findings:
  - title: R5-F1 — Invocation classification still omits constructed and implicit member targets
    priority: high
  - title: R5-F2 — Unreported declaration tokens leak into reference occurrences
    priority: medium
  - title: R5-F3 — Receiver-qualified Java construction loses its written target
    priority: medium
  - title: R5-F4 — Parameter captures turn type and default-value references into declarations
    priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-06T19:33:28-07:00
spec: 2026-10-08-scoped-references/spec.md
implemented: true
description: "A **feature** review of `2026-10-08-scoped-references/spec.md`"
feature: 2026-10-08-scoped-references/review-5.md
previous: 2026-10-08-scoped-references/review-4.md
log: tree-hugger/features/2026-10-08-scoped-references/log.md
implemented_by: claude/opus
next: 2026-10-08-scoped-references/review-6.md
---

# Scoped References: review 5

The feature is **not production ready**. All previously enumerated repairs are verified fixed, including the remaining cache qualifier-loss cases and nine missing matrix columns. Four newly discovered sibling findings remain: missing calls, extra declaration references, lost Java constructor receivers, and false parameter declarations that suppress real references and cause selector ambiguity. None requires a human decision.

This review read the specification, plan, original implementation log, every earlier review, and the repair ledger in [log.md](./log.md). Completion claims were checked against code and public results. Existing implementation changes were preserved; this review changes only this file, the previous review's `next` link, and the specification's iteration count.

## Previous Finding Verification

Legacy identifiers retain the filename and exact title. **Verified fixed** closes the earlier enumerated instances and consumers; it does not claim that every possible sibling in the class was previously covered. The new sibling inventory follows this table.

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
|---|---|---|---|---|
| `review-1.md — Lexical lookup crosses boundaries that do not establish a binding` | Rust inline modules, nested functions/constants/statics, closure controls, conditional chains/else/following statements and loop controls; TS/JS `with`, direct eval and defaults; occurrences, summaries, recursion, relation lookup and CLI | **verified fixed** | Full L1 run passes `LEXICAL_BOUNDARY_MATRIX`, boundary summaries, recursion, relation and CLI rows. Scope boundaries and conditional visibility endpoints are shared by occurrence and relation lookup. | No unchanged required case remains. |
| `review-1.md — Named function expressions bind their names to outer declarations` | TS/JS ordinary and generator expression names, defaults/internal calls, final outer calls, unnamed/class/arrow controls; exclusions, placement, summaries, recursion, relation lookup and callers | **verified fixed** | `EXPRESSION_NAME_MATRIX`, expression summary/recursion/relation tests and `refs::named_function_expressions_never_call_the_outer_declaration` pass. Expression-local bindings are inventoried and excluded even without a declaration record. | R5-F2 contains new declaration-exclusion siblings in other syntax; the original expression cases are not failing. |
| `review-1.md — Invocation classification loses calls across supported source forms` | Direct/wrapped calls in all 16 languages, TS type-only wrappers, member/generic wrappers, Perl/shell calls, receiver/argument/sequence/conditional/dereference negatives; reports, recursion, terminal-call and CommonJS readers | **verified fixed** | Invocation/negative matrices, wrapped-binding and CLI tests pass; terminal-call and CommonJS tests pass. The subsequently accepted PHP constant/property exception and Go indexing limitation are preserved. | R5-F1 is a new constructor/implicit-member sibling, rather than a regression of these source forms. |
| `review-1.md — Cache validation accepts inconsistent reference evidence` | Wrong/malformed/lost retained target IDs, named/ownerless targets, owner/candidate identity, duplicate/omitted callable regions, structural matrix, summaries/dependencies/semantics, null and narrowed controls | **verified fixed** | Public cache mutation tests and retained-evidence tests pass; required keys, retained identities, two-way edges, semantic recomputation and callable regions are enforced. Independent JSON matrix also passes. | None for the required instances. Previously accepted anonymous-evidence and unrelated relation-validation limits remain unchanged. |
| `review-2.md — Parameter defaults incorrectly see body-local var bindings` | Seven callable forms in TS/JS; ordinary/block/destructured/loop `var`; parameter visibility, redeclaration, hoisting, computed/destructured defaults and body eval; let/const/function controls, relation lookup and reports | **verified fixed** | `PARAMETER_DEFAULT_FORMS × BODY_DECLARATIONS`, `PARAMETER_ENVIRONMENT_MATRIX`, relation tests and `refs::parameter_defaults_call_the_outer_declaration_not_a_body_var` pass. | R5-F4 concerns false declaration captures in Python/C#, not the repaired ECMA parameter environment. |
| `review-2.md — Pattern capitalization invents or omits Rust bindings` | Bare/modified capitalized bindings, lowercase constants in patterns/body/guards, paths/tuples/shorthands and irrefutable controls; walker, capture fallback, exclusions, declaration records, summaries, recursion and callers | **verified fixed** | `PATTERN_IDENTITY_MATRIX`, `PATTERN_RECORD_MATRIX`, summary/recursion tests and CLI pattern tests pass. Pattern placement and record exclusion agree. | R5-F2/R5-F4 identify new exclusion/capture siblings outside the earlier Rust/ECMA inventory. |
| `review-2.md — Transparent call wrappers incorrectly limit coverage` | Rust parentheses/generics/type paths, all enumerated TS wrappers, JS parentheses and constructors; direct/self/member/value/sequence/conditional/dereference controls, eval, recursion and shared relation scopes | **verified fixed** | `CALLEE_COVERAGE_MATRIX`, complete/narrowed `RECURSION_MATRIX`, shared operand-reader and eval tests pass. Calls to another known declaration remain complete/`No`. | None for these required cases. New limited-language constructor classifications are R5-F1. |
| `review-2.md — Cache validation accepts nonexistent evidence identities and missing callable regions` | Every-language phantom owners, applicable candidates/targets, owned/ownerless targets, Python/Lua empty callable regions, summaries, callable/narrowed entries and null/bodiless controls | **verified fixed** | `every_kind_evidence_identities_name_retained_declarations_in_every_language` and retained-evidence tests pass. Missing retained IDs and required empty body regions are refused. | None. |
| R3-F1 — TypeScript instantiation expressions hide calls and constructor invocations | `.01–.18`, `.C1–.C7`, ledger `.R`: wrapper/member/constructor operands, usages/arguments and all-language controls; classification, candidates, summaries, coverage/recursion, projection/widening, CLI and revision invalidation | **verified fixed** | Invocation/coverage/recursion rows, exact member candidates, `instantiation_expression_calls_survive_narrow_widened_and_warm_entries`, CLI instantiation test and revision-41 rejection pass. | R5-F1 enumerates other grammars' **constructed type** nodes, not TS instantiation expressions. |
| R3-F2 — Rust associated value calls falsely prove absence of recursion | `.01–.16`, `.C1–.C5`: inherent/trait/enum/generic/wrapped value paths and unrelated initializer controls; function/constructor/usage controls, candidate fallback, coverage, summaries, narrow recursion, semantic widening and callees | **verified fixed** | `VALUE_CALL_MATRIX`, recursion rows, value-summary test, `stored_value_calls_widen_to_unknown_recursion` and CLI coverage test pass. Both unique-member and uncertain-candidate decisions check value kinds. | None; initializers remain unfollowed. |
| R3-F3 — Cache validation accepts false source facts and misordered or duplicate lists | `.L01–.L16.{region,path,candidates}`; `.S01–.S03.{rs,ts,js}.{duplicate,reverse}`; `.S04–.S10`; `.S09r.{rs,ts,js,go,py,java,cs,c,cpp,swift,scala,php,lua,perl,sh,zsh,omitted}`; `.C1–.C4`; `.M.kind` and eight owner/coverage coordinate leaves | **verified fixed** | Both all-language source-fact/list mutation tests pass. `deleted_written_paths_are_recomputed_in_every_language` proves control hits and separate path/start null edits becoming misses. Nonmember full-shape, complete/narrow/projected, omitted-member and text/JSON tests pass. All nine `.M` leaves are present in the permanent matrix and passed. Independently swept 55 columns/395 cells with no failures. **Perl's positive-path prerequisite was removed before the cache repair:** R4-F1 restored package/receiver paths at 19:02; the cache repair ran at 19:14–19:25 and now tests both Perl forms. | None. Deleting both a path and its redundant start is the coordinated-forgery limit expressly excluded by review 4's required repair, not a newly accepted single-field deferral. |
| R3-F4 — Optional and tagged eval calls incorrectly hide lexical declarations | `.01–.07`, `.C1–.C5`: TS/JS optional/tagged/wrapped eval, genuine direct/sequence/member/alias controls; scope barriers, declared bases, summaries/candidates, coverage/recursion, filtering/cache/callers | **verified fixed** | `EVAL_CALL_FORMS`, boundary/coverage/recursion/relation assertions, summary-edge, narrow/widen/warm and CLI callers tests pass; both scope and coverage use the same direct-eval decision. | None; accepted conservative treatment of genuine direct eval is unchanged. |
| R4-F1 — Written-path collection drops qualifiers and can bind rooted names to locals | `.01–.28`, `.C1–.C7`: roots, nested C++ paths/generics, C# alias/conditional/generic paths, PHP nullsafe/namespaces, Perl package/receiver paths and all-language controls; exclusions, lookup/category/stopping, candidates/summaries, recursion, projection/cache/reporting/docs/revision | **verified fixed** | `WRITTEN_PATH_MATRIX` explicitly pins all prior rows and additional Java/Scala/Swift/Go type paths. Rooted binding/summary/recursion tests, path narrow/widen/warm tests, text/JSON callers/callees test and revision-42 invalidation pass. Current rooted Rust calls preserve `::f` and never summarize a shadowing local. | R5-F3 is a new receiver-bearing Java construction sibling. The earlier `new a.b.C()` path is still correct. |
| R4-F2 — Invocation classification omits calls and mistakes PHP callable creation for execution | `.01–.24`, `.C1–.C6`: rooted calls, C# conditional/generic/delegate chains, PHP placeholder versus unpacking for named/qualified/static/instance/variable forms, variable constructors, Perl code-reference and name-only negatives; coverage, candidates/summaries, filtering/cache/reports/docs/revision | **verified fixed** | Exact invocation/negative and limited-language rows pass, as do narrow/widen/projected/warm results and `refs::invocation_kinds_decide_callees_callers_and_the_default_report`. PHP creation remains Usage; real variable calls/unpacking and Perl code-reference calls are Call. Terminal-call/CommonJS controls pass. | R5-F1 carries new constructed/implicit-member nodes outside the earlier rows; none of `.01–.24` regressed. |

No earlier finding was blocked by a human decision. The original implementation log's “Rulings (accepted for implementation)” established Rust/TS/JS resolution, conservative evidence in the other languages, no receiver inference and file-local analysis before these repairs. Those decisions are not reopened. The repair ledger's earlier single-field path-loss deferral has been withdrawn and the required redundant source fact is implemented. The newly repaired Perl cache rows were unblocked by collection support **before** their cache repair, as recorded above.

## Unblocked Findings

### R5-F1 — High — Invocation classification still omits constructed and implicit member targets

**Defect class:** invocation classification follows function instantiations but omits several constructed-type, braced-construction and implicit-member operand shapes, so syntactic calls remain usages and disappear from call reports.

In the tree-hugger library, [is_call_target](../../lib/src/analysis/occurrences.rs:1209) decides which captured names are calls, while [generic_operand](../../lib/src/analysis/occurrences.rs:1440) identifies the name inside an instantiation. C++ `template_type` is crossed by the path reader but not by this invocation reader. Java and Scala `generic_type` are also absent. Swift generic construction uses a separate `constructor_expression` with `constructed_type`, and Swift `.f()` / `.init()` / `.some(1)` put their callee in a `prefix_expression`; none reaches the invocation decision. C++ `C{}` uses `compound_literal_expression`, even when `C` declares a constructor.

For example, the public complete index reports the first two `C` names as usages and the last as a call:

```java
class C<T> { C() {} }
class G { void g() { new C<String>(); new a.b.C<String>(); new C(); } }
```

`hug refs --id <g-id> --callees --json` lists only `new C()`. Selecting the class's ID with `--callers` likewise loses the generic constructor suggestions. All names were already captured; this is not a request to infer receivers or constructor overloads.

The tables below use CLI complete-index probes, copies of primary fixtures with one appended probe, and pinned-grammar trees. Adding/removing type arguments changes the classification. C++ sources with explicitly declared constructors passed `clang++ -std=c++20 -fsyntax-only`; Swift generic construction and implicit `.f()` passed `swiftc -typecheck`. Generic arguments and receiver names have independently expected Usage results. Coverage remains limited and recursion Unknown in these languages; that does not excuse a wrong kind.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R5-F1.01 / Java generic construction | `new C<String>()`, `new a.b.C<String>()` | Final `C` Usage | Final `C` Call; qualifier prefixes and `String` Usage | CLI, copied Java fixture, `generic_type` tree |
| R5-F1.02 / Java diamond | `new C<>()`, `new a.b.C<>()` | `C` Usage | `C` Call | CLI; same constructor node with empty type arguments |
| R5-F1.03 / Java generic inner construction | `new a.b.C<String>.D<Integer>()` | Final `D` Usage; prefix/type arguments Usage | Only final `D` Call | CLI and grammar tree |
| R5-F1.04 / Java generic receiver/anonymous/annotation forms | `outer.new C<String>()`, `new C<String>() {}`, `new @A a.b.C<String>()` | Final `C` Usage | Final `C` Call; receiver/annotation/type arguments stay Usage | CLI; nongeneric annotation/receiver controls call correctly |
| R5-F1.05 / Scala generic construction | `new C[Int]()`, `new a.b.C[Int]()` | `C` Usage | `C` Call; type arguments/prefixes Usage | CLI, copied Scala fixture, `instance_expression > generic_type` tree |
| R5-F1.06 / Scala omitted argument list/anonymous body | `new C[Int]`, `new a.b.C[Int] {}` | `C` Usage | Constructed `C` Call, as for nongeneric `new C` | CLI; nongeneric control passes |
| R5-F1.07 / Swift specialized construction | `C<Int>()`, `N.C<Int>()` | `C` Usage | `C` Call, `N`/`Int` Usage | CLI, copied Swift fixture, compiler and `constructor_expression` tree |
| R5-F1.08 / Swift implicit-member invocation | `let _: C = .f()` versus `.f` | Both `f` Usage | Invocation `f` Call; value `f` Usage | CLI and compiled enum/static-function example; `prefix_expression` tree |
| R5-F1.09 / Swift implicit initializer/enum case | `let _: C = .init()`, `let _: E = .some(1)` | `init` / `some` Usage | Constructed target Call; argument Usage | CLI and grammar tree; explicit `C.init()` is a clean Call control |
| R5-F1.10 / C++ generic allocation | `new C<int>()`, `new N::C<int>()`, `new C<int>`, `new C<int>{}` | `C` Usage | `C` Call; preserve any qualifier | CLI, copied C++ fixture, compiler and `new_expression > template_type` tree |
| R5-F1.11 / C++ braced construction | `Plain{}`, `N::Plain{}`, `C<int>{}`, `N::C<int>{}` with declared constructors | Constructed name Usage | Constructed name Call, with conservative target/candidates | CLI and compiled nonaggregate constructor example; `compound_literal_expression` tree |
| R5-F1.12 / C++ functional-construction controls | `Plain()`, `C<int>()`, `(N::C<int>)()` | Call | Preserve Call | CLI; existing function-instantiation climb |
| R5-F1.13 / Java/Scala/Swift nongeneric controls | `new C()`, `new a.b.C()`; Scala `new C`; Swift `C()`, `N.C()`, `C.init()` | Call | Preserve Call | CLI and prior passing matrices |
| R5-F1.14 / Rust | `C::<u8>(1)`, `N::C::<u8>(1)` | Call; argument Usage | Clean | CLI and invocation/coverage tests; existing generic-function operand rule |
| R5-F1.15 / TypeScript/JavaScript | `new C<number>()`, `new N.C<number>()`, `new (C<number>)()`; JS direct/wrapped construction | Call; arguments/prefixes Usage | Clean | CLI plus TS-instantiation and all-language matrices |
| R5-F1.16 / C# | `new C<int>()`, `new N.C<int>()`, `new C()` | Call | Clean; type arguments Usage | CLI; `generic_name` operand rule |
| R5-F1.17 / PHP | `new N\C()`, `new \N\C()`, `new $C()`; callable creation/unpacking controls | Constructed name/value Call; creation Usage | Clean | CLI plus repaired invocation matrix |
| R5-F1.18 / Python | Named/qualified calls and decorator calls | Call; receiver/arguments Usage | Clean | All-language invocation/negative tests and CLI probes |
| R5-F1.19 / Go | Ordinary `pkg.F()`; `pkg.F[int]()` | Ordinary Call; indexed form has no established called-name classification | Preserve the accepted indexing/generic limitation | CLI, prior invocation rows and documented grammar ambiguity; no new resolution request |
| R5-F1.20 / C | Direct/wrapped function-pointer calls | Call; explicit dereference remains Usage | Clean; no generic constructed-type syntax | Invocation/negative matrices and CLI |
| R5-F1.21 / Lua/Perl/Bash/Zsh | Named/value/receiver calls or command names; argument/name-only negatives | Expected Call/Usage split | Clean; no corresponding generic constructor wrapper | Invocation/negative matrices, R4 repairs and CLI |
| R5-F1.22 / value/type/argument controls | Type annotations `C<T>`, references passed as arguments, Swift `.f`, sequences and dereferences | Usage | Preserve Usage; never label every name in a constructed type as Call | CLI probes and negative matrices; Java/Scala argument nodes are distinct from the constructed operand |

| Child ID | Decision/consumer | Verified effect and required repair |
|---|---|---|
| R5-F1.C1 | Collection, constructed operand and invocation classification | Names exist with correct token spans; traverse only the constructed name, not type arguments/receivers. Add the listed constructor and implicit-member grammar shapes. |
| R5-F1.C2 | Binding, categories, candidates and summaries | All defective languages remain conservative; candidates must not become target IDs, and their summaries stay empty. Classifying the invocation requires no new language resolver. |
| R5-F1.C3 | Coverage and recursion | Limited-language coverage/Unknown recursion is correct and must remain. Supported-language generic/value/wrapper recursion controls pass. |
| R5-F1.C4 | Kind/name filtering, projection, storage and widening | Independent public-API probes preserve the wrong kinds through narrow queries, `WidenedKinds`, record-only semantic widening (`parsed: false`), projected hits and full warm hits. Assert independently expected kinds after each stage. |
| R5-F1.C5 | Default reports, callers and callees, text/JSON | Both Call-only directions drop these sites; default incoming suggestions mislabel them Usage. Independent Java/Scala/Swift/C++ warm CLI runs prove actual hits with zero parses. Add exact row assertions, including absence of extra calls on argument/value controls. |
| R5-F1.C6 | Other readers, documentation and invalidation | Terminal-call and CommonJS readers' existing tests pass and are separate from this classifier. Add permanent rows to the existing matrices, update affected invocation docs/comments/provenance if queries change, and invalidate old extractor results. |

**Required repair:** cover every defective and control child above in L1 occurrence and public report tests. The repair should classify the grammar's constructed/called operand, not broadly promote type names. Preserve the accepted limited-language resolution boundary and Go/PHP exceptions.

### R5-F2 — Medium — Unreported declaration tokens leak into reference occurrences

**Defect class:** declaration exclusion depends on emitted records and a partial fallback rather than all syntactic declaration sites, so an unreported declaration becomes a reference occurrence.

In the tree-hugger library, [Resolver::occurrence](../../lib/src/analysis/occurrences.rs:489) excludes recorded declarations and the supported languages' binding sites. For the other languages it falls back to [is_definition_like_reference](../../lib/src/file/extract/classify.rs:837), whose recognized declaration-parent list does not cover the sites below. Generic metadata readers already identify many parameter names, but those facts do not reach occurrence exclusion. Missing declaration records are permitted; counting their declaration tokens as reads is not.

A small, parseable example produces two `x` usages instead of one:

```python
def g():
    cb = lambda x: x
```

The first token declares the parameter; only the body token reads it. Likewise, the `T` in Java `class G<T> {}` or Go `type G[T Bound] struct {}` is a declaration, while a later `T` in a field/parameter type is a reference. The repair must preserve those later reads, constraint/default expressions, iterables, initializers and body calls.

This table is the complete required inventory from the declaration-site sweep. Each defective row was run through `hug symbols --json-full --no-cache` on isolated source and applicable copied fixtures. Checks compare **the declaration token's source offset**, separately from subsequent reads. Scala value/pattern binders use lowercase names; uppercase stable-pattern identifiers are not mislabeled as bindings. All named clean controls retain their reads or have an explicit syntax/capture invariant.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R5-F2.01 / Java generic declarations | Class/interface `<T>`, method `<U extends Bound>` | Parameter-name token Usage | Exclude the declaration token; retain Bound and later U usages | CLI and copied Java fixture; generic reader identifies the declaration name |
| R5-F2.02 / Java compact constructor | `record R(int x) { R {} }` | Compact constructor's `R` Usage | No declaration-name occurrence | CLI; class/record and ordinary constructor names are excluded controls |
| R5-F2.03 / Java statement label | `label: while (...) { break label; }` | Label definition Usage | Exclude definition; preserve captured break-label usage | CLI with distinct token offsets |
| R5-F2.04 / Scala generic declarations | Class invariant/`+U`/`-V`, function type parameters and bounds | Type-parameter declaration tokens Usage | Exclude only parameter names; retain bounds and later type usages | CLI and copied Scala fixture |
| R5-F2.05 / Scala scalar enumerator | `for (x <- xs) use(x)` | Binder and body x both Usage | Exclude binder; retain iterable and body read | CLI, lowercase pattern reproduction |
| R5-F2.06 / Scala tuple-enumerator control | `for ((a,b) <- xs) use(a)` | Binder names excluded; body read present | Clean | CLI; locals query captures these tuple names |
| R5-F2.07 / Scala match bindings | `case Some(x) => use(x)`; `case x @ Some(y) => use(x)` | Capture/`@` names Usage at declaration offsets | Exclude binders; preserve constructor/stable names and body reads | CLI; uppercase stable identifiers deliberately excluded from this repair inventory |
| R5-F2.08 / Scala value patterns | `val (a,b)=xy`; `val Some(c)=x` | Declared a/b/c Usage | Exclude binders; retain initializer/constructor references | CLI and copied fixture; ordinary/grouped val controls have recorded names |
| R5-F2.09 / Scala abstract type | `trait T { type Element }` | Element Usage | No declaration-token occurrence | CLI; the existing alias query requires a right-hand side, so no record supplies exclusion |
| R5-F2.10 / Go generic declarations | `type G[T Bound] ...`; `func f[U Bound](u U) U` | Declared T/U Usage | Exclude names; retain Bound and later U usages | CLI and copied Go fixture |
| R5-F2.11 / Python type parameters | PEP 695 class/function/alias lists; bounded T, `*Ts`, `**P` | Declaration-name tokens Usage | Exclude names; retain constraints and type/body usages | CLI and copied Python fixture |
| R5-F2.12 / Python type-alias name | `type Alias[T] = T` | Alias token Usage | No declaration-name occurrence; right-hand T remains a usage | CLI; no alias record is currently emitted |
| R5-F2.13 / Python plain lambda parameter | `lambda x: x` | Both x tokens Usage | Only body x Usage | CLI and narrow/widen/projected/warm public API |
| R5-F2.14 / Python annotated variadics | `def g(*args: Bound, **kwargs: Bound)` | args/kwargs declaration tokens Usage | Exclude binders; retain annotation reads | CLI; unannotated variadic names are captured by the existing locals query |
| R5-F2.15 / Python structural patterns | Sequence capture, mapping value/`**rest`, class positional capture, starred sequence and `as` alias | Pattern declaration tokens Usage | Exclude captures; retain matched object, class and body reads | CLI, explicit match rows and copied Python fixture |
| R5-F2.16 / Python starred assignment | `a, *b = xy` | b declaration Usage; a excluded | Both binders excluded; initializer and later reads kept | CLI; ordinary tuple assignment is the positive exclusion control |
| R5-F2.17 / C++ template variants | Defaulted type parameter, type pack and defaulted template-template parameter | U/Ts/TT declaration names Usage | Exclude declaration names; retain Bound/Box/default-value references | CLI and copied C++ fixture |
| R5-F2.18 / C++ template controls | Plain type parameter, non-type parameter, nested plain template-template name | Names excluded | Clean; preserve default expressions and type references | CLI; these names are already captured as records |
| R5-F2.19 / C++ range declarations | `for(auto x : xs)`; structured range `for(auto [a,b] : xs)` | Declaration-name tokens Usage | Exclude binders; preserve iterable/body references | CLI and copied C++ fixture |
| R5-F2.20 / C++ structured declaration | `auto [a,b] = xy` | a/b Usage | Exclude binders; preserve initializer/body reads | CLI |
| R5-F2.21 / C++ unreported named declarations | `namespace Alias=Other`; inline `~C(){}` | Alias and destructor declaration C Usage | Exclude declared names; retain Other | CLI; ordinary class/namespace/concept names are clean controls |
| R5-F2.22 / C# catch/pattern declarations | Catch exception name; `x is C p`, `x is var p`, switch `case C p` | Binder token Usage | Exclude binders; retain type/scrutinee/body reads | CLI and copied C# fixture |
| R5-F2.23 / C# captured-binding controls | Lambda parameter; foreach scalar/tuple; tuple deconstruction; generic parameter names | Names excluded, real reads preserved | Clean | CLI; generic where-clause subject reads remain usages |
| R5-F2.24 / C# unreported named declarations | Delegate name, destructor name and statement label | Declaration-name token Usage | Exclude names; retain captured label use and signature type reads | CLI; parameter type/default loss is separately R5-F4 |
| R5-F2.25 / Swift closure parameter | `{ (x: Int) in x }` | Parameter declaration x Usage | Exclude binder; retain Int and body x | CLI and copied Swift fixture |
| R5-F2.26 / Swift declaration labels | `func g(label local: Int, _ other: Int)` | External label and `_` Usage | Exclude declaration-label tokens; retain Int and local/other body reads | CLI with exact declaration and body offsets; internal parameter records are present |
| R5-F2.27 / Swift parameter pack | `struct Pack<each T> {}` | Declared T Usage | Exclude name | CLI; name is nested in a user type, unlike ordinary type-parameter names |
| R5-F2.28 / Swift controls | Ordinary generic names, property/loop/catch binders, associatedtype name, initializer declaration, statement-label definition | Names excluded or not captured; required reads preserved | Clean | CLI; ordinary generic/associatedtype declaration name is outside the reference query's user-type pattern |
| R5-F2.29 / PHP catch declaration | `catch(Exception $e) { useit($e); }` | Declaration e Usage | Exclude binder; retain Exception/body e | CLI and copied PHP fixture |
| R5-F2.30 / PHP destructuring declarations | `[$a,$b]=$xy`; `list($a,$b)=$xy` | a/b declaration tokens Usage | Exclude declaration targets; retain initializer/body reads | CLI; foreach key/value and ordinary/arrow parameters are clean controls |
| R5-F2.31 / Perl declared loop iterator | `for my $x (...)`; `foreach our $x (...)` | Iterator declaration Usage | Exclude declared iterator; retain iterable/body reads | CLI and copied Perl fixture; ordinary my declarations are excluded; local override remains a reference |
| R5-F2.32 / Lua literal named field | `local M={field=external}; use(M.field)` | Literal key field Usage as well as member read | Exclude the literal field declaration; retain external and later member access | CLI; computed keys and property accesses are reads, not this declaration form |
| R5-F2.33 / Rust controls | Parameters, patterns, generics, named items, imports and labels | No declaration occurrences; constant-pattern reads preserved | Clean | Passing binding/pattern/exclusion matrices and precise shared-site placement/exclusion invariant |
| R5-F2.34 / TypeScript controls | Parameters/defaults, patterns, generics, classes/functions/expressions, aliases and imports | Binding tokens excluded; type/default/body reads preserved | Clean | Passing expression/pattern/default matrices and CLI probes |
| R5-F2.35 / JavaScript controls | Parameters, destructuring, functions/classes/expressions and imports | Binding tokens excluded; default/body reads preserved | Clean | Same shared binding-site inventory; passing tests and default CLI probe |
| R5-F2.36 / C controls | Parameters, for variables, locals, typedef, enum/variant/struct/field declarations and labels | No declaration-token occurrences; initializer/body reads preserved | Clean | CLI; C label nodes are not identifier captures |
| R5-F2.37 / Bash controls | For variable and assignment names versus expansions | Declaration tokens excluded; expansions remain usages | Clean | CLI and command/argument matrix; expansion tokens are genuine reads |
| R5-F2.38 / Zsh controls | Same loop/assignment/expansion distinction | Same clean result | Preserve | CLI and shell matrices |

| Child ID | Decision/consumer | Verified effect and required repair |
|---|---|---|
| R5-F2.C1 | Declaration-name inventory, source spans and exclusion | The first name token in each defective row declares something; exclude that exact span without deleting constraints, defaults, iterables, initializers or later reads. Do not require emitting additional public records merely to exclude a name. |
| R5-F2.C2 | Lookup, category filtering and candidate fallback | Defective rows are in limited languages: leaked tokens become Unresolved or same-name NameOnly suggestions, never proof of a lexical read. Removing them must happen before candidate selection. Preserve Rust constant-pattern/stable Scala identifier reads. |
| R5-F2.C3 | Summaries, ownership, coverage and recursion | Unsupported-language summaries remain empty and recursion Unknown. Preserve anonymous owner boundaries and whole-file coverage; an empty occurrence list is not permission to claim supported resolution. |
| R5-F2.C4 | Filtering, projection, cache and widening | Java generics, Python patterns/lambda and lowercase Scala binders survive function-kind narrowing, kind widening, record-only semantic widening, projection and actual warm hits in public-API probes. Add exact token-offset exclusions and required-read assertions at those stages. |
| R5-F2.C5 | Reference reports and other declaration consumers | Independent `class T {} class G<T> { T f(T x){ return x; } }` lists incoming T suggestions at offsets 19, 24 and 28. Only 24 and 28 are type reads; 19 declares the generic parameter. Call-only directions correctly reject these Usage tokens. Assert the default report excludes offset 19 while retaining the two reads. No new declaration IDs or resolution breadth are required. |
| R5-F2.C6 | Regression inventory, comments/docs and invalidation | Current matrices test supported bindings and selected limited-language examples, not this inventory. Add all defective/control rows to declared L1 targets, update exclusions documentation and affected comments, update query provenance if changed, and invalidate cached evidence. |

**Required repair:** make syntactic declaration exclusion complete for this inventory in every language, independently of whether a declaration becomes a record. Keep genuine references alongside the excluded tokens. This does not request resolving the other 13 languages or adding the missing declarations to their public symbol sets.

### R5-F3 — Medium — Receiver-qualified Java construction loses its written target

**Defect class:** written-path collection recognizes type/namespace qualification but omits a receiver carried by the construction node, dropping source evidence before reports or cache storage.

In the tree-hugger library, [written_path and qualified_parent](../../lib/src/analysis/occurrences.rs:1090) preserve `a.b.C`, but do not inspect Java's `object_creation_expression`. The pinned grammar keeps the receiver as an unfielded child before the constructed type. Thus `outer.new C()` reports a Call of plain `C`, although the source names the inner class through `outer`.

```java
class G {
    void g() { outer.new C(); (outer).new C(); factory().new C(); }
}
```

The independent expected path is the source from the receiver through the name token: `outer.new C`, `(outer).new C`, `factory().new C`. The `new` token inside the path is retained just as `template` is retained in C++ receiver paths. The occurrence span remains only `C`. No enclosing-instance inference or unique target ID is requested.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R5-F3.01 / Java simple receiver | `outer.new C()` | Call C, path absent | Path `outer.new C` | CLI and copied Java fixture; receiver precedes type in construction tree |
| R5-F3.02 / Java wrapped receiver | `(outer).new C()` | Call C, path absent | `(outer).new C` | CLI and grammar tree |
| R5-F3.03 / Java this receiver | `this.new C()` | Call C, path absent | `this.new C` | CLI; no fabricated occurrence for keyword this |
| R5-F3.04 / Java evaluated receiver | `factory().new C()` | Both factory and C Call, C path absent | Preserve factory call separately; C path `factory().new C` | CLI; distinct name-token spans |
| R5-F3.05 / Java chained receiver | `holder.outer.new C()` | outer has `holder.outer`; C has no path | C path `holder.outer.new C` | CLI; intermediate prefix is independently correct |
| R5-F3.06 / Java generic receiver | `outer.new C<String>()`, `(outer).new C<String>()` | No path; also Usage under R5-F1.04 | `outer.new C` / `(outer).new C`, Call after R5-F1 repair; String Usage | CLI, copied fixture and generic construction tree |
| R5-F3.07 / Java receiver with anonymous body | `outer.new C() {}` | Call C, path absent | `outer.new C` | CLI; class body must not enter the path/span |
| R5-F3.08 / Java ordinary path controls | `new N.C()`, `new a.b.C<String>()`, nested generic type paths | Full name-ending path | Clean; preserve paths even while R5-F1 repairs kind | CLI and passing written-path matrix |
| R5-F3.09 / other constructed-path readers | Rust/C++ `N::C...`, TS/JS/C# `N.C...`, Swift `N.C<Int>()`, Scala `a.b.C[Int]`, PHP namespace/value construction | Namespace-qualified names keep their paths; standalone variables have no receiver path | Preserve existing paths and nulls; repair invocation kinds separately under R5-F1 | CLI, all-language path matrix and construction probes; these type/path nodes directly contain the qualifier |
| R5-F3.10 / remaining languages | Python/Go/Lua/Perl qualified callable receivers; C/Bash/Zsh applicable direct/member/command forms | Existing expected path/name representation | Clean; none uses Java's receiver-before-new construction node | Passing written-path rows and grammar-shape invariant; shell command words retain their entire spelling |

| Child ID | Decision/consumer | Verified effect and required repair |
|---|---|---|
| R5-F3.C1 | Collection and source range | Read the receiver only for the constructed type's final name, retain through the name token, and keep receiver/argument/anonymous-body references separate. |
| R5-F3.C2 | Categories, candidates and lookup stopping | Java remains NameOnly/Unresolved and limited. A preserved receiver must not become evidence of a unique target or establish summary edges. |
| R5-F3.C3 | Reports and call selection | Nongeneric C already appears in callees/incoming suggestions but prints only C; generic rows also depend on R5-F1. Test text/JSON default, callers and callees with the full written target. |
| R5-F3.C4 | Projection, persistence, widening and invalidation | Independent public API reproduces null paths in narrow, kind-widened, semantic-widened, projected and warm-hit results. The cache consistently stores/reuses the collector's wrong null, so R3-F3's corruption checks cannot repair it. Test expected source paths and invalidate old extractor results. |
| R5-F3.C5 | Regression controls and documentation | Extend the written-path matrix with all receiver forms; retain ordinary namespace/type controls. Update path docs and affected comments without adding receiver inference. |

**Required repair:** preserve the written receiver for all Java construction rows above, including type wrappers repaired under R5-F1. Checking the constructor name or the final type's namespace alone is insufficient.

### R5-F4 — High — Parameter captures turn type and default-value references into declarations

**Defect class:** parameter queries capture arbitrary identifier children instead of binding-name fields, and false-capture rejection is restricted to resolving languages, so real references become declarations before occurrence exclusion and selector matching.

In the tree-hugger library, [Python locals query](../../lib/queries/vendor/python/locals.scm:33) captures every immediate identifier inside default parameters. The [C# locals query](../../lib/queries/vendor/c_sharp/locals.scm:20) does the same inside parameters, including their named type and a bare default value. [drop_non_declarations](../../lib/src/file/tree_file/definitions.rs:171) returns immediately for languages without resolution rules, so these captures survive into records. [Resolver::occurrence](../../lib/src/analysis/occurrences.rs:489) then correctly excludes their purported declaration spans, deleting actual references.

```python
def external(): pass

def g(x=external): pass
```

The complete index invents a parameter named `external` and omits the initializer's Usage. `hug refs external --json` exits 1 with two declarations: the actual function and the fake `g.external` parameter. Parenthesizing the default, `x=(external)`, restores the reference and removes the fake record. Syntax grouping cannot legitimately add a declaration.

```csharp
class W {}
class G { void g(W x) {} }
```

The complete index invents a parameter named W alongside x and loses the type Usage. The repair ledger's final note describes `g(W obj)` displaying as `g(W: W)` as a separate rendering problem. Independent reproduction shows an earlier declaration-capture problem: the supposed parameter list contains a fake W. Treating the display symptom alone as unrelated would leave incorrect records, references and selector results.

These broad capture patterns predate this feature, but their effects directly violate this feature's declaration exclusion, whole-file occurrence and all-kind selector contracts. The repair must correct the declaration evidence shared by these consumers, not patch reference display or suppress ambiguity errors.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R5-F4.01 / Python ordinary default | `def g(x=external)` | Fake external parameter; initializer Usage absent | Only x declared; external Usage retained | CLI; actual function selector becomes erroneously ambiguous |
| R5-F4.02 / Python typed default | `def g(x: W=external)` | Fake external parameter; W read survives, default read lost | x declaration; W/external usages | CLI and copied Python fixture |
| R5-F4.03 / Python async/keyword-only/positional-only defaults | `async def g(x=external)`; `def g(*,x=external)`; `def g(x=external,/)` | Fake default-name records in all three | No default-name record; real read retained | CLI, separate callable forms |
| R5-F4.04 / Python method/initializer defaults | `class G: def method(self,x=external)...`; `def __init__(self,x:int=external)...` | Fake method/initializer external parameters | Only actual self/x parameter declarations; default reads preserved | CLI; both qualified fake parameter names inspected |
| R5-F4.05 / Python lambda default | `lambda x=external: x` | x and external captured as parameter records; external read lost | x only declared; external and body x usages | CLI and narrowed/widened/warm public API |
| R5-F4.06 / Python grouping/member/call controls | `x=(external)`, `x=obj.external`, `x=external()` | Correct Usage/qualified Usage/Call; no fake default-name parameter | Preserve all three; bare and parenthesized defaults must have the same declaration set | CLI and public API |
| R5-F4.07 / Python annotations/plain parameters | `def g(x: W)` and ordinary scalar parameters | Actual parameter captured; annotation W retained | Clean; keep annotation read and binder exclusion | Typed-default CLI comparison and field-shape invariant |
| R5-F4.08 / C# ordinary/signature type | Method `g(W x)`; interface method signature | Fake W parameter; type Usage missing | x only declared; W Usage | CLI and copied C# fixture |
| R5-F4.09 / C# constructor/primary/record parameters | `G(W x)`, `class Primary(W p)`, `record Rec(W p)` | Fake W parameters at each type token | Only x/p declared; W references retained | CLI, exact source positions and qualified record identities |
| R5-F4.10 / C# local function/lambda | `void Local(W q){}`; `(W q)=>q` | Fake W parameter; type read missing; body q read remains | q only declared; W/body q reads kept | CLI |
| R5-F4.11 / C# delegate/indexer/operator | Delegate parameter `W p`; indexer `this[W p]`; operator parameters `G x,W y` | Type-token parameter records at every site | Actual parameter names only; type usages retained | CLI; delegate declaration-name leak is separately R5-F2.24 |
| R5-F4.12 / C# default value | `const int external=1; void g(int y=external){}` | Fake external parameter; default read absent | y only declared; external Usage | CLI; bare predefined type is not itself an identifier capture |
| R5-F4.13 / C# wrapped type/default controls | `W[] x`, generic `W<int> x`, default `(external)`; implicit untyped lambda parameter | Wrapped type/default reads survive and no fake parameter from their inner identifiers; actual untyped binder excluded | Preserve; wrappers must not determine whether an identifier is declared | CLI for array/grouped/default/lambda forms; generic type node wraps its name away from the broad immediate-child capture |
| R5-F4.14 / TS/JS defaults | `g(x=external,y=external())` | Only x/y records; Usage/Call retained | Clean | CLI and existing false-capture/default-environment tests |
| R5-F4.15 / C++/Scala/PHP/Swift typed/default parameters | Named W type and bare external default, where supported | Actual parameter records only; type/default usages retained | Clean | Separate CLI rows for each language; their queries bind the declarator/name field rather than all identifier children |
| R5-F4.16 / remaining parameter readers | Rust, Go, Java, C, Lua, Perl, Bash and Zsh | Existing captured parameter/local names exclude their declaration tokens; no corresponding default-expression child capture | Preserve | Locals-query inventory and ordinary-parameter controls; C/Go/Lua CLI probes. Unreported binder cases remain enumerated in R5-F2, not excused as clean here. |

| Child ID | Decision/consumer | Verified effect and required repair |
|---|---|---|
| R5-F4.C1 | Definition collection, winning capture and record identity | Reject nonbinding type/value captures before records exist, or field the queries precisely. Actual parameter name/type/default metadata must agree. Update query provenance. |
| R5-F4.C2 | Exclusion, lookup/category/candidate selection and stopping | A type or default name must remain a reference. False same-name parameter records also pollute the all-record candidate set in limited languages; removing just the occurrence exclusion is insufficient. Keep their resolution conservative. |
| R5-F4.C3 | Counts, signatures, predicates, relation lookup and summaries | Fake parameters change declaration counts, selector matches, parameter/signature data and candidate identities. Shared declaration consumers must see the same corrected list; unsupported reference summaries stay empty. Do not introduce a display-only override or silently change diagnostic policy. |
| R5-F4.C4 | CLI name/ID selection, default/incoming/outgoing reports | Python's actual external function becomes a two-declaration ambiguity error; C# W selectors gain fake parameter matches. After repair, real symbols must be selectable, their initializer/type reads present, and genuine ambiguity still an error. Use IDs to inspect default incoming rows independently; usages stay out of callers/callees. |
| R5-F4.C5 | Filtering, projection, persistence and widening | Python/C# public-API probes preserve fake records and missing reads through kind widening, record-only semantic widening, projected results and warm hits. A functions-only result hiding fake parameters must still retain the whole-file real references. Add exact declaration/candidate/read assertions, not equality alone. |
| R5-F4.C6 | Regression scope, docs/comments and invalidation | Extend the declaration-record and occurrence matrices with both query sites and all callable consumers above. Review relevant parameter docs/comments and invalidate stale extractor results. The existing Rust/TS/JS false-capture controls must keep passing. |

**Required repair:** narrow or validate parameter captures for Python and C# independently of their resolution-support status. Remove every fake declaration in the table, restore each real type/default reference, preserve actual binding names and correct wrappers, and test selector/count/signature/candidate and occurrence consumers together. No broader language resolver is needed.

## Input Robustness Matrix

The changed reader is the persistent **JSON cache**; no manifest, lockfile, YAML or TOML reader was added. The strict stored reader requires keys, uses explicit nullable deserializers, typed vectors and duplicate-key/unknown-field rejection, and rejects trailing content. Public-schema defaults do not weaken the stored reader. Inspected `.ok()`, `unwrap_or_default()` and `filter_map` uses do not discard ill-typed JSON evidence elements.

The permanent cache tests passed, including the repaired kind/owner-span/coverage-span leaves and new path/start columns. A separate public CLI sweep restored a tool-written entry before **each of 55 field columns and 395 control/mutation cells**; all outcomes below matched the independently stated expectation. The fixture is the existing cache-test TypeScript example with helper/module_function, Box.size, caller, recursive again, file-level again, imported external, an unreported loop binding, non-ASCII comment and `other.thing()`.

**O** is module_function's owned Scoped helper call, **N** is caller's NameOnly `box.size` call, **Q** is the unresolved qualified `other.thing` call, and **R** is caller's limited region. **H** is an actual hit serving unchanged evidence; **M** is a miss that recomputes the independently expected unedited public index; **—** is an inapplicable element shape. The control must hit and expose the intended evidence before edits are meaningful. Empty edits use `[]`, `{}`, `""` or numeric zero; duplicate-key edits repeat the selected key in its containing object; trailing content appends garbage to the restored document. Nullable unqualified paths/starts remain valid hits; qualified nulls miss. Coordinate zero at these selected nonzero sites is inconsistent.

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

The passing permanent tests additionally checked the qualified nonmember Rust `other::external` path/start column and omitted-record cases. Path-only or start-only deletion recomputes; legitimate unqualified nulls hit. A coordinated deletion of both fields remains the review-4-excluded forgery limit. No cache-reader finding is carried forward. The new source mistakes above are internally consistent cached results, so a hit cannot correct them.

## Blocked Findings

None. All four required repairs can proceed inside the accepted file-local design. They require no receiver inference, additional supported language resolver, compiler integration or human approval. `human_review: false` does not waive any repair.

## Recurrence

- **R5-F1** repeats `review-1.md — Invocation classification loses calls across supported source forms`, `review-2.md — Transparent call wrappers incorrectly limit coverage`, **R3-F1**, and **R4-F2**. These are **newly discovered siblings**, not unchanged earlier cases or demonstrated repair regressions. The missed inputs are constructed-type wrappers, Swift construction/implicit-member operands and C++ braced construction; both CLI Call filters and stored/projection consumers lose the results.
- **R5-F2** repeats the declaration-exclusion part of `review-1.md — Named function expressions bind their names to outer declarations` and `review-2.md — Pattern capitalization invents or omits Rust bindings`. These are **newly discovered siblings** in unreported declaration sites. Earlier repairs inventoried Rust/ECMA binders; the remaining decision is excluding declaration tokens in the limited languages before same-name candidates, default reports and cache storage consume them.
- **R5-F3** repeats **R4-F1**'s missing-written-qualifier class. It is a **newly discovered sibling**: receiver-before-construction syntax was not among that review's namespace-qualified Java controls. The prior full-path rows pass. Collection, invocation-mode coupling, report targets and persisted/projection results are the missed consumers.
- **R5-F4** repeats the false-definition-capture portion of `review-2.md — Pattern capitalization invents or omits Rust bindings`; that repair explicitly handled Rust nonbindings and ECMA parameter defaults. Python/C# broad parameter captures are **newly discovered siblings**, with pre-existing query patterns rather than a regression introduced by the last repair. Record identity, exclusion, candidates, declaration counts/signatures, all-kind ambiguity checks and cached projections all need the shared correction. The ledger's separate C# display note did not enumerate or repair this cause.

All remaining defects are in the titled/frontmatter findings above. There is no unresolved carried required instance hidden in a recommendation or verification section. `recurrence: true` records the repeated classes and the complete inventory this repair cycle must address.

## Requirement Verification and Test Levels

| User-facing requirement | Strongest verification inspected/run | Assessment |
|---|---|---|
| Supported lexical visibility, boundaries, order, hoisting, categories, aliases and shadowing | L1 occurrence/scope/relation/summary matrices, exact public IDs and previous CLI repairs | Earlier required repairs pass; limited languages still intentionally lack lexical resolution. |
| Token spans, written qualifiers, exclusion of declaration names, owners and repeated sites | L1 all-language tables plus independent CLI/grammar probes | R5-F2/F3/F4 block extra/missing source evidence; anonymous/named boundary and repeated-site controls pass. |
| Calls, constructor targets, values versus invocations and member/argument distinction | L1 invocation/negative matrices, independent CLI/API and C++/Swift compiler checks | R5-F1 blocks missing constructed/implicit-member calls. Earlier repaired Call/Usage rows pass. |
| Direct recursion, indirect calls, unknown coverage and established summaries | L1 complete/narrowed recursion, value/eval/boundary and summary matrices | Earlier false Yes/No repairs pass. New limited-language defects must preserve Limited/Unknown and empty established summaries. |
| Whole-file evidence under selection, projection, cache widening/hits and version invalidation | L1 query/cache/upgrade tests; public API with actual Miss, WidenedKinds, record-only WidenedCapabilities and Hit outcomes | Required infrastructure/old repairs pass; new erroneous source evidence survives every inspected stage. |
| JSON reader input shapes and cross-field/source consistency | L1 shipped-CLI mutation matrices and independent 55-column/395-cell sweep | Prior R3-F3 is verified fixed; no remaining single-field qualifier-loss defect. |
| Exact/smart-case name/ID selection, local declarations, ambiguity, errors, empty and file-local reports | L1 refs tests plus independent reports/selector failures | Wiring tests pass. R5-F4 causes fake-name ambiguity; R5-F1/F2/F3 affect required rows/targets. |
| Terminal grouped rows, links, styles, wrapping and file-local boundary text | L2 `level2_declarations_in_tmux`, including the refs checks | One live real-terminal test passed; the appropriate tier exists. No new terminal-rendering failure found or full WezTerm/Kitty run claimed. |
| Keyboard input/terminal encoding | No keyboard-input requirement in this feature | L3 not required. |

Commands run:

- `just test -j 4` in `tree-hugger/`: **1,398 passed, 0 skipped**, 186.231 seconds. This includes every prior repair's permanent test and all declared L1 binaries.
- `just test-l2 level2_declarations_in_tmux` in `tree-hugger/`: **1 passed**, 5.223 seconds; 372 tests excluded by tier/filter. The shared recipe manages resources; no focus request was issued.
- Root `just check-tier-coverage tree-hugger`: **0 stranded tests**.
- Independent shipped-CLI probes: isolated Git repositories, home/config/cache directories; bare snippets and **57 primary-fixture copies** with appended probes, followed by lowercase Scala pattern controls and additional declaration/parameter sites. Baseline fixture diagnostics were semantic warnings; constructor operand trees were inspected separately. The invalid Swift explicit function-specialization experiment was excluded from findings.
- Independent strict JSON-cache probe: **55 columns, 395 control/mutation cells, 0 failures**. Every edited entry was restored from the same tool-written control; hit/miss statistics and expected public output were checked.
- Independent public API: narrow occurrence query → kind widening → semantic widening with `parsed: false` → projected hit → warm hit, on 12 representative constructor/declaration/path/parameter sources. Wrong source facts remain visible; equality is reported only as propagation evidence, never as the correctness oracle.
- `clang++ -std=c++20 -fsyntax-only` accepted the explicit C++ constructor/braced examples; `swiftc -typecheck` accepted generic type construction and an implicit-member invocation. No cross-OS evidence claim is made or used to set readiness.

The library's declared `l1` binary includes occurrence, summary, relation, query and cache modules. The CLI `symbols` binary includes refs, cache robustness and upgrades. Their ordinary test names are L1; the terminal targets declare `required-features = ["terminal-tests"]`, enabled by the live L2 recipe and CLI CI test metadata. No scratch review probe was installed or counted as permanent regression coverage. Lint/formatting was not run for these review-only document changes.

## Nonblocking Recommendations

No additional performance gate or architectural refactor is requested. Indexed lookup, one shared collection pipeline and record-only semantic widening remain suitable. The four source-evidence repairs should extend that pipeline and its existing tables rather than add report-specific corrections.
