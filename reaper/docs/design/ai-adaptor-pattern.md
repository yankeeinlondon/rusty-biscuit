# Inference via the Adapter Pattern

The **Reaper** solution will fully own all of the deterministic operations it uses to perform it's work but when it comes to _non-deterministic_ work **Reaper** will leverage the
`InferenceAdapter` trait defined in **biscuit-contract** so that it can call into an adapter's implementation. 

## Providers

The libraries which provide an adapter implementation of `InferenceAdapter` include:

- `unchained-ai` - provides direct LLM prompting
- `claudine` - provides abstracted Agentic prompting

## Consumers

The **Reaper** library is a consumer of this contract and the **Darkmatter** library is as well.

## Usage Example

TODO

## Considerations

TODO
