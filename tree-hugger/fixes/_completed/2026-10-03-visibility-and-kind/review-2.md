---
$schema: feature-review.yaml
ready: false
findings:
  - title: C++ linkage matching still includes parameter names and namespace spelling
    priority: high
  - title: Conditional wrappers still hide C# access levels and field lists
    priority: high
  - title: Field enumeration drops supported members from symbols and class summaries
    priority: high
  - title: The unchanged extractor revision lets the cache retain repaired defects
    priority: high
  - title: Persisted symbol input lacks a public-result robustness matrix test
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-03T21:32:13-07:00"
spec: 2026-10-03-visibility-and-kind/spec.md
implemented: true
next: 2026-10-03-visibility-and-kind/review-3.md
description: "A **fix** review of `2026-10-03-visibility-and-kind/spec.md`"
fix: 2026-10-03-visibility-and-kind/review-2.md
previous: 2026-10-03-visibility-and-kind/review-1.md
---

The fix is **not production ready**. The examples reported in review 1 are repaired, but the linkage defect has additional instances, supported fields still disappear from output, and unchanged cache keys can conceal the repairs. No finding requires a human decision.

## Previous review and blocked findings

Review 1 has four findings under `Findings`, rather than separate `Unblocked Findings` and `Blocked Findings` sections. All four were unblocked; `human_review` was false. There are no previously blocked findings or intervening human decisions to account for.

| Review 1 finding | Verification of the reported instances | Status |
|---|---|---|
| Earlier static declarations do not reliably reach later C and C++ functions | Multiple declarators, default arguments, and function templates now retain ModulePrivate; overload and namespace controls remain Public | Reported examples repaired; additional instances below |
| C++ template wrappers break member kinds and static flags | Function and variable templates retain their member kind and member-static flag; nested preprocessor/template wrappers also pass | Reported examples repaired |
| C# local functions are incorrectly listed as class methods | Ordinary, static, expression-bodied, accessor-local, and constructor-local functions are Function records and excluded from class method lists | Reported examples repaired |
| Python and Scala infer static membership from unrelated syntax | Decorator decoys remain instance methods; actual static decorators work; Scala traits and nested classes do not inherit an object's static flag | Reported examples repaired |

The added local-type, unnamed-owner, Scala enum/given/extension, typed Rust receiver, and initializer-local regressions are registered in the library's consolidated L1 target and passed. The CLI regressions are in its discovered `symbols` test binary.

## Verification

Using the tree-hugger and rust-testing skills, I read the specification, the earlier review, the implementation log, the changed readers, the adapter and CLI projections, the tests, and the current documentation. On this checkout:

- `just test` in `tree-hugger/`: **681 passed, 0 skipped**.
- `just lint` in `tree-hugger/`: passed.
- `just check-tier-coverage tree-hugger` at the repository root: **0 stranded tests**.
- `cargo build --color=never -p tree-hugger-cli`: passed, supplying the current `target/debug/hug` for the probes.

The additional checks copied the primary fixtures from `lib/tests/fixtures/symbols/` into scratch Git repositories, appended the declarations described below, and ran the shipped CLI with isolated home, configuration, and temporary cache directories. Unedited fixture controls were checked first. `hug --json symbols --no-cache <file>` supplied both the legacy symbols and full records; `hug --plain classes --no-cache <file>` confirmed the affected class lists. These are review probes, not newly committed regression tests.

## Unblocked Findings

### High — C++ linkage matching still includes parameter names and namespace spelling

**Defect class:** textual function identity retains syntax that does not change the enclosing namespace or parameter types, so equivalent declarations fail the earlier-static lookup.

In the tree-hugger library, [parameter_types](../../lib/src/file/extract/visibility/c.rs:231) removes only the outer parameter's name. Names inside function-pointer or function-reference parameter types remain. Comments inside a parameter remain too. [namespace_path](../../lib/src/file/extract/visibility/c.rs:209) stores one string per namespace syntax node, so `namespace a::b` and `namespace a { namespace b { ... } }` have different identities.

