---
$schema: feature-review.yaml
ready: false
findings:
  - title: Relation and membership decisions discard unresolved owner evidence
    priority: high
  - title: Inline namespace paths still repeat broad candidate scans
    priority: medium
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-06T11:37:47-07:00
spec: 2026-10-06-symbol-type-relations/spec.md
log: tree-hugger/features/2026-10-06-symbol-type-relations/log.md
implemented: true
implemented_by: claude/opus
description: "A **feature** review of `2026-10-06-symbol-type-relations/spec.md`"
feature: 2026-10-06-symbol-type-relations/review-8.md
previous: 2026-10-06-symbol-type-relations/review-7.md
next: 2026-10-06-symbol-type-relations/review-9.md
---

# Review of Symbol Type Relations — Iteration 8

The feature is **not production ready**. Review #7's enumerated Swift generic,
inline namespace and bound-attachment cases are repaired. The broader sweep
finds that lookup still confuses distinct owners with namespace reopenings,
Swift extension consumers disagree about unresolved ownership, and ambiguous
inline prefixes still cause repeated scans. Both findings have concrete
repairs within the accepted design; no human decision is required.

## Previous findings and blocked work

Read the specification, implementation log and findings in all seven earlier
reviews. The accepted design retains precise inheritance and conformance
lists, preserves uncertain written bases in `unclassified_bases`, and attaches
Swift extension conformance only to a uniquely resolved same-file owner.

| Previous finding | Assessment |
|---|---|
| Review #7, “Swift nested-type extensions lose enclosing generic bindings” | The listed outer/middle/own generic, qualified, callable/closure and 30 built-in cases now pass. Context layers are shared by actual owner identity, including oversized unselected owners. Complete/selective, widening, actual-hit and outline regressions pass. An unresolved extended owner still loses context; see the high finding. |
| Review #7, “Inline namespace and generic processing still repeat input-sized scans” | The listed broad injected/rooted/bare/explicit/ambiguous leaf cases, fully written deep paths, Swift context sharing, and Rust/C#/Swift bound attachment now pass their counted-work regressions. Broad *intermediate* inline namespace candidates remain unbounded; see the medium finding. |
| Reviews #1–#6, binding evidence and owner identity | Their registered root, scope, name-group, import, write, directive, generic, built-in, owner and cache regressions pass. The owner ambiguities below are additional instances of this class. |
| Review #1, template-template defaults and independently written callable requirements | Registered ownership, default, requirement and projection tests pass. |
| Reviews #2–#4, cache robustness; review #3, C# record classification and schema documentation | The permanent public-output matrices and record-class cases pass. Current documentation agrees with schema 5.0 and extractor revision 35. |

Review #7 has no blocked findings. The older blocked C#/Swift/Scala base-list
work was unblocked by the author's accepted classification choice and is
implemented. It does not need another design approval.

## Verification and scope

Used the tree-hugger and rust-testing skills; consulted the
biscuit-test-harness skill for terminal verification. Reviewed all 16 language
acceptance rows, declaration metadata, relation and ownership consumers,
query/cache behavior, report rendering, current documentation and test
registration. Concurrent reference-report and cyclomatic-metric work was
preserved; its independent behavior is outside this review.

- `NEXTEST_TEST_THREADS=4 just test`: **1,224 passed, zero skipped**, 153.085 seconds.
- `just lint`: passed for both packages.
- `just check-tier-coverage tree-hugger`: zero stranded tests.
- `just test-l2`: the first attempt failed when Bash printed `child setpgid … Operation not permitted` inside a captured report. A second invocation passed all six selected entry points in 12.829 seconds. tmux and WezTerm supplied rendering evidence; Kitty entry points used unavailable-backend gates and supply no Kitty evidence. No keyboard injection or foreground spawn was requested. The initial failure is recorded as a terminal-harness/environment limitation, not a source-relation defect.
- Supplemental shipped-CLI checks copied primary fixtures into isolated temporary Git repositories with separate home/config/cache directories. Each swept source row was checked through complete uncached output, selective uncached output, cache population and a warm hit. Warm runs reported `1 hits` and `parsed 0 files`. Comparisons of complete and selective paths concern the applicable relation facts; their other capabilities intentionally differ.
- Public `SymbolCache::query` probes covered C++, Scala, Rust impls, ambiguous Swift extension membership and Swift generic-context loss through declarations, Containment widening, complete widening, a complete `Hit` and a type-only `Hit`. Hits performed no parse. Widening preserved the defects; equality with fresh extraction does not establish correctness.
- Supplemental public `TreeFile::query` work measurements used copied C++ fixtures at 200/400 repetitions, with an 800-repetition confirmation. Every measurement retained one definitions query and reused the already parsed tree. Temporary probes were outside the repository; no permanent Rust test or implementation file was added by this review.

