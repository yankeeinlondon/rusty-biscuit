---
kind: implementation-log
---

# Test audit: 2026-10-13-named-type-presence-and-local-names

All **635 distinct scoped L1 tests passed** across Darkmatter (240), Claudine
(253), and Messenger (142). The maximum duration was **4.265 seconds**. The
final Messenger run cleared the generated `messenger-research-fixtures`
build cache and ran in parallel without a concurrent lint/build. No test in that cold run exceeded five seconds.

## Counting changes

There are **41 newly named test functions/nextest identities**, **4 replaced
identities**, and therefore **37 net additional identities**. Of the 41 new
functions, 32 are macro-generated and 9 explicitly declared; these are actual
nextest identities, not rows hidden inside one function. The equal-default
regression has three numeric/container cases inside one test and counts once.

The net increase separates into:

- **8 new coverage or guard identities**: two numeric-default regressions,
  the previously missing eighth artifact replacement, one fixture-cache
  correctness test, and four completeness guards for the split case rosters.
- **29 extra scheduling identities preserving existing coverage**: publication
  interruptions (+12), promotion interruptions (+7), renewal refusals (+3),
  and separately scheduled initial-publication outcomes and per-platform
  durable stage checks (+7). These do not claim 29 new functional scenarios.

One existing local-reference test was strengthened to assert the inherited
description and its explicit replacement, instead of only comparing bare
and explicit references with one another. **Seven originally slow test
identities were repaired** through their own changes or shared fixture setup.
The relocated Claudine spike model and its existing differential tests are
preserved coverage, not additional tests.

## Defects and coverage gaps repaired

The public `expand_named_types` API accepts programmatically constructed
`Constraint::Default` values. Its duplicate numeric normalization disagreed
with lowering: equal integer/float defaults at `2^63` produced a conflict,
while `i64::MAX` and a `2^63` float incorrectly merged through a saturating
cast. Both new regressions failed before the fix. Resolution now shares the
converter's normalization, including nested arrays/objects and the unsigned
integer range. Authored literals already passed through normalization in the
grammar; the defect was in the programmatic API. The relevant source comments
were updated, and the duplicate implementation was removed.

The publication interruption matrix assumed seven artifacts, but publication
now also emits a changelog: there are eight. Replacement position eight is
now exercised. A guard checks that the named cases cover the fixture's
artifact count, and each publication case checks that count against the
verified generated snapshot.

Claudine's consolidated L1 target initially could not compile because its
differential spike test imported a model from a moved active feature
directory. Its unchanged model now lives under
`claudine/lib/tests/fixtures/lifecycle_control_flow_spike/`; the import no
longer depends on a spec's lifecycle location. This mirrors the durable
fixture repair already made for Darkmatter in step 1, without adding a
cross-package source-input dependency.

## Coverage and fidelity

| Requirement | Evidence and boundary |
| --- | --- |
| Presence survives bare, explicit local, imported and multi-hop references | Public resolver tests plus real `md schema validate` invocations; authored and completion-phase validation assert absence and wrong-type outcomes. |
| Required expressions, nested objects, array items and union arms | Named-type merge tests assert actual lowered properties and presence; converter/validation tests verify the resulting constraints. Later null and recursive-type semantics are respected. |
| Constraint merge table and contradictions | Named-type merge, conjunctive lowering and block-constraint matrices check bounds, flags, patterns, schemes, defaults, examples, suggestions and typed origins. The new regressions prove the public API's large-number equality contract. |
| Bare names, built-in collisions, forward/unknown/lexical names and dependency invalidation | Parser/resolver tests, serialization cases, DMLS protocol tests, and Claudine schema tests check the selected definition and editor refresh after an imported-file edit. Description assertions now distinguish shared incorrect behavior from correct bare/explicit equivalence. |
| Grouped file matching and passive behavior | Library root-union selection and single-schema controls, real CLI validation, and Claudine completion/supplied-value tests check intersection rather than widening and the empty-intersection result. |
| Shipped schemas and normal invocation | Passive shipped-schema corpus, real `md` and Claudine CLI cases, and Messenger's corpus/validation/publication/refresh tests exercise actual shipped schemas through their consumers. |

