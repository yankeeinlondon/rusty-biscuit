---
$schema: feature-review.yaml
ready: false
findings:
  - title: R6-F1 — Force-unwrapped and null-forgiving invocations disappear from call reports
    priority: high
  - title: R6-F2 — Parameter readers mistake comments for default values
    priority: medium
  - title: R6-F3 — Contextual syntax markers become false reference suggestions
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-06T21:39:42-07:00
spec: 2026-10-08-scoped-references/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-08-scoped-references/log.md
description: "A **feature** review of `2026-10-08-scoped-references/spec.md`"
feature: 2026-10-08-scoped-references/review-6.md
previous: 2026-10-08-scoped-references/review-5.md
next: 2026-10-08-scoped-references/review-7.md
---

# Scoped References: review 6

The feature is **not production ready**. The required instances and consumers of all four review-5 findings are verified fixed. Three additional findings remain: omitted Swift/C# calls, incorrect parameter defaults, and extra Java/Lua reference suggestions. The C# comment-default result is introduced by the latest repair; the other cases are missed siblings. None requires a human decision.

This review read the specification, plan, original implementation log, reviews 1–5 and the repair ledger. It preserved the implementation edits and changed only this review, review 5’s `next` link and the specification’s iteration count. Source errors were reproduced through the shipped CLI and public library/cache APIs. Existing passing tests establish the earlier repairs, not correctness of untested source forms.

## Previous Finding Verification

