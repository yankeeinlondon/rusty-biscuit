# Research CLI Dependencies

## Structure

- `research/cli/Cargo.toml` - Research CLI application

## Production Dependencies

### CLI & Argument Parsing

- [clap](https://github.com/clap-rs/clap) _v4.5.53_ [📄](https://docs.rs/clap)

    _Command-line argument parser with derive API for declarative CLI definitions._

    _Tags: cli, argument-parsing_

### Configuration

- [dotenvy](https://github.com/allan2/dotenvy) _v0.15.7_

    _Well-maintained fork of dotenv for loading environment variables from .env files._

    _Tags: environment, configuration, dotenv_

### Async Runtime

- [tokio](https://github.com/tokio-rs/tokio) _v1.48.0_ [📄](https://tokio.rs/)

    _Asynchronous runtime providing multithreaded task scheduler, reactor, and async I/O primitives for TCP, UDP, and timers._

    _Tags: async, runtime, concurrency, io_

### Logging & Tracing

- [tracing](https://github.com/tokio-rs/tracing) _v0.1_

    _Structured, async-aware logging framework with spans and events._

    _Tags: logging, tracing, observability_

- [tracing-subscriber](https://github.com/tokio-rs/tracing) _v0.3_

    _Utilities for implementing and composing tracing subscribers._

    _Tags: logging, tracing, formatting_

### Text-to-Speech

- [tts](https://crates.io/crates/tts) _v0.26.3_

    _Cross-platform text-to-speech library supporting Windows, Linux, macOS, iOS, Android, and WebAssembly._

    _Tags: tts, audio, accessibility_

## Workspace Dependencies

- [research-lib](../lib) _v0.1.0_

    _Core library for research operations._

    _Tags: workspace, library_
