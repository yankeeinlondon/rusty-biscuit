# Inference Trait Contract

## Status

Draft.

## Summary

Create a new lightweight package area, `biscuit-contract`, to define shared inference contracts used by deterministic libraries that can optionally delegate non-deterministic work to an adapter.

Reaper will use this contract to evaluate web pages and sites through deterministic logic first, then call an injected adapter when an operation requires inference. Darkmatter should be able to use the same contract for Markdown and document-oriented inference needs.

The initial adapter implementations are expected to live in:

- `claudine`, backed by agent calls
- `unchained-ai`, backed by direct LLM calls

## Goals

- Define one stable inference trait that Reaper, Darkmatter, Claudine, and Unchained AI can all depend on.
- Keep Reaper and Darkmatter free of direct dependencies on specific agent or LLM providers.
- Let provider crates implement the contract once without adding new trait methods for every new Reaper or Darkmatter inference task.
- Keep the contract crate small, portable, and focused on data contracts only.

## Non-Goals

- Do not put Reaper-specific page analysis types in `biscuit-contract`.
- Do not put Darkmatter-specific document analysis types in `biscuit-contract`.
- Do not define model routing, provider selection, prompt templates, scraping, Markdown parsing, or agent orchestration in `biscuit-contract`.
- Do not require Reaper or Darkmatter to use inference for deterministic operations.

## Package Area

Add a new package area at:

```text
{repo-root}/biscuit-contract
```

The first crate should be a library crate. The package area may follow the standard `lib` layout if that remains the preferred workspace convention:

```text
biscuit-contract/
  README.md
  justfile
  lib/
    Cargo.toml
    src/
      lib.rs
```

The exact workspace wiring should follow the monorepo conventions at implementation time.

## Contract Design

`biscuit-contract` should define a rolled-up inference adapter trait. The trait should model a single inference call, not task-specific methods such as `categorize_page` or `summarize_document`.

Conceptually, an inference call has:

- `prompt`: prose instructions and context for the adapter provider.
- `output_format`: the expected response shape.
- `capability`: a provider-neutral hint about the kind of model or agent behavior desired.

The response should contain:

- optional agent metadata
- model metadata
- returned data matching the requested output format

Draft shape:

```rust
pub trait InferenceAdapter: Send + Sync {
    async fn infer(
        &self,
        request: InferenceRequest,
    ) -> Result<InferenceResponse, InferenceError>;
}

pub struct InferenceRequest {
    pub prompt: String,
    pub output_format: InferenceOutputFormat,
    pub capability: ModelCapability,
}

pub enum InferenceOutputFormat {
    Prose,
    Structured {
        schema: serde_json::Value,
    },
}

pub enum ModelCapability {
    Cheap,
    Fast,
    Thinking(ThinkingLevel),
    Smart(ThinkingLevel),
}

pub enum ThinkingLevel {
    Low,
    Medium,
    High,
}

pub struct InferenceResponse {
    pub agent: Option<String>,
    pub model: String,
    pub data: InferenceData,
}

pub enum InferenceData {
    Prose(String),
    Structured(serde_json::Value),
}
```

This sketch is intentionally high level. The implementation should decide whether to use native async functions in traits, `async-trait`, boxed futures, or another workspace-consistent approach.

## Structured Output

Structured output should use JSON Schema as the portable contract between caller and adapter.

Reaper and Darkmatter should own their domain-specific output types and schemas. For example, Reaper may request structured page categorization by passing a schema generated from a Reaper-owned type, then deserialize the returned `serde_json::Value` into that type after the adapter returns.

This keeps `biscuit-contract` from becoming coupled to any single consumer crate.

## Reaper Usage

Reaper should provide deterministic page and site analysis without requiring an inference adapter.

When an operation needs non-deterministic judgment, Reaper should:

1. Build a prompt from deterministic page/site context.
2. Select an `InferenceOutputFormat`.
3. Select a `ModelCapability`.
4. Call the injected `InferenceAdapter`.
5. Validate and interpret the response in Reaper-owned types.

Example use cases:

- page categorization
- page intent analysis
- content quality assessment
- summary generation
- site-level synthesis from deterministic crawl results

Reaper should treat adapter responses as untrusted external data. Structured responses should be validated or deserialized before use.

## Darkmatter Usage

Darkmatter should be able to use the same contract for Markdown and document analysis tasks.

Example use cases:

- document categorization
- frontmatter suggestion
- title or summary generation
- document quality assessment
- semantic extraction from Markdown content

Darkmatter should own its own prompts, schemas, and result types. It should not need Reaper-specific inference types.

## Claudine Implementation Outline

Claudine should implement `InferenceAdapter` by translating each `InferenceRequest` into an agent call.

Responsibilities:

- Map `ModelCapability` to Claudine's agent selection or execution strategy.
- Pass the prompt to the selected agent workflow.
- Enforce or request the expected `InferenceOutputFormat`.
- Return provider metadata in `InferenceResponse.agent` and `InferenceResponse.model` where available.
- Convert agent errors into `InferenceError`.

Claudine should not need to know whether a request came from Reaper, Darkmatter, or another consumer.

## Unchained AI Implementation Outline

Unchained AI should implement `InferenceAdapter` by translating each `InferenceRequest` into a direct LLM call.

Responsibilities:

- Map `ModelCapability` to model selection and inference settings.
- Pass the prompt to the selected LLM provider.
- Use structured-output support when `InferenceOutputFormat::Structured` is requested.
- Return the concrete model name in `InferenceResponse.model`.
- Convert provider errors into `InferenceError`.

Unchained AI should not need to know the consumer's domain types. It only needs to satisfy the prompt, output format, and capability contract.

## Dependency Direction

The intended dependency direction is:

```text
biscuit-contract
  ^
  |
  +-- reaper
  +-- darkmatter
  +-- claudine
  +-- unchained-ai
```

Consumer crates depend on `biscuit-contract`; `biscuit-contract` depends on none of the consumers.

## Open Questions

- Should `InferenceRequest` own `String` values, borrow with lifetimes, or support both through builder/conversion APIs?
- Should `InferenceAdapter` be object-safe so callers can store `Arc<dyn InferenceAdapter>`?
- Should `InferenceError` be a concrete error enum, a boxed error, or a small enum with provider-specific opaque details?
- Should structured output schemas be raw `serde_json::Value`, a newtype wrapper, or generated through an existing schema crate?
- Should `ModelCapability` include cost, latency, context-window, or determinism hints in addition to the initial capability enum?
- Should response metadata include token usage, latency, provider name, or raw provider IDs?

## Success Criteria

- `biscuit-contract` defines a small, reusable inference contract.
- Reaper can accept an optional adapter for non-deterministic operations without depending on Claudine or Unchained AI.
- Darkmatter can reuse the same contract without importing Reaper types.
- Claudine and Unchained AI can each implement the adapter once and satisfy future Reaper/Darkmatter inference needs through the same rolled-up `infer` method.
