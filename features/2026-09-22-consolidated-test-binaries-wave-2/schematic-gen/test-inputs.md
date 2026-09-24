# Test-input probe after the move — `schematic-gen`

Spec hazard 6, rulings R15. Script: `measure/test-input-check.py`, which re-runs
`baseline/test-input-probe.py` (the planner's own `test_inputs.scan`) on the moved tree and
lists each unit with `cargo nextest list -E '(<unit>) & (<L1 tier filter>)'` under the CI
feature union (`terminal-tests`).

Probe path: `schematic/gen/tests/fixtures/complex_auth.json`

| Side | Reference (source:line) | Unit | Tests selected |
|---|---|---|---:|
| before | `schematic/gen/tests/openapi_import_test.rs:9` | `binary_id(schematic-gen::openapi_import_test)` | 8 |
| before | `schematic/gen/tests/postman_golden.rs:69` | `binary_id(schematic-gen::postman_golden)` | 6 |
| before | `schematic/gen/tests/postman_golden.rs:70` | `binary_id(schematic-gen::postman_golden)` | 6 |
| after | `schematic/gen/tests/l1/openapi_import_test.rs:9` | `binary_id(schematic-gen::l1) & test(/^openapi_import_test::/)` | 8 |
| after | `schematic/gen/tests/l1/postman_golden.rs:69` | `binary_id(schematic-gen::l1) & test(/^postman_golden::/)` | 6 |
| after | `schematic/gen/tests/l1/postman_golden.rs:70` | `binary_id(schematic-gen::l1) & test(/^postman_golden::/)` | 6 |

Before set mapped through `schematic-gen-migration.json`: 14 tests. After set: 14 tests. Missing: 0. Extra: 0.

**Verdict: identical.** The full lists are in `test-inputs.json`.
