# Project Dependencies

## Structure

This is a Rust workspace with the following modules:

- `Cargo.toml` - Root workspace configuration
- `biscuit-file/lib/Cargo.toml` - File format utilities (PDF, TOML, YAML)
- `biscuit-file/cli/Cargo.toml` - File utilities CLI (`bf`)
- `biscuit-hash/lib/Cargo.toml` - Hashing library (xxHash, BLAKE3, Argon2id)
- `biscuit-hash/cli/Cargo.toml` - Hashing CLI (`bh`)
- `biscuit-speaks/lib/Cargo.toml` - Cross-platform TTS library
- `biscuit-terminal/lib/Cargo.toml` - Terminal detection, image rendering, diagrams
- `biscuit-terminal/cli/Cargo.toml` - Terminal inspector CLI (`bt`)
- `claudine/lib/Cargo.toml` - Universal hook/event handler for agentic CLIs
- `claudine/cli/Cargo.toml` - Hook manager CLI (`claudine`)
- `darkmatter/lib/Cargo.toml` - Markdown parsing, rendering, syntax highlighting
- `darkmatter/cli/Cargo.toml` - Markdown renderer CLI (`md`)
- `homelab/lib/Cargo.toml` - Homelab device control library
- `homelab/cli/Cargo.toml` - Homelab CLI (`homey`)
- `model-citizen/lib/Cargo.toml` - Local LLM model management library
- `model-citizen/cli/Cargo.toml` - Model management CLI (`model`)
- `unchained-ai/model_id/Cargo.toml` - Proc-macro for model ID derivation
- `playa/lib/Cargo.toml` - Audio playback with host player detection
- `playa/cli/Cargo.toml` - Audio player CLI (`playa`)
- `queue/lib/Cargo.toml` - TUI command scheduler library
- `queue/cli/Cargo.toml` - TUI command scheduler CLI (`queue`)
- `research/lib/Cargo.toml` - AI-powered library research
- `research/cli/Cargo.toml` - Research CLI (`research`)
- `schematic/define/Cargo.toml` - REST API definition primitives
- `schematic/definitions/Cargo.toml` - Pre-built API definitions
- `schematic/gen/Cargo.toml` - REST API code generator CLI (`schematic-gen`)
- `schematic/schema/Cargo.toml` - Generated REST API clients (auto-generated)
- `sniff/lib/Cargo.toml` - System discovery (OS, hardware, network, programs)
- `sniff/cli/Cargo.toml` - System discovery CLI (`sniff`)
- `biscuit-speaks/cli/Cargo.toml` - TTS CLI (`so-you-say`)
- `tabby/Cargo.toml` - Future module (no dependencies)
- `tabby/ui/Cargo.toml` - Future UI module
- `tree-hugger/lib/Cargo.toml` - Tree-sitter symbol extraction (16 languages)
- `tree-hugger/cli/Cargo.toml` - Symbol extraction CLI (`hug`)
- `tui/Cargo.toml` - Future TUI module (no dependencies)
- `unchained-ai/lib/Cargo.toml` - LLM pipeline primitives and provider integrations
- `unchained-ai/gen/Cargo.toml` - Provider model enum generator (`gen-models`)
- `unchained-ai/cli/Cargo.toml` - Future AI CLI (`unchained`)

## Workspace Packages

- [biscuit-file](./biscuit-file) _v0.1.0_

    _File format utilities for PDF, TOML, and YAML with multiple PDF backends._

    _Tags: workspace, library, files, pdf_

- [biscuit-file-cli](./biscuit-file/cli) _v0.1.0_

    _CLI tool for file format inspection and conversion._

    _Tags: workspace, cli, files_

- [biscuit-hash](./biscuit-hash) _v0.1.0_

    _Hashing trifecta: xxHash (fast non-crypto), BLAKE3 (fast crypto), Argon2id (passwords)._

    _Tags: workspace, library, hashing_

- [biscuit-hash-cli](./biscuit-hash/cli) _v0.1.0_

    _CLI tool for hashing files and strings._

    _Tags: workspace, cli, hashing_

- [biscuit-speaks](./biscuit-speaks) _v0.1.0_

    _Cross-platform TTS with multi-provider support (ElevenLabs, Say, eSpeak, Kokoro)._

    _Tags: workspace, library, tts, audio_

- [biscuit-terminal](./biscuit-terminal) _v0.1.0_

    _Terminal detection, image rendering (viuer), mermaid diagrams, and capability queries._

    _Tags: workspace, library, terminal_

- [biscuit-terminal-cli](./biscuit-terminal/cli) _v0.1.0_

    _Terminal inspector and diagram renderer CLI._

    _Tags: workspace, cli, terminal_

