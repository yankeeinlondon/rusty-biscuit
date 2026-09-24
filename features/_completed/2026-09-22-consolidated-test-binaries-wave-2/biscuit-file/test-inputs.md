# Test-input probe after the move — `biscuit-file`

Spec hazard 6, rulings R15. Script: `measure/test-input-check.py`, which re-runs
`baseline/test-input-probe.py` (the planner's own `test_inputs.scan`) on the moved tree and
lists each unit with `cargo nextest list -E '(<unit>) & (<L1 tier filter>)'` under the CI
feature union (`fetch`).

Probe path: `biscuit-file/lib/tests/corpus/yaml_corpus.json`

| Side | Reference (source:line) | Unit | Tests selected |
|---|---|---|---:|
| before | `biscuit-file/lib/tests/yaml_corpus.rs:69` | `binary_id(biscuit-file::yaml_corpus)` | 13 |
| before | `biscuit-file/lib/tests/yaml_corpus.rs:70` | `binary_id(biscuit-file::yaml_corpus)` | 13 |
| before | `biscuit-file/lib/tests/yaml_corpus.rs:71` | `binary_id(biscuit-file::yaml_corpus)` | 13 |
| after | `biscuit-file/lib/tests/l1/yaml_corpus.rs:69` | `binary_id(biscuit-file::l1) & test(/^yaml_corpus::/)` | 13 |
| after | `biscuit-file/lib/tests/l1/yaml_corpus.rs:70` | `binary_id(biscuit-file::l1) & test(/^yaml_corpus::/)` | 13 |
| after | `biscuit-file/lib/tests/l1/yaml_corpus.rs:71` | `binary_id(biscuit-file::l1) & test(/^yaml_corpus::/)` | 13 |

Before set mapped through `biscuit-file-migration.json`: 13 tests. After set: 13 tests. Missing: 0. Extra: 0.

**Verdict: identical.** The full lists are in `test-inputs.json`.
