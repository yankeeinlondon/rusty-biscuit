---
created: "2026-06-19T16:59:25"
agent: "codex"
yolo: true
---

### 1. Executive Summary

`biscuit-tui` is a Ratatui-based input component library with a thin `question` CLI wrapper. The core architecture is generally sound: state is externalized, widgets are zero-sized, event loops are factored for synthetic tests, and the CLI has broad coverage around output modes, option sources, completions, and exit codes. Overall risk level: `medium`. The biggest strengths are the consistent component API, extensive unit tests, explicit terminal lifecycle design, and clear separation between library widgets and CLI glue. The biggest concern is that terminal setup can leave the caller's terminal in raw mode if setup fails after `enable_raw_mode()` but before `TerminalGuard` exists. The second concern is cross-platform behavior around captured stdout: the Unix implementation redirects fd 1 to `/dev/tty`, but the non-Unix implementation is a no-op despite this monorepo's macOS/Windows/Linux requirement. The code appears close to production-ready for Unix-like terminals, but still fragile around terminal lifecycle failures and Windows command-substitution-style use.

### 2. Key Findings

#### [Severity: High] Terminal setup can return with raw mode still enabled

- **Location:** `biscuit-tui/lib/src/core/standalone/terminal_lifecycle.rs::prepare_terminal`, lines 29-34; called from `run_standalone_with_chrome` at `biscuit-tui/lib/src/core/standalone/mod.rs`, lines 218-220
- **Why it matters:** Leaving raw mode enabled is one of the highest-impact failure modes for a TUI. It can corrupt the user's shell session after an otherwise recoverable terminal I/O error.
- **Evidence:** `prepare_terminal` calls `enable_raw_mode()?` and then, for fullscreen prompts, `execute!(out, EnterAlternateScreen)?`. If entering the alternate screen fails, the function returns `Err` before `TerminalGuard::new(...)` is created by the caller. Nothing disables raw mode on that path.
- **Recommendation:** Make terminal preparation transactional. Either create a guard immediately after `enable_raw_mode()` or explicitly call `disable_raw_mode()` before returning errors from later setup steps. A small internal `PrepareGuard` that is dismissed only after `TerminalGuard` takes over would keep this localized.
- **Confidence:** high

#### [Severity: Medium] Captured-stdout interactive prompts are Unix-only despite a cross-platform contract

- **Location:** `biscuit-tui/lib/src/core/standalone/terminal_lifecycle.rs::StdoutTtyRedirect`, lines 104-215; non-Unix no-op in `biscuit-tui/lib/src/core/standalone/mod.rs`, lines 347-354; activation call at lines 214-218
- **Why it matters:** The package is required to work on macOS, Windows, and Linux. The documented command-substitution path depends on redirecting prompt rendering away from captured stdout. On non-Unix platforms that redirect never happens, so the prompt can render into the captured data stream or fail to query cursor position through a pipe.
- **Evidence:** The Unix implementation opens `/dev/tty`, `dup`s stdout, and `dup2`s the terminal fd onto `STDOUT_FILENO`. The non-Unix implementation is an empty guard. The comment explicitly says the redirect is needed because Ratatui/crossterm cursor-position probing writes to `io::stdout()` and times out when stdout is a pipe.
- **Recommendation:** Add a Windows implementation using console handles or a different terminal backend/output stream strategy, or change the preflight check to reject `stdout`-piped interactive prompts on non-Unix with a clear error. Add Windows-focused tests for `question` with stdout captured and stderr attached to a terminal-like stream if the harness supports it.
- **Confidence:** medium

#### [Severity: Medium] `InputTableState::new` panics on normal data-shape errors in a public API

- **Location:** `biscuit-tui/lib/src/components/input_table/table.rs::InputTableState::new`, lines 101-116, and `normalize_row`, lines 740-768
- **Why it matters:** Row shape, duplicate IDs, unknown IDs, and missing columns are recoverable validation errors when callers build tables from user or config data. Panicking makes the library harder to embed and forces consumers to pre-validate against private normalization rules.
- **Evidence:** `InputTableState::new` panics when row length differs from column count. `normalize_row` also panics for duplicate column IDs, unknown column IDs, and missing column IDs. The CLI avoids this for its own `--rows` path by validating length first, but library callers do not have a `Result`-returning constructor.
- **Recommendation:** Keep `new` if backward compatibility requires it, but add `try_new(columns, initial_rows) -> Result<Self, InputTableError>` and have CLI code use it. Document `new` as an invariant-enforcing convenience wrapper around `try_new(...).expect(...)`.
- **Confidence:** high

#### [Severity: Medium] `input-table` JSON parsing silently coerces or truncates invalid values

- **Location:** `biscuit-tui/cli/src/commands/input_table/columns.rs`, lines 116-119 and 153-162; `biscuit-tui/cli/src/commands/input_table/mod.rs::parse_cell_value`, lines 156-203
- **Why it matters:** This is a CLI boundary that accepts user JSON. Silent coercion makes mistakes hard to diagnose and can produce a table that does not match the user's intended schema.
- **Evidence:** `max_length` is read with `as_u64().map(|n| n as usize)` and preferred dimensions use `as_u64().map(|n| n as u16)`, so large values truncate on `u16` fields. Cell parsing also coerces unsupported JSON values into strings via `other.to_string()`, treats unsupported boolean shapes as `false`, and accepts comma-splitting for choose-many string values.
- **Recommendation:** Validate numeric ranges with `u16::try_from` and reject invalid types with `InvalidInput` errors that include column/row context. Keep permissive parsing only where it is a documented compatibility contract.
- **Confidence:** high

