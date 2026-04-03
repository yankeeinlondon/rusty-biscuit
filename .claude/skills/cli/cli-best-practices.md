## Important Standards

- agree on output CLI flags: `--json`, `--plain` are mandatory but you may want `--csv`, `--yaml`, `--toml` switches depending on your needs
    - The "default" output should be terminal optimized output
        - This means that you'll be using escape codes to provide nice colors, bold and dim text, italics, etc.
        - If you're designing for the terminal start with the `Prose` component from `biscuit-terminal` as it provides a really ergonomic surface for creating a lot of terminal output
        - Reach for the `Table`, `TwoColumns`, `BlockQuote`, `OrderedList`, and `UnorderedList` to help you design structure in the terminal
    - most terminal apps will strip escape codes out when you cut and paste from them but if you're piping the output the CLI to something else you'll want to have the `--plain` CLI flag to opt-out of any escape codes being used in the output.
- STDOUT vs STDERR
    - use these rules for STDOUT and STDERR
    - The basic rule is:
        - STDOUT is for the data
        - STDERR is for metadata
    - when the output format is `--json` everything typically goes to STDOUT and STDOUT **must** be valid JSON; in rare cases you might output additional metadata to STDERR. Very rare.
- Verbosity
    - use these rules for verbosity:
        - use `--verbose` / `-v` to indicate that MORE output is appropriate
        - use `--quiet` to indicate a minimal amount of metadata output
        - use `--silent` to send NOTHING to STDOUT, and only error messages or context to STDERR
        - use `--no-output` to send NOTHING to STDOUT and STDERR
            - obviously if a user adds both `--json` and `--no-output` to their prompt this is an impossible combination and should result in visible error being shown to the user with enough description to make it clear to the user what they did wrong.
- Help System
      - `--help` / `-h` should be a globally registered switch that will bring up the help system
      - the help system should adapt to the depth of the CLI command structure to provide only the details of the CLI at that depth level
        - a `help` command is also fine to include but it should not show up on the help system
- Shell Completions
    - adding shell completions is always a requirement
    - we want to provide dynamic completions; they are both a lot more useful and easier for the user to include in their shell initialization
    - Completions can be accessed by:
        - if the CLI is very simple then you can add a `--completions <shell>` switch
        - if the CLI has a command based system then it's better to just add a `completions <shell>` command instead of the switch
        - either way when a user calls completions with `--help` instead of the shell name or _in addition_ to the shell name you should:
            - add a new "<b>Examples:</b>" section to the help system and show example of how to add to the shell
            - this requirement also means that the `<shell>` variable you're expecting must be made optional so that `--completions --help` is valid
            - as long as you plan for this, it's easy to address
    - use the `clap_complete` crate 
    - and make sure to include the `derive` and `unstable-ext` features on `clap`
    - Always try to identify parameters in the CLI which can be:
        - auto-completed by an enumerated list of values and make sure the enumerated are provided to the shell completions logic
        - if the parameter is a file path then it should auto-complete to those file types which are valid for the parameter
        - if the parameter is a file reference which uses the `FileReference` struct from `biscuit-file` to resolve file paths then use that as a resolver for valid file paths with the `@` and `!` based file paths
            - the `@` file references are "magic paths" and source from multiple paths when in a repo:
                - `@` can equal the repo's root directory
                - `@` can equal the user's home directory
                - in more advanced cases, `@` can be configured to represent other paths
            - the `!` file references are paths which can be used in monorepos and resolve from the root of the current package.

## How to structure code in a Clap based CLI

A monolithic `main.rs` containing CLI argument parsing, command execution, and formatting logic quickly becomes a maintenance and testing bottleneck (often scaling to thousands of lines). A modular, decoupled approach is strongly recommended.


### Module Breakdown

- **`args.rs`**: Define your Clap structs (`Parser`, `Subcommand`, `Args`). This file should exclusively contain the CLI interface definition, flag attributes, shell completion logic, and help strings.
- **`commands.rs` (or `handlers.rs`)**: Implement the execution logic for each subcommand. These functions take the parsed arguments and act as the glue between the CLI and the core library.
- **`output.rs` (or `format.rs`)**: House data structures and logic dedicated to formatting output (e.g., JSON serialization structs, table generation, text coloring rules).
- **`main.rs`**: Acts as a thin wrapper. It should only handle global environment initializations (e.g., `color_eyre` for error reporting, logging/tracing setup, dynamic completions) and then parse arguments and route them to the appropriate handler in `commands.rs`.
    

### Library Interaction

The CLI crate (typically located in the `./cli` directory off of a package area's root) should act as a frontend relying on a corresponding library (`./lib`) crate for core business logic.

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
