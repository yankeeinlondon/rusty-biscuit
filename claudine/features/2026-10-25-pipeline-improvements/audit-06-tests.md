---
kind: implementation-log
---

# Test Audit: 2026-10-16-core-schema-types

Added **4 genuine coverage tests / Nextest identities** (gross and net +4),
strengthened **0 existing tests**, and modified **0 slow tests**. No scheduling
splits or removed tests. Final scoped parallel L1 runs passed **440 distinct
tests**: 329 in Darkmatter's area and 111 in Claudine's area. Every test was
below five seconds; maximum **4.721 seconds**, in the existing expression-scope
scalar-union conformance test. The four added tests took 0.023–0.110 seconds.

## Coverage and repaired defects

| Requirement | Verification boundary and evidence |
| --- | --- |
| `unknown` / alias `any`, native `null`, defaults, canonical serialization | Core grammar/value tests exercise parsing, located errors, local/imported definitions, lowering, native-value preservation, and serializer round trips. |
| Explicit null-admitting values never require presence; nullable items do not make containers optional | Effective-presence tests compare ordinary, launch, completion, trigger, and composition behavior; raw JSON Schema remains a negative control. Real `md schema validate` and `claudine compose` cases exercise command wiring and launch/completion gates. |
| Alphanumeric options, identity, grammar failures, no scalar coercion | Existing grammar/value suites, option-conflict origins, exact editor diagnostics, and descriptor tests. The added exported-pattern test enumerates all strings up to length three over an eight-character representative alphabet across all 16 combinations of leading/double-period restrictions; both the typed matcher and an independently built `jsonschema` validator must agree with a direct character-rule oracle. |
| Subtype relation ignores ordinary constraints but preserves type identity | Existing scalar, enum, string-family, array, and recursive-reference tests; two new nested-identity regressions cover object, pattern-key, union-array, tuple/spread, and function children, with negative controls for literal/member/options differences, property names, array boundaries, positional optionality, return/rest/fallibility, and unresolved references. Expression-call and recursive typed-consumer tests cover downstream use. |
| Passive `shell` validation and authored-versus-data command shapes | Runtime-parity tests assert approved authored expansion, denied approval, exact data retention without approval, candidate coercion, and schema/composition parity. Closed action shapes are tested through library, real `md` validation, and the LSP harness. |
| Shipped `any(required)` migration and side-effect schemas | Shipped artifact corpus, side-effect partial validation, malformed-trigger-input cases, and shipped CLI validation exercise current schemas rather than obsolete snapshot shapes. |
| Editor keywords/options/hover and `md schema about` | In-memory LSP requests verify completion edits, canonical hover descriptions, diagnostic ranges and related locations; CLI and descriptor tests verify the public catalog. |

The audit found and fixed two previously recorded public-API defects:

- Serialization erased redundant option assertions on unresolved references.
  A rejected `slug(block-leading-dash)` could become accepted `slug` after a
  round trip. The serializer now retains the assertion until its target is
  resolved, while concrete alphanumeric types still canonicalize. The new
  regression checks three restrictions across five local/imported/array/nested
  forms, requiring both original and serialized inputs to retain the typed
  option-conflict error.
- Structured subtype identity used raw child AST equality, so ordinary child
  constraints could change the answer, while an unresolved nested reference
  could incorrectly prove identity. A borrowed recursive comparison now applies
  the existing primitive identity rules inside structures. It retains shape
  and positional boundaries; it introduces no structural object widening.

All three regression tests for these defects were observed failing before the
production fixes and passing afterward. Relevant source comments were updated.
The documentation agent was informed; the mini-review retains the findings
and marks them addressed during the audit.

The grammar/type contracts and protocol assertions are synthetic F1 checks,
appropriate for behavior Darkmatter defines. Exported schemas additionally
cross a real independent JSON Schema consumer (F2). No requirement depends on
a terminal, browser, device, or live provider; **L2 is not applicable** and no
windows were opened. Local runtime evidence is macOS only; no OS-specific code
or path-comparison logic changed.

## Verification

Final Darkmatter scoped parallel L1 command, from `darkmatter/`:

```sh
BISCUIT_TEST_FILTER='test(schemas_core_type_) | test(schemas_effective_presence) | test(core_schema_types) | test(schema_validate_core_types) | test(schemas_runtime_parity) | test(side_effect_partial) | test(simplified::subtype) | test(simplified::alphanumeric) | test(schema_about) | test(schema_phase_validation) | test(schema_triggers::steps_) | test(shipped_schema_corpus) | test(simplified::serialize) | test(call_check) | test(compose_call_precheck) | test(recursive_schema_typed_consumers) | test(expression_type_scope)' just test
```

Result: **329 passed**, maximum **4.721 seconds**, zero tests over five seconds.
Log: `/tmp/pipeline-s06-final.log`. Initial narrower runs also passed 69, 142,
and 178 tests; those repeated identities are not added to the final total.

Claudine consumer command, from `claudine/`:

```sh
BISCUIT_TEST_FILTER='test(compose_presence_parity) | test(composition::schema::tests)' just test
```

Result: **111 passed**, maximum **2.089 seconds**. Log:
`/tmp/pipeline-s06-claudine.log`. These runs used Nextest's parallel L1
scheduling and did not overlap a lint build.

Narrow source/test lint, from the repository root, after the test runs:

```sh
cargo clippy -p darkmatter --lib --test l1 --features effects-instrumentation -- -D warnings
```

Result: exit 0. Log: `/tmp/pipeline-s06-clippy.log`. `git diff --check` passed.
No full-area test suite, formatting command, commit, or cross-OS run was made
by this slice; final package gates belong to the coordinator.