The internal grammar and resolver semantics use synthetic inputs (F1), which
are the contract being defined. Generated JSON Schema is consumed by the real
validator (F2); CLI cases exercise actual executables, and editor cases cross
the serialized LSP harness boundary. No promise here depends on a terminal
encoder, terminal rendering, browser, live provider or device. **L2 is not
applicable** and no terminal/browser windows were opened.

## Speed repairs without lost behavior

| Originally slow identity | Before | Final replacement/shared-fixture maximum |
| --- | ---: | ---: |
| Publication interrupted before selection | 17.863 s | 4.044 s |
| Publication interrupted after selection | 7.727 s | 4.265 s |
| Interrupted promotion | 13.021 s | 2.784 s |
| Automatic renewal refusal matrix | 6.210 s | 2.872 s |
| Initial whole-fleet publication | 6.492 s | 3.010 s |
| Unreadable run record | 6.142 s | 3.134 s |
| Verified unchanged renewal | 5.559 s | 3.108 s |

Interruption and refusal matrices are independent nextest cases with guards.
Immutable baseline setup is generated by the real code once per executable
and input-content revision, then copied into each test's private directory.
The key uses `biscuit-hash` and includes every runtime-read seed byte. An
explicit test checks reuse, input-edit invalidation, and private mutations.
Construction is locked and publishes a complete directory; no test mutates
the cached snapshot. Cache files live beside the executable so build cleanup
removes them.

Initial publication tests start from explicit ready-for-review fixture records
and reviewer inputs, rather than replaying five durable preparation/check
pipelines as setup. Five separately scheduled tests retain the real
preparation and both `check_run` boundaries for every platform, asserting
persisted stage results and validation/delta files; a roster guard prevents
new platforms from escaping those tests. Promotion still reevaluates every
candidate and performs real durable publication. Recovery tests retain their
real writes, interruption points, recovery, verification and follow-up
generation assertions. No production durability, deadline, tier, runner limit,
CI configuration or skip policy changed.

Splitting alone initially increased filesystem contention, so it was not
accepted as a fix. Intermediate failures also caught an incorrect artifact
count and a roster-order mismatch in the new guards. These were corrected.
One intermediate timing run accidentally overlapped clippy compilation and
was discarded as final timing evidence. The final cold parallel run above is
the passing timing evidence.

## Verification

Commands ran from the named package areas through nextest recipes:

```sh
# darkmatter/
BISCUIT_TEST_FILTER='test(schemas_named_type_merge) | test(schemas_conjunctive_lowering) | test(schemas_constraints_block_matrix) | test(named_type_references) | test(schema_validate) | test(shipped_schema_corpus) | test(file_match::) | test(normalize_json_number)' just test
# 240 passed; maximum 2.775 s.

BISCUIT_TEST_FILTER='test(schemas_named_type_merge)' just test
# 34 passed after strengthening the description assertions; maximum 0.153 s.

# claudine/
BISCUIT_TEST_FILTER='test(composition::schema) | test(schema_completion) | test(compose_schema_cli) | test(lifecycle_control_flow_spike)' just test
# 253 passed; maximum 3.354 s.

# messenger/
BISCUIT_TEST_FILTER='binary(research_publication) | binary(research_refresh) | binary(research_corpus) | binary(research_validation)' just test
# 142 passed; maximum 4.265 s; cleared fixture cache, zero slow.

# Repository root, after tests completed:
cargo clippy -p messenger --features desktop,research --test research_publication --test research_refresh -- -D warnings
# Passed.
```

`git diff --check` passed. Logs are
`/tmp/pipeline-s03-darkmatter-final.log`,
`/tmp/pipeline-s03-description-final.log`,
`/tmp/pipeline-s03-claudine-final.log`,
`/tmp/pipeline-s03-messenger-cold-final.log`, and
`/tmp/pipeline-s03-messenger-clippy-final.log`. Before timings are in
`/tmp/pipeline-s03-messenger-initial.log`; numeric red evidence is in
`/tmp/pipeline-s03-regression-red.log`.

This slice makes no full-area or cross-OS execution claim. The coordinator
owns the final combined package gates. No `cargo fmt` or commit was run.
