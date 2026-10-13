---
$schema: feature-review.yaml
ready: false
findings:
  - title: Relation lookup still loses binding and owner evidence after selecting the first name
    priority: high
  - title: Relation processing exceeds the bounded per-file work contract
    priority: medium
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-06T02:09:12-07:00
spec: 2026-10-06-symbol-type-relations/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-06-symbol-type-relations/log.md
description: "A **feature** review of `2026-10-06-symbol-type-relations/spec.md`"
feature: 2026-10-06-symbol-type-relations/review-6.md
previous: 2026-10-06-symbol-type-relations/review-5.md
next: 2026-10-06-symbol-type-relations/review-7.md
---

# Review of Symbol Type Relations — Iteration 6

The feature is **not production ready**. The instances listed in review #5
have been repaired and their registered tests pass. A further sweep finds
remaining instances of the same binding-evidence defect, plus relation indexes
whose work grows much faster than their input. Neither finding needs a human
design decision.

## Previous findings and blocked work

Read the specification, repair logs, and findings of reviews #1–#5 before
examining the current implementation.

| Earlier finding or repair | Current assessment |
|---|---|
| Review #5: qualified continuation through local owners, Rust local modules, and Scala/TypeScript longer paths | Implemented for its listed instances. Exact-position target tests, complete/selective queries, Rust method-only ownership, containment widening, and actual warm hits pass. The selected owner's unrecorded member bindings remain unchecked in the new Python cases below. |
| Review #5: unrecorded binding scope/category, impl parameters, and bodiless signatures | Its listed Rust, TypeScript, and Scala repairs pass. Preserved categories and record-less owner ranges fix those cases. The range and same-name indexes introduce the performance failures below. |
| Review #5: Python directive writes, augmented assignments, and deletion | Its destination-binding rows pass. A class declaration redirected out of a class body is still incorrectly reachable through that class's qualified path; see the first finding. |
| Review #5: PHP class namespaces, Scala package objects, and C++ inline namespaces | Listed identity and classification rows pass. Inline-namespace alias construction has a separate growth problem. |
| Review #5: Swift extension members cannot attach to function/closure-local types | The listed absent-owner controls pass. A same-named closure-local owner also suppresses valid nested file-level owners, as shown below. |
| Reviews #1–#4: generics/defaults/requirements, path roots, Python class scopes, imports, generic overflow, built-in exclusions, and C# record classes | Registered regression matrices pass. The recurring lookup class remains open only for the additional instances enumerated here. |
| Reviews #2–#4: persisted generic/base/span/attribute/modifier members | Permanent public-output robustness matrices pass, including the five attribute/modifier member columns added after review #4. No additional decoder finding. |
| Review #3: current schema examples | Current examples agree with schema 5.0 and extractor revision 33; historical examples remain identified as history. |

Review #5 had no blocked findings. The earlier C#, Swift, and Scala work was
unblocked before its implementation: the author accepted precise `extends`
and `implements` lists, a separate `unclassified_bases` list, and Swift
extension conformance attached only to a uniquely resolved same-file owner.
Those readers and their classification controls are implemented. That decision
needs no reaffirmation.

## Verification and scope

Used the tree-hugger, Rust, rust-testing, and biscuit-test-harness skills.
Reviewed all 16 language acceptance rows, declaration-owned generic parameters,
modifiers and attributes, body lines, declared bases, Rust impl ownership,
Swift extensions, selective analysis, cache readers/projection/widening,
outline output, documentation, and test target/tier registration.

- `just test`: **1,212 passed, zero skipped**, 63.692 seconds.
- `just lint`: passed for both packages.
- `just test-l2`: six selected entry points passed, 23.707 seconds. tmux and
  WezTerm actually rendered the reports. Kitty entry points used their
  unavailable-backend gates, providing no Kitty rendering evidence. The
  harness uses background panes; no foreground injection was requested.
- `just check-tier-coverage tree-hugger`: zero stranded tests.
- Supplemental correctness checks copied the existing language fixtures into
  isolated temporary Git repositories, appended a controlled variation, and
  ran the shipped `hug` binary. The main sweep covered **112 source rows**:
  18 initial Python rows, 61 binding/reader controls, and 33 Swift/directive
  rows. All parsed without error diagnostics. Each had a complete uncached
  result, a type-only uncached result, and a complete warm hit reporting
  `1 hits` and `parsed 0 files`. For Swift extension methods, type-only output
  appropriately omits methods; complete and outline output establish ownership.
