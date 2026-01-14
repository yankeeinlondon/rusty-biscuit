# Agent Development Guide

This document provides essential information for agentic coding agents working in the Dockhand monorepo.

## Build Commands

```bash
# Build all areas
just build

# Build specific package
cargo build -p shared
cargo build -p research-cli -p research-lib

# Install binaries
just install
```

## Test Commands

```bash
# Run all tests
just test

# Run tests for specific package
cargo test -p shared
cargo test -p research-lib

# Run a single test
cargo test -p shared <test_name>

# Run specific test with output
cargo test -p shared --lib providers::base::tests::test_name -- --nocapture

# Run with verbose logging
cargo test -p shared -- --nocapture
```

## Linting

```bash
# Check for common mistakes
cargo clippy

# Check specific package
cargo clippy -p shared
```

## Skills

- At a minimum, all prompts should use the `rust` skill.
- All testing should at a minimum use the `rust` and `rust-testing` skills

## Code Style Guidelines

### Formatting

- **Indentation:** 4 spaces (configured in rustfmt.toml)
- **Edition:** Rust 2024
- Always run `cargo fmt` before committing

### Imports

Organize imports in this order:

1. std library
2. External crates
3. Local modules (same crate)
4. Use `use crate::` for intra-crate imports

Example:

```rust
use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::providers::base::Provider;
use crate::tools::BraveSearchTool;
```

### Types

- Use `Result<T, MyError>` for fallible functions
- Prefer `thiserror::Error` for custom error types
- Use `#[from]` for automatic error conversion
- No `unwrap()` or `expect()` in production code (tests only)

### Naming Conventions

- **Types:** PascalCase (e.g., `ProviderConfig`, `BravePlan`)
- **Functions:** snake_case (e.g., `has_provider_api_key`, `build_auth_header`)
- **Constants:** SCREAMING_SNAKE_CASE (e.g., `PROVIDER_CONFIGS`)
- **Private fields:** snake_case (e.g., `api_key`, `base_url`)




### Error Handling

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Missing API key for {provider}")]
    MissingApiKey { provider: String },
}
```

### Documentation

- **Module docs:** Start with `//!` at top of file
- **Function docs:** Use `///` with sections in order:
  1. Brief summary (no heading)
  2. `## Examples`
  3. `## Returns`
  4. `## Errors` (if applicable)
  5. `## Panics` (if applicable)
  6. `## Safety` (for unsafe)
  7. `## Notes`

**Avoid explicit `# Heading` (H1)** inside `///` docblocks - Rustdoc provides the item name.

### Tracing

- Use `#[tracing::instrument]` on async functions
- Skip sensitive data: `#[instrument(skip(api_key))]`
- Use structured fields: `#[instrument(fields(provider = %provider))]`
- Libraries emit events, applications configure subscribers

### Testing

- Use `#[tokio::test]` for async tests
- Use `#[serial_test::serial]` for environment variable tests
- Test in `tests/` directory for integration tests
- Use `tracing_test` with `#[traced_test]` for tracing assertions

Example:

```rust
#[test]
#[serial_test::serial]
fn test_has_provider_api_key() {
    // test code
}
```

### Code Comments

All public/exported symbols must be documented using
Rust's `///` styled comments.

### Dependencies

- Check `docs/dependencies.md` before adding new dependencies
- Prefer existing dependencies over adding new ones
- All deps listed in `Cargo.toml` with version pinning

## Monorepo Structure

The repository is organized into the following packages:

```txt
dockhand/
├── ai-pipeline
│   ├── cli/          # Binary: `research` (FUTURE)
│   └── lib/          # Core research library
│   └── service/      # Server to abstract AI pipelining functionality (FUTURE)
├── biscuit/          # Common utilities (providers, tools, TTS, codegen)
├── darkmatter/       # Binary: `md` (markdown terminal renderer)
├── research/         # AI-powered library research tools
│   ├── cli/          # Binary: `research`
│   └── lib/          # Core research library
├── schematic/        # Schema generation for API's and other schema
│   ├── define/        # Tooling to Schema definition
│   └── gen/          # Generation code to take definition code -> schema code
│   └── schema/       # Generated schemas
├── sniff/
│   ├── cli/          # Binary: `sniff`
│   └── lib/          # Hardware, Network, OS, and package manager discovery
├── so-you-say/       # Binary: `speak` (TTS CLI)
└── tui/              # Future: ratatui-based interactive chat
```