```cpp
static int callback(void (*)(int));
int callback(void (*cb)(int value)) { return 0; }

namespace a { namespace b { static int helper(int); } }
namespace a::b { int helper(int value) { return value; } }
```

Both later definitions report Public instead of ModulePrivate. This misstates whether a function is visible outside its translation unit. No typedef or cross-file resolution is needed to match these declarations.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| C++ parameter identity | Function-pointer parameter, inner parameter unnamed in prototype and named in definition | Definition Public | ModulePrivate |
| C++ parameter identity | Function-reference parameter, same inner-name difference | Definition Public | ModulePrivate |
| C++ parameter identity | Nested function-pointer parameters acquire names at two levels | Definition Public | ModulePrivate |
| C++ parameter identity | `int` prototype; `int /* explanation */ value` definition | Definition Public | ModulePrivate |
| C++ namespace identity | Earlier nested namespace blocks; later equivalent `a::b` block | Definition Public | ModulePrivate |
| C++ controls | Callback inner names unchanged, or absent in both declarations | ModulePrivate | Same; clean |
| C++ controls | Array parameter retains the same extent; outer name changes | ModulePrivate | Same; clean |
| C++ repaired controls | Simple parameter, multiple declarators, default argument, function template | ModulePrivate | Same; clean |
| C++ isolation controls | Different parameter type or genuinely different namespace | Public | Same; clean; passing regressions retain both |
| C shared linkage reader | Callback parameter gains inner names; parameter gains a comment; multiple declarators | ModulePrivate | Same; clean: C identity uses the name alone |
| Other 14 language readers | Existing fixture visibility matrices | Pass | Clean; they do not perform this C/C++ linkage scan |

Normalize namespace segments independently of block spelling. Remove names at every nested parameter level and comments from parameter-type text, while retaining type tokens, array extents, and overload distinctions. Extend the existing public-library linkage matrix with every failing row and retain a representative shipped-CLI check.

### High — Conditional wrappers still hide C# access levels and field lists

**Defect class:** transparent conditional-compilation wrappers are handled by classification but still treated as boundaries by access and field-list readers.

The tree-hugger library's [extract_csharp_declaration_visibility](../../lib/src/file/extract/visibility/csharp.rs:52) reads the declaration's immediate parent, while [extract_csharp_class_fields](../../lib/src/file/extract/type_metadata.rs:1163) and [extract_csharp_interface_properties](../../lib/src/file/extract/type_metadata.rs:1247) inspect only direct body children. C# also uses `preproc_if`/`preproc_elif`/`preproc_else` wrappers. The shared member classifier looks through them, but these readers do not.

```csharp
class Conditional {
#if FLAG
    static int count;
    void tick() {}
#endif
}
interface ConditionalInterface {
#if FLAG
    int Count { get; }
    void Tick();
#endif
}
```

