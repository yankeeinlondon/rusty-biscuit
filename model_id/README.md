# `model_id` Procedural Macro

This repo provides a Rust Procedural Macro which can be used to map common string-literal representations of an LLM model to and from a enumeration in your Rust code.

Example:

- the model name `gpt-4o` is a valid model name on **OpenAI** (a primary model provider)
- the model name `openai/gpt-4o` is a valid model name on **OpenRouter** (a model aggregator)

To represent these two models let's assume you have the following enumerations in your code base:

```rust
#[derive(ModelId)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderOpenAi {
  Gpt_4o,
  Bespoke(String)
}

#[derive(ModelId)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderOpenrouter {
  OpenAi__Gpt_4o,
  Bespoke(String)
}
```

With the use of the the `ModelId` macro both primary and aggregator provider will automatically be able to provide a string reference to the model name on the give platform by simply calling the `model_id()` function:

```rust
// `gpt-40`
let primary = ProviderOpenAi::Gpt_4o.model_id();
// `openai/gpt-40`
let aggregator = ProviderOpenRouter::OpenAi__Gpt_4o.model_id();
```

## Rules

For this procedural macro to work, your enum variant names must follow these rules and delimiters:

1. **Aggregator providers** must *always* include the underlying provider, delimited from the model by **three underscores**: `Provider___Model`.
2. **Primary providers** must *never* include the provider prefix or the `___` delimiter; they include only the model name.
3. Within the **model portion** of the variant name, any `-` characters in the wire model id are encoded as **two underscores**: `__`.
4. Within the **model portion** of the variant name, any `.` characters in the wire model id are encoded as **one underscore**: `_`.
5. All enums provide a “safety hatch” variant `Bespoke(String)`, which maps to the wire id by borrowing the inner string (`s.as_str()`).
6. For readability, all non-underscore segments (runs of letters/digits between underscores) should use **PascalCase** (recommended convention; the macro does not require it).

