---
$schema: feature-review.yaml
ready: false
findings:
  - title: R3-F1 — TypeScript instantiation expressions hide calls and constructor invocations
    priority: high
  - title: R3-F2 — Rust associated value calls falsely prove absence of recursion
    priority: high
  - title: R3-F3 — Cache validation accepts false source facts and misordered or duplicate lists
    priority: medium
  - title: R3-F4 — Optional and tagged eval calls incorrectly hide lexical declarations
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-06T16:37:36-07:00
spec: 2026-10-08-scoped-references/spec.md
implemented: false
implemented_by: claude/opus
log: tree-hugger/features/2026-10-08-scoped-references/log.md
description: "A **feature** review of `2026-10-08-scoped-references/spec.md`"
feature: 2026-10-08-scoped-references/review-3.md
previous: 2026-10-08-scoped-references/review-2.md
next: 2026-10-08-scoped-references/review-4.md
---

# Scoped References: review 3

The feature is **not production ready**. The required examples from both earlier reviews are repaired, including their previously identified consumers. Four newly discovered findings remain: missing TypeScript calls, false Rust recursion certainty, inconsistent cached evidence, and lost lexical bindings around two forms of `eval`. Passing the earlier examples did not close these defect classes.

This review checked the specification, plan, original implementation log, both earlier reviews, and both repair cycles in [the repair log](./log.md). Implementation completion flags were treated as claims. Only this review, the previous review's link, and the specification's iteration count were edited; existing implementation changes were preserved.

## Previous Finding Verification

The earlier reviews assigned no finding or child IDs. Their stable identifiers below are the review filename plus the exact finding title; neither historical review was rewritten to invent IDs. **Verified fixed** refers to the previously required instances and consumers, not every possible future instance of that class. Newly discovered siblings have separate IDs below.

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
|---|---|---|---|---|
| `review-1.md — Lexical lookup crosses boundaries that do not establish a binding` | Rust inline modules, nested functions/constants/statics, legal closure capture, `if let`/chains/else, `while let`/`for`; TS/JS `with`, direct `eval`, parameter-default controls; occurrences, summaries, recursion, declared-base lookup and CLI reports | **verified fixed** | `LEXICAL_BOUNDARY_MATRIX`, `boundary_crossing_names_make_no_summary_edges`, boundary CLI tests and relation tests passed. Rust item/module boundaries and conditional end positions are enforced in the shared scope tree. Parameter/default siblings required by review 2 were also checked, as described below. | New optional/tagged `eval` siblings are R3-F4. No old reproduction remains failing. |
| `review-1.md — Named function expressions bind their names to outer declarations` | TS/JS ordinary and generator expressions, declaration-token exclusion, expression-local calls/defaults, final outer calls; unnamed/class/arrow controls; placement, summaries, recursion, relation lookup and CLI callers | **verified fixed** | `EXPRESSION_NAME_MATRIX`, expression-name summary tests, relation rows, recursion matrix and `named_function_expressions_never_call_the_outer_declaration` passed. `names_itself` keeps generator names inside their expression; binding names are excluded from occurrences. | None for the required instances. |
| `review-1.md — Invocation classification loses calls across supported source forms` | Direct/wrapped calls in all 16 languages; TS assertion/non-null forms; Perl and shell calls; generic/member captures, nested wrappers, receiver/argument/sequence/conditional/dereference negatives; callers/callees, recursion, terminal-call detection and CommonJS evidence | **verified fixed** | All-language invocation and negative matrices, wrapped binding/coverage tests and CLI wrapper/command tests passed. The shared wrapper traversal and code-side member captures cover the enumerated forms. The accepted PHP exception remains `Usage`: `(f)()` names a constant value. The Go indexing/generic grammar limitation remains documented. The terminal-call and CommonJS consumers use their existing corrected wrapper readers. | New TS instantiation-expression siblings are R3-F1. No accepted PHP/Go decision is reopened. |
| `review-1.md — Cache validation accepts inconsistent reference evidence` | Retained wrong/malformed/lost target IDs, ownerless targets, candidate/owner identity, duplicate/omitted callable regions, span/confidence/capability shape matrix, summary edges/dependencies/semantics, legitimate nullable and narrowed controls | **verified fixed** | Public `reference_evidence_is_served_only_when_consistent` and structural matrix tests passed; `stored_evidence_is_checked_against_what_the_entry_retains` passed. Two-way edge validation and semantic recomputation are present. Required-region and absent-ID siblings from review 2 passed their all-language matrix. | New source-geometry, written-path and list-order/duplication siblings are R3-F3. Accepted limitations concerning undetectable deletion of anonymous evidence and containment/base-target validation remain outside this repair inventory. |
| `review-2.md — Parameter defaults incorrectly see body-local var bindings` | All seven callable forms in both TS and JS; ordinary/nested-block/destructured/loop `var`, parameter visibility/redeclaration/hoisting, body `eval`; `let`/`const`/body-function controls; shared declared-base lookup, public occurrences, incoming/outgoing CLI rows and parameter documentation | **verified fixed** | `PARAMETER_DEFAULT_FORMS × BODY_DECLARATIONS`, `PARAMETER_ENVIRONMENT_MATRIX`, relation rows and `parameter_defaults_call_the_outer_declaration_not_a_body_var` passed. `var_body` implements parameter-expression separation and shared `placement` uses the body's scope. Anonymous expressions have no fabricated declaration owner; generator declarations remain unextracted rather than gaining a false owner. | None for the required instances. |
| `review-2.md — Pattern capitalization invents or omits Rust bindings` | Capitalized bare/`mut`/`ref` names in conditionals, loops and arms; lowercase constants in patterns/body/guards; uppercase/path/tuple/shorthand/irrefutable controls; walker, captured-record fallback, declaration exclusion, summaries, recursion, CLI callers and documentation | **verified fixed** | `PATTERN_IDENTITY_MATRIX`, `PATTERN_RECORD_MATRIX`, recursion rows and both Rust-pattern CLI tests passed. `place_sites` is shared by extraction and occurrence binding; constant reads and unknown refutable names cannot reappear through the record fallback. The one definitions result feeds records, counts, metrics and experimental diagnostics. Related Scala spelling/binder rows passed. | None for the required instances. The documented import/glob and record-less binding limits remain as recorded; this review does not require compiler-backed import resolution. |
| `review-2.md — Transparent call wrappers incorrectly limit coverage` | Rust parentheses/turbofish/type paths; TS/JS parentheses, every listed TS assertion and non-null wrapper, constructors; direct/self/member/value/sequence/conditional/dereference controls; coverage, direct-eval detection, recursion and shared relation scopes | **verified fixed** | `CALLEE_COVERAGE_MATRIX` and complete/narrowed `RECURSION_MATRIX` passed. `invoked_operand` now supplies call/new coverage and `dynamic`; wrapped calls to another known function give `No`, and wrapped self-calls remain `Yes`. Parenthesized direct `eval` was corrected during this repair, before verification. | R3-F1 adds an omitted generic operand shape. R3-F4 adds call-mode decisions outside the earlier wrapper rows. R3-F2 is a distinct missing callable/value decision. |
| `review-2.md — Cache validation accepts nonexistent evidence identities and missing callable regions` | All 16 languages' phantom coverage owners; applicable phantom candidates and owned/ownerless established targets; empty Python/Lua regions; retained target/owner/candidate/summary IDs; narrow entries, nullable imports/unreported bindings and bodiless controls | **verified fixed** | `every_kind_evidence_identities_name_retained_declarations_in_every_language` and retained-evidence tests passed. Absent identities now require an omitted compatible kind. Callable regions are required exactly when `body_span` exists, including empty/comment-only Lua bodies; a bodiless method cannot own one. Revision-40 rejection passed. | R3-F3 concerns retained regions with wrong geometry, source paths, and canonical lists; it does not repeat the repaired phantom-ID or missing-region reproduction. |

