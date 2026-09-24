# Test-input probe after the move — `tree-hugger`

Spec hazard 6, rulings R15. Script: `measure/test-input-check.py`, which re-runs
`baseline/test-input-probe.py` (the planner's own `test_inputs.scan`) on the moved tree and
lists each unit with `cargo nextest list -E '(<unit>) & (<L1 tier filter>)'` under the CI
feature union (`none`).

Probe path: `tree-hugger/lib/tests/fixtures/corpus_manifest.yaml`

| Side | Reference (source:line) | Unit | Tests selected |
|---|---|---|---:|
| before | `tree-hugger/lib/tests/tree_file.rs:126` | `binary_id(tree-hugger::tree_file)` | 89 |
| before | `tree-hugger/lib/tests/tree_file.rs:127` | `binary_id(tree-hugger::tree_file)` | 89 |
| after | `tree-hugger/lib/tests/l1/tree_file.rs:126` | `binary_id(tree-hugger::l1) & test(/^tree_file::/)` | 89 |
| after | `tree-hugger/lib/tests/l1/tree_file.rs:127` | `binary_id(tree-hugger::l1) & test(/^tree_file::/)` | 89 |

Before set mapped through `tree-hugger-migration.json`: 89 tests. After set: 89 tests. Missing: 0. Extra: 0.

**Verdict: identical.** The full lists are in `test-inputs.json`.
