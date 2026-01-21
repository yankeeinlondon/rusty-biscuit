# Project Dependencies

## Structure

This is a Rust workspace with the following modules:

- `Cargo.toml` - Root workspace configuration
- `research/cli/Cargo.toml` - Research CLI application
- `research/lib/Cargo.toml` - Research library with development binary
- `shared/Cargo.toml` - Shared utilities library
- `tui/Cargo.toml` - TUI module (no dependencies yet)

## Workspace Packages

- [research-cli](./research/cli) _v0.1.0_

    _CLI application for research workflows._

    _Tags: workspace, cli_

- [research-lib](./research/lib) _v0.1.0_

    _Core library for research operations._

    _Tags: workspace, library_

 - [shared](./shared) _v0.1.0_

     _Shared utilities and tools._

     _Tags: workspace, library_

 - [tui](./tui) _v0.1.0_

     _Terminal user interface module._

     _Tags: workspace, tui_

- [tree-hugger-lib](./tree-hugger/lib) _v0.1.0_

     _Tree-sitter based library for symbol and diagnostic extraction._

     _Tags: workspace, library_

- [tree-hugger-cli](./tree-hugger/cli) _v0.1.0_

     _CLI tool for exploring symbols, imports, and exports._

     _Tags: workspace, cli_

## Production Dependencies

### AI & LLM

