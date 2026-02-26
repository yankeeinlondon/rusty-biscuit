# CLI Code Structure and Testing Best Practices

## How to structure code in a Clap based CLI

A monolithic `main.rs` containing CLI argument parsing, command execution, and formatting logic quickly becomes a maintenance and testing bottleneck (often scaling to thousands of lines). A modular, decoupled approach is strongly recommended.

### Module Breakdown

- **`args.rs`**: Define your Clap structs (`Parser`, `Subcommand`, `Args`). This file should exclusively contain the CLI interface definition, flag attributes, shell completion logic, and help strings.
- **`commands.rs` (or `handlers.rs`)**: Implement the execution logic for each subcommand. These functions take the parsed arguments and act as the glue between the CLI and the core library.
- **`output.rs` (or `format.rs`)**: House data structures and logic dedicated to formatting output (e.g., JSON serialization structs, table generation, text coloring rules).
- **`main.rs`**: Acts as a thin wrapper. It should only handle global environment initializations (e.g., `color_eyre` for error reporting, logging/tracing setup, dynamic completions) and then parse arguments and route them to the appropriate handler in `commands.rs`.

### Library Interaction
The CLI crate (`cli/`) should act as a frontend relying on a corresponding library (`lib/`) crate for core business logic.

- **Avoid Business Logic in CLI**: The CLI should primarily handle I/O—parsing user input, mapping it to library types, and formatting the output. Complex state manipulation, data fetching, and core computations should live in the library.
- **Testable Types**: By keeping core logic in the library, those functions can be unit-tested natively without needing to mock process arguments or standard streams.

## Effective Testing of a CLI Stack

### Testing Tech Stack
A robust CLI testing strategy leverages the following crates:

- **`assert_cmd`**: The standard for CLI integration testing. It allows you to reliably spawn your compiled binary, pass arguments, and assert on stdout, stderr, and exit codes.
- **`insta`**: Essential for snapshot testing. Complex visual outputs (tables, charts, Mermaid diagrams, ANSI escape sequences) are brittle to test with simple string containment assertions. `insta` captures the exact output structure and tracks visual regressions efficiently.
- **`expectrl`** (or `ptyprocess`): Allows you to spawn a pseudo-terminal (PTY) during tests. Crucial for verifying features that behave differently in a true TTY (like color support, interactive prompts, or terminal dimension queries) inside headless CI environments.
- **`proptest`**: Excellent for property-based testing of custom parsers to ensure they handle adversarial edge cases and invalid data without panicking.
- **`cargo-nextest`**: A fast, reliable test runner. It provides features like retries for flaky tests, which is highly useful when dealing with TTY constraints or timeouts in CI environments.

### Testing Types and Techniques

#### 1. Unit Testing

- **What to test:** Internal CLI helper functions, data serialization (e.g., ensuring a metadata struct serializes to the correct JSON schema), and input parsers (e.g., parsing width strings or extracting hex colors).
- **Technique:** Write standard `#[test]` blocks directly inside `args.rs`, `commands.rs`, and `output.rs`. Because the logic is decoupled from `main`, these functions can be tested quickly without spawning the full binary.

#### 2. Integration Testing

- **What to test:** End-to-end command execution, exit codes, basic structural output assertions, and Clap's argument parsing (e.g., verifying that missing required arguments fail gracefully and print the correct help text).
- **Technique:** Use `tests/integration_test.rs` alongside `assert_cmd::Command`. Ensure the binary executes the full path and handles combinations of flags correctly.

#### 3. Snapshot Testing

- **What to test:** Visual outputs, complex formatting, tabular layouts, and ANSI escape code generation.
- **Technique:** Pipe the CLI output into `insta::assert_snapshot!`. When the output format is intentionally changed, use `cargo insta review` to accept the new snapshot. Ensure you run these tests with forced terminal conditions (e.g., setting `NO_COLOR=1` or `FORCE_COLOR=1` in the environment) to guarantee deterministic results across different machines.

#### 4. Environment and TTY Testing

- **What to test:** Terminal capability detection (like width/height queries, OSC support), interactive components, and anything that conditionally relies on `std::io::stdout().is_terminal()`.
- **Technique:** Use `expectrl` to spawn the binary inside a PTY instance. This simulates a real user terminal session, allowing tests to validate TTY-specific code paths that would otherwise be skipped in standard integration tests.
