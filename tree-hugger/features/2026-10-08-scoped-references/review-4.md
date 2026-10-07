---
$schema: feature-review.yaml
ready: false
findings:
  - title: R4-F1 — Written-path collection drops qualifiers and can bind rooted names to locals
    priority: high
  - title: R4-F2 — Invocation classification omits calls and mistakes PHP callable creation for execution
    priority: high
  - title: R3-F3 — Cache validation accepts false source facts and misordered or duplicate lists
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-06T18:25:50-07:00
spec: 2026-10-08-scoped-references/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-08-scoped-references/log.md
description: "A **feature** review of `2026-10-08-scoped-references/spec.md`"
feature: 2026-10-08-scoped-references/review-4.md
previous: 2026-10-08-scoped-references/review-3.md
next: 2026-10-08-scoped-references/review-5.md
---

# Scoped References: review 4

The feature is **not production ready**. Three findings from review 3 are verified fixed. Its cache finding is partially fixed: invented paths, body geometry, ordering and duplicate lists are checked, but deleting a required written qualifier still changes public output on a cache hit. Two new findings cover source-path collection and invocation classification. They include omitted calls, extra calls, and a compiled Rust self-call incorrectly reported as nonrecursive.

This review read the specification, plan, original implementation log, every earlier review, and the repair ledger in [log.md](./log.md). Completion flags were treated as claims. Only this review, the previous review's `next` link, and the specification's iteration count were changed. Existing implementation edits were preserved.

## Previous Finding Verification

Legacy identifiers are the review filename plus its exact finding title. A **verified fixed** outcome covers every previously enumerated instance and consumer, rather than closing every possible sibling in that defect class. New siblings have their own IDs below. The repair ledger's `R3-F3.S09r` is retained.

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
|---|---|---|---|---|
| `review-1.md — Lexical lookup crosses boundaries that do not establish a binding` | Rust module/nested function/constant/static boundaries; closure capture; conditional chains, else and following statements; loop controls; TS/JS `with`, direct eval and parameter defaults; occurrence IDs, summaries, recursion, declared-base lookup and CLI rows | **verified fixed** | Passing `LEXICAL_BOUNDARY_MATRIX`, boundary-summary and CLI tests; shared scope lookup enforces language boundaries and visibility endpoints. Parameter-environment tests and relation rows also pass. | New rooted-name collection sibling is R4-F1.01; no unchanged earlier reproduction remains failing. |
| `review-1.md — Named function expressions bind their names to outer declarations` | Ordinary/generator expressions in TS/JS, declaration-token exclusions, expression-local calls/defaults and outer calls; unnamed/class/arrow controls; placement, summaries, recursion, relation lookup and callers | **verified fixed** | Passing `EXPRESSION_NAME_MATRIX`, expression summary/recursion/relation rows and `refs::named_function_expressions_never_call_the_outer_declaration`; expression-local names stay inside their expressions. | None for the required instances. |
| `review-1.md — Invocation classification loses calls across supported source forms` | All 16 languages' direct/wrapped calls; TS assertions/non-null; Perl/shell calls; nested member wrappers and generic calls; receiver/argument/sequence/conditional/dereference negatives; call reports, recursion, terminal-call detection and CommonJS evidence | **verified fixed** | Invocation and negative matrices, wrapped-binding tests and CLI tests pass; `parenthesized_terminal_callees_are_terminal` and `commonjs_evidence_counts_anywhere_in_the_file` passed in this run. | R4-F2 enumerates new rooted, conditional and PHP invocation-mode siblings. Preserve the accepted PHP `(f)()` constant-value exception and Go indexing/generic limitation. |
| `review-1.md — Cache validation accepts inconsistent reference evidence` | Wrong retained/malformed/lost target IDs, ownerless targets, owner/candidate identity, duplicate/omitted callable regions, structural matrix, summaries/dependencies/semantics, legitimate nulls and narrowed entries | **verified fixed** | Public cache mutation and retained-evidence tests passed. Required keys, two-way summary edges, semantic recomputation and required callable regions are present. | R3-F3 remains partially fixed for lost written paths. Accepted anonymous-evidence deletion and unrelated containment/base-target limits are unchanged. |
| `review-2.md — Parameter defaults incorrectly see body-local var bindings` | Seven callable forms in both TS/JS; ordinary/block/destructured/loop var; parameter visibility, redeclaration and hoisting; body eval; let/const/body-function controls; relation lookup, occurrences, both report directions and documentation | **verified fixed** | Passing `PARAMETER_DEFAULT_FORMS × BODY_DECLARATIONS`, `PARAMETER_ENVIRONMENT_MATRIX`, relation rows and `refs::parameter_defaults_call_the_outer_declaration_not_a_body_var`; `var_body` separates the environments. | None for the required instances. |
| `review-2.md — Pattern capitalization invents or omits Rust bindings` | Bare/modified capitalized bindings in conditions, loops and arms; lowercase constants in patterns/body/guards; paths, tuples, shorthands and irrefutable controls; walker, captured-record fallback, declaration exclusion, summaries, recursion, callers and docs | **verified fixed** | Passing `PATTERN_IDENTITY_MATRIX`, `PATTERN_RECORD_MATRIX`, summary/recursion rows and both CLI pattern tests. Shared pattern placement runs before record extraction; constant reads cannot reappear as declarations through fallback. | None for the required instances; documented import/glob and unreported-binding limits remain in force. |
| `review-2.md — Transparent call wrappers incorrectly limit coverage` | Rust parentheses/turbofish/type paths; all enumerated TS wrappers; JS parentheses; constructors; direct/self/member/value/sequence/conditional/dereference controls; coverage, direct eval, recursion and shared relation scopes | **verified fixed** | Passing `CALLEE_COVERAGE_MATRIX`, complete/narrowed `RECURSION_MATRIX`, shared `invoked_operand` and eval matrices. Calls to another known declaration give `No`; self-calls give `Yes`. | New root-path classification sibling is R4-F2.01–02. Earlier wrapper reproductions still pass. |
| `review-2.md — Cache validation accepts nonexistent evidence identities and missing callable regions` | All 16 languages' phantom owners; applicable phantom candidates/owned and ownerless targets; Python/Lua empty regions; retained summary identities; callable-covering/narrowed entries; null imports/unreported bindings and bodiless controls | **verified fixed** | Passing `every_kind_evidence_identities_name_retained_declarations_in_every_language` and retained-evidence tests. IDs absent from stored records require an omitted compatible kind; a retained callable owns a region iff its `body_span` exists. | R3-F3 concerns lost source paths, rather than a regression of phantom identities or missing regions. |
| R3-F1 — TypeScript instantiation expressions hide calls and constructor invocations | `.01–.09`: six wrapper forms, member and constructor operands, self recursion; `.10–.18`: direct/noncall and all-language controls; `.C1–.C7`: collection, classification, binding/candidates, summaries, coverage/recursion, filtering/projection/widening and both CLI directions; ledger `.R`: revision invalidation | **verified fixed** | Shared TS `instantiation_expression` operand arm; passing invocation/coverage/recursion matrices and exact member-candidate assertion; `instantiation_expression_calls_survive_narrow_widened_and_warm_entries` asserts exact calls, `WidenedKinds`, `Hit` and expected recursion; CLI instantiation test and revision-41 rejection passed. All required source forms were checked in the tables, rather than inferred from summaries. | New generic **written-path** siblings in C++/C# are R4-F1. They do not repeat the repaired TS call classifier. |
| R3-F2 — Rust associated value calls falsely prove absence of recursion | `.01–.10`: inherent/trait/enum/generic, wrapped and unrelated-initializer value calls; `.11–.16`: top-level values, known methods/constructors, usages, ECMA and limited-language controls; `.C1–.C5`: lookup/candidates, summaries, narrow recursion, semantic widening and callees coverage | **verified fixed** | Passing `VALUE_CALL_MATRIX`, complete/narrowed recursion rows and `value_calls_reference_the_value_not_its_initializer`; both unique-member and uncertain-candidate paths check value kinds. `stored_value_calls_widen_to_unknown_recursion` asserts record-only widening and `Unknown`; CLI value-call coverage test passed. | None for these required instances. Initializers are not followed. |
| R3-F3 — Cache validation accepts false source facts and misordered or duplicate lists | `.L01–.L16.{region,path,candidates}`; `.S01–.S03.{rs,ts,js}.{duplicate,reverse}`; `.S04–.S08` controls; `.S09–.S10` null/empty paths; `.C1–.C4` full/projection, text/JSON, semantic widening and narrowed/null controls; ledger `.M` leaf matrix and `.S09r` residual | **partially fixed** | Both all-language/source-fact and all-resolving-language/list mutation tests passed. Retained body containment, source suffix validation, retained candidate order and first-occurrence summary order are enforced. Full retained-member `.S09`, both `.S10` cells, the text/JSON invented-path regression and existing structural/reference/span matrix tests pass. The permanent matrix still lacks nine leaf columns (R3-F3.M below). Independently changing a qualified nonmember target's path to null still produces a hit and loses the path, as detailed below. | Retain R3-F3.S09r, the incomplete `.M` regression matrix, and consumers `.C1`, `.C2`, `.C4`: a single null edit can erase a nonmember, unresolved or omitted-member qualifier. No accepted decision waived this required repair. |
| R3-F4 — Optional and tagged eval calls incorrectly hide lexical declarations | `.01–.04` optional/tagged and wrapped forms in TS/JS; `.05–.07` direct/sequence/member/alias controls; `.C1–.C5` scope barrier, declared bases, summaries/candidates, coverage/recursion, filtering/cache/callers | **verified fixed** | Shared `ecma::is_direct_eval` checks argument-list and optional-call mode; passing `EVAL_CALL_FORMS`, exact boundary/coverage/recursion/relation assertions, `only_a_direct_eval_cuts_summary_edges`, cache narrow/widen/warm test and CLI callers test. Genuine direct eval remains conservative. | None for the required instances. |

