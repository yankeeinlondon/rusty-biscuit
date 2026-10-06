---
$schema: feature-review.yaml
ready: false
findings:
  - title: Relation lookup still drops import and generic-shadowing evidence
    priority: high
  - title: Built-in name exclusions erase valid Go and Swift bases
    priority: high
  - title: C# record superclasses remain unclassified
    priority: medium
  - title: Current schema examples advertise obsolete versions
    priority: medium
  - title: New relation source-span members lack permanent robustness columns
    priority: medium
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-05T21:59:47-07:00
spec: 2026-10-06-symbol-type-relations/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-06-symbol-type-relations/log.md
description: "A **feature** review of `2026-10-06-symbol-type-relations/spec.md`"
feature: 2026-10-06-symbol-type-relations/review-3.md
previous: 2026-10-06-symbol-type-relations/review-2.md
next: 2026-10-06-symbol-type-relations/review-4.md
---

# Review of Symbol Type Relations — Iteration 3

The feature is **not production ready**. The specific review-2 repairs pass,
and the author resolved the previously blocked design choice. Additional
source forms still produce incorrect identities, omit written bases, or leave
provable inheritance unclassified. Current JSON/cache examples also disagree
with the emitted schema.

## Previous findings and the author decision

Reviewed both earlier reviews, the specification, plan, original implementation
log, and repair log. Review-2's two unblocked findings were implemented. Its
blocked finding was explicitly unblocked before the language-reader follow-up:
the author accepted precise inheritance/conformance lists plus
`relations.unclassified_bases`, with Swift extension conformance attached only
to a uniquely resolved same-file owner. This review applies that accepted
meaning; it does not ask the author to choose again.

| Earlier finding | Assessment of the listed repairs |
|---|---|
| Review-2: Python nested class scopes still assign incorrect base identities | Repaired. All nine listed nested/direct-header/function/method cases are represented in the path matrix. Complete, selective, and actual warm-hit checks pass. |
| Review-2: Generic and base fields lack a permanent cache robustness matrix | The requested columns are repaired and pass. The newly added source_span object also has a column; its six nested members need permanent columns as described below. |
| Review-2: Three languages still omit required base relations | Unblocked and implemented for the listed forms. C#, Swift, and Scala readers replace the pending rows; Swift local-owner extensions aggregate conformance. Additional import, record, and enum cases below remain defective. |
| Review-1: Relation IDs are assigned without proving path meaning | Earlier root, scope, import, generic, inherited-name, and Rust ownership reproductions pass. The broader class recurs in the first finding below. |
| Review-1: Template-template parameter defaults are dropped | Struct/function/alias/method/constructor default rows pass, including cache hits and projections. Swift packs and inside-list requirements, and Rust attributed-parameter positions, retain their repairs. |
| Review-1: Callable requirements disappear without a direct template wrapper | C++ member declarations, definitions, constructors, friends, and out-of-class definitions retain their own requirements. C# list-less requirement cases pass too. |

There are no remaining blocked findings. The repairs below require code and
tests under the already accepted contract, not another design decision.

## Verification and scope

Used the tree-hugger and rust-testing skills. Reviewed relation and generic
readers, record construction, Rust ownership, selective analysis, cache
projection/decoding, schema versions, outline rendering, current documentation,
and test registration. No production code or permanent tests were changed.

- `just test`: **1,102 passed, zero skipped**; 43.932 seconds.
- `just lint`: passed for both packages.
- `just test-l2`: six entry points passed. Declaration rendering executed in
  tmux and WezTerm, including relation styling and wrapping. Kitty entry
  points returned through their unavailable-backend path; they provide no
  Kitty rendering evidence. The existing harness owns spawning and teardown.
- `just check-tier-coverage tree-hugger`: zero stranded tests.
- Supplemental source checks copied the library's language fixtures into
  isolated directories and appended one competing declaration/import/base
  shape at a time. A 102-case sweep and its follow-up checks compared complete
  uncached output, type-only uncached output, and a cached complete run followed
  by a genuine warm hit. The 102 cases agreed across paths; every last run
  reported `1 hits` and `parsed 0 files`. No error diagnostics were reported
  in those complete indexes.
