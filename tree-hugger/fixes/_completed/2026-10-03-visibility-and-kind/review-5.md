---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-03T23:51:58-07:00"
spec: 2026-10-03-visibility-and-kind/spec.md
implemented: false
description: "A **fix** review of `2026-10-03-visibility-and-kind/spec.md`"
fix: 2026-10-03-visibility-and-kind/review-5.md
previous: 2026-10-03-visibility-and-kind/review-4.md
---

The fix is **production ready within the specification's committed scope**. All three findings in review 4 are repaired, their sibling cases have regression coverage, and this review established no remaining actionable defect. No human decision is required.

## Previous review and blocked findings

I first checked review 4's `Unblocked Findings`. Its `Blocked Findings` section contains none; no blocked finding needed to be unblocked before implementation.

| Review 4 finding | Repair and verification | Result |
|---|---|---|
| C++ template identity ignores trailing return types and constraints | Identity now compares leading and trailing return types and both constraint positions for explicit and abbreviated function templates. The public-library matrix covers different, equal, added, removed, and moved constraints; reopened namespaces, conditionals, and linkage blocks; and ordinary-function and exception-specification controls. Representative CLI visibility assertions pass. | Repaired |
| C and C++ signatures drop parameters or substitute returned-function parameters | Signature extraction selects the named function's own declarator. Shared name traversal handles parameter wrappers; optional parameters, packs, and variadic tails retain their details. Exact expected parameter lists pass through the library and converted records, with CLI checks. | Repaired |
| C and C++ declarator wrappers still hide symbols and members | Queries capture names beneath attributes and parentheses and member functions beneath return declarators. Classification and field-name extraction share declarator traversal. Tests compare expected symbols and complete field lists, including private/static/conditional/template members, and CLI class output. | Repaired |

The latest repair also handles parenthesized free-function names and wrapped typedef names that review 4 recorded as adjacent gaps. Its plain uninitialized function-pointer-variable controls pass in both C and C++.

## Verification

I used the tree-hugger and rust-testing skills and inspected the specification, reviews 1–4, implementation logs, declaration and member readers, query captures, signature extraction, conversions, CLI projections, current documentation, and test registration.

| Command | Result |
|---|---|
| `just test` in `tree-hugger/` | **724 passed, 0 skipped** |
| `just lint` in `tree-hugger/` | Passed for both packages |
| `just check-tier-coverage tree-hugger` at the repository root | **0 stranded tests** |
| `cargo build --color=never -p tree-hugger-cli` | Passed |

The library's explicit L1 target declares the new `symbols_signatures` and `symbols_declarator_wrappers` modules. Cargo discovers the CLI `symbols` test binary, whose root declares wiring, cache-upgrade, and cache-robustness modules. Their ordinary test names are selected by the live L1 recipe. Fixture reads use `include_str!` or the existing manifest-directory helper. No newly added test is hidden behind an unselected feature or test tier.

Additional CLI probes copied all **21** symbol fixture files into a scratch Git repository and checked that each unedited file produced symbols. I then appended C/C++ declaration variants and ran the freshly built `hug --json symbols --no-cache`. Home, configuration, and temporary directories were isolated. These probes supplement the permanent regression tests.

## Class sweeps

The sweep checked the declaration readers that make each decision, their library/record conversions, and downstream CLI output. All rows below are clean; observed results match expected results.

| Decision and sibling sites | Shapes checked | Observed and expected result |
|---|---|---|
| C++ template identity, on both current and earlier declarations | Header parameters; leading/trailing returns; header/trailing constraints; absent/present/moved constraints; abbreviated templates | Distinct templates remain Public; matching earlier-static templates become ModulePrivate |
| Shared C/C++ earlier-declaration collector and namespace identity | Multiple declarators, conditional/template/linkage wrappers, namespace reopening and shadowing, nested versus qualified spelling | Only an earlier declaration of the same function supplies internal linkage |
| C/C++ function selection for classification, linkage, and signatures | Pointer, reference, array-pointer, callback and chained-callback returns; attributed/parenthesized names; prototypes and definitions; class members | Correct function selected; its own parameters retained; callback fields remain Fields |
| C/C++ parameter-name and parameter-node readers | Multiple pointer levels, arrays, parentheses, attributes, callbacks with named inner parameters, default-valued callback/array parameters, packs, variadic tails | Outer parameter names retained; callback-inner names do not replace them; default and variadic details retained |
| C/C++ captures and field-name reader | Attributed scalar/pointer/array/callback members; return-wrapped declared/defined methods; parenthesized names; wrapped typedef aliases | Expected symbols present with correct kinds and owners; complete field lists |
| C/C++ access and both static readers | Attributed earlier-static functions; private/static/conditional/template members; function-local storage-static controls | Declaration access preserved; member static and storage static remain distinct |
| Adapter and CLI projections | Library matrices, converted records, JSON signatures, plain symbol labels and class partitions | Preserve the repaired facts |
| Other language readers | All 16-language kind/access/static fixtures, signature regressions, member-wrapper/local-function tests, and complete field-list matrices | No regression in the independent language readers |