Neither earlier review had blocked findings. The first implementation log's accepted release breadth (Rust/TS/JS resolution, conservative evidence elsewhere), file-local boundary and receiver limits preceded the repairs and remain in force. No blocker had to be removed before or after these repairs. The log's separate containment/declared-base cache deferral is not an unresolved required scoped-reference repair.

## Unblocked Findings

### R3-F1 — High — TypeScript instantiation expressions hide calls and constructor invocations

**Defect class:** generic operands are recognized through only selected grammar shapes, so type-only instantiation expressions break invocation classification, member collection and call completeness.

In the tree-hugger library, [generic_operand](../../lib/src/analysis/occurrences.rs:1246), which identifies the function part of generic syntax, has Rust/Scala/C++/C# cases but no TypeScript instantiation-expression case. [wrapped_member_callees](../../lib/src/analysis/occurrences.rs:1176), which supplements the reference query with wrapped member names, also stops before this operand. Merely sharing a helper between classifiers does not make its rule table complete.

```typescript
function f<T>() { (f<number>)(); }
function g() { (f<number>)(); }
```

`hug symbols --json-full --no-cache a.ts` reports both names as `Usage`, with established IDs, limited coverage and `Unknown` recursion. The first is a direct self-call and must be `Call`/`Yes`; the second calls a different established function and must have complete coverage and `No`. `hug refs f --callers --json a.ts` and `hug refs g --callees --json a.ts` both return no rows.

