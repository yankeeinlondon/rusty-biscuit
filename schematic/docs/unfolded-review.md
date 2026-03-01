# Unfolded Circle API - Schematic Implementation Review

This document provides a review of the Unfolded Circle API implementations within Schematic (`schematic-define`), evaluating them against the goals outlined in the original design document (`@schematic/docs/unfolded-circle.md`).

## 1. Design Document Alignment

Overall, the foundational primitives for the Unfolded Circle APIs have been successfully laid out, matching the proposed architecture. However, there are a few areas where the implementation falls short of the full vision described in the design document:

* **Core REST API MVP:** Successfully implemented. The endpoints handle multipart uploads (e.g., `InstallCustomIntegration`, `RestoreBackup`) and non-JSON responses (`ApiResponse::Binary` and `ApiResponse::Text`) exactly as prescribed. The Auth fallback behavior via `EnvMapping` is also correctly wired up for both Bearer and Basic authentication.
* **WebSocket Message Surface (Missing Payload Schemas):** The design document notes a message set size of "~150 publish + 92 subscribe refs". Currently, the `define_unfolded_circle_core_ws_api()` and its `common_messages()` helper only define the *envelope* schemas (Request, Response, Event, Auth). The specific typed message variants (the actual payload models) appear to be missing from the definition layer. If the AsyncAPI importer is meant to populate these, it either hasn't been run or needs to map the imported schemas into the `WebSocketApi` message lists.
* **Integration API Role Inversion:** The design calls out that the Integration API must support the "driver-as-server" role. The implementation correctly addresses this by setting `direction: MessageDirection::Bidirectional` on the Integration WS envelopes, ensuring the generated client/host can both send and receive these envelopes.
* **Auth Strategy Constraints:** The design document explicitly mentions supporting three auth methods for Core REST (Basic, Bearer, Cookie session). Currently, the `RestApi` definition uses `auth: AuthStrategy::BearerToken { header: None }`. While `env_mapping` specifies keys for Basic Auth (`basic_user`, `basic_pass`), Schematic's `RestApi` struct only allows a single `AuthStrategy` variant. The implementation correctly relies on `Headers::use_basic_auth()` overrides, but this limitation means the generated client might not natively document Basic auth as a primary constructor path.

## 2. Idiomatic Rust & Client Improvements

The generated API client could be made more ergonomic and idiomatic with the following adjustments to the definition types:

* **Strict Nullability:** In `core_rest/types.rs`, models like `SystemInfo` and `IntegrationDriverInfo` define every field as `Option<String>`. If the Unfolded Circle OpenAPI spec guarantees certain fields (e.g., `id`, `model`, `version`), they should be strongly typed as `String` rather than `Option<String>`. This prevents users from having to constantly `unwrap()` fields that are always present.
* **Strongly Typed Discriminants:** The `kind`, `type_name`, and `msg` fields in the WebSocket envelopes (e.g., `CoreWsRequestEnvelope`) are typed as `String`. To make the client more robust, these could be represented as enums (e.g., `EnvelopeKind::Req`, `EnvelopeKind::Resp`) or at least `Cow<'static, str>` to avoid heap allocations.

## 3. Performance Improvements

There is a significant performance bottleneck in the way WebSocket envelopes are currently modeled:

* **Defer Payload Parsing (`RawValue` vs `Value`):** In `core_ws/types.rs`, `dock_ws/types.rs`, and `integration_ws/types.rs`, the `msg_data` fields are typed as `Option<serde_json::Value>`. When `serde_json` deserializes into a `Value`, it allocates a full AST for the entire payload. Because this is just an envelope used for routing, it's a massive performance hit.
  **Recommendation:** Change `msg_data` to `Option<Box<serde_json::value::RawValue>>`. This tells Serde to keep the payload as an unparsed string slice while parsing the envelope. The dispatcher can then look at the `msg` type and deserialize the `RawValue` directly into the correct struct, saving a full DOM allocation cycle.
* **Allocation in `common_messages()`:** The `common_messages()` function in `core_ws/mod.rs` instantiates a new `Vec<MessageSchema>` and allocates multiple `String` descriptions every time it is called. Since it's called four times during the API definition, returning a `Vec` from a lazy static or array slice, or using `Cow` strings in the `schematic_define` structs would reduce unnecessary allocation overhead during the code generation phase.

## 4. Documentation Enhancements

The documentation provided to the generated API clients can be enriched:

* **Enhance Endpoint Descriptions:** The `description` fields on `Endpoint` and `WebSocketEndpoint` structs are currently one-liners (e.g., `"Core command and event channel"`). Schematic uses these to generate Rustdoc for the client methods. Expanding these descriptions to include usage examples, typical scenarios, or links to the official Unfolded Circle docs (e.g., pointing to the specific AsyncAPI channel docs) will vastly improve the developer experience.
* **Document WebSocket Payloads:** Add docstrings to the `msg_data` envelope fields explaining how the user (or the generated dispatcher) is expected to cast or deserialize these payloads based on the `msg` identifier.
* **Document Authentication Workarounds:** Since Core REST defines `BearerToken` as its primary strategy, the module-level documentation (`core_rest/mod.rs`) should explicitly include a code snippet demonstrating how to use `Headers::use_basic_auth()` to authenticate with `UCR_CORE_USER` and `UCR_CORE_PASSWORD`.