- Generic boundary checks extended the existing generated wide-parameter
  fixture to all nine generic readers. The 65,536-parameter controls preserved
  shadowing; the 65,537-parameter cases below exposed discarded evidence.
  Selecting the oversized owner through a complete index correctly produced
  `too_many_generic_parameters` instead of a partial list.
- Independent minimal programs were accepted by `go tool compile` for a
  local interface named `int`, and by `swiftc -typecheck` for both a protocol
  alias named `Int` and an explicitly imported protocol named `Int`. The
  imported Swift protocol was supplied by a temporary compiled module.
- A supplemental cache sweep edited all six nested source-span members,
  nine shapes each, in a real CLI-written Swift extension entry. All 54
  public-output checks matched the contract below. These are supplemental
  checks, not permanent regression coverage.

Validation ran against schema 4.3 and extractor revision 29. Concurrent work
on reference occurrences subsequently began changing analysis/cache/index
files; that separate functionality is outside this review. The findings below
are demonstrated by the built CLI and the relation/generic sources reviewed.
Cross-OS proof is left to CI and does not affect the readiness judgment.

## Unblocked Findings

### High — Relation lookup still drops import and generic-shadowing evidence

**Defect class:** failed or incomplete collection of a nearer name binding is
treated as absence of that binding, allowing lookup to assign an identity
that the written path does not prove, or to suppress unrelated valid lookup.