Each row below was run through the shipped CLI. Unless stated otherwise, use `function f<T>() {} function g() { EXPRESSION; }`; a positive direct-generic control proves the target is available. Type-only wrappers do not compute a different function value.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R3-F1.01 / TS generic operand | `(f<number>)()` | Established `Usage`; limited / `Unknown` | Established `Call`; complete / `No` for `g` | Full-index CLI |
| R3-F1.02 / nested parentheses | `((f<number>))()` | Same failure | Same call and completeness | Full-index CLI |
| R3-F1.03 / non-null wrapper | `(f<number>)!()` | Same failure | Same call and completeness | Full-index CLI |
| R3-F1.04 / `as` wrapper | `(f<number> as ()=>void)()` | Same failure | Same call and completeness | Full-index CLI |
| R3-F1.05 / `satisfies` wrapper | `(f<number> satisfies ()=>void)()` | Same failure | Same call and completeness | Full-index CLI |
| R3-F1.06 / type assertion | `(<()=>void>f<number>)()` | Same failure | Same call and completeness | Full-index CLI |
| R3-F1.07 / generic member collection | `class W { m<T>() {} }`; `(obj.m<number>)()` in `g(obj:W)` | `m` is absent altogether | One `Call`, `NameOnly`, candidate `W.m`; receiver remains a usage, coverage remains limited | Full index and empty callers/callees CLI reports |
| R3-F1.08 / constructor operand | `class C<T> {}`; `new (C<number>)()` | Established `Usage`; limited / `Unknown` | Established `Call` of `C`; same complete coverage as direct `new C<number>()` | Full index and empty callers/callees CLI reports |
| R3-F1.09 / self-call consumer | First function in the minimal source | `Unknown` | `Yes` | Full-index semantics |
| R3-F1.10 / direct-generic controls | `f<number>()`, `obj.m<number>()`, `new C<number>()` | Correct calls; member uncertain; other two complete | Preserve these results | Full-index CLI |
| R3-F1.11 / non-call controls | `const x=f<number>`; `consume(f<number>)` | `f` is `Usage` | Preserve `Usage`; only `consume` is a call | Full-index CLI |
| R3-F1.12 / Rust reader | `(f::<u8>)()` | Established `Call`, complete, `g: No` | Clean | Full-index CLI |
| R3-F1.13 / C++ reader | `(f<int>)()` | `Call`, conservative candidate, unsupported-language coverage | Clean under accepted limited-resolution contract | Full-index CLI |
| R3-F1.14 / C# reader | `(f<int>)()` | `Call`, conservative candidate, unsupported-language coverage | Clean under accepted limited-resolution contract | Full-index CLI |
| R3-F1.15 / Scala reader | `(f[Int])()` | `Call`, conservative candidate, unsupported-language coverage | Clean under accepted limited-resolution contract | Full-index CLI |
| R3-F1.16 / Go reader | `(f[int])()` | `Usage`, unsupported-language coverage | Clean under accepted indexing/generic grammar limitation | Full-index CLI; existing invocation matrix |
| R3-F1.17 / JS reader | `((f))()` | Established `Call`, complete / `No` | Clean; no type-instantiation syntax | Full-index CLI |
| R3-F1.18 / remaining language readers | Java, Python, C, Swift, PHP, Perl, Bash, Zsh, Lua | Existing direct/wrapped invocation rows pass | Preserve language-specific calls and explicit unsupported coverage | All-language invocation matrix; Java uses `method_invocation.name`, independently of type arguments |

The decision and consumer sweep is:

| Child ID | Decision/consumer | Required repair or clean invariant |
|---|---|---|
| R3-F1.C1 | Query captures and code-side member collection | Read the instantiation's operand while collecting wrapped members; otherwise the classifier never sees `m`. |
| R3-F1.C2 | `is_call_target` and `invoked_operand` | Add the TS grammar form to the shared operand rules, at arbitrary wrapper depth. Preserve the non-call controls. |
| R3-F1.C3 | Binding and candidate selection | The plain `f`/`C` IDs are already correct. A receiver call must remain uncertain with the correct member candidates. |
| R3-F1.C4 | Reference summaries | Plain generic usages already create summary edges; those edges do not prove the call classifier is correct. Assert call sites separately; member candidates remain outside summaries. |
| R3-F1.C5 | Coverage and recursion | Restore complete coverage for established function/class operands, `Yes` for a self-call and `No` for a call to another function. Keep receiver calls limited. |
| R3-F1.C6 | Kind/name filtering, projection and cache widening | Analysis precedes declaration filtering; projection preserves whole-file occurrences. Pin expected calls through narrow queries and warm/widened entries, rather than checking equality alone. |
| R3-F1.C7 | CLI selection and callers/callees | Both directions filter on `Call`; assert required established rows and separate member suggestions, with no extra calls for value/argument controls. |

**Required repair:** implement and test every defective instance and consumer above at Level 1; update invocation documentation and the extractor revision. The repair log's statement that `(f<u8>)()` remains unclassified is a known implementation limit, not an accepted exception to the syntactic invocation contract. No receiver inference or wider language resolution is needed.

### R3-F2 — High — Rust associated value calls falsely prove absence of recursion

**Defect class:** establishing a qualified value's binding is mistaken for proving that invoking it calls a callable declaration, so indirect calls can receive complete coverage and a false negative recursion verdict.

The tree-hugger library's [resolve_rust_path](../../lib/src/analysis/occurrences.rs:634), which resolves `Self::name` and local type paths, sets `indirect: false` for every unique member, including constants. Its uncertain fallback also loses the value-call limitation. [visit_rust](../../lib/src/analysis/occurrences.rs:277) accepts the path's syntax as direct, and [direct_recursion](../../lib/src/analysis/mod.rs:502), which computes a callable's recursion verdict, consequently returns `No`.

