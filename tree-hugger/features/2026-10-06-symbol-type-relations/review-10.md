---
$schema: feature-review.yaml
ready: false
findings:
  - title: C# partial-owner merging ignores type identity
    priority: high
  - title: Aggregate declaration-fact tests exceed the normal execution budget
    priority: medium
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-06T13:34:47-07:00
spec: 2026-10-06-symbol-type-relations/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-06-symbol-type-relations/log.md
description: "A **feature** review of `2026-10-06-symbol-type-relations/spec.md`"
feature: 2026-10-06-symbol-type-relations/review-10.md
previous: 2026-10-06-symbol-type-relations/review-9.md
next: 2026-10-06-symbol-type-relations/review-11.md
---

# Review of Symbol Type Relations — Iteration 10

The feature is **not production ready**. Review 9's listed cases are repaired,
but its new C# partial-type exception merges declarations that do not identify
one type. This can assign a base's identity from another type, lose a valid
local base, or import another type's inheritance barrier. One high finding
covers the complete tested class; a medium finding covers repeated test-budget
failures across the aggregate verification consumers. No human decision is needed.

## Previous findings and blocked work

Reviewed the specification, accepted mixed-base classification choice, implementation
log, and findings and recurrence sections of reviews 1–9. The accepted behavior
retains precise `extends` and `implements`, stores uncertain written bases in
`unclassified_bases`, and attaches Swift extension facts only to a proved
same-file owner. That decision remains unchanged.

| Previous work | Assessment |
|---|---|
| Review 9: Binding and scope checks still combine distinct source owners | The enumerated repair is implemented: owner-specific scope facts, flags, member-rebinding checks and Swift raw-type checks now use source ancestry. The registered 44 source-owner cases and 30 Swift sibling-shadow spellings exercise complete/selective queries, method-only Rust impl consumers, containment widening and genuine cache hits. The remaining defect is in deciding which C# declarations qualify for the merge exception. |
| Review 9 blocked findings | None existed. |
| Earlier blocked C#/Swift/Scala readers | The author accepted the separate uncertain-base list before review 3. The readers and local Swift extension attachment are implemented; this choice needs no reaffirmation. |
| Earlier generics, attributes/modifiers, body lines, cache-field robustness, C# record categories, schema examples and bounded-work findings | Permanent regressions are registered. No new defect in these classes was found in this review. |

## Verification and scope

Used the tree-hugger, rust-testing and biscuit-test-harness skills. Inspected
source readers, scope summaries and owner lookup, relation consumers, query/cache
behavior, documentation, and test target/tier registration. Concurrent reference
and complexity work is outside this review's requirements. The current executable
reports schema 5.0 and extractor revision 38; review 9's repair itself bumped the
revision from 36 to 37.

- `just test`: stopped on a 30.029-second aggregate cache-test timeout;
  631 passed, one timed out, 603 did not run.
- `NEXTEST_TEST_THREADS=4 just test`: stopped on a 30.016-second aggregate
  query-test timeout; 713 passed, one timed out, 521 did not run. No full L1
  passing result was obtained in this review.
- One-worker aggregate checks passed: query equivalence (23.666 seconds in its
  isolated run, 13.034 in the later focused run), cache widening (18.983), warm
  projection (25.885), work counters (6.731), adapter loss (3.586), and old-schema
  rejection (0.193). These do not close the execution-budget finding.
- The focused run also caught two newly edited JavaScript/TypeScript
  class-expression test rows during concurrent work. Those rows changed while
  this review ran; the latest full path-meaning matrix rerun passed in 4.830
  seconds. This transient snapshot failure is not a third current finding.
- The remaining focused L1 checks (`binary(symbols)` plus the ordinary relation
  tests excluding already checked matrix/boundary cases): **235 passed**, zero
  failures, in 167.549 seconds. This includes the full CLI symbol target and
  cache robustness matrices.
- `just lint`: passed for both packages.
- `just check-tier-coverage tree-hugger`: zero stranded tests.
- `just test-l2`: six selected entry points passed in 22.700 seconds. tmux and
  WezTerm provided actual report-capture evidence; Kitty entry points used
  unavailable-backend gates and provide no Kitty evidence. Background harness
  resources were used without requesting focus or keyboard injection.
- Review frontmatter parsed successfully; scoped `git diff --check` passed.

