---
kind: implementation-log
---

# Test audit: 2026-10-29-recursive-schema-types

Added **8 net Nextest identities**: two new behavioral tests, three new roster
guards, and three additional scheduling identities from splitting preserved
cases. There are **11 new identities and 3 replaced identities**. Strengthened
**1 existing test**. Repaired **3 original slow tests** without removing a
case or assertion. No production behavior changed.

Final scoped parallel L1 runs passed **510 distinct tests**: 354 Darkmatter
library/CLI/DMLS tests and 156 Claudine caller/completion tests. Maximum
duration was **4.052 seconds**; none exceeded five seconds. No L2 contract was
introduced or changed: these requirements concern schemas, command wiring,
and editor protocol responses, rather than terminal input or rendering.

## Coverage changes

`recursive_schema_types::dictionary_and_tuple_recursion_reject_bad_values_beyond_the_old_cutoff`
checks both valid and invalid recursive dictionary values and tuple slots at
depth eight through full document validation. The preexisting descending-edge
matrix checked these shapes with valid inputs only. The new assertions require
the invalid descendant's exact path, so silently treating a deep reference as
unconstrained cannot pass. Its final duration was **0.031 seconds**.

`schema_cache_validity::editing_and_deleting_either_side_of_a_cached_recursive_import_is_observed`
uses one long-lived schema API over a two-file cycle. For each file it edits
a constraint, restores it, deletes the file, and recreates it. It checks each
verdict transition and requires the missing dependency's typed `NoMatch`
error to name the correct reference. The existing transitive cache test was
acyclic. The new test took **0.043 seconds**.

The generator portability test previously returned successfully if neither
Python candidate worked. It now uses the Test Toolkit's L1 `expect_level!`
gate, so a missing interpreter is a failed prerequisite rather than an
untested green result. The real interpreter ran the generator from an
unrelated directory in **0.078 seconds**. Its existing small YAML shim is
retained: this tests path portability and output placement, while the frozen
schema tests independently parse and validate the actual YAML artifacts.

## Slow tests repaired

| Original identity | Observed before | Preserved cases after | Final case maximum |
| --- | ---: | --- | ---: |
| `compose_schema_cli::compose_unresolved_caller_file_fails_before_initialize_when_non_interactive` | 5.372s | Templated sibling and two-tree union, each retaining its resolvable control and no-initialize/no-provider failure | 3.031s |
| `ctx_launch_anchor::caller_file_anchor_survives_direct_and_proxy_success_guards` | 5.080s | Direct and proxied documents, each checking body and lifecycle file identity and launch context | 3.027s |
| `compose_schema_cli::compose_late_caller_file_verdict_names_the_launch_directory` | 5.150s | Resolvable control and late missing-file verdict, retaining provider/event/diagnostic assertions | 2.848s |

Each split has a roster guard comparing the scheduled indices with the source
case table. All original cases remain independently scheduled by Nextest.
The final consumer run's overall maximum, 4.052 seconds, was the existing
`sequence_overlay_pty` setter/status test; the repaired cases were faster.

## Requirement map and fidelity

