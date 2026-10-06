---
$schema: feature-review.yaml
ready: false
findings:
  - title: Grouped Scala fields disappear or lose their type annotations
    priority: high
  - title: Declarator attributes turn C++ constructors into methods
    priority: high
  - title: Scala enum cases lose representable payload types
    priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-05T15:41:26-07:00"
spec: 2026-10-05-deferred-kind-precision/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-05-deferred-kind-precision/log.md
description: "A **feature** review of `2026-10-05-deferred-kind-precision/spec.md`"
feature: 2026-10-05-deferred-kind-precision/review-1.md
next: 2026-10-05-deferred-kind-precision/review-2.md
---

# Review: deferred kind precision

The feature is **not production ready**. The ordinary forms are implemented,
but three required declaration shapes lose records, kind precision, or
metadata. Each finding below includes a sweep of the related extraction
paths, including controls that worked.

The review covers the working tree, including the uncommitted unnamed-owner
regression test. The reviewer changed no implementation code. Existing edits
were preserved. A concurrent session subsequently updated doc-comment
extraction, cache revision, documentation, and related tests; the findings'
classification and metadata paths were unchanged. Package checks were run
again after those edits.

## Verification

Run from `tree-hugger/` on the current macOS host:

- `just test`: **997 passed, 0 skipped** initially; **1,001 passed, 0 skipped**
  on the final rerun after concurrent edits, across five binaries.
- `just lint`: passed for `tree-hugger` and `tree-hugger-cli`.
- `just check-tier-coverage tree-hugger`, from the repository root: passed;
  no stranded tests.

Additional reproductions used the built `target/debug/hug`, with
`symbols --file <scratch-file> --json-full --no-cache`. Every source reported
below parsed without syntax diagnostics. Minimal failing examples were also
checked through the tree-hugger package's public `TreeFile::symbols()` API.
The C++ attributed-constructor reproduction passed
`clang++ -std=c++20 -fsyntax-only`.

The first reproductions copied the existing
[Scala fixture](../../lib/tests/fixtures/symbols/scala.scala) and
[C++ fixture](../../lib/tests/fixtures/symbols/cpp.cpp), verified the unedited
controls, then changed one declaration. The sibling sweeps used isolated
copies of the corresponding declaration forms; their exact shapes are in
the instance tables. Results are correctness checks, not timing measurements.

| User-facing requirement | Strongest relevant verification present | Assessment |
|---|---|---|
| Precise kinds, containers, and negative cases for all ten acceptance-table languages | Level 1: `symbols_kinds` and `symbols_kind_coverage`, public library results | Ordinary forms pass; grouped Scala fields and attributed C++ constructors are missing cases |
| Constructor parameters, defaults, access, static flags, headers, and overloads | Level 1: `symbols_deferred_metadata`, plus affected header/access/static/signature suites | Existing assertions pass; the attributed-declarator classification gap remains |
| Alias target text, including generic and grouped forms | Level 1: metadata suite; additional CLI probes for Go, Swift, Scala | Tested targets retain their source text |
| Field-list agreement and retained type annotations | Level 1: `symbols_field_lists`; CLI `classes::go_and_scala_field_lists_agree_with_field_records` | Existing tests use individual Scala names; grouped names fail |
| Variant names, values, and representable payloads | Level 1: `symbols_field_lists::enum_variant_lists_keep_discriminant_text`, metadata/kind suites | Values pass in tested languages; Scala payload types are unasserted and missing |
| Namespace names, repeated declarations, member qualification, and unrelated identities | Level 1: kind/query/owner/identity suites; CLI probes of repeated PHP namespaces and dotted/nested TypeScript namespaces | Tested names and repeated records remain correct |
| Narrow queries agree with complete queries and avoid unnecessary work | Level 1: `symbols_query::deferred_kind_queries_equal_the_same_kind_of_a_full_query`, query-filter tests | Passing comparisons include optional capability sets and work counters |
| Cache revision invalidation, real warm hits, kind narrowing, capability widening, and unfiltered storage | Level 1: `symbols_cache`, CLI `cache_upgrade` | Passing tests assert actual hits and compare cached results with uncached results |
| CLI kind selectors, constructors in class summaries, counts/statistics/JSON agreement | Level 1: CLI `filters`, `classes`, `reports`, and goldens | Existing cases pass; findings below require new regression inputs |

