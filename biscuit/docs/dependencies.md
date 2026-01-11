# Shared Library Dependencies

## Structure

- `shared/Cargo.toml` - Shared utilities library

## Production Dependencies

### AI & LLM

- [rig-core](https://github.com/0xplaygrounds/rig) _v0.27_ [📄](https://docs.rig.rs/)

    _Opinionated library for building modular and scalable LLM-powered applications with abstractions for completion models, embeddings, and RAG systems._

    _Tags: llm, ai, agents, rag_

### Async Runtime

- [tokio](https://github.com/tokio-rs/tokio) _v1.48_ [📄](https://tokio.rs/)

    _Asynchronous runtime providing multithreaded task scheduler, reactor, and async I/O primitives for TCP, UDP, and timers._

    _Tags: async, runtime, concurrency, io_

### Error Handling

- [thiserror](https://github.com/dtolnay/thiserror) _v2.0_

    _Derive macro for std::error::Error trait._

    _Tags: errors, macros_

### HTTP & Web

- [reqwest](https://github.com/seanmonstar/reqwest) _v0.12_

    _Convenient HTTP client with async/blocking support, JSON, proxies, cookies, and TLS._

    _Tags: http, client, async_

- [scraper](https://github.com/rust-scraper/scraper) _v0.20_

    _HTML parsing and querying with CSS selectors built on html5ever._

    _Tags: html, parsing, web-scraping_

- [url](https://github.com/servo/rust-url) _v2.5_

    _Implementation of the URL Standard for parsing and manipulating URLs._

    _Tags: url, parsing, web_

### Logging & Tracing

- [tracing](https://github.com/tokio-rs/tracing) _v0.1_

    _Structured, async-aware logging framework with spans and events._

    _Tags: logging, tracing, observability_

### Serialization

- [serde](https://github.com/serde-rs/serde) _v1.0_ [📄](https://serde.rs)

    _Industry-standard serialization framework providing derive macros for automatic trait implementation._

    _Tags: serialization, json_

- [serde_json](https://github.com/serde-rs/json) _v1.0_

    _Fast JSON serialization/deserialization using serde._

    _Tags: json, serialization_

### Text-to-Speech

- [tts](https://crates.io/crates/tts) _v0.26.3_

    _Cross-platform text-to-speech library supporting Windows, Linux, macOS, iOS, Android, and WebAssembly._

    _Tags: tts, audio, accessibility_

## Development Dependencies

### Testing

- [tokio](https://github.com/tokio-rs/tokio) _v1.48_ [📄](https://tokio.rs/)

    _Asynchronous runtime with full feature set and test utilities for testing async code._

    _Tags: async, runtime, testing_

- [tracing-subscriber](https://github.com/tokio-rs/tracing) _v0.3_

    _Utilities for implementing and composing tracing subscribers._

    _Tags: logging, tracing, formatting_

- [tracing-test](https://crates.io/crates/tracing-test) _v0.2_

    _Helper macros for testing tracing output with automatic subscriber initialization and log assertions._

    _Tags: testing, tracing, logging_

- [wiremock](https://github.com/LukeMathWalker/wiremock-rs) _v0.6_

    _HTTP mocking library for black-box testing of applications that interact with third-party APIs._

    _Tags: testing, http, mocking_