Earlier reviews contained no blocked findings. The original implementation log's “Rulings (accepted for implementation)” established Rust/TS/JS resolution, limited evidence in the other languages, no receiver inference, and file-local analysis **before** these repairs. Those choices remain accepted. The accepted inability to detect deletion of an anonymous region without surviving derived evidence also preceded this repair. The latest ledger records a proposed additional cache deferral, not an accepted change to the required lost-qualifier repair; writing that limitation into current documentation did not remove the obligation. No prerequisite changed or blocker was removed during this cycle.

## Unblocked Findings

### R4-F1 — High — Written-path collection drops qualifiers and can bind rooted names to locals

**Defect class:** written-path collection examines only selected immediate-parent shapes, losing a root, outer namespace, generic wrapper, conditional receiver or package qualifier before resolution and reporting.

The tree-hugger library's [qualified_parent](../../lib/src/analysis/occurrences.rs:1057), which recognizes a reference's qualifier, requires a named qualifier field on the immediate parent. [written_path](../../lib/src/analysis/occurrences.rs:1080), which supplies the source path in every occurrence, then copies only that parent's range. This misses root-only `::`, names nested inside generic forms, C# alias/conditional member nodes, PHP nullsafe member nodes, and Perl's package and receiver fields. A right-nested C++ path also loses its outer namespace.

This matters beyond presentation. In Rust, losing the root allows ordinary lexical lookup to select a local binding that the absolute path never names:

```rust
fn f() { ::f(); }
fn g() { let f = || {}; ::f(); }
```

`rustc --crate-type lib --edition 2015 --emit=metadata` accepts this source and warns that the first function recurses. The CLI reports both `f` tokens as unqualified `Usage`: the first bound to the root function, the second bound to the local closure variable, and `f.is_recursive: No`. Edition 2015 is a valid Rust source form, and this API has no edition parameter. The repair need not add edition inference or promise resolution of arbitrary absolute paths. It must preserve `::f`, classify the invocation, and retain uncertainty where the edition or root binding is unproved. It must never bind that absolute name to the local `let` or claim `No` for the demonstrated self-call. Invocation classification is also enumerated in R4-F2.

