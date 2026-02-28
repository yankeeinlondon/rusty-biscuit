# Darkmatter CLI Review against Best Practices

This document contains a review of the `darkmatter-cli` package based on the CLI code structure and testing best practices outlined in `biscuit-terminal/docs/cli-testing-best-practices.md`.

## 1. Code Structure

### Current State
The `darkmatter-cli` codebase currently violates the recommended modular approach:
- **Monolithic `main.rs`**: The `main.rs` file is extremely large (~41KB) and contains a mix of command routing, command execution (`run_read`, `run_hash`, `run_compose`), and output formatting logic (`render_terminal_output`, `html_artifact`, `json_artifact`, `print_toc_tree`, `print_delta`).
- **Misplaced `args.rs` Logic**: The CLI argument definitions (`Cli`, `Command`, `OutputFormat`) are currently defined inside a `cli` module within `lib.rs`, rather than a dedicated `args.rs`.

### Recommendations
To align with best practices, the code should be heavily refactored:
- **Extract `args.rs`**: Move the `Cli`, `Command`, `OutputFormat`, and completion functions from `lib.rs` into a new `args.rs` file.
- **Extract `commands.rs`**: Move the execution functions (`run_subcommand`, `run_read`, `run_compose`, `run_get`, `run_hash`, etc.) out of `main.rs` and into `commands.rs`.
- **Extract `output.rs`**: Move the output artifact generation and formatting logic (`markdown_artifact`, `html_artifact`, `json_artifact`, `print_toc_tree`, `print_delta`, `render_terminal_output`) into `output.rs`.
- **Thin `main.rs`**: Refactor `main.rs` so that it only initializes the global environment (like tracing and color_eyre), parses the arguments from `args.rs`, and routes execution to `commands.rs`.

## 2. Library Interaction

### Current State
While the CLI depends on the `darkmatter` library for core markdown processing, `main.rs` still holds significant logic that belongs in the library or separate modules, such as:
- Hashing logic (`hash_single`, `hash_frontmatter`, `hash_body`).
- File collection (`collect_markdown_files`, `collect_markdown_files_recursive`).

### Recommendations
- **Delegate Business Logic**: Move the core hashing logic and file discovery algorithms to the `darkmatter` library crate. The CLI should strictly handle I/O, argument parsing, and mapping library output to the terminal or files.

## 3. Testing Stack and Techniques

### Current State
The CLI uses `assert_cmd`, `predicates`, and `tempfile` for integration testing in `tests/cli.rs`. However, it misses several critical testing crates and techniques recommended by the best practices:
- **No `insta`**: Despite generating complex visual outputs (like formatted markdown, toc trees, and diffs), there are no snapshot tests.
- **No `expectrl`**: There are no PTY tests to simulate real terminal environments and validate TTY-specific code paths.
- **No Unit Tests**: Internal CLI helper functions and formatting logic are currently inaccessible for unit testing because they are not properly exposed in modules.

### Recommendations
- **Implement Snapshot Testing with `insta`**: Use `insta` to capture and track the visual outputs of commands like `md read` and `md diff`. Be sure to force terminal conditions (e.g., `FORCE_COLOR=1` or `NO_COLOR=1`) to ensure deterministic tests.
- **Implement TTY Testing with `expectrl`**: Add `expectrl` to test terminal capability detection and interactive behaviors (e.g., when outputting to a TTY vs. piping to a file).
- **Unit Testing**: After breaking `main.rs` down into `args.rs`, `commands.rs`, and `output.rs`, add unit tests to these modules to independently test input parsers and data formatting.
- **Property-based Testing**: Consider using `proptest` for parsing theme names or handling edge-case CLI inputs.
