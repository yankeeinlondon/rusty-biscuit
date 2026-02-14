# `model_id` Procedural Macro

A Rust procedural macro for mapping LLM model enum variants to and from their wire-format string IDs.

## Overview

This crate provides a `#[derive(ModelId)]` macro that automatically generates:

- `model_id(&self) -> &str` - Returns the canonical wire-format model ID
- `FromStr` implementation - Parses wire IDs back to enum variants
- `ALL: &'static [Self]` - Constant array of all known unit variants (excludes `Bespoke`)

## Example

```rust
use model_id::ModelId;

// Primary provider (no provider prefix)
#[derive(ModelId)]
#[allow(non_camel_case_types)]
pub enum ProviderOpenAi {
    Gpt_4o,
    Gpt_4o__Mini,
    Bespoke(String),
}

// Aggregator provider (includes provider prefix)
#[derive(ModelId)]
#[allow(non_camel_case_types)]
pub enum ProviderOpenRouter {
    OpenAi___Gpt_4o,
    Anthropic___Claude_3__Opus,
    Bespoke(String),
}

// Usage
let primary = ProviderOpenAi::Gpt_4o.model_id();        // "gpt.4o"
let aggregator = ProviderOpenRouter::OpenAi___Gpt_4o.model_id();  // "openai/gpt.4o"

// Parse from wire format
let model: ProviderOpenAi = "gpt.4o".parse().unwrap();
assert_eq!(model, ProviderOpenAi::Gpt_4o);

// Unknown models fall back to Bespoke (if present)
let unknown: ProviderOpenAi = "custom-model".parse().unwrap();
assert_eq!(unknown, ProviderOpenAi::Bespoke("custom-model".to_string()));

// Iterate all known models
for model in ProviderOpenAi::ALL {
    println!("{}", model.model_id());
}
```

## Encoding Rules

For this procedural macro to work, your enum variant names must follow these conventions:

| Rule | Description | Example |
|------|-------------|---------|
| 1. Provider delimiter | Aggregator providers separate provider from model with `___` (three underscores) | `OpenAi___Gpt_4o` → `"openai/gpt.4o"` |
| 2. Primary providers | No provider prefix or `___` delimiter | `Gpt_4o` → `"gpt.4o"` |
| 3. Hyphen encoding | `-` in wire ID is encoded as `__` (two underscores) | `Gpt_4o__Mini` → `"gpt.4o-mini"` |
| 4. Dot encoding | `.` in wire ID is encoded as `_` (single underscore) | `Gpt_3_5` → `"gpt.3.5"` |
| 5. Bespoke fallback | `Bespoke(String)` variant passes through the inner string | `Bespoke("custom".into())` → `"custom"` |
| 6. Casing | PascalCase segments are lowercased in output | `GptFour` → `"gptfour"` |

## Override Attribute

For variant names that don't follow the encoding rules, use the `#[model_id("...")]` attribute to specify the exact wire ID:

```rust
#[derive(ModelId)]
pub enum ProviderOpenAi {
    // Uses normal encoding
    Gpt_4o,

    // Override with explicit wire ID
    #[model_id("gpt-4-turbo-preview")]
    Gpt4TurboPreview,

    #[model_id("claude-3-opus-20240229")]
    Claude3Opus,

    Bespoke(String),
}

assert_eq!(ProviderOpenAi::Gpt4TurboPreview.model_id(), "gpt-4-turbo-preview");
```

## Generated Code

The macro generates the following for each enum:

```rust
// Error type for parsing failures (only used if no Bespoke variant)
pub struct UnknownModelIdError {
    pub model_id: String,
    pub enum_name: String,
}

impl YourEnum {
    // All unit variants (excludes Bespoke)
    pub const ALL: &'static [Self] = &[...];

    // Get wire-format ID
    #[must_use]
    pub fn model_id(&self) -> &str { ... }
}

impl std::str::FromStr for YourEnum {
    type Err = UnknownModelIdError;
    fn from_str(s: &str) -> Result<Self, Self::Err> { ... }
}
```

## Edge Cases

### Multiple `___` delimiters

If a variant name contains multiple `___` delimiters, only the first is treated as the provider separator:

```rust
// OpenAi___Gpt___4 becomes "openai/gpt-4" (second ___ becomes -)
```

### Enums without Bespoke

If your enum doesn't have a `Bespoke(String)` variant, `FromStr` returns `Err(UnknownModelIdError)` for unrecognized IDs:

```rust
#[derive(ModelId)]
pub enum StrictProvider {
    ModelA,
    ModelB,
}

let result: Result<StrictProvider, _> = "unknown".parse();
assert!(result.is_err());
```

### Repeated separators

The macro defensively collapses repeated dashes (`--`) and dots (`..`) in the output to handle edge cases in variant naming.

## Requirements

- Rust 2021 edition or later
- The enum must have at least one variant
- `Bespoke` variant (if present) must be a single-field tuple: `Bespoke(String)`
