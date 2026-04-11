# Claudine CLI Argument Parsing Review

This document reviews the current implementation of CLI argument parsing and error reporting in `claudine-cli`, identifying bugs and suggesting structural improvements.

## 1. Current Implementation Overview

The current parsing logic is split into two layers:
1.  **Clap-based declarative parsing**: Defined in `Cli` and `WrapperArgs` structs using `clap`.
2.  **Manual extraction**: `extract_wrapper_flags_from_passthrough` in `commands/wrap/mod.rs` manually scans the `passthrough` vector for Claudine-specific flags that may have been placed after `--` or otherwise ended up in the passthrough bucket.

Execution is handled by `run_child` (and variants), with some basic API error formatting in `output.rs`.

## 2. Identified Bugs & Limitations

### 2.1 Parser Strictness Prevents Proxying
The most significant bug is that `claudine <provider> --native-flag` fails if `--native-flag` is not explicitly defined in `WrapperArgs`. `clap` defaults to erroring on unknown arguments. This forces users to use the `--` separator (e.g., `claudine gemini -- --native-flag`), which is non-intuitive and deviates from the goal of seamless wrapping.

### 2.2 Dual-Source Truth for Flags
Claudine flags (like `--yolo`, `--interactive`, `--verbose`) are defined in two places:
*   As fields in `WrapperArgs` (for `clap` to parse).
*   As string literals in `extract_wrapper_flags_from_passthrough`.

This duplication is prone to drift and makes the parsing logic harder to reason about. It exists because `clap` is sometimes "too smart" (consuming flags it knows) and sometimes "too dumb" (putting everything after `--` into passthrough).

### 2.3 Global Flag Ambiguity
Global flags in `Cli` (like `--verbose` and `--plain`) are always consumed by the root parser. If an underlying agent also has a `--verbose` flag, there is no way to pass it through without using the `--` separator.

### 2.4 Missing Agent Error Wrapping
While Claudine attempts to format "API Error" JSON from `stderr`, it does not currently wrap general agent CLI failures. If an agent fails with "unrecognized argument" or a native error, Claudine simply passes the exit code through and lets the raw `stderr` flow to the terminal. It fails to "wrap the error with our own reporting structure" as intended.

## 3. Recommended Improvements

### 3.1 Lenient Wrapper Parsing
The wrapper subcommands should be configured to **ignore unknown arguments** and collect them into the `passthrough` bucket automatically.

**Suggestion:**
*   Use `#[command(ignore_errors = true)]` or `Command::ignore_errors(true)` for the wrapper subcommands.
*   Alternatively, use a two-pass approach where `claudine` first parses its own global flags and the subcommand name, then treats the remainder of `argv` as potentially belonging to the agent, minus known Claudine-specific wrapper flags.

### 3.2 Unified Flag Management
Consolidate flag definitions. The `extract_wrapper_flags_from_passthrough` logic should be the primary way wrapper-specific flags are handled, or `WrapperArgs` should be the source of truth, but not both with manual string matching.

### 3.3 Structured Agent Error Reporting
When a wrapped agent exits with a non-zero code, Claudine should intercept this and provide a "Claudine Wrapped Error" report.

**Suggestion:**
*   Implement an `AgentError` reporter that categorizes the failure:
    *   **Configuration Error**: Environment or Claudine-injected flags caused the failure.
    *   **Agent Native Error**: The agent didn't recognize a flag or had a local issue.
    *   **API/Remote Error**: Structured JSON errors (already partially handled).
*   Use `biscuit-terminal`'s `BlockQuote` and `Prose` to render a clear, styled error box that clearly differentiates between Claudine's wrapping context and the agent's internal error.

### 3.4 Improved Stderr Interception
The `try_format_api_error` logic should be expanded to recognize more patterns, including common CLI errors (e.g., "unknown flag", "missing required argument") and reformat them to match Claudine's aesthetic.

## 4. Proposed Refactor Path

1.  **Refactor `main.rs`** to use `try_get_matches` instead of `parse()`, allowing manual intervention when unknown flags are encountered.
2.  **Update `WrapperArgs`** to use a custom `FromArgMatches` implementation or a more flexible `clap` configuration that avoids the "must use `--`" requirement.
3.  **Enhance `exec.rs`** to return a structured `ProcessResult` that includes categorized error information when `exit_code != 0`.
4.  **Create `claudine/cli/src/output/error_report.rs`** to handle the rich rendering of these wrapped failures.