`count` and `tick` report Unknown access instead of Private. The interface members report Unknown instead of Public. The field and interface property disappear from `hug classes`. This occurs with ordinary valid conditional-compilation syntax, even though their kinds, containers, and explicit static flags survive.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| C# default access reader | Unmarked class field, static field, property, method, static method, and nested class under `#if` | Unknown on every declaration | Private |
| C# default access reader | Unmarked class fields under `#elif` and `#else` | Unknown | Private |
| C# default access reader | Unmarked struct and record field/property/method under `#if` | Unknown | Private |
| C# default access reader | Unmarked top-level class under `#if` or `#else`, after the fixture's file-scoped namespace | Unknown | Internal |
| C# namespace default reader | Unmarked class under `#if` in a block namespace | Unknown | Internal |
| C# interface default reader | Unmarked property and method under `#if` | Unknown | Public |
| C# class field reader | Instance/static fields under `#if` and fields under `#elif`/`#else` | Missing from metadata and class fields | Listed with correct access/static flags |
| C# struct field reader | Ordinary field under `#if` | Missing from metadata and class fields | Listed as Private |
| C# interface property reader | Property requirement under `#if` | Missing from metadata and class fields | Listed as Public |
| C# explicit-modifier control | `public static` field/property under `#if` | Correct Public/static standalone records; still absent from metadata | Access/static clean; field-list failure remains |
| C# classifier/static/container controls | All conditional members above | Correct kinds, static flags, and owners | Same; clean |
| C# unwrapped control | Unmarked ordinary class field | Private, listed | Same; clean |
| C++ access and field readers | Unmarked class fields/static field/method under a conditional; nested conditional/template wrappers | Private, correct static flags, fields listed | Same; clean |
| C field reader | Struct field under `#ifdef` | Unknown, listed | Same; clean |
| Swift access and field readers | Fields/static field/method under `#if` in a private struct | ModulePrivate, correct static flags, fields listed | Same; clean |
| Other 12 languages | Existing fixture and member-wrapper controls | Pass | No equivalent conditional-compilation wrapper participates in these readers |

Use the shared wrapper rule for C# owner/default-access lookup and for class/interface field enumeration. Check every branch form, and extend the public-result matrix with each default-access category above. Reuse the existing class/static controls to keep kind and storage semantics intact. Apply the reader fix consistently to class, struct, record, interface, and namespace contexts; the separate field finding covers omissions that also occur without a wrapper.

### High — Field enumeration drops supported members from symbols and class summaries

**Defect class:** field discovery and field-list projection enumerate only a subset of valid owner, declarator, or binding forms, causing supported members to disappear or disagree between output paths.

In the tree-hugger library, [extract_type_metadata](../../lib/src/file/extract/type_metadata.rs:42) dispatches to independent language readers. Several omit owner kinds or inspect only the first binding. [align_field_metadata](../../lib/src/file/tree_file.rs:1991) corrects flags on existing metadata entries but cannot add missing fields. In tree-hugger-cli, [extract_class_summaries](../../cli/src/main.rs:2849) takes its field lists exclusively from that metadata, so the omissions reach `hug classes` even when standalone records are correct.