- [claudine](./claudine) _v0.1.0_

    _Universal hook/event handler for agentic CLIs (Claude, Codex, Gemini, Goose, etc.)._

    _Tags: workspace, library, hooks, events_

- [claudine-cli](./claudine/cli) _v0.1.0_

    _Hook manager CLI for agentic tool integration._

    _Tags: workspace, cli, hooks_

- [darkmatter](./darkmatter) _v0.1.0_

    _Markdown parsing, rendering, mermaid diagrams, and syntax highlighting._

    _Tags: workspace, library, markdown, rendering_

- [darkmatter-cli](./darkmatter/cli) _v0.1.0_

    _Themed markdown renderer for terminal and HTML output._

    _Tags: workspace, cli, markdown_

- [homelab](./homelab) _v0.1.0_

    _Homelab device control library for AV receivers and smart home devices._

    _Tags: workspace, library, homelab, iot_

- [homey](./homelab/cli) _v0.1.0_

    _Homelab device control CLI._

    _Tags: workspace, cli, homelab_

- [model-citizen](./model-citizen) _v0.1.0_

    _Local LLM model management across Ollama, LM Studio, and Llama.cpp._

    _Tags: workspace, library, llm, models_

- [model-citizen-cli](./model-citizen/cli) _v0.1.0_

    _CLI for managing local LLM models._

    _Tags: workspace, cli, llm_

- [model_id](./model_id) _v0.1.0_

    _Proc-macro for deriving model ID types._

    _Tags: workspace, proc-macro_

- [playa](./playa) _v0.1.0_

    _Audio playback via host CLI players with format detection and 53 embedded sound effects._

    _Tags: workspace, library, audio, playback_

- [playa-cli](./playa/cli) _v0.1.0_

    _Audio player CLI with sound effects._

    _Tags: workspace, cli, audio_

- [queue](./queue) _v0.1.0_

    _TUI command scheduler core library with persistence and async execution._

    _Tags: workspace, library, tui, scheduler_

- [queue-cli](./queue/cli) _v0.1.0_

    _TUI-based command scheduler application._

    _Tags: workspace, cli, tui_

- [research](./research) _v0.1.0_

    _AI-powered library research with two-phase LLM pipeline._

    _Tags: workspace, library, ai, research_

- [research-cli](./research/cli) _v0.1.0_

    _CLI for AI-powered library research._

    _Tags: workspace, cli, research_

- [schematic-define](./schematic/define) _v0.1.0_

    _REST API definition primitives (RestApi, Endpoint, AuthStrategy)._

    _Tags: workspace, library, api, codegen_

- [schematic-definitions](./schematic/definitions) _v0.1.0_

    _Pre-built API definitions (OpenAI, Ollama, ElevenLabs, HuggingFace, etc.)._

    _Tags: workspace, library, api, definitions_

- [schematic-gen](./schematic/gen) _v0.1.0_

    _Code generator for REST API client code from schematic definitions._

    _Tags: workspace, cli, codegen_

- [schematic-schema](./schematic/schema) _v0.1.0_

    _Generated REST API clients (auto-generated, do not edit)._

    _Tags: workspace, library, api, generated_

- [sniff](./sniff) _v0.1.0_

    _System discovery: OS, hardware, network interfaces, programs, and services._

    _Tags: workspace, library, system, detection_

- [sniff-cli](./sniff/cli) _v0.1.0_

    _System discovery CLI._

    _Tags: workspace, cli, system_

- [biscuit-speaks-cli](./biscuit-speaks/cli) _v0.1.0_

    _TTS CLI wrapping biscuit-speaks (binary: `so-you-say`)._

    _Tags: workspace, cli, tts_

- [tree-hugger](./tree-hugger) _v0.1.0_

    _Tree-sitter based symbol extraction for 16 programming languages._

    _Tags: workspace, library, parsing, symbols_

- [tree-hugger-cli](./tree-hugger/cli) _v0.1.0_

    _CLI for exploring symbols, imports, and exports._

    _Tags: workspace, cli, parsing_

- [unchained-ai](./unchained-ai) _v0.1.0_

    _LLM pipeline primitives, provider registry, model catalogs, and rig-core integration._

    _Tags: workspace, library, ai, llm_

- [unchained-ai-gen](./unchained-ai/gen) _v0.1.0_

    _Provider model enum generator binary._

    _Tags: workspace, cli, codegen_

## Production Dependencies

### AI & LLM