Supplemental reproductions made one edit to the registered C# partial-owner
source shapes in the tree-hugger library's
[relation matrix](../../lib/tests/l1/symbols_relations.rs:3601), then ran the
shipped `hug` binary against scratch files. **67 source rows** were run through
uncached complete JSON, uncached selective `types` JSON, cache population and
warm complete JSON. Relation lists agreed across all four paths. Every warm
run reported one hit, zero misses and zero parses. These passing cache checks
also preserved the incorrect source facts below.

## Unblocked Findings

### High — C# partial-owner merging ignores type identity

**Defect class:** a merge exception based on spelling and a `partial` token
combines members and scope facts from declarations with different generic
arities (numbers of type parameters), declaration categories, or enclosing type identities.

In the tree-hugger library,
[is_csharp_partial](../../lib/src/file/extract/relations.rs:269) reads only the
modifier. The
[Declarations scope summary](../../lib/src/file/tree_file/relations.rs:911),
which supplies the decision about whether repeated declarations are one owner,
uses that boolean to clear `not_merging`. It checks neither generic arity nor
compatible declaration kind nor the actual enclosing owner. The
[traversal and anchoring decisions](../../lib/src/file/tree_file/relations.rs:1229)
therefore allow lookup through a combined name and bypass the source-specific
facts added in the previous repair.

This is observable on ordinary C# generic overloads of a type name. `H` and
`H<T>` are different types even when both are partial. Their private nested
members are not shared. The following edit to the registered positive control
still assigns `ReviewChild` the ID of the second declaration's `ReviewBase`:

```csharp
partial class ReviewHolder {
    class ReviewChild : ReviewBase {}
}
partial class ReviewHolder<T> {
    class ReviewBase {}
}
```

Expected: preserve `ReviewBase` as a text-only uncertain base; no local declaration
visible to the first owner proves its identity or category. Removing `<T>` is
the positive control and must retain the proved relation. With a `ReviewBase`
also in the first owner, the edit instead makes a proved relation uncertain.
With a file-level `ReviewBase`, adding `: External` only to `ReviewHolder<T>`
incorrectly blocks the first owner's outward lookup.

A compact reproduction uses that source saved as `case.cs`, with the working
directory set to its scratch directory:

```sh
hug --json-full symbols case.cs --no-cache
hug --json types case.cs --no-cache
hug --json-full --verbose symbols case.cs
hug --json-full --verbose symbols case.cs
```

The first command reports an `extends` target whose ID belongs to the member
on the second source line of the compact fixture. The final command reports
`cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files` and the same bad ID.

### Partial-owner instance sweep

All rows use `ReviewChild : ReviewBase` inside the first owner and put the sole
`ReviewBase` inside the second, unless the row says otherwise. “Borrowed ID”
means the second owner's nested class is incorrectly selected. “Text only”
means preserve the written target without an ID or qualified name, in
`unclassified_bases`. Each combined shape below was run separately.

| Site | Shapes tested | Observed result | Expected result |
|---|---|---|---|
| C# partial class | Owner arities 0/1, 0/2 and 1/2 | Borrowed ID, `extends` | Text only |
| C# partial struct | Owner arities 0/1, 0/2 and 1/2 | Borrowed ID, `extends` | Text only |
| C# partial interface | Owner arities 0/1, 0/2 and 1/2 | Borrowed ID, `extends` | Text only |
| C# partial record | Owner arities 0/1, 0/2 and 1/2 | Borrowed ID, `extends` | Text only |
| C# partial record struct | Owner arities 0/1, 0/2 and 1/2 | Borrowed ID, `extends` | Text only |
| C# explicit partial record class | Owner arities 1/2 | Borrowed ID, `extends` | Text only |
| C# matching fragments | Each of class/struct/interface/record/record struct at arity 0; each of all six written categories at arity 1 with identical parameters | Correct second-fragment member ID | Same; genuine fragments share members |
| C# own member | First `H` has Base/Child; `H<T>` has a second Base | Own proved relation becomes text-only uncertain | First owner's Base ID, `extends` |
| C# inherited-name barrier | File Base; Child in `H`; only `H<T>` extends External | File Base becomes text-only uncertain | File Base ID, `extends` |
| C# outside path | Empty `H`; only `H<T>` declares Base; outside Child writes `H.Base` | Borrowed ID, `extends` | Text only; bare H cannot denote H<T> |
| C# nested owner, partial enclosing types | `Outer` and `Outer<T>` contain same-arity partial H | Borrowed ID from the other enclosing type | Text only; include enclosing identity in merge eligibility |
| C# nested owner, non-partial enclosing types | Same shape, with non-partial Outer declarations | Borrowed ID from the other enclosing type | Text only |
| C# namespace beside partial type | First partial H contains Child; namespace H contains Base | Borrowed namespace member ID | Text only; a namespace is not a fragment of a type |
| C# generic-binding control | File Base; Child in partial H; only H<Base> binds a generic named Base | Correct file Base ID | Same; generic context already uses source ancestry |
| C# non-partial control | Non-partial H and H<T>; Base only in the latter | Text only | Same; the failing bypass is partial-specific |
| C# containment control | Probe method inside first owner's Child; Base only in H<T> | Child belongs to first H; Probe belongs to Child; base ID still wrong | Preserve these source-contained memberships and fix the base |

