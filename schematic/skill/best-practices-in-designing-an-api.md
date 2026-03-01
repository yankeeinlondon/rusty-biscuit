# Best Practices when Designing an API

This document will provide details into what a well designed API definition -- using primitives from `schematic-define` looks like:

## Best Practices

Based on reviews of existing API definitions (such as Eversolo and Unfolded Circle), the following patterns have been identified to help improve developer experience (DX), maintain idiomatic Rust design, and optimize performance.

### 1. Idiomatic Rust & Typing

* **Strict Nullability:** Avoid defaulting all fields to `Option<T>` out of caution. If the underlying API or specification guarantees that a field is present, type it strongly (e.g., `String` rather than `Option<String>`). This prevents downstream users from having to constantly `unwrap()` fields that are always present.
* **Strongly Typed Discriminants:** Avoid "stringly typed" fields for enumerations, flags, discriminants (like `kind` or `type_name` in envelopes), or fixed-set parameters. Define proper enums or at least use `Cow<'static, str>` to avoid heap allocations. Document exhaustive lists of valid values directly in the endpoint description to prevent runtime validation errors.
* **Standardized Derives:** Consistently derive `Eq` and `Hash` on response and request models (unless floating-point numbers or unstructured `serde_json::Value` are involved) to maximize their usability in standard collections (like `HashSet` or as `HashMap` keys).

### 2. Documentation & Developer Experience (DX)

* **Contextualize Types & Enums:** Document what enum-like integers mean (e.g., `state: 0 = Stopped, 1 = Playing`), and explain *why* optional fields might be missing (e.g., "May be absent on older firmware versions"). This surfaces critical context in the IDE for developers using the generated client.
* **Rich Endpoint Descriptions:** Move beyond one-liner descriptions. Expand descriptions to include usage examples, typical scenarios, common parameter values (e.g., `"Remote key constant (e.g., Key.VolumeUp, Key.MediaPlay)"`), and links to the official API documentation.
* **Code Examples in Module Docs:** Provide concrete code examples of client instantiation. This is particularly important for demonstrating how to override a placeholder base URL or host IP at runtime, or how to handle secondary authentication methods (e.g., demonstrating the use of `Headers::use_basic_auth()` when Bearer auth is the default).
* **Document Payload Handling:** For envelope structures (like WebSockets), document how the generic payload should be cast or deserialized based on the message identifier.

### 3. Performance Optimization

* **Defer Payload Parsing (`RawValue` vs `Value`):** When modeling WebSocket envelopes or generic message wrappers, avoid using `Option<serde_json::Value>` for payload data. This allocates a full AST for the entire payload before the target type is known, creating a significant performance bottleneck. Instead, use `Option<Box<serde_json::value::RawValue>>` to keep the payload as an unparsed string slice and defer parsing until the dispatcher can deserialize directly into the target struct.
* **Pre-allocate Vectors:** When building lists of endpoints where the total count is known and static, initialize the vector with `Vec::with_capacity(n)` instead of `Vec::new()` to eliminate unnecessary reallocations during the schema generation phase.
* **Avoid Repeated Allocations in Helpers:** Be mindful of allocations in helper functions (e.g., returning `Vec`s with allocated strings multiple times). Use lazy statics, array slices, or `Cow` strings where appropriate to reduce overhead during the API definition building phase.
* **String Allocations in High-Frequency Polling:** If an endpoint is meant to be polled frequently (e.g., getting current playback state multiple times a second), be aware that generated models will allocate owned `String` fields repeatedly. Document this memory churn overhead so downstream consumers can optimize their polling logic (e.g., by diffing numerical timestamps/positions instead of full metadata payloads).