Cross-OS execution evidence is left to CI and does not decide readiness.

## Unblocked Findings

### High — Relation and membership decisions discard unresolved owner evidence

**Defect class:** later lookup and projection replace the actual owner decision
with a displayed-name match, treating distinct or unresolved owners as one
proved declaration and sometimes treating unknown generic context as empty.

In the tree-hugger library,
[Declarations::is_traversable](../../lib/src/file/tree_file/relations.rs:1035),
which decides whether a qualified base can continue through an owner, permits
several candidates whenever they are Module/Namespace records. That does not
prove they are reopenings of the same namespace. Rust modules and Scala
objects do not merge this way. Two C++ namespaces injected from different
inline namespaces are also distinct, even though the queried spelling is the
same. The tree-hugger library's
[Declarations::each_summary](../../lib/src/file/tree_file/relations.rs:817)
combines their flags and candidates without retaining this distinction.

Append this to the C++ symbol fixture:

```cpp
namespace ReviewNs {
    inline namespace a { namespace Shared { struct ReviewBase {}; } }
    inline namespace b { namespace Shared {} }
}
struct ReviewChild : ReviewNs::Shared::ReviewBase {};
```

```sh
hug --json-full symbols --no-cache probe.cpp
hug --json types --all-scopes --no-cache probe.cpp
hug --verbose --json-full symbols probe.cpp
hug --verbose --json-full symbols probe.cpp
```

Every result identifies `ReviewNs::a::Shared::ReviewBase`. The intermediate
`Shared` name is ambiguous, so the written base must remain without an ID or
qualified name. `clang++ -fsyntax-only` independently reports “reference to
'Shared' is ambiguous,” naming both candidate namespaces. Tree-sitter accepts
the source without syntax diagnostics. A unique final name cannot repair an
ambiguous earlier segment.

#### Namespace and module owner sweep

“Text only” means no target ID or qualified name. For a Swift/Scala mixed base
whose category is unproved, it also means `unclassified_bases` rather than a
fabricated `extends` category.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| C++ named continuation | The snippet above; rooted `::ReviewNs::Shared::ReviewBase`; bare `Shared::ReviewBase` inside ReviewNs | Base ID in all three | Text only: two distinct Shared namespaces |
| C++ named continuation | Shared in the enclosing namespace plus Shared in an inline child | Injected Base ID | Text only; direct and injected owners are distinct |
| C++ named continuation | Additional nested inline namespace; separately an extra `Shared::Inner` segment | Base ID in both | Text only; ambiguity must stop every later segment |
| C++ explicit-owner control | `ReviewNs::a::Shared::ReviewBase` | Exact Base ID | Same; explicit a identifies one owner |
| C++ reopening controls | Ordinary repeated Shared namespace; repeated definitions of the same canonical inline `a::Shared`; differently named injected namespace | Exact Base ID | Same; actual namespace identity, not record count, governs merging |
| C++ ambiguity controls | Two final ReviewBase declarations; two injected struct Shared owners | Text only | Same; final-name/type-owner ambiguity is already handled |
| Rust named module continuation | Two `mod ReviewHolder` declarations, with ReviewBase only in one; relative and `crate::`-rooted paths | Base ID | Text only; modules are competing declarations, not reopenings |
| Rust conditional/nested module continuation | Same owners with opposite cfg attributes; separately `ReviewHolder::Inner::ReviewBase` | Base ID | Text only; extraction does not evaluate cfg or select a module branch |
| Rust source-anchored context | Both modules have Base; Child is inside the first and writes bare Base, `self::Base`, or an inner module's `super::Base` | Text only in all three | First module's own Base ID, identified by its source scope; do not merge its body with the second module |
| Rust lexical controls | Duplicate local modules queried from outside both; `self::Base` inside the first local module | Text only / first local module's Base ID | Same; the syntax-owner route is clean |
| Scala named object continuation | Two ReviewHolder objects, with Base in only one; separately `ReviewHolder.Inner.ReviewBase` | Base ID and extends | Text-only unclassified base; objects do not reopen one namespace |
| Scala source-owner context | Each ReviewHolder object declares Base; Child is inside the first and uses bare Base | Unclassified text | First object's Base ID in extends; its body is an identified source scope |
| Scala controls | Duplicate local objects outside both; class/object companion with Base inside the object | Text only / exact object member ID | Same; ambiguity and type/value separation must be preserved |
| TypeScript namespace continuation | Repeated declarations of one namespace containing a class | Exact Base ID | Same; valid declaration merging |
| C# namespace continuation | Repeated declarations of one namespace containing a class | Exact Base ID | Same; actual namespace reopening |
| PHP namespace continuation | Repeated block namespace and repeated statement namespace containing a class; rooted target | Exact Base ID | Same; actual namespace reopening |
| Java, Swift, C++ type-owner continuation | Duplicate ordinary owners with a member in only one | Text only | Same; these owners are already rejected |
| TypeScript, JavaScript, Python, Go controls | Duplicate ordinary names followed by a qualified base | Text only | Same; no namespace-like owner is incorrectly merged in these rows |
| C, Perl, Bash, Zsh, Lua | Registered absent-relation rows | No invented base | Same; no applicable base resolver |