“Verified fixed” closes the complete earlier enumerated inventory and its consumers, not every possible sibling. The rows include all historical findings, including the legacy single-Findings reviews, and reconcile additional open observations in the ledger. All test evidence labeled “current L1” was run in this review’s full 1,412-test run.

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
|---|---|---|---|---|
| `review-1.md — Lexical lookup crosses boundaries that do not establish a binding` | Rust inline modules, nested functions/constants/statics, closure controls, conditional chains/else/following statements and loop controls; TS/JS `with`, direct eval and defaults; occurrences, summaries, recursion, relation lookup and CLI | **verified fixed** | Current L1: `LEXICAL_BOUNDARY_MATRIX`, boundary summaries/recursion, shared relation lookup and boundary CLI tests pass. Lookup enforces module/item boundaries and conditional endpoints. | None of the earlier required instances remains failing. |
| `review-1.md — Named function expressions bind their names to outer declarations` | TS/JS ordinary and generator expression names, defaults/internal calls, final outer calls, unnamed/class/arrow controls; exclusions, placement, summaries, recursion, relation lookup and callers | **verified fixed** | Current L1: `EXPRESSION_NAME_MATRIX`, expression-local summaries/recursion/relation and CLI callers pass. The shared binding inventory excludes expression names and confines visibility to the expression. | No unchanged case remains; new syntax-marker exclusions are R6-F3. |
| `review-1.md — Invocation classification loses calls across supported source forms` | Direct/wrapped calls in all 16 languages, TS type-only wrappers, member/generic wrappers, Perl/shell calls, receiver/argument/sequence/conditional/dereference negatives; reports, recursion, terminal-call and CommonJS readers | **verified fixed** | Current L1: all-language invocation/negative matrices, wrapped binding, CLI, terminal-call and CommonJS tests pass. R6-F1 is outside the earlier enumerated wrappers. | R6-F1 is a newly discovered wrapper sibling; earlier direct/parenthesized cases pass. |
| `review-1.md — Cache validation accepts inconsistent reference evidence` | Wrong/malformed/lost retained target IDs, named/ownerless targets, owner/candidate identity, duplicate/omitted callable regions, structural matrix, summaries/dependencies/semantics, null and narrowed controls | **verified fixed** | Current L1: structural/reference matrix, retained-evidence tests and two-way summary/semantic validation pass; strict stored types require keys and distinguish nullable values. | None; accepted anonymous-evidence and unrelated relation-validation limits are preserved. |
| `review-2.md — Parameter defaults incorrectly see body-local var bindings` | Seven callable forms in TS/JS; ordinary/block/destructured/loop `var`; parameter visibility, redeclaration, hoisting, computed/destructured defaults and body eval; let/const/function controls, relation lookup and reports | **verified fixed** | Current L1: both languages’ callable-form/body-declaration matrix, parameter-environment controls, relation lookup and CLI tests pass. `var_body` isolates body bindings from defaults. | None. Default *metadata* in R6-F2 is separate from parameter-environment lookup. |
| `review-2.md — Pattern capitalization invents or omits Rust bindings` | Bare/modified capitalized bindings, lowercase constants in patterns/body/guards, paths/tuples/shorthands and irrefutable controls; walker, capture fallback, exclusions, declaration records, summaries, recursion and callers | **verified fixed** | Current L1: pattern identity/record, summaries, recursion and CLI rows pass. Query repairs also remove Rust path/range captures; actual binding identity precedes record exclusion. | None of the earlier required pattern cases remains failing. |
| `review-2.md — Transparent call wrappers incorrectly limit coverage` | Rust parentheses/generics/type paths, all enumerated TS wrappers, JS parentheses and constructors; direct/self/member/value/sequence/conditional/dereference controls, eval, recursion and shared relation scopes | **verified fixed** | Current L1: callee-coverage and complete/narrowed recursion matrices, shared operand/eval readers pass. Calls to another established declaration still prove No where coverage is complete. | None of the earlier required Rust/ECMA coverage cases remains failing. |
| `review-2.md — Cache validation accepts nonexistent evidence identities and missing callable regions` | Every-language phantom owners, applicable candidates/targets, owned/ownerless targets, Python/Lua empty callable regions, summaries, callable/narrowed entries and null/bodiless controls | **verified fixed** | Current L1: every-language retained identity mutation tests and empty Python/Lua callable-region cases pass. Missing identities require a compatible omitted kind; regions use retained body facts. | None. |
| R3-F1 — TypeScript instantiation expressions hide calls and constructor invocations | `.01–.18`, `.C1–.C7`, ledger `.R`: wrapper/member/constructor operands, usages/arguments and all-language controls; classification, candidates, summaries, coverage/recursion, projection/widening, CLI and revision invalidation | **verified fixed** | Current L1: TS instantiation invocation/coverage/recursion, member candidates, cache narrowing/widening/hits, CLI and stale revision tests pass. Both classifier and operand reader share the instantiation rule. | None of .01–.18/C1–C7/R remains failing. |
| R3-F2 — Rust associated value calls falsely prove absence of recursion | `.01–.16`, `.C1–.C5`: inherent/trait/enum/generic/wrapped value paths and unrelated initializer controls; function/constructor/usage controls, candidate fallback, coverage, summaries, narrow recursion, semantic widening and callees | **verified fixed** | Current L1: associated value call, summary and complete/narrowed recursion rows, record-only semantic widening and CLI coverage pass. Unique members and uncertain candidates both check value kinds. | None; initializer inference remains excluded. |
| R3-F3 — Cache validation accepts false source facts and misordered or duplicate lists | `.L01–.L16.{region,path,candidates}`; `.S01–.S03.{rs,ts,js}.{duplicate,reverse}`; `.S04–.S10`; `.S09r.{rs,ts,js,go,py,java,cs,c,cpp,swift,scala,php,lua,perl,sh,zsh,omitted}`; `.C1–.C4`; `.M.kind` and eight owner/coverage coordinate leaves | **verified fixed** | Current L1: all-language source-fact/candidate/region edits, duplicate/reversed summaries, path/start deletion, omitted-record and full leaf matrices pass. Retained geometry, written source paths, list order and summaries are checked. Perl collection was repaired at 19:02 before the earlier 19:14–19:25 cache repair; this cycle preserves that unblocking. Coordinated deletion of both redundant fields remains the review-4-excluded limit. | None of the carried S09r or nine M leaves remains unresolved. |
| R3-F4 — Optional and tagged eval calls incorrectly hide lexical declarations | `.01–.07`, `.C1–.C5`: TS/JS optional/tagged/wrapped eval, genuine direct/sequence/member/alias controls; scope barriers, declared bases, summaries/candidates, coverage/recursion, filtering/cache/callers | **verified fixed** | Current L1: `EVAL_CALL_FORMS`, scope/relation/summary/coverage/recursion, cache and CLI tests pass. Both binding and coverage distinguish optional/tagged calls from direct eval. | None; conservative direct-eval handling remains accepted. |
| R4-F1 — Written-path collection drops qualifiers and can bind rooted names to locals | `.01–.28`, `.C1–.C7`: roots, nested C++ paths/generics, C# alias/conditional/generic paths, PHP nullsafe/namespaces, Perl package/receiver paths and all-language controls; exclusions, lookup/category/stopping, candidates/summaries, recursion, projection/cache/reporting/docs/revision | **verified fixed** | Current L1: `WRITTEN_PATH_MATRIX`, rooted Rust target/summary/recursion, narrow/wide/warm/projection, text/JSON reports and stale-entry tests pass. Rooted names never fall into local lookup. Java receiver rows are now independently restored too. | None of .01–.28/C1–C7 remains failing. |
| R4-F2 — Invocation classification omits calls and mistakes PHP callable creation for execution | `.01–.24`, `.C1–.C6`: rooted calls, C# conditional/generic/delegate chains, PHP placeholder versus unpacking for named/qualified/static/instance/variable forms, variable constructors, Perl code-reference and name-only negatives; coverage, candidates/summaries, filtering/cache/reports/docs/revision | **verified fixed** | Current L1: every .01–.24 invocation row, placeholder/unpacking negatives, conservative summaries/recursion, cache/projection and report tests pass. The invocation climb handles those roots, conditional members and variable calls. | R6-F1 adds force-unwrapped/null-forgiving callee wrappers; no .01–.24 regression. |
| R5-F1 — Invocation classification still omits constructed and implicit member targets | .01–.22, .C1–.C6; ledger .S1–.S20: Java/Scala/Swift/C++ constructed/generic/implicit targets, all-language negatives, binding/candidates, summaries, coverage/recursion, cache/projection, reports/docs/revision | **verified fixed** | All required review-5 rows pass current invocation/negative matrices, conservative-result assertions, six-stage cache tests and `refs::constructed_types_and_implicit_members_are_callees_and_callers`. Swift subscripts remain Usage. Scope is closed for those rows; ledger .S12’s additional grammar limitation is evaluated separately below. | New wrappers not enumerated previously are R6-F1. |
| R5-F2 — Unreported declaration tokens leak into reference occurrences | .01–.38, .C1–.C6 and .S1–.S20: all declaration sites/controls, constraints/defaults/body reads, exclusions/candidates, ownership/summaries/recursion, filter/cache/CLI/docs/revision | **verified fixed** | Current `DECLARATION_EXCLUSION_MATRIX` asserts exact missing declaration offsets **and** retained real-read offsets; all rows pass. Shared Python/Scala pattern readers, wildcard controls, cache unreported-declaration stages and CLI suggestions pass. Exclusion precedes candidate selection. | R6-F3 covers separate contextual syntax-marker tokens, not any old declared-name row. |
| R5-F3 — Receiver-qualified Java construction loses its written target | .01–.10, .C1–.C5, .S1–.S15; R5-X.01–.15 (fixed Python-star/PHP-static siblings plus controls): receiver/generic/anonymous paths, candidates/lookup, reports, cache/projection/docs | **verified fixed** | Current full-path matrix, conservative Java target assertions, `written_receiver_paths_hold_through_the_cache` and receiver text/JSON reports pass. Exact receiver paths, independent receiver/argument reads, Swift operator exclusion, Python star and PHP static paths are asserted. R5-F1’s generic-classification prerequisite was repaired before these dependent rows. | None of the required paths remains failing. |
| R5-F4 — Parameter captures turn type and default-value references into declarations | .01–.16, .C1–.C6, .S1–.S3: Python/C# callable forms, actual record lists/types/default reads, PHP constants/defaults and Rust pattern paths, counts/signatures/candidates/selection/cache/provenance/revision | **verified fixed** | Current parameter-capture matrix, signature tests, cache six-stage exact record/read assertions, CLI ambiguity and revision-43 invalidation pass. Python/C#/PHP/Rust queries field binding names precisely. R5-F4 was repaired before R5-F2 exclusion consumed its definition set. | R6-F2 is a new comment-position sibling; C# comment-as-default is a repair regression, not a failure of the previously enumerated bare default/type cases. |
| R5-F1.S12 — Swift specialized explicit initializer path (repair-ledger discovery) | `C<Int>.init()` written qualifier; classification and conservative coverage; ledger’s blocked grammar prerequisite | **still blocked** | Independent pinned-grammar/CLI probe reproduces a no-path `init` Call when the grammar chooses comparisons. Another context parses the path correctly, so the limitation is context-sensitive. Grammar 0.7.1 is unchanged; no prerequisite was removed before or after repair. Original accepted design explicitly uses the pinned grammar and limited unsupported-language evidence. | Optional grammar expansion; not an unimplemented resolved form or required review-5 repair. The limitation stays documented; it does not by itself block this contract. |
| Repair-ledger observations — Java inferred-type marker and Lua variable attributes | `var` in Java type-inference slots; Lua `<const>`/`<close>`; real same-named reads, candidates, reports and storage | **unresolved** | Independent all-language marker sweep and Java/Lua default reports reproduce false NameOnly suggestions. Ledger explicitly observed these tokens but excluded them from its declaration-name class. They still violate occurrence meaning. | Carried into R6-F3 with explicit child IDs; no prerequisite or human blocker. |