- Separate declarations-to-complete widening checks for the Python import
  and Swift nested-extension reproductions reported `1 partial hits`, then a
  genuine complete hit. The wrong facts persist through both paths.
- Python interpreter checks confirmed that the imported replacement and
  nested-pattern/match replacements become `object`, and that deletion makes
  the old member inaccessible. `swiftc -typecheck` accepted the Swift
  closure-shadow reproduction below.
- Performance checks appended generated shapes to copies of the existing
  fixtures and called the public `TreeFile::query(&SymbolQuery::new())`,
  isolating declaration analysis. They compared returned relations and
  `work_counters().shared_context_steps` at 200 and 400 repetitions. The
  six local-owner reader shapes each retained the expected target IDs and
  parsed cleanly. Inline-namespace timing is supplemental evidence, not a
  proposed timing-based test gate.

The concurrent scoped-reference and cyclomatic-metric functionality is present
in this worktree. Its independent behavior is outside this feature's review;
its tests are included in the passing suite. Cross-OS execution evidence is
left to CI and does not determine readiness. This review changes only the
requested review files and specification metadata.

## Unblocked Findings

### High — Relation lookup still loses binding and owner evidence after selecting the first name

**Defect class:** later path and ownership decisions substitute declaration
records or displayed-name prefixes for the actual bindings and syntax owners,
so evidence correctly collected earlier no longer controls the result.

The tree-hugger library's
[Declarations::resolve](../../lib/src/file/tree_file/relations.rs:825), which
assigns same-file IDs to bases and impl owners, checks import/unrecorded
bindings only while choosing a relative path's first name. Later named
segments use [named](../../lib/src/file/tree_file/relations.rs:644), and
local-owner segments use [members](../../lib/src/file/tree_file/relations.rs:656).
Neither consults the selected owner's imports, unrecorded writes, or redirected
bindings. An old nested-class record can therefore survive a later replacement
or deletion as a false relation target.

Append this to a copy of the Python fixture:

```python
class ReviewHolder:
    class ReviewBase: pass
    from builtins import object as ReviewBase
class ReviewChild(ReviewHolder.ReviewBase): pass
```

Run from the scratch repository:

```sh
hug --json-full symbols --no-cache probe.py
hug --json types --all-scopes --no-cache probe.py
hug --verbose --json-full symbols probe.py
hug --verbose --json-full symbols probe.py
```

All report the old `ReviewHolder.ReviewBase` declaration's ID in
`ReviewChild.relations.extends`. Python actually gives Child the imported
`object` base. Keep the written target, with no same-file ID or qualified name.
An incorrect ID makes an inheritance graph point to a declaration that did
not supply the base; cache hits preserve the mistake.

#### Python member-binding sweep

Every row below was checked with both a module-level Holder and a Holder
inside `def reviewProbe(...)`. “Old ID” identifies the original nested class,
not just its displayed name. “Text only” preserves the written base with
neither an ID nor a qualified name.

| Site / shape tested | Observed result | Expected result |
|---|---|---|
| Qualified member after `from external import ReviewBase`, or `import external as ReviewBase` | Old ID in both owner contexts | Text only; imported bindings do not identify the old local class |
| Qualified member after `from external import *` | Old ID in both contexts | Text only; the import can replace the member |
| Qualified member after `del ReviewBase` | Old ID in both contexts | Text only; the member was removed |
| Qualified member after `ReviewBase += external` | Old ID in both contexts | Text only; the old binding is no longer proved |
| Qualified member after nested tuple/list assignment or a starred target binding ReviewBase | Old ID in both contexts | Text only; retain all nested pattern bindings |
| Qualified member after dictionary `match` capture `case {"base": ReviewBase}` | Old ID in both contexts | Text only; the pattern binds the member |
| Qualified member after simple assignment, flat tuple/list assignment, `for`, `with ... as`, `except ... as`, or walrus binding | Text only in both contexts | Observed result; clean controls, mostly because these forms produce a conflicting record |
| Qualified member after a same-named `def` or a second same-named class | Text only in both contexts | Observed conservative result; clean |
| Unedited Holder; unrelated import or unrelated deletion | Correct nested-class ID in both contexts | Preserve these positive controls |
| Three-segment `ReviewHolder.ReviewMiddle.ReviewBase`: import/delete the final member | Old Base ID in both contexts | Text only |
| The same three-segment path: import/delete the intermediate ReviewMiddle owner | Old Base ID in both contexts | Text only; continuation cannot pass through the old owner |
| Direct qualified write after Holder: `ReviewHolder.ReviewBase = external`, `+= external`, or `del ReviewHolder.ReviewBase` | Old ID in both contexts | Text only; an explicit write to that proved path invalidates the old member binding |
| A class-body `global ReviewBase` followed by `class ReviewBase: pass`; module-level and function-local Holder | Fabricated Holder.Base ID | Text only; the declaration binds at the directive destination, not as Holder's member |
| A function-local Holder's `nonlocal ReviewBase` followed by `class ReviewBase: pass` | Fabricated Holder.Base ID | Text only for the same reason |
| Each directive above followed instead by `def`, import, simple/tuple assignment, or deletion, with no Holder member | Text only | Observed result; clean destination controls |