For the Rust source-anchored cases, the module declaration is known from the
Child's actual ancestors. Choosing that module's own member does not evaluate
cfg. This is the same distinction the passing function-local-module control
already preserves.

#### Rust impl consumer sweep

The same module ambiguity affects the tree-hugger library's
[resolve_self_type](../../lib/src/file/tree_file/relations.rs:1750), which
establishes impl ownership, and
[resolve_impl](../../lib/src/file/tree_file/relations.rs:1729), which establishes
trait conformance. Use two same-named modules with the struct or trait in only
one, then an impl outside both.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Impl self type, trait unambiguous | `impl ReviewTrait for ReviewHolder::ReviewBase` | Local Base gains conformance; method gains parent/container and Type container kind | Self unresolved: no type aggregation or local parent/membership; keep textual self container and the independently proved trait |
| Impl trait, self unambiguous | `impl ReviewHolder::ReviewTrait for ReviewBase` | Local trait ID on type and member | Keep owner/membership, but trait target text only |
| Both paths ambiguous | Qualified self and trait through the duplicate module | Both IDs fabricated | Neither path resolved; preserve written target/container text |
| Empty positive impl | Ambiguous qualified self; no members | Base gains implements | No aggregation onto that unproved self type |
| Inherent impl | Ambiguous qualified self; one method | Base gains method membership; method has local parent/container | No local ownership; inherent impl still has no conformance |
| Member-only metadata and cache widening | These sources, declarations followed by Containment and full-index requests | Same false kind, ownership and trait facts; Containment reparses once | Same conservative decisions across all requests and actual hits |
| Negative/ordinary unique/local-owner controls | Registered impl matrices | No negative conformance; exact unique self/trait IDs and membership | Same; retain |

#### Swift separately written owner sweep

Swift exposes another projection of the same class. The tree-hugger library's
[extension_owners](../../lib/src/file/tree_file/relations.rs:1376) correctly
rejects ambiguous or imported owners for conformance. Its
[type_parameters_around](../../lib/src/file/tree_file/relations.rs:1305) restores
owner generic layers only for `Some(owner)`; unresolved context becomes no
context. Separately, the tree-hugger library's
[find_container_by_identity](../../lib/src/analysis/mod.rs:795) and
[nearest_qualifying_owner](../../lib/src/analysis/mod.rs:855), which place
out-of-body members, can select a local record by name even when extension
owner resolution rejected it. The nearest-preceding rule needed by repeated
Lua tables is not proof of Swift extension ownership.

```swift
protocol ReviewP {}
struct ReviewHolder { struct ReviewBase {} }
struct ReviewHolder {}
extension ReviewHolder.ReviewBase: ReviewP { func review_probe() {} }
```

ReviewBase correctly receives no conformance, but it incorrectly receives
`review_probe` in members, and the method receives its parent/container IDs.
Replacing the second Holder with `typealias ReviewHolder = External` or a
selective imported type produces the same disagreement.

For lost generic context, append instead:

```swift
class ReviewBase {}
struct ReviewHolder<ReviewBase> { struct Inner {} }
struct ReviewHolder {}
extension ReviewHolder.Inner { class ReviewChild: ReviewBase {} }
```

