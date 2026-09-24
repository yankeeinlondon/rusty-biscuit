# Test-input probe after the move — `sniff-cli`

Spec hazard 6, rulings R15. Script: `measure/test-input-check.py`, which re-runs
`baseline/test-input-probe.py` (the planner's own `test_inputs.scan`) on the moved tree and
lists each unit with `cargo nextest list -E '(<unit>) & (<L1 tier filter>)'` under the CI
feature union (`test-fixtures`).

Probe path: `sniff/cli/tests/snapshots/snapshots__cargo_monorepo_structure_json.snap` (after the move: `sniff/cli/tests/l1/snapshots/l1__snapshots__cargo_monorepo_structure_json.snap`)

| Side | Reference (source:line) | Unit | Tests selected |
|---|---|---|---:|
| before | `sniff/cli/tests/spawn_site_guard.rs:38` | `binary_id(sniff-cli::spawn_site_guard)` | 8 |
| after | `sniff/cli/tests/l1/spawn_site_guard.rs:38` | `binary_id(sniff-cli::l1) & test(/^spawn_site_guard::/)` | 8 |

Before set mapped through `sniff-cli-migration.json`: 8 tests. After set: 8 tests. Missing: 0. Extra: 0.

**Verdict: identical.** The full lists are in `test-inputs.json`.

The probe enumerates `git ls-files`. The moved tree is uncommitted, so the run used a
temporary `GIT_INDEX_FILE` holding `git add -A sniff/cli` (the checkout's own index was not
touched). Without it the scan sees only the old, tracked snapshot paths and finds no after
reference, which is an artifact of the uncommitted move, not of the planner.