The mismatched-kind negative rows are deliberately not valid C# programs.
Extraction supports incomplete/conflicting source, and must not invent proof
that these declarations are fragments of one type. The arity and enclosing-arity
rows express ordinary distinct C# types and demonstrate the defect independently
of invalid-program policy.

Every pair below was tested **in both source orders**, with the same member
borrowing reproduction. This enumerates all 30 directed unequal pairs among
the six written declaration forms.

| Site pair | Shape tested | Observed result | Expected result |
|---|---|---|---|
| class / struct | Both partial, same name and arity | Borrowed ID | Text only |
| class / interface | Same | Borrowed ID | Text only |
| class / record | Same | Borrowed ID | Text only |
| class / record class | Same | Borrowed ID | Text only |
| class / record struct | Same | Borrowed ID | Text only |
| struct / interface | Same | Borrowed ID | Text only |
| struct / record | Same | Borrowed ID | Text only |
| struct / record class | Same | Borrowed ID | Text only |
| struct / record struct | Same | Borrowed ID | Text only |
| interface / record | Same | Borrowed ID | Text only |
| interface / record class | Same | Borrowed ID | Text only |
| interface / record struct | Same | Borrowed ID | Text only |
| record / record class | Same; equivalent written record-class forms | Correct fragment member ID | Same; preserve this clean compatibility case |
| record / record struct | Same | Borrowed ID | Text only |
| record class / record struct | Same | Borrowed ID | Text only |

### Consumer and other-language sweep

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Summary construction, owner ambiguity and path traversal | All C# rows above | `partial` alone marks a declaration as merging; relative and outside paths read combined names | Merge only matching type identities |
| Source anchoring and member candidates | Different arity with one Base or two Bases | Borrow a sibling member or lose own unique member | Read matching fragments of the actual source owner |
| Scope flags / inherited barriers | Base clause added only to H<T> | Removes H's proved outward relation | Isolate flags by proved owner group |
| Generic context | Sibling generic named Base | Clean | Retain source-based generic evidence |
| Ordinary containment | Child and Probe nested in first owner | Clean | Retain exact enclosing IDs |
| Written-base classification | Borrowed class Base, or own/file class Base blocked by sibling | Invents `extends` proof or changes proved `extends` to uncertain | Classify only using the correct visible target |
| Cache and selective projections | All 67 supplemental rows, all four command paths | Consistent, including actual warm hits; consistently wrong on failing rows | Correct extracted facts on every path |
| Rust modules/impl consumers | Registered source-owner bare/self/super/import/glob cases, all four impl consumers | Repaired exact target and member-owner assertions | Same; never merge repeated modules by name |
| Swift types/extensions and built-in decision | Registered competing-owner, unresolved-extension and all 30 sibling raw-type rows | Repaired owner and exclusion assertions | Same |
| Java/C++/Scala/Python repeated owners | Registered sibling member/import/pattern/inherited-barrier cases | Source-owner assertions hold | Same |
| C++/TypeScript/C#/PHP reopened namespaces | Registered reopening controls | Intentional shared namespace lookup | Same; namespaces only merge with compatible namespace owners |
| JavaScript lexical classes; Go interfaces/receivers | Registered lexical-owner and absent-owner controls | Conservative/source-contained assertions hold | Same |
| C/Perl/Bash/Zsh/Lua | Registered absent-relation rows | No invented base | Same; no partial-type merge reader |

The same reproduction cannot use C# `partial` in those other languages; their
registered competing-owner and reopening counterparts test the corresponding
owner decision. `relations/lexical.rs` uses syntax scopes; `relations/inline.rs`
handles C++ namespace lookup. Neither introduces a second partial merge rule.
All C# merge consumers share the summary decision above.

### Required repair and tests