ReviewChild incorrectly identifies the file-level class and reports extends.
An owner whose lookup is ambiguous does not establish that its generic
context is empty. Preserve the base text and its uncertain category.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Swift extension member containment | Duplicate direct owner, duplicate final nested owner, duplicate intermediate owner | Selects one local type; adds member and parent/container IDs | No fabricated local owner or membership; preserve textual owner identity |
| Same containment reader | Alias shares direct owner name or intermediate owner name | Selects the unrelated local type | No local owner; alias expansion is outside scope |
| Same containment reader | Selective struct import shares direct/intermediate owner name | Selects local type despite unresolved extension lookup | No local owner; imported identity is unproved |
| Extension conformance aggregation on those seven shapes | Same source, with `: ReviewP` | Does not attach conformance | Clean; member linking must honor the same owner decision |
| Extension-body generic lookup | Ambiguous nested owner with a possible enclosing parameter ReviewBase; separately alias-blocked owner | File-level Base ID and extends | Text-only unclassified base; no proof that the owner lacks that binding |
| Extension-body external-owner lookup | File-level Base; `extension External { class ReviewChild: ReviewBase {} }` | File-level Base ID | Text only while the external owner's generic/member context is unknown |
| Built-in exclusion with unresolved owner | Ambiguous nested owner has possible Int/etc. parameter; enum child uses that name | Drops every one of the 30 spellings below | Preserve text in unclassified bases; absent owner evidence cannot prove a standard raw-value type |
| Nearer lexical control | Unknown owner; extension method declares its own Base before Child | Exact function-local Base ID | Same; preserve lexical precedence before uncertain owner context |
| Known-owner and import controls | Unique nested owner; selective function import sharing its name | Correct conformance and membership | Same; value-only imports do not hide the owner type |
| Wrong external path and local-shadow controls | `External.ReviewBase` beside a local Base; registered function/closure-only and shadow-owner cases | No fabricated owner / correct visible file-level owner | Same; retain |
| Known-owner generic contexts | Review #7's 46 rows and oversized-owner regression | Correct text-only generic targets or exact control IDs | Same; those repairs are implemented |

The 30 raw-type spellings were individually checked through all four CLI
paths: `Int`, `Int8`, `Int16`, `Int32`, `Int64`, `UInt`, `UInt8`, `UInt16`,
`UInt32`, `UInt64`, `String`, `Character`, `Float`, `Double`; their 14 `Swift.`
forms; `Unicode.Scalar`; and `Swift.Unicode.Scalar`. The possible owner's
parameter is the simple name or the qualified spelling's first segment.

**Required repair:** retain owner identity and uncertainty throughout lookup.
Merge only namespaces the language and canonical owner path prove to be the
same. A Rust module/Scala object selected from source ancestry must use that
owner's members, not the combined displayed-name bucket. Every continuation
must stop at an unresolved intermediate owner. Feed the Swift extension's
conservative owner result into member containment and cache widening as well
as conformance; do not reselect a record by proximity. Preserve uncertainty
about generic/inherited member context when an extension owner cannot be
proved, while allowing nearer lexical bindings to win.

Extend the shared relation and owner-identity matrices with **every failing
and clean row above**, asserting IDs and source positions rather than only
names. Include all 30 lost raw-type forms, Rust self/trait/member-only/empty/
inherent consumers, Swift owner conformance/member disagreement, cold and
actual warm output, narrowing and widening. Keep existing IDs and written
text stable. Bump the extractor revision to invalidate wrong stored facts.
Update current docs, skill and behavior-linked comments: their promises of
conservative paths and consistent ownership currently exceed the code.

### Medium — Inline namespace paths still repeat broad candidate scans

**Defect class:** memoizing a broad intermediate prefix does not bound later
lookups when every distinct suffix revisits all candidate owners under that
prefix.

The tree-hugger library's
[Declarations::denoted](../../lib/src/file/tree_file/relations.rs:500), which
finds canonical names reached through an inline-namespace spelling, stores a
vector of every owner matching each prefix. For `ReviewNs::Shared::BaseN`,
`ReviewNs::Shared` has n distinct canonical owners. Each new BaseN then loops
over all n parents and calls the tree-hugger library's
[InlineNames::members](../../lib/src/file/tree_file/relations/inline.rs:58),
which searches the member range. Prefix caching does not remove these n²
range lookups. The high finding explains why these prefixes must also stop
as ambiguous; carrying a giant candidate vector is not necessary to report
that uncertainty.