The tests used the shipped CLI on scratch sources and copies of the primary language fixtures with appended probe declarations. Generic/non-generic controls differ only at the operand's type arguments; conditional controls differ at the access operator. Every defective source path is independently known from the source bytes ending at the name token. Type arguments **after** the name token are outside that path; type arguments in a qualifier remain inside it.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R4-F1.01 / Rust root-only path and binding | `::f()`; `let f=…; ::f()`; also root `::m` inside `::m::f()` | No path; ordinary lookup can establish the shadowing local ID | Path `::f` / `::m`; no lexical-local target for an absolute name; conservatively uncertain when root resolution is unproved | Full-index CLI; compiled 2015 source above; inspected `resolve_supported` fallback |
| R4-F1.02 / C++ global root | `::f()`, `(::f)()`, `((::f))()` | No written path | `::f` at every depth | Full-index CLI |
| R4-F1.03 / C++ nested namespaces | `A::B::f()` | Final path `B::f`; captured `B` has no path | Full path `A::B::f`, with `A::B` on the captured prefix | Full-index CLI; simple `ns::f()` control retains its full path |
| R4-F1.04 / C++ generic namespace function | `r4::f<int>()`, `(r4::f<int>)()`; `auto x=r4::f<int>` | `f` has no path, in calls and value usage | `r4::f`; preserve Call/Usage distinction | Copied `cpp.cpp`, full index and cold/warm callees |
| R4-F1.05 / C++ generic member | `obj.m<int>()`, `(obj.m<int>)()` | `m` is a Call with no receiver path | `obj.m` | Same fixture/CLI; ordinary `obj.m()` control |
| R4-F1.06 / C++ dependent member | `obj.template m<int>()`, `(obj.template m<int>)()` | Call without receiver | `obj.template m` | Same fixture/CLI |
| R4-F1.07 / C++ pointer member | `p->template m<int>()` | Call without receiver | `p->template m` | Same fixture/CLI; ordinary C/C++ pointer-member control |
| R4-F1.08 / C++ generic type and constructor | `r4::C<int> v`; `r4::C<int>()` | Captured `C` has no path in either Usage or Call | `r4::C` in both, with the existing correct kind | Same fixture/CLI |
| R4-F1.09 / C++ generic nested scope | `N::C<int>::member()`; `A::B::C<int>::m()` | `C<int>::member` / `C<int>::m`, dropping outer scopes; intermediate generic type also loses its path | `N::C<int>::member` / `A::B::C<int>::m`; preserve each captured prefix too | Full-index CLI |
| R4-F1.10 / C# generic method | `obj.M<int>()`, `(obj.M<int>)()` | Call with no path | `obj.M` | Copied `c_sharp.cs`, full index and cold/warm callees |
| R4-F1.11 / C# generic qualified type/constructor | `N.C<int> c=new N.C<int>()`; full `global::Example.Symbols.R4W<int>` variant | Captured generic type/constructor name has no path | `N.C` / `global::Example.Symbols.R4W`, for both usage and constructor call | Same fixture/CLI |
| R4-F1.12 / C# alias-qualified name | `X::C`; captured `N` in `global::N.C` | `C` / `N` has no path | `X::C` / `global::N`; do not invent a qualifier on the alias token itself | Full-index CLI; subsequent ordinary dotted `C` path is a clean control |
| R4-F1.13 / C# conditional member | `obj?.F()`, `obj?.M<int>()`, `obj?.A()`, `obj?.A?.Invoke()`; noncall `obj?.A` | Conditional member names have no path; calls also misclassified under R4-F2 | Keep `obj?.F`, `obj?.M`, `obj?.A`, `obj?.A?.Invoke`; retain intermediate prefix usage | Copied fixture and independent full-index/CLI probes |
| R4-F1.14 / PHP nullsafe member | `$o?->m()`, `($o)?->m()`; `$o?->p`, `($o)?->p` | No receiver path on either calls or property usages | `$o?->m`, `($o)?->m`, `$o?->p`, `($o)?->p` | Copied `php.php` plus property probe; ordinary `$o->m` / `$o->p` controls retain paths |
| R4-F1.15 / Perl package call/reference | `R4::r4f()`, `&R4::r4f()`, `\&R4::r4f` | No package path; correct Call/Usage distinction survives | Written package path `R4::r4f`; retain each occurrence's current kind | Copied `perl.pl`, full index and cold/warm callees |
| R4-F1.16 / Perl receiver call | `R4->r4f()`, `$obj->r4f()` | Calls without a receiver path | `R4->r4f`, `$obj->r4f` | Same fixture/CLI |
| R4-F1.17 / Rust qualified/generic control | `m::f::<u8>()`, `(m::f::<u8>)()`, `Self::h::<u8>()` | Full written paths, established Self member where supported | Clean; preserve generic call classification and conservative arbitrary-module resolution | Full-index CLI and passing invocation matrix |
| R4-F1.18 / TS control | `obj.m<number>()`, `(obj.m<number>)()`, `obj?.m<number>()`, `A.B.f()` | Full paths; calls remain uncertain on receivers | Clean; preserve paths and candidates without inventing IDs | Full-index CLI |
| R4-F1.19 / JS control | `obj.m()`, `(obj.m)()`, `obj?.m()`, `A.B.f()` | Full paths | Clean | Full-index CLI |
| R4-F1.20 / Java control | `obj.<String>m()`, `obj.m()` | `obj.<String>m` / `obj.m`, both calls | Clean | Full-index CLI |
| R4-F1.21 / Go control | `pkg.F()`, `pkg.F[int]()` | Ordinary qualified call retains `pkg.F`; generic-index occurrence remains a usage under the accepted grammar limitation | Preserve ordinary path and accepted classification; no inference from indexing syntax | Full-index CLI |
| R4-F1.22 / C control | `obj.f()`, `p->f()` | Full receiver paths | Clean | Full-index CLI |
| R4-F1.23 / Python control | `obj.f()`, `A.B.f()` and value access | Full paths, correct invocation/usage distinction | Clean | Full-index CLI |
| R4-F1.24 / Swift control | `W.f(1)`, `obj?.f()`, `obj!.f()`, `A.B.f()` | Full paths | Clean | Full-index CLI |
| R4-F1.25 / Scala control | `W.f[Int]()`, `(W.f[Int])()`, `W.f` | Full paths, correct Call/Usage | Clean | Full-index CLI |
| R4-F1.26 / Lua control | `W.f()`, `W:f()`, `A.B.f()` | Full paths | Clean | Full-index CLI |
| R4-F1.27 / Bash and Zsh controls | Bare command `cb`; path command `other/external` | Command target carries the command spelling itself; no separate language namespace qualifier | Clean: the whole command-name token, rather than a fabricated suffix/namespace, is the written name | Shipped CLI and shell invocation matrix; no qualifier is discarded from that token |
| R4-F1.28 / non-generic C++/C#/PHP controls | `ns::f()`, `obj.A()`, `System.Console.WriteLine()`, `W::m()`, `\Ns\f()` | Paths retained | Preserve these positive controls; repair the missing shapes without changing literal source spelling | Full-index CLI |

The decision and consumer sweep:

