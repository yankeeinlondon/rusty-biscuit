---
$schema: feature-review.yaml
ready: false
findings:
  - title: Swift nested-type extensions lose enclosing generic bindings
    priority: high
  - title: Inline namespace and generic processing still repeat input-sized scans
    priority: medium
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-06T03:09:51-07:00
spec: 2026-10-06-symbol-type-relations/spec.md
implemented: true
implemented_by: codex
log: 2026-10-06-symbol-type-relations/log.md
description: "A **feature** review of `2026-10-06-symbol-type-relations/spec.md`"
feature: 2026-10-06-symbol-type-relations/review-7.md
previous: 2026-10-06-symbol-type-relations/review-6.md
next: 2026-10-06-symbol-type-relations/review-8.md
---

# Review of Symbol Type Relations — Iteration 7

The feature is **not production ready**. Review #6's listed correctness and
performance instances are repaired, but the class sweep finds two remaining
problems: Swift extensions lose generic bindings from the extended type's
enclosing declarations, and several source readers still repeatedly scan
collections that grow with their input. Both repairs fit the accepted design;
neither requires a human decision.

## Previous findings and blocked work

Read the specification, implementation log, and all six preceding reviews.

| Previous finding | Assessment |
|---|---|
| Review #6, “Relation lookup still loses binding and owner evidence after selecting the first name” | Its enumerated Python member imports, writes, deletions, patterns and directive destinations now stay unresolved. Swift nested extensions beside closure shadows retain the file-level owner's membership. Registered complete/selective, widening, actual-hit and outline regressions pass. Additional Python siblings below are clean. Enclosing generic bindings in nested Swift extensions remain defective. |
| Review #6, “Relation processing exceeds the bounded per-file work contract” | Its eight repeated-owner shapes, reopened namespaces, written-base deduplication and omitted-segment inline-depth cases now pass their work budgets. The new owner/name indexes implement the requested repairs. Broad inline namespaces, fully written deep paths, generic extension contexts and separately written generic bounds were not adequately swept; see the medium finding. |
| Reviews #1–#5, path identity, lexical scope, name groups, imports, generic shadowing, directive destinations and built-in exclusions | Registered regressions pass. The supplementary eight-language generic-shadowing controls pass. The new Swift cases are additional instances of the same lost-binding class. |
| Review #1, template-template defaults and callable requirements; review #3, C# record classification and current schema examples | Repairs remain in place and their registered tests pass. Current schema examples identify schema 5.0 and historical versions appropriately. |
| Reviews #2–#4, permanent generic/base/span/attribute/modifier cache robustness columns | All registered public-output matrices pass. No additional decoding finding. |

Review #6 has no blocked findings. The older mixed-base work was unblocked
before implementation: the author accepted precise `extends` and `implements`
lists, a separate `unclassified_bases` list, and Swift extension conformance
attached only to a uniquely resolved same-file owner. Those readers are built;
the accepted decision does not need reaffirmation.

## Verification and scope

Used the tree-hugger, Rust, rust-testing and biscuit-test-harness skills.
Reviewed all 16 language acceptance rows, generic ownership and bounds,
relation readers and lookup consumers, Rust impl ownership, Swift extensions,
modifiers, attributes, callable body lines, selective work, cache decoding and
projection, outline output, documentation, and test target/tier registration.

- `just test`: **1,219 passed, zero skipped**, 56.455 seconds.
- `just lint`: passed for both packages.
- `just check-tier-coverage tree-hugger`: zero stranded tests.
- `just test-l2`: six selected entry points passed, 24.367 seconds. tmux and
  WezTerm supplied real-terminal rendering evidence. Kitty's entry points
  returned through their unavailable-backend gates; they supply no Kitty
  rendering evidence. The recipe used background resources without keyboard
  injection or a request to focus a window.
- Supplemental shipped-CLI checks copied primary language fixtures into
  isolated temporary Git repositories with separate home/config/cache
  directories. **76 source rows** each had uncached complete output,
  uncached type-only output, a cold complete run and a genuine warm run
  reporting `1 hits` and `parsed 0 files`. No CLI invocation failed and no
  complete result reported diagnostics. The rows include 18 additional Python
  binding cases, 49 Swift cases, and nine controls covering the other eight
  languages with explicit generics.
