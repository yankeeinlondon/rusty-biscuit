---
name: dockhand-library
description: Expert knowledge for working with the Dockhand shared library - comprehensive utilities for LLM providers, markdown manipulation, code generation, terminal rendering, and agent tools in the Dockhand monorepo
---

# Dockhand Shared Library

The shared library (`shared` crate) provides comprehensive utilities for building AI-powered Rust applications in the Dockhand monorepo.

## Core Principles

- **AST-based safety**: Code generation uses `syn` for parsing, never regex
- **Provider flexibility**: Discover models from 8+ providers via APIs or curated lists
- **Tracing-first**: Comprehensive OpenTelemetry instrumentation throughout
- **Terminal-aware**: Adaptive rendering based on terminal capabilities
- **Type-safe**: Strong typing with `thiserror` for errors and `serde` for serialization

## Quick Reference

```rust
// Provider discovery
use shared::providers::{Provider, has_provider_api_key, get_provider_models};

// Markdown manipulation
use shared::markdown::Markdown;
let mut md: Markdown = content.into();
md.normalize(Some(HeadingLevel::H1))?;

// Agent tools
use shared::tools::{BraveSearchTool, ScreenScrapeTool};
let search = BraveSearchTool::from_env();

// Safe code injection
use shared::codegen::inject_enum;
inject_enum("ModelId", new_enum_code, "src/types.rs")?;
```

## Module Overview

| Module | Purpose |
|--------|---------|
| [`providers`](./providers.md) | LLM provider discovery and model enumeration |
| [`markdown`](./markdown.md) | Document manipulation with frontmatter and AST support |
| [`codegen`](./codegen.md) | Safe AST-based Rust code injection |
| [`tools`](./tools.md) | rig-core agent tools (Brave Search, Screen Scrape) |
| [`mermaid`](./mermaid.md) | Diagram theming and rendering |
| [`interpolate`](./interpolate.md) | Content interpolation for strings, markdown, HTML |
| [`isolate`](./isolate.md) | Content extraction from structured documents |
| [`terminal`](./terminal.md) | Terminal capability detection and rendering |

## Detailed Topics

- [Provider System Architecture](./providers.md) - Three-tier provider discovery
- [Markdown Processing](./markdown.md) - Frontmatter, normalization, delta analysis
- [Code Generation Safety](./codegen.md) - AST manipulation and atomic writes
- [Agent Tool Integration](./tools.md) - rig-core compatible tools with tracing
- [Terminal Rendering](./terminal.md) - Color detection and adaptive output
- [Testing Utilities](./testing.md) - Terminal output verification helpers

## Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `rig-core` | 0.27 | Agent framework |
| `syn` | 2.0 | AST parsing |
| `pulldown-cmark` | 0.13 | Markdown parsing |
| `syntect` | 5.2 | Syntax highlighting |
| `reqwest` | 0.12 | HTTP client |
| `tokio` | 1.48 | Async runtime |

## Usage Examples

### Provider Discovery

```rust
// Check API key availability
if has_provider_api_key(Provider::OpenAI) {
    let models = get_provider_models(Provider::OpenAI).await?;
}

// Generate provider list
use shared::providers::{generate_provider_list, ProviderListFormat};
let json = generate_provider_list(Some(ProviderListFormat::StringLiterals)).await?;
```

### Markdown Processing

```rust
// Load and normalize markdown
let mut md = Markdown::try_from(Path::new("README.md"))?;
let (normalized, report) = md.normalize(Some(HeadingLevel::H1))?;

// Compare documents
let delta = original.delta(&updated);
println!("{}", delta.summary());
```

## Resources

- [Architecture Overview](./architecture.md) - Design patterns and principles
- [Tracing Guide](./tracing.md) - OpenTelemetry instrumentation
- API documentation: `cargo doc --package shared --open`