The tree-hugger library's
[python_targets](../../lib/src/file/tree_file/relations/lexical.rs:822), which
collects assignment/deletion bindings, explicitly ignores attribute targets.
That explains the direct qualified-write rows. The repair need only invalidate
an explicitly written, provable same-file owner path; it does not require
executing expressions, inferring arbitrary object types, or resolving imports.
For local-owner continuation, syntax containment alone also cannot prove that
a declaration redirected by `global` or `nonlocal` is that owner's member.

#### Swift owner-identity sweep

The tree-hugger library's
[out_of_view_owners](../../lib/src/analysis/mod.rs:692), which excludes local
types from file-level Swift extension membership, collects local owners by
qualified-name string and hides every record with that prefix. A closure's
local type can share that string with a distinct file-level type. The filter
then hides the file-level type's nested declarations too.

```swift
struct ReviewHolder { struct ReviewInner {} }
let reviewClosure = { struct ReviewHolder { struct ReviewInner {} } }
protocol ReviewP {}
extension ReviewHolder.ReviewInner: ReviewP { func reviewMethod() {} }
```

The root `ReviewInner` correctly receives `implements ReviewP`, but the method
has no `relations.parent` or `relations.container`, and that root type's
`relations.members` omits it. The outline places the method under an unresolved
`[in type ReviewHolder.ReviewInner]` heading. It should belong to the file-level
nested type; the closure's separate type is out of view.

| Site / shape tested | Observed result | Expected result |
|---|---|---|
| Swift extension of a file-level nested type, plus a same-named closure-local outer type | Conformance attaches; method parent/container/membership disappear | Keep conformance and attach the method to the file-level nested type |
| Swift extension through three type segments, plus the same closure shadow | Same missing ownership | Same correct file-level owner |
| Swift direct file-level type extension plus a same-named closure-local type | Correct file-level ownership | Observed result; clean one-segment control |
| Two-/three-segment extension with no shadow, a differently named closure owner, or a same-named owner in a named function | Correct file-level ownership | Preserve all three controls at each depth |
| One-/two-/three-segment extension whose only potential owner is closure-local | No parent/container fabricated | Observed result; preserve the absent-owner repair |
| Swift extension owner and conformance lookup on the same nested-shadow sources | Correct owner/conformance IDs | Preserve this clean consumer while repairing member linking |

#### Other readers and consumers swept

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust supertraits and impl self/trait consumers | Named/local-module paths, type/value separation, qualified aliases, and all registered ownership/widening rows | Correct targets or text-only aliases; local-module ownership retained | Clean for these forms; include them in the shared regression corpus |
| TypeScript bases | Namespace interface beside a same-named value; namespace alias/import target; registered three-segment paths | Interface ID for type lookup; alias/import stays text | Clean |
| JavaScript bases | Assigned class property used as `Holder.Base` | Text only | Clean; property-initializer type inference remains outside scope |
| Java and C++ bases | Nested class beside a same-named value; C++ nested type alias | Correct class ID; alias stays text | Clean name-group controls |
| C# bases | Nested type and qualified namespace alias/import controls | Correct type ID or unresolved/unclassified text | Clean |
| Swift bases | Nested class beside a same-named value; nested alias; registered mixed-list/built-in controls | Correct class ID; alias unresolved/unclassified | Clean base lookup; member projection is defective as above |
| Scala bases | Object's nested class beside a same-named value, nested alias, import-only target | Class ID for type lookup; alias/import unclassified text | Clean |
| Go and PHP bases | Local interface embed and namespace-relative superclass; registered import/namespace controls | Correct local IDs; unproved imports remain textual | Clean; no Python-style nested mutable class namespace in these source forms |
| C, Perl, Bash, Zsh, Lua | Registered absent-inheritance rows and primary fixtures | No required relation invented | Clean; no applicable declared-base reader |
| Complete/selective analysis and persistent-cache projection | All Python source rows above | Same false target IDs | Correct binding decision on every path |
| Complete hit and declarations-to-complete widening | Python import and Swift nested-shadow cases | Actual hits/widening retain the incorrect facts | Correct facts after analysis, widening, and service |
| Outline consumer | Python import and Swift nested-shadow cases | Wrong target identity is serialized; Swift method appears under an unresolved owner heading | Correct upstream identity and membership; keep existing rendering rules |