No earlier required repair was blocked by human review. The original implementation log’s accepted decisions establish Rust/TypeScript/JavaScript resolution, conservative occurrences elsewhere, the pinned grammar, file-local analysis and no receiver inference. They preceded the repairs and remain accepted. The single-field cache-path repair was not waived: it is now verified, including the formerly missing columns. The ledger’s “four parents fixed” claim matches the required review-5 rows, but does not close the syntax-marker observations or expand grammar support.

## Unblocked Findings

### R6-F1 — High — Force-unwrapped and null-forgiving invocations disappear from call reports

**Defect class:** transparent callee traversal omits language-specific postfix wrappers that retain the callable operand, so syntactic invocations remain usages and are filtered out of callers and callees.

In the tree-hugger library, [wrapper_operand](../../lib/src/analysis/occurrences.rs:1516) identifies syntax that preserves an operand, and [is_call_target](../../lib/src/analysis/occurrences.rs:1358) uses it to classify reference sites. TypeScript’s non-null wrapper works. Swift’s `postfix_expression` with `!` and C#’s `postfix_unary_expression` with `!` are absent. Both pinned grammars parse these examples without errors:

```swift
func g(cb: (() -> Void)?) { cb!(); (cb!)(); cb?() }
```

```csharp
class G {
    System.Action F;
    void g(System.Action cb) { cb!(); (cb!)(); F!(); (F!)(); cb(); F(); }
}
```

`swiftc -typecheck` accepts the bare/grouped Swift force-unwrapped invocations. The Swift complete index labels the first two `cb` tokens Usage and the optional call Call; the callee report contains only the optional call. C# similarly labels all four `!` invocations Usage and reports only the two ordinary calls. These are calls through values; correcting the kind must preserve uncertain binding and must not infer an initializer or executable method implementation.