The new library tests are included by the declared `lib/tests/l1/main.rs`
target; the affected CLI modules are in its declared symbols test target.
Their names have no higher-tier marker. Rendering and keyboard handling did
not change, so these data-extraction requirements need Level 1 verification;
no additional Level 2 or Level 3 test is required for this feature.

No manifest, configuration, cache deserializer, or structured-document
reader changed in this feature. The configuration-field robustness matrix
does not apply to these source-declaration query additions.

## Findings

### High — Grouped Scala fields disappear or lose their type annotations

**Defect class:** extraction paths assume a declaration has one name,
silently omitting grouped definitions and dropping the shared type from
later names in grouped declarations.

In the tree-hugger package, the
[Scala definition queries](../../lib/queries/vendor/scala/locals.scm) capture
only an `identifier` directly under a definition's `pattern`. The pinned
grammar instead represents `val a, b: Int = 1` as
`pattern: (identifiers (identifier) (identifier))`. This is a grouped-name
declaration, distinct from the deliberately excluded tuple pattern
`val (a, b) = ...`.

For declarations without an initializer, the queries do capture every
`name`, but the tree-hugger package's
[extract_scala_body_fields](../../lib/src/file/extract/type_metadata.rs)
reads only `child_by_field_name("name")`, which returns the first one.
Its shared
[align_field_metadata](../../lib/src/file/tree_file/definitions.rs)
adds the other records to the list with `type_annotation: None`. Therefore
`hug classes` loses written type information even when `hug symbols`
contains both names. The spec requires grouped names individually and
completed field lists without lost annotations.

**Fixture reproduction:** in a copy of `scala.scala`, replace only
`val publicField: Int = 1` with
`val publicField, secondField: Int = 1`. The control reports `publicField`
as a Field and lists its `Int` type under `PlainWidget`. The edited copy
reports neither field and removes the original field from the owner's list.

**Instance sweep:** each Scala row was tested for both `val` and `var`.
The single-name versions were positive controls.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Scala class-body definition | `class C { val a, b: Int = 1 }`, and `var` | Neither Field nor field-list entry exists | Two Fields; both listed with `Int` |
| Scala trait-body definition | `trait C { val a, b: Int = 1 }`, and `var` | Neither Field nor field-list entry exists | Two Fields; both listed with `Int` |
| Scala object-body definition | `object C { val a, b: Int = 1 }`, and `var` | Neither Field exists | Two static Fields; retain the established Module payload policy |
| Scala enum-body definition | `enum C { case V; val a, b: Int = 1 }`, and `var` | Neither Field exists | Two instance Fields; retain the established Enum variant-list policy |
| Scala class-body declaration | `class C { val a, b: Int }`, and `var` | Both Fields exist; only `a` has `Int` in the owner's field list | Both list entries retain `Int` |
| Scala trait-body declaration | `trait C { val a, b: Int }`, and `var` | Both Fields exist; only `a` has `Int` in the owner's field list | Both list entries retain `Int` |
| Scala controls, all four owners | Single-name initialized `val`/`var`; class/trait single-name declarations | Field retained; class/trait list retains its type | Same; clean |
| Go struct query and reader | `type C struct { a, b int }` | Both Fields and both `int` list entries | Same; clean |
| C struct query and reader | `struct C { int a, b; };` | Both Fields and both typed list entries | Same; clean |
| C++ struct query and reader | `struct C { int a, b; };` | Both Fields and both typed list entries | Same; clean |
| Java class query and reader | `class C { int a, b; }` | Both Fields and both typed list entries | Same; clean |
| C# class query and reader | `class C { int a, b; }` | Both Fields and both typed list entries | Same; clean |
| PHP class query and reader | `class C { public int $a, $b; }` | Both Fields and both typed list entries | Same; clean |
| Swift class query and reader | `class C { var a: Int = 1, b: Int = 2 }` | Both Fields; public API field list retains both `Int` annotations | Same; clean; existing Type record limitation retained |
| Rust struct query and reader | `struct C { a: i32, b: i32 }` | Both Fields and both typed list entries | Same; clean; language has separate field declarations |
| Python class query and reader | Class-body `a: int = 1` and `b: int = 2` | Both Fields and typed list entries | Same; clean |
| TypeScript class query and reader | `class C { a: number; b: number; }` | Both Properties and typed list entries | Same; clean |
| JavaScript class query and reader | `class C { a = 1; b = 2; }` | Both Properties and list entries without invented types | Same; clean |

