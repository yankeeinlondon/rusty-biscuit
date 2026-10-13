---
$schema: feature-review.yaml
ready: false
findings:
  - title: C# relation lookup discards generic arity before selecting the target
    priority: high
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-06T15:01:02-07:00
spec: 2026-10-06-symbol-type-relations/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-06-symbol-type-relations/log.md
description: "A **feature** review of `2026-10-06-symbol-type-relations/spec.md`"
feature: 2026-10-06-symbol-type-relations/review-11.md
previous: 2026-10-06-symbol-type-relations/review-10.md
next: 2026-10-06-symbol-type-relations/review-12.md
---

# Review of Symbol Type Relations — Iteration 11

The feature is **not production ready**. The enumerated C# partial-fragment repairs from review 10 are implemented, but arity-aware identity selection does not reach every lookup site. Ordinary C# types sharing a name with different numbers of type parameters lose provable local relations. One high finding covers the affected lookup class. No human decision is needed.

## Previous findings and blocked work

| Previous work | Assessment |
|---|---|
| Review 10: C# partial-owner merging ignores type identity | Implemented for the enumerated cases. Owner groups now require compatible declaration category, generic arity and enclosing owner identity. Registered negative arity/category/enclosing-owner cases, own-member and inherited-barrier cases, and compatible-fragment controls remain in the shared query/cache corpus. The remaining defect is selecting an arity-specific target or member before rejecting multiple name matches. |
| Review 10: Aggregate declaration-fact tests exceed the normal execution budget | Implemented: complete/selective equivalence, cache projection, cache widening, adapter and counter consumers are partitioned into statically registered groups. Path-meaning consumers and work-shape tables are partitioned too. Coverage guards check every source and shape is selected exactly once. See execution evidence below. |
| Review 10 blocked findings | None. |
| Earlier blocked mixed-base readers | Unblocked before review 3 by the author's accepted choice: precise inheritance and conformance lists plus `unclassified_bases` for uncertain categories. C#, Swift and Scala readers are implemented. Swift external/unproved extension owners remain the documented boundary. No new decision or reaffirmation is required. |
| Earlier generic defaults/requirements, attributes/modifiers, callable body lines, strict cache fields and bounded-work repairs | Inspected current readers and registered regression consumers; no separate finding reproduced. |

## Unblocked Findings

### High — C# relation lookup discards generic arity before selecting the target

**Defect class:** lookup selects candidates or stops at a nearer name binding before applying the written generic argument count that distinguishes type identities.

In the tree-hugger library, [Declarations::resolve](../../lib/src/file/tree_file/relations.rs:1663), which chooses the local declaration named by a written base, applies `arity_fits` only after `resolve_path` has produced a unique target. Its [final named lookup](../../lib/src/file/tree_file/relations.rs:1789) calls `only` on every same-spelled candidate first. Its [source-owner member branches](../../lib/src/file/tree_file/relations.rs:1747) likewise require exactly one member before checking arity. Consequently a different-arity sibling prevents the correct declaration from ever reaching that check.

The new [written_group](../../lib/src/file/tree_file/relations.rs:1126) index distinguishes intermediate named path owners, but is not used for the final named target or before selecting members of an anchored owner. [Declarations::members](../../lib/src/file/tree_file/relations.rs:1248) and its bounded candidate summary also omit arity. The [binding_scope](../../lib/src/file/tree_file/relations.rs:1560) outward search also stops at a nearer same-spelled declaration without considering arity: `class Base {} class H { class Base<T> {} class Child : Base {} }` loses its valid file-level target. Namespace and source-anchored scopes fail the same way. Filtering a truncated summary afterward would still be unsafe when three or more arities exist; index the discriminating evidence before truncation while retaining bounded work.

A single edit to a positive control reproduces the issue:

```csharp
class Base<T> {}
class Child : Base<int> {}
```

This reports `extends` with the ID of `Base<T>`. Add `class Base {}` without changing either existing declaration:

```csharp
class Base {}
class Base<T> {}
class Child : Base<int> {}
```

Now the shipped CLI reports only `unclassified_bases: [{"name":"Base<int>"}]`. Expected: the same inheritance category and the ID of the one-parameter declaration. C# permits both types; the argument count proves which one is named. Replacing both class targets with interfaces similarly changes a proved `implements` relation into an uncertain base. This loses information needed by consumers and displays the wrong relation heading in an outline.

From a scratch directory containing `case.cs`, reproduce both public output paths and a genuine warm hit:

```sh
hug symbols case.cs --json-full --no-cache
hug types case.cs --json --no-cache
hug symbols case.cs --json-full --verbose
hug symbols case.cs --json-full --verbose
```