This is visible in the unedited primary fixtures: JavaScript's class-body properties, TypeScript's hash-private property, C#'s `Count`/`Total` properties, and the Rust/C/C++ union fields are already absent from their class field lists. The positive adapter round-trip tests preserve those incomplete lists; they do not establish completeness against the source declarations.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| JavaScript metadata dispatcher | Ordinary, hash-private, static, and uninitialized class-body properties | Standalone Property records; no metadata or class fields | All declared properties listed, with their access and static flags |
| TypeScript type reader | Ordinary/static fields of an abstract class | Standalone properties; no class fields | Both fields listed |
| TypeScript class field reader | Instance and static `#private` properties | Standalone properties correct; absent from class fields | Both listed as Private in the appropriate sections |
| TypeScript query/name readers | Quoted class properties and quoted interface property signatures | Not reported as symbols or metadata fields | Property records and field-list entries, consistent with unquoted declarations |
| Rust type reader | Named union fields, including an array-typed field | Standalone fields correct; union metadata/class field list empty | Union fields listed |
| C and C++ type readers | Named union fields | Standalone fields correct; union metadata/class field list empty | Union fields listed |
| C and C++ field readers | Anonymous union members promoted into the enclosing named type | Standalone fields name the enclosing type; metadata lists only its direct scalar field | Promoted fields included in that type's field list |
| C and C++ query/declarator readers | Array fields, pointer-array fields, and function-pointer fields | Absent from standalone symbols and metadata | Field records and field-list entries |
| C++ query/declarator readers | Reference field and static array field | Absent from standalone symbols and metadata | Field records; static array in Static Fields |
| C and C++ queries | Double-pointer field | Metadata contains it; standalone symbol absent | Field record matching metadata |
| C++ template field reader | Static variable template with an array declarator | Standalone Field with member-static flag; absent from metadata/class fields | Listed under Static Fields |
| C# class/struct field reader | Ordinary and static properties | Standalone Property records; no property entries in class fields | Both listed with correct flags; preserve setter-only access text |
| C# record type reader | Explicit body field, static field, and property | Standalone members; record field list empty | All explicit members listed |
| Java record type reader | Explicit static field in a record body | Standalone Field; metadata lists only record components | Body field listed under Static Fields, alongside existing component metadata |
| Python query/class field reader | `first = second = 0` in a class | `first`: static Field; `second`: non-static Variable; metadata lists only `first` | Both static Fields and both class-field entries |
| Python class field reader | Tuple/pattern assignment, with or without parentheses | Both standalone names are static Fields; neither is in metadata | Both names listed under Static Fields |
| Swift class field reader | `var first = 1, second = 2`; static equivalent | Both standalone names have correct flags; metadata keeps only the first | Both names listed in each appropriate section |
| C/C++ controls | Scalar, single-pointer, and bitfield members; wrapped scalar variable template | Correct records and lists | Same; clean |
| TypeScript controls | Unquoted properties of ordinary classes and interfaces | Correct records and lists | Same; clean |
| C# interface property reader | Instance and static interface properties | Correct records and lists | Same; clean |
| Java class/interface readers | Multiple names in one class field or interface constant declaration | Every name listed with correct flags | Same; clean |
| Rust controls | Struct scalar/array fields; tuple-struct metadata | Correct fields/metadata | Same; clean; tuple fields have no named standalone declarations |
| Python controls | Single annotated/unannotated name, qualified ClassVar, and annotation decoy | Correct records and lists | Same; clean |
| Swift protocol reader | Property requirement | Correct field and metadata entry | Same; clean |
| PHP class/trait readers | Multiple property names in one declaration, ordinary array/static properties | Every name listed with correct flags | Same; clean |
| Go / Scala | Go struct metadata; Scala object/trait/class controls | Existing behavior retained | Standalone fields are deferred by this spec; no new requirement |
| Lua / Perl / Bash / Zsh | Primary fixtures and appended function/local controls | Existing functions, variables, and access behavior retained | No class-field reader applies |
| Adapter and CLI field projection | Incomplete lists above, compared with standalone records and class output | Preserve omissions; do not restore missing fields | Preserve a complete extracted list |

Representative reproductions to append to the corresponding primary fixtures:

```typescript
abstract class Fields { static count = 0; }
class PrivateFields { static #count = 0; }
interface Named { "label": string; }
```

```rust
union Fields { number: i32, bytes: [u8; 4] }
```

```cpp
struct Fields {
    int values[3];
    int **pointer;
    int (*callback)(int);
    template<class T> static T cache[3];
};
```

```csharp
class Fields { public static int Count { get; private set; } }
record RecordFields { public int value; }
```

```python
class Fields:
    first = second = 0
    a, b = (1, 2)
```

```swift
struct Fields { static var first = 1, second = 2 }
```

Complete the existing readers and query patterns for the enumerated forms. Extend a shared L1 matrix to compare expected source-declared names with both standalone records and type metadata, including flags and access; checking only entries that already exist misses this defect. Keep representative CLI assertions for Static Fields and Instance Fields. Update query provenance when captures change, and correct the documentation's claim that symbol and class field lists agree. The explicitly documented TypeAlias metadata loss and deferred Go/Scala fields remain outside this finding. Java enum bodies were also checked: their standalone fields exist, but enum metadata does not carry those fields and `hug classes` excludes Enum owners; this finding does not request changing that class-owner selection policy.

### High — The unchanged extractor revision lets the cache retain repaired defects

**Defect class:** behavior-changing extraction repairs reuse the earlier extractor identity, allowing valid cached records from the broken implementation to bypass repaired readers.

