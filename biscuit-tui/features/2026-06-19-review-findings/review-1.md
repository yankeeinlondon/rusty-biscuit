---
ready: false
agent: codex/default
created: 2026-06-26T13:33:49
implemented: true
---

# Review 1

## Findings

1. **High — F2 Windows captured-stdout behavior is not verified at the level required by the spec.**

   The implementation adds a Windows `StdoutTtyRedirect` that gates on `io::stdout().is_terminal()`, opens `CONOUT$`, and swaps `STD_OUTPUT_HANDLE` with `SetStdHandle` in `biscuit-tui/lib/src/core/standalone/terminal_lifecycle.rs:415`. The spec requires proving that `question` with stdout captured and a console attached renders the prompt to the console, that the captured stream receives only the submitted value, and that Crossterm/`io::stdout()` writes after activation really target `CONOUT$`.

   The added Windows tests in `biscuit-tui/lib/src/core/standalone/tests.rs:989` only prove that `CONOUT$` can be opened, that `SetStdHandle` is reflected by `GetStdHandle`, and that the guard is inactive when stdout is already a console. They do **not** exercise the active redirect path, do **not** write through `io::stdout()` after `StdoutTtyRedirect::activate_if_piped()`, and do **not** spawn `question` with stdout captured while stderr remains attached to a console. That leaves the actual user-facing F2 requirement unverified.

   **Strongest verification present:** Windows compile check plus Windows-only in-process handle tests, effectively Level 1/compile-time.  
   **Required verification:** Windows behavioral integration for the captured-stdout prompt shape, or an equivalent Windows-only test/manual CI reproduction that proves the active redirect path and captured stream contract.

## Verification-Level Review

- **F1 terminal preparation transactional cleanup:** Level 1 fault-injection tests cover the error path after raw mode enablement and the success dismissal path. This is appropriate because the requirement is lifecycle cleanup logic, not terminal rendering or keyboard encoding.
- **F2 Windows captured stdout:** compile check passes, but behavioral verification is missing. This is the blocking gap above.
- **F3 `InputTableState::try_new`:** Level 1 unit tests cover row shape, duplicate/unknown/missing IDs, typed mismatches, and `new` panic compatibility. Appropriate for this API contract.
- **F4 input-table JSON validation:** Level 1 CLI/parser tests cover checked numeric conversion, present-wrong-type fields, row value mismatches, and documented permissive paths. Appropriate for JSON boundary validation.
- **F5 relaxed hotkey modifier matching:** Level 1 reducer tests cover `CONTROL|SHIFT`, `ALT|SHIFT`, `CONTROL|ALT`, and unchanged bare modifier chords in both choose components. Existing Level 2 tmux tests cover baseline real-terminal chord submission and badge rendering, but the new shifted-modifier payload itself is only synthetic; acceptable if the intended contract is "handle this crossterm payload once delivered."

## Notes

- I did not find functional gaps in F1, F3, F4, or F5 during source review.
- The implementation updates the public API docs, `docs/dependencies.md`, and the `biscuit-tui` skill for `InputTableError` / `try_new`.

## Commands Run

- `just test` from `biscuit-tui/` — passed.
- `rustup target add x86_64-pc-windows-msvc` — installed the target for review.
- `cargo check --manifest-path biscuit-tui/lib/Cargo.toml --target x86_64-pc-windows-msvc --color=never` — passed.
- `just lint` from `biscuit-tui/` — passed.
- `just test-l2` — not run in this non-interactive review session because it spawns/attaches real terminal panes; coverage was reviewed from source.

## Production Readiness

Not production ready. The Windows captured-stdout remediation is the cross-platform part of the feature, and its actual user-facing behavior remains unverified.
