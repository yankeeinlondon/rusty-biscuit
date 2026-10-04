---
$schema: feature-review.yaml
ready: false
findings:
  - title: C++ template identity ignores trailing return types and constraints
    priority: high
  - title: C and C++ signatures drop parameters or substitute returned-function parameters
    priority: high
  - title: C and C++ declarator wrappers still hide symbols and members
    priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-03T23:09:45-07:00"
spec: 2026-10-03-visibility-and-kind/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/fixes/2026-10-03-visibility-and-kind/log.md
description: "A **fix** review of `2026-10-03-visibility-and-kind/spec.md`"
fix: 2026-10-03-visibility-and-kind/review-4.md
previous: 2026-10-03-visibility-and-kind/review-3.md
next: 2026-10-03-visibility-and-kind/review-5.md
---

The fix is **not production ready**. The examples listed in review 3 are repaired, but related valid declarations still receive incorrect visibility, lose signature parameters, or disappear from symbol and class output. All three findings can be repaired without a human decision.

## Previous review and blocked findings

I first checked both findings in review 3's `Unblocked Findings` section. Its `Blocked Findings` section contains none, so no blocked finding needed to be unblocked before the latest implementation.

| Previous finding | Current evidence | Result |
|---|---|---|
| Function linkage matching crosses namespace boundaries and selects return declarators | The public-library linkage matrix now covers namespace separation and shadowing, returned function pointers/references/array pointers, multiple declarators, conditional wrappers, linkage blocks, and controls; representative CLI assertions pass | Every listed instance repaired; template identity has the remaining cases below |
| C++ const linkage reads qualifiers from the wrong declarator level | The public-library const-object matrix covers nested pointers, arrays, parenthesized declarators, references before the constexpr rule, multiple names, wrappers, scope guards, and C controls; representative CLI assertions pass | Repaired; no further object-constness defect established |

Review 1's local-function, member-wrapper, decorator-decoy, and nearest-owner regressions pass. Review 2's conditional access, field-list, implicit-static-constant, cache-upgrade, and persisted-input regressions also pass. `EXTRACTOR_REVISION` is now `"3"`; schema-2 and schema-3/revision-1/revision-2 upgrade tests pass, alongside current-key reuse controls.

## Verification

I used the tree-hugger and rust-testing skills, read the specification and all three earlier reviews, and inspected the implementation log, declaration readers, query captures, signature readers, model conversions, CLI projections, documentation, and test wiring.

- `just test` in `tree-hugger/`: **714 passed, 0 skipped**.
- `just lint` in `tree-hugger/`: passed for both packages.
- `just check-tier-coverage tree-hugger`: **0 stranded tests**.
- `cargo build --color=never -p tree-hugger-cli`: passed.

The consolidated library L1 target declares the regression modules. The CLI's discovered `symbols` test binary declares its wiring, cache-upgrade, and cache-robustness modules. Ordinary test names keep these checks in the running L1 recipe.

Additional probes copied the primary language fixtures into a scratch Git repository, checked the unedited fixture as a positive control, appended the declarations below, and ran the newly built `hug --json symbols --no-cache`. Home, configuration, and temporary directories were isolated. Class omissions were also checked with `hug --plain classes`; template visibility was checked with `hug --plain symbols`. Clang accepted the complete template-identity probe with `-std=c++20 -fsyntax-only`, and the attributed-declaration probes with C++20 and C23 syntax checks. These review probes do not replace permanent regression tests.

## Unblocked Findings

### High — C++ template identity ignores trailing return types and constraints

**Defect class:** function identity discards syntax that distinguishes function templates, allowing an earlier static overload to assign private visibility to a different function.

In the tree-hugger library, [FunctionIdentity::of](../../lib/src/file/extract/visibility/c.rs:279), which identifies earlier declarations of a function, compares template parameter tokens and the result of [return_type](../../lib/src/file/extract/visibility/c.rs:340). That return reader stops at the named function declarator, excluding its trailing return type. The identity also omits constraints both after `template<…>` and after the function's parameter list. As a result, the implementation's documented “can only miss, never invent” linkage invariant still fails.

```cpp
template<class T> static auto probe(T) -> int;
template<class T> auto probe(T value) -> long { return 0; }
// The second declaration is a different template: Public, not ModulePrivate.

template<class T> concept Small = sizeof(T) < 4;
template<class T> concept Big = sizeof(T) >= 4;
template<class T> requires Small<T> static int constrained(T);
template<class T> requires Big<T> int constrained(T value) { return 0; }
// Different constraints also distinguish these templates.
```