Append this generated shape to the C++ fixture, using n=200 and n=400:

```python
source = "namespace ReviewNs {\n"
for i in range(n):
    source += (f"inline namespace v{i} {{ namespace Shared {{ "
               f"struct ReviewBase{i} {{}}; }} }}\n")
source += "}\n"
for i in range(n):
    source += f"struct ReviewChild{i}: ReviewNs::Shared::ReviewBase{i} {{}};\n"
```

Parse before taking `work_counters()`, run the public declarations-only query,
and compare `shared_context_steps`. These are counted operations, not timing
thresholds. Every failing row currently fabricates n IDs; expected output is
n textual targets with no IDs, with work bounded by declarations and paths.

| Site / shape tested | Steps at n=200 | Steps at n=400 | Observed / expected result |
|---|---:|---:|---|
| `denoted` → `InlineNames::members`, qualified Shared prefix | 93,661 | 346,463 | 3.70× work for 2× input; expected approximately 2× and textual targets |
| Same lookup, rooted prefix | 93,260 | 345,662 | Same defect; anchoring does not bound candidates |
| Same lookup, bare Shared inside ReviewNs | 94,260 | 347,662 | Same defect through first-segment scope lookup |
| Same lookup, extra `Shared::Inner` segment | 101,261 | 362,463 | Same defect at deeper continuation |
| Explicit `ReviewNs::vN::Shared::ReviewBaseN` | 18,643 | 37,243 | Correct n IDs and approximately 2× work; clean control |
| Ordinary namespace control, explicit vN | 6,158 | 11,958 | Correct n IDs and bounded growth; clean |
| Same final Base name in every Shared namespace | 17,461 | 34,863 | Text-only targets and approximately 2× work; clean growth, because the same complete spelling is cached |
| Shared owners are structs instead of namespaces | 14,461 | 28,063 | Text-only targets; bounded growth because traversal stops; clean |
| `each_summary`, `qualifies`, `named`, `is_bound`, `is_traversable`, `scope_flags`, `named_member_rebound` | Qualified/rooted/bare/deeper shapes above | Same dispatch | These consumers share the broad-prefix representation; preserve its ambiguity instead of requesting all suffix candidates |
| Non-C++ relation lookup and syntax-owner member indexing | Registered repeated-owner/name/module/reopening matrices | Passing | No inline-subtree loop; clean controls |
| Swift owner context and Rust/C#/Swift separately written bounds | Registered shared-context and bound-attachment work matrices | Passing | Owner-identity sharing and indexed bound subjects are repaired |
| Base-text deduplication and deep unambiguous inline paths | Registered source/linker dedup and 4/8/16-depth matrices | Passing | Preserve these earlier repairs |

At n=800 the first shape costs **1,332,065** steps, confirming repeated broad
work rather than a fixed fixture overhead. Each measured query runs one
locals query and performs no new parse. The existing work tests use unique
injected leaf names or one repeated complete ambiguous name; they do not
exercise distinct suffixes under a broad ambiguous intermediate namespace.

**Required repair:** represent a prefix's unique/merged/ambiguous owner meaning
without propagating all distinct owners into every suffix lookup. Stop after
ambiguity is proved, preserving actual canonical namespace reopening and
exact unambiguous paths. Add every measured shape and its exact expected
output to the registered relation-work matrix, retaining the existing work
budget and growth checks. Candidate/range work must remain counted. Do not
replace the check with a timeout or remove the correctness assertions.

## Blocked Findings

None. Both findings can be repaired within the accepted source-only contract.

## Recurrence

The high finding repeats the owner/binding-evidence class reported in review
#1, “Relation IDs are assigned without proving path meaning”; review #2,
“Python nested class scopes still assign incorrect base identities”; review
#3, “Relation lookup still drops import and generic-shadowing evidence” and
“Built-in name exclusions erase valid Go and Swift bases”; review #4,
“Relation lookup still substitutes record names for source binding evidence”;
review #5, “Relation lookup loses binding evidence between syntax scopes and
display-name indexes”; review #6, “Relation lookup still loses binding and
owner evidence after selecting the first name”; and review #7, “Swift
nested-type extensions lose enclosing generic bindings.”

