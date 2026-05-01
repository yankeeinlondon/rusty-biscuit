# Review 7 Implementation Plan

## Goal

Address every finding in `review-7.md` for the ChooseOne improvements feature and leave the `biscuit-tui` package area with passing focused tests, passing gated PTY verification, passing package tests, and no clippy warnings/errors.

This plan covers:

- PTY keyboard protocol verification failures in `biscuit-tui/cli/tests/keyboard_protocol.rs`.
- PTY shell completion verification failures in `biscuit-tui/cli/tests/completions_shell.rs`.
- Forced hotkey badge modes being overwritten by transient modifier events in `ChooseOne` and `ChooseMany`.
- Hotkey parsing silently truncating multi-character specs.

## Phase 1 - Fix Gated PTY Harnesses

### Scope

Repair the review's failing gated verification tests before changing production behavior so the remaining phases have trustworthy gates.

### Implementation Steps

1. Update `biscuit-tui/cli/tests/completions_shell.rs`.
   - Replace the process-id-only temp root with a unique directory per test invocation.
   - Use a deterministic helper that includes at least process id plus an atomic counter or timestamp-derived suffix.
   - Prefer `create_dir_all` for nested test directories, but avoid letting independent tests share the same `fpath` or `bash_completion.d` path.
   - Keep generated completion scripts isolated per shell session.

2. Update `biscuit-tui/cli/tests/keyboard_protocol.rs`.
   - Stop passing shell assignments such as `TERM=dumb ...` as the executable name to `expectrl::spawn`.
   - Spawn through a reliable shell wrapper, for example `sh -lc 'TERM=dumb exec /path/to/question ...'`, or use an expectrl command API that supports setting environment variables explicitly if available in the current dependency version.
   - Apply the same reliable spawn strategy to the normal command path so quoting and paths with spaces are handled consistently.
   - Keep command arguments semantically identical: `question choose-one --height 5 "Red" "Green" "Blue"`.

3. Stabilize PTY reads/writes only as needed.
   - After launch, drain initial prompt output before sending keyboard bytes.
   - When a test writes a chord that submits the prompt, do not require additional Enter unless the process is still running.
   - Preserve the intent of the tests: real PTY path, real `prepare_terminal`, and no synthetic bypass.

### Tests to Add or Adjust

- Add unit coverage for the unique tempdir helper if it is nontrivial.
- Keep all existing zsh/bash candidate assertions.
- Keep all existing keyboard-protocol assertions for bare Ctrl, chord fallback, and dumb terminal behavior.

### Focused Verification

Run:

```bash
RUN_PTY_TESTS=1 cargo test -p tui-chrome-cli --test keyboard_protocol -- --nocapture
RUN_SHELL_TESTS=1 cargo test -p tui-chrome-cli --test completions_shell -- --nocapture
```

Expected result: all enabled gated tests pass. If a local shell dependency is missing, the relevant test may skip only through existing explicit skip behavior; harness errors such as shared tempdirs, bad executable names, or PTY I/O panics must be fixed.

## Phase 2 - Make Hotkey Badge Overrides Authoritative

### Scope

Fix the public `--hotkey-badges never/always/ctrl/alt` behavior so explicit override modes cannot be changed by modifier-only press/release events or chord fallback.

### Implementation Steps

1. Update `biscuit-tui/lib/src/components/choose_one.rs`.
   - Add an override field, for example `hotkey_display_override: Option<HotkeyDisplayMode>`, to `ChooseOneState`.
   - Initialize it to `None`.
   - Change `with_hotkey_display(mode)` to set the override to `Some(mode)`, set the current display to `mode`, and clear any fallback deadline.
   - Add a helper such as `set_transient_hotkey_display` or `hotkey_display_is_forced` so event handling can skip modifier-only and chord-fallback mutations when an override is present.
   - Make `current_hotkey_display(now)` return the override immediately when present.
   - Preserve existing auto-mode behavior when the override is `None`.

2. Update `biscuit-tui/lib/src/components/choose_many.rs` with the same model.
   - Keep API semantics aligned with `ChooseOneState::with_hotkey_display`.
   - Avoid duplicating complex logic if a small private helper in each file is clearer and local.

3. Review CLI state construction.
   - `biscuit-tui/cli/src/commands/choose_one.rs` and `choose_many.rs` already call `with_hotkey_display` when `resolve_hotkey_badges` returns `Some`.
   - No CLI flag mapping should need to change unless naming/documentation is now inaccurate.

### Tests to Add or Adjust

Add library tests in both `choose_one.rs` and `choose_many.rs`:

- Forced `HotkeyDisplayMode::Hidden` remains hidden after Ctrl modifier press.
- Forced `HotkeyDisplayMode::Hidden` remains hidden after Alt or Ctrl chord fallback.
- Forced `HotkeyDisplayMode::CtrlHeld` remains `CtrlHeld` after Ctrl release.
- Forced `HotkeyDisplayMode::AltHeld` remains `AltHeld` after Alt release.
- Auto mode still transitions to `CtrlHeld` on Ctrl press and back to `Hidden` on release.
- Auto mode still briefly shows badges after Ctrl/Alt chord fallback and expires through `current_hotkey_display`.

