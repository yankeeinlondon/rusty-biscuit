# Completion Checklist

The following checklist should be used as a way to validate that a newly created API definition is completely done:

## 1. Design & Specification Alignment

- [ ] All endpoints specified in the design document are implemented.
- [ ] The base URL, default port, and `AuthStrategy` are correctly configured.
- [ ] All HTTP methods match the specification or known API quirks (e.g., mutative actions using GET if required).
- [ ] Response types are correctly mapped (e.g., using `ApiResponse::json_type(...)` even if the server returns `text/plain` with a JSON body).
- [ ] Known typos, quirks, and irregular field names in the upstream API are handled (e.g., using `#[serde(rename = "...")]`).

## 2. Idiomatic Rust & Typing

- [ ] **Strict Nullability:** Fields guaranteed to be present by the API are strictly typed (e.g., `String` instead of `Option<String>`).
- [ ] **Strong Discriminants:** "Stringly typed" enumerations or status flags are modeled as enums or `Cow<'static, str>` with exhaustive valid values documented.
- [ ] **Standardized Derives:** Data models derive `Eq` and `Hash` consistently (unless floating-point numbers or `serde_json::Value` are involved) for maximum usability in collections.
- [ ] **Boolean Query Params:** Flags like `0/1` are modeled as `QueryParamType::Boolean` if supported, or explicitly documented if integer mapping is the safest fallback.

## 3. Documentation & Developer Experience (DX)

- [ ] **Contextualized Types:** Enum-like integers (e.g., `state: 0 = Stopped`) are documented, and reasons for missing optional fields are explained.
- [ ] **Rich Endpoint Descriptions:** Descriptions provide usage examples, typical scenarios, common parameter values, and links to the official API docs.
- [ ] **Client Instantiation Examples:** Module-level docs include code snippets demonstrating how to instantiate the client, override placeholder base URLs/IPs, and handle secondary authentication methods (e.g., `Headers::use_basic_auth()`).
- [ ] **Payload Documentation:** Envelope structures (e.g., WebSockets) clearly document how the generic payload should be cast or deserialized based on the message identifier.

## 4. Performance Optimizations

- [ ] **Deferred JSON Parsing:** Envelope payloads use `Option<Box<serde_json::value::RawValue>>` instead of `Option<serde_json::Value>` to avoid costly AST allocations during routing.
- [ ] **Pre-allocated Vectors:** Endpoint vectors with a known static size are initialized with `Vec::with_capacity(n)`.
- [ ] **Minimized Helper Allocations:** Repetitive string allocations in helper functions are avoided (using lazy statics, array slices, or `Cow`).
- [ ] **Memory Churn Warning:** Endpoints expected to be polled at high frequencies document the potential memory overhead of generating owned `String` fields repeatedly.

## 5. Verification & Testing

- [ ] The definition includes unit tests validating metadata (name, base URL, docs URL).
- [ ] The definition includes unit tests validating the total endpoint count, authentication strategy, and `env_mapping` defaults.
- [ ] Unit tests verify that endpoints have the correct HTTP methods and that parameters (query, path) are correctly mapped.
- [ ] A final verification pass (e.g., `cargo test -p schematic-gen`, `just generate`, `cargo check -p schematic-schema`) completes without errors.