Every defective shape below was exercised through the complete-index CLI; representative sources were also appended to copies of the shipped Swift and C# fixtures. Expected results come from invocation syntax, separately from cached/uncached agreement.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R6-F1.01 / Swift force unwrap | `cb!()` | cb Usage | cb Call | CLI, error-free pinned tree, Swift compiler |
| R6-F1.02 / Swift grouped unwrap | `(cb!)()`, `((cb)!)()` | cb Usage | cb Call at arbitrary grouping depth | CLI, pinned tree, compiler |
| R6-F1.03 / Swift member unwrap | `x.cb!()`, `(x.cb!)()` | member cb Usage; path x.cb preserved | member cb Call; receiver x Usage, same written path | CLI and member-tree inspection |
| R6-F1.04 / C# null-forgiving operand | `cb!()`, `(cb!)()` | cb Usage | cb Call | CLI; error-free `postfix_unary_expression` tree |
| R6-F1.05 / C# member and nested grouping | `obj.F!()`, `(obj.F!)()`, `((obj.F)!)()`; local field `F!()` | F Usage; member path preserved | F Call, receiver Usage; preserve candidates/path | CLI, field/parameter candidate fixture, pinned tree |
| R6-F1.06 / Swift negative/control | `cb!`, `use(cb!)`, `(condition ? cb : other)!()`, `(a[0]!)()`; `cb?()`, `x!.f()`, ordinary calls | value/argument/conditional/subscript names Usage; optional/direct targets Call | Preserve those results; unwrap traversal must not cross a conditional or subscript into an invented target | Independent negative CLI probe and prior invocation matrices |
| R6-F1.07 / C# negative/control | `cb!`, `consume(cb!)`, `(condition ? cb : other)!()`, `(arr[0]!)()`; `obj!.F()`, ordinary calls | Correct Usage/Call split | Preserve; postfix increment/decrement are not null-forgiving wrappers | Negative CLI probe; exact `!` token invariant, not a rule for every postfix node |
| R6-F1.08 / TypeScript | `cb!()`, `(cb!)()`, `obj.F!()`, `(obj.F!)()`, optional calls | Call | Clean; retain type-only wrapper and member collection | Independent CLI and current invocation/coverage/recursion tests |
| R6-F1.09 / JavaScript | optional bare/member/grouped calls | Call | Clean; no postfix null assertion in its grammar | Independent CLI and current matrices |
| R6-F1.10 / Rust | parentheses, turbofish, qualified methods; `?`/dereference computed operands | Current positive/negative matrix passes | Clean: no force-unwrapped nullable callee; `?` is control flow, not a TS assertion | Current grammar-specific invocation/coverage/recursion rows |
| R6-F1.11 / C/C++ | grouped names/members, template targets, explicit dereference | Called names Call; dereferenced name Usage | Clean; dereference computes a callee, so do not generalize the new rule to it | Independent C++ CLI and current C/C++ matrices |
| R6-F1.12 / Scala | `(f[Int])()`, `(x.f[Int])()`, `(x.f)()`; `(f _)()` | Generic/member targets Call; eta-expanded method name Usage | Clean; eta expansion creates a function value and is not a type-only wrapper | Independent CLI and current matrices |
| R6-F1.13 / Python/Go/Lua | each language’s direct and grouped callable names/member controls | Required Call/Usage split passes | Clean; no corresponding null-assertion postfix wrapper. Go’s indexed-generic ambiguity remains accepted | Current all-language invocation/negative rows |
| R6-F1.14 / Java | method/constructed targets, generic and receiver construction | Call | Clean; no callable-name null-forgiving syntax | Current exact invocation/receiver matrices |
| R6-F1.15 / PHP/Perl | variable/grouped calls, creation/unpacking, code-reference calls and name-only negatives | Required Call/Usage split passes | Clean; preserve the accepted PHP constant/property distinction | Current paired mode/negative rows and CLI tests |
| R6-F1.16 / Bash/Zsh | command name and argument/expansion controls | Command targets Call; arguments Usage | Clean; these readers use command position, not postfix-expression traversal | Current shell matrices and CLI tests |
| R6-F1.17 / Swift repeated/mixed assertions | Optional-optional cb: `cb!!()`, `(cb!)!()`, `((cb!)!)()`, `cb?!()`, `(cb!)?()` | Every cb Usage | Every cb Call; preserve optional dispatch and uncertain value binding | Independent CLI, `swiftc -typecheck` accepts the complete row |
| R6-F1.18 / C# repeated/conditional assertions | `cb!!()`, `(cb!)!()`, `((cb!)!)()`, `obj?.F!()`, `obj.F!!()` | cb/F Usage | Final cb/F Call; receiver obj Usage; preserve `obj?.F`/`obj.F` paths | Independent CLI and the same postfix-operand decision |