The tree-hugger library's
[import_binding](../../lib/src/file/extract/relations.rs:558), which collects
names imports hide from same-file relation lookup, reads only the last plain
Scala import entry or the first selector group. It does not read an unbraced
`as_renamed_identifier` as an alias. Braced `as` selectors, including exclusion
selectors, become opaque imports and unnecessarily hide unrelated names.
These are supported grammar forms, not grammar limitations. Scala specifies
comma-separated imports as separate import expressions and supports both
braced and unbraced `as` renaming. See the
[Scala import clause specification](https://www.scala-lang.org/files/archive/spec/2.13/04-basic-declarations-and-definitions.html#import-clauses)
and [Scala 3 import syntax](https://docs.scala-lang.org/scala3/reference/changed-features/imports.html).

For example, append this to the Scala symbol fixture:

```scala
class ReviewBase
trait ReviewOther
object ReviewOuter {
  import lib.ReviewBase, lib.ReviewOther
  class ReviewChild extends ReviewBase with ReviewOther
}
```

The library assigns `ReviewChild.extends[0]` the ID of the enclosing package's
`ReviewBase`. The imported `ReviewBase` must stay textual and unclassified;
the unrelated local declaration does not prove its identity. Replacing the
import with `import lib.Other as ReviewBase` produces the same wrong link.

The same evidence-loss class appears in
[type_parameters_around](../../lib/src/file/tree_file/relations.rs:497), the
tree-hugger library helper that collects enclosing generic names before
resolving a relation. `if let Ok(generics) = declared_generics(...)` silently
drops every name when one position exceeds `u16`. A selective query can skip
materializing the oversized owner and then resolve its parameter name to a
different declaration. This is not the intended selected-owner error rule:
the query may omit the owner, but it must still respect the owner's bindings.

This Python generator produces the boundary failure without a huge snippet
in the review:

```python
from pathlib import Path

parameters = ["ReviewBase"] + [f"T{i}" for i in range(65536)]
Path("probe.py").write_text(
    "class ReviewBase: pass\n"
    f"def factory[{','.join(parameters)}]():\n"
    "    class ReviewChild(ReviewBase): pass\n"
)
```

From the fixture directory:

```sh
hug --json-full symbols --no-cache probe.py
hug --json types --all-scopes --no-cache probe.py
hug --verbose --json types probe.py
hug --verbose --json types probe.py
```

The complete command correctly exits 1 with the generic-count error. The
type-only commands exit 0 and incorrectly identify module `ReviewBase` as the
base; the last run serves that wrong link from cache. Removing one generated
parameter makes the base correctly unresolved. C# uses an oversized generic
class containing a nested interface, queried with `types --kind interface`,
because C# does not support local class declarations in a method.

The import sweep covered every implemented import-binding reader, with the
same competing local name and imported name shape:

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Scala plain import | Single `lib.ReviewBase` | Textual/unclassified | Same; clean |
| Scala comma imports | `lib.ReviewBase, lib.ReviewOther` | First name incorrectly resolves; last stays textual | Both imports block local identity |
| Scala mixed imports | `lib.ReviewBase, other.{Other}` | ReviewBase incorrectly resolves | Imported ReviewBase stays textual |
| Scala multiple selector groups | `lib.{Other}, lib.{ReviewBase}` | Second group's ReviewBase incorrectly resolves | Read every selector group |
| Scala selector group first | `lib.{ReviewBase}, other.Other` | ReviewBase stays textual | Same; clean for this name |
| Scala legacy rename | `lib.{Other => ReviewBase}` | Textual/unclassified | Same; clean |
| Scala unbraced rename | `lib.Other as ReviewBase` | Assigns unrelated package type ID | No ID; unclassified |
| Scala braced rename | `lib.{Other as ReviewBase}` | Imported base stays textual | Same for the imported name; unrelated-name failure in next row |
| Scala braced rename/exclusion | `lib.{Other as Renamed}` or `lib.{Other as _}`, with Child extending unrelated outer ReviewBase | ReviewBase becomes unresolved | Resolve the unaffected outer ReviewBase |
| Scala wildcard controls | Alias plus `_`, with outer-only Base; wildcard with a stronger local Base | Outer-only base textual; stronger local base resolves | Same; preserve these controls |
| Scala no-import control | Child extends outer ReviewBase | Resolves outer ReviewBase | Same; clean |
| Rust imports | Grouped rename, grouped rename plus glob | Text only | Same; clean |
| C++ imports/aliases | Using-declaration and namespace alias | Text only | Same; clean |
| PHP imports | Grouped use with renamed class | Text only | Same; clean |
| Python imports | Multiple from-import entries and renamed entry | Text only | Same; clean |
| TypeScript imports | Namespace import alias | Text only | Same; clean |
| JavaScript imports | Multiple named entries with rename | Text only | Same; clean |
| C# imports | Namespace alias and static using; `global::` bypass control | Relative imported bases textual; explicit global target resolves | Same; clean |
| Swift imports | Selective class import and whole-module control | Named import textual; stronger local declaration resolves through module import | Same; clean |
| Java/Go | No import-binding reader; existing same-file-conflict boundary | Existing positive relation matrices pass | No new import reader required by this finding |
| C/Perl/Bash/Zsh/Lua | No applicable required declared-base/import-resolution branch | Negative relation matrices remain empty | Same |

The generic-name sweep checked each reader at the position boundary:

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust, TypeScript, Go, Python, Java | 65,537 parameters on an enclosing function/method; selected inner type names first parameter as a base | Assigns an unrelated type/trait ID | Retain shadowing; no target ID |
| Swift, Scala | Same enclosing-callable shape | Assigns ID and moves ambiguous base into extends | Retain textual/unclassified base |
| C# | Oversized enclosing class; interface-only query of nested interface | Assigns a target ID for the generic name | Retain textual base, no ID |
| C++ | Oversized template function; local struct base | Text only | Same; clean despite the shared helper's discarded metadata |
| All nine generic readers | Same shape with 65,536 parameters | Text only; Swift/Scala unclassified | Same; clean |
| All nine generic readers | Complete index selecting oversized owner | Explicit generic-count error | Same; clean |
| Rust impl consumers | Oversized enclosing function; positive impl of its first parameter, type-only/member-only/outline queries | No type aggregation; member container kind absent; textual outline owner | Same; clean in this shape |
| JavaScript/C/PHP/Perl/Bash/Zsh/Lua | No required explicit generic-list reader | Negative generic matrices pass | Not applicable |

**Required repair:** collect every Scala import expression and both rename
node forms, using grammar boundaries rather than splitting source strings.
Do not turn a finite renamed/excluded selector into a wildcard. Separate
generic-name evidence from conversion to positioned record metadata, so an
unselected oversized owner cannot erase shadowing. Keep explicit errors when
the oversized owner is selected, and the clean Rust/C++ controls above.
Extend the shared path/query/cache matrices with every failing shape, including
the 65,536/65,537 boundary and genuine warm hits. Inspect Rust ownership and
cache-widening consumers of the same helper. Update its error-handling comment
and current lookup documentation, and bump the extractor revision.

### High — Built-in name exclusions erase valid Go and Swift bases

**Defect class:** spelling-based non-relation exclusions ignore shadowing or
failed resolution, deleting a written base whose non-relation meaning has not
been established.

In the tree-hugger library,
[go_bases](../../lib/src/file/extract/relations.rs:460), which reads embedded
named types in interfaces, excludes 19 names before same-file lookup. A local
interface can legally have any of those names. Go's universe declarations
can be shadowed; see [Go declarations and scope](https://go.dev/ref/spec#Declarations_and_scope).

```go
package review
type int interface { M() }
type ReviewChild interface { int }
```

This program compiles. `hug --json-full symbols --no-cache probe.go` reports
no extends relation on ReviewChild. It must retain `int` and resolve the
same-file interface. The failure applies to **every** excluded name:
`bool`, `byte`, `complex64`, `complex128`, `float32`, `float64`, `int`, `int8`,
`int16`, `int32`, `int64`, `rune`, `string`, `uint`, `uint8`, `uint16`,
`uint32`, `uint64`, and `uintptr`.

The tree-hugger library's
[written_bases](../../lib/src/file/tree_file/relations.rs:613), which classifies
written targets and excludes Swift enum raw types, similarly treats an
unresolved standard-looking name as proof of a raw-value type:

```swift
protocol ReviewProtocol {}
typealias Int = ReviewProtocol
enum ReviewChild: Int { case a }
```

This program also type-checks. The extractor drops `Int` completely. Alias
expansion is outside scope, so retaining `Int` in unclassified bases with no
ID is appropriate; deleting the conformance is not. An imported protocol
named `Int` has the same failure. Both failures survive selective queries
and complete warm-cache hits.

The sweep covered all relation readers and both spelling-based exclusions:

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Go interface reader, all 19 names above | Same-file interface given the excluded name; Child embeds it | No relation for every name | Extends entry with the local interface ID |
| Go alias | `type int = ReviewProtocol`; Child embeds int | No relation | Preserve int text; alias stays unresolved |
| Go function-local declaration | Local interface int and local Child | No relation | Preserve and resolve the local interface |
| Go primitive/constraint controls | Unshadowed int, `~string`, union | No relation | Same; clean |
| Swift enum classification, all 14 simple names | Alias to a protocol, separately selective import of a protocol of that name | No relation in every alias/import case | Preserve text without inventing identity/category; use unclassified when category remains unproved |
| Swift qualified exclusion | Local `Unicode.Scalar` protocol alias, separately imported Unicode owner | No relation | Preserve Unicode.Scalar text; no alias/external ID |
| Swift local-protocol controls | Same-file protocol for each of the 14 simple names, plus nested Unicode.Scalar protocol | Implements entry with the protocol ID | Same; clean |
| Swift enum raw-value controls | Unshadowed standard names; explicit raw case; Unicode.Scalar raw case | No relation | Same; clean |
| Swift qualified standard raw type | `enum ReviewChild: Swift.Int { case a }` | Invents unclassified Swift.Int base | Exclude a proven standard raw type; retain a shadowing custom owner if present |
| Swift qualified raw-case control | `Swift.Int` plus `case a = 1` | No relation | Same; clean |
| C# enum reader | Underlying int | No relation | Same; syntax proves an underlying type |
| Rust supertraits | Lifetime and relaxed bounds versus written trait names | Existing positive/negative public matrices pass | Retain the syntactic exclusion rule |
| TypeScript/JavaScript/Java/C++/Scala/PHP | Local and external named bases in language matrices | Written bases retained | Same; no standard-name exclusion in these readers |
| C/Perl/Bash/Zsh/Lua | No required base construct; negative matrices | No invented relations | Same |

The 14 simple Swift names are `Int`, `Int8`, `Int16`, `Int32`, `Int64`, `UInt`,
`UInt8`, `UInt16`, `UInt32`, `UInt64`, `String`, `Character`, `Float`, and
`Double`; `Unicode.Scalar` is the fifteenth, qualified exclusion.

**Required repair:** make exclusions follow binding evidence, not just the
written spelling or the absence of a resolved ID. Preserve text when an
alias/import/ambiguity prevents proof. Reuse the per-file index rather than
adding per-base whole-file scans. Extend permanent relation and warm-cache
matrices for all 19 Go names, all 15 Swift spellings, alias/import/local-scope
shapes, and clean primitive/raw-case controls. Update the exclusion comments
and docs and invalidate prior extraction entries.

### Medium — C# record superclasses remain unclassified

**Defect class:** relation classification relies on a broad symbol kind and
ignores declaration syntax that proves a target's category.

The tree-hugger library's
[written_bases](../../lib/src/file/tree_file/relations.rs:655) recognizes a
resolved Class kind as inheritance, and checks Swift class syntax when its
kind is Type. It has no corresponding C# record-class syntax check. C#
records deliberately remain Type records under this feature's existing-kind
boundary; that does not make their inheritance uncertain. A plain `record`
is a record class, while `record struct` is a value type. See
[Microsoft's record reference](https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/builtin-types/record).

```csharp
record ReviewBase;
record ReviewChild : ReviewBase;
```

Complete, selective, and warm CLI output all put ReviewBase in
`unclassified_bases` with its correct ID. The target's declaration establishes
inheritance, so the entry belongs in `extends`. Consumers currently miss
this inheritance if they read the precise list the author accepted.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| C# implicit record class | `record ReviewBase; record ReviewChild : ReviewBase;` | Unclassified with ID | Extends with the same ID |
| C# explicit record class | `record class ReviewBase;` | Unclassified with ID | Extends |
| C# positional record class | `record ReviewBase(int X = 0);` with a non-invocation base entry | Unclassified with ID | Extends |
| C# generic record class | `record ReviewBase<T>;` with ReviewBase<int> base | Unclassified with ID; arguments retained | Extends; retain written arguments |
| C# qualified record class | Local namespace's ReviewNS.ReviewBase | Unclassified with ID | Extends |
| C# ordinary class/interface controls | Ordinary class target, separately interface target | Extends / implements | Same; clean |
| C# struct/record-struct target controls | Struct spelling in an otherwise parsable base list | Not classified as superclass | Keep that negative classification; these are not legal superclasses |
| C# alias/duplicate controls | Alias requiring expansion; duplicate record declaration | Unclassified, no ID | Same; clean |
| C# owner syntax controls | Struct and record-struct interface lists; primary-constructor base invocation | Existing complete/selective matrix classifies correctly | Same; clean |
| Swift broad Type kind | Local class, struct/actor conformance, enum raw cases, local-owner extension | Existing matrix and supplemental controls classify these forms | Preserve; the enum-exclusion defects are separate above |
| Scala concrete/trait/object owners | Local class first parent, local trait first parent, trait inheritance, object parents | Existing matrix classifies correctly | Preserve; imports are covered by the first finding |
| Other implemented relation readers | Their separate extends/implements syntax | Existing public matrices pass | No mixed-list target classification branch to change |

**Required repair:** classify eligible same-file targets from language
declaration syntax where SymbolKind is broader than the language category.
Handle implicit and explicit record classes without treating record structs
as superclasses or changing public kinds/IDs. Add all five failing target
forms and the negative controls to complete/selective/warm-cache matrices;
verify the resulting outline label and update the mixed-list documentation.
Bump the extractor revision.

### Medium — Current schema examples advertise obsolete versions

**Defect class:** current format examples retain a historical schema number
after a record-shape change, contradicting the serialized output and exact
version cache contract.

The tree-hugger library's
[SchemaVersion::CURRENT](../../lib/src/shared/schema_v2/types.rs:38), used by
records, reports, indexes, and cache entries, is 4.3. The accepted mixed-base
shape already requires at least 4.2; the concurrent callable-complexity shape
advanced it to 4.3. Current report and cache examples still advertise 4.1.
Readers copying them cannot tell which fields exist or which entries the
cache accepts.

Copied the Rust fixture and ran JSON, JSONL, and full-index commands; their
envelopes and records emitted 4.3. The resulting persisted entry also emitted
4.3. The permanent incompatible-version tests confirm that 4.1 is refused.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| [CLI README JSON example](../../cli/README.md:151) | Current report envelope | Example says 4.1; CLI emits 4.3 | Current version, or explicit historical example |
| [Symbol reports JSON example](../../docs/symbol-reports.md:760) | Outer envelope and nested record | Both examples say 4.1; both emitted values are 4.3 | Consistent current versions |
| [Symbol reports field table](../../docs/symbol-reports.md:811) | Described schema_version | Says 4.1 | Describe the current record version |
| [Cache design entry example](../../docs/cache-design.md:235) | Newly written entry | Example says 4.1; writer emits 4.3 | Current cache shape |
| Area README current schema statement | Current public record version | Says 4.3 | Same; clean |
| Schema design version history and source version constants | Explicitly historical 4.0/4.1/4.2 plus current 4.3 | History distinguishes versions | Keep historical numbers |
| Library README and relation documentation | New lists and extension source evidence | Current behavior described | Keep, with source-behavior corrections above |
| Tree-hugger skill and API reference | Current schema plus metric-addition history | Current 4.3; older numbers describe when keys were added | Same; clean |

**Required repair:** update all four current documentation sites together.
Keep intentionally historical release/version descriptions historical. No
schema bump or production code change is needed for this documentation repair.

### Medium — New relation source-span members lack permanent robustness columns

**Defect class:** a newly persisted compound field has a whole-object shape
test but omits field-by-field malformed-input tests for its load-bearing
members.

The tree-hugger CLI's
[cache robustness matrix](../../cli/tests/symbols/cache_robustness.rs:272)
now has a `target source_span` column. It corrupts the whole object, or one
member chosen by map order. It never removes or duplicates an individual
nested field. The tree-hugger library's
[SymbolRef::source_span](../../lib/src/shared/schema_v2/facets.rs:351) is new
persisted evidence, and the CLI's
[header_writes](../../cli/src/render.rs:1010) uses its byte members to decide
whether to suppress a conformance label. The nested values therefore affect
public reporting and need the same matrix discipline as nested generics.

This is a **testing gap, not an observed decoder defect**. The supplemental
sweep started with the real Swift symbol fixture plus a local type and
extension, used the CLI-written extension target's span, renamed a separate
function as a service sentinel, edited one nested field per cell, and asserted
the stored public value or uncached output after rejection. All cells passed.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| source_span.start.line | All nine shapes in the table below | Supplemental checks pass; no permanent column | Permanent public-output column |
| source_span.start.column | Same | Same | Same |
| source_span.end.line | Same | Same | Same |
| source_span.end.column | Same | Same | Same |
| source_span.start_byte | Same | Same | Same, protecting outline suppression evidence |
| source_span.end_byte | Same | Same | Same, protecting outline suppression evidence |
| source_span whole object | Existing nine-shape matrix | Permanent checks pass | Keep |
| Other relation-target fields and all three lists | Existing field-by-field matrix | Permanent checks pass | Keep |
| Nested generic fields and declaration constraints | Existing field-by-field matrix | Permanent checks pass | Keep |
| Metrics, attributes, modifiers, entry metadata | Existing public-result columns | Permanent checks pass | Keep; no new decoder defect found |
| Manifest/lockfile/configuration formats | No affected reader | Not applicable | No additional format matrix |

**Required repair:** add the six nested columns to the existing format-wide
test, with positive controls, a separate service sentinel, and one edit per
cell. Preserve the currently observed strict required-member behavior; no
decoder change is needed. Use a real relation-span control or the existing
populated target control, and retain the permanent full-signature extension
outline check that demonstrates why the byte fields matter. Keep these tests
in the declared, running L1 CLI target.

## Blocked Findings

None. The mixed-list and Swift-extension choice was accepted before the
implementation being reviewed.

## Recurrence

The first finding repeats **review-1, “Relation IDs are assigned without
proving path meaning,”** and the scope-evidence class of **review-2, “Python
nested class scopes still assign incorrect base identities.”** Their listed
reproductions were repaired, but the broader binding-evidence sweep missed
generic-reader failure at the position boundary. The subsequent addition of
Scala's import reader also needed to sweep every import expression, both
rename forms, selector groups, exclusions, and unrelated-name controls.

This review carries the complete nine-reader generic-boundary sweep, every
implemented import-binding reader, all newly failing Scala forms, and Rust
ownership consumers. Repairing one Scala alias or one oversized Python owner
alone would leave the same class present elsewhere.

The source-span coverage finding repeats **review-2, “Generic and base fields
lack a permanent cache robustness matrix.”** That repair delivered the
original generic/list/name/ID/qualified-name columns, but the language-reader
follow-up added a compound target field and stopped the matrix at its object
boundary. It should have swept all six nested members when persisting the
new evidence. This review supplies every shape for every new member.

The Go/Swift spelling exclusions, C# record classification, and current-schema
examples were not titled findings in either earlier review. The earlier
missing-language finding is now unblocked work with new implementation
defects, rather than an unresolved author choice. `recurrence: true` reflects
the repeated relation-identity class, not a request to reopen that choice.

## Input robustness matrix

JSON cache entries are the affected persisted format. No manifest, lockfile,
YAML, TOML, or configuration reader was added by this feature. The permanent
format-wide test starts with a CLI-written Python fixture, populates optional
controls, changes one field per cell, and checks the complete public result.
An independently renamed sentinel distinguishes service from recomputation.
The matrix now covers the columns requested in review-2; it passed here.

In the tables below, **serve** means the stored public value is returned;
**recompute** means rejection and output equal to an uncached run. Collection
absence intentionally means empty under this schema's omitted-empty encoding.
Optional scalar absence and null intentionally both mean no value. The
control row proves service with populated values. Scalar element-error shapes
use a wrong array shape; object element-error shapes corrupt object members.

| Shape | generic_params | Declaration constraints | extends | implements | unclassified_bases |
|---|---|---|---|---|---|
| Populated control | Serve | Serve | Serve | Serve | Serve |
| Absent | Serve empty | Serve empty | Serve empty | Serve empty | Serve empty |
| Null | Recompute | Recompute | Recompute | Recompute | Recompute |
| Wrong whole-field type | Recompute | Recompute | Recompute | Recompute | Recompute |
| One wrong element | Recompute | Recompute | Recompute | Recompute | Recompute |
| Every element wrong | Recompute | Recompute | Recompute | Recompute | Recompute |
| Empty | Serve empty | Serve empty | Serve empty | Serve empty | Serve empty |
| Duplicate key | Recompute | Recompute | Recompute | Recompute | Recompute |
| Trailing invalid content | Recompute | Recompute | Recompute | Recompute | Recompute |

| Shape | Generic name | kind | constraints | default | variance | position |
|---|---|---|---|---|---|---|
| Populated control | Serve | Serve | Serve | Serve | Serve | Serve |
| Absent | Recompute | Recompute | Serve empty | Serve none | Serve none | Recompute |
| Null | Recompute | Recompute | Recompute | Serve none | Serve none | Recompute |
| Wrong whole-field type | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |
| One wrong element | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |
| Every element wrong | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |
| Empty | Serve empty string | Recompute | Serve empty | Serve empty string | Serve empty string | Serve zero |
| Duplicate key | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |
| Trailing invalid content | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |

| Shape | Target name | ID | qualified_name | source_span | metrics.lines |
|---|---|---|---|---|---|
| Populated control | Serve | Serve | Serve | Serve | Serve count |
| Absent | Recompute | Serve none | Serve none | Serve none | Recompute |
| Null | Recompute | Serve none | Serve none | Serve none | Serve null |
| Wrong whole-field type | Recompute | Recompute | Recompute | Recompute | Recompute |
| One wrong element | Recompute | Recompute | Recompute | Recompute | Recompute |
| Every element wrong | Recompute | Recompute | Recompute | Recompute | Recompute |
| Empty | Serve empty string | Serve empty string | Serve empty string | Recompute | Serve zero |
| Duplicate key | Recompute | Recompute | Recompute | Recompute | Recompute |
| Trailing invalid content | Recompute | Recompute | Recompute | Recompute | Recompute |

All three relation lists share the same target decoder. Existing attributes,
modifiers, other metric keys, and entry metadata retain their permanent shape
columns. Negative, fractional, overflowing, and wrong-type line counts are
rejected by the separate numeric sweep. No new malformed-cache coercion was
observed. The remaining defects concern extraction of valid source facts,
and the missing permanent nested-span coverage, not observed service of
damaged JSON.

The six new span-member columns have this observed supplemental contract.
Zero is served literally by the existing unsigned-integer decoder; this table
describes stored-field acceptance, not a claim that source positions are
zero-based. The public result distinguishes it from an absent member.

| Shape | start.line | start.column | end.line | end.column | start_byte | end_byte |
|---|---|---|---|---|---|---|
| Populated control | Serve | Serve | Serve | Serve | Serve | Serve |
| Absent | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |
| Null | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |
| Wrong whole-field type | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |
| One wrong element | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |
| Every element wrong | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |
| Empty numeric control (0) | Serve zero | Serve zero | Serve zero | Serve zero | Serve zero | Serve zero |
| Duplicate key | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |
| Trailing invalid content | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |

## Requirement-to-verification assessment

| User-facing requirement | Strongest relevant verification | Assessment |
|---|---|---|
| Explicit generic ownership, bounds, defaults, variance, packs, positions, and selected-owner overflow | Level 1 complete/selective/warm-hit matrices and boundary tests | Listed repairs pass; enclosing-name evidence is lost on overflow in selective queries |
| Declared bases, source order, full written text, deduplication, conservative identities | Level 1 library matrices plus shipped-CLI complete/selective/warm probes | Import/generic identity defects and spelling-based omission defects above |
| C#/Swift/Scala precise and unclassified categories | Level 1 language matrices, cache/query corpus, supplemental record and enum checks | Readers implemented; C# record classification and Swift exclusions incomplete |
| Swift uniquely local extension attachment, aggregation, conditional-summary boundary | Level 1 extraction/cache matrices and plain/full-signature outlines | Local, external, aliased, actor, repeated, and before-owner controls pass; accepted external-owner loss documented |
| Rust positive/negative/empty impls, aggregation, full self/trait text, member-only ownership, containment widening | Level 1 public-query, adapter, widened-cache, warm-hit checks | Existing repairs and supplemental oversized-owner control pass |
| Written modifiers, raw attributes, ownership, repetitions, fixed ordering | Level 1 sixteen-language matrices and cache/projection checks | No additional defect found |
| Callable body spans/line counts, expression and absent bodies, nested callables, LF/CRLF, half-open boundaries | Level 1 body matrix and formula assertions | Appropriate verification; no additional defect found |
| One definitions capture, optional-pass avoidance, shared indexed relation lookup | Level 1 work counters and growth checks | Appropriate verification; no additional performance finding |
| Schema, exact-version invalidation, required metric keys, malformed-field handling | Level 1 CLI public-output matrices and revision-upgrade tests | Decoder checks pass; nested new source-span members lack permanent columns, and current-format documentation needs correction |
| JSON/JSONL/full indexes, selectors, counts, sorting, signature modes | Level 1 shipped-CLI tests and supplemental probes | Appropriate tier; incorrect extracted facts propagate to reports/cache |
| Styled outline relations and wrapping | Level 2 real-pane capture in tmux and WezTerm, supported by Level 1 output checks | Appropriate tier present; Kitty not exercised |
| Keyboard/mouse/paste/input-encoder behavior | No such behavior added | Level 3 is not required |
| Older-model adapter losses and current topic pages | Level 1 adapter assertions plus documentation sweep | Losses retained; current lookup/category examples must follow the repairs |

The library matrix modules are declared in `tests/l1/main.rs`; CLI cache and
outline modules are declared in `tests/symbols/main.rs`. L2 binaries declare
`terminal-tests`, CI metadata enables it, and the area has a live `test-l2`
recipe. Passing these existing tests does not establish readiness for the
additional source forms they do not yet exercise.
