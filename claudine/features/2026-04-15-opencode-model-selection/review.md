# Review: OpenCode Model Resolution & AgentError Consistency

The implementation of the OpenCode Model Selection feature has been reviewed against the `spec.md` design.

## Summary of Findings

Overall, the core functionality is well-implemented and meets the primary goals of the design. The precedence logic for model resolution is correct, and the launch-time status logs provide good visibility. The error reporting has been significantly improved with better classification and suggestion parsing.

## Functional Gaps

- **Missing Markup in Error Summaries:** Both `AgentErrorReport::no_model_provided` and `AgentErrorReport::invalid_model` are missing requested Prose markup (e.g., `<yellow>`, `<blue>`, `<dim>`) for several tokens specified in the design.
- **Incomplete Error Body:** The footer text regarding aggregator model formats (`[aggregator]/[provider]/[model]`) is missing from both the "No Model Provided" and "Invalid Model Specified" error summaries.
- **Manual Bullet Points:** The "No Model Provided" error uses manual ASCII bullets (`•`) instead of the `UnorderedList` component mentioned in the spec.
- **Suggestions Rendering Location:** The `Did you mean:` suggestions are rendered *outside* the `BlockQuote` (after it), whereas the spec implied they should be part of the styled error surface.

## Broken or Incomplete Features

- **Classification Fallback:** In `error_report.rs`, if `classify_native_cli_error` returns `None`, the code still falls back to a generic `{provider} exited with error code {exit_code}` summary. While not explicitly broken, this generic line was intended to be replaced by the more specific `AgentError` blocks for model-related failures.

## Test Coverage

- **Strong Unit Tests:** `claudine/cli/src/commands/wrap/profile.rs` and `claudine/cli/src/output/error_report.rs` have comprehensive unit tests covering the resolution precedence, config file parsing, and error classification/suggestion parsing.
- **Integration Tests:** `claudine/cli/tests/wrap_commands.rs` includes high-fidelity integration tests using stub binaries, verifying pre-flight failures, CLI overrides, and environment variable precedence.
- **Missing Coverage:** There is no specific test for the `~/.config/opencode/config.json` precedence *interleaved* with environment variables in the integration suite (though it is covered in unit tests).

## Ergonomics & Performance

- **Performance:** The model resolution logic is performant; it reads the config file only once during the launch phase.
- **Ergonomics:** The `OpenCodeModelSource` enum is a great addition for carrying provenance info through to the error reporting layer.
- **Opportunity:** The resolution and flag injection logic in `claudine/cli/src/commands/wrap/mod.rs` (lines 776–810) is somewhat verbose and could be refactored into a helper method in `profile.rs` to keep the main launch loop cleaner.

## Recommended Actions

1. Update `AgentErrorReport::no_model_provided` and `AgentErrorReport::invalid_model` to include the missing markup and aggregator footer text.
2. Consider moving the `Did you mean:` suggestions inside the `BlockQuote` for a more cohesive "Agent Error" appearance.
3. Refactor the OpenCode-specific resolution block in `mod.rs` into the `OpencodeWrapper` profile to maintain better separation of concerns.
