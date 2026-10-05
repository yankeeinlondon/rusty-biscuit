---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-05T16:39:28-07:00"
spec: 2026-10-05-deferred-kind-precision/spec.md
implemented: false
description: "A **feature** review of `2026-10-05-deferred-kind-precision/spec.md`"
feature: 2026-10-05-deferred-kind-precision/review-2.md
previous: 2026-10-05-deferred-kind-precision/review-1.md
---

# Review #2: deferred kind precision

The feature is **production ready within the specification's boundaries**.
All three findings from review #1 are implemented, and the follow-up sweeps
found no remaining defect in those classes. No human decision is required.

This review covers the current working tree, including the repairs recorded
in the implementation log and the existing concurrent documentation changes.
It checks the specification, the earlier review, the extraction paths,
public-result assertions, cache behavior, CLI selectors, and current docs.
Only review documents and the requested frontmatter were changed by this
review.

## Previous findings

Review #1 has one Findings section rather than separate Unblocked Findings
and Blocked Findings sections. All three findings were unblocked; its
frontmatter records `human_review: false`, `has_blocked_findings: false`, and
`blocked: false`. There were no blocked findings to unblock before this repair.

| Earlier finding | Implementation and verification | Result |
|---|---|---|
| Grouped Scala fields disappear or lose their type annotations | The Scala query captures every grouped identifier. The field reader enumerates every declaration name and copies its shared type. Tests cover class, trait, object, and enum owners; both keywords; local and tuple exclusions; headers; static flags; typed class summaries; narrow queries; and cache reuse. | Resolved |
| Declarator attributes turn C++ constructors into methods | Constructor classification uses the shared declarator traversal through attributes and parentheses, preserving owner and return-type exclusions. Tests check the six reported forms, parenthesized constructors, metadata, negative cases, attached deprecation attributes, and both CLI kind selectors. | Resolved |
| Scala enum cases lose representable payload types | The Scala enum reader visits the grammar's case wrapper and reads full-case parameter clauses into named payload fields. Tests assert written types, defaults, generics, brace/indentation syntax, adapter conversion, narrowed enum queries, CLI JSON, and actual warm-cache hits. | Resolved |

## Reproduction and sibling sweeps

The shipped CLI was rebuilt with `cargo build --color=never -p tree-hugger-cli`.
Probes used `target/debug/hug symbols --file <scratch-file> --json-full --no-cache`.
The first three reproductions copied the existing
[Scala fixture](../../lib/tests/fixtures/symbols/scala.scala) and
[C++ fixture](../../lib/tests/fixtures/symbols/cpp.cpp), checked the unedited
controls, and changed one declaration exactly as in review #1:

- Group `publicField, secondField: Int = 1`: both Field records and both
  typed entries now appear under `PlainWidget`.
- Add `[[deprecated]]` after `AccessWidget`: the record remains Constructor.
- Add `Value(x: Int, y: String)` after the unit Scala cases: the owning enum
  now lists both named payload fields with their written types.

The tables below enumerate the sibling readers and detectors checked through
the same CLI result. Existing public-library tests additionally check the
metadata projections and exclusions. An expected limitation is identified
explicitly rather than treated as a successful payload extraction.

### Grouped-name extraction and field readers

Defect class checked: a declaration reader assumes one name and omits later
names or loses their shared type annotation.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Scala class query and reader | Grouped `val` and `var`, initialized and declared | Both Field records; shared type retained in list | Same; clean |
| Scala trait query and reader | Grouped `val` and `var`, initialized and declared | Both Field records; shared type retained in list | Same; clean |
| Scala object query | Grouped initialized `val` and `var` | Both Field records; static flags covered by library tests | Same; Module payload policy retained |
| Scala enum query | Grouped initialized `val` and `var` | Both Field records; instance flags covered by library tests | Same; ordinary field list remains outside Enum payload |
| Go struct query and reader | `type C struct { a, b int }` | Both Field records; typed list covered by field-list tests | Same; clean |
| C struct query and reader | `struct C { int a, b; };` | Both Field records and typed entries | Same; clean |
| C++ struct query and reader | `struct C { int a, b; };` | Both Field records and typed entries | Same; clean |
| Java class query and reader | `class C { int a, b; }` | Both Field records and typed entries | Same; clean |
| C# class query and reader | `class C { int a, b; }` | Both Field records and typed entries | Same; clean |
| PHP class query and reader | `public int $a, $b;` | Both Field records and typed entries | Same; clean |
| Swift class query and reader | `var a: Int = 1, b: Int = 2` | Both Field records; library tests check typed lists | Same; existing Type record limitation retained |
| Rust struct query and reader | `a: i32, b: i32` | Both Field records and typed entries | Same; separate field declarations |
| Python class query and reader | Two annotated class assignments | Both Field records and typed entries | Same; separate assignments |
| TypeScript class query and reader | `a: number; b: number;` | Both Property records and typed entries | Same; separate property declarations |
| JavaScript class query and reader | `a = 1; b = 2;` | Both Property records; no invented type | Same; clean |