- [rig-core](https://github.com/0xplaygrounds/rig) _v0.27.0_ [📄](https://docs.rig.rs/)

    _Opinionated library for building modular and scalable LLM-powered applications with abstractions for completion models, embeddings, and RAG systems._

    _Tags: llm, ai, agents, rag_

### Async & Concurrency

- [futures](https://github.com/rust-lang/futures-rs) _v0.3_

    _Zero-cost asynchronous programming library providing Stream trait and async utilities._

    _Tags: async, futures, streams_

- [tokio](https://github.com/tokio-rs/tokio) _v1.48.0_ [📄](https://tokio.rs/)

    _Asynchronous runtime providing multithreaded task scheduler, reactor, and async I/O primitives for TCP, UDP, and timers._

    _Tags: async, runtime, concurrency, io_

### CLI & Terminal

- [clap](https://github.com/clap-rs/clap) _v4.5.53_ [📄](https://docs.rs/clap)

    _Command-line argument parser with derive API for declarative CLI definitions._

    _Tags: cli, argument-parsing_

- [inquire](https://github.com/mikaelmello/inquire) _v0.9_

    _Library for building interactive CLI prompts with support for text, select, multiselect, date, editor, and password prompts._

    _Tags: cli, interactive, prompts_

- [tts](https://crates.io/crates/tts) _v0.26.3_

    _Cross-platform text-to-speech library supporting Windows, Linux, macOS, iOS, Android, and WebAssembly._

    _Tags: tts, audio, accessibility_

### Configuration & Environment

- [dotenvy](https://github.com/allan2/dotenvy) _v0.15.7_

    _Well-maintained fork of dotenv for loading environment variables from .env files._

    _Tags: environment, configuration, dotenv_

### Date & Time

- [chrono](https://github.com/chronotope/chrono) _v0.4_

    _Date and time library providing timezone-aware types and operations in the proleptic Gregorian calendar._

    _Tags: date, time, timezone_

### Error Handling

- [thiserror](https://github.com/dtolnay/thiserror) _v2.0_

    _Derive macro for std::error::Error trait._

    _Tags: errors, macros_

### Filesystem

- [dirs](https://github.com/dirs-dev/dirs-rs) _v5.0_

    _Library providing platform-specific standard directories for config, cache, and data on Linux, Windows, and macOS._

    _Tags: filesystem, directories, paths_

### Hashing

- [biscuit-hash](./biscuit-hash) _v0.1.0_

    _Local library providing xxHash content hashing for file fingerprinting._

    _Tags: hashing, xxhash, local_

### Git

- [ignore](https://github.com/BurntSushi/ripgrep/tree/master/crates/ignore) _v0.4_

    _Glob matching library with .gitignore support for file filtering._

    _Tags: git, ignore, glob_

### Parsing

- [tree-sitter](https://github.com/tree-sitter/tree-sitter) _v0.26.3_

    _Incremental parsing system for creating syntax trees of source code._

    _Tags: parsing, syntax-tree, incremental_

- [tree-sitter-rust](https://github.com/tree-sitter/tree-sitter-rust) _v0.24.0_

    _Rust grammar for tree-sitter._

    _Tags: parsing, rust, grammar_

- [tree-sitter-javascript](https://github.com/tree-sitter/tree-sitter-javascript) _v0.25.0_

    _JavaScript grammar for tree-sitter._

    _Tags: parsing, javascript, grammar_

- [tree-sitter-typescript](https://github.com/tree-sitter/tree-sitter-typescript) _v0.23.2_

    _TypeScript grammar for tree-sitter._

    _Tags: parsing, typescript, grammar_

- [tree-sitter-go](https://github.com/tree-sitter/tree-sitter-go) _v0.25.0_

    _Go grammar for tree-sitter._

    _Tags: parsing, go, grammar_

- [tree-sitter-python](https://github.com/tree-sitter/tree-sitter-python) _v0.25.0_

    _Python grammar for tree-sitter._

    _Tags: parsing, python, grammar_

- [tree-sitter-java](https://github.com/tree-sitter/tree-sitter-java) _v0.23.5_

    _Java grammar for tree-sitter._

    _Tags: parsing, java, grammar_

- [tree-sitter-php](https://github.com/tree-sitter/tree-sitter-php) _v0.24.2_

    _PHP grammar for tree-sitter._

    _Tags: parsing, php, grammar_

- [tree-sitter-perl](https://github.com/ganezdragon/tree-sitter-perl) _v1.1.2_

    _Perl grammar for tree-sitter._

    _Tags: parsing, perl, grammar_

- [tree-sitter-bash](https://github.com/tree-sitter/tree-sitter-bash) _v0.25.1_

    _Bash grammar for tree-sitter._

    _Tags: parsing, bash, grammar_

- [tree-sitter-zsh](https://github.com/zsh-users/tree-sitter-zsh) _v0.52.0_

    _Zsh grammar for tree-sitter._

    _Tags: parsing, zsh, grammar_

- [tree-sitter-c](https://github.com/tree-sitter/tree-sitter-c) _v0.24.1_

    _C grammar for tree-sitter._

    _Tags: parsing, c, grammar_

- [tree-sitter-cpp](https://github.com/tree-sitter/tree-sitter-cpp) _v0.23.4_

    _C++ grammar for tree-sitter._

    _Tags: parsing, cpp, grammar_

- [tree-sitter-c-sharp](https://github.com/tree-sitter/tree-sitter-c-sharp) _v0.23.1_

    _C# grammar for tree-sitter._

    _Tags: parsing, c-sharp, grammar_

- [tree-sitter-swift](https://github.com/alex-pinkus/tree-sitter-swift) _v0.7.1_

    _Swift grammar for tree-sitter._

    _Tags: parsing, swift, grammar_

- [tree-sitter-scala](https://github.com/tree-sitter/tree-sitter-scala) _v0.24.0_

    _Scala grammar for tree-sitter._

    _Tags: parsing, scala, grammar_

- [tree-sitter-lua](https://github.com/Azganoth/tree-sitter-lua) _v0.4.1_

    _Lua grammar for tree-sitter._

    _Tags: parsing, lua, grammar_

### HTTP & Web

- [reqwest](https://github.com/seanmonstar/reqwest) _v0.12_

    _Convenient HTTP client with async/blocking support, JSON, proxies, cookies, and TLS._

    _Tags: http, client, async_

- [scraper](https://github.com/rust-scraper/scraper) _v0.20_

    _HTML parsing and querying with CSS selectors built on html5ever._

    _Tags: html, parsing, web-scraping_

- [url](https://github.com/servo/rust-url) _v2.5.0_

    _Implementation of the URL Standard for parsing and manipulating URLs._

    _Tags: url, parsing, web_

### Logging & Tracing

- [tracing](https://github.com/tokio-rs/tracing) _v0.1_

    _Structured, async-aware logging framework with spans and events._

    _Tags: logging, tracing, observability_

- [tracing-subscriber](https://github.com/tokio-rs/tracing) _v0.3_

    _Utilities for implementing and composing tracing subscribers._

    _Tags: logging, tracing, formatting_

### Markdown

- [pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark) _v0.13.0_

    _Efficient CommonMark/Markdown parser using pull-parsing approach._

    _Tags: markdown, parsing_

- [pulldown-cmark-to-cmark](https://github.com/Byron/pulldown-cmark-to-cmark) _v22.0.0_

    _Converts pulldown-cmark Events back to markdown strings, enabling markdown transformation filters._

    _Tags: markdown, serialization, filters_

### Serialization

- [serde](https://github.com/serde-rs/serde) _v1.0_ [📄](https://serde.rs)

    _Industry-standard serialization framework providing derive macros for automatic trait implementation._

    _Tags: serialization, json_

- [serde_json](https://github.com/serde-rs/json) _v1.0_

    _Fast JSON serialization/deserialization using serde._

    _Tags: json, serialization_

## Development Dependencies

### Testing

- [tempfile](https://github.com/Stebalien/tempfile) _v3.15_

    _Secure cross-platform temporary file/directory creation with automatic cleanup._

    _Tags: testing, filesystem, cleanup_

- [tracing-test](https://crates.io/crates/tracing-test) _v0.2_

    _Helper macros for testing tracing output with automatic subscriber initialization and log assertions._

    _Tags: testing, tracing, logging_

- [wiremock](https://github.com/LukeMathWalker/wiremock-rs) _v0.6_

    _HTTP mocking library for black-box testing of applications that interact with third-party APIs._

    _Tags: testing, http, mocking_