These enumerate the language field readers dispatched by
`extract_type_metadata`; Bash, Zsh, Perl, and Lua have no corresponding
type-field reader. Module and Enum record schemas do not carry an ordinary
field list; their existing limitations are not additional findings here.

**Correction:** capture every direct name in the Scala `identifiers` form
and enumerate all declaration `name` fields in the metadata reader, copying
the shared annotation to each entry. Keep tuple/destructuring exclusions.
Extend the existing Level 1 kind and field-list matrices across owner kinds
and both keywords, and add a CLI class-summary assertion with grouped typed
names. Single-name positive tests and narrow/full equality cannot detect a
record that both extraction paths omit.

### High — Declarator attributes turn C++ constructors into methods

**Defect class:** constructor detection assumes an unwrapped identifier
where a valid declarator can wrap the same name in attributes.

In the tree-hugger package,
[is_constructor_declaration](../../lib/src/file/extract/classify.rs)
requires the function declarator's immediate declared child to be an
`identifier` equal to the capture. `C [[deprecated]] (int x);` introduces
an `attributed_declarator` at that point. The shared declarator logic still
finds its name and parameters, but this separate kind check rejects it.
The record becomes Method, so `hug functions --kind constructor` excludes
a valid explicit instance constructor and `--kind method` includes it.
On the edited fixture, filtering the exact name `AccessWidget` with
`--count --no-cache` returned **0** for Constructor and **1** for Method.

**Fixture reproduction:** copy `cpp.cpp` and replace only
`AccessWidget();` with `AccessWidget [[deprecated]] ();`. The unedited
record is Constructor; the edited record is Method. Both files parse cleanly.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| C++ in-class declaration | `C [[deprecated]] ();` | Method | Constructor |
| C++ in-class definition | `C [[deprecated]] (int x) {}` | Method with parameter `x` | Constructor retaining `x` |
| C++ defaulted constructor | `C [[deprecated]] () = default;` | Method | Constructor |
| C++ deleted constructor | `C [[deprecated]] (int x) = delete;` | Method | Constructor retaining `x` |
| C++ template-wrapped constructor | `template<class T> C [[deprecated]] (T x) {}` | Method with parameter `x` | Constructor retaining `x` |
| C++ explicit constructor | `explicit C [[deprecated]] (int x);` | Method | Constructor retaining `x` |
| C++ leading-attribute control | `[[deprecated]] C(int x);` | Constructor with `x` | Same; clean |
| C++ plain control | `C();` | Constructor | Same; clean |
| Python constructor detector | Decorated class `def __init__(self, x: int = 1)` | Constructor, parameters retained | Same; clean |
| PHP constructor detector | `#[Deprecated] function __construct(int $x) {}` in class | Constructor, parameter retained | Same; clean |
| Java constructor capture | `@Deprecated C(int x) {}` in class | Constructor, parameter retained | Same; clean |
| C# constructor capture | `[Obsolete] C(int x) {}` in class | Constructor, parameter retained | Same; clean |
| Swift constructor capture | `@available(*, deprecated) init(x: Int) {}` in class | Constructor, parameter retained | Same; clean |
| JavaScript constructor detector | Class `constructor(x) {}` | Constructor, parameter retained | Same; clean |
| TypeScript constructor detector | Class `constructor(x: number) {}` | Constructor, parameter retained | Same; clean |

These cover every language with constructor classification or captures.
The other eight supported languages have no constructor path to sweep.
All C++ forms in the table were tested inside a class with public access.

**Correction:** use the existing shared C/C++ declarator traversal to
identify the declared name through permitted wrappers. Preserve the
no-return-type and direct-owner requirements and the exclusions for
destructors, conversion operators, and out-of-class definitions. Add these
forms to existing Level 1 kind/metadata tests and verify both CLI kind
selectors. Broadening the check to any nested identifier would accidentally
accept the excluded forms.

### High — Scala enum cases lose representable payload types