- Public `SymbolCache::query` checks for three Swift failures compared each
  served index with fresh analysis through declarations, containment widening
  without a parse, complete widening with a parse, a complete `Hit`, and a
  type-only `Hit`. All preserve the incorrect upstream facts.
- Plain outline checks on the same three Swift sources retain correct nested
  placement, print the incorrect `extends ReviewBase` category, retain the
  conformance label whose JSON target ID is wrong, and omit the required
  textual Int base.
- Supplemental `TreeFile::query(&SymbolQuery::new())` measurements used copied
  fixtures, generated repeated shapes, and `shared_context_steps`. Parsing
  occurred before measurement; exact base IDs, ambiguous results, relation
  order/count and generic-parameter/bound counts were checked. Timings below
  are debug-build observations, not proposed test thresholds. Temporary probe
  source was removed after use; this review adds no permanent Rust tests.

Concurrent reference-report and cyclomatic-metric work is present in the
checkout. Its independent behavior is outside this feature's scope. Cross-OS
execution evidence is left to CI and does not decide readiness. Existing
implementation changes were preserved.

## Unblocked Findings

### High — Swift nested-type extensions lose enclosing generic bindings

**Defect class:** when a declaration's lookup context is reconstructed through
a separately written owner, only that owner's own generic bindings are restored,
so bindings from its enclosing declarations disappear before relation and
built-in decisions.

The tree-hugger library's
[type_parameters_around](../../lib/src/file/tree_file/relations.rs:1238), which
collects generic names visible to relation lookup, substitutes the resolved
owner when walking a Swift extension. It reads that owner's own parameter
list, then continues up the extension's source ancestors. A file-level
extension has none of the nested owner's enclosing type declarations among
those ancestors. The tree-hugger library's
[swift_extensions](../../lib/src/file/tree_file/relations.rs:1575), which adds
extension conformances to a same-file type, likewise supplies only that
type's own list. Both lose the outer type's parameters.

Append this to a copy of the Swift fixture:

```swift
class ReviewBase {}
struct ReviewOuter<ReviewBase> { struct ReviewInner {} }
extension ReviewOuter.ReviewInner {
    class ReviewChild: ReviewBase {}
}
```

```sh
hug --json-full symbols --no-cache probe.swift
hug --json types --all-scopes --no-cache probe.swift
hug --verbose --json-full symbols probe.swift
hug --verbose --json-full symbols probe.swift
```

Every result assigns the file-level class's ID and qualified name to
`ReviewChild.relations.extends`. The written name denotes the outer generic
parameter, so it must remain textual in `unclassified_bases`, with no ID or
qualified name. `swiftc -typecheck` confirms that binding: it diagnoses
inheritance from the non-class type parameter, rather than treating the
file-level class as the base. These are deliberately negative source cases;
Tree-sitter parses them cleanly, and the specification explicitly forbids
resolving a type parameter to an unrelated declaration even in such input.

#### Owner-context and consumer sweep

