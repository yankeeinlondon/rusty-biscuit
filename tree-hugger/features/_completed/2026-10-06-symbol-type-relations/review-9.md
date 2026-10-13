---
$schema: feature-review.yaml
ready: false
findings:
  - title: Binding and scope checks still combine distinct source owners
    priority: high
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-06T12:51:45-07:00
spec: 2026-10-06-symbol-type-relations/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-06-symbol-type-relations/log.md
description: "A **feature** review of `2026-10-06-symbol-type-relations/spec.md`"
feature: 2026-10-06-symbol-type-relations/review-9.md
previous: 2026-10-06-symbol-type-relations/review-8.md
next: 2026-10-06-symbol-type-relations/review-10.md
---

# Review of Symbol Type Relations — Iteration 9

The feature is **not production ready**. Review 8's enumerated cases are
repaired, including conservative Swift extension membership and bounded C++
inline lookup. A remaining instance of the owner-identity defect combines
imports, inherited-name barriers, or possible built-in shadowing from distinct
owners that share a name. This removes proved relations and Rust ownership,
changes proved superclass categories, and invents Swift raw-type bases.
One high finding carries the whole tested class; no human decision is needed.

## Previous findings and blocked work

Read the specification, accepted classification choice, implementation log,
and findings and recurrence assessments in reviews 1–8. The accepted design
uses precise `extends` and `implements`, retains uncertain written bases in
`unclassified_bases`, and attaches Swift extension facts only to a uniquely
proved same-file owner. That choice remains unchanged.

| Review 8 finding | Assessment |
|---|---|
| Relation and membership decisions discard unresolved owner evidence | The listed duplicate Rust module/Scala object, distinct C++ inline namespace, source-anchored member, Rust impl, unresolved Swift owner/context, and Go receiver cases now have registered passing regressions. Swift conformance and membership use the same decision; Containment widening reparses when needed. The source-anchored **declaration candidates** are repaired, but related import, scope-flag and built-in checks still combine different owners; see the finding below. |
| Inline namespace paths still repeat broad candidate scans | Repaired. `Denoted` bounds continuation to one qualifier owner; ambiguity stops further lookups. Ordinary scopes use exact-name lookup. Registered broad intermediate, explicit, rooted, bare, deeper, same-final-name, struct-owner, function-beside-namespace and deep-path work regressions pass. Inspection found no remaining per-suffix broad-owner scan at these sites. |

Review 8 has no blocked findings. The earlier missing C#/Swift/Scala base-list
readers were unblocked by the author's accepted choice and are implemented.
Earlier template defaults, callable requirements, cache robustness columns,
C# record classification and schema-documentation findings remain repaired.
Their registered regressions pass. The new finding concerns another instance
of owner evidence loss, rather than a request to reconsider those decisions.

## Verification and scope

Used the tree-hugger, rust-testing and biscuit-test-harness skills. Reviewed
the feature's declaration readers, resolution and ownership consumers,
query/cache projections, outline rendering, current documentation, test
matrices and target/tier registration. Concurrent scoped-reference and
cyclomatic-metric work was preserved and its independent requirements were
not judged as part of this feature.

- `NEXTEST_TEST_THREADS=4 just test`: **1,229 passed, zero skipped**, 142.515 seconds.
- `just lint`: passed for the library and CLI.
- `just check-tier-coverage tree-hugger`: zero stranded tests.
- `just test-l2`: **six selected entry points passed**, 12.835 seconds. tmux and WezTerm supplied report-rendering evidence; Kitty's two entry points used unavailable-backend gates and supply no Kitty evidence. The recipe spawned background resources; no foreground spawn or keyboard injection was requested.
- Supplemental shipped-CLI probes used copied language fixtures in temporary Git repositories with isolated home/config/cache directories. Each usable source shape had a control before the sibling edit, then complete uncached, selective type uncached, cache population and warm complete runs. Every warm run reported `1 hits` and `parsed 0 files`; complete cold and warm facts agreed. Selected type relation facts agreed with complete output. There were 71 usable probe rows, including all 30 Swift raw-type spellings. Three exploratory rows with unsuitable or unchanged setup were excluded from conclusions.
- Twelve clean source-owner rows were additionally checked against exact target IDs and the first owner's declaration source position, including duplicate members, sibling generic parameters, pattern bindings and function-local Rust modules.
- The executable Python reproduction below was also checked in the host interpreter: the first class's child retains the first class's base after the second class imports another base.
- No implementation code or permanent tests were added. The new shapes are supplemental review evidence and must become registered regressions during repair. Existing cache-widening tests passed; these new shapes were checked through public CLI cold/selective/warm paths, not an additional supplemental in-process widening probe.