The tree-hugger library's [EXTRACTOR_REVISION](../../lib/src/file/mod.rs:9) is still `"1"`, as it was before this repair cycle. The implementation log explicitly records this choice. Section 8 requires a bump whenever extracted records change; it does not exempt changes on an unreleased branch. The current documentation and code comments promise the same rule.

The tree-hugger-cli [analyze_tree_file](../../cli/src/main.rs:1634) builds a persistent key from that revision. A valid cache hit returns before the new extraction code runs. The historical cache test proves the schema-2/literal-key upgrade, but does not prove invalidation of schema-3, revision-1 entries from before these repairs.

Reproduction: copy the Python fixture, append a class method decorated with `@notstaticmethod`, and run `hug --json symbols python.py` in an isolated cache. In its written snapshot, set that method's `modifiers` to `[{"type":"Static"}]`, reproducing its pre-repair value without changing the source or key. Run the same command again, then with `--no-cache`.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Persistent cache consumer | Schema-3/revision-1 entry with the pre-repair Python static value | Cached run reports static; uncached run reports instance | An upgrade to the repaired extractor recomputes the instance flag |
| Extractor provenance writer | Records before and after these repairs | Both stamp `0.1.0+rev.1` | Different revisions identify different extraction rules |
| All 16 language key consumers | Shared revision component and persistent-hit branch inspected | Every language reuses the unchanged revision | Repairs to any language change the shared revision |
| Historical schema-2 key control | Existing isolated cache-upgrade regression | Old key ignored | Same; clean |
| Current warm-hit control | Valid snapshot under the current key, including a renamed-record sentinel | Entry served | Same; clean: reuse should still work within one revision |
| In-process and legacy in-memory consumers | Their shared key construction and lifetime inspected | Same revision component; caches are process-local | No additional surviving-upgrade instance; shared revision bump is sufficient |

Bump the revision for these repairs and subsequent behavior repairs. Extend the cache-upgrade regression with a hard-coded schema-3/revision-1 key carrying old semantics, prove rejection through corrected public output, and retain the warm-hit control. Preserve the existing schema-2 test's historical formula.

### Medium — Persisted symbol input lacks a public-result robustness matrix test

**Defect class:** deserialization coverage checks successful round trips and generic corruption, but does not systematically assert the supported and rejected shapes of the load-bearing symbol fields through the consumer's public result.

The tree-hugger library's [cache corruption test](../../lib/tests/l1/cache_tests.rs:335) checks the cache reader's return value, and [symbols_adapter](../../lib/tests/l1/symbols_adapter.rs:277) checks valid serialization. The tree-hugger-cli [cache_upgrade test](../../cli/tests/symbols/cache_upgrade.rs:125) exercises upgrades and warm hits. None walks the required field/shape matrix through CLI output. Review 1 explicitly noted that its corruption matrix existed only as review evidence; this cycle did not add a permanent test for it.

The full sweep below used a snapshot written by the current CLI from a copied Python fixture, with a method and an annotated class field. Each cell restored the original snapshot and changed one field. Positive controls populated the optional values; a renamed record proved whether the entry was served. Malformed entries were recomputed. No additional parser defect was found in these cells.

The reader is the shared JSON [PersistentCache::get](../../lib/src/cache/mod.rs:434); its dispatcher serves all 16 languages. There is no new manifest, lockfile, YAML, or TOML reader in this fix. The two tables are halves of the same JSON matrix. **S** means served; **R** means rejected and recomputed. For scalar fields, the element-shape rows supply an array, which is an invalid whole-field shape. Empty arrays apply to collection fields, and empty strings to string/enum fields; an empty string is invalid for the boolean field.

