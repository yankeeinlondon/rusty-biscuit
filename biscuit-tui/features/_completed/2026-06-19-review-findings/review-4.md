---
ready: false
agent: codex/default
created: 2026-06-29T21:05:09
implemented: true
---

# Review 4

The review-3 Level 3 hotkey gap is addressed: this iteration adds macOS
WezTerm/cliclick physical-key coverage for Ctrl+Shift and Alt+Shift in both
`choose-one` and `choose-many`, while preserving the lower-level Level 1 and
Level 2 contracts.

I do not consider the feature production-ready yet. One high-severity
verification mismatch remains for the Windows captured-stdout contract.

## Findings

### High: Windows captured-stdout CI still does not prove the required attached-console shape

Spec requirement: on Windows, `question` with stdout captured and a console
attached must render the prompt to the console and write only the submitted
value to captured stdout.

Strongest verification present: Level 1/in-process handle tests, macOS
cross-compilation for `x86_64-pc-windows-gnu`, and an ignored Windows
integration test wired to a GitHub Actions workflow.

The new test explicitly requires "a real attached console" at
`biscuit-tui/cli/tests/windows_captured_stdout.rs:118`, and its setup depends
on stderr/stdin being inherited so the prompt can still see the console
(`windows_captured_stdout.rs:129`). That matches the production guard:
`run_standalone_with_chrome` exits before rendering when both stdout and stderr
are non-terminals (`lib/src/core/standalone/mod.rs:214`), and the Windows
redirect is inactive when `stderr` is not terminal
(`terminal_lifecycle.rs:418`).

The workflow added at
`.github/workflows/biscuit-tui-windows-captured-stdout.yml:53` runs plain
`cargo test` on `windows-latest`. GitHub Actions captures process output; it
does not establish or assert that the test process has an attached Windows
console whose stderr satisfies `is_terminal()`. `--nocapture` only disables the
Rust test harness output capture; it does not turn the Actions runner pipe into
a console. In that environment the child `question` process can take the
headless error path instead of exercising the required captured-stdout-with-
console behavior, so this workflow is not yet reliable evidence for F2.

Recommended fix: run the boundary test under a harness that explicitly creates
and verifies the Windows console shape before spawning `question`, or make the
test itself allocate/attach a console and fail early with a clear diagnostic
unless `stderr.is_terminal()` and `CONOUT$` are usable. The CI assertion should
prove the precondition, then assert captured stdout has no ESC/TUI bytes and
contains the submitted value. A successful run of the current workflow alone is
not enough unless it records that the attached-console precondition held.

## Verification Level Matrix

| Requirement | Strongest observed verification | Result |
|---|---:|---|
| F1: failed terminal setup unwinds raw mode/alt-screen state | Level 1 fault-injected unit tests | Acceptable |
| F2: Windows captured stdout renders prompt to console and value to captured stream | Level 1 handle tests + compile check + workflow that does not prove an attached console | Gap, needs executed Windows boundary verification in the required console shape |
| F3: `try_new` typed errors for invalid table rows, including missing column ids | Level 1 public API unit tests | Acceptable |
| F4: strict `input-table` JSON validation | Level 1 CLI/parser tests | Acceptable |
| F5: relaxed Ctrl/Alt+Shift hotkey matching for `choose-one` and `choose-many` | Level 3 physical-key tests for both components, plus Level 2 byte and Level 1 reducer tests | Acceptable |

## Checks Run

- `cargo test --color=never -p biscuit-tui input_table::table::tests::try_new_returns_missing_column_id_with_context --lib` — passed.
- `cargo test --color=never -p biscuit-tui-cli --test level3_chord_select --no-run` — passed.
- `cargo test --color=never -p biscuit-tui-cli --test windows_captured_stdout --no-run` — passed on macOS host profile.
- `cargo check --color=never -p biscuit-tui-cli --target x86_64-pc-windows-gnu --test windows_captured_stdout` — passed.

I did not run `just test-l2`, `just test-l3`, `just lint`, or the Windows
workflow. The remaining blocker is about the Windows runtime environment, not
macOS compilation.