Cross-OS execution evidence is left to CI and does not decide readiness.

## Unblocked Findings

### High — Binding and scope checks still combine distinct source owners

**Defect class:** after source ancestry identifies one owner, auxiliary binding,
inherited-scope and built-in checks still combine facts by displayed owner
name, allowing a different same-named owner's facts to change the result.

In the tree-hugger library,
[Declarations::walk](../../lib/src/file/tree_file/relations.rs:781), which
collects imports and bindings needed to prove relation targets, keys named
scope facts by `self.bound[idx]`, rather than declaration identity.
[Declarations::binding_scope](../../lib/src/file/tree_file/relations.rs:1251),
which chooses where a relative path begins, checks those combined imports and
unrecorded bindings **before** `anchor_around` identifies the source owner.
Its inherited-name barrier also comes from
[Declarations::scope_flags](../../lib/src/file/tree_file/relations.rs:971),
which combines every same-named owner's base clauses.

[Declarations::resolve_path](../../lib/src/file/tree_file/relations.rs:1350),
which continues a target path and handles Rust `self::`/`super::`, now uses an
anchored owner's own declaration candidates. However, that branch calls
[Declarations::owner_rebinds](../../lib/src/file/tree_file/relations.rs:1058)
with the owner's **name**, so another owner's import still rejects the selected
member. Finally,
[Declarations::may_bind](../../lib/src/file/tree_file/relations.rs:1288),
which decides whether a standard-looking Go/Swift spelling might be shadowed,
looks at combined named members instead of the actual source owner. In Swift
this retains non-base raw types that the first owner's scope does not shadow.

Why this matters: relation consumers receive missing identities and lost
Rust conformance/membership, and Swift/Scala/C# consumers see known superclass
relations downgraded to uncertain bases. The inverse built-in error adds
relations that are not declared inheritance or conformance. Warm caching
faithfully retains these wrong facts.

### Reproduce through the shipped CLI

Append this to a copy of the Rust symbol fixture in a scratch repository:

```rust
mod ReviewHolder {
    pub trait ReviewBase {}
    pub trait ReviewChild: ReviewBase {}
    pub trait ReviewSelf: self::ReviewBase {}
    mod inner { pub trait ReviewUp: super::ReviewBase {} }
}
mod ReviewHolder { use external::ReviewBase; }
```

```sh
hug --json-full symbols --no-cache probe.rs
hug --json types --all-scopes --no-cache probe.rs
hug --verbose --json-full symbols probe.rs
hug --verbose --json-full symbols probe.rs
```

All three supertraits retain their text but lose the first module's Base ID.
Remove only the second module's import and all three resolve to the first
module's Base. The source identifies that first module directly; deciding its
own members does not select a `cfg` branch or resolve an external import.

The same defect occurs in executable Python:

```python
class ReviewHolder:
    class ReviewBase: pass
    class ReviewChild(ReviewBase): pass
class ReviewHolder:
    from builtins import object as ReviewBase
```

The first Child inherits the first Base. The second class's import cannot
change that evaluation, yet the CLI loses the target ID. Removing only that
import restores it. This is not merely an ambiguity in compiler-invalid
input. Duplicate Rust/Java/Swift/Scala/C# declarations in the other rows are
source-evidence cases under the same policy already accepted in review 8;
the extractor neither compiles the program nor evaluates conditional code.

For inherited barriers, use a file-level Base and put Child in a base-less
Holder, then add `extends External` (or `: External`) to a second Holder.
For Swift raw types:

```swift
struct ReviewHolder { enum ReviewChild: Int { case value } }
struct ReviewHolder { class Int {} }
```

Without the second Holder's Int, Child correctly has no base relation. Adding
that unrelated member invents an unresolved `unclassified_bases` entry `Int`.
For qualified spellings, the unrelated member is the first segment: `class
Swift {}` or `class Unicode {}`.

### Binding and scope instance sweep

