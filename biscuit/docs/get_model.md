# The `get_model()` and `create_stack()` utility functions

```rust
enum ModelProvider {
    anthropic_claude_opus_4_5,
    anthropic_claude_sonnet_4_5,
    // ...
}

/// A category of LLM provider/models which can perform
/// either a particular use-case well, or are of similar 
/// level of sophistication
enum ModelKind {
    // sophistication
    fast,
    normal,
    smart,
    // use-case
    summarize,
    scrape,
    consolidate,
    // explicit
    try_explicit(ModelProvider, ModelKind)
}

struct ModelStack(Client[])

let fast_model = ModelStack::new([
    ModelProvider::anthropic_claude_haiku_4_5,
    // ...
]);

let normal_model = ModelStack::new([
    ModelProvider::anthropic_claude_sonnet_4_5,
    // ...
]);

/// Get's a `rig` **Client<T>** to work with an LLM.
function get_model(kind: ModelKind, desc: Option<&str>): Result<Client, NoValidModel> {
    //
}
```

The `get_model()` utility function takes a `ModelKind` and returns a **rig** `Client<T>` so that the caller can interact with the LLM model.

- if the optional `desc` property is passed in then this function will log a message to STDERR:

    `- using the ${model} from ${provider} to ${desc}`