| Child ID | Decision/consumer | Observed effect and required repair |
|---|---|---|
| R6-F1.C1 | Capture, token spans, wrapper and generic/member traversal | All defective names are already captured. Add only the `!` operand shapes to shared traversal, at arbitrary wrapper depth; keep token spans, qualified member paths and generic arguments intact. |
| R6-F1.C2 | Lookup/category/candidates and stopping | Swift/C# remain NameOnly/Unresolved, with absent target IDs. The parameter/field fixture identifies the expected candidates independently. Null assertions do not justify resolution or following stored values. |
| R6-F1.C3 | Summaries, coverage and recursion | Limited-language established summaries stay empty, every region retains unsupported-language coverage and callable recursion stays Unknown. Rust/ECMA direct-call, indirect-value and eval readers must keep their passing decisions. |
| R6-F1.C4 | Filtering, projection, storage and widening | Both defective sources remain wrong through functions-only Miss → Hit → semantic widening (`parsed: false`) → kind widening → complete Hit → projected Hit. Assert expected Calls at each stage; equality alone is not the oracle. |
| R6-F1.C5 | CLI default/incoming/outgoing, ID selection, text/JSON | [build_report](../../cli/src/refs.rs:271) in tree-hugger-cli filters callers/callees on Call. Independent ID-selected default reports show the erroneous Usage rows, and both Call directions omit them, cold and warm with actual hits. Repair tests must assert all required rows and unchanged negative rows. |
| R6-F1.C6 | Other readers, docs and invalidation | `invoked_operand` shares the wrapper table; supported coverage/eval and wrapped-member consumers must remain consistent. Terminal-call and CommonJS readers have their own passing contracts. Add declared L1 regressions, update invocation docs/comments and invalidate stale extractor entries. |

**Required repair:** classify every .01–.05 and .17–.18 invocation correctly through shared language-specific wrapper rules, preserve .06–.16 controls, and verify .C1–.C6. No broader resolver or receiver inference is needed.

### R6-F2 — Medium — Parameter readers mistake comments for default values

**Defect class:** parameter readers choose neighboring syntax children rather than the default expression, allowing comment nodes to change optionality or become a reported default.

The tree-hugger library’s newly repaired [extract_csharp_parameter](../../lib/src/file/extract/signatures.rs:887) requires the name’s immediate next sibling to be `=` and then takes the next named sibling as the value. A comment is a named sibling too. The shared [find_default_value_after_equals](../../lib/src/file/extract/signatures.rs:1284), used by PHP/Scala and the Swift single-parameter helper, likewise returns the first child after `=`.

```csharp
class G {
    void g(int x /* before */ = 1, int y = /* after */ 2) {}
}
```

The public record marks x required (`is_optional: false`, no default) and y optional with `default_value: "/* after */"`. The ID-selected text reference report displays `g(x: int, y: int = /* after */)`. The independently expected metadata is optional x/y with defaults `1`/`2`. Comments do not alter parameter contracts. C# comment-as-value is introduced by the latest repair: the prior reader omitted defaults entirely, while the new reader invents a comment default. PHP/Scala already have the same missed sibling in the shared helper.

The sweep used an unedited `g(x=1,y=2,z=3)` control, then inserted one comment before `=`, after `=`, or on both sides in an otherwise identical h. Equivalent syntax was used for all languages with parameter defaults. Primary C#/PHP/Scala fixture copies reproduce the failures.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R6-F2.01 / C# before equals | `int x /* before */ = 1` | no default; required | default 1; optional | Complete-index metadata, pinned tree |
| R6-F2.02 / C# after equals | `int y = /* after */ 2` | default is comment text | default 2; optional | Complete-index metadata and reference declaration/header |
| R6-F2.03 / C# both positions | `int z /* before */ = /* after */ 3` | no default; required | default 3; optional | Same fixture/control; this combines both sibling decisions |
| R6-F2.04 / PHP helper | before/after/both comments | before-only correct; after/both comment text | defaults 1/2/3; all optional | Public CLI, copied PHP fixture; helper called by PHP parameter extraction |
| R6-F2.05 / Scala helper | before/after/both comments | before-only correct; after/both comment text | defaults 1/2/3; all optional | Public CLI, copied Scala fixture; shared helper |
| R6-F2.06 / Swift | before/after/both comments | correct defaults and optionality | Clean; default is a declaration sibling carrying `default_value` | Public CLI; `extract_swift_parameters` uses the field and supplies the correct value. The helper call in `extract_swift_single_parameter` is still part of the repair inventory if its node contains a default. |
| R6-F2.07 / Python | line comments before/after equals inside multiline parameter list | correct defaults and optionality | Clean | Public CLI; default reader uses `value` field |
| R6-F2.08 / JavaScript/TypeScript | before/after/both block comments | correct defaults and optionality | Clean | Both language probes through the public CLI; default reader uses `right`/`value` fields |
| R6-F2.09 / C++ | before/after/both comments | correct defaults and optionality | Clean | Public CLI; reader uses `default_value` field |
| R6-F2.10 / Rust/Go/Java/C/Lua/Perl/Bash/Zsh | default-reader applicability | No corresponding default-expression reader for these ordinary parameter forms | Clean invariant: these paths do not use the affected helper to manufacture a default | Parameter dispatcher and language-reader code; their parameter rows pass current L1 |
| R6-F2.11 / all default readers | unedited control, grouped/default-call values and noncomment type/name data | plain controls correct; comments produce no extra parameter records or occurrences | Preserve names, types, declaration counts, arity and genuine default-expression reads | Current parameter/record/signature tests and independent control records; this defect changes metadata, not the number of declared parameters |
| R6-F2.12 / C# constructor/local-function consumers | constructor `G(int x /* before */ = 1, int y = /* after */ 2)` and local `Local` with the same parameters | both lose x optionality and use the comment as y default | both parameters optional with defaults 1/2 in both callable records | Independent complete-index CLI; constructor and local-function FunctionData inspected separately |
| R6-F2.13 / PHP constructor/method consumers | `__construct` and `method` with before/after comments | before-only correct; both after-comment defaults are comment text | preserve defaults 1/2 and optionality | Independent complete-index CLI; each named callable record inspected |
| R6-F2.14 / Scala method consumer | class method with before/after comments | before-only correct; after-comment default is comment text | preserve defaults 1/2 and optionality | Independent complete-index CLI; method metadata inspected separately from top-level function |