Independent appended probes combined wrappers beyond the individual regression rows: an attributed name inside parentheses after an earlier static prototype; a chained returned callback; an attributed callback-returning member; a static member template returning two pointer levels; default-valued callback and array parameters; and pointer-valued trailing template returns. Each produced the expected public result.

Earlier defect classes were checked against every prior review: local C# functions remain Functions; transparent member wrappers retain ownership and access; Python decorator decoys and Scala nested owners do not invent static members; field lists remain complete for the supported forms; C++ object constness follows the declarator nearest the name; and earlier-static lookup preserves namespace and overload isolation.

The extractor revision is now `"4"`. Hard-coded historical schema-2 and schema-3/revision-1, revision-2, and revision-3 cache entries are rejected; current-key mutation controls prove persistent reuse. The revision therefore invalidates records affected by all three latest repairs.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No finding recurs in this review. The previously recurring linkage, declarator-selection, and field-enumeration classes now pass their expanded regression matrices and the additional probes above.

## Input robustness matrix

No new manifest, lockfile, or configuration format reader was introduced. The relevant persisted input remains the JSON symbol snapshot. Its CLI-written-fixture test walks ten load-bearing fields: kind, container kind, visibility level, modifiers, declaration text, implemented-trait relations, kind-data discriminator, type fields, field visibility, and field static flag.

For each field, the test exercises a positive control, absence, explicit null, wrong whole-field type, one wrong element, all wrong elements, empty content, a duplicate key, and trailing invalid content: **90 cells**. The test passed. Accepted optional/default shapes assert the served public value and cache sentinel; rejected shapes assert source-derived recomputation. The corresponding documented outcomes remain consistent with the schema. Signature traversal changed source extraction, not this serialized-input contract.

## Verification levels

| User-facing requirement | Strongest verification present | Assessment |
|---|---|---|
| Kinds, containers, visibility, and both static meanings across supported languages | L1 public-library matrices and CLI subprocess tests | Appropriate; passing |
| Function signatures, parameters, defaults, and variadic details | L1 source-derived expected lists through library/record conversion and representative CLI output | Appropriate; passing |
| Complete field lists and class static/instance grouping | L1 expected-name/flag matrices across output paths and CLI class assertions | Appropriate; passing |
| Declaration text and Rust implemented-trait relations | L1 public-library and record assertions | Appropriate; passing |
| Constructor/alias filtering, visibility labels, and JSON/model agreement | L1 isolated CLI subprocess assertions | Appropriate; passing |
| Pretty/plain textual label agreement | L1 forced-style output with styling removed | Appropriate for label agreement; does not claim real-terminal rendering proof |
| Schema versions, stale-cache rejection, reuse, and malformed persisted input | L1 isolated CLI snapshot tests | Appropriate; passing |

The specification explicitly assigns these analysis and textual-output contracts to L1. It adds no keyboard, mouse, paste, scrolling, or terminal-emulator behavior requiring L2/L3. Cross-OS evidence is left to CI and does not affect this readiness decision.

## Scope and limitations

Readiness does not expand the committed precision matrix. Current documentation records unnamed C/C++ parameters, base-type-only C/C++ type annotations, conservative template-header spelling comparison, deeper wrappers around qualified out-of-class C++ definitions, cross-file resolution limits, and the two known type-metadata adapter losses. Returned-callback parameter symbols retain their pre-existing containing-type scope, as recorded in the implementation log; the repaired function signature itself selects only the actual function's parameters. None is a newly established defect in the committed contract.

Implementation is complete and ready for review. The fix remains in its active directory for the author to close the review cycle.