“Own ID” means the target in the actual source owner, checked against the
control's public identity; “root ID” means the file/package-level Base.
“No base” means all three relation lists are empty.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust relative first segment | Base and Child in first module; second module imports Base | Base text without ID | Own Base ID |
| Rust anchored continuation | Same edit; `self::Base` and inner `super::Base` | Both text without ID | Own Base ID in both |
| Rust wildcard first segment | Second module has `use external::*` | Bare Base loses ID | Own Base ID; sibling glob has no effect |
| Rust wildcard anchored control | Same sibling glob; `self::Base` | Own ID retained | Same; qualified local declaration wins |
| Rust local-module controls | Both modules are inside a function; second imports Base; bare and `self::Base` | Own IDs retained | Same; syntax-scope binding checks are clean |
| Python direct class-body lookup | Later same-named class imports Base, including executable `builtins.object as Base` | First Child loses Base ID | First class's own Base ID |
| Python unrecorded-binding control | Later class assigns `Base, x = value` | First Child keeps own ID | Same; retain |
| Scala named import | Base/Child in first object; second object imports Base | Unclassified text without ID | Own ID in extends |
| Scala wildcard barrier | File-level Base; Child in first object; second imports `external._` | Unclassified text without ID | Root ID in extends |
| Scala pattern-binding controls | Second object has `val (reviewBase, x) = value`; also uppercase stable-pattern spelling | First Child keeps own Base ID | Same; value-group binding must not hide a type |
| Java inherited-name barrier | File/package Base; Child in first base-less Holder; second Holder extends External | Extends text without ID | Root ID in extends |
| Swift inherited-name barrier | Same source-owner arrangement | Unclassified text without ID | Root ID in extends |
| Scala inherited-name barrier | Same source-owner arrangement | Unclassified text without ID | Root ID in extends |
| C# inherited-name barrier | Same arrangement, two non-partial Holders | Unclassified text without ID | Root ID in extends; these are not partial fragments |
| C# own-member lookup | First non-partial Holder has Base/Child; second also declares Base | Unclassified text without ID | First Holder's own Base ID in extends |
| C# partial control | Child in one partial Holder fragment; Base in the other | Exact member Base ID in extends | Same; preserve genuine partial merging |
| Java/C++/Swift own-member controls | Both owners declare Base; Child is inside the first | First owner's Base ID | Same; source-anchored candidate repair works |
| C++ inherited-sibling control | File Base; first Holder has Child; second has `: External` | Root Base ID retained | Same; no sibling inheritance leakage in this tested row |
| C++ mixed named-owner import barrier | Struct Holder has own Base/Child; same-named namespace contains a using-directive | Base text without ID | Struct's own Base ID; another owner kind does not supply its imports |
| Java/C++/Swift/Scala/C# generic-owner controls | First Holder has Base/Child; second owns a generic parameter named Base | Own Base ID retained | Same; actual ancestor generic layers are clean |
| Swift unresolved-extension control | Duplicate Holder; extension of a nested type writes Child: Base; sibling gains inheritance | Both control and edit remain text-only unclassified | Same; extension owner is independently unproved |
| TypeScript namespace control | Reopened namespace; Child in one fragment, exported Base in another | Exact Base ID | Same; namespace merging is intentional |
| JavaScript callable control | Child/Base in first class method; same-named second class extends External | Function-local Base ID retained | Same; lexical lookup is clean |
| Python plain-owner control | Second same-named class gains only an external base | First Child keeps its own Base ID | Same; the import check, not ordinary candidate lookup, fails |
| Go interface control | Add a second same-named interface embedding External | First interface keeps its file-level Base ID | Same; no invented structural conformance |
| PHP namespace controls | Reopened namespace blocks | Exact local Base ID | Same; namespace merging is intentional |
| C/Perl/Bash/Zsh/Lua | Registered absent-relation language rows | No invented base | Same; no applicable relation-owner lookup |

Same-owner negative controls also remain conservative: Rust, Python and Scala
imports **inside the actual owner** leave a conflicting target unresolved;
a C++ using-directive in the actual namespace does so under the existing
conservative policy; Java's actual Holder extending External blocks outward
lookup. These controls distinguish an unrelated sibling from genuine binding
uncertainty and must survive the repair.

### Rust impl consumer sweep

The impl is inside the first module, with unique local self/trait declarations.
Only the second same-named module gains the stated import.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Self-type resolution, positive impl | Sibling imports self type Base; impl has one method | Base loses implements; method loses container ID | Preserve Base conformance and method ownership |
| Trait resolution, positive impl | Sibling imports Trait; self type stays unique | Base and method keep only text-only Trait | Own Trait ID on both; ownership unchanged |
| Empty positive impl | Sibling imports Base; impl has no members | No conformance on Base | Preserve the first module's conformance summary |
| Inherent impl | Sibling imports Base; one method | Method loses container ID | Preserve first module's member ownership; no conformance |
| Outside-owner ambiguity controls | Registered duplicate-module self, trait, both-path and empty/inherent cases from review 8 | Unproved external traversal remains unresolved | Same; anchoring applies only when source identifies the owner |
| Public CLI projections/cache | Complete cold, selective types, population and actual warm hit for the failing rows | Same faulty type facts; warm full output preserves faulty member facts | Correct facts on every path; add in-process method-only and widening regressions |