| Child ID | Decision/consumer | Required repair or verified invariant |
|---|---|---|
| R6-F2.C1 | C# reader and shared PHP/Scala/Swift helper | Select the actual expression after `=`, skipping grammar extras; use fields where available. Both the search for `=` and the selection after it must survive comments. |
| R6-F2.C2 | Callable consumers | Every method, constructor, local function or other extracted callable that delegates to these readers inherits their decision. Exercise method/constructor/local-function C# paths and PHP/Scala functions/methods; preserve actual name/type/default reference collection, rather than changing records just to repair display. |
| R6-F2.C3 | Public parameter metadata and presentation | Default presence determines `is_optional`, and text declaration rendering displays defaults. Check the actual expression text, optionality and short reference header. Counts/arity/name/type selectors remain unchanged and must stay so. |
| R6-F2.C4 | Filtering/cache/projection/widening | C#/PHP/Scala default metadata remains wrong through all six cache stages described in R6-F1.C4, including actual Hits and record-only semantic widening. Correct source metadata, then invalidate stale entries; a cache reader cannot infer the intended expression from a stored comment. |
| R6-F2.C5 | Tests and docs/comments | Add one comment-position matrix per supported default language in a declared L1 target; verify public records and reference headers. Review the edited reader’s docs/comments and existing signature contract. This finding is in the previous repair’s required signature/default consumer scope; PHP/Scala are sibling sites outside its enumerated cases. |

**Required repair:** repair .01–.05, .12–.14 and the shared helper, preserve .06–.11, and verify .C1–.C5. A comment must neither erase a default nor supply one.

### R6-F3 — Medium — Contextual syntax markers become false reference suggestions

**Defect class:** broad identifier captures treat built-in syntax markers as names being referenced, so same-named declarations gain extra incoming suggestions even though no binding is read.

The new tree-hugger library [declaration-name inventory](../../lib/src/analysis/declaration_names.rs:76) fixes declared tokens. [Resolver::occurrence](../../lib/src/analysis/occurrences.rs:498), which filters captures before lookup, still retains Java’s inferred-type `var` and Lua variable-attribute names. The repair ledger explicitly observed these but left them outside its declared-name class. They are still incorrect reference evidence.

```lua
local const=1; local close=2
local x <const> = const
local y <close> = close
```

An ID-selected `hug refs` report for const shows Usage at line 2, column 10 (the built-in attribute) **and** column 19 (the real initializer read); only column 19 belongs. Close has the same extra row on line 3. Never filter by spelling globally: the initializer and later same-named reads must remain.

```java
record R() {
    void g() { var x=factory(); int var=1; use(var); }
}
```

The modern record syntax establishes that this example uses Java supporting inferred `var`. Selecting the actual variable named var yields both the type-inference marker and the argument read as suggestions. The marker is not a reference to that variable. Its declaration token is correctly excluded already. Preserve genuine type names and legal same-spelled variables; do not impose a spelling-only filter or silently assume a language version for ambiguous legacy source.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R6-F3.01 / Lua const attribute | `local x <const> = const` with declared const | attribute and initializer both Usage/NameOnly | exclude attribute; keep initializer reference | CLI, error-free `attribute(identifier)` tree, default report, copied Lua fixture |
| R6-F3.02 / Lua close attribute | `local y <close> = close` with declared close | same two references | only initializer Usage | Same; both built-in Lua variable attributes swept |
| R6-F3.03 / Lua controls | declaration and later argument reads of const/close; computed table/member names | genuine reads retained | Preserve; only identifier nodes in attribute syntax are excluded | Independent CLI plus current Lua declaration/operand matrices |
| R6-F3.04 / Java local inferred type | `var x=factory()` inside the record reproduction | var Usage, candidate is later variable var | exclude inference marker, keep factory Call and later var read | CLI/default ID report; copied Java fixture |
| R6-F3.05 / Java inferred-type slots | enhanced-for `for(var y:xs)`, resource `try(var r=f())`, lambda `(var z)->z`, another local declaration | every type-slot var is Usage | exclude inferred-type marker; retain iterable/initializer/body reads | Complete-index CLI and pinned type-slot trees |
| R6-F3.06 / Java controls/version boundary | `int var=1; use(var)`; ordinary named types; ambiguous legacy class/import named var | actual reads must survive; modern reproduction produces the false marker row separately | retain real variables/types; if older-version type syntax is genuinely ambiguous, do not assert a false binding or erase a provable real type read | Token-role invariant; modern record fixture disambiguates defective cases. No Java compiler/version integration requested. |
| R6-F3.07 / Rust/TypeScript/JavaScript | visibility/static/async/mutability/type operators and ordinary names | tested modifier tokens do not become reference rows | Clean; grammar tokens and existing exclusions distinguish modifiers from identifier uses | Independent three-language marker CLI sweep, current binding/exclusion matrices |
| R6-F3.08 / Python/Go/C/C++ | async/await/type alias, go/defer, storage/typedef/constexpr/auto modifiers | tested modifier tokens absent; actual calls/type/value reads present | Clean; built-in types may be reported according to the reference query, but modifier tokens are not invented bindings | Independent four-language marker CLI sweep and reader/query invariants |
| R6-F3.09 / C# | record/readonly/async/var/await modifiers; dynamic type-use control | tested modifiers absent; type/value reads retained | Clean for the modifier class; do not strip a possible named type merely because its spelling is contextual | Independent CLI and grammar/occurrence role invariant |
| R6-F3.10 / Swift/Scala/PHP | static/mutating/async/throws, lazy/inline/given/using, readonly/static | tested markers absent; actual type/value reads retained | Clean | Independent three-language marker CLI sweep and current matrices |
| R6-F3.11 / Perl/Bash/Zsh | my/our/state; local/readonly/export versus expansions and command names | declaration modifiers not emitted as references; ordinary commands/reads retained | Clean; a shell builtin used as a command is legitimately a Call | Independent CLI sweep and current shell/Perl matrices |