| Criteria | Boundary and evidence |
| --- | --- |
| 1–3: pinned validator, acyclic compatibility, typed walkers | Independent Draft 2020-12 validator probes; unchanged frozen golden outputs; recursive clean, completion, reporting, rewrite, caller-file, typed-consumer, and call-check suites. Golden text/digests and schema output are transformation contracts; the real validator is the independent consumer. |
| 4–5: finite size and lazy editor traversal | Frozen recursive/unrolled grammar comparison, two-megabyte bound, depth-invariance controls, and LSP completion/hover/definition tests with cursor-path lookup counters. No elapsed-time assertion substitutes for these work/size contracts. |
| 6: presence and finite leaves | Required empty-list leaves; missing/null child failures; optional-null versus non-null list items; use-site constraints and independently constrained references. New dictionary/tuple invalid-child controls close the descending-shape gap. |
| 7: generator portability | Real Python executes the generator in a fresh-checkout-shaped temporary tree from another CWD; exact output placement is asserted and Python is now an explicit L1 requirement. |
| 8 and 11: graphs, merge safety, descending cycles | Self/mutual/two-file cycles, alias rejection and located spans, mixed descending/non-descending groups, namespace separation, raw/simplified assembly and literal-data preservation. The new long-lived cyclic-dependency test adds edit/delete/recreate verdict checks. |
| 9: phase, conversion, invocation | Launch/Completion shared-definition tests, conversions beyond the old four-expansion cutoff, atomic failure/resource-limit controls, actual `md schema validate`/`about`/compose subprocesses, and LSP positioned diagnostics. Caller/completion tests exercise launch-directory identity through the real Claudine CLI. |
| 10: consumer audit | Recursive typed-consumer, caller-file, assignment, glob/match, root-union, editor, and call-check suites. The concurrent documentation agent owns current documentation and canonical references. |
| 12: timing output/configuration | Library hit/miss/no-compilation/early-error tests; real CLI verbosity/stdout checks; real DMLS process log/configuration tests, including thresholds and context-sensitive cache separation. |
| 13–14: benchmark and recorded evidence | Inspected the benchmark's frozen-only cold/warm fixtures, setup-time cache reset and cache-probe assertions; reran the cache-reset behavior test. The implementation log already records each waste item's disposition and one-host before/after results with load. Those historical measurements were not rerun or presented as new audit measurements. |
| 15: cache and trigger behavior | Structural sharing with current positions, context-sensitive file/match controls, baselines/bundles/trigger ordering/phases, transitive edits/deletions, nearer candidates, and matched/unmatched malformed payload errors. |

Synthetic grammar and value inputs are appropriate F1 transformation checks.
Real `jsonschema` consumption, hermetic `md`/Claudine subprocesses, DMLS stdio
logs, and the Python generator cross their implementation boundaries. No
provider wire-format, live network, audio, or native keyboard behavior is
assumed by this feature, so no F3 resource is needed. The editor tests prove
protocol responses and positions, not a particular editor's presentation.

## Verification

Final commands, each L1 run using parallel Nextest execution:

```sh
cd darkmatter
BISCUIT_TEST_FILTER='test(recursive_schema) | test(schema_recursive) | test(schema_cache_validity) | test(schema_validation_timings) | test(schema_validate_timings) | test(validation_timing) | test(recursive_call_check) | test(coercion_translator_conformance) | test(reset_process_caches) | test(match_copy) | test(reference_cycle) | test(assignment) | test(trigger_payload_load_errors) | test(golden) | test(frozen)' just test
# 354 passed; maximum 1.164s

cd ../claudine
just test supplied caller_file root_union schema_completion
# 156 passed; maximum 4.052s

cd ..
cargo clippy -p darkmatter -p claudine-cli --test l1 --features darkmatter/effects-instrumentation,claudine-cli/test-fixtures -- -D warnings
git diff --check
```

Logs: `/tmp/pipeline-s19-final.log`,
`/tmp/pipeline-s19-claudine-final.log`, and `/tmp/pipeline-s19-lint.log`.
The exact two-target Clippy command and `git diff --check` both passed.
The initial 150-test consumer run exposed two slow identities; an intermediate
154-test run exposed the third. The final 156-test run passed with no slow
test. An accidental root invocation interpreted test-name filters as package
selectors and failed before execution; the documented command ran from the
correct package area.

Evidence is from this macOS host. Fixtures use temporary directories and
explicit schema text, and the existing Windows-aware native-versus-portable
path assertions were preserved. The early/late caller-file fixtures already
have Unix gates; this audit neither adds platform skips nor claims those
Unix subprocess cases are native-Windows evidence. Cross-OS execution and
the coordinator's final package gates are separate evidence.