| Shape | `kind` | `identity.container_kind` | `visibility.level` | `modifiers` | `source.declaration_text` |
|---|---|---|---|---|---|
| Unedited positive control | S | S | S | S | S |
| Absent | R | S: unknown | R | S: empty list | S: absent text |
| Explicit null | R | S: unknown | R | R | S: absent text |
| Wrong whole-field type (`123`) | R | R | R | R | R |
| One wrong element | R | R | R | R | R |
| Every element wrong | R | R | R | R | R |
| Empty | R | R | R | S: `[]` | S: `""` |
| Duplicate key | R | R | R | R | R |
| Trailing invalid content | R | R | R | R | R |

| Shape | `relations.implements` | `kind_data.kind_data_type` | Type data `fields` | Field data `visibility` | Field data `is_static` |
|---|---|---|---|---|---|
| Unedited positive control | S | S | S | S | S |
| Absent | S: empty list | R | S: empty list | S: unknown | R |
| Explicit null | R | R | R | S: unknown | R |
| Wrong whole-field type (`123`) | R | R | R | R | R |
| One wrong element | R | R | R | R | R |
| Every element wrong | R | R | R | R | R |
| Empty | S: `[]` | R | S: `[]` | R | R |
| Duplicate key | R | R | R | R | R |
| Trailing invalid content | R | R | R | R | R |

Optional unknown values and default empty collections are explicitly supported by this schema. Optional declaration text also accepts an empty string at the serialization boundary, although extraction promises nonempty text; this review does not propose a new semantic cache validator. The matrix should pin the existing reader contract, not redefine these permitted defaults.

Add one table-driven JSON test to the existing CLI `symbols` binary, based on a tool-written snapshot. Assert the field's resulting public value for each accepted shape, and source-derived output for rejected shapes, with a positive control for every column. Share setup with the upgrade test rather than adding a separate harness. All reader sites, columns, and shapes requiring that coverage are enumerated above.

## Recurrence

The linkage finding repeats **review-1.md, “Earlier static declarations do not reliably reach later C and C++ functions.”** That repair should have swept both halves of C++ identity: every nested parameter declarator handled by `parameter_types`/`parameter_name`, and every equivalent enclosing-namespace form handled by `namespace_path`. It repaired top-level names/defaults and declarator enumeration, but left nested parameter names, parameter comments, and nested-versus-qualified namespace syntax. This review's table includes all failing instances found in that sweep, with the C reader and the repaired overload/namespace controls checked alongside them.

The conditional-wrapper finding repeats **review-1.md, “C++ template wrappers break member kinds and static flags.”** The repair expanded the shared wrapper rule and fixed C/C++ field and access readers, but did not sweep the C# access, class-field, and interface-property readers that encounter the same preprocessor node kinds. The C# class/struct field helper, record owner context, interface-property helper, and all C# default-access contexts need that same rule. The table above carries the failing branch/default categories and the clean classification, static, C/C++, and Swift controls; the broader field table separately enumerates owner/declarator omissions that occur without conditional wrappers.

## Blocked Findings

None.

## Verification levels

| User-facing requirement | Strongest verification present | Assessment |
|---|---|---|
| Kinds, visibility, containers, declaration text, Rust trait relations, and both static meanings | L1 public-library fixture matrices and boundary regressions | Appropriate level; add the remaining linkage and field rows |
| Agreement across adapters and schema-3 serialization | L1 conversions and serialization checks | Appropriate level; completeness must also be checked against source-declared fields |
| Visibility labels, constructor/alias inclusion, and static/instance class grouping | L1 isolated shipped-CLI subprocess tests | Appropriate for this textual/data contract; add the missing field projections |
| Pretty/plain label agreement | L1 forced-style output with styling stripped | Proves labels; no claim of emulator rendering verification |
| Cache upgrade and reuse | L1 isolated persisted-snapshot mutation | Appropriate level; missing prior revision-1 upgrade and permanent robustness matrix |

The specification introduces no keyboard, mouse, paste, scrolling, or terminal-emulator-specific rendering contract. Its verification section explicitly assigns these behaviors to L1. No L2 or L3 mismatch blocks readiness here. Cross-OS proof is left to CI and does not affect this review's readiness decision.
