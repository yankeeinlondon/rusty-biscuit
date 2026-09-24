# Test-input probe after the move — `claudine-gen`

Spec hazard 6, rulings R15. Script: `measure/test-input-check.py`, which re-runs
`baseline/test-input-probe.py` (the planner's own `test_inputs.scan`) on the moved tree and
lists each unit with `cargo nextest list -E '(<unit>) & (<L1 tier filter>)'` under the CI
feature union (`terminal-tests`).

Probe path: `claudine/docs/research/agent-errors/_schema.yaml`

| Side | Reference (source:line) | Unit | Tests selected |
|---|---|---|---:|
| before | `claudine/gen/tests/agent_errors_check.rs:46` | `binary_id(claudine-gen::agent_errors_check)` | 10 |
| after | `claudine/gen/tests/l1/agent_errors_check.rs:46` | `binary_id(claudine-gen::l1) & test(/^agent_errors_check::/)` | 10 |

Before set mapped through `claudine-gen-migration.json`: 10 tests. After set: 10 tests. Missing: 0. Extra: 0.

**Verdict: identical.** The full lists are in `test-inputs.json`.