### Built-in decision sweep

The Swift rows all put an enum in the first non-generic Holder and add a type
named by the raw expression's first segment **only inside the second Holder**.
Every unedited control reports no base. Every edited row instead reports that
raw expression as unresolved `unclassified_bases`, through complete,
selective and actual warm output.

| Site | Shapes tested individually | Observed result | Expected result |
|---|---|---|---|
| Swift `may_bind`, bare raw types | Int, Int8, Int16, Int32, Int64, UInt, UInt8, UInt16, UInt32, UInt64, String, Character, Float, Double | Fourteen invented unclassified bases | No base in all fourteen |
| Swift `may_bind`, qualified standard raw types | Swift.Int, Swift.Int8, Swift.Int16, Swift.Int32, Swift.Int64, Swift.UInt, Swift.UInt8, Swift.UInt16, Swift.UInt32, Swift.UInt64, Swift.String, Swift.Character, Swift.Float, Swift.Double | Fourteen invented unclassified bases | No base in all fourteen |
| Swift `may_bind`, nested standard raw types | Unicode.Scalar; Swift.Unicode.Scalar | Two invented unclassified bases | No base in both |
| Swift known generic/alias/import and unknown-extension controls | Registered raw-type shadowing and all 30 unproved-extension-owner rows | Written uncertain bases preserved | Same; do not restore spelling-only deletion |
| Swift standard raw-type controls | All 30 unedited probe controls and registered standard rows | No base | Same |
| Go built-in reader/control family | Registered 19 predeclared spellings, local interfaces, type/value import and lexical-shadow rows | Correct exclusion or local interface relation | Same; Go has no applicable nested named type body for this sibling edit |
| Other fourteen languages | Relation readers do not use this Go/Swift exclusion | No corresponding built-in decision | No applicable sibling site |

### Required repair and coverage

Preserve declaration identity for **all** facts used after an owner is selected,
not only its member records. Keep imports, wildcard/unknown bindings,
unrecorded bindings, scope kind and inherited-name barriers indexed by source
owner where owners compete. A source-anchored lookup must read that owner's
facts. The built-in decision must use the same actual scope and candidate
visibility as relation resolution. Namespace reopenings and compatible C#
partial fragments retain their intentional shared lookup; exempting every
C# type by language alone is insufficient.

Sweep and repair the collectors and consumers together: `walk`, `summaries`,
`scope_flags`, `binding_scope`, `owner_rebinds`, the anchored branch of
`resolve_path`, and `may_bind`; verify `named_member_rebound`,
`syntax_member_rebound`, `owner_binds`, `type_parameters_around`,
`resolve_self_type`/`resolve_impl`, `extension_owners`, `swift_extensions`,
`written_bases` and cache projection against the same decisions. The direct
syntax binding path, ancestor generic layers, genuine namespace/partial
merging, and unresolved-extension handling were clean in the listed controls.

Add every failing and clean row above to the registered relation/owner
matrices, including all 30 Swift spellings and all four Rust impl consumers.
Assert exact target/owner IDs and source positions, classification, members
and written text. Run complete/declarations-only/method-only queries,
matching-capability narrowing, Containment/complete widening and actual warm
hits. Keep bounded-work counter regressions: changing owner keys must not
introduce per-relation whole-file scans. Bump the extractor revision to reject
stored wrong facts, and update current docs, skill and behavior-linked comments.

The current documentation's promise that an inside-owner lookup reads only
that owner's members exceeds this implementation once imports and inherited
barriers are considered. That is documentation drift caused by the incomplete
repair; correct the code and accompanying explanation together.

## Blocked Findings

None. This repair follows the accepted same-file and conservative-ownership
contract and requires no new author choice.

## Recurrence

This finding repeats review 8's **Relation and membership decisions discard
unresolved owner evidence**. That repair should have swept every sibling
consumer of its new anchored owner: import/unrecorded checks before anchoring,
member-rebinding checks after anchoring, source-owner scope flags, and the
built-in shadowing projection. It repaired declaration candidates and Swift
extension consumers, but explicitly treated merged flags/bindings as harmless
conservatism. They are not harmless when source ancestry proves a different
owner: they remove required local identities and invent raw-type bases.

The same broader evidence-loss class appears in each earlier review:

- Review 1, **Relation IDs are assigned without proving path meaning**.
- Review 2, **Python nested class scopes still assign incorrect base identities**.
- Review 3, **Relation lookup still drops import and generic-shadowing evidence**, and its **Built-in name exclusions erase valid Go and Swift bases** sibling decision.
- Review 4, **Relation lookup still substitutes record names for source binding evidence**.
- Review 5, **Relation lookup loses binding evidence between syntax scopes and display-name indexes**.
- Review 6, **Relation lookup still loses binding and owner evidence after selecting the first name**.
- Review 7, **Swift nested-type extensions lose enclosing generic bindings**, another consumer of owner context.

