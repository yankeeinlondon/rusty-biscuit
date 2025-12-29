# Tester Sub-Agent Quick Reference (Rust)

Quick copy-paste templates for invoking the tester sub-agent for Rust projects.

## Basic Invocation Template

```typescript
Task({
    subagent_type: "general-purpose",
    description: "Create unit tests for [FEATURE_NAME]",
    model: "claude-sonnet-4-5-20250929",
    prompt: `You are the tester sub-agent. Your task is to create comprehensive tests for a Rust feature.

## Context
Read and follow the instructions in: agents/tester-agent.md
Activate the rust-testing skill for Rust testing patterns and best practices.

## Input Information
- **Feature Log File:** .ai/features/[YYYY-MM-DD].[feature-name]
- **Test Scope:** [module path or test pattern]
- **Feature Summary:** [1-2 sentence description]

## Your Task
1. Read the feature log file at the path specified above
2. Activate the rust-testing skill
3. Follow the complete workflow documented in agents/tester-agent.md
4. Create comprehensive tests following Rust conventions:
   - Unit tests: In \`#[cfg(test)] mod tests\` within the source file
   - Integration tests: In \`tests/\` directory
5. Update the feature log file with test details
6. Return a concise summary (see tester-agent.md for format)

## Rust Testing Conventions
- Use \`#[test]\` attribute for test functions
- Use \`#[cfg(test)]\` to conditionally compile test modules
- Use \`use super::*;\` to access private functions in unit tests
- Prefer \`assert_eq!\`, \`assert_ne!\`, \`assert!\` macros
- Use \`#[should_panic(expected = "message")]\` for panic tests
- Consider property-based tests with proptest for complex logic

Execute the workflow now and report back with your summary.`
})
```

## Example: New Feature Tests

```typescript
Task({
    subagent_type: "general-purpose",
    description: "Create unit tests for config parser",
    model: "claude-sonnet-4-5-20250929",
    prompt: `You are the tester sub-agent. Your task is to create comprehensive tests for a Rust feature.

## Context
Read and follow the instructions in: agents/tester-agent.md
Activate the rust-testing skill for Rust testing patterns and best practices.

## Input Information
- **Feature Log File:** .ai/features/2025-11-13.config-parser
- **Test Scope:** src/config/*.rs and tests/config_*.rs
- **Feature Summary:** Add TOML configuration parsing with validation, default values, and environment variable overrides.

## Your Task
1. Read the feature log file at the path specified above
2. Activate the rust-testing skill
3. Follow the complete workflow documented in agents/tester-agent.md
4. Create comprehensive tests following Rust conventions:
   - Unit tests: In \`#[cfg(test)] mod tests\` within src/config/parser.rs
   - Integration tests: In \`tests/config_integration.rs\`
5. Update the feature log file with test details
6. Return a concise summary (see tester-agent.md for format)

## Rust Testing Conventions
- Use \`#[test]\` attribute for test functions
- Use \`#[cfg(test)]\` to conditionally compile test modules
- Use \`use super::*;\` to access private functions in unit tests
- Prefer \`assert_eq!\`, \`assert_ne!\`, \`assert!\` macros
- Use \`#[should_panic(expected = "message")]\` for panic tests
- Consider property-based tests with proptest for complex logic

Execute the workflow now and report back with your summary.`
})
```

## Example: Bug Fix Tests

```typescript
Task({
    subagent_type: "general-purpose",
    description: "Create unit tests for path handling fix",
    model: "claude-sonnet-4-5-20250929",
    prompt: `You are the tester sub-agent. Your task is to create comprehensive tests for a Rust bug fix.

## Context
Read and follow the instructions in: agents/tester-agent.md
Activate the rust-testing skill for Rust testing patterns and best practices.

## Input Information
- **Feature Log File:** .ai/features/2025-11-13.path-handling-fix
- **Test Scope:** src/utils/path.rs
- **Feature Summary:** Fix path canonicalization to handle symlinks and relative paths correctly on all platforms.

## Your Task
1. Read the feature log file at the path specified above
2. Activate the rust-testing skill
3. Follow the complete workflow documented in agents/tester-agent.md
4. Create comprehensive tests following Rust conventions:
   - Unit tests: In \`#[cfg(test)] mod tests\` within src/utils/path.rs
   - Add regression tests that reproduce the original bug
5. Update the feature log file with test details
6. Return a concise summary (see tester-agent.md for format)

## Rust Testing Conventions
- Use \`#[test]\` attribute for test functions
- Use \`#[cfg(test)]\` to conditionally compile test modules
- Use \`use super::*;\` to access private functions in unit tests
- Prefer \`assert_eq!\`, \`assert_ne!\`, \`assert!\` macros
- Use \`#[should_panic(expected = "message")]\` for panic tests
- Consider tempfile crate for filesystem tests

Execute the workflow now and report back with your summary.`
})
```

## Example: Enhancement with Property Tests

```typescript
Task({
    subagent_type: "general-purpose",
    description: "Create tests for serialization enhancement",
    model: "claude-sonnet-4-5-20250929",
    prompt: `You are the tester sub-agent. Your task is to create comprehensive tests for a Rust enhancement.

## Context
Read and follow the instructions in: agents/tester-agent.md
Activate the rust-testing skill for Rust testing patterns and best practices.

## Input Information
- **Feature Log File:** .ai/features/2025-11-13.serialization-enhancement
- **Test Scope:** src/serde/*.rs and tests/serde_*.rs
- **Feature Summary:** Enhance serialization to support custom formats with streaming and zero-copy deserialization.

## Your Task
1. Read the feature log file at the path specified above
2. Activate the rust-testing skill
3. Follow the complete workflow documented in agents/tester-agent.md
4. Create comprehensive tests following Rust conventions:
   - Unit tests: In \`#[cfg(test)] mod tests\` within source files
   - Integration tests: In \`tests/serde_integration.rs\`
   - Property tests: Using proptest for roundtrip invariants
5. Update the feature log file with test details
6. Return a concise summary (see tester-agent.md for format)

## Rust Testing Conventions
- Use \`#[test]\` attribute for test functions
- Use proptest for property-based testing (roundtrip: serialize then deserialize equals original)
- Use \`#[cfg(test)]\` to conditionally compile test modules
- Consider snapshot tests with insta for complex output verification

Execute the workflow now and report back with your summary.`
})
```

## Parameters to Customize

When invoking, replace these placeholders:

| Placeholder | Description | Example |
|------------|-------------|---------|
| `[FEATURE_NAME]` | Short name for description | `async-runtime` |
| `[YYYY-MM-DD]` | Current date | `2025-11-13` |
| `[feature-name]` | Kebab-case feature name | `async-runtime` |
| `[module path or test pattern]` | Rust module path or file pattern | `src/runtime/*.rs` |
| `[1-2 sentence description]` | Brief feature summary | `Add async task spawning with priority scheduling.` |

## After Sub-Agent Returns

Expected response format:

```markdown
## Tests Created

**Unit Tests:**
- src/feature/mod.rs: X tests in `#[cfg(test)] mod tests`
- src/feature/parser.rs: Y tests in `#[cfg(test)] mod tests`

**Integration Tests:**
- tests/feature_integration.rs (Z tests)

**Property Tests:**
- tests/feature_proptest.rs (N property tests)

**Total Tests:** X unit + Y integration + Z property tests

**Test Status:** All tests currently FAILING (as expected - no implementation yet)

**Test Coverage:**
- Happy path: X tests
- Edge cases: Y tests
- Error handling: Z tests
- Property invariants: N tests

**Feature Log Updated:** .ai/features/YYYY-MM-DD.feature-name
```

### What to Do Next

1. Review the summary
2. Verify the feature log was updated (read it if needed)
3. Run `cargo test --no-run` to verify tests compile
4. Update the feature log to mark Step 3 complete
5. Move to Step 4 (Implementation)

## Common Test Patterns

| Test Type | Location | Command |
|-----------|----------|---------|
| Unit tests | `src/**/*.rs` (inline `#[cfg(test)]`) | `cargo test --lib` |
| Integration tests | `tests/*.rs` | `cargo test --test '*'` |
| Doc tests | `src/**/*.rs` (doc comments) | `cargo test --doc` |
| Specific test | Any | `cargo test test_name` |
| Single integration file | `tests/foo.rs` | `cargo test --test foo` |
| All tests | All locations | `cargo nextest run` |

