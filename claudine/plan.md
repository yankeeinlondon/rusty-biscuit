---
phases: 4
created: 2026-05-04
start_phase: 1
---

# Execution Plan - Terminal Escape Code Bleed Fix

The "escape code bleed" occurs when terminal background color queries (OSC 11) are sent to `stdout` but the response is not consumed from `stdin`, typically in non-interactive sessions where `stdout` is a terminal but `stdin` is redirected, or when redundant queries are made and time out.

## User Review Required

> [!IMPORTANT]
> This plan involves modifying `biscuit-terminal`, a core library in the monorepo. While it fixes the reported bleed, it also introduces caching for terminal queries which may slightly change behavior if a user changes their terminal theme *during* a long-running session (though this is standard for most CLI tools).

- **Critical Change:** `query_osc_actual` will now require both `stdin` and `stdout` to be terminals before attempting a query.
- **Critical Change:** `Terminal::color_mode()` will now be cached globally to prevent redundant TTY queries.

- [ ] Confirm if caching `color_mode` for the duration of the process is acceptable.
- [ ] Confirm if requiring `stdin` to be a terminal for OSC queries is acceptable (this is the most robust way to prevent the bleed).

## Proposed Changes

### biscuit-terminal

#### [lib]
- **`src/discovery/detection.rs`**: 
    - Add a global `OnceLock<ColorMode>` to cache `color_mode()`.
    - Update `color_mode()` to use this cache.
- **`src/discovery/osc_queries.rs`**:
    - Update `query_osc_actual` to check `std::io::stdin().is_terminal()` in addition to `is_tty()` (which checks `stdout`).
    - Add a "drain" mechanism in `query_osc_actual` to consume any trailing bytes after a successful parse or timeout to prevent leftovers.
- **`src/terminal.rs`**:
    - Add `color_mode: ColorMode` field to `Terminal` struct.
    - Update `TerminalBuilder` and `new_terminal()` to detect and store `color_mode` during initialization.
    - Update `Terminal::new_optimistic()` to default to `ColorMode::Dark`.
- **Components (`HorizontalRule`, `Prose`, `Status`, etc.)**:
    - Refactor to use `term.color_mode` (instance field) instead of `Terminal::color_mode()` (static method) to avoid redundant (even if cached) lookups.

### claudine

#### [cli]
- **`src/log.rs`**: Ensure `terminal()` and `optimistic_terminal()` correctly propagate or set the `color_mode`.
- **`src/commands/wrap/mod.rs`**: Verify that wrapping logic doesn't interfere with TTY detection.

## Verification Plan

### Automated Tests
- Create a reproduction test in `biscuit-terminal/lib/tests/osc_bleed_repro.rs` that:
    - Spawns a process with `stdout` as a PTY and `stdin` as a pipe/file.
    - Calls `bg_color()`.
    - Verifies that no escape codes are written to `stdout` if `stdin` is not a terminal.
    - Verifies that if they are written, they are consumed correctly if `stdin` IS a terminal.
- Add unit tests for `TerminalBuilder` to ensure `color_mode` overrides work.
- Add unit tests for `color_mode()` caching.

### Manual Verification
- Run `claudine` in a simulated non-interactive environment (e.g., `claudine wrap ... < /dev/null`) and check the logs for `^[]11;...`.
- Verify icons and styling still work correctly in normal interactive use.

---

## Phase 1: Research & Reproduction (Parallelizable: No)
Confirm the root cause and establish a baseline for the fix.

1. Create `biscuit-terminal/lib/tests/reproduce_bleed.rs` to simulate the non-interactive TTY environment.
2. Run the reproduction test and confirm it fails (observes the bleed).
3. Trace `Terminal::color_mode()` calls in `claudine` to quantify redundancy.

## Phase 2: Core Library Fixes in `biscuit-terminal` (Parallelizable: No)
Apply the fixes to the discovery and detection logic.

1. **Robust Queries**: Modify `biscuit-terminal/lib/src/discovery/osc_queries.rs` to check `stdin.is_terminal()` and improve buffer draining.
2. **Global Caching**: Implement `OnceLock` caching for `color_mode()` in `biscuit-terminal/lib/src/discovery/detection.rs`.
3. **Struct Update**: Add `color_mode` field to `Terminal` and `TerminalBuilder` in `biscuit-terminal/lib/src/terminal.rs`.
4. **Validation**: Run `biscuit-terminal` unit tests and the reproduction test from Phase 1.

## Phase 3: Component Refactoring (Parallelizable: Yes)
Update components to use instance-based color mode.

1. Update `HorizontalRule` in `biscuit-terminal/lib/src/components/horizontal_rule.rs`.
2. Update `Prose` in `biscuit-terminal/lib/src/components/prose.rs`.
3. Update `Status` and `StatusBlock` in `biscuit-terminal/lib/src/components/status.rs` and `status_block.rs`.
4. Update `Mermaid` and `GraphExpression` in `biscuit-terminal/lib/src/components/mermaid.rs` and `graph_expression.rs`.
5. **Validation**: Run full `biscuit-terminal` test suite.

## Phase 4: `claudine` Integration & Final Validation (Parallelizable: No)
Ensure the fix works at the application level.

1. Update `claudine/cli/src/log.rs` to leverage the updated `Terminal` struct.
2. Final manual verification with `claudine wrap` in non-interactive mode.
3. Perform a full monorepo lint and test run.
4. Update `README.md` or `docs/` if any terminal detection defaults changed significantly.