Those fixes should have swept namespace-like *intermediate* owners as well as
final types: distinct injected C++ namespaces, duplicate named Rust modules
and Scala objects, source-anchored lookup inside those owners, all Rust impl
consumers, and every Swift extension consumer when its owner is ambiguous,
aliased, imported or external. Restoring known-owner generic layers should
also have distinguished unresolved owner context from a proved empty context,
including all built-in exclusions. This review carries those sibling sites
and clean controls together.

The medium finding repeats review #6, “Relation processing exceeds the bounded
per-file work contract,” and review #7, “Inline namespace and generic
processing still repeat input-sized scans.” The inline repair should have
swept broad intermediate prefixes with *distinct suffixes*, alongside broad
leaf lookup, repeated identical ambiguous names and deep unique paths. This
review measures qualified, rooted, bare and deeper variants and the explicit,
ordinary, same-leaf and type-owner controls. `recurrence: true` records both
classes; no earlier cache-decoder, C# record or schema-documentation finding
was reopened.

## Input Robustness Matrix

The feature's persistent JSON cache is the affected file format. No new
manifest, lockfile or configuration reader is involved. The registered
public-output matrices passed in this review. Their positive controls come
from shipped-CLI fixture output; each malformed cell changes one field and
asserts service or recomputation through public output. The findings above
concern source evidence, not cache decoding.

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
| Structured generic ownership, order, bounds/defaults/variance and overflow | L1 public API | Nine generic-language matrices, absent-language rows, ownership/default/requirement tests and position boundaries pass. Unresolved Swift owner context affects relation interpretation, as listed above. |
| Written base text, order, deduplication, exact identities and categories | L1 public API/CLI | All 16 registered rows and earlier path regressions pass. Supplemental intermediate-owner ambiguity and unresolved Swift context expose wrong IDs/categories or lost text; blocks readiness. |
| Rust empty/multiple/negative impls and member/type ownership | L1 public API/CLI | Registered matrices and widening/hit controls pass. Supplemental duplicate-module self/trait cases expose false aggregation and ownership; blocks readiness. |
| Swift extension conformance and membership | L1 public API/CLI | Known generic owners and closure-shadow repairs pass. Supplemental ambiguous/alias/import owners have no conformance but fabricated membership, including after widening/hits; blocks readiness. |
| Declaration-owned modifiers and raw attributes | L1 public API/CLI | Every language row, wrapper/leakage controls, ordering, repeated/empty/raw argument cases and persistent member matrices pass. No new finding. |
| Callable body lines, expression/empty/absent/nested bodies and LF/CRLF | L1 public API | Body-node/span matrices, arithmetic boundaries, required/null metric keys and projection tests pass. No new finding. |
| One parse/definitions query, skipped optional passes, bounded shared work | L1 counters | Registered work regressions pass. Supplemental broad intermediate inline prefixes grow about fourfold with doubled input; blocks readiness. All supplemental queries still use one definitions query. |
| Cache narrowing/widening, actual hits and exact invalidation | L1 isolated API/CLI | Decoder, projection, schema/revision and warm-hit tests pass. New probes demonstrate that the cache preserves upstream owner defects; it does not cause them. |
| Outline base labels, signature suppression, selectors, sorting, wrapping and styling | L1 CLI plus L2 real-terminal capture | Registered outline/golden checks pass. tmux and WezTerm rendered the reports successfully on the second invocation; first-run shell-noise failure is documented above. Source ownership/category defects can still change placement or labels. |
| Counts/stats and JSON/JSONL/full-index facts | L1 CLI | Registered format/count/stat/golden checks pass. Supplemental JSON faithfully exposes the faulty source facts. |
| Keyboard, paste, IME or mouse promises | L3 when terminal input encoding is promised | No such behavior is added by this feature; no missing L3 requirement. |
| Current schema/cache/report/API docs and older-model adapter losses | Source/doc inspection plus L1 schema/adapter tests | Schema 5.0, revision 35 and documented adapter losses agree. Conservative owner and bounded-work claims need the repairs above. |

The library sets `autotests = false`; [tests/l1/main.rs](../../lib/tests/l1/main.rs)
declares the feature's language, owner, cache, query and work modules. CLI
matrices are declared by [tests/symbols/main.rs](../../cli/tests/symbols/main.rs).
Ordinary names select L1. Terminal targets require `terminal-tests`, which the
live L2 recipe and CLI CI metadata enable. The tier audit found no stranded
test. Passing aggregate counts do not close the reproduced failures.

The feature remains active for repair and another review. Closing its lifecycle
directory remains the author's responsibility.