| Child ID | Decision/consumer | Required repair or verified invariant |
|---|---|---|
| R4-F1.C1 | Collection, exclusions and source spans | Build the full written qualifier through the actual grammar wrappers and nested path nodes. Keep the occurrence's name-token span; do not consume argument/type-argument references or duplicate calls. Existing captured names are present in every defective row. |
| R4-F1.C2 | Lexical lookup, category selection and stopping | Rooted Rust names must reach qualified-path handling before ordinary lookup. Preserve the written root and never establish a shadowing local. Keep unsupported paths uncertain rather than expanding receiver or edition inference. |
| R4-F1.C3 | Candidates, summaries and incoming references | C++/C#/PHP/Perl keep their conservative candidates and no established edges. Rust must lose the false local summary/incoming row. A preserved qualifier must not be mistaken for proof of one target. |
| R4-F1.C4 | Invocation, coverage and recursion | Root-only qualifiers also block the shared invocation climb: R4-F2.01–02. Assert `Call`, appropriate uncertainty and no false `No`; a correct path string alone does not verify these consumers. |
| R4-F1.C5 | Filtering, projection, storage and widening | Analysis precedes filtering. `coverage::project` retains whole-file occurrences, and the cache serves their paths unchanged. Cold/warm `refs --callees` reproduced the same missing paths with an actual warm hit. Add independent expected-path assertions through kind/name narrowing, projection, widening and hits. |
| R4-F1.C6 | CLI text/JSON, callers/callees and default references | Reports retain target paths while prefixing owner/candidate declaration names. Current callees display merely `Call m` / `Call f`. Assert the full written target in both formats, candidate groups in incoming reports, and no wrong established local row. |
| R4-F1.C7 | Other readers and documentation | Dead-code terminal-call detection and ECMA CommonJS evidence have separate readers; their existing corrected tests pass and must stay unchanged. Update invocation/path documentation, affected comments and the extractor revision alongside the repair. |

**Required repair:** implement the complete list above with Level-1 expected-path, identity and report assertions. Extend the shared path reader deliberately by grammar shape; include roots and whole nested qualifiers, rather than adding generic wrappers while leaving conditional/package shapes behind. No broader language resolution is requested.

### R4-F2 — High — Invocation classification omits calls and mistakes PHP callable creation for execution

**Defect class:** invocation classification equates selected parent shapes with calls, missing names behind root, conditional-member, PHP variable-name and Perl variable-callee nodes while treating PHP callable creation as execution.

The tree-hugger library's [is_call_target](../../lib/src/analysis/occurrences.rs:1091), which decides which sites `hug refs` lists as calls, cannot cross a root-only qualifier or C# conditional member binding. PHP's `variable_name` also blocks the climb. Perl's special [is_perl_invocation](../../lib/src/analysis/occurrences.rs:1319) recognizes only a `function_name` field; code-reference invocations have either an `object_return_value` scalar with no method name, or a `call_expression_with_variable`. Their already captured scalar token is also left as Usage. Conversely, it treats every named PHP call-expression shape as a call without checking the argument placeholder that means “create a callable.”

```csharp
class W { public void F() {} }
class G { void g(W obj) { obj?.F(); } }
```