- [rig-core](https://github.com/0xplaygrounds/rig) _v0.29.0_ [📄](https://docs.rig.rs/)

    _Opinionated library for building modular and scalable LLM-powered applications with abstractions for completion models, embeddings, and RAG systems._

    _Tags: llm, ai, agents, rag_

### Async & Concurrency

- [async-trait](https://github.com/dtolnay/async-trait) _v0.1_ [📄](https://docs.rs/async-trait)

    _Type erasure for async trait methods enabling async fn in dyn traits. Provides #[async_trait] macro for traits and impls._

    _Tags: async, traits_

- [futures](https://github.com/rust-lang/futures-rs) _v0.3_ [📄](https://docs.rs/futures)

    _Zero-cost asynchronous programming library providing Stream trait and async utilities._

    _Tags: async, futures, streams_

- [rayon](https://github.com/rayon-rs/rayon) _v1.11.0_

    _Data parallelism library using work-stealing thread pool for parallel processing._

    _Tags: parallelism, performance, concurrency_

- [tokio](https://github.com/tokio-rs/tokio) _v1.49.0_ [📄](https://tokio.rs/)

    _Asynchronous runtime providing multithreaded task scheduler, reactor, and async I/O primitives for TCP, UDP, and timers._

    _Tags: async, runtime, concurrency, io_

### CLI & Terminal

- [clap](https://github.com/clap-rs/clap) _v4.5.54_ [📄](https://docs.rs/clap)

    _Command-line argument parser with derive API for declarative CLI definitions._

    _Tags: cli, argument-parsing_

- [clap_complete](https://github.com/clap-rs/clap) _v4.5_ [📄](https://docs.rs/clap_complete)

    _Generate shell completion scripts for clap Command. Supports bash, zsh, fish, and PowerShell._

    _Tags: cli, completions, shell_

- [color-eyre](https://github.com/eyre-rs/color-eyre) _v0.6_

    _Colorful error reports with panic hooks, backtraces, and span traces._

    _Tags: errors, cli, diagnostics_

- [colored](https://github.com/mackwic/colored) _v3.0_

    _Terminal color and styling library using ANSI escape codes._

    _Tags: terminal, colors, cli_

- [comfy-table](https://github.com/Nukesor/comfy-table) _v7.1_ [📄](https://docs.rs/comfy-table)

    _Build beautiful terminal tables with automatic content wrapping, ANSI styling, and customizable borders._

    _Tags: table, terminal, formatting_

- [crossterm](https://github.com/crossterm-rs/crossterm) _v0.29_ [📄](https://docs.rs/crossterm)

    _Cross-platform terminal manipulation library for colors, cursor movement, keyboard input, and terminal control._

    _Tags: terminal, cli, cross-platform_

- [indicatif](https://github.com/console-rs/indicatif) _v0.17_ [📄](https://docs.rs/indicatif)

    _Progress bar and CLI reporting library with spinners and color support._

    _Tags: cli, progress, terminal_

- [inquire](https://github.com/mikaelmello/inquire) _v0.9_ [📄](https://docs.rs/inquire)

    _Library for building interactive CLI prompts with text, select, multiselect, date, editor, and password prompts._

    _Tags: cli, interactive, prompts_

- [owo-colors](https://github.com/owo-colors/owo-colors) _v4.2_ [📄](https://docs.rs/owo-colors)

    _Zero-allocation no_std-compatible terminal color library. Drop-in replacement for colored with color detection support._

    _Tags: terminal, formatting, colors, no-std_

- [ratatui](https://github.com/ratatui/ratatui) _v0.30_ [📄](https://docs.rs/ratatui)

    _Terminal user interface framework with modular architecture. Provides widgets, layouts, styling for interactive console applications._

    _Tags: tui, terminal, ui_

- [tabled](https://github.com/zhiburt/tabled) _v0.18_ [📄](https://docs.rs/tabled)

    _Pretty print tables of Rust structs and enums with derive macros, builder pattern, and styling._

    _Tags: cli, formatting, tables_

### Configuration & Environment

- [dotenvy](https://github.com/allan2/dotenvy) _v0.15.7_ [📄](https://docs.rs/dotenvy)

    _Well-maintained fork of dotenv for loading environment variables from .env files._

    _Tags: environment, configuration, dotenv_

- [shellexpand](https://github.com/netvl/shellexpand) _v3_ [📄](https://docs.rs/shellexpand)

    _Shell-like variable expansion ($VAR, ${VAR}) and tilde (~) expansion with default value support._

    _Tags: shell, expansion, environment_

### Data Structures

- [bytes](https://github.com/tokio-rs/bytes) _v1_ [📄](https://docs.rs/bytes)

    _Efficient byte buffer types for zero-copy network programming with reference counting._

    _Tags: data-structures, network, buffers_

- [indexmap](https://github.com/indexmap-rs/indexmap) _v2_ [📄](https://docs.rs/indexmap)

    _Hash table with consistent insertion order and fast iteration. Drop-in compatible with HashMap._

    _Tags: data-structures, collections, hashmap_

- [lazy_static](https://github.com/rust-lang-nursery/lazy-static.rs) _v1.5_ [📄](https://docs.rs/lazy_static)

    _Macro for declaring lazily evaluated statics with runtime initialization. Consider std::sync::OnceLock for newer code._

    _Tags: initialization, static, lazy_

### Date & Time

- [chrono](https://github.com/chronotope/chrono) _v0.4.43_ [📄](https://docs.rs/chrono)

    _Date and time library providing timezone-aware types and operations in the proleptic Gregorian calendar._

    _Tags: date, time, timezone_

### Encoding

- [base64](https://github.com/marshallpierce/rust-base64) _v0.22_ [📄](https://docs.rs/base64)

    _Fast base64 encoding/decoding with support for multiple alphabets and URL-safe variants._

    _Tags: encoding, base64, data_

- [percent-encoding](https://github.com/servo/rust-url) _v2.3_ [📄](https://docs.rs/percent-encoding)

    _Percent encoding and decoding for URL components following RFC 3986._

    _Tags: encoding, url, web_

- [urlencoding](https://github.com/bt/rust_urlencoding) _v2.1_ [📄](https://docs.rs/urlencoding)

    _URL percentage encoding library. Treats + as literal during decoding, not as space._

    _Tags: encoding, url, web_

### Error Handling

- [thiserror](https://github.com/dtolnay/thiserror) _v2.0_

    _Derive macro for std::error::Error trait._

    _Tags: errors, macros_

### File Type Detection

- [infer](https://github.com/bojand/infer) _v0.19_ [📄](https://docs.rs/infer)

    _Infer file and MIME type by checking magic number signatures. No magic file database required, supports no_std._

    _Tags: file-type, mime, magic-bytes_

### Filesystem

- [dirs](https://github.com/dirs-dev/dirs-rs) _v6.0_ [📄](https://docs.rs/dirs)

    _Platform-specific standard directories for config, cache, and data on Linux, Windows, and macOS._

    _Tags: filesystem, directories, paths_

- [fs2](https://github.com/danburkert/fs2-rs) _v0.4_ [📄](https://docs.rs/fs2)

    _Cross-platform file locks and file duplication via flock(2) on Unix and LockFile on Windows._

    _Tags: filesystem, locking, io_

- [glob](https://github.com/rust-lang/glob) _v0.3_

    _Pattern matching for file paths supporting glob syntax._

    _Tags: filesystem, patterns_

- [globset](https://github.com/BurntSushi/ripgrep) _v0.4_

    _Cross-platform glob matching library from ripgrep supporting multiple patterns simultaneously._

    _Tags: filesystem, patterns, glob_

- [ignore](https://github.com/BurntSushi/ripgrep) _v0.4_

    _Fast recursive directory iterator respecting .gitignore and file type filters._

    _Tags: filesystem, gitignore, filtering_

- [walkdir](https://github.com/BurntSushi/walkdir) _v2.5_ [📄](https://docs.rs/walkdir)

    _Efficient recursive directory traversal with symlink control and pruning capabilities._

    _Tags: filesystem, directory, traversal_

- [which](https://github.com/harryfei/which-rs) _v7.0_ [📄](https://docs.rs/which)

    _Find executable binaries in PATH, similar to the Unix `which` command._

    _Tags: path, executable, search_

### Git

- [git2](https://github.com/rust-lang/git2-rs) _v0.20.3_ [📄](https://docs.rs/git2)

    _Threadsafe and memory-safe Rust bindings to libgit2 for interoperating with git repositories._

    _Tags: git, vcs, development-tools_

### Hashing

- [argon2](https://github.com/RustCrypto/password-hashes) _v0.5.3_ [📄](https://docs.rs/argon2)

    _Pure Rust implementation of Argon2 password hashing (Argon2d, Argon2i, Argon2id). PHC winner with no_std support._

    _Tags: password, hashing, cryptography_

- [blake3](https://github.com/BLAKE3-team/BLAKE3) _v1.8.3_ [📄](https://docs.rs/blake3)

    _Official Rust implementation of BLAKE3 cryptographic hash with SIMD optimizations and optional multithreading._

    _Tags: hashing, cryptography, blake3_

- [xxhash-rust](https://github.com/DoumanAsh/xxhash-rust) _v0.8.15_ [📄](https://docs.rs/xxhash-rust)

    _Pure Rust implementation of xxHash algorithms (xxh32, xxh64, xxh3) with SIMD optimizations._

    _Tags: hashing, xxhash, performance_

### HTTP & Web

- [reqwest](https://github.com/seanmonstar/reqwest) _v0.13.2_ [📄](https://docs.rs/reqwest)

    _Convenient HTTP client with async/blocking support, JSON, proxies, cookies, and TLS._

    _Tags: http, client, async_

- [scraper](https://github.com/rust-scraper/scraper) _v0.25_ [📄](https://docs.rs/scraper)

    _HTML parsing and querying with CSS selectors built on html5ever._

    _Tags: html, parsing, web-scraping_

- [url](https://github.com/servo/rust-url) _v2.5_ [📄](https://docs.rs/url)

    _Implementation of the URL Standard for parsing and manipulating URLs._

    _Tags: url, parsing, web_

### Image Processing

- [image](https://github.com/image-rs/image) _v0.25_ [📄](https://docs.rs/image)

    _Imaging library providing basic image processing and native Rust encoders/decoders for common formats._

    _Tags: image, processing, encoding_

- [viuer](https://github.com/atanunq/viuer) _v0.11_ [📄](https://docs.rs/viuer)

    _Display images in the terminal using iTerm, Kitty graphics protocols, or lower half blocks._

    _Tags: terminal, image, rendering_

### Internationalization

- [gender_guesser](https://github.com/ozkriff/gender_guesser) _v0.2_ [📄](https://docs.rs/gender_guesser)

    _Guess gender from first names using a Detector struct and name databases._

    _Tags: gender, name, prediction_

- [unic-langid](https://github.com/zbraniecki/unic-locale) _v0.9.6_ [📄](https://docs.rs/unic-langid)

    _Parse, manipulate, and serialize Unicode Language Identifiers (UTS #35)._

    _Tags: i18n, unicode, locale_

### Language Detection

- [ec4rs](https://github.com/TheDaemoness/ec4rs) _v1_ [📄](https://docs.rs/ec4rs)

    _EditorConfig core library in safe Rust for editors, formatters, and style linters._

    _Tags: editorconfig, parsing, development-tools_

- [hyperpolyglot](https://github.com/monkslc/hyperpolyglot) _v0.1.7_ [📄](https://docs.rs/hyperpolyglot)

    _Fast programming language detector based on GitHub's Linguist using filename, extension, and heuristics._

    _Tags: detection, parsing, languages_

### Logging & Tracing

- [tracing](https://github.com/tokio-rs/tracing) _v0.1_ [📄](https://docs.rs/tracing)

    _Structured, async-aware logging framework with spans and events._

    _Tags: logging, tracing, observability_

- [tracing-subscriber](https://github.com/tokio-rs/tracing) _v0.3_ [📄](https://docs.rs/tracing-subscriber)

    _Utilities for implementing and composing tracing subscribers._

    _Tags: logging, tracing, formatting_

### Markdown

- [markdown](https://github.com/wooorm/markdown-rs) _v1.0.0-alpha.22_ [📄](https://docs.rs/markdown)

    _CommonMark compliant parser with AST support and extensions (GFM, MDX, math, frontmatter)._

    _Tags: parser, commonmark, markdown_

- [pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark) _v0.13.0_

    _Efficient CommonMark/Markdown parser using pull-parsing approach._

    _Tags: markdown, parsing_

- [pulldown-cmark-to-cmark](https://github.com/Byron/pulldown-cmark-to-cmark) _v22.0.0_ [📄](https://docs.rs/pulldown-cmark-to-cmark)

    _Converts pulldown-cmark Events back to markdown strings, enabling markdown transformation filters._

    _Tags: markdown, serialization, filters_

### Multimedia

- [symphonia](https://github.com/pdeljanov/Symphonia) _v0.5.5_ [📄](https://docs.rs/symphonia)

    _Pure Rust multimedia format demuxing, tag reading, and audio decoding. Supports 12+ formats with performance within 15% of FFmpeg._

    _Tags: audio, decoding, multimedia_

### OpenAPI & Schema

- [openapiv3](https://github.com/glademiller/openapiv3) _v2_ [📄](https://docs.rs/openapiv3)

    _Data structures representing OpenAPI v3.0.x specification with serde support and 100% round-trip fidelity._

    _Tags: openapi, api, serialization_

- [schemars](https://github.com/GREsau/schemars) _v1_ [📄](https://docs.rs/schemars)

    _Generate JSON Schema documents from Rust code using derive macros. Compatible with serde attributes._

    _Tags: json-schema, serialization, validation_

### PDF

- [lopdf](https://github.com/J-F-Liu/lopdf) _v0.36_ [📄](https://docs.rs/lopdf)

    _PDF document manipulation with object streams support (PDF 1.5+). Handles reading, parsing, and creating PDF files._

    _Tags: pdf, manipulation, document_

- [pdf-extract](https://github.com/jrmuizel/pdf-extract) _v0.7_ [📄](https://docs.rs/pdf-extract)

    _Pure Rust library to extract text content from PDF files._

    _Tags: pdf, text-extraction, parsing_

- [pdfium-render](https://github.com/ajrcarey/pdfium-render) _v0.8_ [📄](https://docs.rs/pdfium-render)

    _High-level Rust wrapper around PDFium (Chromium's PDF library) for rendering and editing. Optional feature._

    _Tags: pdf, rendering, pdfium_

### Platform-Specific

- [coreaudio-sys](https://crates.io/crates/coreaudio-sys) _v0.2_

    _Rust bindings to Apple's Core Audio API. macOS-only, optional for audio ducking._

    _Tags: audio, macos, platform_

- [libc](https://github.com/rust-lang/libc) _v0.2_ [📄](https://docs.rs/libc)

    _Raw FFI bindings to platform APIs and C standard library._

    _Tags: ffi, bindings, system_

- [metal](https://github.com/gfx-rs/metal-rs) _v0.33_ [📄](https://docs.rs/metal)

    _Rust bindings for Apple's Metal 3D Graphics API. macOS-only for GPU detection._

    _Tags: graphics, gpu, macos_

### Proc-Macro Utilities

- [proc-macro2](https://github.com/dtolnay/proc-macro2) _v1.0_ [📄](https://docs.rs/proc-macro2)

    _Substitute implementation of compiler's proc_macro API enabling unit testing._

    _Tags: proc-macro, tokens, compiler_

- [quote](https://github.com/dtolnay/quote) _v1.0_ [📄](https://docs.rs/quote)

    _Quasi-quoting macro for turning Rust syntax trees into source code tokens._

    _Tags: proc-macro, code-generation, tokens_

- [syn](https://github.com/dtolnay/syn) _v2.0_ [📄](https://docs.rs/syn)

    _Parsing library for Rust tokens into syntax trees. Foundation for procedural macros._

    _Tags: ast, parsing, proc-macro_

### Random

- [rand](https://github.com/rust-random/rand) _v0.8_ [📄](https://docs.rs/rand)

    _Random number generators with fast implementations and broad distribution support._

    _Tags: random, rng, generation_

### Regex & Text Processing

- [html-escape](https://github.com/magiclen/html-escape) _v0.2_

    _HTML entity encoding for safe HTML output generation._

    _Tags: html, escaping, security_

- [regex](https://github.com/rust-lang/regex) _v1.11_ [📄](https://docs.rs/regex)

    _Fast regular expression engine with Unicode support and linear-time guarantees._

    _Tags: regex, text-processing_

- [similar](https://github.com/mitsuhiko/similar) _v2.6_ [📄](https://docs.rs/similar)

    _Dependency-free diffing library implementing Myers and Patience algorithms. Supports inline diffs and byte slices._

    _Tags: diff, text, algorithms_

- [textwrap](https://github.com/mgeisler/textwrap) _v0.16_ [📄](https://docs.rs/textwrap)

    _Word wrapping and indenting text with optimal-fit algorithm. Supports Unicode, emojis, and hyphenation._

    _Tags: text, wrapping, formatting_

- [unicode-width](https://crates.io/crates/unicode-width) _v0.2_

    _Determine displayed width of Unicode characters for terminal rendering._

    _Tags: unicode, text, terminal_

### Schema Validation

- [jsonschema](https://github.com/Stranger6667/jsonschema-rs) _v0.28_ [📄](https://docs.rs/jsonschema)

    _High-performance JSON Schema validator with reusable validators and fancy-regex support. Optional feature._

    _Tags: json-schema, validation, json_

### Serialization

- [serde](https://github.com/serde-rs/serde) _v1.0_ [📄](https://serde.rs)

    _Industry-standard serialization framework providing derive macros for automatic trait implementation._

    _Tags: serialization, json_

- [serde_json](https://github.com/serde-rs/json) _v1.0_

    _Fast JSON serialization/deserialization using serde._

    _Tags: json, serialization_

- [serde_yaml](https://github.com/dtolnay/serde-yaml) _v0.9_

    _YAML data format for Serde. DEPRECATED (no longer maintained as of v0.9.34) - migrate to serde_yaml_ng._

    _Tags: yaml, serialization, deprecated_

- [serde_yaml_ng](https://github.com/acatton/serde-yaml-ng) _v0.10_ [📄](https://docs.rs/serde_yaml_ng)

    _Strongly typed YAML library using Serde. Fork and continuation of serde-yaml with YAML 1.1 support._

    _Tags: yaml, serialization, serde_

- [strum](https://github.com/Peternator7/strum) _v0.27_ [📄](https://docs.rs/strum)

    _Derive macros for enums providing string conversion, iteration, and property access._

    _Tags: enums, derive, macros_

- [toml](https://github.com/toml-rs/toml) _v0.9_ [📄](https://docs.rs/toml)

    _Native Rust encoder and decoder of TOML-formatted files with serde integration._

    _Tags: serialization, config, toml_

- [toml_edit](https://github.com/toml-rs/toml) _v0.22_ [📄](https://docs.rs/toml_edit)

    _Format-preserving TOML parser and editor. Preserves comments, whitespace, and item order._

    _Tags: toml, parser, formatting_

### SQL & Database

- [rusqlite](https://github.com/rusqlite/rusqlite) _v0.31_ [📄](https://docs.rs/rusqlite)

    _Ergonomic SQLite bindings for Rust used by Claudine's local reporting index._

    _Tags: database, sqlite, embedded_

- [sqlx](https://github.com/launchbadge/sqlx) _v0.8_ [📄](https://docs.rs/sqlx)

    _Async, pure Rust SQL toolkit with compile-time checked queries. Supports PostgreSQL, MySQL, and SQLite._

    _Tags: database, async, sql_

### Syntax Highlighting

- [syntect](https://github.com/trishume/syntect) _v5.2_ [📄](https://docs.rs/syntect)

    _Syntax highlighting using Sublime Text/TextMate definitions with high performance._

    _Tags: syntax-highlighting, terminal, html_

- [two-face](https://github.com/CosmicHorrorDev/two-face) _v0.5_ [📄](https://docs.rs/two-face)

    _Extra syntect syntaxes and themes curated by bat. Includes TOML, TypeScript, Dockerfile, and more._

    _Tags: syntax, highlighting, themes_

### System Information

- [sysinfo](https://github.com/GuillaumeGomez/sysinfo) _v0.37.2_ [📄](https://docs.rs/sysinfo)

    _Cross-platform library for system information including processes, CPUs, disks, and networks._

    _Tags: system, information, monitoring_

### Terminal Detection & Capabilities

- [supports-hyperlinks](https://github.com/zkat/supports-hyperlinks) _v3.2.0_ [📄](https://docs.rs/supports-hyperlinks)

    _Detect whether the current terminal supports rendering hyperlinks._

    _Tags: terminal, hyperlinks, detection_

- [termbg](https://github.com/dalance/termbg) _v0.6_ [📄](https://docs.rs/termbg)

    _Cross-platform terminal background color detection using xterm sequences or COLORFGBG._

    _Tags: terminal, detection, colors_

- [terminal_size](https://github.com/eminence/terminal-size) _v0.4.3_ [📄](https://docs.rs/terminal_size)

    _Cross-platform terminal dimension detection._

    _Tags: terminal, size, cli_

- [termini](https://github.com/meh/rust-termini) _v1.0_ [📄](https://docs.rs/termini)

    _Minimal terminfo library providing access to terminal capability databases._

    _Tags: terminfo, terminal, capabilities_

### Tree-sitter Grammars

- [tree-sitter](https://github.com/tree-sitter/tree-sitter) _v0.26.3_

    _Incremental parsing system for creating syntax trees of source code._

    _Tags: parsing, syntax-tree, incremental_

- [tree-sitter-bash](https://github.com/tree-sitter/tree-sitter-bash) _v0.25.1_

    _Bash grammar for tree-sitter._

    _Tags: parsing, bash, grammar_

- [tree-sitter-c](https://github.com/tree-sitter/tree-sitter-c) _v0.24.1_

    _C grammar for tree-sitter._

    _Tags: parsing, c, grammar_

- [tree-sitter-c-sharp](https://github.com/tree-sitter/tree-sitter-c-sharp) _v0.23.1_

    _C# grammar for tree-sitter._

    _Tags: parsing, c-sharp, grammar_

- [tree-sitter-cpp](https://github.com/tree-sitter/tree-sitter-cpp) _v0.23.4_

    _C++ grammar for tree-sitter._

    _Tags: parsing, cpp, grammar_

- [tree-sitter-go](https://github.com/tree-sitter/tree-sitter-go) _v0.25.0_

    _Go grammar for tree-sitter._

    _Tags: parsing, go, grammar_

- [tree-sitter-java](https://github.com/tree-sitter/tree-sitter-java) _v0.23.5_

    _Java grammar for tree-sitter._

    _Tags: parsing, java, grammar_

- [tree-sitter-javascript](https://github.com/tree-sitter/tree-sitter-javascript) _v0.25.0_

    _JavaScript grammar for tree-sitter._

    _Tags: parsing, javascript, grammar_

- [tree-sitter-lua](https://github.com/Azganoth/tree-sitter-lua) _v0.4.1_

    _Lua grammar for tree-sitter._

    _Tags: parsing, lua, grammar_

- [tree-sitter-perl](https://github.com/ganezdragon/tree-sitter-perl) _v1.1.2_

    _Perl grammar for tree-sitter._

    _Tags: parsing, perl, grammar_

- [tree-sitter-php](https://github.com/tree-sitter/tree-sitter-php) _v0.24.2_

    _PHP grammar for tree-sitter._

    _Tags: parsing, php, grammar_

- [tree-sitter-python](https://github.com/tree-sitter/tree-sitter-python) _v0.25.0_

    _Python grammar for tree-sitter._

    _Tags: parsing, python, grammar_

- [tree-sitter-rust](https://github.com/tree-sitter/tree-sitter-rust) _v0.24.0_

    _Rust grammar for tree-sitter._

    _Tags: parsing, rust, grammar_

- [tree-sitter-scala](https://github.com/tree-sitter/tree-sitter-scala) _v0.24.0_

    _Scala grammar for tree-sitter._

    _Tags: parsing, scala, grammar_

- [tree-sitter-swift](https://github.com/alex-pinkus/tree-sitter-swift) _v0.7.1_

    _Swift grammar for tree-sitter._

    _Tags: parsing, swift, grammar_

- [tree-sitter-typescript](https://github.com/tree-sitter/tree-sitter-typescript) _v0.23.2_

    _TypeScript grammar for tree-sitter._

    _Tags: parsing, typescript, grammar_

- [tree-sitter-zsh](https://github.com/zsh-users/tree-sitter-zsh) _v0.52.0_

    _Zsh grammar for tree-sitter._

    _Tags: parsing, zsh, grammar_

### URL & Web Utilities

- [open](https://github.com/Byron/open-rs) _v5_ [📄](https://docs.rs/open)

    _Open paths or URLs using default system applications. Nonblocking operation._

    _Tags: system, desktop, urls_

### Versioning

- [semver](https://github.com/dtolnay/semver) _v1.0_ [📄](https://docs.rs/semver)

    _Parser and evaluator for Cargo's flavor of Semantic Versioning._

    _Tags: parsing, versioning_

### Code Generation

- [prettyplease](https://github.com/dtolnay/prettyplease) _v0.2_ [📄](https://docs.rs/prettyplease)

    _Minimal syn syntax tree pretty-printer producing rustfmt-quality code formatting._

    _Tags: formatting, code-generation, syn_

### Network

- [getifaddrs](https://github.com/mmastrac/getifaddrs) _v0.6.0_ [📄](https://docs.rs/getifaddrs)

    _Cross-platform library for retrieving network interface addresses and indices._

    _Tags: network, system_

## Development Dependencies

### Testing

- [assert_cmd](https://github.com/assert-rs/assert_cmd) _v2.0_

    _CLI integration testing helpers for running binaries and asserting on outputs._

    _Tags: testing, cli, integration-tests_

- [insta](https://github.com/mitsuhiko/insta) _v1.41_ [📄](https://insta.rs)

    _Snapshot testing library with VS Code integration and beautiful diffs._

    _Tags: testing, snapshots, tdd_

- [predicates](https://github.com/assert-rs/predicates-rs) _v3.1_

    _Boolean-valued predicate functions for flexible assertions._

    _Tags: testing, assertions, predicates_

- [proptest](https://github.com/proptest-rs/proptest) _v1.5_

    _Property-based testing framework generating arbitrary inputs and shrinking failing cases._

    _Tags: testing, property-testing, fuzzing_

- [serial_test](https://github.com/palfrey/serial_test) _v3.2_

    _Run tests serially using attribute macro to avoid race conditions._

    _Tags: testing, serial-execution, synchronization_

- [tempfile](https://github.com/Stebalien/tempfile) _v3.15_

    _Secure cross-platform temporary file/directory creation with automatic cleanup._

    _Tags: testing, filesystem, cleanup_

- [trybuild](https://github.com/dtolnay/trybuild) _v1.0_ [📄](https://docs.rs/trybuild)

    _Test harness for UI tests of compiler diagnostics, commonly used for procedural macro testing._

    _Tags: testing, development-tools, macros_

- [wiremock](https://github.com/LukeMathWalker/wiremock-rs) _v0.6_ [📄](https://docs.rs/wiremock)

    _HTTP mocking library for black-box testing of applications that interact with third-party APIs._

    _Tags: testing, http, mocking_

### Tracing (Test)

- [tracing-test](https://github.com/tokio-rs/tracing) _v0.2_ [📄](https://docs.rs/tracing-test)

    _Helper macros for testing tracing output with automatic subscriber initialization and log assertions._

    _Tags: testing, tracing, logging_