The sweep covers both identities compared by the shared scan, template parameter lists, leading and trailing return syntax, both constraint positions, namespace/wrapper collection, C's name-only rule, and output projection. The table reports the later declaration.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| C++ template return reader | `auto … -> int` static prototype followed by `auto … -> long` definition | ModulePrivate | Public |
| C++ template header reader | `requires Small<T>` prototype versus `requires Big<T>` definition after the template header | ModulePrivate | Public |
| C++ function declarator reader | Trailing `requires Small<T>` prototype versus trailing `requires Big<T>` definition | ModulePrivate | Public |
| C++ constraint presence | Constrained prototype followed by unconstrained definition | ModulePrivate | Public |
| C++ constraint presence, reverse | Unconstrained prototype followed by constrained definition | ModulePrivate | Public |
| C++ namespace collection | Different trailing return types inside two blocks reopening the same named namespace | ModulePrivate | Public |
| C++ conditional collection | Different trailing return types with the static prototype under `#if` | ModulePrivate | Public |
| C++ template parameter control | `template<Small T>` versus `template<Big T>` | Public | Same; clean |
| C++ ordinary return control | Template returning `int` versus template returning `long` before the name | Public | Same; clean |
| C++ identical trailing-return control | Both declarations use `auto … -> int` | ModulePrivate | Same; clean |
| C++ identical constraint control | Both declarations have the same trailing `requires Small<T>` | ModulePrivate | Same; clean |
| Shared C/C++ scan and other C++ controls | Existing name, overload, namespace, template/non-template, parameter normalization, return-declarator, and wrapper matrices | Pass | Same; clean |
| Other 14 languages | Their visibility matrices | Pass | No other language invokes this function-identity scan |
| CLI projection | JSON and plain output for the failing template forms | Preserve erroneous ModulePrivate | Preserve corrected Public |

Include the actual trailing return type and both associated constraint positions in template identity. Conservative token comparison is sufficient; unresolved equivalence should miss a static declaration rather than invent one. Extend the existing public-result linkage matrix with every failing and clean row, retain a representative CLI assertion, bump the extractor revision, and correct the corresponding identity comments and current linkage documentation.

### High — C and C++ signatures drop parameters or substitute returned-function parameters

**Defect class:** signature readers assume shallow declaration and parameter shapes, so declarator wrappers either erase real parameters or expose the parameters of a returned function instead.