“Text only” below means the written base survives with neither ID nor target
qualified name. All shipped-CLI rows ran through the four paths described above.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Swift declaration bases inside a nested-type extension | Parameter belongs to outer type; two- and three-segment extended owner | File-level Base ID, classified as `extends` | Text only in `unclassified_bases` |
| Same reader | Parameter belongs to middle type of a three-segment owner | File-level Base ID | Text only in `unclassified_bases` |
| Same reader, qualified base | Outer parameter `ReviewHolder`; base `ReviewHolder.ReviewBase` beside a real file-level Holder.Base | Unrelated Holder.Base ID | Text only in `unclassified_bases`; the first segment is the parameter |
| Same reader in an extension method and in its closure | Outer parameter named ReviewBase | File-level Base ID in both | Text only in `unclassified_bases` |
| Swift extension conformance aggregation | `protocol ReviewBase {}; struct ReviewOuter<ReviewBase> { struct ReviewChild {} }; extension ReviewOuter.ReviewChild: ReviewBase {}` | File-level protocol ID in Child's `implements` | Text-only `implements`; extension syntax still establishes the category |
| Swift built-in exclusion in the nested extension | Outer parameter with each of `Int`, `Int8`, `Int16`, `Int32`, `Int64`, `UInt`, `UInt8`, `UInt16`, `UInt32`, `UInt64`, `String`, `Character`, `Float`, `Double`; enum's first base is that name | Drops the relation in every case | Keep each written name in `unclassified_bases` |
| Same exclusion, qualified spellings | Outer parameter `Swift`; every `Swift.`-qualified spelling above and `Swift.Unicode.Scalar`; outer parameter `Unicode` with `Unicode.Scalar` | Drops all 16 qualified forms | Keep their text in `unclassified_bases`; the qualifier is a parameter |
| Swift direct generic-owner extension; nested owner with its own generic parameter | Base on a child declared in the extension | Text-only unclassified base | Clean; retain |
| Swift direct and nested extension conformance, owner's own parameter | Extension base named like a file-level protocol | Text-only `implements` | Clean; retain |
| Swift ordinary nested body, without a separately written extension | Outer generic parameter named ReviewBase | Text-only unclassified base | Clean; syntax ancestors retain the context |
| Swift nongeneric nested extension | Child base / extension conformance names a file-level class / protocol | Correct declaration IDs | Clean; retain positive resolution |
| Swift nearer local class inside the extension method | Local ReviewBase beside outer parameter and file-level Base | Correct function-local Base ID | Clean; a nearer lexical declaration must retain precedence |
| Swift built-in controls | Extended type's own parameter Int; no parameter Int | Keeps textual Int / drops unshadowed built-in | Clean; retain both outcomes |
| Swift extension header's ordinary nested name | Protocol declared only inside Outer, bare ReviewBase in extension's header | Text only | Clean; `swiftc` reports the bare name unavailable there. Do not restore all owner members as lexical header names while restoring generic parameters |
| Complete/selective results, cache widening and projection | Wrong-ID, wrong-conformance-ID and dropped-Int cases | Same errors after widening, complete hit and narrowed hit | Correct facts on every path; no silent cache recomputation |
| Containment and outline consumers | Nested extension sources above | Parent/container identify the intended Inner, but the serialized base is wrong or missing | Preserve correct ownership; show the corrected textual base/category |

The generic-owner sibling sweep used the same colliding parameter/base shape
against every other applicable language reader:

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust trait/impl owner lookup | Generic trait's callable declares a child trait; registered enclosing-impl rows | Text-only supertrait / self type remains unresolved | Clean |
| TypeScript class implementation | Callable inside a generic class declares the child | Text-only `implements` | Clean |
| Go interface embedding | Generic callable's local interface embeds the parameter | Text-only `extends` | Clean |
| Python class bases | Two nested class bodies inside a generic class | Text-only `extends` | Clean |
| Java superclass | Two nested classes inside a generic class | Text-only `extends` | Clean |
| C# mixed base list | Two nested classes inside a generic class | Text-only unclassified base | Clean |
| C++ base list | Nested generic owner; separately written template member definition | Text-only `extends` in both | Clean |
| Scala parent list | Two nested classes inside a generic class | Text-only unclassified base | Clean |
| JavaScript, C, PHP, Perl, Bash, Zsh, Lua | Registered absence-of-explicit-generics rows | No invented generic parameter context | Clean; no applicable explicit-generic owner restoration |
| Python named and syntax-owner continuation | Dictionary-rest, class-keyword, sequence-star, alternative and `as` match captures; qualified `for`, nested-pattern, `with` and annotated-assignment writes, each at module and function scope | All 18 cases remain text-only | Clean; retain the review #6 repair |
| Shared import, alias, root and lexical decisions | Registered path-meaning and owner-identity matrices across all supported relation readers | Passing exact-target and absent-owner controls | Retain; no new failure found there |

**Required repair:** recover the resolved Swift owner's generic layers through
its actual declaration ancestors, including every enclosing type, and share
them by owner identity. Use that context for both extension-body lookup and
extension conformance aggregation. Preserve lexical precedence, unnumbered
oversized parameter names, textual unresolved targets, and built-in exclusions
when no binding exists. Do not copy enclosing parameters into the child's
serialized `generic_params` or make enclosing ordinary members visible in an
extension header.

Extend the tree-hugger library's
[shared relation matrix](../../lib/tests/l1/symbols_relations.rs) with every
failing and clean shape above, including all 30 built-in spellings/qualifiers.
Reuse those rows in complete/selective queries, projection, widening and
actual-hit checks. Assert exact target IDs or their absence, relation category,
written text and absence of target qualified names. Include outline checks for
the corrected category and retained raw-looking base. Update the relevant
comments, current relation docs and skill, and bump the extractor revision so
warm entries do not retain these wrong facts.