The name `F` is `Usage`, and `hug refs g --callees --json a.cs` lists no call. A conditional member invocation is still a syntactic invocation even though null may prevent execution. This follows the same rule as the correctly classified TS/JS optional calls. [Microsoft's member-access reference](https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/operators/member-access-operators) documents conditional invocation.

```php
<?php
function f() {}
function g($cb) { f(...); $cb(); }
```

The CLI lists `f(...)` as a call and omits `$cb()`. PHP's [callable-creation syntax](https://www.php.net/manual/en/functions.first_class_callable_syntax.php) creates a closure without invoking `f`; its [variable-functions reference](https://www.php.net/manual/en/functions.variable-functions.php) defines `$cb()` as a call. The public occurrences must therefore report `f` as `Usage` and `cb` as `Call`, with conservative confidence preserved. No tracking of the callable's eventual invocation or initializer is needed.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R4-F2.01 / Rust absolute name | `::f()`, `(::f)()`, including the compiled self-call in R4-F1 | `Usage`; first function reports complete coverage and `No` | `Call`; never false `No` for the compiled self-call; preserve root and uncertainty under R4-F1 | Public full index and rustc positive control |
| R4-F2.02 / C++ absolute name | `::f()`, `(::f)()`, `((::f))()` | `Usage`; omitted from callees | `Call`, conservative confidence; written path `::f` | Full-index and reference CLI probes |
| R4-F2.03 / C# conditional method | `obj?.F()` | `Usage`; missing callees and callers suggestion | `Call` with the same member candidates and a conditional receiver path | Copied primary fixture and independent snippet; cold/warm CLI |
| R4-F2.04 / C# conditional generic method | `obj?.M<int>()` | `Usage` | `Call`, generic operand and conditional member traversed together | Same |
| R4-F2.05 / C# conditional delegate member | `obj?.A()` | `A` is `Usage` | `A` is the syntactic `Call` target; no known called implementation | Same; ordinary delegate-call control below |
| R4-F2.06 / C# chained conditional invocation | `obj?.A?.Invoke()`; `cb?.Invoke()` | Final `Invoke` is `Usage` | Final `Invoke` is `Call`; preceding receiver/member names remain usages | Same; `obj?.A.Invoke()` already correctly calls `Invoke` |
| R4-F2.07 / PHP plain callable creation | `f(...)`, `(f(...))` | `Call` | `Usage`; surrounding parentheses do not execute the created closure | Copied PHP fixture, full index, cold/warm callees and text |
| R4-F2.08 / PHP qualified callable creation | `\Ns\f(...)` | `Call` | Qualified `Usage` | Independent full-index/reference CLI |
| R4-F2.09 / PHP static callable creation | `W::s(...)` | `Call` | `Usage`, retaining `W::s` | Copied fixture and full-index/reference CLI |
| R4-F2.10 / PHP instance callable creation | `$o->m(...)` | `Call` | `Usage`, retaining `$o->m`; receiver remains a usage | Same |
| R4-F2.11 / PHP callable creation used as an argument | `consume(f(...))` | Both names are calls | Only `consume` is a Call; `f` is a Usage | Same; proves the exclusion must apply inside another invocation |
| R4-F2.12 / PHP variable-call target | `$cb()`, `($cb)()`, `(($cb))()` | All `cb` occurrences are usages | All are syntactic calls of the variable's value, without a fabricated function ID | Copied fixture and independent value-callee sweep |
| R4-F2.13 / PHP dynamic constructor | `new $C()` | `C` is `Usage` | `Call` for the constructed operand; implementation remains unknown | Same |
| R4-F2.14 / PHP variable callable-creation control | `$cb(...)`, `($cb)(...)` | `Usage` | Clean; preserve this when fixing ordinary variable calls | Same |
| R4-F2.15 / C# direct/generic/delegate controls | `obj.F()`, `obj!.F()`, `obj.M<int>()`, `(obj.M<int>)()`, `cb()`, `(cb)()`, `obj?.A.Invoke()` | Correct calls | Preserve; receiver/argument names stay usages | Full-index CLI and cold/warm reports |
| R4-F2.16 / PHP ordinary named/member/constructor controls | `f()`, `W::s()`, `$o->m()`, `$o?->m()`, `new F()` | Calls | Preserve calls; nullsafe qualifier loss is separately R4-F1.14 | Full-index CLI and passing invocation matrix |
| R4-F2.17 / PHP constant/dereference-like value controls | `(f)()` names a constant; `($o->m)()` reads a property's value | Named function/method token is Usage under the accepted distinction | Preserve the accepted constant-versus-function and property-versus-method distinction; do not mark every parenthesized PHP name as a function call | Full-index CLI and existing negative matrix; this repair specifically needs variable-valued callee syntax |
| R4-F2.18 / Rust/TS/JS/Go/Python/C/C++/Swift/Scala/Lua direct-value controls | A parameter or function-pointer variable `cb()` and `(cb)()` in each language | Call; supported-language value calls remain indirect | Clean; retain expected Call without inferring an initializer; Go indexing limitation remains accepted | All-language value-callee CLI sweep; passing binding/coverage/recursion matrices |
| R4-F2.19 / TS/JS/Swift optional-member controls | `obj?.m()`, `obj.m?.()`, `obj?.m?.()` (TS/JS); `obj?.f()` (Swift) | Final member is Call | Clean; receiver references remain Usage | Shipped CLI |
| R4-F2.20 / Java control | `cb.run()` | `run` Call, `cb` Usage | Clean; Java invokes the method name rather than the receiver | Shipped CLI |
| R4-F2.21 / Perl controls | `f()`, `Pkg::f()`, `$obj->f()`; `\&f`, `defined &f`, `exists &f` | Ordinary invocations are Call; name-only forms are Usage | Clean classification; written path loss is R4-F1.15–16 | Shipped CLI and passing Perl invocation/negative rows |
| R4-F2.22 / Bash/Zsh controls | Bare and unquoted-variable command names; receiver/argument variable references | Command-name occurrences are Call; interpolation/argument references are Usage | Preserve the command-name target rather than promoting every variable inside a computed command string | Shipped CLI and shell invocation/negative rows |
| R4-F2.23 / Perl variable/code-reference invocation | `$cb->()`, `$cb->(1)`, `&$cb()` | Captured `$cb` is Usage; omitted from callees and incoming call suggestions | `$cb` is Call, retaining conservative confidence; do not resolve its initializer | Public full-index and callees CLI; pinned grammar AST shows `method_invocation object_return_value: scalar_variable` without `function_name`, or `call_expression_with_variable` |
| R4-F2.24 / Perl variable-name negative controls | `$cb;`, `consume($cb)`, receiver `$cb` in `$cb->m()`, `\&$cb`, `defined &$cb`, `exists &$cb` | Scalar occurrences are Usage; method name `m` is Call | Preserve these distinctions when adding variable callees; a named-method receiver or a code reference taken as a value is not itself called | Same public CLI sweep and pinned grammar inspection |


Perl code-reference invocation is documented by [the Perl reference manual](https://perldoc.perl.org/perlref); the pinned grammar exposes the scalar as the callable operand here. This differs from a scalar used as the receiver of a named method or an explicit dereference operand inside a separately computed callee. The PHP sweep also ran `f(...$args)` and `$cb(...$args)`: the named function is correctly a Call, while the variable target still has the R4-F2.12 defect. It distinguishes the literal placeholder `...` from real argument unpacking. The latter invokes the target and must remain a Call. This distinction applies to all named, qualified, static, instance and variable-valued PHP callees; add paired placeholder/unpacked-argument rows for these readers. Placeholder creation with a nullsafe operator or `new` is invalid PHP and is not a promised resolved form. C#/C++/Rust wrapper traversal must work at arbitrary depth; it must not promote names inside sequences, conditional values or explicit dereference operands, whose existing negative rows pass.

| Child ID | Decision/consumer | Required repair or verified invariant |
|---|---|---|
| R4-F2.C1 | Capture, wrapper/path climb and call-mode decision | All missing names already exist in the occurrence list. Read the actual callable operand through the relevant grammar nodes, including Perl variable-call nodes and code-reference method invocations that have no method name and distinguish PHP placeholder creation from an argument list that executes it. Repair R4-F1's root traversal consistently. |
| R4-F2.C2 | Binding, candidate selection and summaries | Keep supported lexical bindings and conservative unsupported-language candidates. Conditional dispatch and PHP callable creation do not justify additional IDs. Limited-language summaries remain empty; no candidate becomes an established edge. Rooted Rust names must not summarize a shadowing local. |
| R4-F2.C3 | Coverage and recursion | The compiled Rust self-call currently receives false `No`; verify the corrected conservative result independently. C#/PHP keep `unsupported_language` and `Unknown`, which does not excuse a wrong syntactic kind. Re-run supported-language coverage/recursion controls, including value calls and direct eval. |
| R4-F2.C4 | Filtering, projection, cache widening and warm hits | Exact defective rows persist in cold and warm reports; actual warm hits were checked. Add expected Call/Usage assertions in narrow/projected/widened results and old-revision rejection. Equality with a fresh wrong result is insufficient. |
| R4-F2.C5 | CLI selection, incoming/callees, text and JSON | `refs::build_report` filters both directions on Call. Current PHP reports contain extra callable-creation rows; C#/PHP/root-only reports lose required rows. Test both missing and extra results, including incoming NameOnly suggestions and the default mixed-reference report. |
| R4-F2.C6 | Other invocation consumers | The terminal-call and CommonJS consumers do not consume this classifier and their previous regression tests passed. Keep their behavior and the accepted PHP/Go exceptions. Update the classifier's comments, invocation documentation and extractor revision with the repair. |

**Required repair:** add all defective shapes and paired controls to Level-1 invocation matrices and public report tests. Restore conditional, rooted and variable-valued calls (PHP and Perl), exclude callable creation, and preserve conservative resolution. Do not infer a function-pointer initializer or expand the supported-language table.

### R3-F3 — Medium — Cache validation accepts false source facts and misordered or duplicate lists

**Defect class:** a nullable stored path is validated from retained declaration category rather than retaining enough source evidence to distinguish an unqualified occurrence from a qualified occurrence whose path was deleted.

**Remaining child: R3-F3.S09r.** The tree-hugger library's [validate_evidence](../../lib/src/cache/stored_evidence.rs:236), which decides whether stored reference evidence may be served, now rejects a present path that is empty or not a source suffix. Its null branch rejects a resolving-language occurrence only when its target or candidate is a **retained member**. Qualified items, unresolved paths, omitted members and every language without binding rules can still lose their path through one field edit.

The independently reproduced smallest case is:

```rust
fn g() { other::external(); }
```

Let the shipped CLI write a complete entry and confirm an unedited warm hit. Change only the final occurrence's `target.qualified_name` from `other::external` to null. `hug symbols --json-full --verbose a.rs` reports **one hit, zero parses**, and exposes `external` with no path. `hug refs g --callees` likewise loses the written qualifier. The source and cache key are unchanged. A miss followed by recomputation must preserve `other::external`.

The same mutation was swept over copies of all primary fixtures, appending a qualified probe when necessary and restoring the original tool-written entry before each edit. Every applicable defective row had an unedited positive hit, then an edited hit with the original path absent from public output. Bash/Zsh store a path command's entire spelling as its name, and the chosen Perl forms currently emit no path at collection; those are distinguished from a successful cache validation test.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R3-F3.S09r.rs | `crate::outer_module` in primary fixture; `other::external` in minimal source | Null path served | Reject single-field path loss and recompute original path | Independent CLI mutations, zero-parse hits; full shape row below |
| R3-F3.S09r.ts | Appended `other.external()` | Null path served | Recompute written path | Copied primary fixture/CLI mutation |
| R3-F3.S09r.js | Same JS probe | Null path served | Same | Same |
| R3-F3.S09r.go | `fmt.Sprint` | Null path served | Same | Primary fixture/CLI mutation |
| R3-F3.S09r.py | `self.assigned_through_self` | Null path served | Same | Same |
| R3-F3.S09r.java | `com.example` | Null path served | Same | Same |
| R3-F3.S09r.cs | `Example.Symbols` | Null path served | Same | Same |
| R3-F3.S09r.c | Appended `o.external()` | Null path served | Same | Copied primary fixture/CLI mutation |
| R3-F3.S09r.cpp | `gnu::deprecated` | Null path served | Same | Primary fixture/CLI mutation |
| R3-F3.S09r.swift | Appended `Other.external()` | Null path served | Same | Copied primary fixture/CLI mutation |
| R3-F3.S09r.scala | Same qualified probe | Null path served | Same | Same |
| R3-F3.S09r.php | `$this->privateField` | Null path served | Same | Primary fixture/CLI mutation |
| R3-F3.S09r.lua | `other.external()` inserted before fixture's final return | Null path served | Same | Copied primary fixture/CLI mutation |
| R3-F3.S09r.perl | Package/receiver probes | Collector already emits null, so there is no positive non-null control to mutate | Repair R4-F1.15–16 first, then exercise this cache cell; do not call an absent control a cache pass | Independent collection sweep; shared validator has no rules-language member protection here |
| R3-F3.S09r.sh / .zsh | `other/external` command | Entire command spelling remains in `target.name`, rather than a separate path field | Clean for this shape: null does not erase part of the written name | Public shell probe and collector invariant |
| R3-F3.S09r.omitted | Methods-omitted resolving-language entry with a receiver occurrence | Null path remains servable under the retained-record guard | Retain enough stored source evidence to reject path loss even when the member record is omitted | Passing `stored_evidence_geometry_paths_and_order_are_checked` explicitly expects this edited entry to be served; shared null branch inspected |
| R3-F3.S09 / retained-member control | TS/JS receiver and Rust Self-member target/candidate retained | Null path rejected | Clean, preserve rejection | Public matrix and list-edit tests passed |
| R3-F3.S10 / empty-path controls | Empty string on qualified and unqualified targets | Rejected | Clean; present empty path is never a valid qualifier | Public matrix/list-edit tests and independent shape sweep |

The already repaired child inventory is complete: `.L01–.L16.region`, `.path` and `.candidates` passed the 16-language mutation test; `.S01–.S03` duplicate and reverse edits passed for each of Rust/TS/JS; `.S04–.S07` duplicate/order controls passed; `.S08` fabricated unsupported-language summaries were refused in all 13 languages. Retained body geometry is checked before semantic widening, and existing null import/unreported-binding controls still pass. Those repairs are not reopened.

Remaining consumers retain their IDs: **R3-F3.C1** (complete/narrow/projected indexes expose lost paths), **R3-F3.C2** (text/JSON reference reports display lost paths), and **R3-F3.C4** (omitted-record controls must stop accepting path loss while valid unqualified nulls still hit). **R3-F3.C3** body-geometry/semantic reuse is verified fixed. Selection and summary IDs do not change in a path-only cache mutation; this defect must not be described as a new binding computation on a hit.

**Required repair:** retain a cache-only written-qualifier span or equivalent redundant source fact and validate its agreement with the nullable path, including omitted records; alternatively verify ambiguous cases against syntax. This closes the required **single-field** corruption case without pretending to detect every coordinated forgery. A second stored field could also be forged, just as an attacker could edit an occurrence and its summaries together, but that is not a reason to omit cross-field consistency checks. Avoid an approximate scan of punctuation: the repair log correctly identifies Rust macro-token, leading-root and JS spread controls that such a scan can reject incorrectly. Preserve those positive controls and test the full list above through public hits/misses and both report formats.

**R3-F3.M — remaining regression coverage.** The ledger calls its matrix work fixed, but `evidence_columns`, `reference_leaf_columns` and `span_leaf_columns` in the CLI's [cache robustness tests](../../cli/tests/symbols/cache_robustness.rs:1319) omit `occurrences[*].kind` and the four point coordinates in each owner and coverage span. The independent 51-column sweep above confirms that the current shared reader rejects invalid shapes at these sites; it does not install permanent regression protection. Retain child IDs `R3-F3.M.kind`, `R3-F3.M.owner_span.start.line`, `.owner_span.start.column`, `.owner_span.end.line`, `.owner_span.end.column`, `.coverage.span.start.line`, `.coverage.span.start.column`, `.coverage.span.end.line` and `.coverage.span.end.column`. Add every missing field to the tool-written public-result matrix, with control, absence, null, wrong type, empty, duplicate-key and trailing-content outcomes. None has an element-type shape. Their observed and expected row is H for the unedited control and M for every applicable edit. This completes the earlier required per-field matrix rather than adding another parser defect.

The ledger's request for a reviewer/author decision does not make this blocked. The previous review expressly required retaining enough evidence to detect qualifier loss, and a stored-only consistency repair remains within that requirement. No human approval or broader resolution design is needed.

## Input Robustness Matrix

The changed format reader is the persistent **JSON cache**. This feature adds no manifest, lockfile, YAML or TOML reader. The public cache tests passed the structural/reference/span matrix, including control hits and miss-and-recompute outcomes. Public serde defaults on occurrences are kept outside the strict stored reader; required stored keys, `nullable`, `deny_unknown_fields` and typed vectors reject absent/wrong-type/duplicate/trailing data. The searched `.ok()`, `unwrap_or_default()` and `filter_map` sites act on typed facts, rather than silently discarding malformed JSON elements.

The following shape row independently restores a tool-written entry from `fn g(){other::external();}` for each edit. The whole column is accounted for, rather than treating the null failure alone as the class sweep. Element-type shapes do not apply to this scalar field.

| Shape | Qualified nonmember `target.qualified_name` | Independently expected public outcome |
|---|---|---|
| Unedited control | Hit, original path retained | Hit, original path retained |
| Absent | Miss, original path recomputed | Miss |
| Explicit null | **Hit, path lost** | **Miss, original path recomputed** |
| Wrong whole type (`123`) | Miss, original path recomputed | Miss |
| One/every wrong element | Not applicable to a scalar | Not applicable |
| Empty string | Miss, original path recomputed | Miss |
| Duplicate key | Miss, original path recomputed | Miss |
| Trailing invalid content | Miss, original path recomputed | Miss |

Every other language/context column in R3-F3.S09r follows the same required-key/string deserializer and source-suffix validator: absent, wrong whole type, empty, duplicate and trailing content reject; unqualified null is a valid hit; the qualified-null exceptions are enumerated above. Retained-member qualified null is now a miss. The remaining per-field matrix below records the complete reader inventory using the prior review's field grouping and the current corrected outcomes. Shared point/span readers enforce the same constraints in every containing span; the independent sweep also edited each coordinate at each site.

The positive fixture is the prior review's TypeScript example: `helper`, a self-calling `again`, `Box.size`, a `caller` containing `box.size()` and `missing()`, a file-level `helper()` call, and an imported `external` called by `useit`. **O** is `again`'s owned self-call; **N** is the qualified `box.size` occurrence; **R** is `caller`'s limited region. **H** means an actual hit; **M** means a miss with recomputation equal to the independently expected unedited index; **—** marks an element shape that does not apply. Numeric empty edits use zero, object empties use `{}`, and string empties use `""`. Duplicate-key edits duplicate the selected key at its containing object; trailing edits append garbage to a restored valid entry.

The independent sweep ran **51 field/control columns and 316 shape edits**, with a positive hit per column and no unexpected structural outcomes. This includes `completed_capabilities`, occurrence kind, and every line/column leaf in all three spans. The remaining failure belongs to the separate qualified-nonmember null context above; a retained member is deliberately the positive qualification control in this fixture.

| Shape | `completed_capabilities` |
|---|---|
| Unedited control | H |
| Absent | M |
| Explicit null | M |
| Wrong whole type | M |
| One wrong element | M |
| Every wrong element | M |
| Empty | M |
| Duplicate key | M |
| Trailing/invalid content | M |

### Lists and occurrence fields

| Shape | `occurrences` | `reference_coverage` | `O.owner` | `O.owner_span` | `O.target` | `O.candidates` | `O.span` | `O.kind` | `O.confidence` |
|---|---|---|---|---|---|---|---|---|---|
| Unedited control | H | H | H | H | H | H | H | H | H |
| Absent | M | M | M | M | M | M | M | M | M |
| Explicit null | M | M | M | M | M | M | M | M | M |
| Wrong whole type | M | M | M | M | M | M | M | M | M |
| One wrong element | M | M | — | — | — | M | — | — | — |
| Every wrong element | M | M | — | — | — | M | — | — | — |
| Empty | M | M | M | M | M | H | M | M | M |
| Duplicate key | M | M | M | M | M | M | M | M | M |
| Trailing/invalid content | M | M | M | M | M | M | M | M | M |

### Reference names and IDs

| Shape | `O.owner.id` | `O.owner.name` | `O.owner.qualified_name` | `O.target.id` | `O.target.name` | `O.target.qualified_name` | `N.candidates.0.id` | `N.candidates.0.name` | `N.candidates.0.qualified_name` | `N.target.qualified_name` |
|---|---|---|---|---|---|---|---|---|---|---|
| Unedited control | H | H | H | H | H | H | H | H | H | H |
| Absent | M | M | M | M | M | M | M | M | M | M |
| Explicit null | M | M | M | M | M | H | M | M | M | M |
| Wrong whole type | M | M | M | M | M | M | M | M | M | M |
| One wrong element | — | — | — | — | — | — | — | — | — | — |
| Every wrong element | — | — | — | — | — | — | — | — | — | — |
| Empty | M | M | M | M | M | M | M | M | M | M |
| Duplicate key | M | M | M | M | M | M | M | M | M | M |
| Trailing/invalid content | M | M | M | M | M | M | M | M | M | M |

### Coverage fields

| Shape | `R.owner` | `R.span` | `R.state` | `R.reasons` | `R.owner.id` | `R.owner.name` | `R.owner.qualified_name` |
|---|---|---|---|---|---|---|---|
| Unedited control | H | H | H | H | H | H | H |
| Absent | M | M | M | M | M | M | M |
| Explicit null | M | M | M | M | M | M | M |
| Wrong whole type | M | M | M | M | M | M | M |
| One wrong element | — | — | — | M | — | — | — |
| Every wrong element | — | — | — | M | — | — | — |
| Empty | M | M | M | M | M | M | M |
| Duplicate key | M | M | M | M | M | M | M |
| Trailing/invalid content | M | M | M | M | M | M | M |

### Occurrence span

| Shape | `O.span.start_byte` | `O.span.end_byte` | `O.span.start` | `O.span.end` | `O.span.start.line` | `O.span.start.column` | `O.span.end.line` | `O.span.end.column` |
|---|---|---|---|---|---|---|---|---|
| Unedited control | H | H | H | H | H | H | H | H |
| Absent | M | M | M | M | M | M | M | M |
| Explicit null | M | M | M | M | M | M | M | M |
| Wrong whole type | M | M | M | M | M | M | M | M |
| One wrong element | — | — | — | — | — | — | — | — |
| Every wrong element | — | — | — | — | — | — | — | — |
| Empty | M | M | M | M | M | M | M | M |
| Duplicate key | M | M | M | M | M | M | M | M |
| Trailing/invalid content | M | M | M | M | M | M | M | M |

### Owner span

| Shape | `O.owner_span.start_byte` | `O.owner_span.end_byte` | `O.owner_span.start` | `O.owner_span.end` | `O.owner_span.start.line` | `O.owner_span.start.column` | `O.owner_span.end.line` | `O.owner_span.end.column` |
|---|---|---|---|---|---|---|---|---|
| Unedited control | H | H | H | H | H | H | H | H |
| Absent | M | M | M | M | M | M | M | M |
| Explicit null | M | M | M | M | M | M | M | M |
| Wrong whole type | M | M | M | M | M | M | M | M |
| One wrong element | — | — | — | — | — | — | — | — |
| Every wrong element | — | — | — | — | — | — | — | — |
| Empty | M | M | M | M | M | M | M | M |
| Duplicate key | M | M | M | M | M | M | M | M |
| Trailing/invalid content | M | M | M | M | M | M | M | M |

### Coverage span

| Shape | `R.span.start_byte` | `R.span.end_byte` | `R.span.start` | `R.span.end` | `R.span.start.line` | `R.span.start.column` | `R.span.end.line` | `R.span.end.column` |
|---|---|---|---|---|---|---|---|---|
| Unedited control | H | H | H | H | H | H | H | H |
| Absent | M | M | M | M | M | M | M | M |
| Explicit null | M | M | M | M | M | M | M | M |
| Wrong whole type | M | M | M | M | M | M | M | M |
| One wrong element | — | — | — | — | — | — | — | — |
| Every wrong element | — | — | — | — | — | — | — | — |
| Empty | M | M | M | M | M | M | M | M |
| Duplicate key | M | M | M | M | M | M | M | M |
| Trailing/invalid content | M | M | M | M | M | M | M | M |


## Blocked Findings

None. Every required repair can proceed within the accepted file-local design. A human review alone is not a readiness blocker and is not requested.

## Recurrence

- **R3-F3.S09r** retains the semantic-cache-consistency class from `review-1.md — Cache validation accepts inconsistent reference evidence`, `review-2.md — Cache validation accepts nonexistent evidence identities and missing callable regions`, and **R3-F3** in review 3. The full retained-member null reproduction is fixed. The remaining nonmember/omitted-member/language cases are **newly enumerated siblings of the original null-loss case**, explicitly left deferred in the repair ledger. This is a partial repair, not a phantom-ID, body-geometry or list-order regression. The missed decision is how to prove a null path was written unqualified; projection and both report formats preserve the lost evidence.
- **R4-F1** is a **newly discovered source-collection sibling** of the written-source-fact issue in review 3's R3-F3. Cached invented-path and retained-member-null repairs did not inspect the collector's own generic, conditional, package, rooted and nested-namespace shapes. The rooted Rust case also repeats the incorrect lexical-binding class from review 1's boundary finding: missing qualification redirects lookup to a local. Collection, category/lookup decisions, summaries, recursion and display were all traced here.
- **R4-F2** repeats `review-1.md — Invocation classification loses calls across supported source forms`, `review-2.md — Transparent call wrappers incorrectly limit coverage`, and review 3's **R3-F1** invocation-operand class. Root-only paths, C# conditional member binding, Perl code-reference calls and PHP variable/placeholder modes are **newly discovered siblings**. The old direct/wrapper/TS-instantiation cases pass. The missed decisions are traversal through a qualified/variable name, Perl named-method versus code-reference call mode and whether call-like syntax executes or creates a callable; both CLI directions consume the resulting kind.

No repair-introduced regression was demonstrated. Passing an older reproduction is not evidence that all sibling grammar shapes were handled. `recurrence: true` carries the complete remaining inventory in this review.

## Requirement Verification and Test Levels

| User-facing requirement | Strongest verification inspected/run | Assessment |
|---|---|---|
| Lexical visibility, order, hoisting, parameters, patterns, aliases and categories | Level 1: occurrence/scope/relation matrices and public ID probes | Previous required repairs pass. Rooted Rust occurrence identity is blocked by R4-F1. |
| Invocation forms, member/constructor capture, declarations excluded, owner boundaries and repeated sites | Level 1: all-language invocation/negative/ownership tests and shipped CLI | R4-F2 has missing and extra calls; R4-F1 has missing/truncated written paths. |
| Honest recursion and indirect/value uncertainty | Level 1: complete/narrowed recursion, value-call and eval matrices; compiled rooted-Rust source | R3-F1/F2/F4 repairs pass. False rooted self-call `No` remains under R4-F1/F2. |
| Established summaries, whole-file evidence, filters, projection, widening, actual warm hits, version invalidation and skipped work | Level 1: reference/query/cache/upgrade tests plus independent source/cache mutation probes | Old identities, geometry and ordering pass; R3-F3.S09r and new source facts still block correctness. |
| Name/ID selection, ambiguity, locals, errors, empty/file-local reports and versioned JSON | Level 1: `refs` tests and independent incoming/outgoing report checks | CLI wiring passes. Wrong/missing rows are recorded under their source/cache findings, rather than excused by passing wiring tests. |
| Terminal headings, grouped rows, styles, links and wrapping | Level 2: live `level2_declarations_in_tmux`, including `check_refs`; WezTerm/Kitty entry points inspected | tmux passed twice. Full L2 stopped at a WezTerm capture timeout before finishing; Kitty's entry point returned via its unavailable-backend gate. No complete WezTerm pass is claimed for this run. Appropriate real-terminal tests exist; no requirement relies solely on a style unit test. |
| OS keyboard input/encoding | No such requirement in this feature | Level 3 is not needed. |

Commands run in `tree-hugger/`:

- `just test symbols_occurrences symbols_reference_summaries symbols_cache cache_robustness refs:: symbols_relations symbols_query symbols_coverage symbols_adapter cache_upgrade stored_evidence parenthesized_terminal_callees commonjs_evidence_counts_anywhere -j 4`: **327 passed**, no failures, 1,057 excluded by the filter. This is scoped evidence, not a claim that the full area's 1,384 tests were rerun.
- `just test-l2`: **2 successful entry points, 1 failed, 3 not run** after fail-fast. The declaration test in tmux exercised the reports; Kitty skipped through its backend gate. WezTerm failed at `capture_scrollback` with “command timed out after 10s,” rather than a rendered-content mismatch. The cause was not established, so this review does not claim an implementation regression or a full L2 pass.
- `just test-l2 level2_declarations_in_tmux`: **1 passed**, independently completing the declaration/refs rendering assertions through a real terminal after the full run's capture failure.
- Root `just check-tier-coverage tree-hugger`: **zero stranded tests**.

The library's declared `l1` binary includes the occurrence, summary, relation, query and cache modules. The CLI `symbols` binary includes refs, cache robustness and cache upgrade. Their ordinary names select Level 1. Both Level-2 targets declare `required-features = ["terminal-tests"]`; the live recipe and CLI CI metadata enable that feature. `check_refs` is called by the tmux, WezTerm and Kitty declaration entry points. No review probe was installed as a test or mistaken for permanent regression coverage.

Experiments used test-owned scratch Git repositories, home/config/temp directories, source copies and cache entries. Commands were noninteractive; real-terminal runs used the shared harness recipes without requesting focus. Lint or formatting was not run for these review-only document changes. Cross-OS evidence is left to CI and does not determine readiness here.

## Nonblocking Recommendations

None separate from the required repairs. Indexed lookup, a single definitions/reference collection, whole-file evidence projection and record-only semantic widening remain appropriate. No broader refactor, compiler integration or new performance gate is requested.