```rust
struct S;
impl S {
    const C: fn() = Self::f;
    fn f() { Self::C(); }
}
```

This compiles with `rustc --crate-type lib --edition 2024 --emit=metadata`. The CLI reports `C` as a `Scoped` call to the constant's ID, the method's coverage as `Complete`, and `f.is_recursive` as `No`. `C` holds a function pointer; this feature deliberately does not follow values to their initializer. The correct verdict is **Unknown**, with `indirect_dispatch`, while retaining the call's binding to `C`. Reporting `Yes` by following the initializer would also violate the contract.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R3-F2.01 / inherent self path | `Self::C()` in the minimal source | `Scoped` to constant; complete / `No` | Same target; limited / `Unknown` | Full-index and callees CLI; rustc accepted |
| R3-F2.02 / explicit type path | `S::C()` in a nongeneric impl | Same failure | Same target; limited / `Unknown` | Full-index CLI |
| R3-F2.03 / wrapped self path | `(Self::C)()`, `((Self::C))()` | Same failure | Same value-call limitation through wrappers | Full-index CLI; nongeneric source compiled |
| R3-F2.04 / wrapped type path | `(S::C)()` | Same failure | Same value-call limitation | Full-index CLI; rustc accepted |
| R3-F2.05 / generic impl | `Self::C()` inside `impl<T> S<T>` | Same failure | Limited / `Unknown` | Full-index CLI; rustc accepted |
| R3-F2.06 / generic type qualifier | `S::<u8>::C()` | Candidate-only constant, but complete / `No` | Preserve conservative binding; indirect value call remains limited / `Unknown` | Full-index CLI |
| R3-F2.07 / enum inherent member | `impl E { const C:fn()=Self::f; fn f(){Self::C();} }` | Same false `No` | Limited / `Unknown` | Full-index CLI; rustc accepted |
| R3-F2.08 / trait self path | `trait T { const C:fn(); fn f(){Self::C();} }`; an impl supplies `Self::f` | Candidate-only constant; complete / `No` | Preserve uncertain binding; limited / `Unknown` | Full-index CLI; rustc accepted |
| R3-F2.09 / trait explicit type path | `S::C()` in that trait method | Same failure | Same limitation | Full-index CLI; rustc accepted |
| R3-F2.10 / unrelated constant initializer | Inherent `C:fn()=h`, then `Self::C()` inside `f` | Complete / `No` | `Unknown`; value inference is outside this feature even when the initializer appears unrelated | Full-index CLI; rustc accepted |
| R3-F2.11 / top-level values | `const C:fn()=f; fn f(){C();}` and `static C:fn()=f` | Limited / `Unknown` | Clean | Full-index CLI |
| R3-F2.12 / known member functions | `Self::h()` in `f`; `Self::f()` in `f` | Complete / `No`; proven self-call / `Yes` | Clean; preserve both | Full-index CLI; rustc accepted |
| R3-F2.13 / constructor | `enum E { V(u8) }`; `E::V(1)` in `f` | Known constructor call, complete / `No` | Clean; constructing a variant is not a function-pointer invocation | Full-index CLI; rustc accepted |
| R3-F2.14 / value-only usage | `let _p=Self::C` in `f` | `Usage`, complete / `No` | Clean; do not limit a region merely for reading a value | Full-index CLI; rustc accepted |
| R3-F2.15 / ECMA value readers | TS/JS `let`/`var` callbacks and `const cb=f`; direct `const cb=()=>{}` control | Value calls limited; directly declared const function expression complete | Preserve the accepted callable rules | Full-index CLI in both languages |
| R3-F2.16 / other 13 languages | Function/value/member occurrences | Every region has unsupported-language coverage; recursion cannot be `No` | Clean conservative invariant | Passing all-language evidence tests; no supported path resolver runs |

**Required repair:** distinguish callable declarations and constructors from value bindings on both the unique-member and candidate/fallback paths. Carry this decision into occurrence coverage before summaries and semantics consume it. Add Level-1 tests for R3-F2.01–10, and preserve R3-F2.11–16. Consumer IDs are **R3-F2.C1** (path/category lookup and candidates), **C2** (occurrence coverage and unchanged constant summary edges), **C3** (complete/narrowed recursion), **C4** (stored-occurrence semantic widening), and **C5** (CLI callees coverage). Document qualified function-pointer calls as indirect; never manufacture a reference to the initializer's function.

### R3-F3 — Medium — Cache validation accepts false source facts and misordered or duplicate lists

**Defect class:** stored evidence is checked for types, identities and membership, but source-derived geometry and written paths, candidate ordering, and summary ordering/deduplication are not enforced.

In the tree-hugger library, [validate_evidence](../../lib/src/cache/stored_evidence.rs:233), which decides whether persistent reference evidence may be served, requires a retained callable to own a region when it has a body. It does not require that region to contain the retained body. Target `qualified_name` is not checked against the source at all. Candidate IDs are checked as a set, while summary edges are checked individually; neither preserves the required list order, and summaries may contain duplicates.