### Medium — Inline namespace and generic processing still repeat input-sized scans

**Defect class:** source lookups still traverse or rebuild a collection that
grows with the input for each declaration or requirement, making analysis grow
quadratically or worse while some of the repeated work is absent from counters.

The tree-hugger library's
[denoted](../../lib/src/file/tree_file/relations.rs:489), which finds C++ names
reachable through inline namespaces, removed exponential alias construction.
However, each lookup segment traverses every inline descendant of each current
namespace again. Its callers restart this work for prefixes and subsequent
lookups. A file with many sibling inline namespaces therefore repeatedly scans
the file-sized namespace set, even with unique member names or every namespace
segment written explicitly. Gathering ambiguous candidates also retains a
growing `found.contains` scan in
[Few::collect](../../lib/src/file/tree_file/relations.rs:306).

The tree-hugger library's
[swift_extensions](../../lib/src/file/tree_file/relations.rs:1587) reparses the
extended owner's complete generic list and creates a new name set for every
extension. The extension branch of
[type_parameters_around](../../lib/src/file/tree_file/relations.rs:1264) keys
its memoized layer on each extension node rather than the shared owner.
This does not meet the adjacent comment's “Each owner's list is read once”
contract. Finally, the tree-hugger library's
[attach](../../lib/src/file/extract/generics.rs:179), which places separately
written bounds on their generic parameter, searches the whole parameter list
for every clause in Rust, C# and Swift.

#### Inline lookup measurements and sibling callers

Append the generated equivalent of this shape to the C++ fixture, once for
each index `i`, then query declarations through the public API:

```cpp
namespace ReviewNs {
    inline namespace v0 { struct ReviewBase0 {}; }
    inline namespace v1 { struct ReviewBase1 {}; }
    // Continue through v(n-1).
}
struct ReviewChild0 : ReviewNs::ReviewBase0 {};
struct ReviewChild1 : ReviewNs::ReviewBase1 {};
// Continue through ReviewChild(n-1).
```

All outputs have the correct exact IDs. The problem is work, not relation
count or correctness. The table uses counted steps; expected growth is
approximately proportional to declarations and written path length.

| Site / shape tested | Steps at n=200 | Steps at n=400 | Observed / expected result |
|---|---:|---:|---|
| `named`, `is_bound`, `is_traversable`: distinct injected bases, `ReviewNs::ReviewBaseN` | 45,557 | 170,757 | Correct n IDs; repeated sibling traversal. Expected roughly twice the work |
| Same lookup, rooted `::ReviewNs::ReviewBaseN` | 44,957 | 169,557 | Correct n IDs; anchoring does not remove repeated traversal |
| `scope_flags` / first-segment lookup: bare ReviewBaseN inside ReviewNs | 85,357 | 330,357 | Correct n IDs; same defect in unqualified lookup |
| Explicit `ReviewNs::vN::ReviewBaseN` | 126,957 | 493,557 | Correct n IDs; writing every segment makes the repeated work larger |
| Ambiguous injected name: every inline namespace declares ReviewBase | 85,357 | 330,357 | Correct text-only results; candidate `contains` comparisons add uncounted work |
| Matched ordinary-namespace control: remove `inline`, use explicit vN path | 4,957 | 9,557 | Correct n IDs and bounded counted work; clean |
| `named_member_rebound` / `each_summary` / `denoted` | The same qualified/rooted/bare inputs | The measurements above | These consumers repeatedly invoke the same traversal; include all in the repair, rather than optimizing only final candidate collection |
| Fully written nested path | Depth 4: 505; depth 8: 967 | Depth 16: 3,923 | Correct Base ID; doubling depth 8→16 exceeds the current inline-depth test's growth bound |
| Outer-only / innermost-only nested spellings | Registered depths 4/8/16 | Passing | Clean after review #6; the test omits the fully written path |
| Non-C++ readers, syntax-owner member index, record-less-owner bindings, reopened namespaces and both base deduplicators | Registered repeated-name/work matrices | Passing | Clean controls; preserve the prior repair |

`may_bind` has no C++ built-in exclusion caller, and Python-only qualified-write
resolution does not traverse inline namespaces. Rust impl and Swift extension
resolution use the same dispatch but have no C++ inline namespace tree. These
are not additional inline-scan sites.

