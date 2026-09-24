# Test-input probe after the move — `biscuit-tui-cli`

Spec hazard 6, rulings R15. `baseline/test-inputs.md` records no probe for this package:
its tests read no tracked non-source file, so there is no unit to narrow. The control is
that the move must not change that.

`baseline/test-input-probe.py` (the planner's own `test_inputs.scan`) was re-run on the
moved tree. The move is uncommitted, so the run used a temporary `GIT_INDEX_FILE` holding
`git add -A biscuit-tui sniff/cli tools/test-toolkit` (the checkout's own index was not
touched).

| Side | References | Unit |
|---|---:|---|
| before (`baseline/test-input-probe-before.json`) | 0 | — |
| after | 1 | `binary_id(biscuit-tui-cli::l1) & test(=test_layout::every_test_source_is_compiled_by_a_declared_target)` |

The one after reference is new, not moved: the layout gate
(`biscuit-tui/cli/tests/l1/test_layout.rs:10`) reads `biscuit-tui/cli/Cargo.toml` through
`manifest_dir!()`, so a `Cargo.toml` change now schedules exactly that gate. Every other
package this wave migrated gained the same row. No former test gained or lost a reference.

**Verdict: unchanged** (no former test reads a tracked non-source file).

F5 note (R15): `windows_captured_stdout` moved from a non-marker binary into `level2`, which
`test_inputs.py` never narrows into. It has no test-input reference before or after, so no
narrowing is lost.