Build a bounded per-file representation of compatible owner groups. For C#,
merge only actual fragments of one type: compatible declaration category,
generic arity, and enclosing owner identity. Keep `record` and `record class`
compatible. A namespace and a partial type never form one group. A source site
must read only its matching group's members, bindings and inheritance barriers;
qualified lookup must prove which group its path denotes. If the bounded lookup
cannot prove that identity, retain written text without an ID. This does not
require a compiler, new symbol IDs or cross-file resolution.

Add every failing and clean row in these tables to the registered relation
matrices. Assert target IDs against source positions, relation category, own
member precedence and source-contained memberships. Carry the rows through
complete/declarations-only and matching-capability projections, Containment
widening and genuine warm hits. Keep existing bounded-work checks and add a
repeated compatible/incompatible fragment shape so the repair does not scan
all same-named fragments per relation. Bump the extractor revision and update
the tree-hugger docs, skill, and behavior-linked comments together.

The tree-hugger library's `is_csharp_partial` comment currently equates the
modifier with shared membership. The area's
[type-relations page](../../docs/type-relations.md:303) and skill likewise omit
the identity conditions. That is drift in the merge contract; correct it as
part of this repair, without weakening the accepted conservative-ID rule.

### Medium — Aggregate declaration-fact tests exceed the normal execution budget

**Defect class:** one test repeatedly processes an expanding source corpus across
many query combinations, so ordinary parallel execution exceeds its fixed
30-second limit even when the assertions pass in isolation.

The tree-hugger library's
[query-equivalence test](../../lib/tests/l1/symbols_query.rs:797) runs four kind
selections across six capability selections for every source from the growing
feature matrices. The two
[cache-equivalence tests](../../lib/tests/l1/symbols_cache.rs:1120) similarly
perform full-corpus repeated analysis in a single nextest test. Review 9 adds
74 source rows to this shared corpus. Nextest's default profile terminates each
ordinary test after six five-second intervals.

Two actual area L1 runs failed on this class, without an assertion failure:

- Default concurrency: warm-cache/projection test timed out at 30.029 seconds;
  631 passed, one timed out, and 603 tests did not run.
- Four workers: query-equivalence test timed out at 30.016 seconds;
  713 passed, one timed out, and 521 tests did not run.
- Query-equivalence alone, one worker: passed in 23.666 seconds.

The default run overlapped lint/build work, and the four-worker run overlapped
terminal checks and supplemental CLI probes. These are local observations,
not a claim of an assertion defect or a cross-OS failure. A test with this
little runtime headroom cannot reliably run alongside the other tests selected
by the normal recipe. The implementation log records the same aggregate-test
timeout sensitivity in the earlier full-corpus runs.

### Aggregate-test instance sweep

All six tests reading the shared source corpus are included below. The same
unedited corpus was exercised through their declared L1 targets; no fixture
mutation was needed for this workload defect.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Query kind/capability equivalence | Full corpus, 24 selections per source | Four-worker timeout; isolated one-worker pass at 23.666 seconds | Finish with headroom under normal area concurrency |
| Cache warm hits and projection | Full corpus, complete plus six narrowed selections | Default-run timeout; four-worker pass at 23.180 seconds | Same |
| Cache Containment/Metrics widening | Full corpus, two widenings and three served selections | Four-worker pass at 22.510 seconds; earlier log also records aggregate timeout sensitivity | Same; include this equally large sibling in the repair |
| Query work counters | Full corpus, declarations and complete analysis | Four-worker pass at 9.231 seconds | Same; clean in this run |
| Older-model adapter loss | Full corpus, conversion round trips | Four-worker pass at 6.843 seconds | Same; clean in this run |
| Older-schema cache rejection | Source writer invoked, then only the Rust impl control used | Four-worker pass at 0.292 seconds | Same; clean in this run |

Partition the expensive tests into bounded, statically registered corpus groups
that nextest can schedule independently, preserving every existing assertion
and full row coverage. Apply that partition to all three expensive sibling
consumers, and keep a coverage assertion so a new matrix row cannot fall outside
the groups. Do not turn these in-process checks into a gated resource tier,
remove cases, or add retries to hide the timeout. Verify the ordinary area L1
recipe completes at its normal concurrency. The one-worker rerun supplies
assertion evidence but does not close this stability finding.

## Blocked Findings

None. The finding follows the existing conservative same-file identity contract.

## Recurrence

This repeats review 9's **Binding and scope checks still combine distinct source
owners**, and review 8's **Relation and membership decisions discard unresolved
owner evidence**. Review 9 explicitly required preserving **compatible C# partial
fragments**. Its repair distinguished partial from non-partial syntax but did
not sweep arity, declaration category, namespace/type collisions, or enclosing
owner identity. These are the complete missed sibling groups enumerated above.