These cover all languages dispatched by the type-field reader. Lua, Perl,
Bash, and Zsh have no corresponding type-field reader. Scala tuple patterns
remain excluded, and grouped method locals remain Variables in the regression
tests. The documented Scala grammar ambiguity for adjacent uninitialized and
untyped declarations remains a demonstrated grammar limitation.

### Constructor detectors and captures

Defect class checked: a detector rejects a declared constructor name because
syntax wrappers separate it from its function declarator.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| C++ in-class declaration | `C [[deprecated]] ();` | Constructor | Same; clean |
| C++ in-class definition | `C [[deprecated]] (int x) {}` | Constructor with `x` | Same; clean |
| C++ defaulted constructor | `C [[deprecated]] () = default;` | Constructor | Same; clean |
| C++ deleted constructor | `C [[deprecated]] (int x) = delete;` | Constructor with `x` | Same; clean |
| C++ template wrapper | `template<class T> C [[deprecated]] (T x) {}` | Constructor with `x` | Same; clean |
| C++ explicit constructor | `explicit C [[deprecated]] (int x);` | Constructor with `x` | Same; clean |
| C++ parenthesized name | `(C)(int x);` | Constructor with `x` | Same; additional sibling repaired |
| C++ controls | Leading `[[deprecated]] C(int x);` and plain `C();` | Constructor | Same; clean |
| Python detector | Decorated `__init__(self, x: int = 1)` | Constructor | Same; clean |
| PHP detector | Attributed `__construct(int $x)` | Constructor | Same; clean |
| Java capture | Annotated `C(int x) {}` | Constructor | Same; clean |
| C# capture | Attributed `C(int x) {}` | Constructor | Same; clean |
| Swift capture | Attributed `init(x: Int) {}` | Constructor | Same; clean |
| JavaScript detector | Class `constructor(x) {}` | Constructor | Same; clean |
| TypeScript detector | Class `constructor(x: number) {}` | Constructor | Same; clean |

These cover every constructor path. The other eight languages have none.
Negative regression rows keep destructors, conversion operators, methods
with return types, and out-of-class C++ definitions from becoming
Constructors. Additional C# probes with `static`, `unsafe static`, and
`static unsafe` constructors produced no Constructor; an instance constructor
with an expression body did. Attribute tests distinguish attributes belonging
to the declared entity from attributes belonging to its type and from
attributes belonging to another name in a grouped declaration.

### Enum readers and variant-list completion

Defect class checked: list completion creates unit variants after a reader
omits representable declaration payloads.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Scala reader and reconciliation | Typed full case, default argument, generic `T`, grouped and unit cases | Named payload types retained; no default becomes a discriminant | Same; clean |
| Scala multiple-clause reader | Curried parameters and named `using` parameters, via public-library/cache tests | Every clause retained in order | Same; documented flattening policy |
| Rust reader and reconciliation | Tuple and named payload variants | Tuple and named types retained | Same; clean |
| TypeScript reader and reconciliation | Bare member and `B = 2` | Names and written discriminant retained | Same; clean |
| C reader and reconciliation | `enum E { A, B = 2 };` | Names and written discriminant retained | Same; clean |
| C++ reader and reconciliation | Scoped enum with `B = 2` | Names and written discriminant retained | Same; clean |
| C# reader and reconciliation | Enum with `B = 2` | Names and written discriminant retained | Same; clean |
| PHP reader and reconciliation | Backed cases with values `1`, `2` | Names and written discriminants retained | Same; clean |
| Java reader and reconciliation | Unit constant and constant with constructor argument | Names retained; no invented declaration payload or discriminant | Same; clean |
| Swift reader and record projection | Unit and associated-value case | Owner remains Type with Unknown record payload | Existing documented owner limitation; outside required reclassification |

