# Test-input probe after the move — `biscuit-terminal-cli`

Spec hazard 6, rulings R15. Script: `measure/test-input-check.py`, which re-runs
`baseline/test-input-probe.py` (the planner's own `test_inputs.scan`) on the moved tree and
lists each unit with `cargo nextest list -E '(<unit>) & (<L1 tier filter>)'` under the CI
feature union (`terminal-tests`).

Probe path: `biscuit-terminal/cli/README.md`

| Side | Reference (source:line) | Unit | Tests selected |
|---|---|---|---:|
| before | `biscuit-terminal/cli/tests/integration_test.rs:2205` | `binary_id(biscuit-terminal-cli::integration_test) & test(=public_docs_do_not_advertise_removed_atomic_tokens)` | 1 |
| after | `biscuit-terminal/cli/tests/l1/integration_test.rs:2205` | `binary_id(biscuit-terminal-cli::l1) & test(=integration_test::public_docs_do_not_advertise_removed_atomic_tokens)` | 1 |

Before set mapped through `biscuit-terminal-cli-migration.json`: 1 tests. After set: 1 tests. Missing: 0. Extra: 0.

**Verdict: identical.** The full lists are in `test-inputs.json`.
