# Test-input probe after the move — `dmls`

Spec hazard 6, rulings R15. Script: `measure/test-input-check.py`, which re-runs
`baseline/test-input-probe.py` (the planner's own `test_inputs.scan`) on the moved tree and
lists each unit with `cargo nextest list -E '(<unit>) & (<L1 tier filter>)'` under the CI
feature union (`terminal-tests,effects-instrumentation`).

Probe path: `darkmatter/dmls/tests/fixtures/mapping_only_corpus/_docs.md`

| Side | Reference (source:line) | Unit | Tests selected |
|---|---|---|---:|
| before | `darkmatter/dmls/tests/mapping_only_corpus.rs:35` | `binary_id(dmls::mapping_only_corpus)` | 2 |
| before | `darkmatter/dmls/tests/mapping_only_corpus.rs:36` | `binary_id(dmls::mapping_only_corpus)` | 2 |
| after | `darkmatter/dmls/tests/l1/mapping_only_corpus.rs:35` | `binary_id(dmls::l1) & test(/^mapping_only_corpus::/)` | 2 |
| after | `darkmatter/dmls/tests/l1/mapping_only_corpus.rs:36` | `binary_id(dmls::l1) & test(/^mapping_only_corpus::/)` | 2 |

Before set mapped through `dmls-migration.json`: 2 tests. After set: 2 tests. Missing: 0. Extra: 0.

**Verdict: identical.** The full lists are in `test-inputs.json`.
