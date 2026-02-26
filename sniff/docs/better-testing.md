# Better Testing for sniff-cli

This document outlines suggested improvements for the `sniff-cli` package based on the project's [CLI Testing Best Practices](../../biscuit-terminal/docs/cli-testing-best-practices.md).

## 1. Modularize Code Structure

Currently, `sniff-cli/src/main.rs` is monolithic (over 800 lines) and handles argument parsing, subcommand execution, business logic, and unit testing.

### Recommendations:

- **Extract `args.rs`**: Move `Cli`, `Commands`, `DocsFilter`, `ServiceStateArg`, and their implementations to a new `src/args.rs`.
- **Extract `commands.rs`**: Move the subcommand handlers (`handle_remote_url`, `handle_shorthand`, `enrich_result_dependencies`, `fetch_readme`, `resolve_remote_name`) and the `run()` function logic to `src/commands.rs`.
- **Thin `main.rs`**: Reduce `main.rs` to just global initialization (if any) and the `main` function that routes to `commands.rs`.
- **Move Business Logic to Library**: Functions like `is_major_update` and `is_owner_repo_shorthand` should be moved to the `sniff` library (e.g., `sniff/lib/src/package/mod.rs` or a new utility module) where they can be tested more naturally.

## 2. Enhance Testing Strategy

The current tests in `tests/cli.rs` rely heavily on string containment assertions (`predicate::str::contains`), which are brittle and don't verify the full visual structure of the output.

### Recommendations:

- **Adopt `insta` for Snapshot Testing**:
    - Use `insta::assert_snapshot!` for all text, table, and JSON outputs.
    - This ensures that any intentional change in formatting is explicitly reviewed and captured.
    - Example:

      ```rust
      #[test]
      fn test_os_subcommand_snapshot() {
          let mut cmd = cargo_bin_cmd!("sniff");
          let output = cmd.arg("os").output().unwrap();
          let stdout = String::from_utf8(output.stdout).unwrap();
          insta::assert_snapshot!(stdout);
      }
      ```

- **Introduce `expectrl` for TTY Testing**:
    - `sniff` output likely changes based on terminal capabilities (colors, layout).
    - Use `expectrl` to verify behavior in a pseudo-terminal (PTY) to ensure colors and interactive elements work as expected in CI.
- **Property-based Testing with `proptest`**:
    - Use `proptest` for the version parsing and major update detection logic (`is_major_update`) to handle edge cases in version strings.

## 3. Modularize Unit Tests

The 400+ lines of unit tests at the bottom of `main.rs` should be distributed to the modules they test.

### Recommendations:

- Move argument parsing tests to `src/args.rs`.
- Move helper function tests to where the helpers reside (ideally the library).
- This keeps the codebase cleaner and helps developers find relevant tests more quickly.

## 4. Summary of Proposed File Structure

```text
sniff/cli/src/
├── args.rs       # Cli, Commands, DocsFilter, ServiceStateArg + parsing tests
├── commands.rs   # run(), handler functions, enrichment logic
├── main.rs       # Thin entry point
├── output/       # (Keep existing structure)
└── images/       # (Keep existing structure)
```