In the tree-hugger library, [TreeFile's signature extraction](../../lib/src/file/tree_file.rs:1861) passes the widened declaration to [extract_parameters](../../lib/src/file/extract/signatures.rs:26). The latter looks only for an immediate function declarator. A pointer-return declaration has that function deeper in its declarator chain. A returned-function declaration can instead expose the returned function's parameter list. Separately, [extract_c_parameter](../../lib/src/file/extract/signatures.rs:736) accepts too few parameter node kinds, while [find_c_param_name](../../lib/src/file/extract/signatures.rs:768) searches only direct names and one pointer/reference layer.

This is extracted data loss, not merely formatting. The CLI's structured signature is wrong before rendering. The implementation log records only the function-pointer-return example as deferred; the sweep establishes ordinary pointer-return functions and methods are affected too. Section 5 requires function signatures, and the verification contract calls for signature-content regression coverage. An empty signature does not preserve a function that has parameters.

```c
int *pointer_probe(int value) { return 0; }
// signature.parameters is []; expected value.
int (*callback_probe(int actual))(long returned) { return 0; }
// signature.parameters contains returned; expected actual.
int array_parameter_probe(int values[2]) { return 0; }
// signature.parameters is []; expected values.
```

| Site | Shape tested | Observed parameter names | Expected parameter names |
|---|---|---|---|
| C/C++ declaration-to-signature reader | Free function returning one or two pointer levels | None in both languages | Its declared parameter |
| C/C++ declaration-to-signature reader | Function returning a function pointer | Returned function's named parameter, or none when unnamed | Actual function's parameter |
| C/C++ declaration-to-signature reader | Chained returned callbacks | Deepest returned function's parameter (`third`) | Actual function's parameter (`first`) |
| C/C++ declaration-to-signature reader | Function returning a pointer to an array | None | Actual function's parameter |
| C++ declaration-to-signature reader | Function returning a function reference | None | Actual function's parameter |
| C/C++ declaration/wrapper sites | Pointer-return prototype, explicit-static definition, and definition under `#if` | None | Its declared parameter |
| C++ declaration/wrapper sites | Free pointer-return template and free pointer-return function in `extern "C"` | None | Its declared parameter |
| C++ member reader | In-class pointer-return or reference-return definition, including a static method template | None | Its declared parameter |
| C++ qualified definition reader | Out-of-class pointer-return definition | None | Its declared parameter; kind precision for out-of-class definitions remains outside this finding |
| C/C++ parameter-name reader | Double pointer, array, function pointer, parenthesized name, and attributed name | Each omitted in both languages | Each declared parameter retained |
| C++ parameter-node reader | Default-valued scalar parameter and template parameter pack | Each omitted | `defaulted`; `values` respectively |
| C++ variadic parameter reader | `int first, ...` | Only `first` | `first` and the variadic tail |
| C/C++ controls | Scalar-return function with scalar parameter; single-pointer parameter | Correct names | Same; clean |
| C++ reference controls | Lvalue- and rvalue-reference parameters | Correct names | Same; clean |
| C variadic control | `int first, ...` | `first` and `...` | Same; clean |
| Adapter and CLI signature paths | JSON output and existing round-trip tests | Preserve the already wrong or empty signature | Preserve a correctly extracted signature |
| Other language signature readers | Existing fixture/signature suites; appended annotated/decorated methods in Rust, Java, C#, PHP, Swift, Scala, Python, TypeScript, and JavaScript | Their ordinary named parameters retained | Same; these languages do not use C declarator chains |
| Perl, Bash, Zsh | Existing parameterless signature expectations; attributed Perl sub probe | Signatures present | Same; no new parameter-discovery requirement for these languages |
| Go | Existing receiver/parameter regression suite | Pass | Same; its distinct parameter-list selection remains intact |

Select the named function's own declarator using the same name-aware traversal already needed for linkage, then read only its parameter list. Walk each parameter's own declarator to its name, handling optional parameters, packs, and variadic tails explicitly. Do not descend into callback parameter lists to discover the containing parameter's name. Add source-derived expected parameter lists through the public library, conversions, and representative CLI output; a round trip that preserves an empty input is not a sufficient test. Keep the return-label presentation policy outside this repair, update drifted parameter-reader comments, and invalidate previously cached signatures.

### High — C and C++ declarator wrappers still hide symbols and members

**Defect class:** declaration captures and member-name readers match fixed shallow shapes instead of traversing transparent declarator wrappers, so supported declarations disappear from every downstream view.

The tree-hugger library's [C captures](../../lib/queries/vendor/c/locals.scm) and [C++ captures](../../lib/queries/vendor/cpp/locals.scm) require direct name children for several forms. An attribute attached to the name breaks that shape. The C++ method patterns also cover only a direct function declarator or one wrapper. The library's [c_field_name](../../lib/src/file/extract/type_metadata.rs:1024), which discovers names for field metadata, does not traverse an attributed declarator. [align_field_metadata](../../lib/src/file/tree_file.rs:2003) cannot complete a field absent from both captures and metadata.

```cpp
struct AttributeOwner {
    int scalar [[maybe_unused]];
    int *pointer [[maybe_unused]];
    int values [[maybe_unused]] [2];
    int (*callback [[maybe_unused]])(int);
};
// No fields in standalone symbols, type metadata, or hug classes.

struct MethodOwner {
    int *declared(int value);
    int (*defined(int value))(long) { return 0; }
};
// Both methods disappear.
```

The attributed examples are accepted by Clang in both C23 and C++20. The sweep covers each affected declaration role, C++ method declaration/definition paths, metadata, and the corresponding annotation/decorator paths in other languages.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| C/C++ variable captures | Attributed scalar, pointer, array, function-pointer variable, and function-local scalar | No symbol | Variable, with the applicable access/storage flags |
| C/C++ function captures | Attribute between function name and parameter list: `int f [[deprecated]] (int value);` | No symbol | Function with its parameter |
| C/C++ field captures and metadata reader | Attributed scalar, pointer, array, and function-pointer fields | No standalone fields; empty metadata/class field list | All four named fields, with matching metadata and class output |
| C++ method declaration captures | Pointer-, reference-, double-pointer-, function-pointer-, and array-pointer-return prototypes in a type body | No method symbol | Method owned by that type |
| C++ method definition captures | Double-pointer-, function-pointer-, chained-callback-, and array-pointer-return in-class definitions | No method symbol | Method owned by that type |
| C/C++ attribute-position control | Attribute before a scalar declaration: `[[maybe_unused]] const int x = 1;` | Variable; C Public, C++ ModulePrivate | Same; clean |
| C/C++ enumerator control | Attribute after enumerator name | Enumerator retained with current kind | Same; enum-kind precision is deferred |
| C++ method controls | Scalar-return prototype/definition and one-pointer-return in-class definition | Method retained | Same; clean; pointer definition's signature is finding 2 |
| Rust, Java, C#, PHP, Swift, Python, TypeScript, JavaScript | Appended annotated/decorated members and methods | Members retained; expected metadata lists populated | Same; clean |
| Scala | Annotated method and class-body `val` | Method and Variable retained | Same; standalone Field precision is explicitly deferred |
| Perl | Named sub with `:lvalue` attribute | Function retained | Same; clean |
| Go, Lua, Bash, Zsh | Primary fixture guards | Pass | No equivalent declaration-attribute wrapper applies to this sweep |
| Adapter and CLI class projection | Attributed C/C++ owner with four fields | Empty fields remain empty in JSON and `hug classes` | Preserve complete captured members |

Additional capture gaps found in the same sweep are **outside the committed precision examples**: parenthesized free-function names are absent in C and C++; pointer and attributed typedef aliases are absent in both; an uninitialized function-pointer variable is retained in C but absent in C++. Record those alongside the existing deferred precision limitations rather than silently expanding this repair's alias/function-pointer coverage. The required repair above concerns the supported class-field and in-class-method forms and their declaration wrappers.

Extend capture/name traversal for the required rows, keeping field/function-pointer and method distinctions intact. Compare expected names against standalone records and metadata rather than iterating only reported entries. Add representative CLI coverage, update changed query provenance/inventory, bump the extractor revision, and document any remaining capture limitations accurately. The implementation log's single attributed-variable observation did not cover the attributed fields or method forms listed here.

## Recurrence

The template-identity finding repeats the earlier-static identity class from review 1, **“Earlier static declarations do not reliably reach later C and C++ functions,”** review 2, **“C++ linkage matching still includes parameter names and namespace spelling,”** and review 3, **“Function linkage matching crosses namespace boundaries and selects return declarators.”** The latest repair added template return/header identity, so its sweep needed both return-type positions and every associated constraint position, including constrained/unconstrained controls. Those sibling sites were missed.

The signature finding repeats review 3's **wrong declarator selection** class at another reader: linkage now selects the named function correctly, but signature extraction still selects an immediate or returned function declarator. That repair should have checked declaration-to-signature selection and the parameter-name reader as siblings of the linkage declarator reader. The implementation log acknowledged one instance without sweeping ordinary pointer returns, member returns, and parameter wrappers. This review includes the complete reproduced list from that sweep.

The capture finding repeats review 2's **“Field enumeration drops supported members from symbols and class summaries”** class. The earlier fix swept pointer, array, and parenthesized declarators but missed attributed declarators in both the capture query and metadata name reader. Its neighboring C++ method captures also stop after one return wrapper. Review 1's **“C++ template wrappers break member kinds and static flags”** concerned owner wrappers after capture; those repaired owner wrappers remain clean and are distinct from declarations never captured at all.

## Blocked Findings

None.

## Input robustness matrix

No additional manifest, lockfile, or configuration reader was introduced. The relevant serialized input is the persisted JSON symbol snapshot. The existing `cache_robustness` test uses a CLI-written fixture and covers ten load-bearing columns: kind, container kind, visibility level, modifiers, declaration text, implements relations, kind-data discriminator, type fields, field visibility, and field static flag.

Each column has a positive control and the documented absent, explicit-null, wrong-whole-type, one-wrong-element, all-wrong-elements, empty, duplicate-key, and invalid-content shapes. The 90-cell test passed. Accepted shapes check the public served value and cache sentinel; rejected shapes check source-derived recomputation. Optional/default behavior is recorded in `docs/cache-design.md`. These results do not conceal or explain the extraction defects above.

## Verification levels

| User-facing requirement | Strongest verification present | Assessment |
|---|---|---|
| Per-language kinds, containers, visibility, and both static flags | L1 public-library matrices and CLI subprocesses | Appropriate level; missing declaration/template variants are findings 1 and 3 |
| Function signatures and content surviving conversion | L1 presence, selected content, and round-trip tests | Appropriate level; source-derived C/C++ parameter coverage is missing as finding 2 shows |
| Field lists and static/instance class grouping | L1 expected-name matrices and CLI subprocesses | Appropriate level; attributed members remain missing as finding 3 shows |
| Declaration text and Rust trait relations | L1 public-library matrices | Passing |
| Visibility labels, constructor/alias inclusion, JSON agreement | L1 CLI wiring assertions | Passing for covered inputs; additional failing inputs need permanent assertions |
| Pretty/plain label agreement | L1 forced-style output with styling removed | Appropriate for textual label agreement; no real-emulator claim |
| Schema upgrades, cache reuse, malformed persisted input | L1 isolated CLI snapshot mutation | Passing |

The specification defines textual and structured analysis output, without keyboard, mouse, paste, scrolling, or terminal-emulator-specific behavior. L2/L3 evidence is not required for these contracts. Cross-OS proof and human review are external to this readiness decision.