#### Generic-reader sweep

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Swift extension conformance context | One owner with n generic parameters; n distinct empty protocols and n conforming extensions | Correct n ordered target IDs. At n=200/400/800: 3,242/6,242/12,242 counted steps, but the owner list is read and rebuilt n times, at least n² parameter visits; debug query time approximately 122/414/1,508 ms | Read/share the owner's context once; count parameter visits and set construction |
| Swift extension-body context | n extensions of one owner with n parameters, each declaring a child extending the file-level Base | Correct n exact IDs. At n=200/400/800: 3,846/7,446/14,646 counted steps, but a separate full layer is built under each extension node; debug query time approximately 220/780/2,934 ms | Cache the layer under owner identity, retain enclosing layers, count the work and reuse it across extension bodies and conformances |
| Rust separately written bounds | n parameters and `where T0: Clone, …, T(n-1): Clone` | Public queries at 2,000/4,000/8,000 retain all parameters with one bound each; every reader invocation searches 1+…+n entries through `attach` | Build subject-name→parameter-position lookup once and preserve bound order |
| C# separately written bounds | n parameters and `where T0: class … where T(n-1): class` | Same correct public parameter/bound counts; same n(n+1)/2 comparisons per reader invocation | Same bounded subject lookup |
| Swift separately written bounds | n parameters and `where T0: ReviewP, …, T(n-1): ReviewP`, tested after the list and inside the angle brackets | Both public forms retain all 2,000/4,000/8,000 parameters with one bound each; same repeated subject scan through `attach` | Same bounded lookup for both clause locations |
| Rust/C#/Swift work accounting | The three bound-list shapes above | Counters omit `attach` comparisons. Swift reports 246 shared-context steps at all three sizes despite 2,001,000 / 8,002,000 / 32,004,000 subject comparisons in one reader invocation | Count the repeated work so a passing linear-counter test cannot hide it |
| TypeScript, Go, Python, Java, C++, Scala generic readers | Registered bound/default/requirement matrices and source inspection | Inline bounds are attached while reading their own parameter; no equivalent separately-written subject scan | Clean for this defect class; keep their existing ownership and defaults |
| Ordinary named-owner context layers | Registered generic-owner work matrix and nested-owner controls | Owner lists shared across ordinary descendants | Clean; extend that reuse to separately written Swift extensions |
| Generic-free languages | JavaScript, C, PHP, Perl, Bash, Zsh, Lua matrices | No structured generic list or subject attachment | No applicable scan |

The generic-bound comparison totals describe the loop in one reader invocation,
not a claim that today's public counter measures it. The supplementary public
results establish that these populated lists are load-bearing. Timings alone
are not the finding: the repeated input-sized traversal is visible in the code.

**Required repair:** index effective inline-namespace member/segment lookup
once, retaining ambiguity and namespace merging without enumerating all full
omitted-segment spellings. Bound candidate summaries need not gather every
candidate after ambiguity is established. Share Swift generic contexts by the
actual owner and its enclosing declarations. Index generic-bound subjects once
per parameter list in all three `attach` consumers. Keep written ordering,
deduplication, generic positions and conservative target decisions unchanged.

Extend the tree-hugger library's
[relation work tests](../../lib/tests/l1/symbols_relation_work.rs) with every
inline shape above and both Swift extension consumers. Add counted growth
checks for Rust/C#/Swift separately written bounds, including Swift's two
clause locations. Counters must include candidate comparisons and parameter
list/set work. Assert exact results alongside growth bounds and retain one
definitions query and at most one parse. Update behavior-linked comments,
current docs and skill; do not replace these checks with wall-clock thresholds
or relax the work budget.

## Blocked Findings

None. Both findings have concrete repairs within the agreed contract.

## Recurrence

The high finding repeats the missing-binding-evidence class in:

- Review #1, “Relation IDs are assigned without proving path meaning.”
- Review #2, “Python nested class scopes still assign incorrect base identities.”
- Review #3, “Relation lookup still drops import and generic-shadowing evidence,”
  and “Built-in name exclusions erase valid Go and Swift bases.”
- Review #4, “Relation lookup still substitutes record names for source binding evidence.”
- Review #5, “Relation lookup loses binding evidence between syntax scopes and display-name indexes.”
- Review #6, “Relation lookup still loses binding and owner evidence after selecting the first name.”

