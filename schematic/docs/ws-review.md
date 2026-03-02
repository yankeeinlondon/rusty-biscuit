# Schematic WebSocket Runtime Review

## Implementation Status (2026-03-02)

- Implemented: typed path/query connection parameter lowering with URL substitution and encoded query assembly.
- Implemented: correlated request timeout now uses `WsClientOptions.request_timeout`.
- Implemented: transport-level wiring now uses `connect_async_with_config(..., websocket_config, disable_nagle)`.
- Implemented: `WsClientOptions::builder()` and fluent setters for websocket runtime options.
- Implemented: reconnect supervisor loop with backoff/jitter policy, pending-request fail-fast on disconnect, and manual-close shutdown behavior.
- Implemented: outbound serialization path now supports binary JSON frames for non-`JsonText` frame formats.
- Implemented: runtime integration tests in `schematic/schema/tests/ws_runtime.rs` for path/query handshake behavior, timeout handling, disconnect handling, and reconnect behavior.
- Deferred with explicit TODO: `WsClientOptions.receive_timeout` is defined but not yet enforced as an inbound idle timeout.

## 1. Design Goal Alignment

The current implementation has made significant progress against the goals set out in `ws-client-design.md`. The generator produces `ws_shared.rs` and the WebSocket modules contain transport handles, task spawning, and message handling for both client and host roles.

### What's Implemented Successfully

- **`ws_shared.rs` Scaffolding:** Provides a strong typed error enum (`WsError`), `WsClientOptions`, and transport handle lifecycle.
- **Correlated Request-Response:** Implements pending maps, atomic IDs, and `request()` methods for APIs supporting correlation.
- **Host Role Generation:** Basic host-mode code is generated for bidirectional APIs (like Unfolded Circle Integration WS).
- **Event Streaming:** Inbound event consumption through streams adapter (`events()`) and direct polling (`next_event()`).
- **Auth Headers:** Correctly applies header-based authorization strategies directly into `tokio_tungstenite` handshakes via the `Headers` builder.

### Missing or Incomplete Goals

- **Path and Connection Parameters:** The generator completely ignores the `WebSocketEndpoint.connection_params` field and path substitution. Path parameters like `{voice_id}` (e.g., ElevenLabs API) remain hardcoded strings (`/v1/text-to-speech/{voice_id}/stream-input`) at runtime. Query parameters are also not appended to the connection string.
- **Reconnect Policy:** Although `WsClientOptions` defines `ReconnectPolicy`, the reconnect loop is not implemented in the generated `connect_*` methods. When a connection drops, the internal reader task simply sets the connection state to `Disconnected` and exits.
- **`request_timeout` Override:** The generated `request()` method hardcodes `std::time::Duration::from_secs(30u64)` and ignores the `request_timeout` configured in `WsClientOptions`.
- **Nagle's Algorithm & Configuration:** `disable_nagle` is ignored. `tungstenite::WebSocketConfig` is absent from `WsClientOptions` completely, skipping features like write buffer sizing and frame size limits.

## 2. Ergonomic and Performance Opportunities

**Ergonomics:**

- **Typed Connection Parameters:** `connect_<endpoint>` methods currently only accept `options: WsClientOptions`. To preserve the ergonomic standard set by the REST clients, they should accept required path parameters and query parameters as arguments, e.g., `connect_text_to_speech(&self, voice_id: impl Into<String>, options: WsClientOptions)`.
- **Builder Pattern for Options:** `WsClientOptions` could benefit from a fluent builder (`WsClientOptions::builder().disable_nagle().build()`) to improve the initialization ergonomics over struct initialization.

**Performance:**

- **Zero-Copy Serialization:** The current outbound writer converts the struct to a JSON `String` first via `serde_json::to_string()`, then creates a `Message::Text(String)`. Converting this directly to `Vec<u8>` or writing into a pooled buffer could save intermediate string allocations.
- **Connection Multiplexing (Future):** While not in scope for v1, future optimization should consider sharing a single transport handle across multiple endpoint instances if the provider supports sub-protocols.

## 3. Test Coverage Validation

### Current State

- **Validation Tests:** `ws_codegen.rs` successfully covers the plan lowering phase and AST structural checks (verifying that the generated files parse as valid `syn::File` Rust code).
- **Method Checking:** Checks verify whether `request()`, `connect_*()`, and `events()` methods are injected correctly based on the endpoint role and correlation settings.
- **Lack of Integration Tests:** There are currently **no runtime integration tests** to validate live behavior. The tests only ensure the generated code is syntactically valid Rust.

### Required Test Improvements

- **Local WebSocket Test Server:** We need `tokio-tungstenite`-backed mock servers within `schematic/schema/tests/` to perform End-to-End integration testing for:
  - Connection handshakes (validating the `{voice_id}` substitution once implemented).
  - Correlated request-response timeout tracking.
  - Verification that the connection drops correctly signal the watcher.
  - Reconnect loop verification (once implemented).

## 4. Suggested Next Steps

1. **Fix Path/Query Parameters:** Update `ws_codegen/client.rs` to extract `connection_params` and generate typed arguments for the `connect_*` methods. Substitute path parameters in the URL and serialize query parameters identically to the REST client implementation.
2. **Fix Timeout Hardcoding:** Update `ws_codegen/client.rs` to reference the `max_pending` and `request_timeout` passed into the connection from `WsClientOptions`.
3. **Write E2E Contract Tests:** Before tackling the `ReconnectPolicy`, implement basic e2e HTTP/WS tests to assert that the generated client can successfully echo messages to a mock server.
4. **Implement Reconnect Loop:** Add a manager task inside the connection handler that intercepts the `reader_handle` exit state and triggers a dial loop based on the `ReconnectPolicy` backoff.