Three independent mutations were applied to tool-written entries, restoring the original entry before every edit:

1. Replace a callable region containing no occurrences with its record's `name_span`. The unchanged `body_span` lies outside the new region.
2. Set one occurrence's `target.qualified_name` to `__never_written_review3__.<name>`, which is absent from the source. Its name, ID, span, confidence and summaries stay unchanged.
3. Reverse two genuine `NameOnly` candidates. The source positions and IDs of both retained declarations remain available to establish the required order.

Each unedited control was an actual hit with zero parses and agreed with uncached output. **Every mutated row below was also a hit with zero parses and changed public output; every mutation should instead miss and reproduce the independently expected original result.**

Each language row has three required child IDs: append `.region`, `.path`, or `.candidates` to the row ID. Primary fixtures were copied from `lib/tests/fixtures/symbols/`. Scala's region case appends `object CoverageControl { def empty() = {} }`, because its primary fixture has no named region with zero occurrences. Candidate cases add two same-named members/functions/prototypes and one reference; Lua's two function definitions are inserted before its final `return Widget`.

| Child ID / language | Region owner tested | Invented path target/confidence | Reversed candidates | Verification evidence |
|---|---|---|---|---|
| R3-F3.L01 / Rust | `with_receiver` | `parameter_value`, Scoped | Two inherent members | Three public full-index cache mutations |
| R3-F3.L02 / TypeScript | `withReceiver` | `parameterValue`, Scoped | Two class members | Same |
| R3-F3.L03 / JavaScript | `variadic` | `parameterValue`, Scoped | Two class members | Same |
| R3-F3.L04 / Go | `TestHelper` | `int`, Unresolved | Two receiver members | Same |
| R3-F3.L05 / Python | `with_receiver` | `typing`, Unresolved | Two class members | Same |
| R3-F3.L06 / Java | `variadic` | `com`, Unresolved | Two overloaded methods | Same |
| R3-F3.L07 / C# | `TestHelper` | `Example`, Unresolved | Two overloaded methods | Same |
| R3-F3.L08 / C | `test_helper` | `parameter_value`, NameOnly | Two prototypes | Same |
| R3-F3.L09 / C++ | `variadic` | `AccessWidget`, NameOnly | Two prototypes | Same |
| R3-F3.L10 / Swift | `testHelper` | `Int`, Unresolved | Two overloaded functions | Same |
| R3-F3.L11 / Scala | Appended `empty` | `example`, Unresolved | Two overloaded methods | Same |
| R3-F3.L12 / PHP | `test_helper` | `Example`, Unresolved | Two class methods | Same |
| R3-F3.L13 / Perl | `test_helper` | `strict`, Unresolved | Two package subs | Same |
| R3-F3.L14 / Bash | `outer_function` | `1`, Unresolved | Two function definitions | Same |
| R3-F3.L15 / Zsh | `outer_function` | `print`, Unresolved | Two function definitions | Same |
| R3-F3.L16 / Lua | `test_helper` | `parameterValue`, NameOnly | Two function definitions | Same |

The region defect is provable from retained `body_span`, not guessed syntax. **Do not require equality with `declaration_span`:** C/C++ records can retain just the declarator span while their region includes the body. Their body span still has to be contained. For paths, [written_path](../../lib/src/analysis/occurrences.rs:1044) always copies source text ending at the occurrence's end byte; a non-null invented path can be rejected without parsing. A present empty string is never a written qualifier.

