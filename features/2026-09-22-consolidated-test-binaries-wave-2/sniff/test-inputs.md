# Test-input probe after the move — `sniff`

Spec hazard 6, rulings R15. Script: `measure/test-input-check.py`, which re-runs
`baseline/test-input-probe.py` (the planner's own `test_inputs.scan`) on the moved tree and
lists each unit with `cargo nextest list -E '(<unit>) & (<L1 tier filter>)'` under the CI
feature union (`remote`).

Probe path: `sniff/lib/benches/ci-bench-ids.txt`

| Side | Reference (source:line) | Unit | Tests selected |
|---|---|---|---:|
| before | `sniff/lib/tests/bench_ids_sync.rs:21` | `binary_id(sniff::bench_ids_sync)` | 3 |
| after | `sniff/lib/tests/l1/bench_ids_sync.rs:21` | `binary_id(sniff::l1) & test(/^bench_ids_sync::/)` | 3 |

Before set mapped through `sniff-migration.json`: 3 tests. After set: 3 tests. Missing: 0. Extra: 0.

**Verdict: identical.** The full lists are in `test-inputs.json`.
