# Project Dependencies

## Recent Dependency Notes

- `playa/lib` uses `fs4` for its private cross-process spool locks,
  `biscuit-hash` for stable user/cache fingerprints, `chrono` for protocol
  deadlines, and `windows-sys` for atomic replacement on Windows. `playa-cli`
  uses the `biscuit-file` file-reference and portable-path authorities at the
  CLI boundary; the library remains independent of CLI path syntax.
- `biscuit-speaks/lib`'s optional `playa` feature enables
  `playa/native-playback`; Linux CI therefore provisions `libasound2-dev`
  alongside `espeak-ng`. It uses `biscuit-hash` xxHash for content-addressed
  TTS audio cache names and `fs4` for detached-helper/test coordination. See
  [`biscuit-speaks/docs/dependencies.md`](./biscuit-speaks/docs/dependencies.md).
- Both Claudine crates explicitly enable `playa/native-playback`; their Linux
  native policy includes `libasound2-dev`. Their direct `biscuit-file` and
  `biscuit-hash` edges remain composition/MCP/session authorities rather than
  substitutes for Playa's spool path and fingerprint authorities.
- `unchained-ai/lib` aliases `xpty` as `portable-pty` so the existing
  cross-platform PTY API remains stable while Windows ConPTY sessions avoid
  cursor inheritance, whose open-time handshake cannot be answered through
  `portable-pty` before its pipe handles are returned.
- `messenger/lib` uses `test-toolkit` only as a development dependency so its
  desktop-stub resolver tests restore `MESSENGER_STUB_BIN_DIR` safely while
  serializing process-environment mutation.
- `worktree/lib` uses `biscuit-hash` for the SHA-pair cache file name. The cache
  stores deterministic ahead/behind and clean-merge results under the user cache
  directory, keyed by canonical repo-root xxHash plus branch tip SHAs.
- `claudine/contract` (`claudine-contract`) implements
  `biscuit_contract::inference::InferenceAdapter` over a Claudine
  non-interactive, tool-free agentic-CLI session. It is the one crate that
  depends on **both** `biscuit-contract` and `claudine` (lib); it must not
  depend on `claudine-cli`. Beyond those two it adds `async-trait`, `tokio`,
  `serde_json`, `jsonschema` (`0.42`, the workspace-wide pin, for adapter-owned
  Draft 2020-12 validation), `tempfile` (isolated session CWD), `thiserror`,
  and `tracing`. See
  [`claudine/contract/docs/dependencies.md`](./claudine/contract/docs/dependencies.md).
- `unchained-ai/contract` (`unchained-ai-contract`) implements
  `biscuit_contract::inference::InferenceAdapter` over the `unchained-ai`
  single-turn execution surface and capability-based model resolver. It is the
  one crate that depends on **both** `biscuit-contract` and `unchained-ai`
  (lib); it must not depend on `unchained-ai-cli`. Beyond those two it adds
  `async-trait`, `tokio`, `serde_json`, `jsonschema` (`0.42`, the workspace-wide
  pin, for adapter-owned Draft 2020-12 validation), `thiserror`, and `tracing`.
  See
  [`unchained-ai/contract/docs/dependencies.md`](./unchained-ai/contract/docs/dependencies.md).
- `biscuit-contract/lib` is a provider-neutral inference contract crate. It
  depends only on `async-trait` (object-safety for `Arc<dyn
  InferenceAdapter>`), `serde_json` (JSON Schema + structured payloads), and
  `thiserror` (error impl). `tokio` is permitted in `[dev-dependencies]` only
  for `#[tokio::test]`. See
  [`biscuit-contract/docs/dependencies.md`](./biscuit-contract/docs/dependencies.md)
  for the full rationale and forbidden-class list.
