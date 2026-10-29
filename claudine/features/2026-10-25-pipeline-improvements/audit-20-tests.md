---
kind: implementation-log
---

# Test audit: 2026-10-30-claudine-recursive-schemas

Added **one genuine test function/Nextest identity**; gross and net identity
increase are both **one**. No existing tests were strengthened, no scheduling
splits were needed, and **zero slow tests were modified**. Production behavior
is unchanged.

Final verification passed **1,085 distinct L1 test identities**, with no test
over five seconds and a maximum of **4.666 seconds**.

The new container test combines the four container paths at container depth
four with a generated action in the `else` branch of conditional depth four.
For both Launch and Completion, it accepts a deferred numeric argument beside
a valid literal and rejects a bad literal sibling, missing argument, extra
argument, or unknown named parameter at the exact nested instance path. Its
40 verdicts count as one test, not 40 additions. This closes the combination
gap between existing deep-container literal tests and shallow phased
operand-deferral tests.

The test creates one temporary repository and one discovery context, resolves
each container path once, and reuses that effective schema across its cases.
It uses the existing guarded four-path roster. No worker threads, real speech,
live provider, terminal, or browser are involved.

## Requirement coverage

| Criterion | Boundary and evidence |
| --- | --- |
| 1: recursive schema structure and size | `claudine_schema_parity`, `claudine_schemas_embedded`, and `container_schema_recursion` resolve disk and embedded bundles, inspect definition identities and the absence of retired unrolled names, and enforce the compact effective-schema size budget. |
| 2: generated argument types | Shared generator and operand suites check scalar, positional, named, optional, nullable, variadic, overloaded, source-excluded, and numeric-recovery cases against actual registrations. Missing metadata and removed overloads fail generation; generated regions must match regeneration byte-for-byte. |
| 3: recursive profile restrictions | All three event families are exercised in branches, dictionary entries, bundles, and loop forms; unknown verbs and initialization/cancellation restrictions retain exact paths. |
| 4: parser-only nesting limit | Five-level controls pass both schema and parser; sixth-level and guarded-bundle controls pass the schema and fail the parser. Container events start independent counts. |
| 5: editor parity and passivity | Real DMLS protocol harness and editor-provider tests check CLI-matching diagnostic positions, depth-four completion/hover, profile filtering, and recorder evidence that validation/editor requests execute no actions. |
| 6: launch and shipped corpus | Real hermetic Claudine CLI cases prove malformed controls block providers, valid deferred operands reach event-time binding, wrong evaluated values block dispatch, and dry-run output retains authored operands. The separate shipped-corpus slice checks examples, research documents, and frozen prompt copies. |
| 7: documentation and regeneration | The concurrent documentation agent owns canonical pages. Shared generation tests compare parsed forms and bytes with the checked-in catalog projection, including deterministic repeated projections. No frozen input fixture was rewritten. |
| 8: deferred arguments and conversion | Shared raw/stored/phased tests cover literal-token origin, malformed templates, structured child deferral, invalid siblings/keys/arity, positions, unchanged failed conversion, and event-time no-dispatch errors. The new test repeats the important literal/shape distinction through both phases under recursive container and conditional references. |
| 9: metadata coverage | `lifecycle_schema_generation` verifies every catalog side effect and expression function, including explicit failure on missing effect metadata, removed overloads, and name collisions. No generator-local type table is introduced. |
| 10: recursive containers | The guarded four-path matrix covers all seven events, all eight forbidden cancellation directives at roots and both depth-four branches, valid stop/empty cancellation, initialization restrictions, generated literal arguments, scalar steps, file references, and independent depth counts. The new phased-deferral test covers deeper combined behavior. |
| 11: recorded performance goal | Reused the implementation log's equivalent nested-container sample: 60.7–64.6 ms validation stage total and 0.09–0.14 s command wall time. The under-200-ms objective is a recorded goal, not a test gate; no benchmark was rerun or threshold relaxed. |

The schema, coercion, and recursive graph contracts are transformation tests
(F1). Real JSON Schema compilation, generated-file loading, hermetic `md` and
Claudine processes, and DMLS protocol sessions exercise the corresponding
consumer/wiring boundaries (F2). Fake provider/service recorders prove the
specific dispatch/passivity assertions; they do not claim live-provider
behavior. The feature promises no terminal input/rendering or hardware
behavior, so **L2 is not applicable** and no application window was opened.

## Verification

All L1 selections used Nextest's parallel execution through canonical recipes.
Lint ran after test execution, without competing for the measured CPU budget.

From `claudine/`:

```sh
BISCUIT_TEST_FILTER='test(container_schema_recursion) | test(claudine_schema_parity) | test(claudine_schemas) | test(sequence_schema) | test(launch_trigger_schemas)' just test
BISCUIT_TEST_FILTER='test(container_schema_recursion)' just test-library
BISCUIT_TEST_FILTER='test(claudine_schema_shipped_corpus)' just test-library
```

From `darkmatter/`:

```sh
BISCUIT_TEST_FILTER='test(lifecycle_generated_operands) | test(lifecycle_operand_validation) | test(lifecycle_event_binding) | test(lifecycle_schema_generation) | test(schema_nested_template_deferral) | test(claudine_editor_parity) | test(claudine_schema_bundle) | test(schema_validate_timings)' just test
```

From the repository root:

```sh
cargo clippy -p claudine --test l1 -- -D warnings
git diff --check
```

The initial Claudine scope passed **881 tests**, maximum **4.666 seconds**.
The final container module passed **19 tests**, maximum **2.661 seconds**
(the new regression). Eighteen module identities repeat initial observations.
The Darkmatter/CLI/DMLS slice passed **136 tests**, maximum **3.746 seconds**.
The shipped/frozen corpus passed **67 tests**, maximum **2.830 seconds**.
The union of those selections contains **1,085 distinct identities**.
Exact-target Clippy and `git diff --check` passed. Compile and link time are
separate from per-test durations; the existing macOS unwind-table linker note
does not indicate a failing assertion or Clippy diagnostic.

Logs: `/tmp/pipeline-s20-claudine-initial.log`,
`/tmp/pipeline-s20-container-final.log`, `/tmp/pipeline-s20-darkmatter.log`,
`/tmp/pipeline-s20-corpus.log`, and `/tmp/pipeline-s20-clippy.log`.

## Limits and integrated behavior

Evidence here is from the current macOS host. No platform-specific production
code or path-comparison contract changed; no cross-OS result is claimed. The
new test uses temporary repository paths and the existing portable fixture.
Later pipeline changes supersede the snapshot's old trigger-disable spelling;
the audit retains the current policy grammar. Existing deliberate schema/parser
differences, including parser-only nesting and optional category-key null
handling, are not silently reclassified as schema requirements.