Those fixes should have swept both consumers of a separately written Swift
owner's generic context: extension-body relation lookup and extension
conformance aggregation, then the built-in exclusion that reads the same
context. A direct generic owner was covered; enclosing generic owners of a
nested extended type were not. This review enumerates both consumers, bare
and qualified bases, nested callable/closure sites, all 30 built-in forms,
and the clean same-class readers and cache consumers.

The medium finding repeats review #6's “Relation processing exceeds the bounded
per-file work contract.” Removing exponential inline aliases should also have
swept wide sibling namespaces, explicit and rooted spellings, unqualified
lookup, ambiguous candidate collection, and fully written deep paths. Sharing
generic layers should have included Swift's separately written owners and
separately written parameter-bound attachment in Rust/C#/Swift. Those sites
are all enumerated in this review. `recurrence: true` records both classes;
the earlier cache-decoder and schema-documentation findings remain closed.

## Input Robustness Matrix

The following format-wide matrix was rechecked in the passing registered CLI
tests. The review #6 repair adds no serialized field or new input format.
The Swift binding defect above is independent of cache decoding.


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

| Requirement | Appropriate level | Strongest evidence and assessment |
|---|---|---|
| Explicit generics, ownership, order, bounds/defaults/variance, overflow | L1 public API | Language, boundary, complete/selective and cache matrices pass. Separately written bound attachment has the work defect above; no additional generic-value defect found. |
| Written base text, order, deduplication, exact local identities and mixed categories | L1 public API/CLI | Registered matrices pass. Nested Swift extension context yields false IDs/categories and loses written bases. Blocks readiness. |
| Rust empty/multiple/negative impls, type/member conformance and self ownership | L1 public API/CLI | Method-only, lexical/module, aggregation, containment-widening and actual-hit regressions pass. |
| Swift extension conformance and containment | L1 public API/CLI | Closure-shadow membership repair passes. Nested generic owner context remains incomplete in both conformance and body lookup. Blocks readiness. |
| Declaration-only modifiers and raw attributes, repetitions and empty arguments | L1 public API/CLI | Every language row, leakage/ownership controls, modifier ordering and all cache member columns pass. |
| Callable body lines, no-body/empty/expression/nested forms, LF/CRLF, column-1 endpoints | L1 public API | Body-line, arithmetic boundary, required metric key, query and projection checks pass. |
| One parse/definitions query, skipped optional passes, bounded shared work | L1 work counters | Existing controls pass; broad/deep inline lookup and generic scans fail the bounded-work contract or escape counters. Blocks readiness. |
| Actual cache hit, narrowing, widening and exact schema/revision invalidation | L1 isolated API/CLI | Decoder, projection and invalidation matrices pass. Supplemental real hits/widening faithfully preserve wrong Swift upstream facts; cache equivalence alone cannot prove source correctness. |
| Outline labels, signature suppression, placement, sorting, wrapping, styling and plain output | L1 CLI plus L2 real-terminal capture | Registered outline/golden checks and tmux/WezTerm capture pass. Wrong Swift relation fields can still change the heading or remove a required written base; rendering mechanics have appropriate L2 evidence. |
| Counts/stats and JSON/JSONL/full-index facts | L1 CLI | Report, format and golden tests pass. Supplemental JSON exposes the Swift defect. |
| Keyboard, paste, IME or mouse input behavior | L3 only for an encoder promise | This feature makes no input-event promise; no missing L3 requirement. |
| Current schema/cache/report/API docs and older-model adapter losses | Source/doc inspection plus L1 schema/adapter tests | Schema 5.0, extractor revision 34 and documented losses agree. Generic-context and bounded-work comments/docs need the repairs above. |

The library's `autotests = false` target declares the relevant modules in
[tests/l1/main.rs](../../lib/tests/l1/main.rs), including the new relation-work
module. CLI matrices are declared by
[tests/symbols/main.rs](../../cli/tests/symbols/main.rs). Their ordinary test
names select L1. Both terminal targets declare `required-features =
["terminal-tests"]`; the live L2 recipe and package CI metadata enable it.
The tier audit found no stranded test. Passing aggregate counts do not close
the supplemental failures listed above.

The feature remains active for repair and another review; moving it into the
completed lifecycle directory remains the author's responsibility.
