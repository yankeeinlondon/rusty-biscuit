---
$schema: feature-review.yaml
ready: false
findings:
  - title: Function linkage matching crosses namespace boundaries and selects return declarators
    priority: high
  - title: C++ const linkage reads qualifiers from the wrong declarator level
    priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-03T22:48:23-07:00"
spec: 2026-10-03-visibility-and-kind/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/fixes/2026-10-03-visibility-and-kind/log.md
description: "A **fix** review of `2026-10-03-visibility-and-kind/spec.md`"
fix: 2026-10-03-visibility-and-kind/review-3.md
previous: 2026-10-03-visibility-and-kind/review-2.md
next: 2026-10-03-visibility-and-kind/review-4.md
---

The fix is **not production ready**. All examples explicitly listed in review 2 are repaired, but the same-file function-linkage repair still has missed declarator forms and now also assigns private visibility across unrelated namespace boundaries. A separate C++ qualifier defect reverses visibility for nested pointers and parenthesized declarations. No finding requires a human decision.

## Previous review and blocked findings

The first check was the five findings in review 2's `Unblocked Findings` section. Its `Blocked Findings` section says “None”; no blocked finding needed to be unblocked before this implementation.

| Review 2 finding | Evidence in the current implementation | Result |
|---|---|---|
| C++ linkage matching still includes parameter names and namespace spelling | The public-library matrix covers nested callback names, comments, nested versus qualified namespace blocks, and overload isolation; the representative CLI test passes | Listed examples repaired; the full identity sweep finds the instances below |
| Conditional wrappers still hide C# access levels and field lists | The shared member-slot and flattened-member readers cover class, struct, record, interface, namespace, and conditional branches; the public wrapper tests pass | Listed examples repaired |
| Field enumeration drops supported members from symbols and class summaries | Complete expected field lists are compared with standalone symbols, metadata, adapter round trips, and pipeline records; CLI list and implicit-static-constant tests pass | Listed examples repaired |
| The unchanged extractor revision lets the cache retain repaired defects | `EXTRACTOR_REVISION` is now `"2"`; hard-coded schema-2 and schema-3/revision-1 entries are rejected, and current-key warm-hit controls pass | Repaired |
| Persisted symbol input lacks a public-result robustness matrix test | One tool-written JSON snapshot drives ten field columns and nine shapes through CLI output, with served-value and recomputed-output assertions | Repaired |

Review 1's local-function, template-wrapper, decorator-decoy, and nearest-owner regressions also pass. The field sweep covers JavaScript, TypeScript, Rust, C, C++, C#, Java, Python, Swift, PHP, and the intentionally metadata-only Go/Scala forms. Lua, Perl, Bash, and Zsh have no applicable class-field reader. No additional field-list defect was established.

## Verification

I used the tree-hugger and rust-testing skills and reviewed the specification, both earlier reviews, implementation log, extraction readers, adapter, CLI projections, current documentation, and test wiring.

- `just test` in `tree-hugger/`: **708 passed, 0 skipped**.
- `just lint` in `tree-hugger/`: passed for both packages.
- `just check-tier-coverage tree-hugger`: **0 stranded tests**.
- `cargo build --color=never -p tree-hugger-cli`: passed.

The library's consolidated L1 target declares the new modules. Cargo discovers the CLI `symbols` binary, whose `main.rs` declares the cache matrix. These tests have ordinary L1 names and are selected by the live recipe.

Additional reproductions copied the primary C/C++ fixtures into scratch Git repositories, appended declarations, and ran the current shipped binary with `--json symbols --no-cache`. Control runs used the unedited fixtures; final probes isolated home, configuration, and temporary directories. Tables below show the later declaration's visibility when a prototype precedes it. The probes are review evidence, not permanent regression tests.

## Unblocked Findings

### High — Function linkage matching crosses namespace boundaries and selects return declarators

**Defect class:** same-file function identity uses candidate ancestor namespaces and the outermost function declarator instead of the function's actual namespace and named declarator, causing both false matches and missed matches.