- `biscuit-terminal/lib` reads terminal-app config values via its `app_metadata`
  module. Structured formats (TOML/YAML/JSON5) are parsed through a
  `default-features = false` path dependency on `biscuit-file`
  (`features = ["toml","yaml","json5"]`), which normalizes each to a single
  `serde_json::Value` so one shared dot-path resolver reads them all — the
  underlying `toml` / `serde_yaml_ng` / `json-five` parsers stay *indirect*. The
  only structured parser added **directly** is `plist` (v1), for iTerm2 / Apple
  Terminal XML+binary property lists. It also uses `url` (v2.5) to convert
  filesystem paths into portable OSC8/file-link targets. No dependency cycle:
  `biscuit-file` does not depend on `biscuit-terminal`.
- `biscuit-terminal/cli` adds a path dependency on `sniff/lib` for
  `bt about [APP]` install detection. The library remains sniff-free; this
  dependency is CLI-only._
- `biscuit-file/lib` uses `url` for HTTP(S) file-reference classification and
  gates `reqwest`, `bytes`, and `tokio` behind the off-by-default `fetch`
  feature for policy-enforced HTTP access.
- `darkmatter/lib` enables `biscuit-file/fetch` and uses `reqwest`, `tokio`,
  and `url` for remote URL composition, persistent remote cache revalidation,
  and side-effect `http_post` host-policy enforcement.
- `claudine-gen` uses `biscuit-file`'s `file-reference` feature to resolve
  schema-constrained empirical research fixtures relative to their topic.
- `darkmatter/lib` takes a direct `fancy-regex` dependency (already in the tree
  transitively via `jsonschema`) so SimplifiedSchema pattern-key literal
  precedence (Feature C) can emit negative-lookahead `patternProperties`: such
  schemas opt into `jsonschema`'s backtracking `fancy-regex` engine, while every
  lookaround-free schema stays on the linear (ReDoS-safe) `regex` engine.