| Child ID | Decision/consumer | Required repair or verified invariant |
|---|---|---|
| R6-F3.C1 | Capture, role exclusion and token offsets | Exclude marker tokens by grammar role before candidates are chosen; keep actual declaration exclusions and later reads. Both .01/.02 attributes and all .04/.05 Java slots are required. |
| R6-F3.C2 | Categories/candidate fallback and lookup stopping | Java/Lua use the all-record NameOnly fallback; the unrelated var/const/close records must not receive these tokens as candidates. Keep genuine uncertain reads and conservative IDs. No resolver expansion is needed. |
| R6-F3.C3 | Ownership, summaries/coverage/recursion | False marker rows remain Usage; unsupported summaries are empty and recursion Unknown. Excluding syntax markers must not remove coverage regions or upgrade unsupported languages to Complete. |
| R6-F3.C4 | Default reports versus Call directions | Cold/warm ID-selected default reports retain the extra rows with actual hits; callers/callees already reject them as Usage. Assert exact default offsets and genuine read retention, not just empty Call reports. |
| R6-F3.C5 | Narrowing, projection/cache/widening | Java/Lua probes carry the false markers through functions-only cache entries, record-only semantics widening, kind widening, complete/projected Hits. Evidence must become correct before storage; retain whole-file real reads when declarations are filtered. |
| R6-F3.C6 | Tests, docs and revision | Extend the existing occurrence/exclusion matrices with marker roles and same-spelling controls, plus CLI and cache assertions. Update exclusions docs/comments and stale-entry invalidation. Do not resolve this by adding synthetic declarations for keywords. |

**Required repair:** remove every false marker occurrence in .01/.02/.04/.05 with syntax-aware exclusion, preserve the real reads and all controls, and verify .C1–.C6. The ledger observations are now required repairs with stable child IDs.

## Blocked Findings

None of the three required repairs is blocked. The ledger’s R5-F1.S12 remains a documented pinned-grammar limit under the accepted release boundary, not a request to upgrade grammars as part of this repair cycle. Human review is not required and does not waive the unblocked work.

## Recurrence

- **R6-F1** repeats `review-1.md — Invocation classification loses calls across supported source forms`, `review-2.md — Transparent call wrappers incorrectly limit coverage`, R3-F1, R4-F2 and R5-F1. These are **newly discovered siblings**: earlier tests cover parentheses, generics, TS assertions, constructors, optional calls and receiver assertions, but not Swift/C# assertions on the **invoked operand**. Classification, shared operand traversal, CLI Call filtering and stored projections were the missed decisions/consumers. Earlier required rows still pass.
- **R6-F2** repeats R5-F4’s parameter syntax-role confusion in a previously required signature/default consumer. The C# comment-as-default is a **repair regression** introduced by replacing the old default reader with adjacent-child lookup; optionality loss before `=` is an unclosed sibling. PHP/Scala are **newly enumerated pre-existing siblings** in the shared helper. The missed decisions are skipping grammar extras before/after `=`, deriving optionality, displaying defaults and persisting record metadata. None is a failure of the prior bare type/default capture cases.
- **R6-F3** repeats the extra-occurrence/exclusion class of `review-1.md — Named function expressions bind their names to outer declarations`, `review-2.md — Pattern capitalization invents or omits Rust bindings` and R5-F2. These are **unchanged ledger observations outside the prior enumerated declaration cases**, now enumerated as contextual-marker siblings. No repair regression is claimed. The missed decisions are role exclusion before all-record candidates, incoming default suggestions and whole-file cache evidence.