If existing tests assert the old forced-mode behavior only at construction time, extend them rather than replacing the auto-mode tests.

### Focused Verification

Run:

```bash
cargo test -p tui-chrome hotkey_display
cargo test -p tui-chrome choose_one::tests::with_hotkey_display_forces_mode_and_clears_deadline
cargo test -p tui-chrome choose_many::tests
```

If exact test module filters differ, use `cargo test -p tui-chrome hotkey` and then the full package test command in the final phase.

## Phase 3 - Reject Invalid Multi-Character Hotkey Specs

### Scope

Make CLI hotkey parsing match the spec's single-character contract for `[CTRL+{char}]`, `[ALT+{char}]`, `[OPT+{char}]`, and object-source `hotkey` fields.

### Implementation Steps

1. Update `biscuit-tui/cli/src/choice_normalize.rs`.
   - Change `parse_hotkey_spec` so it returns `Some` only when the modifier suffix contains exactly one Unicode scalar value.
   - Return `None` for empty suffixes such as `ALT+`.
   - Return `None` for multi-character suffixes such as `CTRL+AB` or `CTRL+RED`.
   - Preserve case-insensitive modifier parsing and ASCII lowercase normalization for alphabetic hotkey characters.

2. Decide how bracketed prefixes fail.
   - Object hotkey fields already route invalid specs through `NormalizeError::InvalidHotkey`; keep that behavior.
   - For bracketed prefixes in labels, prefer treating a recognized modifier prefix with an invalid suffix as an invalid hotkey instead of silently leaving the text untouched.
   - If that requires changing `extract_hotkey` from `Option` to `Result`, keep the error local to `choice_normalize.rs` and ensure errors include the original option text.

3. Keep unsupported non-hotkey bracket text non-breaking.
   - Strings like `[note] Red` should remain ordinary labels unless existing tests specify otherwise.
   - Only `CTRL+`, `ALT+`, and `OPT+` prefixes with invalid suffixes should become invalid hotkeys.

### Tests to Add or Adjust

Add CLI normalization tests in `biscuit-tui/cli/src/choice_normalize.rs`:

- `parse_hotkey_spec("CTRL+AB") == None`.
- `parse_hotkey_spec("CTRL+RED") == None`.
- `parse_hotkey_spec("ALT+") == None`.
- `parse_hotkey_spec("OPT+") == None`.
- Object source hotkey `"CTRL+AB"` returns `NormalizeError::InvalidHotkey`.
- Object source hotkey `"ALT+"` returns `NormalizeError::InvalidHotkey`.
- Bracketed prefix `[CTRL+AB] Red` returns `NormalizeError::InvalidHotkey`.
- Bracketed prefix `[ALT+] Red` returns `NormalizeError::InvalidHotkey`.
- Valid single-character specs continue to parse: `CTRL+R`, `ctrl+x`, `ALT+1`, `OPT+-` if punctuation is intended to be accepted as a single character.

### Focused Verification

Run:

```bash
cargo test -p tui-chrome-cli choice_normalize
cargo test -p tui-chrome-cli hotkey
```

## Phase 4 - Full Package Verification and Lint Cleanup

### Scope

Prove the whole `biscuit-tui` package area is clean after phases 1 through 3.

### Required Verification Commands

Run from the repository root or the `biscuit-tui` area, using the area commands when available:

```bash
cargo test -p tui-chrome -p tui-chrome-cli
RUN_PTY_TESTS=1 cargo test -p tui-chrome-cli --test keyboard_protocol -- --nocapture
RUN_SHELL_TESTS=1 cargo test -p tui-chrome-cli --test completions_shell -- --nocapture
cargo clippy -p tui-chrome -p tui-chrome-cli --all-targets -- -D warnings
cargo test -p tui-chrome -p tui-chrome-cli
```

If the local `biscuit-tui/justfile` wraps these exact checks, the developer may use `just test` and `just lint`, but the gated PTY commands must still be run explicitly because default tests skip them without environment variables.

### Lint Expectations

- Fix all clippy warnings/errors in the `biscuit-tui` package area, even if they predate the current change.
- Do not suppress lints unless the suppression is narrowly justified and local.
- After lint fixes, rerun the full non-gated package tests to prove lint cleanup did not change behavior.

### Documentation Expectations

No README or component documentation update is expected unless implementation changes the public behavior beyond enforcing the already-documented review expectations. If docs currently say forced hotkey badge modes are lifetime-forcing, keep them and make the code match. If the parser behavior is documented as accepting a single character, no doc change is needed.

## Completion Criteria

The review is complete when:

- `review-7.md` finding 1 is resolved by passing gated PTY keyboard and shell completion tests.
- `review-7.md` finding 2 is resolved by forced hotkey badge mode tests for both choice components.
- `review-7.md` finding 3 is resolved by parser and normalization tests for invalid multi-character/empty hotkey specs.
- `cargo test -p tui-chrome -p tui-chrome-cli` passes.
- `cargo clippy -p tui-chrome -p tui-chrome-cli --all-targets -- -D warnings` passes.
- The final test rerun after lint cleanup passes.