In the tree-hugger library, [FunctionIdentity::of](../../lib/src/file/extract/visibility/c.rs:223), which identifies functions for earlier-static lookup, generates every enclosing namespace prefix even for an **unqualified** declaration. Consequently, a declaration of `child::f` can match an earlier `static f` in its parent namespace. For a qualified definition, it likewise accepts any matching candidate without checking whether a nearer namespace already determines the name. This can report an externally visible function as ModulePrivate.

The same library's [function_declarators](../../lib/src/file/extract/visibility/c.rs:190), used by both C and C++, stops at the first function declarator in a chain. In a function returning a function pointer, that is the **returned function type**, whose declarator includes the actual function and its parameters. Renaming the actual function's parameter then changes the supposed function name. A return-array declarator has the same problem. The earlier `static` is missed, even in C, where the specification requires matching by name alone.

```cpp
static int scope_leak(int);
namespace unrelated {
    int scope_leak(int x) { return x; } // Expected Public; reports ModulePrivate
}

static int (*pointer_result(int))(int);
int (*pointer_result(int value))(int) { return 0; }
// Expected ModulePrivate; reports Public, in both C and C++
```

The sweep enumerated the shared declaration collector, declarator walker, name reader, namespace construction, parameter normalization, C/C++ visibility dispatchers, and CLI projection. It checked these forms on fixture copies:

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| C++ namespace identity | Global `static f(int)` followed by unqualified `f(int)` inside a named namespace | ModulePrivate | Public |
| C++ namespace identity | Parent-namespace static function followed by same-named function in a nested child block | ModulePrivate | Public |
| C++ namespace identity | Same parent/child separation using `namespace a::b` spelling | ModulePrivate | Public |
| C++ namespace identity | Child function under a preprocessor conditional, template header, or `extern "C"` block | ModulePrivate in all three forms | Public |
| C++ qualified namespace identity | Global `shadow::f` is static; `host::shadow::f` has a public prototype; definition is `shadow::f` inside `host` | Public prototype, ModulePrivate definition | Both host declarations Public |
| Shared C/C++ declarator identity | Function returns a function pointer; actual parameter unnamed in prototype and named in definition | Public definition in both languages | ModulePrivate |
| Shared C/C++ declarator identity | Function returns a pointer to an array; actual parameter gains a name | Public definition in both languages | ModulePrivate |
| Shared C/C++ declarator identity | Returned callback type contains another callback; parameters gain names | Public definition in both languages | ModulePrivate |
| C++ declarator identity | Function returns a function reference; actual parameter gains a name | Public definition | ModulePrivate |
| C++ declarator/namespace identity | Function-pointer return inside the same named namespace | Public definition | ModulePrivate |
| Shared C/C++ declaration collector | Function-pointer return is the second declarator of a static prototype | Public definition in both languages | ModulePrivate |
| Shared C/C++ declaration collector | Function-pointer-return prototype is under `#if` | Public definition in both languages | ModulePrivate |
| C++ linkage wrapper | Function-pointer-return prototype and definition in `extern "C"` | Public definition | ModulePrivate |
| Shared C/C++ control | Function-pointer return with identical actual parameter spelling in both declarations | ModulePrivate | Same; clean |
| Shared C/C++ control | Ordinary pointer return, actual parameter renamed | ModulePrivate | Same; clean |
| Shared C/C++ control | Explicitly static function-pointer-return definition | ModulePrivate, storage-static true | Same; clean |
| C++ namespace control | Same namespace, sibling namespaces, genuinely different overload | Same namespace ModulePrivate; sibling/different overload Public | Same; clean |
| C++ qualified-name control | Single namespace-qualified definition at file scope | ModulePrivate | Same; clean |
| C++ parameter/namespace normalization controls | Nested callback parameters, parameter comments, defaults, namespace spelling, pointer/array decay, qualifiers, variadic distinction | Existing public-result matrix passes | Same; clean |
| Other 14 languages and adapter/CLI paths | All-language visibility and conversion matrices; shipped CLI output | Existing matrices pass; CLI preserves the wrong C/C++ values above | No other language uses this linkage scan; projections must preserve corrected values |