**Defect class:** enum-list completion synthesizes unit variants for
parameterized declarations without reading payload data the schema can retain.

The tree-hugger package's
[extract_scala_enum_cases](../../lib/src/file/extract/type_metadata.rs)
handles older bare-name shapes but never visits a `full_enum_case` and its
`class_parameters`. The new standalone variant captures do find
`case Value(x: Int, y: String)`, but
[align_variant_metadata](../../lib/src/file/tree_file/definitions.rs)
constructs `VariantInfo::unit` for every standalone record. The final
owner list therefore holds only `{"name":"Value"}`.

The tree-hugger package's existing
[EnumVariantData](../../lib/src/shared/schema_v2/kinds.rs) supports
`tuple_fields` and `struct_fields`, and its adapter preserves those fields.
Retaining declared payload types requires no record-schema extension.
The spec expressly requires payload that the existing enum metadata can
express. Standalone EnumVariant `kind_data: Unknown` remains correct; this
finding concerns the owning enum's list.

**Fixture reproduction:** copy `scala.scala` and change only the enum case
declaration from `case Red, Green` to
`case Red, Green` followed by `case Value(x: Int, y: String)` on the next
indented line. The control correctly lists Red and Green. The edited copy
adds the Value record and list entry but retains neither payload type.
The pinned grammar exposes both typed `class_parameter` nodes without errors.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Scala enum reader plus reconciliation | `case Value(x: Int, y: String)` | Owner entry has only `name` | Retain `Int` and `String` in existing payload fields |
| Scala enum reader plus reconciliation | `case Value(x: Int = 1, y: List[String])` | Owner entry has only `name` | Retain `Int` and `List[String]`; do not invent a discriminant |
| Scala generic enum reader plus reconciliation | `enum E[T] { case Value(x: T) }` | Owner entry has only `name` | Retain unresolved payload type `T` |
| Scala unit-case control | `case Empty`, and grouped `case Red, Green` | Unit entries, once each | Same; clean |
| Rust enum reader plus reconciliation | `Value(i32, String)` and `Value { x: i32, y: String }` | Tuple and named payload types retained | Same; clean |
| TypeScript enum reader plus reconciliation | Bare and quoted members; `B = 2` | Names and discriminant retained | Same; clean; no declaration payload |
| C enum reader plus reconciliation | `enum E { A, B = 2 };` | Names and discriminant retained | Same; clean |
| C++ enum reader plus reconciliation | `enum class E { A, B = 2 };` | Names and discriminant retained | Same; clean |
| C# enum reader plus reconciliation | `enum E { A, B = 2 }` | Names and discriminant retained | Same; clean |
| PHP enum reader plus reconciliation | Backed cases `A = 1`, `B = 2` | Names and discriminants retained | Same; clean |
| Java enum reader plus reconciliation | `A, B(2)` with explicit constructors | Names retained; no invented discriminant or declaration payload | Same; clean; constructor argument values are not payload type declarations |
| Swift enum reader/record projection | Unit case and `Value(Int, String)` | Owner remains Type with Unknown record payload | Existing documented owner limitation; outside required type reclassification |

These enumerate every enum-variant metadata reader. Other languages have
no such reader. The failing Scala data is already absent in
`TreeFile::symbols()`, so the adapter and CLI cannot recover it;
the Rust controls prove the existing owner payload survives projection.

**Correction:** read the full Scala case's declared parameter types when
building the owner's variant list, then let reconciliation preserve them.
Keep default argument values separate from discriminants, and preserve the
standalone Parameter records. Extend existing Level 1 metadata/list tests
to assert payloads on `symbols()`, complete records, the adapter round trip,
and a narrowed enum query; include warm-cache equality for the repaired
metadata. Assert data contents, not just variant names or counts.

## Remaining scope notes

The implementation documents the pre-existing Go interface reader's
`method_spec`/`method_elem` mismatch: interface methods are absent from its
field list. That omission predates this feature, is recorded in its
implementation log, and is not counted as a new blocking finding here.
The retained Go/Swift Type classifications and broader-constructor limits
also remain within the specification's recommended boundaries.

No human decision blocks these three corrections. Leave the feature in its
active directory, fix the extraction paths and corresponding regression
coverage, bump the extractor revision for changed cached records, and rerun
the package checks before the next implementation review.
