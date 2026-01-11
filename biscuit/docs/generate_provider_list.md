# The shared `generate_provider_list()` utility function

```rust
enum ProviderListFormat {
    string_literals,
    rust_enum
}

#[tokio::async]
async function generate_provider_list(output: Option<ProviderListFormat>): Result<string> {
    // 
}

```


This utility function is provided to other libraries in the monorepo as a means to update the list of LLM providers/models. Output can be either:

- A JSON list of string literals which follow the `${provider}/${model}` naming convention
- A text for a Rust `enum` who's members follow the `${provider}_${model}` naming convention

In both cases the return type is a _string_ but the structure of the returned string depends on what you pass in as the output type. By default the output type is a JSON list of string literals.

> **Note:** when calling this function with the `rust_enum` output you will likely want to use the [`inject_enum()`](./inject_enum.md) shared function.