- `darkmatter/dmls` (`dmls`) is the Darkmatter Language Server. Protocol
  stack: `lsp-server` (stdio framing, in-memory test connections) +
  `lsp-types` (LSP 3.17 types) + `crossbeam-channel` (the channel family
  `lsp-server` exposes; worker pool later). `line-index` (rust-analyzer's
  crate) backs the source-map module and never leaks past it; `rlsp-yaml-parser`
  (lossless byte-span YAML) backs the Phase-7 `FrontmatterAst` overlay and
  likewise never leaks past it; `url` supplies
  file-URI ↔ path conversion (`lsp-types` 0.97's `Uri` has none); `toml`
  parses `.dmls.toml`; `tracing`/`tracing-subscriber` log to stderr or
  `--log-file` only (stdout is reserved for LSP framing). Workspace discovery
  (Phase 3) uses `ignore` (gitignore-aware walk, symlinks not followed) +
  `globset` (include/exclude globs). Wiki-link resolution (Phase 5 / R-8) uses
  `unicode-normalization` for NFC so a vault resolves identically across OSes.
  Semantic authorities: `darkmatter` (lib),
  `biscuit-file` (`file-reference` feature, default features off), and
  `biscuit-hash` (xxHash content-hash identity for graph invalidation). See
  [`darkmatter/docs/dependencies.md`](./darkmatter/docs/dependencies.md).

## Structure

This is a Rust workspace with the following modules:

- `Cargo.toml` - Root workspace configuration
- `biscuit-contract/lib/Cargo.toml` - Shared provider-neutral inference contract (async-trait, serde_json, thiserror)
- `biscuit-file/lib/Cargo.toml` - File format utilities (PDF, TOML, YAML)
- `biscuit-file/cli/Cargo.toml` - File utilities CLI (`bf`)
- `biscuit-hash/lib/Cargo.toml` - Hashing library (xxHash, BLAKE3, Argon2id)
- `biscuit-hash/cli/Cargo.toml` - Hashing CLI (`bh`)
- `biscuit-icon/lib/Cargo.toml` - Curated offline domain icons + on-demand Iconify lookup (renderable, biscuit-terminal, rusqlite bundled, reqwest, strum)
- `biscuit-icon/cli/Cargo.toml` - Icon CLI (`icon`) (clap, clap_complete unstable-dynamic, color-eyre)
- `biscuit-speaks/lib/Cargo.toml` - Cross-platform TTS library (native-first Playa feature, xxHash audio cache, detached preparation)
- `biscuit-terminal/lib/Cargo.toml` - Terminal detection, image rendering, diagrams
- `biscuit-terminal/cli/Cargo.toml` - Terminal inspector CLI (`bt`)
- `claudine/lib/Cargo.toml` - Universal hook/event handler for agentic CLIs
- `claudine/contract/Cargo.toml` - InferenceAdapter over tool-free agentic-CLI sessions (biscuit-contract, claudine, jsonschema, tempfile)
- `claudine/cli/Cargo.toml` - Hook manager CLI (`claudine`)
- `darkmatter/lib/Cargo.toml` - Markdown parsing, rendering, syntax highlighting
- `darkmatter/cli/Cargo.toml` - Markdown renderer CLI (`md`)
- `darkmatter/dmls/Cargo.toml` - Darkmatter Language Server (`dmls`) (lsp-server, lsp-types, line-index, rlsp-yaml-parser, ignore, globset, unicode-normalization, biscuit-hash)
- `homelab/lib/Cargo.toml` - Homelab device control library
- `homelab/cli/Cargo.toml` - Homelab CLI (`homey`)
- `model-citizen/lib/Cargo.toml` - Local LLM model management library
- `model-citizen/cli/Cargo.toml` - Model management CLI (`model`)
- `unchained-ai/model_id/Cargo.toml` - Proc-macro for model ID derivation
- `playa/lib/Cargo.toml` - Native-first audio playback, host-player fallback, metadata probing, and completion reports
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
- `biscuit-tui/lib/Cargo.toml` - TUI chrome (input components built on Ratatui)
- `biscuit-tui/cli/Cargo.toml` - TUI input CLI (`question`)
- `unchained-ai/lib/Cargo.toml` - LLM pipeline primitives and provider integrations
- `unchained-ai/gen/Cargo.toml` - Provider model enum generator (`gen-models`)
- `unchained-ai/cli/Cargo.toml` - Future AI CLI (`unchained`)
- `worktree/lib/Cargo.toml` - Git worktree business logic (git subprocess orchestration, SHA-pair status cache)
- `worktree/cli/Cargo.toml` - Worktree CLI (`wt`)
- `tools/test-toolkit/Cargo.toml` - Shared test lifecycle helpers
- `biscuit-test-harness/Cargo.toml` - Real-terminal test harness (WezTerm, Kitty, tmux, Apple Terminal)
- `biscuit-browser-harness/Cargo.toml` - Headless browser test harness (Chrome/Chromium)

## Workspace Packages

- [biscuit-contract](./biscuit-contract) _v0.1.0_

    _Shared provider-neutral contract for one text-inference operation. Defines the object-safe `InferenceAdapter` trait, request/response types, and `InferenceError` categories. Depends only on `async-trait`, `serde_json`, and `thiserror`; see [`biscuit-contract/docs/dependencies.md`](./biscuit-contract/docs/dependencies.md)._

    _Tags: workspace, library, contract, ai, inference_

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

- [biscuit-visualized](./biscuit-visualized) _v0.1.0_

    _Shared visualization library for Mermaid diagrams and graph rendering. Pure Rust with no external dependencies._

    _Tags: workspace, library, visualization_

- [biscuit-tui](./biscuit-tui/lib) _v0.1.0_

    _TUI input components (text input, toggle, choice, text area, grid) built on Ratatui. Embeddable widgets and a standalone runner._

    _Tags: workspace, library, tui, inputs_

- [biscuit-tui-cli](./biscuit-tui/cli) _v0.1.0_

    _Interactive prompt CLI (`question`) exposing biscuit-tui components as subcommands with raw/json/null output modes._

    _Tags: workspace, cli, tui, inputs_

- [claudine](./claudine) _v0.1.0_

    _Universal hook/event handler for agentic CLIs (Claude, Codex, Gemini, Goose, etc.)._

    _Tags: workspace, library, hooks, events_

- [claudine-cli](./claudine/cli) _v0.1.0_

    _Hook manager CLI for agentic tool integration. Uses the rendezvous client/core crates on every target to record lifecycle `requeue(...)` deferred-execution entries (UDS on Unix, named pipe on Windows) and falls back to a local durable JSONL queue when the daemon is unreachable._

    _Tags: workspace, cli, hooks_

- [claudine-contract](./claudine/contract) _v0.1.0_

    _`InferenceAdapter` backed by a Claudine non-interactive, tool-free, filesystem-isolated agentic-CLI session. Bridges `biscuit-contract` and `claudine` for deterministic consumers (Reaper, Darkmatter); see [`claudine/contract/docs/dependencies.md`](./claudine/contract/docs/dependencies.md)._

    _Tags: workspace, library, inference, adapter_

- [darkmatter](./darkmatter) _v0.1.0_

    _Markdown parsing, rendering, mermaid diagrams, and syntax highlighting._

    _Tags: workspace, library, markdown, rendering_

- [darkmatter-cli](./darkmatter/cli) _v0.1.0_

    _Themed markdown renderer for terminal and HTML output._

    _Tags: workspace, cli, markdown_

- [dmls](./darkmatter/dmls) _v0.1.0_

    _Darkmatter Language Server: LSP 3.17 over stdio for Markdown, Darkmatter DSL, and SimplifiedSchema frontmatter._

    _Tags: workspace, cli, lsp, markdown_

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

    _Native-first audio playback with host CLI fallback, format and metadata detection, completion reports, and embedded sound effects._

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

- [unchained-ai-contract](./unchained-ai/contract) _v0.1.0_

    _`InferenceAdapter` backed by the `unchained-ai` single-turn execution surface and capability-based model resolver. Bridges `biscuit-contract` and `unchained-ai` for deterministic consumers (Reaper, Darkmatter); see [`unchained-ai/contract/docs/dependencies.md`](./unchained-ai/contract/docs/dependencies.md)._

    _Tags: workspace, library, inference, adapter_

- [messenger](./messenger/lib) _v0.1.0_

    _Unified outbound messaging library for Rust (Discord, Slack, Signal, WhatsApp, Telegram, desktop OS notifications)._

    _Tags: workspace, library, messaging, notifications_

- [messenger-cli](./messenger/cli) _v0.1.0_

    _Messenger CLI binary (`messenger`) with route config, receipts, and interactive setup._

    _Tags: workspace, cli, messaging, notifications_

- [test-toolkit](./tools/test-toolkit) _v0.1.0_

    _Shared test lifecycle helpers, including tracing phase spans and environment-variable guards. The optional `leak-sweep` feature adds a cross-platform post-run orphan-process detector binary (pulls `clap` + `sysinfo`, gated off by default so dev-dependency consumers do not inherit them)._

    _Tags: workspace, library, testing_

- [biscuit-test-harness](./biscuit-test-harness) _v0.1.0_

    _Real-terminal test harness with backends for WezTerm, Kitty, tmux, and Apple Terminal. Provides `SharedHarness`, `TerminalHarness` trait, and capture utilities._

    _Tags: workspace, library, testing, terminal_

- [biscuit-browser-harness](./biscuit-browser-harness) _v0.1.0_

    _Headless browser test harness wrapping `chromiumoxide`. Provides `BrowserHarness` trait, `ChromeHarness` implementation, and skip-clean contract for CI._

    _Tags: workspace, library, testing, browser_

## Production Dependencies

### AI & LLM

- [rig-core](https://github.com/0xplaygrounds/rig) _v0.31.0_ [📄](https://docs.rig.rs/)

    _Opinionated library for building modular and scalable LLM-powered applications with abstractions for completion models, embeddings, and RAG systems._

    _Tags: llm, ai, agents, rag_

### Async & Concurrency

- [async-trait](https://github.com/dtolnay/async-trait) _v0.1_ [📄](https://docs.rs/async-trait)

    _Type erasure for async trait methods enabling async fn in dyn traits. Provides #[async_trait] macro for traits and impls._

    _Tags: async, traits_

- [futures](https://github.com/rust-lang/futures-rs) _v0.3_ [📄](https://docs.rs/futures)

    _Zero-cost asynchronous programming library providing Stream trait and async utilities._

    _Tags: async, futures, streams_

- [dashmap](https://github.com/xacrimon/dashmap) _v6.1_ [📄](https://docs.rs/dashmap)

    _Concurrent hash map with sharded locking, used for low-contention run-local caches in Darkmatter._

    _Tags: concurrency, hashmap, performance_

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

- [heck](https://github.com/withoutboats/heck) _v0.5_ [📄](https://docs.rs/heck)

    _Case conversion helpers used by the `question` CLI naming-convention flags._

    _Tags: cli, strings, case-conversion_

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

- [plist](https://github.com/ebarnard/rust-plist) _v1_ [📄](https://docs.rs/plist)

    _Apple property-list (XML and binary) reader/writer. Used by biscuit-terminal's
    app-metadata value extractor to read iTerm2 / Apple Terminal config settings._

    _Tags: configuration, plist, macos, parsing_

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

- [gix](https://github.com/GitoxideLabs/gitoxide) _v0.84.0_ [📄](https://docs.rs/gix)

    _Pure-Rust Git repository inspection library (status, diff, history, refs, remotes, config, worktrees)._

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

### Notifications & Messaging

- [notify-rust](https://github.com/hoodie/notify-rust) _v4_ [📄](https://docs.rs/notify-rust)

    _Cross-platform desktop notifications via the freedesktop.org D-Bus Notifications interface. Used by `messenger`'s Linux desktop backend._

    _Tags: notifications, dbus, linux, desktop_

- [winrt-notification](https://github.com/allenbenz/winrt-notification) _v0.5_ [📄](https://docs.rs/winrt-notification)

    _Thin wrapper around the WinRT toast notification API. Used by `messenger`'s Windows desktop backend for unpackaged Win32 apps._

    _Tags: notifications, winrt, windows, toast_

- [objc2-user-notifications](https://github.com/madsmtm/objc2) _v0.3_ [📄](https://docs.rs/objc2-user-notifications)

    _Safe Rust bindings to Apple's `UserNotifications.framework`. Used as the opt-in native path for `messenger`'s macOS desktop backend._

    _Tags: notifications, macos, objc, framework_

- [objc2-foundation](https://github.com/madsmtm/objc2) _v0.3_ [📄](https://docs.rs/objc2-foundation)

    _Safe Rust bindings to Apple's Foundation framework (NSString, NSError, NSDictionary). Companion crate to `objc2-user-notifications` in the macOS desktop backend._

    _Tags: foundation, macos, objc, bindings_

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

- [windows-sys](https://github.com/microsoft/windows-rs) _v0.61_ [📄](https://docs.rs/windows-sys)

    _Raw Win32 console API bindings (`CreateFileW`, `GetStdHandle`, `SetStdHandle`, `CloseHandle`) used by `biscuit-tui` to redirect captured stdout to `CONOUT$` for interactive prompts on Windows. Console detection gates on `GetConsoleMode` via `std::io::IsTerminal`. Target-scoped to `[target.'cfg(windows)'.dependencies]`; Unix builds never pull it in._

    _Tags: ffi, windows, platform_

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

- [uuid](https://github.com/uuid-rs/uuid) _v1_ [📄](https://docs.rs/uuid)

    _Universally unique identifier (UUID) generation and parsing. Used by `messenger`'s desktop backends to synthesize stable `notification_id` values on macOS (AppleScript/native) and Windows (WinRT toast)._

    _Tags: uuid, identifiers, random_

### Regex & Text Processing

- [html-escape](https://github.com/magiclen/html-escape) _v0.2_

    _HTML entity encoding for safe HTML output generation._

    _Tags: html, escaping, security_

- [memchr](https://github.com/BurntSushi/memchr) _v2.7_ [📄](https://docs.rs/memchr)

    _Fast substring and byte search routines (memmem, memchr, memrchr) with SIMD acceleration. Used by `tree-hugger` for zero-allocation newline counting during god-file candidate screening._

    _Tags: text-processing, performance, search_

- [regex](https://github.com/rust-lang/regex) _v1.11_ [📄](https://docs.rs/regex)

    _Fast regular expression engine with Unicode support and linear-time guarantees._

    _Tags: regex, text-processing_

- [similar](https://github.com/mitsuhiko/similar) _v2.6_ [📄](https://docs.rs/similar)

    _Dependency-free diffing library implementing Myers and Patience algorithms. Supports inline diffs and byte slices._

    _Tags: diff, text, algorithms_

- [textwrap](https://github.com/mgeisler/textwrap) _v0.16_ [📄](https://docs.rs/textwrap)

    _Word wrapping and indenting text with optimal-fit algorithm. Supports Unicode, emojis, and hyphenation._

    _Tags: text, wrapping, formatting_

- [unicode-normalization](https://crates.io/crates/unicode-normalization) _v0.1_

    _Unicode normalization forms (NFC/NFD/NFKC/NFKD). DMLS normalizes wiki-link targets and logical paths to NFC for cross-platform-identical resolution._

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

- [quick-xml](https://github.com/tafia/quick-xml) _v0.39_ [📄](https://docs.rs/quick-xml)

    _Fast streaming XML reader/writer. Used by `darkmatter/lib` to allowlist-sanitize the promoted Mermaid static `<svg>` before it is emitted as raw HTML, stripping `<script>`/`<foreignObject>`/event-handler/external-ref payloads._

    _Tags: xml, parsing, sanitization, security_

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

- [chromiumoxide](https://github.com/mattsse/chromiumoxide) _v0.7_ [📄](https://docs.rs/chromiumoxide)

    _Headless-browser automation over the Chrome DevTools Protocol. Used by `darkmatter/lib` browser-render tests to assert on browser-computed styles of HTML/CSS output and to screenshot renders. Skips cleanly when no Chrome/Chromium is present._

    _Tags: testing, browser, html, css_

- [futures-util](https://github.com/rust-lang/futures-rs) _v0.3_ [📄](https://docs.rs/futures-util)

    _Async stream/future combinators. Used in `darkmatter/lib` browser-render tests to pump the `chromiumoxide` CDP handler stream._

    _Tags: testing, async, browser_

- [insta](https://github.com/mitsuhiko/insta) _v1.41_ [📄](https://insta.rs)

    _Snapshot testing library with VS Code integration and beautiful diffs._

    _Tags: testing, snapshots, tdd_

- [criterion](https://github.com/bheisler/criterion.rs) _v0.5_ [📄](https://docs.rs/criterion)

    _Statistics-driven micro-benchmarking framework for Rust with HTML reports and regression detection. Used by `sniff/lib/benches/perf.rs`._

    _Tags: testing, benchmarking, performance_

- [predicates](https://github.com/assert-rs/predicates-rs) _v3.1_

    _Boolean-valued predicate functions for flexible assertions._

    _Tags: testing, assertions, predicates_

- [proptest](https://github.com/proptest-rs/proptest) _v1.5_

    _Property-based testing framework generating arbitrary inputs and shrinking failing cases._

    _Tags: testing, property-testing, fuzzing_

- [rstest](https://github.com/la10736/rstest) _v0.25_

    _Fixture-based and parameterized testing framework used for new and modified Claudine tests._

    _Tags: testing, fixtures, parameterized-tests_

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
