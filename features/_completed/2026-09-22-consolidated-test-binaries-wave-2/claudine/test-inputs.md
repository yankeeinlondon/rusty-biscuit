# Test-input probe after the move — `claudine`

Spec hazard 6, rulings R15. Script: `measure/test-input-check.py`, which re-runs
`baseline/test-input-probe.py` (the planner's own `test_inputs.scan`) on the moved tree and
lists each unit with `cargo nextest list -E '(<unit>) & (<L1 tier filter>)'` under the CI
feature union (`none`).

Probe path: `claudine/cli/Cargo.toml`

| Side | Reference (source:line) | Unit | Tests selected |
|---|---|---|---:|
| before | `claudine/lib/tests/boundary_lint.rs:78` | `binary_id(claudine::boundary_lint)` | 7 |
| before | `claudine/lib/tests/tts_phase5_contract.rs:14` | `binary_id(claudine::tts_phase5_contract) & test(=shipped_claudine_artifacts_enable_native_detached_audio)` | 1 |
| after | `claudine/lib/tests/l1/boundary_lint.rs:78` | `binary_id(claudine::l1) & test(/^boundary_lint::/)` | 7 |
| after | `claudine/lib/tests/l1/tts_phase5_contract.rs:14` | `binary_id(claudine::l1) & test(=tts_phase5_contract::shipped_claudine_artifacts_enable_native_detached_audio)` | 1 |

Before set mapped through `claudine-migration.json`: 8 tests. After set: 8 tests. Missing: 0. Extra: 0.

**Verdict: identical.** The full lists are in `test-inputs.json`.