## Requirement Verification and Test Levels

| User-facing requirement | Strongest verification inspected/run | Assessment |
|---|---|---|
| Lexical visibility/order/hoisting/categories/import aliases and supported boundaries | L1 scope, relation, occurrence, summary/recursion and CLI matrices | Earlier required repairs verified; limited languages intentionally remain conservative. |
| Call/Usage split, constructors, receivers, wrappers and repeated sites | L1 exact all-language matrices and independent CLI/grammar probes | R6-F1 loses calls; required constructor and receiver repairs pass. |
| Declaration exclusions, token spans/paths, owners and whole-file code | L1 exact excluded/retained offsets, all-language paths/ownership and reports | Earlier exclusions/paths pass; R6-F3 adds false marker rows. |
| Shared records, parameter facts and reference headers | L1 exact records/signatures, independent CLI metadata/header probes | R5-F4’s captured-name/default-read repairs pass; R6-F2 corrupts defaults/optionality. |
| Honest recursion and established relationships | L1 complete/narrowed recursion and summary matrices | Old false Yes/No cases pass; new limited-language repairs must retain Unknown and empty established summaries. |
| Selection, projection, skipped work, persistence/widening and version invalidation | L1 query/cache/counter/upgrade matrices plus independent six-stage API probes | Infrastructure passes; incorrect source facts propagate consistently and need collection/reader repairs. |
| Exact/smart-case/ID selectors, ambiguity, errors/empty results and boundary | L1 refs suite and independent ID-selected reports | Prior false-parameter ambiguity is fixed. New Call omissions and extra default suggestions remain. |
| Grouped terminal rows, wrapping, links/style and boundary on empty reports | L2 real tmux capture via `level2_declarations_in_tmux` | Passed. Appropriate tier is live; no full WezTerm/Kitty evidence is claimed. |
| Keyboard input encoding/hotkeys | No such requirement | L3 unnecessary. |

Verification performed:

- `just test -j 4` in `tree-hugger/`: **1,412 passed, 0 skipped**, 177.530 seconds. Every earlier repair’s permanent tests and the new declared L1 modules ran.
- `just test-l2 level2_declarations_in_tmux`: **1 passed**, 5.145 seconds; 377 tests excluded by feature/tier/filter. Shared harness managed the terminal without requesting focus.
- Root `just check-tier-coverage tree-hugger`: **0 stranded tests**. The library’s `l1` target includes occurrence/query/summary/cache/signature tests. The CLI’s `symbols` target includes refs and cache robustness/upgrades. Terminal targets require `terminal-tests`, enabled in the live recipe and CLI test metadata.
- Independent isolated-CLI sweeps: nullable callee wrappers and negative/control forms; all eight default-capable language readers with before/after/both-comment controls; modifier/marker forms in all 16 languages; default/callers/callees JSON and text using IDs, and actual warm-hit statistics. Six primary-fixture copies reproduce the three findings.
- Independent public cache API: seven representative sources (Swift/C# calls, C#/PHP/Scala defaults, Java/Lua markers), each traversing narrow Miss, narrow Hit, semantic widening with `parsed: false`, kind widening, complete Hit and projected Hit. Observed wrong source facts persist; independently stated expected results above determine correctness.
- `swiftc -typecheck` accepted the force-unwrapped-call source. Pinned Swift/C# invocation and C# comment-default trees are error-free. C++ parenthesized `new (C)` probes pass clang but the pinned parser reports missing type nodes; those were **excluded from actionable invocation findings**, consistent with the accepted grammar boundary.

No scratch review probes were added to permanent targets or counted as regression coverage. No implementation was edited, no formatting command or commit ran, and no cross-OS evidence was used to decide readiness.

## Input Robustness Matrix

The feature’s file-format reader is the persistent JSON cache. This repair cycle did not modify that reader or add another format. Its load-bearing reference-evidence columns were reviewed and the permanent public-CLI mutation tests were rerun in the full L1 suite: `reference_evidence_is_served_only_when_consistent`, reference/span-leaf matrices, all-language identity/source-fact tests and list edits. The entry/record coverage columns also ran; their current contract is in [cache-design.md](../../docs/cache-design.md). No new source/configuration parser shape is introduced by this review.

Each permanent matrix restores a tool-written control entry before one field edit and checks the served public result and hit/miss outcome. O is an owned Scoped helper call, N a NameOnly member call, Q an unresolved qualified call, R a limited region. H means a real Hit; M means a miss and recomputation of the independently expected control; — means no element-type shape applies. Empty/null expected hits below are legitimate unqualified nullable controls, not permissive absent-field defaults. Duplicate-key edits duplicate the key inside its object; trailing-content edits append invalid JSON to the control document.

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

No additional performance gate, resolver breadth or architectural refactor is requested. Preserve indexed lookup, one definition/occurrence pipeline and record-only semantic widening. The Swift specialized-initializer grammar limitation is context-sensitive: describe it that way in future grammar maintenance, and retain its existing conservative coverage. The newly probed C++ parenthesized-allocation limitation can be recorded alongside other grammar limits without implying the source is invalid. Neither limitation changes the accepted pinned-grammar release contract.