The ordering and consumer sweep found these additional required instances:

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R3-F3.S01 / `relations.references` | Duplicate first edge; separately reverse edges from `g` calling `f`, `h`, imported `a`, imported `b` | Both served | Reject duplicate targets and non-first-occurrence order | CLI mutations in Rust, TS and JS |
| R3-F3.S02 / `relations.referenced_by` | Duplicate first owner; separately reverse owners `g`, `j` of calls to `f` | Both served | Reject duplicate owners and wrong occurrence order | CLI mutations in Rust, TS and JS |
| R3-F3.S03 / `relations.dependencies` | Duplicate first import; separately reverse imported `a`, `b` | Both served | Reject duplicates and wrong occurrence order | CLI mutations in Rust, TS and JS |
| R3-F3.S04 / candidate duplication, control | Duplicate an existing candidate | Recomputed | Clean: distinct IDs are already enforced | CLI mutations in Rust, TS and JS |
| R3-F3.S05 / occurrence ordering/duplication, control | Reverse occurrences; separately duplicate one | Recomputed | Clean | CLI mutations in Rust, TS and JS |
| R3-F3.S06 / coverage ordering/duplication, control | Reverse regions; separately duplicate one | Recomputed | Clean | CLI mutations in Rust, TS and JS |
| R3-F3.S07 / reason ordering/duplication, control | Reverse two reasons; separately duplicate one | Recomputed | Clean | CLI mutations in Rust, TS and JS |
| R3-F3.S08 / other-language summaries, control | Languages without resolution rules | No summary edge can have a supporting Scoped occurrence | Keep summaries empty; reject fabricated additions | `resolves == false` rejects Scoped; every summary edge must satisfy `made`, which therefore has no supporting tuple; limited-language summary tests passed |
| R3-F3.S09 / required written qualifier | Set the qualified member target's `qualified_name` to null | Served; `box.size` becomes merely `size` | Recompute rather than lose a required written qualifier; keep unqualified null controls valid | Public structural matrix below |
| R3-F3.S10 / empty written path | Set qualified and unqualified targets' `qualified_name` to an empty string | Both served | Recompute; the collector never emits a present empty path | Public structural matrix below |
| R3-F3.C1 / full-index and query projection | All mutations above | Wrong source facts/lists exposed in JSON | Recompute; preserve correct whole-file evidence through kind/name projection | Full-index CLI and shared `coverage::project` invariant |
| R3-F3.C2 / reference text/JSON | `struct S; impl S { fn h(){} fn f(){Self::h();} }`; mutate only the call's path to `Other::h` | Warm `hug refs --id <f-id> --callees --verbose` prints `Call Other::h`, although source says `Self::h` | Original written path in both formats | Actual hit, zero parses, text reproduction |
| R3-F3.C3 / callable coverage and semantic widening | Mutated retained region | Retained body is outside its purported owner region; the region is reused by record-only consumers | Enforce retained body geometry before reuse | Public mutation; shared stored-occurrence semantic widening reader |
| R3-F3.C4 / legitimate narrowed/nullable controls | Genuinely omitted owners/targets, imports, unreported bindings and unqualified null paths | Existing narrowed controls pass | Preserve these; validate only facts retained or provable from source | Retained-evidence and projection tests passed |

For R3-F3.S01–07, the language-and-mutation child IDs append `.rs.duplicate`, `.rs.reverse`, `.ts.duplicate`, `.ts.reverse`, `.js.duplicate`, or `.js.reverse` to the site ID. This names each required instance independently of the shared validator.

**Required repair:** enforce the retained-body containment invariant, written paths checked against source, declaration-order candidates and occurrence-order deduplicated summaries. Cover every defective child above in public cache mutation tests and add the missing written-path column to the permanent robustness matrix. Preserve valid null paths for unqualified occurrences and omitted-record controls, while retaining enough source evidence to detect a lost required qualifier. This does not require rejecting the previously accepted, indistinguishable deletion of an entire anonymous region or validating unrelated containment/base targets.

#### Input robustness matrix

The changed format reader is the persistent **JSON cache**. There are no manifest, lockfile, YAML or TOML readers added by this feature. The existing structural/evidence tests passed. This review additionally walked **50 field/control columns and 308 shape edits** through the public CLI, from one tool-written positive fixture, restoring the original before each edit. The tables below show all those columns in groups to keep them readable.

The fixture was:

```typescript
function helper() {}
function again() { again(); }
class Box { size() {} }
function caller(box: Box) { box.size(); missing(); }
helper();
import { external } from "./external";
function useit() { external(); }
```

In the tables, **O** means its owned Scoped self-call (`occurrences[0]`), **N** the qualified member occurrence (`occurrences[3]`), **R** the named limited `caller` region (`reference_coverage[4]`). `M` is a miss with recomputation to the original result; `H` is a hit. `—` means an element shape does not apply to that field. Duplicate means a duplicate JSON key, rather than a repeated list element. Each trailing-content cell appends garbage to a restored valid document. A zero number is the empty numeric mutation; point coordinates and their byte offsets must still agree.

##### Lists and occurrence fields

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

##### Reference names and IDs

| Shape | `O.owner.id` | `O.owner.name` | `O.owner.qualified_name` | `O.target.id` | `O.target.name` | `O.target.qualified_name` | `N.candidates.0.id` | `N.candidates.0.name` | `N.candidates.0.qualified_name` | `N.target.qualified_name` |
|---|---|---|---|---|---|---|---|---|---|---|
| Unedited control | H | H | H | H | H | H | H | H | H | H |
| Absent | M | M | M | M | M | M | M | M | M | M |
| Explicit null | M | M | M | M | M | H | M | M | M | H |
| Wrong whole type | M | M | M | M | M | M | M | M | M | M |
| One wrong element | — | — | — | — | — | — | — | — | — | — |
| Every wrong element | — | — | — | — | — | — | — | — | — | — |
| Empty | M | M | M | M | M | H | M | M | M | H |
| Duplicate key | M | M | M | M | M | M | M | M | M | M |
| Trailing/invalid content | M | M | M | M | M | M | M | M | M | M |

##### Coverage fields

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

##### Occurrence span

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

##### Owner span

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

##### Coverage span

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

