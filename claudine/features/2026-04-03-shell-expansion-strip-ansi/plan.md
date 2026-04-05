# Implementation Plan: Shell Expansion ANSI Stripping

## Overview
This plan outlines the steps successfully taken to implement ANSI escape code stripping for the `::shell` directive expansion in the `darkmatter` library, based on the `design.md` specifications.

## Steps

### 1. Update `ShellExpansionOptions`
**File:** `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`
- Add `pub strip_ansi: bool` to the `ShellExpansionOptions` struct.
- Update the `Clone` and `Debug` implementations to include the new field.
- Set `strip_ansi: true` in the `Default` implementation to ensure secure and clean defaults.

### 2. Update `ComposeOptions`
**File:** `darkmatter/lib/src/markdown/compose/types.rs`
- Add `pub shell_strip_ansi: bool` to the `ComposeOptions` struct.
- Initialize `shell_strip_ansi: true` within `new_with_context()`.
- Expose the builder method `with_shell_strip_ansi(mut self, enabled: bool) -> Self`.
- Update the internal `shell_options(&self)` method to pass `self.shell_strip_ansi` through to the generated `ShellExpansionOptions`.

### 3. Implement Proactive and Reactive Stripping
**File:** `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs`
- **Proactive (`NO_COLOR=1`):** Inside `execute_command`, inject the `NO_COLOR=1` environment variable into the `std::process::Command` builder if `shell_opts.strip_ansi` is true. This politely asks tools to disable coloring at the source.
- **Reactive (Regex Stripping):** Inside `execute_command`, after collecting and combining stdout and stderr, conditionally pass the output through `biscuit_terminal::prelude::strip_escape_codes(output)` if `shell_opts.strip_ansi` is true. Ensure this is applied to both the success branch and the `ExecutionFailed` error branch streams.

### 4. Testing & Validation
**File:** `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs`
- Add unit tests verifying:
  - `execute_command_strips_ansi_by_default`: Ensures raw ANSI strings (e.g., `\x1b[31mhello\x1b[0m`) emitted by commands like `echo` are successfully stripped to plain text.
  - `execute_command_keeps_ansi_when_opt_out`: Ensures ANSI strings are preserved when `strip_ansi: false` is explicitly provided.
  - `execute_command_sets_no_color_env`: Executes `env` and verifies that `NO_COLOR=1` is present in the captured output.
- Run `just test` across the `darkmatter` package area to ensure both library and CLI tests pass without regression.

## Status
✅ Implemented and verified via `cargo test`.
