# Schematic Dependencies

## Structure

- `define/Cargo.toml` - REST API definition types
- `gen/Cargo.toml` - Code generator for REST API definitions
- `runtime/Cargo.toml` - Hand-written request runtime shared by generated REST clients
- `schema/Cargo.toml` - Generated REST API client code (auto-generated)

## Workspace Packages

- [schematic-define](../define) _v0.1.0_

    _REST API definition types for schematic code generation._

    _Tags: workspace, definitions_

- [schematic-runtime](../runtime) _v0.1.0_

    _Hand-written request runtime shared by schematic-generated REST clients: the response envelope, errors, metadata redaction, credentials, deadline, and the `Decode` boundary._

    _Tags: workspace, library, http_

- [schematic-schema](../schema) _v0.1.0_

    _Generated REST API client code from schematic definitions (auto-generated)._

    _Tags: workspace, generated_

---

## schematic-define

### Production Dependencies

- `url` v2 validates HTTP(S) base URLs at REST API build.
- `schemars` v1 generates provider-neutral typed registrations with OpenAPI 3.0 settings.
- `serde_json` v1 is an unconditional production dependency for schema component wire schemas and transfer data.

- [serde](https://github.com/serde-rs/serde) _v1.0_ [📄](https://serde.rs)

    _Industry-standard serialization framework providing derive macros for automatic trait implementation._

    _Tags: serialization, json_

- [strum](https://github.com/Peternator7/strum) _v0.26_ [📄](https://docs.rs/strum)

    _Derive macros for enums providing string conversion, iteration, and property access._

    _Tags: enums, derive, macros_

- [thiserror](https://github.com/dtolnay/thiserror) _v2.0_

    _Derive macro for std::error::Error trait._

    _Tags: errors, macros_

---

## schematic-gen

### Production Dependencies

- [clap](https://github.com/clap-rs/clap) _v4.5_ [📄](https://docs.rs/clap)

    _Command-line argument parser with derive API for declarative CLI definitions._

    _Tags: cli, argument-parsing_

- [thiserror](https://github.com/dtolnay/thiserror) _v2.0_

    _Derive macro for std::error::Error trait._

    _Tags: errors, macros_

- [schematic-define](../define) _v0.1.0_

    _Local workspace dependency for REST API definition types._

    _Tags: workspace_

- [syn](https://github.com/dtolnay/syn) _v2.0_ [📄](https://docs.rs/syn)

    _Parsing library for Rust tokens into syntax trees. Foundation for procedural macros._

    _Tags: ast, parsing, proc-macro_

- [quote](https://github.com/dtolnay/quote) _v1.0_ [📄](https://docs.rs/quote)

    _Quasi-quoting macro for turning Rust syntax tree data structures into tokens of source code._

    _Tags: proc-macro, code-generation_

- [proc-macro2](https://github.com/dtolnay/proc-macro2) _v1.0_ [📄](https://docs.rs/proc-macro2)

    _Substitute implementation of compiler's proc_macro API enabling unit testing and non-macro use cases._

    _Tags: proc-macro, tokens_

- [prettyplease](https://github.com/dtolnay/prettyplease) _v0.2_

    _Minimal syn syntax tree pretty-printer producing rustfmt-quality code formatting._

    _Tags: formatting, code-generation_

- [biscuit-file](../../biscuit-file/lib) _v0.1.0_

    _Local workspace dependency (`toml`, `yaml` features): TOML-safe string serialization in the generated manifest and YAML reading in the AsyncAPI importer._

    _Tags: workspace_

- [colored](https://github.com/colored-rs/colored) _v3.0_

    _Terminal colors for the generator's status and diagnostic output._

    _Tags: cli, terminal_

- [schematic-definitions](../definitions) _v0.1.0_

    _Local workspace dependency holding the API definitions the generator reads._

    _Tags: workspace_

- [serde](https://github.com/serde-rs/serde) _v1.0_ [📄](https://serde.rs)

    _Serialization framework; `ValidationDiagnostic` derives `Serialize`._

    _Tags: serialization_

- [serde_json](https://github.com/serde-rs/json) _v1.0_

    _JSON writer for the exported OpenAPI documents and Postman collections._

    _Tags: json, serialization_

- [openapiv3](https://github.com/glademiller/openapiv3) _v2_ [📄](https://docs.rs/openapiv3)

    _OpenAPI 3 document types used by the OpenAPI importer and exporter._

    _Tags: openapi, serialization_

- [indexmap](https://github.com/indexmap-rs/indexmap) _v2_ [📄](https://docs.rs/indexmap)

    _Insertion-ordered maps, so generation plans and exported documents keep a deterministic order._

    _Tags: collections_

### Development Dependencies

- [tempfile](https://github.com/Stebalien/tempfile) _v3.15_

    _Secure cross-platform temporary file/directory creation with automatic cleanup._

    _Tags: testing, filesystem, cleanup_

- [biscuit-test-harness](../../biscuit-test-harness) _v0.1.0_

    _Declared twice on purpose: an unconditional `[dev-dependencies]` entry every
    test target can use, plus an optional `[dependencies]` entry behind
    `schematic-gen/terminal-tests` that carries it into the L2 tmux tier.
    `level2/terminal_capture.rs` uses its `bin_exe!` macro to locate the generator; the
    L1 artifact and Postman tests use `manifest_dir!()` to resolve committed
    artifacts through the run-time manifest directory, so an archived run reads
    the executing checkout rather than the builder's._

    _Tags: workspace, testing_

- [test-toolkit](../../tools/test-toolkit) _v0.1.0_

    _Declared twice, like `biscuit-test-harness`: optional under the same
    `terminal-tests` feature, where its L2 gate records tmux execution evidence
    for CI's backend-proof verification, and an unconditional
    `[dev-dependencies]` entry for the `test_layout` gate in the consolidated
    `l1` test binary. The dev entry adds no crate to the workspace._

    _Tags: workspace, testing_

- [jsonschema](https://github.com/Stranger6667/jsonschema) _v0.55_ [📄](https://docs.rs/jsonschema)

    _JSON Schema validator (default features off) used by the L1 tests that validate exported Postman collections against the published schema._

    _Tags: testing, json-schema_

---

## schematic-runtime

Every crate below was already in the workspace graph, so adding the runtime
adds no package to `Cargo.lock`. Every production dependency is also already
in `schematic/schema/Cargo.lock`, so the generated crate can depend on the
runtime offline. The runtime does not depend on `schematic-define`, and it
keeps HTTP-library types out of its `Decode` boundary: a codec reads a byte
stream, not a library response.

### Production Dependencies

- [base64](https://github.com/marshallpierce/rust-base64) _v0.22_ [📄](https://docs.rs/base64)

    _Encodes HTTP Basic credentials._

    _Tags: encoding, auth_

- [bytes](https://github.com/tokio-rs/bytes) _v1_ [📄](https://docs.rs/bytes)

    _Cheaply cloned byte buffers for raw response bodies kept as evidence and for replayable request bodies._

    _Tags: bytes, buffers_

- [futures-core](https://github.com/rust-lang/futures-rs) _v0.3_ [📄](https://docs.rs/futures-core)

    _The `Stream` trait behind `LiveBody`, the live response body handed to codecs._

    _Tags: async, streams_

- [futures-util](https://github.com/rust-lang/futures-rs) _v0.3_ [📄](https://docs.rs/futures-util)

    _Default features off; `stream::unfold` adapts the HTTP library's body reads into the `LiveBody` stream._

    _Tags: async, streams_

- [http](https://github.com/hyperium/http) _v1_ [📄](https://docs.rs/http)

    _`Method`, `StatusCode`, and `HeaderMap` for request parts and metadata; repeated headers stay repeated._

    _Tags: http, types_

- [reqwest](https://github.com/seanmonstar/reqwest) _v0.13_ [📄](https://docs.rs/reqwest)

    _The HTTP client every managed send uses. Default features off, `rustls` only (aws-lc-rs provider, platform verifier), the same TLS stack the generated crate already used. The runtime always builds it with redirect following disabled and never accepts one from a caller. Without the `system-proxy` feature it still honors `HTTP_PROXY`, `HTTPS_PROXY`, and `ALL_PROXY`, as generated clients did._

    _Tags: http, client, async_

- [rustls](https://github.com/rustls/rustls) _v0.23_ [📄](https://docs.rs/rustls)

    _Default features off; only its error type is used, to classify a failed handshake as `Tls` without reading the library's error text._

    _Tags: tls_

- [serde](https://github.com/serde-rs/serde) _v1_ [📄](https://serde.rs)

    _`DeserializeOwned`, the bound of the `Json<T>` codec._

    _Tags: serialization_

- [serde_json](https://github.com/serde-rs/json) _v1_ [📄](https://docs.rs/serde_json)

    _Parses JSON bodies in the `Json<T>` codec; a successful parse is the only validity check._

    _Tags: json, serialization_

- [tokio](https://github.com/tokio-rs/tokio) _v1_ [📄](https://docs.rs/tokio)

    _`time` and `sync` features: the monotonic `Instant` of the shared `Deadline`, and `timeout_at` bounding each send's head wait and body reads by one expiration; an async `Mutex` so the partitions of one pager session send its open request once._

    _Tags: async, time, sync_

- [url](https://github.com/servo/rust-url) _v2_ [📄](https://docs.rs/url)

    _URL parsing and component slicing for redacted request and `Location` URLs._

    _Tags: url, parsing_

### Development Dependencies

- [tokio](https://github.com/tokio-rs/tokio) _v1_ [📄](https://docs.rs/tokio)

    _Adds `macros`, `rt`, `rt-multi-thread`, `sync`, `net`, `io-util`, and `test-util` for async tests and raw-socket mocks (cut-off bodies, stalled heads, connection counting, proxy routing); `test-util` pauses time so expiration tests run instantly._

    _Tags: async, testing_

- [futures-util](https://github.com/rust-lang/futures-rs) _v0.3_ [📄](https://docs.rs/futures-util)

    _Builds in-memory body streams for `Decode` tests._

    _Tags: async, testing_

- [wiremock](https://github.com/LukeMathWalker/wiremock-rs) _v0.6_ [📄](https://docs.rs/wiremock)

    _Local HTTP mock that records what each managed send put on the wire._

    _Tags: testing, http, mock_

- [rcgen](https://github.com/rustls/rcgen) _v0.13_ [📄](https://docs.rs/rcgen)

    _Generates a test CA, a CA-signed leaf, and a self-signed leaf for the local TLS mock._

    _Tags: testing, tls, certificates_

- [tokio-rustls](https://github.com/rustls/tokio-rustls) _v0.26_ [📄](https://docs.rs/tokio-rustls)

    _Default features off, `aws_lc_rs` and `tls12`: the local TLS mock's acceptor._

    _Tags: testing, tls_

- [rustls](https://github.com/rustls/rustls) _v0.23_ [📄](https://docs.rs/rustls)

    _Adds `aws_lc_rs` and `std` for the local TLS mock's server configuration._

    _Tags: testing, tls_

- [serde](https://github.com/serde-rs/serde) _v1_ [📄](https://serde.rs)

    _Adds `derive` for test success and error body types._

    _Tags: testing, serialization_

- [trybuild](https://github.com/dtolnay/trybuild) _v1.0_ [📄](https://docs.rs/trybuild)

    _Compile-fail cases checked against their expected compiler diagnostic. It runs Cargo, so its one test is `slow_` and runs through `just test-compile-fail`, not the archive-based L1 run._

    _Tags: testing, compile-fail_

- [biscuit-test-harness](../../biscuit-test-harness) _v0.1.0_

    _`manifest_dir!()` resolves the layout gate's and the compile-fail cases' paths at run time._

    _Tags: workspace, testing_

- [test-toolkit](../../tools/test-toolkit) _v0.1.0_

    _The `test_layout` gate for the consolidated `l1` binary, and `EnvGuard` to disable a caching compiler wrapper that rewrites diagnostic paths and to isolate credential environment variables._

    _Tags: workspace, testing_

---

## schematic-schema (Generated)

> Note: This package is auto-generated by schematic-gen. Do not edit manually.

### Production Dependencies

- [schematic-runtime](../runtime) _v0.1.0_

    _Sends every generated REST request (path dependency, re-exported as `schematic_schema::schematic_runtime`); the crate has no HTTP client of its own._

    _Tags: workspace, http_

- [serde](https://github.com/serde-rs/serde) _v1.0_ [📄](https://serde.rs)

    _Industry-standard serialization framework providing derive macros for automatic trait implementation._

    _Tags: serialization, json_

- [serde_json](https://github.com/serde-rs/json) _v1.0_

    _Fast JSON serialization/deserialization using serde._

    _Tags: json, serialization_

- [thiserror](https://github.com/dtolnay/thiserror) _v2.0_

    _Derives the WebSocket error type in `ws_shared.rs`; REST errors are the runtime's._

    _Tags: errors, macros_

- [tokio](https://github.com/tokio-rs/tokio) _v1.43_ [📄](https://tokio.rs/)

    _Asynchronous runtime providing multithreaded task scheduler, reactor, and async I/O primitives._

    _Tags: async, runtime, concurrency_

- [schematic-define](../define) _v0.1.0_

    _Local dependency used only by the WebSocket modules (`Headers`, `ApiKeyEnv`, `EnvMapping`, `EnvList`, and the `websocket` types); the REST modules take their runtime types from `schematic-runtime`._

    _Tags: workspace_

- [schematic-definitions](../definitions) _v0.1.0_

    _Local dependency for the request and response types the generated clients import._

    _Tags: workspace_

- [bytes](https://github.com/tokio-rs/bytes) _v1_ [📄](https://docs.rs/bytes)

    _Byte buffers returned by binary response methods._

    _Tags: bytes, io_

- [urlencoding](https://github.com/kornelski/rust_urlencoding) _v2.1_ [📄](https://docs.rs/urlencoding)

    _Percent-encodes path parameters, and the WebSocket modules' query pairs; the runtime encodes REST query pairs._

    _Tags: url, encoding_

- [futures-util](https://github.com/rust-lang/futures-rs) _v0.3_

    _Stream combinators for the WebSocket clients._

    _Tags: async, streams_

- [tokio-stream](https://github.com/tokio-rs/tokio) _v0.1_

    _Stream adapters for the WebSocket clients._

    _Tags: async, streams_

- [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite) _v0.26_ [📄](https://docs.rs/tokio-tungstenite)

    _WebSocket transport (`connect`, `rustls-tls-native-roots`) for the generated WebSocket clients._

    _Tags: websocket, async_

### Development Dependencies

- [wiremock](https://github.com/LukeMathWalker/wiremock-rs) _v0.6_ [📄](https://docs.rs/wiremock)

    _Local mock HTTP server for tests of the generated clients._

    _Tags: testing, http_

- [serial_test](https://github.com/palfrey/serial_test) _v3_

    _Serializes tests that share process state._

    _Tags: testing_

- [trybuild](https://github.com/dtolnay/trybuild) _v1.0_ [📄](https://docs.rs/trybuild)

    _Compile-fail cases of the generated pager surface, run on the real generated endpoints and checked against their expected diagnostics. The generator writes this dev-dependency into the crate's manifest._

    _Tags: testing, compile-fail_

- `schematic-gen` uses `biscuit-hash` with the `blake3` feature as a dev dependency for canonical definition and schema baseline fingerprints. This adds no runtime dependency or new external crate.