The empty `O.candidates` hit and null unqualified `O.target.qualified_name` hit are valid controls. The empty-string hits in **both target qualified-name columns are defects**; a written path is nonempty source text. The null hit for the originally qualified member also loses required written evidence. Missing keys still reject correctly, so absence and null are not being conflated by deserialization. These cases belong to R3-F3, together with its well-typed geometry and canonical-list mutations; no robustness defect is left outside the findings inventory.

The reader's nullable fields use an explicit `nullable` deserializer, and its stored structs have required keys and `deny_unknown_fields`. The searched `filter_map`, `unwrap_or_default` and `.ok()` sites do not silently filter malformed JSON collections: they operate on already typed evidence, optional references or checked numeric conversion. The structural matrix is therefore largely clean; its target-path exceptions and semantic invariants remain required repairs. The existing public test matrix should absorb the newly exercised leaf fields and failing cells so these proofs become regression coverage.

### R3-F4 — Medium — Optional and tagged eval calls incorrectly hide lexical declarations

**Defect class:** a direct-evaluation decision is made from the callee's spelling without checking how it is invoked, so calls that cannot introduce local bindings still stop lexical lookup.

In the tree-hugger library, [ECMA dynamic](../../lib/src/analysis/scope/ecma.rs:63), which decides whether lookup may be affected by direct evaluation, sees every `call_expression` whose unwrapped function is named `eval` as direct evaluation. [visit_ecma](../../lib/src/analysis/occurrences.rs:299), which assigns coverage limitations, repeats that spelling-only decision. Both optional calls and tagged-template calls use this node kind. The former performs indirect evaluation; the latter passes a template array rather than source text and does not evaluate code.

```javascript
class Base {}
function outer() {
    function f() {
        eval?.("var f = 1");
        class D extends Base {}
        f();
    }
}
```

Substitute ``eval`var f = 1` `` for the optional call for the second failure. The CLI loses the established self-call (`Unresolved` in this nested example), reports `Unknown` recursion, and omits `Base`'s ID from `D.relations.extends`. `hug refs --id <f-id> --callers --json a.js` returns no established row. Neither form can introduce a body-local `f` or `Base`; both must preserve the lexical IDs and the proven `Yes` self-call. This changes required results, not merely how much confidence text is printed.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R3-F4.01 / optional call | `eval?.("...")`, TS and JS | Function marked dynamic; lexical self-call loses ID; `Unknown` | No direct-eval barrier; established self-call, `Yes` | Full-index and callers CLI |
| R3-F4.02 / wrapped optional call | `(eval)?.("...")`, TS and JS | Same barrier and `Unknown` | Same established lexical self-call | Full-index CLI |
| R3-F4.03 / tagged call | ``eval`...` ``, TS and JS | Same barrier and `Unknown` | No direct evaluation; preserve lexical IDs and `Yes` | Full-index and callers CLI |
| R3-F4.04 / wrapped tagged call | ``(eval)`...` ``, TS and JS | Same failure | Same preservation | Full-index CLI |
| R3-F4.05 / true direct controls | `eval("...")`, `((eval))("...")`, TS and JS | Uncertain lexical lookup, limited / `Unknown` | Clean under the accepted conservative direct-eval rule | Full-index CLI; wrapped-eval regression matrix |
| R3-F4.06 / indirect sequence control | `(0,eval)("...")`, TS and JS | Lexical self-call remains established; `Yes`; expression call may remain limited | Clean | Full-index and callers CLI |
| R3-F4.07 / member and alias controls | `globalThis.eval("...")`; `const e=eval; e("...")`, TS and JS | Self-call remains established, `Yes` | Clean; neither is direct eval | Full-index CLI |
| R3-F4.C1 / scope placement and lookup stopping | Optional/tagged forms in a nested named function | Dynamic function scope ends lookup before parent declarations | Preserve normal lookup; do not install the dynamic source | Shared `dynamic` → scope placement → lookup; public missing IDs |
| R3-F4.C2 / declared-base lookup | `class D extends Base {}` in that function | Text-only base, no `Base` ID | Established base ID | Public full index in both languages; relation reader shares `dynamic` |
| R3-F4.C3 / summaries and candidates | Local self-call after optional/tagged eval | No local self target/edge; top-level counterpart is merely a suggestion | Established self target and summaries; no fabricated extra candidates | Nested and top-level full-index reproductions |
| R3-F4.C4 / coverage and recursion | Same sources | Spurious unsupported-binding limit and loss of `Yes` | No direct-eval limitation; preserve proven self-call even if another invocation remains conservatively limited | Full-index semantics |
| R3-F4.C5 / filtering, cache and CLI | Both report directions and stored evidence | Callers drops required established rows; caches/projection preserve the incorrect result | Fix analysis before filtering/storage; assert exact IDs and rows on fresh and warm/narrow results | CLI callers reproduction; projection/store invariant |

A separate Node execution independently checked the call-mode distinction: with a function-local `let marker=1`, direct and parenthesized `eval("marker=2")` return `2`; optional evaluation and tagged evaluation return `1`. This verifies why those forms cannot introduce bindings into the enclosing function. The existing decision to treat genuine direct eval conservatively, including strict code, is unchanged.