These enumerate all enum metadata readers. Shared reconciliation preserves
the reader's payload when pairing it with a standalone variant; the adapter,
narrow queries, CLI projection, and cache tests verify that the payload
survives subsequent processing. Standalone EnumVariant records correctly
retain Unknown kind data without becoming Unknown symbols.

## Requirement and test-level assessment

| User-facing requirement | Strongest relevant verification | Assessment |
|---|---|---|
| Precise kinds, containers, grouped names, and negative cases across the acceptance languages | Level 1: `symbols_kinds`, `symbols_kind_coverage`, and CLI probes | Required forms covered; declared grammar limits remain explicit |
| Constructor parameters, defaults, access, instance/static flags, attributes, headers, and overloads | Level 1: `symbols_deferred_metadata`, declarator-wrapper, header, signature, visibility, and static suites; CLI filters | Appropriate for extraction and selector behavior |
| Alias targets, including grouped and generic forms | Level 1: metadata tests; additional Go, Scala, and Swift CLI probes | Written target text retained; abstract Scala type remains excluded |
| Field records agree with typed owner lists and class summaries | Level 1: `symbols_field_lists`, static/header tests, CLI `classes` | Grouped Scala repairs verified on public results |
| Variant names, values, and representable payloads | Level 1: field-list payload/discriminant tests, adapter conversion, narrowed query, CLI JSON, warm-cache test | Payload contents asserted rather than only names or counts |
| Namespace names, repeated declarations, qualification, and identity | Level 1: kind/metadata/name/identity suites; repeated dotted TypeScript and both PHP namespace-form probes | Repeated records retained; members qualified once |
| Narrow and complete queries agree; unnecessary analysis stays skipped | Level 1: `symbols_query` with three capability sets and work counters | One definitions capture; no unwanted diagnostics or optional work |
| Cache invalidation, actual hits, narrowing, widening, and unfiltered storage | Level 1: `symbols_cache`, CLI `cache_upgrade` | Revision 17 invalidates earlier records; warm hits and stored coverage asserted |
| CLI command sets, kind selectors, constructors in class summaries, counts, statistics, and JSON | Level 1: CLI `filters`, `classes`, `reports`, and golden assertions | Commands consume the same records |

The library's `autotests = false` target includes all affected modules in
[main.rs](../../lib/tests/l1/main.rs). The CLI tests are included by its
[symbols target](../../cli/tests/symbols/main.rs). These tests have ordinary
Level 1 names, and the tier audit reports no stranded tests. This feature
changes source extraction and report data, with no new rendering or keyboard
behavior requiring Level 2 or Level 3 verification.

No configuration, manifest, lockfile, or cache deserializer contract changed.
The configuration-field input robustness matrix therefore does not apply.
Source grammar cases are checked through public symbol results instead.

## Verification

Package checks ran on the macOS host. Cross-OS evidence is left to CI and is
not a readiness gate for this review.

- `just lint`: passed for both packages.
- `just check-tier-coverage tree-hugger`: passed; zero stranded tests.
- `RAYON_NUM_THREADS=2 just test --test-threads 2`: **1,021 passed, 0 skipped**
  across five binaries; 372.241 seconds.

The initial `just test` run passed 554 tests and timed out in the existing
`file_identity::god_files_paths_are_relative_to_any_spelling_of_the_root`
test; fail-fast left 466 tests unrun. The follow-up used
`RAYON_NUM_THREADS=2 just test --test-threads 2` to check the entire area with
less contention. That same test passed in 13.612 seconds, and the entire
rerun passed. The initial timeout is recorded here rather than counted as a
passing run.

Current documentation describes grouped Scala fields, wrapped C++
constructors and entity attributes, Scala case payloads and parameter-clause
flattening, and the current extractor revision. The existing Go interface
field-list omission and Go/Swift Type classifications remain the previously
documented scope limits; they are not new findings.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No finding remains or repeats a prior defect class. The only earlier review
in this feature directory is review #1; all three of its classes were swept
again as listed above. `recurrence` is `false`.