Identify the function declarator that directly declares the captured name, following declarator chains through returned function and array types. An unqualified declaration must have exactly its enclosing namespace. Resolve qualified namespace candidates against the same-file namespace declarations; do not attach an unrelated static declaration merely because one candidate matches. When no earlier declaration can be established as the same function, retain the declaration's own linkage rule, as the specification requires. Preserve the repaired parameter normalization and overload controls.

Extend the public-library linkage matrix with every failing row above and representative CLI cases for both false-private and false-public output. Bump the extractor revision again. The current [linkage documentation](../../docs/symbol-kinds-and-visibility.md:371) says the scan can only miss a static declaration and never invent one; that claim has drifted from the code. Restore that invariant in the code and review the corresponding comments and documentation alongside the repair.

### High — C++ const linkage reads qualifiers from the wrong declarator level

**Defect class:** namespace-scope object linkage inspects the first declarator wrapper rather than the object's own type, confusing a const pointee with a const pointer and treating parenthesized references as const objects.

In the tree-hugger library, [is_const_object](../../lib/src/file/extract/visibility/c.rs:104), which decides whether an unmarked C++ variable has private linkage, checks only the outer syntax node. In `char *const *p`, that node describes the inner const pointer, although `p` itself is mutable. In `char **const p`, it sees the unqualified inner pointer and misses the object's const qualifier. Parenthesized pointer and reference declarators fall back to the declaration's base-type qualifiers. The early `constexpr` return also treats a constexpr reference as a const object, despite the helper's documented invariant that a reference is never a const object.

```cpp
char *const *mutable_pointer_to_const_pointer;
// Expected Public; reports ModulePrivate
char **const const_double_pointer = 0;
// Expected ModulePrivate; reports Public
int (*const const_function_pointer)(int) = 0;
// Expected ModulePrivate; reports Public
```

This is separate from function-redeclaration identity: it affects variables without any earlier declaration. The complete decision-site sweep covers the C++ object helper, its shared linkage caller, scope and local guards, member access, C's non-const linkage rule, adapter, and CLI. The table lists clean sibling forms as well as failures.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| C++ pointer-object reader | `char *const *p` and `const int *const *p` | ModulePrivate | Public: the outer pointer is mutable |
| C++ pointer-object reader | `char **const p` and `const int **const p` | Public | ModulePrivate: the outer pointer is const |
| C++ array-object reader | Array of const double pointers: `char **const p[2]` | Public | ModulePrivate |
| C++ array-object reader | Array of mutable pointers to const pointers: `char *const *p[2]` | ModulePrivate | Public |
| C++ parenthesized-object reader | Const function pointer: `int (*const p)(int)` | Public | ModulePrivate |
| C++ parenthesized-object reader | Mutable pointer to const array: `const int (*p)[2]` | ModulePrivate | Public |
| C++ parenthesized-object reader | Const pointer to mutable array: `int (*const p)[2]` | Public | ModulePrivate |
| C++ reference-object reader | `const int (&r)[2]` | ModulePrivate | Public: a reference is not a const object |
| C++ constexpr-object reader | `constexpr const int &r = array[0]` | ModulePrivate | Public: constexpr does not turn a reference into a const object |
| C++ per-name decision | `int a, **const b = 0, *const *c = 0;` | Public, Public, ModulePrivate | Public, ModulePrivate, Public |
| C++ scope/wrapper readers | Const/mutable nested pointers inside a named namespace, `#if`, or `extern "C"` block | Const outer pointer Public; mutable outer pointer ModulePrivate | Const outer pointer ModulePrivate; mutable outer pointer Public |
| C++ pointer controls | Scalar const, ordinary mutable/const single pointer, mutable double pointer | Correct Public/ModulePrivate values | Same; clean |
| C++ array controls | Const scalar array and multidimensional array; mutable array; array of mutable pointers to const data; array of const single pointers | Correct Public/ModulePrivate values | Same; clean |
| C++ parenthesized/reference controls | Mutable function pointer; ordinary reference to const scalar | Public | Same; clean |
| C++ override controls | `extern` and `inline` const nested pointers or const arrays; constexpr pointer | Extern/inline Public; constexpr pointer ModulePrivate | Same; clean |
| C++ unnamed-namespace control | Mutable nested pointer in unnamed namespace | ModulePrivate | Same; clean: namespace rule applies independently |
| C++ local/member controls | Const/mutable nested pointers in function body and struct body | Locals Unknown; struct fields Public | Same; clean: object linkage does not set local/member access |
| C shared linkage reader | File-scope const scalar, mutable/const nested pointers, const function pointer | Public | Same; clean: C has no implicit const-private rule |
| Other 14 language readers and adapter/CLI paths | Visibility/conversion matrices and shipped CLI probes | Existing matrices pass; CLI preserves the wrong variable values above | No other language uses this C++ object helper; projections must preserve corrected values |