**Required repair:** use binding evidence at every path segment, in both named
and syntax-owner continuation. Preserve redirected declaration placement when
checking membership. Track explicit qualified Python writes when their owner
path is proved. Hide Swift descendants by their actual source owner, rather
than by a qualified-name prefix shared with a different declaration.

Extend the tree-hugger library's
[shared relation matrix](../../lib/tests/l1/symbols_relations.rs) and
[owner-identity matrix](../../lib/tests/l1/symbols_owner_identity.rs) with every
failing shape and clean control above. Assert exact IDs and declaration
positions, not only names. Reuse rows for complete/selective queries, actual
warm hits, narrowing, and widening; assert both Swift target conformance and
member parent/container/membership. Extend the CLI outline ownership checks.
Update behavior-linked comments, current relation docs, and the skill alongside
the repair, and bump the extractor revision to invalidate wrong stored facts.

### Medium — Relation processing exceeds the bounded per-file work contract

**Defect class:** an index is built once, but each lookup rescans an input-sized
name bucket, and inline namespaces eagerly materialize combinations of aliases,
so a single analysis no longer performs bounded work per declaration/path.

In the tree-hugger library,
[owner_binds](../../lib/src/file/tree_file/relations.rs:613), which checks
parameters of owners without records, scans every same-named owner's range
for each unrelated lookup. [members](../../lib/src/file/tree_file/relations.rs:656)
scans every same-displayed-name member, then walks ancestors to recover the
selected syntax owner. Repeated sibling owners therefore recreate the
per-relation scanning that the specification explicitly excludes.

These are declaration-only public API measurements over copied fixtures.
`n` is the number of appended repetitions. Every local-owner row returns the
correct `n` new target IDs, so faster growth is not caused by extra output.

| Site / shape | Steps at n=200 | Steps at n=400 | Expected result |
|---|---:|---:|---|
| Rust record-less impl parameters: repeat `impl<ReviewBase> ReviewHolder<ReviewBase> {}` followed by an unrelated `trait ReviewChildN: ReviewBase {}` | 43,656 | 166,656 | Work approximately doubles; outside traits retain the file-level Base ID |
| TypeScript record-less signature parameters: repeat `declare function fN<ReviewBase>(): void` followed by an unrelated interface extending ReviewBase | 41,632 | 163,032 | Work approximately doubles; interfaces retain the file-level Base ID |
| C++ sibling blocks with same-named Holder/Base and Child deriving from `Holder::Base` | 124,273 | 488,273 | Work approximately doubles; keep each selected owner's Base ID |
| Rust sibling blocks with same-named local modules and supertrait paths | 85,250 | 329,850 | Same |
| Java sibling blocks with same-named local classes and nested-class superclass paths | 82,800 | 325,400 | Same |
| Swift sibling `do` blocks with same-named local classes and nested-class base paths | 82,763 | 325,363 | Same |
| Scala sibling blocks with same-named local objects and nested-class base paths | 82,429 | 324,629 | Same |
| Python repeated function definitions, each with a Holder/Base and Child using `Holder.Base` | 82,918 | 325,718 | Same; separate function syntax scopes must have separately indexed members |

A compact C++ generator for the third row is:

```python
source = "void reviewProbe() {\n" + (
    "{ struct ReviewHolder { struct ReviewBase {}; }; "
    "struct ReviewChild : ReviewHolder::ReviewBase {}; }\n"
) * n + "}\n"
```