The earlier source-binding class also appears in review 1's **Relation IDs are
assigned without proving path meaning**, review 2's **Python nested class scopes
still assign incorrect base identities**, review 3's **Relation lookup still
drops import and generic-shadowing evidence**, review 4's **Relation lookup still
substitutes record names for source binding evidence**, review 5's **Relation
lookup loses binding evidence between syntax scopes and display-name indexes**,
and review 6's **Relation lookup still loses binding and owner evidence after
selecting the first name**. Review 7's **Swift nested-type extensions lose enclosing
generic bindings** is another owner-context consumer, but its enumerated Swift
sites remain repaired. The independent production lookup-performance, cache-matrix, generics,
record-category and schema-example classes have no new finding. The aggregate
test-budget finding concerns verification workload rather than relation lookup
complexity; no earlier review labeled it as a finding, although earlier logs
record the same runtime sensitivity.

`recurrence: true` records the remaining owner-identity class; no previously
accepted design choice has changed.

## Input Robustness Matrix

Persistent JSON cache is the only affected serialized input format; this
feature adds no manifest, lockfile, YAML or TOML reader. The registered public-output robustness tests passed in the focused
verification run. Each matrix starts from
shipped fixture output, changes one field per cell, and uses public service
or recomputation results rather than a parser-only assertion. The remaining owner defect concerns source evidence, not decoder contracts. The full feature-field
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

Test counts below describe execution, not completeness. Each requirement has
an appropriate verification level and explicit assessed coverage.

| User-facing requirement | Appropriate level | Evidence and assessment |
|---|---|---|
| Explicit generic parameters, order, ownership, bounds/defaults/variance, requirements and overflow | L1 public API | Registered generic, boundary and ownership cases; existing completed tests in the full attempt plus scoped checks. No new metadata defect found. |
| Written relation text/order, conservative identities and categories | L1 API/CLI | Registered language/path/source-owner matrices and supplemental 67-row CLI sweep. C# partial-owner identity failures block readiness. |
| Rust positive/negative/empty/inherent impls and method/type membership | L1 API/CLI | Registered exact-owner, method-only, query and cache-widening checks. Review 9's four sibling-import consumers are repaired. |
| Swift extensions and raw-type exclusion; Go interface exclusions | L1 API/CLI | Registered owner/generic/uncertain-owner and all raw-type rows; source-owner cache regressions. No new finding. |
| Declaration-owned modifiers, attributes, source order and raw arguments | L1 API/CLI | Registered per-language, wrapper, leakage and attribute/member decoder matrices. No new finding. |
| Callable physical body lines, absent/expression/nested bodies and LF/CRLF boundaries | L1 API | Shared body node/span inspected; registered metric/span and projection cases. No new finding. |
| One parse/definitions query, skipped optional passes and bounded relation work | L1 counters | Registered source-fact and bounded-work tests. No new production lookup-work defect found; aggregate verification tests have a separate execution-budget finding. |
| Cache narrowing/widening, genuine hits, versions and strict fields | L1 isolated API/CLI | Registered source/owner projection, widening and robustness tests; supplemental genuine hits preserve the C# defect. Large aggregate tests are timing-sensitive. |
| Outline labels, signature suppression, selectors, sorting, wrapping and styling | L1 CLI plus L2 terminal capture | Registered CLI assertions and real tmux/WezTerm report capture. No separate rendering defect; upstream wrong relations can mislabel bases. |
| JSON/JSONL/full index, counts and statistics | L1 CLI | Registered symbol report/golden checks and supplemental complete/selective results. Formats faithfully expose the source defect. |
| Keyboard, paste, IME or mouse behavior | L3 only when OS input encoding is promised | This feature introduces no input behavior; no L3 requirement. |
| Current docs, schema and older-model adapter losses | Inspection plus L1 | Schema/version and conversion contracts agree; partial-owner wording needs the compatibility conditions described in the high finding. |

The tree-hugger library's
[L1 target](../../lib/tests/l1/main.rs) declares the relevant matrices despite
`autotests = false`. The CLI's
[symbol target](../../cli/tests/symbols/main.rs) declares the report/cache modules.
Ordinary names select L1. Terminal targets require `terminal-tests`; the live
`test-l2` recipe and CLI CI metadata enable it. The tier audit found zero stranded
tests. No tests were added or renamed by this review.

This review changes only review/spec metadata and the review document. It does
not repair implementation code or move the feature into a lifecycle directory.
Implementation needs another repair cycle; the author closes the feature after
review completion.