The final invocation reports one hit and zero parses while retaining the incorrect relation.

### Instance sweep

For each target category below, the fixture declares `Base`, `Base<T>` and `Base<T,U>`; the child separately writes `Base`, `Base<int>` and `Base<int,string>`. Each shape is run with and without `partial`. Namespace, nested and competing-owner versions preserve the same declaration family and vary only the location/path. Every source is served through uncached complete output, cache population and warm complete output. Expected identity is the sole declaration with the written arity; target category still follows the accepted classification rules. Struct forms test identity conservatism on incomplete/invalid inheritance source; class and interface rows independently demonstrate the failure on valid C# programs.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Final bare target | All six forms × partial/non-partial × arities 0/1/2 (36 sources) | All 36 lose the target ID | Select the sole matching-arity declaration; class/record-class inheritance and interface conformance retain their proved category |
| Final namespace-qualified target | Same 36 declarations under `namespace N`, child writes `N.Base<…>` | All 36 lose the target ID | Matching-arity ID and in-file qualified name |
| Final nested target under one outer owner | Same 36 declarations in unique `H`, child writes `H.Base<…>` | All 36 lose the target ID | Matching-arity ID and in-file qualified name |
| Final member of a source-anchored owner | Same 36 declarations in `H`, child inside that H; unrelated `H<T>` outside | All 36 lose the target ID | Select only the matching-arity member of the actual owner |
| Intermediate named owner | `H`, `H<T>`, `H<T,U>` each contain Base; outside child writes `H<…>.Base`; partial/non-partial, repeated across all six fixture-form groups (36 sources) | All 36 resolve correctly | Same; clean named intermediate-owner branch |
| Nested intermediate named owner | Unique Outer contains `H`, `H<T>`, `H<T,U>` with Base; outside child writes `Outer.H<…>.Base`; partial/non-partial, repeated class/interface fixture groups (12 sources) | All 12 resolve correctly | Same; clean nested named traversal |
| Intermediate member of a source-anchored owner | Same H family inside Outer, child inside that Outer, competing Outer<T> outside; child writes `H<…>.Base` (12 sources) | All 12 lose the target ID | Select matching-arity H before entering its Base member |
| Unique-target control | Only matching-arity class or interface Base exists; partial/non-partial × arities 0/1/2 (12 sources) | All 12 resolve correctly | Same; positive extraction control |
| Selective public report | Class/interface unique control and added different-arity sibling, `types --json --no-cache` (four sources) | Both controls resolve; both sibling edits erase the ID and category | Preserve control relation after adding the distinct sibling |
| Outward lookup from a unique type | File Base of the written arity; H contains only a wrong-arity Base plus Child; all six forms × partial states × arities 0/1/2 (36 sources) | All 36 lose the outer target ID | Skip the nearer wrong-arity type and use the matching outer target |
| Outward lookup from a source-anchored type | Same shape plus competing H<T> outside (36 sources) | All 36 lose the outer target ID | Same, using the actual owner's evidence |
| Outward lookup from a namespace | File Base of the written arity; namespace N contains only a wrong-arity Base plus Child (36 sources) | All 36 lose the outer target ID | Continue to the matching outer target |
| Rooted target | All six forms × partial/non-partial × arities 0/1/2; child writes `global::Base<…>` (36 sources) | All 36 lose the target ID | Matching-arity target, with the same root lookup barriers as before |
| Cache service | Every complete-index source above, cold/population/warm | Equal relations; every warm run reports one hit and zero parses | Correct source facts on all paths; current cache faithfully preserves the defect |

The sweeps include all six written C# forms: class, struct, interface, record, record class and record struct. They cover each written arity, both partial states and both named and source-anchored lookup. Unique-target class/interface controls resolve correctly; adding the different-arity siblings causes the failure. Selective `types --json` independently confirms the class and interface controls and failures. Cold/warm results agree, so cache decoding is not the source of the defect.

### Reader and consumer sweep