Append it to the C++ fixture, parse with `TreeFile::new`, snapshot
`work_counters()`, query with `SymbolQuery::new`, then compare
`work_counters().since(before).shared_context_steps` at the two sizes.
The analogous Rust row repeats an impl with the same parameter name and a
separately named trait. Existing
[linear-growth tests](../../lib/tests/l1/symbols_relations.rs:2935) use distinct
owner names and do not exercise these buckets.

#### Inline namespaces and remaining processing sites

The tree-hugger library's
[inline_names](../../lib/src/file/tree_file/relations.rs:341), which makes
C++ inline-namespace members reachable from the enclosing namespace,
materializes every combination of omitted inline segments. With `d` nested
inline namespaces, one Base alone obtains `2^d` indexed names. This happens
before any relation needs those variants. Its `found.contains` comparisons
add another avoidable scan. The counter increments once per record, hiding
most of that work.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| C++ inline alias construction | 5, 10, 12, 14 nested inline namespaces around one Base, followed by Child deriving from bare Base | Correct ID; 32, 1,024, 4,096, 16,384 Base-name variants. Query times approximately 11, 18, 58, 571 ms while counted steps are only 374, 389, 395, 401 | Resolve the written path without materializing every omitted-segment combination; counters account for actual lookup work |
| Named-scope traversal: `named` / `is_traversable` | Reopen one C++/TypeScript namespace 200/400 times and use qualified local bases | Correct IDs; `is_traversable` repeatedly collects the entire namespace bucket; candidate visits are uncounted | Derive merged-namespace traversability/uniqueness once per indexed name/group |
| Base-text deduplication in the source reader and record linker | One Rust trait with 200/400 distinct supertraits; 200/400 distinct positive trait impls on one self type | Correct lists; both [source-list deduplication](../../lib/src/file/extract/relations.rs:226) and [record-list deduplication](../../lib/src/file/tree_file/relations.rs:1121) linearly search the growing list for every insertion; comparisons are uncounted | Retain source order and exact-expression deduplication with bounded membership checks |
| First-segment lexical lookup, ordinary distinct-name bases, light-declaration collection, and one relation walk | Existing query-count and five linear-growth shapes | Passing work limits; no additional full-tree walk | Preserve these clean controls |
| Generic/modifier/attribute/metric projection and warm service | Existing feature-wide cache/query corpus | Correct projection, real hits, no unnecessary optional passes | Preserve; they are not a reason to redo source lookup on a full hit |
| C#, Go, PHP and JavaScript | Existing declared-base matrices, plus the applicable named/alias controls above | Correct results; no equivalent local nested-type member path in the tested supported source forms | Keep normal indexed lookup; shared named-list decisions still need bounded summaries |
| C, Perl, Bash, Zsh, Lua | No declared-base reader | No corresponding processing | No new work |

The namespace and deduplication rows are source-inspection findings paired
with public-result controls, rather than claims that today's counter measures
their loops. The inline timings are observations from a debug build; the
combinatorial name count is the structural defect. A small valid source can
already consume disproportionate time, and deeper nesting makes it worse.

**Required repair:** index local members by syntax owner plus name/group,
index record-less bindings by owning scope, and precompute named-scope
uniqueness/traversability. Represent inline-namespace visibility without
expanding all combinations. Use ordered output plus membership sets for
written-target deduplication. These are local index repairs; no compiler,
new capability, or public schema change is needed.

Extend permanent work-count tests with **all eight repeated-name shapes**,
reopened namespaces, multiple bases/impl aggregation, and nested inline
namespaces. Count candidate visits and alias/deduplication work so a linear
record counter cannot conceal expensive processing. Assert expected IDs and
source order alongside the budget. Avoid wall-clock thresholds. Review the
bounded-work claims in comments/docs and retain one definitions query and one
parse per analysis.

## Blocked Findings

None. Both findings are within the accepted source-extraction and performance
contract. No human decision or cross-OS proof is needed to repair them.

## Recurrence

The high finding repeats the lookup-evidence class of:

- Review #1, **Relation IDs are assigned without proving path meaning**:
  displayed names remain insufficient proof for intermediate bindings.
- Review #2, **Python nested class scopes still assign incorrect base identities**:
  class namespace contents must come from the actual binding scope.
- Review #3, **Relation lookup still drops import and generic-shadowing evidence**:
  correctly collected imports must affect later segments too.