## Cargo Nextest Commands

Prefer nextest for better output and parallelism:

```bash
cargo nextest run                     # Run all tests
cargo nextest run -E 'test(parse)'    # Filter by name expression
cargo nextest run --no-fail-fast      # Continue on failures
cargo nextest run --retries 2         # Retry flaky tests
cargo nextest list                    # List all tests
```

## Test Crates to Consider

| Crate | Use Case | Add to Cargo.toml |
|-------|----------|-------------------|
| `proptest` | Property-based testing | `proptest = "1"` |
| `rstest` | Fixtures and parameterized tests | `rstest = "0.18"` |
| `pretty_assertions` | Better diff output | `pretty_assertions = "1"` |
| `insta` | Snapshot testing | `insta = "1"` |
| `tempfile` | Temporary files/dirs | `tempfile = "3"` |
| `mockall` | Mock trait implementations | `mockall = "0.13"` |
| `test-case` | Data-driven tests | `test-case = "3"` |

## Troubleshooting

### Sub-Agent Returns Questions

If the sub-agent asks for clarification:
1. Answer the questions
2. Re-invoke with additional context
3. The feature log maintains state

### Sub-Agent Reports Errors

If the sub-agent encounters errors:
1. Check the feature log file exists and is readable
2. Verify `Cargo.toml` has test dependencies
3. Ensure the crate compiles: `cargo check`
4. Re-invoke with corrections

### Tests Don't Compile

Common issues:
1. Missing `use super::*;` in test module
2. Missing `#[cfg(test)]` on test module
3. Test dependencies not in `[dev-dependencies]`
4. Private items not accessible (move to integration tests)

### Need to Modify Tests

If you need to adjust the tests created:
1. You can make small edits directly
2. For major changes, re-invoke the sub-agent with updated feature log
3. The sub-agent can iterate based on feedback

## Model Selection

- **sonnet** (recommended): For comprehensive test creation with complex logic
- **haiku**: For simple, straightforward test creation (faster, cheaper)

Most features should use `sonnet` to ensure high-quality comprehensive test coverage.