| Site | Shape/check | Observed result | Expected result |
|---|---|---|---|
| C# path reader | Bare, generic, qualified and `global::` spellings above | Written segment arities are retained in `BasePath` | Same; clean input reader |
| C# declaration reader and owner groups | Ordinary/partial class, struct, interface and both record categories, arities 0/1/2 | Declaration arity/category are retained; review-10 negative/compatible-fragment regressions pass | Same; clean grouping evidence |
| Final named selection | Bare, namespace-qualified, nested and rooted target families | Uniqueness is checked before arity | Select among the correct-arity candidates |
| Source-anchored member selection | Final and intermediate member families in competing outer owners | Uniqueness is checked before arity | Select matching-arity members within the actual owner |
| Named intermediate-owner traversal | Direct and nested H families | Arity selects the correct owner group | Same; clean branch |
| Outward binding-scope selection | Wrong-arity nearer Base in unique/named/anchored type and namespace scopes | The nearer spelling stops search even though it cannot denote the written arity | Continue outward when that declaration cannot bind the written type name |
| Source fact classification | Class/interface controls with one distinct sibling added | Proved extends/implements becomes uncertain | Preserve target category and identity |
| Complete index, selective report and persistent projection | Supplemental CLI cases and registered query/cache corpus | Consistent; cache hits preserve the incorrect facts | Correct facts consistently |
| Rust impl / Swift extension owner consumers | Registered source-owner, generic-shadow, empty/inherent/trait and extension matrices | L1 checks pass | Same; these consumers do not implement C# argument-count selection |
| Other language readers | Registered per-language relation/generic/attribute/modifier matrices, including no-construct and grammar-limit rows | L1 checks pass | Same; no C# partial/arity exception applies |
| Ordinary containment, bounded lookup work, schema invalidation | Registered partial containment, work-shape and version tests | L1 checks pass | Preserve these guarantees during the repair |

### Required repair and verification

Carry written arity into outward binding-scope selection, the final named candidate decision and every source-owner member decision. Do not discard arity while constructing bounded summaries. Apply this to ordinary and partial declarations, bare and qualified targets, and final and intermediate members of anchored owners. Keep competing declarations with the same arity unresolved; preserve the no-single-declaration rule for genuinely repeated partial target fragments, wrong-arity negatives, alias/import/generic-shadow barriers, and the existing clean intermediate path controls.

Register these cases in the tree-hugger library's [relation matrix and shared corpus](../../lib/tests/l1/symbols_relations.rs:3840), so complete and declarations-only queries, selective kind queries, cache projection/widening and genuine warm hits exercise them. Include class/interface classification assertions and bounded-work rows with several arities. Update current docs and the extractor revision alongside the behavioral repair. No compiler lookup, alias expansion or schema change is needed.

## Blocked Findings

None.

## Recurrence

This is the same identity-evidence class as review 10's **C# partial-owner merging ignores type identity** and review 9's **Binding and scope checks still combine distinct source owners**. Review 10 explicitly required arity to distinguish C# type identities, including outside paths. Its repair fixed grouping and named intermediate-owner traversal, but did not sweep final-target selection, source-anchored member selection with an overloaded member family, and outward search past a wrong-arity declaration. Those sibling sites are enumerated above, including the clean intermediate named-path and unique-target controls.

Reviews 1–6 and 8 also report source-binding/owner-evidence loss at different lookup stages; review 7 reports the related Swift owner-generic context consumer. Their enumerated non-C# cases remain registered. No repeated generic-default, cache-decoder, schema-example or production-work finding was reproduced. `recurrence: true` records the C# identity class.

## Input Robustness Matrix

Persistent JSON cache is the only affected serialized input format; this feature adds no manifest, lockfile, YAML or TOML reader. The following contracts remain covered by registered public-output matrix tests. Execution results are recorded in Verification below. Each matrix starts from
shipped fixture output, changes one field per cell, and uses public service
or recomputation results rather than a parser-only assertion. The finding below concerns source lookup, not decoder contracts. The full feature-field
matrix follows; absent collections intentionally encode empty collections.

Below, **S** means the edited entry is served with the stated value;
**R** means it is rejected and recomputed. “Control” means its populated
original value. Wrong-type scalar cells use the opposite scalar type; numeric
fields use a string. The one/all-element shapes use arrays for scalar fields.
Empty means `[]` for lists, `""` for strings, `{}` for objects, or the explicit
zero control where shown. These are the cache format's documented outcomes,
not a claim that malformed source programs have these shapes.

### Collections populated by this feature

| Shape | generic_params | declaration constraints | extends | implements | unclassified_bases | attributes | modifiers |
|---|---|---|---|---|---|---|---|
| Unedited control | S(control) | S(control) | S(control) | S(control) | S(control) | S(control) | S(control) |
| Absent | S([]) | S([]) | S([]) | S([]) | S([]) | S([]) | S([]) |
| Explicit null | R | R | R | R | R | R | R |
| Wrong whole type | R | R | R | R | R | R | R |
| Wrong one element | R | R | R | R | R | R | R |
| Wrong every element | R | R | R | R | R | R | R |
| Empty | S([]) | S([]) | S([]) | S([]) | S([]) | S([]) | S([]) |
| Duplicate key | R | R | R | R | R | R | R |
| Trailing invalid document content | R | R | R | R | R | R | R |