- Review #4, **Relation lookup still substitutes record names for source binding evidence**:
  bindings without records still disappear from qualified member lookup.
- Review #5, **Relation lookup loses binding evidence between syntax scopes and display-name indexes**:
  its syntax-owner continuation fixes sibling-owner identity, but not that
  owner's imports/writes/directive placement; its Swift out-of-view filter
  introduces another display-name substitution when hiding descendants.

Those repairs should have swept every segment's recorded and unrecorded
bindings, redirected class declarations as qualified members, and Swift's
same-display-name **visible plus invisible** owner pairs. Their listed cases
are repaired; these sibling transitions were missed. The complete tested
instance/control lists and consumer checks are above, so `recurrence` is true.

The medium performance finding is a new reported class. Earlier reviews asked
for bounded per-file work, but did not report these growth failures. The earlier
compound-cache-field testing class remains closed for this feature's fields.

## Input Robustness Matrix

The following format-wide matrix was rechecked in the passing registered CLI
tests. The latest repair adds no serialized field or new input format.
The source-binding defects above are independent of cache decoding.


JSON persistent cache is the only affected serialized input format. This
feature changes no manifest, lockfile or configuration reader; there is no
YAML/TOML input side. The registered CLI tests start from real fixture output,
edit one field per cell, retain a positive control and service sentinel, and
assert public output or equality with fresh extraction after rejection.

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

| User-facing requirement | Appropriate level | Strongest evidence and assessment |
|---|---|---|
| Explicit generic ownership, order, bounds/defaults/variance, overflow | L1 public API | Per-language, complete/selective, boundary, and warm/projection matrices pass. Repeated record-less parameters expose the separate work-growth finding. |
| Complete base text, source order, deduplication, exact local target identity, mixed-base categories | L1 public API/CLI | Registered matrices pass; supplemental Python qualified-member cases produce false IDs. Blocks readiness. |
| Rust empty/multiple/negative impls, type/member trait facts, self ownership and aggregation | L1 public API/CLI | Registered method-only, local-module, selective, containment-widening and actual-hit tests pass. Repeated ownership/name buckets exceed the work contract. |
| Swift same-file extension conformance and membership | L1 public API/CLI | Conformance controls pass; same-named closure-shadow cases lose valid nested-owner membership and produce an unresolved outline heading. Blocks readiness. |
| Raw attributes, repetitions, empty arguments, declaration-only modifiers, no body/string leakage | L1 public API/CLI | Language matrices and all attribute/modifier cache member columns pass. |
| Callable body lines, absent/null semantics, expression/empty/nested/absent bodies, LF/CRLF and column-1 endpoint | L1 public API | Body-line matrix, arithmetic boundaries, requested-facet keys and projections pass. |
| Selective analysis avoids optional passes and bounds shared source work | L1 work-counter assertions | One-parse/query/link and optional-pass controls pass. Supplemental repeated-name growth and inline expansion fail the required work limit; the counters omit some loops. |
| Cache hit, narrowing, widening, exact schema/revision invalidation | L1 isolated API/CLI | Permanent coverage, decoder, upgrade and projection matrices pass. Actual supplemental hits retain wrong upstream facts; they are real hits, not silent recomputation. |
| Outline placement, written base labels, signature suppression, sorting, wrapping, plain output, unchanged counts/stats | L1 CLI plus L2 real-terminal capture | Outline/golden tests and tmux/WezTerm capture pass for their fixtures. Swift's additional source case fails ownership/placement at L1; existing L2 covers the rendering mechanics. |
| Keyboard/input-encoder behavior | L3 if required | This feature makes no such promise; no missing L3 requirement. |
| Current schema/cache/report/API docs and older-model adapter losses | Source/doc inspection plus L1 schema/adapter tests | Versions and documented losses agree. Correct binding and bounded-work claims must be maintained with the repairs. |

The library modules are compiled by the declared `tests/l1/main.rs` target
(`autotests = false`); CLI matrices are included by `tests/symbols/main.rs`.
Their test names select L1. Both terminal targets require `terminal-tests`,
the live L2 recipe enables it, and package CI metadata includes it. The tier
audit confirms no stranded tests. This review adds no permanent tests.

Implementation requires the two repairs above before another readiness
assessment. The feature directory stays active for the author's review-cycle
closure.