Those fixes should have kept the actual owner through both candidate selection
and every predicate that can accept, reject or classify that candidate. This
review sweeps the source-owner binding, inherited barrier, built-in and impl
consumers together and lists the clean counterparts. `recurrence: true`
records that missed class sweep. The earlier performance, cache-member matrix,
C# record-category and schema-example classes have no new finding here.

## Input Robustness Matrix

Persistent JSON cache is the only affected serialized input format; this
feature adds no manifest, lockfile, YAML or TOML reader. All registered
public-output robustness tests passed in this run. Each matrix starts from
shipped fixture output, changes one field per cell, and uses public service
or recomputation results rather than a parser-only assertion. The owner
repair changes source evidence, not decoder contracts. The full feature-field
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
and revision invalidation tests passed. No additional input-robustness finding
remains for this feature's format fields.

## Requirement-to-verification assessment

| Requirement | Appropriate level | Strongest evidence and assessment |
|---|---|---|
| Explicit generics: ownership, order, bounds/defaults/variance, requirements, overflow | L1 public API | Registered nine-language generics/ownership/requirements/overflow matrices passed; unsupported forms have documented grammar limits. Ancestor generic layers were clean in the supplemental sibling sweep. |
| Declared bases: written text/order, exact identities, conservative classification | L1 public API/CLI | All sixteen registered language rows and prior path regressions passed. New source-owner import/inheritance cases lose required identities/categories; blocks readiness. |
| Go/Swift built-in exclusion | L1 public API/CLI | Registered controls passed. All thirty supplemental Swift sibling-owner spellings invent raw-type bases; blocks readiness. Go controls remain clean. |
| Rust multiple/empty/negative/inherent impls; member/type ownership | L1 public API/CLI | Registered regressions passed. New source-anchored sibling-import cases lose conformance or membership; blocks readiness. |
| Swift extension conformance/membership and uncertain generic context | L1 public API/CLI | Prior known/unproved owner, nested generic, containment-widening and actual-hit regressions passed. No new extension-owner disagreement found. |
| Declaration-owned modifiers and raw attributes | L1 public API/CLI | All language/wrapper/leakage/order/repetition/empty-argument matrices and persistent member-column tests passed. No new finding. |
| Callable body lines: same span, expression/empty/absent/nested forms, LF/CRLF, end-column boundary | L1 public API | Metrics/body-span/arithmetic/required-key and projection regressions passed; shared body reader inspection agrees with the contract. No new finding. |
| One parse/definitions query; no unnecessary optional passes; bounded work | L1 counters | Registered query counters, shared-context, broad/deep inline and separately written bound work regressions passed. The review-8 performance finding is repaired. |
| Cache narrowing/widening, actual hits, schema/revision invalidation and strict fields | L1 isolated API/CLI | Registered projection/widening/invalidation/decoder tests passed. Supplemental cold/selective/warm facts agree but preserve the source defects. Current schema is 5.0, extractor revision 36. |
| Outline labels, signature suppression, selectors, sorting, wrapping and style | L1 CLI plus L2 terminal capture | Registered outline/golden checks and tmux/WezTerm captures passed. Faulty upstream relations can still alter labels; no separate rendering defect found. |
| Counts/stats and JSON/JSONL/full-index facts | L1 CLI | Registered format/count/stat/golden tests passed. Supplemental JSON exposes the wrong source facts faithfully. |
| Keyboard, paste, IME or mouse behavior | L3 when terminal input encoding is promised | This feature introduces no such requirement; no L3 gap. |
| Current docs/schema/cache/report/API and lossy older-model adapters | Inspection plus L1 adapter/schema tests | Versions and adapter losses agree. Inside-owner isolation claims require the repair above. |

The library's [L1 target](../../lib/tests/l1/main.rs) declares generics,
relations, owner identity, shared context, relation work, query, cache,
metrics and adapter modules despite `autotests = false`. The CLI's
[symbol target](../../cli/tests/symbols/main.rs) declares outline, reports,
cache robustness and upgrade modules. Their ordinary names select L1.
Terminal targets require `terminal-tests`; the live L2 recipe and CLI CI
metadata enable it. The tier audit found no stranded tests. Passing these
registered suites does not close the supplemental reproduced failures.

Implementation requires one further repair cycle. The feature directory stays
active; the author closes it after the review cycle.