Absent lists are explicitly the format's encoding for empty lists; explicit
null is rejected. This default is documented and independently asserted.

### Every generic-parameter member

| Shape | name | kind | constraints | default | variance | position |
|---|---|---|---|---|---|---|
| Unedited control | S(T) | S(Type) | S([MatrixBase]) | S(MatrixBase) | S(out) | S(0) |
| Absent | R | R | S([]) | S(None) | S(None) | R |
| Explicit null | R | R | R | S(None) | S(None) | R |
| Wrong whole type | R | R | R | R | R | R |
| Wrong one element | R | R | R | R | R | R |
| Wrong every element | R | R | R | R | R | R |
| Empty | S("") | R | S([]) | S("") | S("") | S(0) |
| Duplicate key | R | R | R | R | R | R |
| Trailing invalid document content | R | R | R | R | R | R |

### Every relation-target member

| Shape | name | id | qualified_name | source_span |
|---|---|---|---|---|
| Unedited control | S(MatrixBase) | S(matrix-id) | S(MatrixBase) | S(control span) |
| Absent | R | S(None) | S(None) | S(None) |
| Explicit null | R | S(None) | S(None) | S(None) |
| Wrong whole type | R | R | R | R |
| Wrong one element | R | R | R | R |
| Wrong every element | R | R | R | R |
| Empty | S("") | S("") | S("") | R |
| Duplicate key | R | R | R | R |
| Trailing invalid document content | R | R | R | R |

### Every relation-span member

| Shape | start.line | start.column | end.line | end.column | start_byte | end_byte |
|---|---|---|---|---|---|---|
| Unedited control | S(2) | S(3) | S(4) | S(5) | S(6) | S(7) |
| Absent | R | R | R | R | R | R |
| Explicit null | R | R | R | R | R | R |
| Wrong whole type | R | R | R | R | R | R |
| Wrong one element | R | R | R | R | R | R |
| Wrong every element | R | R | R | R | R | R |
| Empty/zero control | S(0) | S(0) | S(0) | S(0) | S(0) | S(0) |
| Duplicate key | R | R | R | R | R | R |
| Trailing invalid document content | R | R | R | R | R | R |

These cells exercise the unsigned-integer decoder contract. They do not claim
that an arbitrary edited span is semantically valid source evidence. The same
span decoder serves `source.body_span`; relation lists and container/member
references share the target decoder.

### Attribute/modifier member repair from review 4

| Shape | Attribute.name | Attribute.value | Modifier.type — Custom | Modifier.Custom.value | Modifier.type — Static |
|---|---|---|---|---|---|
| Unedited control | S(ReviewAttr) | S(`"raw value"`) | S(Custom) | S(final) | S(Static) |
| Absent | R | S(None) | R | R | R |
| Explicit null | R | S(None) | R | R | R |
| Wrong whole type | R | R | R | R | R |
| Wrong one element | R | R | R | R | R |
| Wrong every element | R | R | R | R | R |
| Empty string | S("") | S("") | R | S("") | R |
| Duplicate key | R | R | R | R | R |
| Trailing invalid document content | R | R | R | R | R |

All 45 cells are now permanent in the tree-hugger CLI's
[member matrix](../../cli/tests/symbols/cache_robustness.rs:928), including
unedited extraction controls for a nonempty raw argument, a Custom payload and
a Static tag. Absent/null optional attribute values are a documented None
contract, tested separately. No decoder defect was observed.

### Metrics inside a requested facet

| Shape | whole facet | parameters | has_docs | lines |
|---|---|---|---|---|
| Unedited control | S(control object) | S(1) | S(false) | S(3) |
| Absent | R | R | R | R |
| Explicit null | R | S(None) | S(None) | S(None) |
| Wrong whole type | R | R | R | R |
| Wrong one element | R | R | R | R |
| Wrong every element | R | R | R | R |
| Empty | R | R | R | S(0) |
| Duplicate key | R | R | R | R |
| Trailing invalid document content | R | R | R | R |

Facet absence when Metrics is unrequested is separately verified by cache
projection. The concurrent `cyclomatic` member also has a required-key column,
but its counting rules are outside this feature. Existing entry/record version,
fingerprint, coverage and reference-evidence matrices passed. Current schema
and revision invalidation tests passed. No additional decoder finding was reproduced for this feature's format fields.

## Requirement-to-verification assessment