#### [Severity: Low] Choice hotkey matching is stricter than typical terminal modifier payloads

- **Location:** `biscuit-tui/lib/src/components/choose_one.rs`, lines 576-594; `biscuit-tui/lib/src/components/choose_many.rs`, lines 582-601
- **Why it matters:** Some terminals include extra modifier bits such as `SHIFT` for uppercase chords. Exact matching on `KeyModifiers::CONTROL` or `KeyModifiers::ALT` can ignore otherwise valid hotkeys when extra benign modifiers are present.
- **Evidence:** The handlers use `match event.modifiers { KeyModifiers::CONTROL => ... KeyModifiers::ALT => ... }`. Other code in the package often uses `.contains(...)` for modifier checks, including Ctrl-C and table navigation.
- **Recommendation:** Match with `event.modifiers.contains(KeyModifiers::CONTROL)` / `.contains(KeyModifiers::ALT)` and decide explicitly whether `CONTROL | ALT` should prefer one map, be rejected, or be treated as ambiguous. Add tests for `CONTROL | SHIFT` and `ALT | SHIFT`.
- **Confidence:** medium

### 3. Rust-Idiomaticity Notes

- The public widget/state split is idiomatic for Ratatui and keeps rendering cheap.
- Error modeling is stronger in the CLI than in `InputTableState::new`; a typed `InputTableError` would make the library API more Rust-idiomatic than panics for caller-provided data.
- `CellState` intentionally uses `#[allow(clippy::large_enum_variant)]`; that is reasonable for a value-owned UI state enum, and boxing would likely complicate ownership without a measured win.
- The fd redirection code is carefully scoped behind `#[cfg(unix)]`, but its process-global nature should stay private and heavily tested.

### 4. Testing Gaps

- Add a regression test for terminal preparation failure after raw mode is enabled. This likely needs a small injectable terminal-preparation abstraction rather than real terminal fault injection.
- Add Windows coverage or an explicit non-Unix rejection test for captured-stdout interactive prompts.
- Add `InputTableState::try_new` tests for row length mismatch, duplicate IDs, unknown IDs, and missing IDs.
- Add CLI `input-table` tests for oversized `preferred_width` / `preferred_height`, non-string `initial` fields, and invalid boolean cell values.
- Add hotkey tests for `CONTROL | SHIFT` and `ALT | SHIFT` modifier combinations.

### 5. Unsafe Code Review

Production unsafe exists in the Unix stdout redirect only:

- `libc::open(c"/dev/tty".as_ptr(), ...)` at `terminal_lifecycle.rs:163`
  - **Invariant:** the C string must be NUL-terminated and the fd result must be checked before use.
  - **Assessment:** upheld. The code uses a static C string literal and checks `tty_fd < 0`.
  - **Documentation:** documented with a `SAFETY:` comment.
  - **Region size:** minimal.

- `libc::dup`, `libc::dup2`, and `libc::close` at `terminal_lifecycle.rs:171-213`
  - **Invariant:** only valid open fds are duplicated/restored/closed, and each owned fd is closed once.
  - **Assessment:** mostly upheld. Error paths close owned fds, `Option` ownership prevents double close on drop, and `STDOUT_FILENO` is a valid process fd under normal process startup assumptions.
  - **Documentation:** individual `SAFETY:` comments exist for the main fd operations. The close-only unsafe calls are less explicitly documented but are inside the same ownership block.
  - **Region size:** reasonably minimized.

Test-only unsafe exists in `terminal_style.rs:277-297` for environment mutation under Rust 2024. It is guarded by a process-wide mutex in the test module, which is the right invariant for safe test execution.

### 6. Prioritized Next Steps

1. Make terminal preparation transactional so raw mode is always restored on partial setup failure.
2. Decide and implement the non-Unix captured-stdout behavior: support it properly on Windows or reject it clearly before touching terminal state.
3. Add `InputTableState::try_new` and move recoverable row-shape validation out of panics for embedders.
4. Tighten `input-table` CLI JSON validation around numeric ranges and unexpected cell types.
5. Relax choice hotkey modifier matching where extra modifiers should be harmless, and add tests for those terminal payloads.
6. Restore `cargo fmt --check` availability in the active stable toolchain; both checked manifests failed because `cargo-fmt` is not installed.

Quality gates run during review:

- `just test` from `biscuit-tui`: passed for `biscuit-tui` and `biscuit-tui-cli`.
- `just lint` from `biscuit-tui`: passed for both crates.
- `cargo fmt --check --manifest-path biscuit-tui/lib/Cargo.toml`: could not run because `cargo-fmt` is not installed for `stable-aarch64-apple-darwin`.
- `cargo fmt --check --manifest-path biscuit-tui/cli/Cargo.toml`: same rustfmt installation blocker.
