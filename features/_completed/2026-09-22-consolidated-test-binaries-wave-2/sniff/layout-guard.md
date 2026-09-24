# Layout gate red/green — `sniff`

Command: `cargo nextest run -p sniff --test l1 every_test_source_is_compiled_by_a_declared_target`
Revision: `c02e691d9` plus the uncommitted move. Script: `measure/layout-guard.sh`.

## 1. Planted stray root `tests/stray.rs`

Expected: fail. Exit status: 100 (as expected).

```text
        FAIL [   0.078s] (1/1) sniff::l1 test_layout::every_test_source_is_compiled_by_a_declared_target
    test test_layout::every_test_source_is_compiled_by_a_declared_target ... FAILED
    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 269 filtered out; finished in 0.06s
    test sources outside every declared test target:
    tests/stray.rs: a top-level test file is never compiled; move it into a declared target's directory and declare it in that target's main.rs
     Summary [   0.080s] 1 test run: 0 passed, 1 failed, 269 skipped
        FAIL [   0.078s] (1/1) sniff::l1 test_layout::every_test_source_is_compiled_by_a_declared_target
```

## 2. Undeclared module `tests/l1/orphan.rs`

Expected: fail. Exit status: 100 (as expected).

```text
        FAIL [   0.077s] (1/1) sniff::l1 test_layout::every_test_source_is_compiled_by_a_declared_target
    test test_layout::every_test_source_is_compiled_by_a_declared_target ... FAILED
    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 269 filtered out; finished in 0.06s
    test sources outside every declared test target:
    tests/l1/orphan.rs: no declared test target compiles this module; declare it with `mod`
     Summary [   0.077s] 1 test run: 0 passed, 1 failed, 269 skipped
        FAIL [   0.077s] (1/1) sniff::l1 test_layout::every_test_source_is_compiled_by_a_declared_target
```

## 3. Both removed

Expected: pass. Exit status: 0 (as expected).

```text
        PASS [   0.076s] (1/1) sniff::l1 test_layout::every_test_source_is_compiled_by_a_declared_target
     Summary [   0.076s] 1 test run: 1 passed, 269 skipped
```