**Required repair:** share a direct-eval predicate that includes invocation mode between binding and coverage readers; preserve transparent TS wrappers while excluding optional and tagged evaluation from the local-binding barrier. Add Level-1 rows for every defective form in both languages, including relation IDs, summary edges, recursion and established CLI callers. Correct the direct-eval documentation to state this distinction. This is a newly discovered sibling, not a failing rerun of the earlier parenthesized-direct-eval repair.

## Blocked Findings

None. All four findings can be repaired within the accepted file-local design. Human review is not required to complete this repair inventory.

## Recurrence

- **R3-F1** repeats the classes of `review-1.md — Invocation classification loses calls across supported source forms` and `review-2.md — Transparent call wrappers incorrectly limit coverage`. It is a **newly discovered sibling**: TS instantiation operands were omitted from the generic rule table and member collector. The prior explicit wrapper cases pass; the remaining missing decisions are collection, invocation, coverage, recursion and both CLI call filters.
- **R3-F3** repeats the semantic-consistency class of both earlier cache findings. These are **newly discovered siblings**, not phantom-ID or missing-region regressions: body containment, written paths, candidate order, and summary order/duplication remain unchecked. All 16 language rows and all three summary consumers are now enumerated.
- **R3-F4** repeats the binding-boundary class of `review-1.md — Lexical lookup crosses boundaries that do not establish a binding`, and the direct-eval consumer swept during the iteration-2 wrapper repair. It is a **newly discovered sibling**: optional/tagged call mode was missed in both binding and coverage predicates, and the shared declared-base reader inherited the barrier.
- **R3-F2** is a newly discovered callable-versus-value defect in qualified Rust resolution. It is not a regression of the earlier wrapper cases or a request for value inference.

No unchanged earlier failing reproduction or repair-introduced regression was demonstrated. `recurrence: true` records the three repeated classes above. The original named-expression, parameter-default and Rust-pattern required instances remain repaired.

## Requirement Verification and Test Levels

| User-facing requirement | Strongest verification inspected/run | Assessment |
|---|---|---|
| Language visibility, order/hoisting, parameters, patterns, aliases and type/value categories | Level 1: scope matrices, declared-relation tests, public full-index reproductions | Earlier required repairs pass; R3-F4 loses established lexical results. |
| Invocation classification, member captures, constructor calls, declaration exclusions, ownership and repeated spans | Level 1: all-language invocation/ownership tests and shipped CLI | R3-F1 omits required call results. |
| Honest direct recursion and indirect-call uncertainty | Level 1: complete/narrowed recursion matrix and compiled Rust reproductions | R3-F1 and R3-F2 block correctness; R3-F4 also suppresses proven self-calls. |
| Established summaries, whole-file evidence, filtering, projection, cache hits/widening, version rejection and skipped work | Level 1: reference-summary, query, cache and cache-upgrade tests, plus cache mutation sweeps with independently stated expected results | Existing equivalence/work tests pass. R3-F3 shows why equivalence alone is not enough. |
| Exact/smart-case and ID selection, ambiguity, local targets, empty/conflicting reports, file-local boundary and versioned JSON | Level 1: public `refs` tests and new CLI reproductions | Wiring tests pass; missing/misreported rows are carried under their analysis/cache findings. |
| Terminal headings, grouped rows, styles, hyperlinks and wrapping | Level 2: `check_refs` in the real tmux and WezTerm declaration-report tests | Appropriate level present and passing; Kitty entry points skipped because the backend was unavailable. |
| OS keyboard encoding/input activation | No such feature requirement | Level 3 is not required. |

Commands run in the package area:

- `just test symbols_occurrences symbols_reference_summaries symbols_cache cache_robustness refs:: symbols_relations`: **188 passed**, no failures.
- `just test symbols_query symbols_coverage symbols_adapter cache_upgrade stored_evidence`: **120 passed**, no failures. One retained-evidence test also matched the earlier run's filter.
- `just test parenthesized_terminal_callees commonjs_evidence_counts_anywhere`: **2 passed**, verifying the previous invocation repair's terminal-call and CommonJS consumers.
- `just test-l2`: six entry points returned successfully; tmux and WezTerm exercised the real-terminal reports, and the two Kitty entry points skipped.
- `just check-tier-coverage tree-hugger` from the root: **zero stranded tests**.

The library `l1` target declares the reviewed modules; the CLI `symbols` target declares `refs`, cache robustness and upgrade modules. Their ordinary test names select Level 1. Real-terminal targets require `terminal-tests`, which is enabled by the live recipe and CLI CI metadata. Review experiments used isolated scratch repositories and cache/home directories; no terminal or browser focus was requested. No implementation tests were added by this review. Lint/formatting was not rerun for these review-only document edits. Cross-OS evidence and a human sign-off are not readiness blockers here.

## Nonblocking Recommendations

None separate from the required repairs. The existing indexed lookup, shared extraction result, capability projection and occurrence-based semantic widening are suitable for the feature's intended scope; no broader refactor or performance gate is requested.