Determine the object's outermost type by walking the declarator toward the captured name, retaining array-element constness and the distinction between pointers, references, and function pointers. Check reference identity before the constexpr shortcut. Apply the decision separately to every name in a multi-declarator declaration, while keeping explicit static, extern/inline, unnamed namespace, member, and local rules intact.

Add one table-driven public-result regression covering every failing and clean declarator form above, including scope wrappers and multiple names. Retain representative shipped-CLI assertions, bump the extractor revision, and update the relevant function comments and current linkage documentation with the corrected nested-declarator behavior.

## Recurrence

The function-identity finding repeats the class of **review-1.md, “Earlier static declarations do not reliably reach later C and C++ functions,”** and **review-2.md, “C++ linkage matching still includes parameter names and namespace spelling.”** The earlier sweeps should have included every function-bearing declarator chain on both sides of the lookup, not only callback types inside parameters and simple pointer returns. They missed function-pointer, function-reference, and array-pointer return types, including multi-declarator and wrapped prototypes. The namespace repair also needed parent-versus-child isolation and nearer qualified-namespace shadowing controls; sibling-namespace controls alone do not catch the new false matches. This review carries all failing instances established in that sweep and the clean sibling sites.

The const-object finding is a new class relative to both earlier reviews: neither reported object-level qualifier selection. Both findings concern the specification's linkage contract, but they have different decision sites and repairs.

## Blocked Findings

None.

## Input robustness matrix

The persistent JSON symbol snapshot is the only serialized input reader changed or exercised by this fix. The new `cache_robustness` test permanently checks all ten load-bearing columns identified in review 2: kind, container kind, visibility level, modifiers, declaration text, implements relations, kind-data discriminator, type fields, field visibility, and field static flag. Every column has a positive control and absent, null, wrong whole type, one wrong element, all wrong elements, empty, duplicate-key, and invalid-content cells. Accepted cells assert the public value and a served-entry sentinel; rejected cells assert source-derived public output. All 90 cells passed. Optional/default values are the documented schema contract. No additional manifest, lockfile, or configuration parser was introduced by this fix.

## Verification levels

| User-facing requirement | Strongest verification present | Assessment |
|---|---|---|
| Symbol kinds, containers, access, declaration text, trait relations, and static flags across languages | L1 public-library matrices and boundary regressions | Appropriate level; missing linkage cases are enumerated in the findings |
| Model conversions and schema-3 serialization | L1 exhaustive conversion and round-trip checks | Appropriate level; passing |
| Complete field lists and static/instance grouping | L1 source-declared expected lists through library, adapter, pipeline, and CLI | Appropriate level; listed prior defects repaired |
| Visibility labels and constructor/alias command inclusion | L1 shipped-CLI subprocess assertions | Appropriate level; new linkage results need regression assertions |
| Pretty/plain label agreement | L1 forced-style output with styling removed | Verifies labels, without claiming real-emulator rendering proof |
| Cache upgrades, current-key reuse, and malformed snapshots | L1 isolated CLI cache mutation and full field/shape matrix | Appropriate level; passing |

This specification defines textual and structured output, without keyboard, mouse, paste, scrolling, or terminal-emulator-specific behavior. L2/L3 evidence is not required for these contracts. Cross-OS evidence does not affect this review's readiness decision.