| Requirement | Appropriate verification | Assessment |
|---|---|---|
| Explicit generics, order, ownership, bounds/defaults/variance and requirements | L1 public API | Per-language matrices, boundary cases, and selective/cached corpus comparisons are registered. |
| Written base text/order, local identity and category | L1 API/CLI | Language, path, source-owner and partial-identity matrices plus supplemental CLI sweep. The arity-selection finding blocks readiness. |
| Rust trait/inherent/negative/empty impls, method-only facts and member ownership | L1 API/CLI | Impl consumers and containment/cache-widening checks are registered. No new finding. |
| Swift extensions, raw-type exclusion and Go interface exclusions | L1 API/CLI | Owner/generic/shadow and unsupported-form matrices are registered. No new finding. |
| Declaration-owned modifiers and raw attributes, order and arguments | L1 API/CLI | Language, wrapper/leakage and serialized-member matrices are registered. No new finding. |
| Physical body lines, absent/expression/nested bodies, LF/CRLF and boundaries | L1 API | Shared body-node logic, metric/span matrix and projection tests. No new finding. |
| Skipped optional work, one parse/definitions query and bounded relation lookup | L1 counters | Query counters and individually registered work shapes; no new production performance finding. |
| Cache projection/widening, actual hits, strict fields and versions | L1 public API/CLI | Partitioned corpus consumers and robustness matrices; supplemental warm hits expose the source defect. |
| Outline labels, signatures, selection, wrapping and style | L1 CLI plus L2 terminal capture | Real tmux/WezTerm capture passed; incorrect upstream C# classification can still produce an incorrect label. |
| JSON/JSONL/full indexes, counts and statistics | L1 CLI | Registered CLI report/golden checks; supplemental complete/selective output. |
| Keyboard, paste, IME or mouse behavior | L3 if promised | No new input behavior is promised by this feature; L3 is unnecessary. |
| Current docs, schema and older-model losses | Inspection plus L1 | Current schema 5.0 and extractor revision 39 agree. Arity-path wording needs implementation to cover the missing branches. |

The library's [declared L1 target](../../lib/Cargo.toml:55) includes the relevant modules in [main.rs](../../lib/tests/l1/main.rs:31), despite `autotests = false`. Static group/shape macros register ordinary L1 names and coverage guards check their completeness. CLI report/cache modules belong to the declared symbol target. The [real-terminal targets](../../cli/Cargo.toml:38) require `terminal-tests`, which the live L2 recipe and CI test metadata enable. The tier audit reports zero stranded tests. This review adds no test targets or tier markers.

## Verification

- First `just test` attempt: 581 passed, one timeout, 767 unrun, in 106.561 seconds. The timed-out test was the older `symbols_cache::projected_results_equal_uncached_queries` at 30.019 seconds; the newly partitioned declaration-fact groups completed. This attempt overlapped lint/L2 build activity. It provides no full-suite passing evidence.
- Full `just test` rerun at normal concurrency, without changing timeouts or retry settings: **1,349 passed, zero skipped**, 182.029 seconds. All seven groups of each declaration-fact consumer ran, as did the source-owner/partial identity, schema/cache robustness, metrics and report tests. This closes the enumerated aggregate-test repair assessment; the first-attempt timeout remains a local execution limitation, not a new source-lookup defect.
- `just lint`: passed for both packages.
- `just check-tier-coverage tree-hugger`: zero stranded tests.
- `just test-l2`: six selected entry points passed in 32.433 seconds. tmux and WezTerm supplied real-terminal capture; Kitty entry points used unavailable-backend gates and supply no Kitty rendering evidence. The harness used background resources without keyboard injection or focus requests.
- Supplemental complete-output sweeps: **360 sources**, each uncached, populated, and warm; every warm invocation reported one hit and zero parses, and relations agreed. The 36-source intermediate-owner group and 12-source nested-intermediate group repeat the same path fixtures across category loops, rather than representing distinct path spellings. Four selective `types --json` runs independently verify class/interface controls and failures. Removing the nearer wrong-arity declaration is additionally checked as a single-edit positive control on 18 class/interface outward-lookup sources.
- Review frontmatter and scoped whitespace checks: YAML parsed successfully and scoped `git diff --check` passed..

Used the tree-hugger, rust-testing and biscuit-test-harness skills. Inspected the spec, accepted classification choice, implementation log, earlier findings, source readers, shared scope/owner lookup, query/cache consumers, current docs, and test registration. Concurrent scoped-reference and complexity changes are outside this feature except where shared behavior affects its contract. No implementation code was changed by this review.

Only this review document and the requested previous-review/spec metadata were edited. The feature remains active for the author to close after review.
